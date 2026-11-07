//! Session state machine (SPEC §4.3).
//!
//! [`SessionMachine::apply`] records one event. [`SessionMachine::tick`] ages
//! working sessions out through `working_ttl` and drops idle sessions after
//! 24 hours. Both take time as an argument.
//!
//! Liveness is [`SessionMachine::reap_dead_pids`]. The daemon checks `/proc`
//! and passes the answer in; this module never looks at the process table.
//! Waiting grace is not applied here. A `Waiting` session stays `Waiting` for
//! display, and [`crate::policy::decide`] stops counting it after the grace.

use crate::config::{LIVENESS_INTERVAL_SECS, SESSION_EXPIRY_SECS};
use crate::types::{Agent, Event, EventKind, SessionPhase};

/// How often the daemon should call [`SessionMachine::reap_dead_pids`].
pub const LIVENESS_INTERVAL_MS: i64 = (LIVENESS_INTERVAL_SECS as i64) * 1_000;

/// Age at which an untouched idle session is removed.
pub const SESSION_EXPIRY_MS: i64 = (SESSION_EXPIRY_SECS as i64) * 1_000;

/// One tracked agent session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Which agent owns the session.
    pub agent: Agent,
    /// Agent-provided id.
    pub session_id: String,
    /// Working, waiting, or idle.
    pub phase: SessionPhase,
    /// Timestamp of the last event that changed or refreshed the session.
    pub last_activity_ms: i64,
    /// When the current wait started. Unchanged by repeated `Waiting` events.
    pub waiting_since_ms: Option<i64>,
    /// Pid, when the hook sent one. Absent pids are never reaped.
    pub pid: Option<u32>,
    /// Working directory, when the hook sent one.
    pub cwd: Option<String>,
}

impl Session {
    /// The view [`crate::policy::decide`] needs. Policy does not see pids or paths.
    #[must_use]
    pub fn view(&self) -> SessionView {
        SessionView {
            phase: self.phase,
            waiting_since_ms: self.waiting_since_ms,
        }
    }

    fn from_event(event: &Event) -> Self {
        let (phase, waiting_since_ms) = match event.kind {
            EventKind::Start | EventKind::Idle => (SessionPhase::Idle, None),
            EventKind::Working => (SessionPhase::Working, None),
            EventKind::Waiting => (SessionPhase::Waiting, Some(event.ts_ms)),
            // `End` removes a session; it is handled before this constructor.
            EventKind::End => (SessionPhase::Idle, None),
        };
        Self {
            agent: event.agent,
            session_id: event.session_id.clone(),
            phase,
            last_activity_ms: event.ts_ms,
            waiting_since_ms,
            pid: event.pid,
            cwd: event.cwd.clone(),
        }
    }
}

/// What policy needs to know about one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionView {
    /// Current phase.
    pub phase: SessionPhase,
    /// Start of the current wait, if the phase is `Waiting`.
    pub waiting_since_ms: Option<i64>,
}

/// Identity of a session that a tick or a reap touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRef {
    /// Owning agent.
    pub agent: Agent,
    /// Session id.
    pub session_id: String,
}

/// What [`SessionMachine::apply`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// The session did not exist and now does.
    Created,
    /// Phase and/or activity changed, and the session is still present.
    Updated,
    /// Working or waiting became idle. Later milestones notify on this.
    Finished,
    /// `End` removed the session.
    Removed,
    /// The event did not change the session. Waiting-from-idle is the specified case.
    Ignored,
}

/// Sessions that [`SessionMachine::tick`] moved or deleted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TickOutcome {
    /// Working sessions whose `working_ttl` elapsed. They are now idle.
    pub finished: Vec<SessionRef>,
    /// Idle sessions removed because they were untouched for 24 hours.
    pub pruned: Vec<SessionRef>,
}

/// All sessions for one user. Not shared across threads; the daemon owns it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionMachine {
    sessions: Vec<Session>,
}

impl SessionMachine {
    /// Empty machine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }

    /// Sessions in insertion order.
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// Look up by agent and id. The same id may exist on two agents.
    #[must_use]
    pub fn get(&self, agent: Agent, session_id: &str) -> Option<&Session> {
        self.position(agent, session_id)
            .map(|idx| &self.sessions[idx])
    }

    /// Views in insertion order, for [`crate::policy::decide`].
    #[must_use]
    pub fn views(&self) -> Vec<SessionView> {
        self.sessions.iter().map(Session::view).collect()
    }

    /// Apply one event. The table in SPEC §4.3 is the whole transition function.
    pub fn apply(&mut self, event: &Event) -> ApplyOutcome {
        if event.kind == EventKind::End {
            return self.remove(event.agent, &event.session_id);
        }
        if let Some(idx) = self.position(event.agent, &event.session_id) {
            return self.transition(idx, event);
        }
        self.sessions.push(Session::from_event(event));
        ApplyOutcome::Created
    }

    /// Apply `working_ttl` and the 24 hour idle expiry.
    ///
    /// A working session with no event for `working_ttl_ms` becomes idle and is
    /// reported in [`TickOutcome::finished`]. That guards a lost Stop. An idle
    /// session whose last activity is `idle_expiry_ms` or older is removed.
    /// Pass [`SESSION_EXPIRY_MS`] unless a test needs a shorter horizon.
    /// `Waiting` is left alone; grace is a policy concern.
    pub fn tick(&mut self, now_ms: i64, working_ttl_ms: i64, idle_expiry_ms: i64) -> TickOutcome {
        let mut outcome = TickOutcome::default();
        let mut index = 0;
        while index < self.sessions.len() {
            if self.sessions[index].phase == SessionPhase::Working
                && elapsed(now_ms, self.sessions[index].last_activity_ms) >= working_ttl_ms
            {
                let session = &mut self.sessions[index];
                session.phase = SessionPhase::Idle;
                session.waiting_since_ms = None;
                // The synthetic idle starts the "untouched" clock. Otherwise a
                // session that just timed out would already be part way to expiry.
                session.last_activity_ms = now_ms;
                outcome.finished.push(session_ref(session));
            }
            if self.sessions[index].phase == SessionPhase::Idle
                && elapsed(now_ms, self.sessions[index].last_activity_ms) >= idle_expiry_ms
            {
                let session = self.sessions.remove(index);
                outcome.pruned.push(session_ref(&session));
                continue;
            }
            index += 1;
        }
        outcome
    }

    /// Remove sessions whose pid the caller reports as dead.
    ///
    /// Sessions with no pid are kept. `pid_alive` is called only for sessions
    /// that have a pid. The closure is where `/proc/<pid>` is allowed to be
    /// checked, and that closure is not this crate.
    pub fn reap_dead_pids(&mut self, mut pid_alive: impl FnMut(u32) -> bool) -> Vec<SessionRef> {
        let mut removed = Vec::new();
        self.sessions.retain(|session| {
            let Some(pid) = session.pid else {
                return true;
            };
            if pid_alive(pid) {
                return true;
            }
            removed.push(session_ref(session));
            false
        });
        removed
    }

    /// Earliest time a working TTL or an idle expiry will fire, if any.
    #[must_use]
    pub fn next_deadline_ms(
        &self,
        now_ms: i64,
        working_ttl_ms: i64,
        idle_expiry_ms: i64,
    ) -> Option<i64> {
        let mut best: Option<i64> = None;
        for session in &self.sessions {
            let limit = match session.phase {
                SessionPhase::Working => session.last_activity_ms.saturating_add(working_ttl_ms),
                SessionPhase::Idle => session.last_activity_ms.saturating_add(idle_expiry_ms),
                SessionPhase::Waiting => continue,
            };
            if limit > now_ms {
                best = Some(match best {
                    Some(current) => current.min(limit),
                    None => limit,
                });
            }
        }
        best
    }

    fn position(&self, agent: Agent, session_id: &str) -> Option<usize> {
        self.sessions
            .iter()
            .position(|session| session.agent == agent && session.session_id == session_id)
    }

    fn remove(&mut self, agent: Agent, session_id: &str) -> ApplyOutcome {
        let Some(idx) = self.position(agent, session_id) else {
            return ApplyOutcome::Ignored;
        };
        self.sessions.remove(idx);
        ApplyOutcome::Removed
    }

    fn transition(&mut self, idx: usize, event: &Event) -> ApplyOutcome {
        let phase = self.sessions[idx].phase;
        match (phase, event.kind) {
            // An idle-prompt notification is not work. Leave the session untouched
            // so it does not count as activity and does not refresh expiry.
            (SessionPhase::Idle, EventKind::Waiting) => ApplyOutcome::Ignored,
            (SessionPhase::Working | SessionPhase::Waiting, EventKind::Idle) => {
                let session = &mut self.sessions[idx];
                session.phase = SessionPhase::Idle;
                session.waiting_since_ms = None;
                touch(session, event);
                ApplyOutcome::Finished
            }
            (SessionPhase::Working, EventKind::Waiting) => {
                let session = &mut self.sessions[idx];
                session.phase = SessionPhase::Waiting;
                session.waiting_since_ms = Some(event.ts_ms);
                touch(session, event);
                ApplyOutcome::Updated
            }
            (SessionPhase::Waiting | SessionPhase::Idle, EventKind::Working) => {
                let session = &mut self.sessions[idx];
                session.phase = SessionPhase::Working;
                session.waiting_since_ms = None;
                touch(session, event);
                ApplyOutcome::Updated
            }
            // Repeated waits do not move `waiting_since`. Heartbeats only refresh activity.
            (SessionPhase::Waiting, EventKind::Waiting)
            | (SessionPhase::Working, EventKind::Working)
            | (SessionPhase::Idle, EventKind::Idle)
            | (_, EventKind::Start) => {
                touch(&mut self.sessions[idx], event);
                ApplyOutcome::Updated
            }
            (_, EventKind::End) => {
                self.sessions.remove(idx);
                ApplyOutcome::Removed
            }
        }
    }
}

fn touch(session: &mut Session, event: &Event) {
    session.last_activity_ms = event.ts_ms;
    if let Some(pid) = event.pid {
        session.pid = Some(pid);
    }
    if let Some(cwd) = &event.cwd {
        session.cwd = Some(cwd.clone());
    }
}

fn session_ref(session: &Session) -> SessionRef {
    SessionRef {
        agent: session.agent,
        session_id: session.session_id.clone(),
    }
}

fn elapsed(now_ms: i64, since_ms: i64) -> i64 {
    now_ms.saturating_sub(since_ms)
}
