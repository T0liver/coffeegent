//! Domain types shared by the session machine and the policy engine.
//!
//! `Agent` and `Event` match SPEC §4.1 and §4.2. Nothing here performs I/O.

use std::fmt;

/// Which coding agent produced an event.
///
/// `Codex` is part of the enum so config and later milestones can name it.
/// It has no adapter until M7, and `agents.codex` defaults to off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Agent {
    /// Claude Code.
    Claude,
    /// Cursor.
    Cursor,
    /// Codex. No adapter until a later milestone.
    Codex,
}

impl Agent {
    /// Parse the lowercase wire name (`claude`, `cursor`, `codex`).
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "claude" => Some(Self::Claude),
            "cursor" => Some(Self::Cursor),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    /// Lowercase wire name.
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}

/// What the agent hook reported.
///
/// `Working` is also the heartbeat: repeating it refreshes `last_activity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Session became visible. Creates an idle session.
    Start,
    /// The agent is doing work, or is still alive (heartbeat).
    Working,
    /// The agent is blocked on the user.
    Waiting,
    /// The agent finished the current turn.
    Idle,
    /// The session is gone. Removes it.
    End,
}

impl EventKind {
    /// Parse the lowercase wire name.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "start" => Some(Self::Start),
            "working" => Some(Self::Working),
            "waiting" => Some(Self::Waiting),
            "idle" => Some(Self::Idle),
            "end" => Some(Self::End),
            _ => None,
        }
    }
}

/// One hook event. The daemon assigns `ts_ms` when it receives the event.
///
/// `cwd` is a path only. Prompt text never belongs in this struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Which agent emitted the event.
    pub agent: Agent,
    /// Agent-provided id, or `pid:<pid>` when the agent omitted one.
    pub session_id: String,
    /// What happened.
    pub kind: EventKind,
    /// Unix time in milliseconds, assigned by the daemon on receipt.
    pub ts_ms: i64,
    /// Agent process id when the hook could determine one.
    pub pid: Option<u32>,
    /// Working directory path. Never prompt content.
    pub cwd: Option<String>,
}

/// How long the sleep lock should stay held. Stored in `state.json` once the
/// user sets it; until then the config file supplies it.
///
/// Wire forms are `while_working` (config) and `while-working` (CLI). Both parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// Hold only while a session is working or inside the waiting grace period.
    #[default]
    WhileWorking,
    /// Hold until the user picks another mode.
    Always,
    /// Never hold for work, reachable, or always. Cutoff and pause still win.
    Off,
}

impl Mode {
    /// Parse `while_working`, `while-working`, `always`, or `off`.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "while_working" | "while-working" => Some(Self::WhileWorking),
            "always" => Some(Self::Always),
            "off" => Some(Self::Off),
            _ => None,
        }
    }

    /// Config-file spelling (`while_working`, not the CLI hyphen).
    #[must_use]
    pub fn as_config(self) -> &'static str {
        match self {
            Self::WhileWorking => "while_working",
            Self::Always => "always",
            Self::Off => "off",
        }
    }

    /// Flip between `while_working` and `off`.
    ///
    /// `always` goes to `off`, and `off` goes to `while_working`. `toggle` does
    /// not return to `always`. See `docs/rulings.md`.
    ///
    /// ```
    /// use coffeegent_core::types::Mode;
    /// assert_eq!(Mode::WhileWorking.toggle(), Mode::Off);
    /// assert_eq!(Mode::Off.toggle(), Mode::WhileWorking);
    /// assert_eq!(Mode::Always.toggle(), Mode::Off);
    /// ```
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::WhileWorking | Self::Always => Self::Off,
            Self::Off => Self::WhileWorking,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_config())
    }
}

/// Phase of one tracked session. `End` is not a phase; it removes the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionPhase {
    /// Work is in progress. Counts toward the hold, and ages out via `working_ttl`.
    Working,
    /// Blocked on the user. Contributes to the hold only inside the grace period.
    Waiting,
    /// Not working. Does not hold the lock. Pruned after 24 hours untouched.
    Idle,
}
