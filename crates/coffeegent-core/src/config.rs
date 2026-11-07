//! Configuration values and pure TOML parsing (SPEC §8).
//!
//! The caller reads `config.toml` and passes the text here. A missing file is
//! the caller's signal to use [`Config::default`]; this function does not open
//! a path. Unknown keys become warnings. Invalid values are errors.
//!
//! Once `mode` has been saved in `state.json`, it wins over the file. Pause and
//! reachable are not config fields, so parsing a file cannot change them.
//! See `docs/rulings.md`.

use thiserror::Error;

use crate::duration::TimeOfDay;
use crate::policy::PolicyConfig;
use crate::types::Mode;

/// Idle sessions untouched this long are removed. Not a config key.
pub const SESSION_EXPIRY_SECS: u64 = 24 * 60 * 60;

/// How often the daemon should ask which pids are still alive. The check
/// itself stays outside this crate.
pub const LIVENESS_INTERVAL_SECS: u64 = 10;

/// Whole parsed file plus warnings for keys this version does not understand.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedConfig {
    /// Config after defaults were filled in for omitted keys.
    pub config: Config,
    /// Unknown keys, in document order. The caller logs them.
    pub warnings: Vec<String>,
}

/// Everything SPEC §8 puts in `config.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Starting mode, used only when state has no saved mode.
    pub mode: Mode,
    /// Grace, working TTL, minimum hold, and the "tomorrow" clock time.
    pub timing: TimingConfig,
    /// Battery and thermal cutoff thresholds.
    pub cutoff: CutoffConfig,
    /// Lid inhibit and lock-on-close preferences.
    pub lid: LidConfig,
    /// Which temperature sensors the daemon should read.
    pub thermal: ThermalSensorConfig,
    /// Notification toggles. Delivery is a later milestone.
    pub notifications: NotificationsConfig,
    /// Which agents are tracked.
    pub agents: AgentsConfig,
}

/// `[timing]`.
#[derive(Debug, Clone, PartialEq)]
pub struct TimingConfig {
    /// How long a `Waiting` session still counts as a reason to hold.
    pub waiting_grace_secs: u64,
    /// How long a `Working` session may go without events before it becomes idle.
    ///
    /// Default 600 because a long shell command can sit between tool events.
    /// Gate G1 may replace this from measured gaps.
    pub working_ttl_secs: u64,
    /// Shortest hold after a fresh acquire, for work-ended releases only.
    pub min_hold_secs: u64,
    /// Local time used by the `tomorrow` duration token.
    pub tomorrow_at: TimeOfDay,
}

/// `[cutoff]`.
#[derive(Debug, Clone, PartialEq)]
pub struct CutoffConfig {
    /// Discharge percentage at or below which the battery latch closes.
    pub battery_percent: f32,
    /// Temperature at or above which a full window closes the thermal latch.
    pub thermal_c: f32,
    /// How many consecutive per-second samples must be hot.
    pub thermal_window_secs: u64,
    /// Temperature below which a full re-arm window opens the latch.
    pub thermal_rearm_c: f32,
    /// How many consecutive cool samples clear the thermal latch.
    pub thermal_rearm_secs: u64,
    /// When true, a closed lid is required before thermal cutoff runs.
    /// `lid_closed == None` (no sensor) still runs it.
    pub thermal_only_when_lid_closed: bool,
    /// When false, a latched cutoff does not override Reachable or Always.
    pub apply_to_reachable: bool,
}

/// `[lid]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LidConfig {
    /// When to include `handle-lid-switch` on the inhibitor.
    pub lid_inhibit: LidInhibit,
    /// Ask logind to lock the session when the lid closes while we hold.
    pub lock_on_close: bool,
}

/// `lid.lid_inhibit`. `never` is diagnostic and breaks the main feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidInhibit {
    /// Include the lid switch while the sleep lock is held.
    WhenHolding,
    /// Omit it. Documented as breaking lid-closed inhibit.
    Never,
}

/// `[thermal]` sensor selection. The daemon does the sysfs reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermalSensorConfig {
    /// Preferred sensor `type` names, not sysfs indexes.
    pub sensor_types: Vec<String>,
    /// Types left out of the fallback "max of the rest" reading.
    pub ignore_types: Vec<String>,
}

/// `[notifications]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationsConfig {
    /// Tell the user when Claude is waiting.
    pub waiting: bool,
    /// Tell the user when a long enough work stint goes idle.
    pub finished: bool,
    /// Minimum working time before a finished notification.
    pub finished_min_work_secs: u64,
    /// Minimum gap between notifications for one session.
    pub cooldown_secs: u64,
}

/// `[agents]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsConfig {
    /// Track Claude Code.
    pub claude: bool,
    /// Track Cursor.
    pub cursor: bool,
    /// Track Codex. Default false until a later milestone.
    pub codex: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::WhileWorking,
            timing: TimingConfig::default(),
            cutoff: CutoffConfig::default(),
            lid: LidConfig::default(),
            thermal: ThermalSensorConfig::default(),
            notifications: NotificationsConfig::default(),
            agents: AgentsConfig::default(),
        }
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            waiting_grace_secs: 300,
            working_ttl_secs: 600,
            min_hold_secs: 5,
            tomorrow_at: TimeOfDay::default_tomorrow(),
        }
    }
}

impl Default for CutoffConfig {
    fn default() -> Self {
        Self {
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

impl Default for LidConfig {
    fn default() -> Self {
        Self {
            lid_inhibit: LidInhibit::WhenHolding,
            lock_on_close: true,
        }
    }
}

impl Default for ThermalSensorConfig {
    fn default() -> Self {
        Self {
            sensor_types: vec!["x86_pkg_temp".to_owned()],
            ignore_types: vec!["INT3400".to_owned(), "SEN1".to_owned(), "SEN3".to_owned()],
        }
    }
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            waiting: true,
            finished: true,
            finished_min_work_secs: 30,
            cooldown_secs: 10,
        }
    }
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            claude: true,
            cursor: true,
            codex: false,
        }
    }
}

impl TimingConfig {
    /// `working_ttl_secs` as unix milliseconds.
    #[must_use]
    pub fn working_ttl_ms(&self) -> i64 {
        secs_to_ms(self.working_ttl_secs)
    }
}

impl Config {
    /// The slice of config [`crate::policy::decide`] reads.
    #[must_use]
    pub fn policy(&self) -> PolicyConfig {
        PolicyConfig {
            waiting_grace_ms: secs_to_ms(self.timing.waiting_grace_secs),
            min_hold_ms: secs_to_ms(self.timing.min_hold_secs),
            battery_percent: self.cutoff.battery_percent,
            thermal_c: self.cutoff.thermal_c,
            thermal_window_secs: self.cutoff.thermal_window_secs,
            thermal_rearm_c: self.cutoff.thermal_rearm_c,
            thermal_rearm_secs: self.cutoff.thermal_rearm_secs,
            thermal_only_when_lid_closed: self.cutoff.thermal_only_when_lid_closed,
            apply_to_reachable: self.cutoff.apply_to_reachable,
        }
    }
}

/// Saved mode wins. The config mode applies only when `persisted` is `None`.
///
/// `coffeegent mode` and `toggle` write state, not the file. `reload` refreshes
/// the rest of the config and leaves mode, pause, and reachable alone. Pause
/// and reachable are absent from [`Config`] for that reason. See `docs/rulings.md`.
///
/// ```
/// use coffeegent_core::config::effective_mode;
/// use coffeegent_core::types::Mode;
/// assert_eq!(effective_mode(Some(Mode::Off), Mode::Always), Mode::Off);
/// assert_eq!(effective_mode(None, Mode::Always), Mode::Always);
/// ```
#[must_use]
pub fn effective_mode(persisted: Option<Mode>, config_mode: Mode) -> Mode {
    persisted.unwrap_or(config_mode)
}

/// Parse TOML text. Empty text is [`Config::default`] with no warnings.
pub fn parse_config(text: &str) -> Result<ParsedConfig, ConfigError> {
    if text.trim().is_empty() {
        return Ok(ParsedConfig {
            config: Config::default(),
            warnings: Vec::new(),
        });
    }
    let value: toml::Value =
        toml::from_str(text).map_err(|err| ConfigError::Parse(err.to_string()))?;
    let table = match value {
        toml::Value::Table(table) => table,
        _ => {
            return Err(ConfigError::Invalid {
                field: "config",
                value: "document is not a table".to_owned(),
            });
        }
    };

    let mut config = Config::default();
    let mut warnings = Vec::new();
    for (key, value) in table {
        match key.as_str() {
            "mode" => config.mode = required_mode(&value)?,
            "timing" => config.timing = parse_timing(value, &mut warnings)?,
            "cutoff" => config.cutoff = parse_cutoff(value, &mut warnings)?,
            "lid" => config.lid = parse_lid(value, &mut warnings)?,
            "thermal" => config.thermal = parse_thermal(value, &mut warnings)?,
            "notifications" => config.notifications = parse_notifications(value, &mut warnings)?,
            "agents" => config.agents = parse_agents(value, &mut warnings)?,
            other => warnings.push(format!("unknown config key `{other}`")),
        }
    }
    Ok(ParsedConfig { config, warnings })
}

/// Why config text was rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// The TOML syntax itself is wrong.
    #[error("could not parse config: {0}")]
    Parse(String),
    /// A known key has the wrong type or an unknown enum spelling.
    #[error("invalid value for {field}: {value}")]
    Invalid {
        /// Dotted config path, for example `timing.tomorrow_at`.
        field: &'static str,
        /// The text we could not accept.
        value: String,
    },
}

fn secs_to_ms(secs: u64) -> i64 {
    let ms = secs.saturating_mul(1_000);
    i64::try_from(ms).unwrap_or(i64::MAX)
}

fn required_mode(value: &toml::Value) -> Result<Mode, ConfigError> {
    let text = value.as_str().ok_or_else(|| ConfigError::Invalid {
        field: "mode",
        value: value.to_string(),
    })?;
    Mode::from_wire(text).ok_or_else(|| ConfigError::Invalid {
        field: "mode",
        value: text.to_owned(),
    })
}

fn parse_timing(
    value: toml::Value,
    warnings: &mut Vec<String>,
) -> Result<TimingConfig, ConfigError> {
    let mut timing = TimingConfig::default();
    for (key, value) in require_table("timing", value)? {
        match key.as_str() {
            "waiting_grace_secs" => {
                timing.waiting_grace_secs = required_u64("timing.waiting_grace_secs", &value)?
            }
            "working_ttl_secs" => {
                timing.working_ttl_secs = required_u64("timing.working_ttl_secs", &value)?
            }
            "min_hold_secs" => timing.min_hold_secs = required_u64("timing.min_hold_secs", &value)?,
            "tomorrow_at" => timing.tomorrow_at = required_time(&value)?,
            other => warnings.push(format!("unknown config key `timing.{other}`")),
        }
    }
    Ok(timing)
}

fn parse_cutoff(
    value: toml::Value,
    warnings: &mut Vec<String>,
) -> Result<CutoffConfig, ConfigError> {
    let mut cutoff = CutoffConfig::default();
    for (key, value) in require_table("cutoff", value)? {
        match key.as_str() {
            "battery_percent" => {
                cutoff.battery_percent = required_f32("cutoff.battery_percent", &value)?
            }
            "thermal_c" => cutoff.thermal_c = required_f32("cutoff.thermal_c", &value)?,
            "thermal_window_secs" => {
                cutoff.thermal_window_secs = required_u64("cutoff.thermal_window_secs", &value)?;
            }
            "thermal_rearm_c" => {
                cutoff.thermal_rearm_c = required_f32("cutoff.thermal_rearm_c", &value)?
            }
            "thermal_rearm_secs" => {
                cutoff.thermal_rearm_secs = required_u64("cutoff.thermal_rearm_secs", &value)?;
            }
            "thermal_only_when_lid_closed" => {
                cutoff.thermal_only_when_lid_closed =
                    required_bool("cutoff.thermal_only_when_lid_closed", &value)?;
            }
            "apply_to_reachable" => {
                cutoff.apply_to_reachable = required_bool("cutoff.apply_to_reachable", &value)?;
            }
            other => warnings.push(format!("unknown config key `cutoff.{other}`")),
        }
    }
    Ok(cutoff)
}

fn parse_lid(value: toml::Value, warnings: &mut Vec<String>) -> Result<LidConfig, ConfigError> {
    let mut lid = LidConfig::default();
    for (key, value) in require_table("lid", value)? {
        match key.as_str() {
            "lid_inhibit" => lid.lid_inhibit = required_inhibit(&value)?,
            "lock_on_close" => lid.lock_on_close = required_bool("lid.lock_on_close", &value)?,
            other => warnings.push(format!("unknown config key `lid.{other}`")),
        }
    }
    Ok(lid)
}

fn parse_thermal(
    value: toml::Value,
    warnings: &mut Vec<String>,
) -> Result<ThermalSensorConfig, ConfigError> {
    let mut thermal = ThermalSensorConfig::default();
    for (key, value) in require_table("thermal", value)? {
        match key.as_str() {
            "sensor_types" => {
                thermal.sensor_types = required_strings("thermal.sensor_types", &value)?
            }
            "ignore_types" => {
                thermal.ignore_types = required_strings("thermal.ignore_types", &value)?
            }
            other => warnings.push(format!("unknown config key `thermal.{other}`")),
        }
    }
    Ok(thermal)
}

fn parse_notifications(
    value: toml::Value,
    warnings: &mut Vec<String>,
) -> Result<NotificationsConfig, ConfigError> {
    let mut notifications = NotificationsConfig::default();
    for (key, value) in require_table("notifications", value)? {
        match key.as_str() {
            "waiting" => notifications.waiting = required_bool("notifications.waiting", &value)?,
            "finished" => notifications.finished = required_bool("notifications.finished", &value)?,
            "finished_min_work_secs" => {
                notifications.finished_min_work_secs =
                    required_u64("notifications.finished_min_work_secs", &value)?;
            }
            "cooldown_secs" => {
                notifications.cooldown_secs = required_u64("notifications.cooldown_secs", &value)?;
            }
            other => warnings.push(format!("unknown config key `notifications.{other}`")),
        }
    }
    Ok(notifications)
}

fn parse_agents(
    value: toml::Value,
    warnings: &mut Vec<String>,
) -> Result<AgentsConfig, ConfigError> {
    let mut agents = AgentsConfig::default();
    for (key, value) in require_table("agents", value)? {
        match key.as_str() {
            "claude" => agents.claude = required_bool("agents.claude", &value)?,
            "cursor" => agents.cursor = required_bool("agents.cursor", &value)?,
            "codex" => agents.codex = required_bool("agents.codex", &value)?,
            other => warnings.push(format!("unknown config key `agents.{other}`")),
        }
    }
    Ok(agents)
}

fn require_table(
    field: &'static str,
    value: toml::Value,
) -> Result<toml::map::Map<String, toml::Value>, ConfigError> {
    match value {
        toml::Value::Table(table) => Ok(table),
        other => Err(ConfigError::Invalid {
            field,
            value: other.to_string(),
        }),
    }
}

fn required_u64(field: &'static str, value: &toml::Value) -> Result<u64, ConfigError> {
    let Some(integer) = value.as_integer() else {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
        });
    };
    u64::try_from(integer).map_err(|_| ConfigError::Invalid {
        field,
        value: value.to_string(),
    })
}

fn required_f32(field: &'static str, value: &toml::Value) -> Result<f32, ConfigError> {
    if let Some(integer) = value.as_integer() {
        return Ok(integer as f32);
    }
    if let Some(float) = value.as_float() {
        return Ok(float as f32);
    }
    Err(ConfigError::Invalid {
        field,
        value: value.to_string(),
    })
}

fn required_bool(field: &'static str, value: &toml::Value) -> Result<bool, ConfigError> {
    value.as_bool().ok_or_else(|| ConfigError::Invalid {
        field,
        value: value.to_string(),
    })
}

fn required_time(value: &toml::Value) -> Result<TimeOfDay, ConfigError> {
    let text = value.as_str().ok_or_else(|| ConfigError::Invalid {
        field: "timing.tomorrow_at",
        value: value.to_string(),
    })?;
    TimeOfDay::parse(text).map_err(|_| ConfigError::Invalid {
        field: "timing.tomorrow_at",
        value: text.to_owned(),
    })
}

fn required_inhibit(value: &toml::Value) -> Result<LidInhibit, ConfigError> {
    let text = value.as_str().ok_or_else(|| ConfigError::Invalid {
        field: "lid.lid_inhibit",
        value: value.to_string(),
    })?;
    match text {
        "when_holding" => Ok(LidInhibit::WhenHolding),
        "never" => Ok(LidInhibit::Never),
        _ => Err(ConfigError::Invalid {
            field: "lid.lid_inhibit",
            value: text.to_owned(),
        }),
    }
}

fn required_strings(field: &'static str, value: &toml::Value) -> Result<Vec<String>, ConfigError> {
    let Some(items) = value.as_array() else {
        return Err(ConfigError::Invalid {
            field,
            value: value.to_string(),
        });
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(ConfigError::Invalid {
                field,
                value: item.to_string(),
            });
        };
        out.push(text.to_owned());
    }
    Ok(out)
}
