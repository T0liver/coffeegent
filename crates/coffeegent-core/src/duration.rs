//! Human-readable timers for `pause` and `reachable`.
//!
//! SPEC §10.1 says durations use `humantime`. `tomorrow` and `off` are product
//! words, not humantime syntax. `tomorrow_at` comes from config (default 06:00).
//! The local offset is an argument so this module does not read the timezone
//! database.

use std::time::Duration;

use thiserror::Error;

/// Clock time on a local calendar, with no date and no timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    /// Hour from 0 to 23.
    pub hour: u8,
    /// Minute from 0 to 59.
    pub minute: u8,
}

impl TimeOfDay {
    /// 06:00, the SPEC default for `timing.tomorrow_at`.
    #[must_use]
    pub const fn default_tomorrow() -> Self {
        Self { hour: 6, minute: 0 }
    }

    /// Parse `HH:MM` (hour 0–23, minute 0–59). Seconds are not accepted.
    pub fn parse(text: &str) -> Result<Self, DurationError> {
        let (hour, minute) = text
            .split_once(':')
            .ok_or_else(|| DurationError::BadTime(text.to_owned()))?;
        if hour.len() != 2 || minute.len() != 2 {
            return Err(DurationError::BadTime(text.to_owned()));
        }
        let hour: u8 = hour
            .parse()
            .map_err(|_| DurationError::BadTime(text.to_owned()))?;
        let minute: u8 = minute
            .parse()
            .map_err(|_| DurationError::BadTime(text.to_owned()))?;
        if hour > 23 || minute > 59 {
            return Err(DurationError::BadTime(text.to_owned()));
        }
        Ok(Self { hour, minute })
    }
}

/// A timer token before it is placed on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerToken {
    /// A humantime duration measured from `now`.
    After(Duration),
    /// The next calendar day's `tomorrow_at`, in the caller's offset.
    Tomorrow,
    /// No end. Callers store this as [`i64::MAX`] in the until-fields.
    Indefinite,
    /// Cancel the timer (`reachable off`).
    Off,
}

/// Where a token sits once `now` and the local offset are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTimer {
    /// Absolute unix milliseconds.
    UntilMs(i64),
    /// Store as [`i64::MAX`] (SPEC §5: indefinite pause).
    Indefinite,
    /// Drop the timer.
    Clear,
}

impl ResolvedTimer {
    /// Value for `paused_until_ms` / `reachable_until_ms`.
    ///
    /// `None` clears the field. [`i64::MAX`] means indefinite.
    #[must_use]
    pub fn as_until_ms(self) -> Option<i64> {
        match self {
            Self::UntilMs(ms) => Some(ms),
            Self::Indefinite => Some(i64::MAX),
            Self::Clear => None,
        }
    }
}

/// Why a duration string was rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DurationError {
    /// Blank input.
    #[error("empty duration")]
    Empty,
    /// Not humantime and not `tomorrow`, `off`, or `indefinite`.
    #[error("invalid duration `{0}`")]
    Invalid(String),
    /// Not `HH:MM`.
    #[error("invalid time of day `{0}`")]
    BadTime(String),
    /// Adding the duration to `now` overflowed.
    #[error("duration does not fit in unix milliseconds")]
    Overflow,
}

/// Parse `30m`, `1h`, a general humantime string, `tomorrow`, `off`, or `indefinite`.
///
/// Keyword matching is case-insensitive. Humantime's own spelling is unchanged
/// (`30m`, not `30M`).
pub fn parse_timer(input: &str) -> Result<TimerToken, DurationError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(DurationError::Empty);
    }
    match text.to_ascii_lowercase().as_str() {
        "tomorrow" => return Ok(TimerToken::Tomorrow),
        "off" => return Ok(TimerToken::Off),
        "indefinite" => return Ok(TimerToken::Indefinite),
        _ => {}
    }
    humantime::parse_duration(text)
        .map(TimerToken::After)
        .map_err(|_| DurationError::Invalid(text.to_owned()))
}

/// Place a token on the unix timeline.
///
/// `utc_offset_secs` is the local offset to use for `tomorrow` (east of UTC is
/// positive). This function does not discover the offset itself. "Tomorrow"
/// means the next calendar date at `tomorrow_at` in that offset, not "later
/// today if the time has not passed yet".
pub fn resolve_timer(
    token: &TimerToken,
    now_ms: i64,
    tomorrow_at: TimeOfDay,
    utc_offset_secs: i32,
) -> Result<ResolvedTimer, DurationError> {
    match token {
        TimerToken::Off => Ok(ResolvedTimer::Clear),
        TimerToken::Indefinite => Ok(ResolvedTimer::Indefinite),
        TimerToken::Tomorrow => {
            let ms = tomorrow_unix_ms(now_ms, tomorrow_at, utc_offset_secs)?;
            Ok(ResolvedTimer::UntilMs(ms))
        }
        TimerToken::After(duration) => {
            let add = i64::try_from(duration.as_millis()).map_err(|_| DurationError::Overflow)?;
            let until = now_ms.checked_add(add).ok_or(DurationError::Overflow)?;
            Ok(ResolvedTimer::UntilMs(until))
        }
    }
}

/// Unix milliseconds of the next local calendar day at `at`.
pub fn tomorrow_unix_ms(
    now_ms: i64,
    at: TimeOfDay,
    utc_offset_secs: i32,
) -> Result<i64, DurationError> {
    let offset = i64::from(utc_offset_secs);
    let now_sec = now_ms.div_euclid(1000);
    let local_sec = now_sec.checked_add(offset).ok_or(DurationError::Overflow)?;
    let day = local_sec.div_euclid(86_400);
    let next_midnight = day
        .checked_add(1)
        .and_then(|next| next.checked_mul(86_400))
        .ok_or(DurationError::Overflow)?;
    let target_local = next_midnight
        .checked_add(i64::from(at.hour) * 3_600)
        .and_then(|t| t.checked_add(i64::from(at.minute) * 60))
        .ok_or(DurationError::Overflow)?;
    let target_unix = target_local
        .checked_sub(offset)
        .ok_or(DurationError::Overflow)?;
    target_unix.checked_mul(1000).ok_or(DurationError::Overflow)
}
