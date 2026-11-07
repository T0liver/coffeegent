//! Hold policy (SPEC §5).
//!
//! [`decide`] is a pure function. It does not read the clock or any sensor.
//! The caller passes `now_ms`, the power sample, and the previous latch.
//!
//! Precedence, highest first:
//! 1. Latched cutoff (battery, else thermal), unless `apply_to_reachable` is
//!    false and Reachable or Always would hold.
//! 2. Pause, then mode off.
//! 3. Reachable.
//! 4. Always.
//! 5. While-working: any `Working` session, or a `Waiting` session still inside
//!    the grace period.
//! 6. Otherwise release.
//!
//! `min_hold_secs` runs after that list and only delays a release into
//! [`HoldReason::NothingToDo`] when the previous hold was work, waiting grace,
//! or reachable. A latched cutoff, pause, and mode off release immediately
//! (`docs/rulings.md`).

use crate::session::SessionView;
use crate::types::{Mode, SessionPhase};

/// Numbers from `[timing]` and `[cutoff]` that the decision reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyConfig {
    /// Waiting grace in milliseconds.
    pub waiting_grace_ms: i64,
    /// Minimum hold in milliseconds. Zero disables the delay.
    pub min_hold_ms: i64,
    /// Battery percent at or below which the latch closes while discharging.
    pub battery_percent: f32,
    /// Thermal cutoff in degrees Celsius.
    pub thermal_c: f32,
    /// Consecutive hot samples required to close the thermal latch.
    pub thermal_window_secs: u64,
    /// Temperature the re-arm window must sit below.
    pub thermal_rearm_c: f32,
    /// Consecutive cool samples required to open the thermal latch.
    pub thermal_rearm_secs: u64,
    /// Restrict thermal evaluation to a closed lid. Unknown lid still evaluates.
    pub thermal_only_when_lid_closed: bool,
    /// When false, cutoff does not override Reachable or Always.
    pub apply_to_reachable: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            waiting_grace_ms: 300_000,
            min_hold_ms: 5_000,
            battery_percent: 20.0,
            thermal_c: 85.0,
            thermal_window_secs: 20,
            thermal_rearm_c: 70.0,
            thermal_rearm_secs: 30,
            thermal_only_when_lid_closed: true,
            apply_to_reachable: true,
        }
    }
}

/// One power and thermal observation. The daemon fills this in; the core trusts it.
///
/// `discharging` is true only for the UPower state Discharging. "Not charging"
/// and "Fully charged" are not discharging, including at 100%.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PowerSnapshot {
    /// A battery is present. Desktops leave this false.
    pub battery_present: bool,
    /// True only when the battery state is Discharging.
    pub discharging: bool,
    /// Percent remaining, when the backend could read one.
    pub battery_percent: Option<f32>,
    /// One temperature sample, degrees Celsius. `None` means this second has no reading.
    pub temp_c: Option<f32>,
    /// `Some(true)` lid closed, `Some(false)` open, `None` if the machine has no lid.
    pub lid_closed: Option<bool>,
}

/// Latch flags plus the thermal ring. The caller owns this and passes it back.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CutoffState {
    /// Battery cutoff is latched.
    pub battery_latched: bool,
    /// Thermal cutoff is latched. It is not applied while the lid rule says not to evaluate.
    pub thermal_latched: bool,
    /// Newest sample at the end. One entry per `decide` call that carried `temp_c`.
    pub samples_c: Vec<f32>,
}

/// Why [`decide`] held or released. Counts are sessions that contributed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    /// At least one session is `Working`.
    Working(usize),
    /// No session is working, but this many waits are still inside the grace period.
    WaitingGrace(usize),
    /// The stay-reachable timer is active.
    Reachable,
    /// Mode is always.
    AlwaysMode,
    /// The user paused holds.
    Paused,
    /// Mode is off.
    ModeOff,
    /// Battery cutoff is latched and it applies.
    CutoffBattery,
    /// Thermal cutoff is latched and it applies.
    CutoffThermal,
    /// Nothing in the precedence list wants a hold.
    NothingToDo,
    /// Work, grace, or reachable ended, but the acquire is younger than `min_hold_secs`.
    ///
    /// This is not a new precedence step. It only postpones [`Self::NothingToDo`].
    MinHold,
}

/// Everything [`decide`] needs. Borrowed data stays with the caller.
///
/// `cutoff` is owned, matching SPEC §5, so the caller can move the previous
/// latch in and store [`Decision::new_cutoff`] for the next tick.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyInput<'a> {
    /// Unix time in milliseconds. There is no other clock.
    pub now_ms: i64,
    /// Effective mode. Use [`crate::config::effective_mode`] so a saved mode wins.
    pub mode: Mode,
    /// Pause end, or [`i64::MAX`] for indefinite. `None` means not paused.
    pub paused_until_ms: Option<i64>,
    /// Reachable end, or [`i64::MAX`] for indefinite. `None` means not reachable.
    pub reachable_until_ms: Option<i64>,
    /// Current sessions.
    pub sessions: &'a [SessionView],
    /// Latest power and thermal sample.
    pub power: &'a PowerSnapshot,
    /// Latches and the thermal ring from the previous decision.
    pub cutoff: CutoffState,
    /// Thresholds.
    pub cfg: &'a PolicyConfig,
    /// When the current hold was acquired. `None` if the lock is not held.
    ///
    /// The daemon sets this to `now_ms` on the tick that first returns `hold`,
    /// keeps it while `hold` stays true, and clears it when `hold` becomes false.
    pub hold_since_ms: Option<i64>,
    /// [`Decision::reason`] from the previous tick while the lock is held.
    pub held_reason: Option<HoldReason>,
}

/// What the daemon should do with the inhibitor, plus the latch to store.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    /// Whether the sleep lock should be held.
    pub hold: bool,
    /// Why.
    pub reason: HoldReason,
    /// Latch and thermal ring after this sample.
    pub new_cutoff: CutoffState,
    /// Next time the decision may change with no new event or sample.
    ///
    /// The daemon also wakes on events, samples, and the session tick.
    pub next_deadline_ms: Option<i64>,
}

/// Pause and reachable cannot both be set. The last update wins (SPEC §5).
///
/// Passing both to [`decide`] anyway still prefers pause, because pause is
/// higher in the precedence list. This type is how commands stay exclusive
/// before that.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivityTimers {
    /// Pause end, [`i64::MAX`] if indefinite.
    pub paused_until_ms: Option<i64>,
    /// Reachable end, [`i64::MAX`] if indefinite.
    pub reachable_until_ms: Option<i64>,
}

impl ActivityTimers {
    /// Pause until `until_ms` and cancel reachable.
    pub fn pause(&mut self, until_ms: i64) {
        self.paused_until_ms = Some(until_ms);
        self.reachable_until_ms = None;
    }

    /// Cancel pause. Reachable is left as it is.
    pub fn resume(&mut self) {
        self.paused_until_ms = None;
    }

    /// Stay reachable until `until_ms` and cancel pause.
    pub fn reachable(&mut self, until_ms: i64) {
        self.reachable_until_ms = Some(until_ms);
        self.paused_until_ms = None;
    }

    /// Cancel reachable. Pause is left as it is.
    pub fn clear_reachable(&mut self) {
        self.reachable_until_ms = None;
    }
}

/// Decide whether to hold the sleep lock.
///
/// Each call with [`PowerSnapshot::temp_c`] set appends one thermal sample.
/// The daemon should do that once per second. `None` leaves the ring and the
/// thermal latch unchanged, and a missing temperature never newly triggers.
#[must_use]
pub fn decide(input: &PolicyInput<'_>) -> Decision {
    let mut cutoff = input.cutoff.clone();
    update_battery(&mut cutoff, input.power, input.cfg);
    update_thermal(&mut cutoff, input.power, input.cfg);

    let (raw_hold, raw_reason) = preliminary(input, &cutoff);
    let (hold, reason) = apply_min_hold(input, raw_hold, raw_reason);
    let next_deadline_ms = next_deadline(input, hold, reason);
    Decision {
        hold,
        reason,
        new_cutoff: cutoff,
        next_deadline_ms,
    }
}

fn update_battery(cutoff: &mut CutoffState, power: &PowerSnapshot, cfg: &PolicyConfig) {
    let on_battery = power.battery_present && power.discharging;
    let clear_at = cfg.battery_percent + 5.0;
    if cutoff.battery_latched {
        // Clear on AC (anything that is not actually discharging) or after +5%.
        // A missing percent does not count as recovered.
        let recovered = power
            .battery_percent
            .is_some_and(|percent| percent >= clear_at);
        if !on_battery || recovered {
            cutoff.battery_latched = false;
        }
        return;
    }
    if !on_battery {
        return;
    }
    if power
        .battery_percent
        .is_some_and(|percent| percent <= cfg.battery_percent)
    {
        cutoff.battery_latched = true;
    }
}

fn update_thermal(cutoff: &mut CutoffState, power: &PowerSnapshot, cfg: &PolicyConfig) {
    // No reading: do not invent a sample, and do not move the latch.
    let Some(sample) = power.temp_c else {
        return;
    };
    let window = as_len(cfg.thermal_window_secs);
    let rearm = as_len(cfg.thermal_rearm_secs);
    // The spec sizes the ring from the re-arm length. The window can be
    // configured longer, so keep whichever the rules actually look at.
    let cap = window.max(rearm);
    if cap > 0 {
        cutoff.samples_c.push(sample);
        if cutoff.samples_c.len() > cap {
            let extra = cutoff.samples_c.len() - cap;
            cutoff.samples_c.drain(0..extra);
        }
    }
    // Lid open (when the flag says so) freezes the latch. Samples still land
    // in the ring so a later close sees the real recent temperature.
    if !thermal_evaluates(power, cfg) {
        return;
    }
    if !cutoff.thermal_latched {
        if window_is(cutoff, window, |temp| temp >= cfg.thermal_c) {
            cutoff.thermal_latched = true;
        }
        return;
    }
    if window_is(cutoff, rearm, |temp| temp < cfg.thermal_rearm_c) {
        cutoff.thermal_latched = false;
    }
}

fn thermal_evaluates(power: &PowerSnapshot, cfg: &PolicyConfig) -> bool {
    if !cfg.thermal_only_when_lid_closed {
        return true;
    }
    // No lid sensor: a desktop cannot satisfy "lid closed", and the cutoff
    // would otherwise never run. Unknown evaluates; open does not.
    !matches!(power.lid_closed, Some(false))
}

fn window_is(cutoff: &CutoffState, len: usize, pred: impl Fn(f32) -> bool) -> bool {
    if len == 0 || cutoff.samples_c.len() < len {
        return false;
    }
    let start = cutoff.samples_c.len() - len;
    cutoff.samples_c[start..].iter().copied().all(pred)
}

fn as_len(secs: u64) -> usize {
    usize::try_from(secs).unwrap_or(usize::MAX)
}

fn preliminary(input: &PolicyInput<'_>, cutoff: &CutoffState) -> (bool, HoldReason) {
    if let Some(reason) = cutoff_reason(input, cutoff) {
        // `apply_to_reachable = false` also leaves Always alone. The flag's
        // name is historical; SPEC §5 groups Reachable and Always together.
        let exempt = !input.cfg.apply_to_reachable
            && (timer_active(input.now_ms, input.reachable_until_ms) || input.mode == Mode::Always);
        if !exempt {
            return (false, reason);
        }
    }
    if timer_active(input.now_ms, input.paused_until_ms) {
        return (false, HoldReason::Paused);
    }
    if input.mode == Mode::Off {
        return (false, HoldReason::ModeOff);
    }
    if timer_active(input.now_ms, input.reachable_until_ms) {
        return (true, HoldReason::Reachable);
    }
    if input.mode == Mode::Always {
        return (true, HoldReason::AlwaysMode);
    }
    let working = input
        .sessions
        .iter()
        .filter(|session| session.phase == SessionPhase::Working)
        .count();
    if working > 0 {
        return (true, HoldReason::Working(working));
    }
    let grace = input
        .sessions
        .iter()
        .filter(|session| in_grace(input, session))
        .count();
    if grace > 0 {
        return (true, HoldReason::WaitingGrace(grace));
    }
    (false, HoldReason::NothingToDo)
}

fn cutoff_reason(input: &PolicyInput<'_>, cutoff: &CutoffState) -> Option<HoldReason> {
    if cutoff.battery_latched {
        return Some(HoldReason::CutoffBattery);
    }
    if cutoff.thermal_latched && thermal_evaluates(input.power, input.cfg) {
        return Some(HoldReason::CutoffThermal);
    }
    None
}

fn in_grace(input: &PolicyInput<'_>, session: &SessionView) -> bool {
    if session.phase != SessionPhase::Waiting {
        return false;
    }
    let Some(since) = session.waiting_since_ms else {
        return false;
    };
    input.now_ms.saturating_sub(since) < input.cfg.waiting_grace_ms
}

fn apply_min_hold(input: &PolicyInput<'_>, hold: bool, reason: HoldReason) -> (bool, HoldReason) {
    if hold || reason != HoldReason::NothingToDo {
        return (hold, reason);
    }
    let Some(since) = input.hold_since_ms else {
        return (false, reason);
    };
    // Only a release because work ended: the previous reason was work, grace,
    // reachable, or a min-hold we are already serving. Leaving Always is not
    // in that list, and cutoff / pause / mode off never arrive here.
    let work_ended = matches!(
        input.held_reason,
        Some(
            HoldReason::Working(_)
                | HoldReason::WaitingGrace(_)
                | HoldReason::Reachable
                | HoldReason::MinHold
        )
    );
    if !work_ended {
        return (false, reason);
    }
    if input.now_ms.saturating_sub(since) < input.cfg.min_hold_ms {
        return (true, HoldReason::MinHold);
    }
    (false, reason)
}

fn next_deadline(input: &PolicyInput<'_>, hold: bool, reason: HoldReason) -> Option<i64> {
    let mut best: Option<i64> = None;
    let mut push = |when: i64| {
        if when > input.now_ms {
            best = Some(match best {
                Some(current) => current.min(when),
                None => when,
            });
        }
    };

    if hold && reason == HoldReason::MinHold {
        if let Some(since) = input.hold_since_ms {
            push(since.saturating_add(input.cfg.min_hold_ms));
        }
    }
    if let Some(until) = input.paused_until_ms {
        if timer_active(input.now_ms, input.paused_until_ms) && until != i64::MAX {
            push(until);
        }
    }
    if hold && reason == HoldReason::Reachable {
        if let Some(until) = input.reachable_until_ms {
            if until != i64::MAX {
                push(until);
            }
        }
    }
    if hold && matches!(reason, HoldReason::Working(_) | HoldReason::WaitingGrace(_)) {
        for session in input.sessions {
            if !in_grace(input, session) {
                continue;
            }
            if let Some(since) = session.waiting_since_ms {
                push(since.saturating_add(input.cfg.waiting_grace_ms));
            }
        }
    }
    best
}

fn timer_active(now_ms: i64, until_ms: Option<i64>) -> bool {
    match until_ms {
        Some(until) => now_ms < until,
        None => false,
    }
}
