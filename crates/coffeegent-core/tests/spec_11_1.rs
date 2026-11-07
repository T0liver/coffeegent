//! SPEC §11.1 core tests: every session transition, every precedence step,
//! both cutoff latches, and the confirmed rulings in `docs/rulings.md`.

use std::time::Duration;

use coffeegent_core::config::{self, effective_mode, Config};
use coffeegent_core::duration::{self, TimeOfDay, TimerToken};
use coffeegent_core::policy::{
    decide, ActivityTimers, CutoffState, HoldReason, PolicyConfig, PolicyInput, PowerSnapshot,
};
use coffeegent_core::session::{ApplyOutcome, SessionMachine, SESSION_EXPIRY_MS};
use coffeegent_core::types::{Agent, Event, EventKind, Mode, SessionPhase};

const TTL_MS: i64 = 600_000;
const GRACE_MS: i64 = 300_000;

fn event(kind: EventKind, ts_ms: i64) -> Event {
    Event {
        agent: Agent::Claude,
        session_id: "s".to_owned(),
        kind,
        ts_ms,
        pid: None,
        cwd: None,
    }
}

fn phase(machine: &SessionMachine) -> SessionPhase {
    machine.get(Agent::Claude, "s").expect("session").phase
}

fn policy() -> PolicyConfig {
    Config::default().policy()
}

fn plugged() -> PowerSnapshot {
    PowerSnapshot {
        battery_present: true,
        discharging: false,
        battery_percent: Some(80.0),
        temp_c: Some(45.0),
        lid_closed: Some(false),
    }
}

fn input<'a>(
    now_ms: i64,
    mode: Mode,
    sessions: &'a [coffeegent_core::session::SessionView],
    power: &'a PowerSnapshot,
    cutoff: CutoffState,
    cfg: &'a PolicyConfig,
) -> PolicyInput<'a> {
    PolicyInput {
        now_ms,
        mode,
        paused_until_ms: None,
        reachable_until_ms: None,
        sessions,
        power,
        cutoff,
        cfg,
        hold_since_ms: None,
        held_reason: None,
    }
}

fn feed_temp(samples: usize, temp: f32, lid: Option<bool>, cfg: &PolicyConfig) -> CutoffState {
    let mut cutoff = CutoffState::default();
    let power = PowerSnapshot {
        temp_c: Some(temp),
        lid_closed: lid,
        ..plugged()
    };
    let sessions = [];
    for _ in 0..samples {
        let decision = decide(&input(0, Mode::Always, &sessions, &power, cutoff, cfg));
        cutoff = decision.new_cutoff;
    }
    cutoff
}

#[test]
fn fsm_start_creates_idle() {
    let mut machine = SessionMachine::new();
    assert_eq!(
        machine.apply(&event(EventKind::Start, 10)),
        ApplyOutcome::Created
    );
    let session = machine.get(Agent::Claude, "s").expect("created");
    assert_eq!(session.phase, SessionPhase::Idle);
    assert_eq!(session.last_activity_ms, 10);
    assert_eq!(session.waiting_since_ms, None);
}

#[test]
fn fsm_lazy_create_follows_the_event() {
    let mut working = SessionMachine::new();
    working.apply(&event(EventKind::Working, 1));
    assert_eq!(working.sessions()[0].phase, SessionPhase::Working);

    let mut waiting = SessionMachine::new();
    waiting.apply(&event(EventKind::Waiting, 2));
    assert_eq!(waiting.sessions()[0].phase, SessionPhase::Waiting);
    assert_eq!(waiting.sessions()[0].waiting_since_ms, Some(2));

    let mut idle = SessionMachine::new();
    idle.apply(&event(EventKind::Idle, 3));
    assert_eq!(idle.sessions()[0].phase, SessionPhase::Idle);
}

#[test]
fn fsm_idle_to_working() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Start, 0));
    assert_eq!(
        machine.apply(&event(EventKind::Working, 5)),
        ApplyOutcome::Updated
    );
    assert_eq!(phase(&machine), SessionPhase::Working);
    assert_eq!(machine.sessions()[0].last_activity_ms, 5);
}

#[test]
fn fsm_working_heartbeat_refreshes_activity_and_ttl() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    assert_eq!(
        machine.apply(&event(EventKind::Working, 500_000)),
        ApplyOutcome::Updated
    );
    assert_eq!(phase(&machine), SessionPhase::Working);
    assert_eq!(machine.sessions()[0].last_activity_ms, 500_000);
    let early = machine.tick(600_000, TTL_MS, SESSION_EXPIRY_MS);
    assert!(early.finished.is_empty());
    assert_eq!(phase(&machine), SessionPhase::Working);
    let fired = machine.tick(500_000 + TTL_MS, TTL_MS, SESSION_EXPIRY_MS);
    assert_eq!(fired.finished.len(), 1);
    assert_eq!(phase(&machine), SessionPhase::Idle);
}

#[test]
fn fsm_working_to_waiting_records_waiting_since() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    machine.apply(&event(EventKind::Waiting, 40));
    let session = machine.get(Agent::Claude, "s").expect("session");
    assert_eq!(session.phase, SessionPhase::Waiting);
    assert_eq!(session.waiting_since_ms, Some(40));
}

#[test]
fn fsm_waiting_to_working_clears_wait() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    machine.apply(&event(EventKind::Waiting, 40));
    machine.apply(&event(EventKind::Working, 80));
    let session = machine.get(Agent::Claude, "s").expect("session");
    assert_eq!(session.phase, SessionPhase::Working);
    assert_eq!(session.waiting_since_ms, None);
    assert_eq!(session.last_activity_ms, 80);
}

#[test]
fn fsm_repeated_waiting_keeps_waiting_since() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    machine.apply(&event(EventKind::Waiting, 40));
    machine.apply(&event(EventKind::Waiting, 90));
    let session = machine.get(Agent::Claude, "s").expect("session");
    assert_eq!(session.phase, SessionPhase::Waiting);
    assert_eq!(session.waiting_since_ms, Some(40));
    assert_eq!(session.last_activity_ms, 90);
}

#[test]
fn fsm_waiting_from_idle_is_ignored() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Start, 10));
    assert_eq!(
        machine.apply(&event(EventKind::Waiting, 50)),
        ApplyOutcome::Ignored
    );
    let session = machine.get(Agent::Claude, "s").expect("session");
    assert_eq!(session.phase, SessionPhase::Idle);
    assert_eq!(session.last_activity_ms, 10);
    assert_eq!(session.waiting_since_ms, None);
}

#[test]
fn fsm_working_or_waiting_to_idle_finishes() {
    let mut working = SessionMachine::new();
    working.apply(&event(EventKind::Working, 0));
    assert_eq!(
        working.apply(&event(EventKind::Idle, 15)),
        ApplyOutcome::Finished
    );
    assert_eq!(phase(&working), SessionPhase::Idle);

    let mut waiting = SessionMachine::new();
    waiting.apply(&event(EventKind::Working, 0));
    waiting.apply(&event(EventKind::Waiting, 5));
    assert_eq!(
        waiting.apply(&event(EventKind::Idle, 15)),
        ApplyOutcome::Finished
    );
    assert_eq!(phase(&waiting), SessionPhase::Idle);
    assert_eq!(waiting.sessions()[0].waiting_since_ms, None);
}

#[test]
fn fsm_end_removes_any_phase_and_ignores_unknown() {
    let mut machine = SessionMachine::new();
    assert_eq!(
        machine.apply(&event(EventKind::End, 1)),
        ApplyOutcome::Ignored
    );
    machine.apply(&event(EventKind::Working, 0));
    assert_eq!(
        machine.apply(&event(EventKind::End, 9)),
        ApplyOutcome::Removed
    );
    assert!(machine.sessions().is_empty());
}

#[test]
fn fsm_working_ttl_becomes_idle_at_the_boundary() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    let still = machine.tick(TTL_MS - 1, TTL_MS, SESSION_EXPIRY_MS);
    assert!(still.finished.is_empty());
    assert_eq!(phase(&machine), SessionPhase::Working);
    let fired = machine.tick(TTL_MS, TTL_MS, SESSION_EXPIRY_MS);
    assert_eq!(fired.finished[0].session_id, "s");
    assert_eq!(phase(&machine), SessionPhase::Idle);
    assert_eq!(machine.sessions()[0].last_activity_ms, TTL_MS);
}

#[test]
fn fsm_idle_expiry_prunes_at_24h_and_leaves_fresh_idle() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Idle, 0));
    let kept = machine.tick(SESSION_EXPIRY_MS - 1, TTL_MS, SESSION_EXPIRY_MS);
    assert!(kept.pruned.is_empty());
    let dropped = machine.tick(SESSION_EXPIRY_MS, TTL_MS, SESSION_EXPIRY_MS);
    assert_eq!(dropped.pruned.len(), 1);
    assert!(machine.sessions().is_empty());
}

#[test]
fn fsm_reap_removes_only_dead_pids() {
    let mut machine = SessionMachine::new();
    machine.apply(&Event {
        pid: Some(1),
        session_id: "alive".to_owned(),
        ..event(EventKind::Working, 0)
    });
    machine.apply(&Event {
        agent: Agent::Cursor,
        pid: Some(2),
        session_id: "dead".to_owned(),
        ..event(EventKind::Working, 0)
    });
    machine.apply(&Event {
        agent: Agent::Codex,
        pid: None,
        session_id: "untracked".to_owned(),
        ..event(EventKind::Idle, 0)
    });
    let removed = machine.reap_dead_pids(|pid| pid != 2);
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].session_id, "dead");
    assert!(machine.get(Agent::Claude, "alive").is_some());
    assert!(machine.get(Agent::Codex, "untracked").is_some());
    assert!(machine.get(Agent::Cursor, "dead").is_none());
}

#[test]
fn fsm_same_id_on_two_agents_are_distinct() {
    let mut machine = SessionMachine::new();
    machine.apply(&event(EventKind::Working, 0));
    machine.apply(&Event {
        agent: Agent::Cursor,
        ..event(EventKind::Idle, 0)
    });
    assert_eq!(machine.sessions().len(), 2);
    machine.apply(&event(EventKind::End, 1));
    assert!(machine.get(Agent::Claude, "s").is_none());
    assert_eq!(
        machine.get(Agent::Cursor, "s").expect("cursor").phase,
        SessionPhase::Idle
    );
}

#[test]
fn precedence_cutoff_beats_reachable_and_always() {
    let cfg = policy();
    let power = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: Some(10.0),
        temp_c: None,
        lid_closed: None,
    };
    let sessions = [];
    let mut reachable = input(
        0,
        Mode::WhileWorking,
        &sessions,
        &power,
        CutoffState::default(),
        &cfg,
    );
    reachable.reachable_until_ms = Some(60_000);
    let decision = decide(&reachable);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::CutoffBattery);
    assert!(decision.new_cutoff.battery_latched);

    let mut always = input(
        0,
        Mode::Always,
        &sessions,
        &power,
        CutoffState::default(),
        &cfg,
    );
    always.reachable_until_ms = Some(i64::MAX);
    let decision = decide(&always);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::CutoffBattery);
}

#[test]
fn precedence_cutoff_spares_reachable_and_always_when_flag_is_false() {
    let mut cfg = policy();
    cfg.apply_to_reachable = false;
    let power = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: Some(10.0),
        ..plugged()
    };
    let sessions = [];
    let mut reachable = input(
        0,
        Mode::WhileWorking,
        &sessions,
        &power,
        CutoffState::default(),
        &cfg,
    );
    reachable.reachable_until_ms = Some(60_000);
    let decision = decide(&reachable);
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::Reachable);
    assert!(decision.new_cutoff.battery_latched);

    let always = input(
        0,
        Mode::Always,
        &sessions,
        &power,
        decision.new_cutoff,
        &cfg,
    );
    let decision = decide(&always);
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::AlwaysMode);
}

#[test]
fn precedence_pause_and_mode_off_release_before_reachable() {
    let cfg = policy();
    let power = plugged();
    let sessions = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Working,
        waiting_since_ms: None,
    }];
    let mut paused = input(
        1_000,
        Mode::Always,
        &sessions,
        &power,
        CutoffState::default(),
        &cfg,
    );
    paused.paused_until_ms = Some(50_000);
    paused.reachable_until_ms = Some(80_000);
    let decision = decide(&paused);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::Paused);
    assert_eq!(decision.next_deadline_ms, Some(50_000));

    let mut off = input(
        0,
        Mode::Off,
        &sessions,
        &power,
        CutoffState::default(),
        &cfg,
    );
    off.reachable_until_ms = Some(80_000);
    let decision = decide(&off);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::ModeOff);
}

#[test]
fn precedence_reachable_then_always_then_while_working() {
    let cfg = policy();
    let power = plugged();
    let idle = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Idle,
        waiting_since_ms: None,
    }];
    let mut reachable = input(
        0,
        Mode::WhileWorking,
        &idle,
        &power,
        CutoffState::default(),
        &cfg,
    );
    reachable.reachable_until_ms = Some(7_200_000);
    let decision = decide(&reachable);
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::Reachable);
    assert_eq!(decision.next_deadline_ms, Some(7_200_000));

    let always = input(0, Mode::Always, &idle, &power, CutoffState::default(), &cfg);
    let decision = decide(&always);
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::AlwaysMode);
    assert_eq!(decision.next_deadline_ms, None);

    let working = [
        coffeegent_core::session::SessionView {
            phase: SessionPhase::Working,
            waiting_since_ms: None,
        },
        coffeegent_core::session::SessionView {
            phase: SessionPhase::Working,
            waiting_since_ms: None,
        },
        coffeegent_core::session::SessionView {
            phase: SessionPhase::Idle,
            waiting_since_ms: None,
        },
    ];
    let decision = decide(&input(
        0,
        Mode::WhileWorking,
        &working,
        &power,
        CutoffState::default(),
        &cfg,
    ));
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::Working(2));
}

#[test]
fn precedence_waiting_grace_holds_then_expires() {
    let cfg = policy();
    let power = plugged();
    let waiting = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Waiting,
        waiting_since_ms: Some(1_000),
    }];
    let inside = decide(&input(
        1_000 + GRACE_MS - 1,
        Mode::WhileWorking,
        &waiting,
        &power,
        CutoffState::default(),
        &cfg,
    ));
    assert!(inside.hold);
    assert_eq!(inside.reason, HoldReason::WaitingGrace(1));
    assert_eq!(inside.next_deadline_ms, Some(1_000 + GRACE_MS));

    let edge = decide(&input(
        1_000 + GRACE_MS,
        Mode::WhileWorking,
        &waiting,
        &power,
        CutoffState::default(),
        &cfg,
    ));
    assert!(!edge.hold);
    assert_eq!(edge.reason, HoldReason::NothingToDo);
}

#[test]
fn precedence_idle_is_nothing_to_do() {
    let cfg = policy();
    let power = plugged();
    let idle = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Idle,
        waiting_since_ms: None,
    }];
    let decision = decide(&input(
        0,
        Mode::WhileWorking,
        &idle,
        &power,
        CutoffState::default(),
        &cfg,
    ));
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::NothingToDo);
}

#[test]
fn pause_and_reachable_last_command_wins() {
    let mut timers = ActivityTimers::default();
    timers.pause(30);
    timers.reachable(60);
    assert_eq!(timers.paused_until_ms, None);
    assert_eq!(timers.reachable_until_ms, Some(60));
    timers.pause(90);
    assert_eq!(timers.paused_until_ms, Some(90));
    assert_eq!(timers.reachable_until_ms, None);
    timers.resume();
    assert_eq!(timers.paused_until_ms, None);
}

#[test]
fn battery_triggers_at_threshold_only_while_discharging() {
    let cfg = policy();
    let sessions = [];
    let above = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: Some(20.1),
        temp_c: None,
        lid_closed: None,
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &above,
        CutoffState::default(),
        &cfg,
    ));
    assert!(!decision.new_cutoff.battery_latched);
    assert!(decision.hold);

    let at = PowerSnapshot {
        battery_percent: Some(20.0),
        ..above
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &at,
        CutoffState::default(),
        &cfg,
    ));
    assert!(decision.new_cutoff.battery_latched);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::CutoffBattery);
}

#[test]
fn battery_clears_on_ac_and_at_plus_five_percent() {
    let cfg = policy();
    let sessions = [];
    let latched = CutoffState {
        battery_latched: true,
        ..CutoffState::default()
    };
    let still_low = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: Some(24.0),
        temp_c: None,
        lid_closed: None,
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &still_low,
        latched.clone(),
        &cfg,
    ));
    assert!(decision.new_cutoff.battery_latched);

    let recovered = PowerSnapshot {
        battery_percent: Some(25.0),
        ..still_low
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &recovered,
        latched.clone(),
        &cfg,
    ));
    assert!(!decision.new_cutoff.battery_latched);
    assert!(decision.hold);

    let on_ac = PowerSnapshot {
        discharging: false,
        battery_percent: Some(10.0),
        ..still_low
    };
    let decision = decide(&input(0, Mode::Always, &sessions, &on_ac, latched, &cfg));
    assert!(!decision.new_cutoff.battery_latched);
    assert_eq!(decision.reason, HoldReason::AlwaysMode);
}

#[test]
fn not_charging_at_100_percent_is_not_discharging() {
    // UPower "Not charging" must be passed in as discharging=false. Percent
    // alone, including 100% or a low charge-limit, never closes the latch.
    let cfg = policy();
    let sessions = [];
    for percent in [100.0, 10.0] {
        let power = PowerSnapshot {
            battery_present: true,
            discharging: false,
            battery_percent: Some(percent),
            temp_c: None,
            lid_closed: None,
        };
        let decision = decide(&input(
            0,
            Mode::Always,
            &sessions,
            &power,
            CutoffState::default(),
            &cfg,
        ));
        assert!(
            !decision.new_cutoff.battery_latched,
            "percent {percent} while not discharging must not latch"
        );
        assert!(decision.hold);
    }
}

#[test]
fn battery_without_a_percent_or_a_battery_does_not_trigger() {
    let cfg = policy();
    let sessions = [];
    let missing = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: None,
        temp_c: None,
        lid_closed: None,
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &missing,
        CutoffState::default(),
        &cfg,
    ));
    assert!(!decision.new_cutoff.battery_latched);

    let desktop = PowerSnapshot {
        battery_present: false,
        discharging: true,
        battery_percent: Some(1.0),
        ..missing
    };
    let decision = decide(&input(
        0,
        Mode::Always,
        &sessions,
        &desktop,
        CutoffState::default(),
        &cfg,
    ));
    assert!(!decision.new_cutoff.battery_latched);
}

#[test]
fn thermal_window_trips_and_a_single_spike_does_not() {
    let cfg = policy();
    let sustained_idle_spike = feed_temp(20, 78.0, Some(true), &cfg);
    assert!(
        !sustained_idle_spike.thermal_latched,
        "78 C is the observed idle spike and is under 85 C"
    );

    let mut cutoff = CutoffState::default();
    let sessions = [];
    let cool = PowerSnapshot {
        temp_c: Some(60.0),
        lid_closed: Some(true),
        ..plugged()
    };
    let hot = PowerSnapshot {
        temp_c: Some(96.0),
        lid_closed: Some(true),
        ..plugged()
    };
    for _ in 0..19 {
        cutoff = decide(&input(0, Mode::Always, &sessions, &cool, cutoff, &cfg)).new_cutoff;
    }
    cutoff = decide(&input(0, Mode::Always, &sessions, &hot, cutoff, &cfg)).new_cutoff;
    assert!(!cutoff.thermal_latched);

    let hot_window = feed_temp(20, 85.0, Some(true), &cfg);
    assert!(hot_window.thermal_latched);
}

#[test]
fn thermal_rearms_only_after_a_full_cool_window() {
    let cfg = policy();
    let sessions = [];
    let mut cutoff = feed_temp(20, 90.0, Some(true), &cfg);
    assert!(cutoff.thermal_latched);
    let cool = PowerSnapshot {
        temp_c: Some(69.0),
        lid_closed: Some(true),
        ..plugged()
    };
    for _ in 0..29 {
        let decision = decide(&input(0, Mode::Always, &sessions, &cool, cutoff, &cfg));
        cutoff = decision.new_cutoff;
        assert!(cutoff.thermal_latched);
        assert!(!decision.hold);
    }
    let decision = decide(&input(0, Mode::Always, &sessions, &cool, cutoff, &cfg));
    assert!(!decision.new_cutoff.thermal_latched);
    assert!(decision.hold);
}

#[test]
fn thermal_only_when_lid_closed_closed_open_and_unknown() {
    let cfg = policy();
    assert!(feed_temp(20, 90.0, Some(true), &cfg).thermal_latched);
    assert!(!feed_temp(20, 90.0, Some(false), &cfg).thermal_latched);
    assert!(
        feed_temp(20, 90.0, None, &cfg).thermal_latched,
        "no lid sensor still evaluates"
    );

    let mut open_cfg = policy();
    open_cfg.thermal_only_when_lid_closed = false;
    assert!(feed_temp(20, 90.0, Some(false), &open_cfg).thermal_latched);

    let latched = feed_temp(20, 90.0, Some(true), &cfg);
    let power = PowerSnapshot {
        temp_c: Some(90.0),
        lid_closed: Some(false),
        ..plugged()
    };
    let sessions = [];
    let decision = decide(&input(0, Mode::Always, &sessions, &power, latched, &cfg));
    assert!(decision.hold, "an open lid does not apply a thermal latch");
    assert!(decision.new_cutoff.thermal_latched);
    assert_eq!(decision.reason, HoldReason::AlwaysMode);
}

#[test]
fn thermal_reading_missing_does_not_trigger() {
    let cfg = policy();
    let power = PowerSnapshot {
        temp_c: None,
        lid_closed: Some(true),
        ..plugged()
    };
    let sessions = [];
    let mut cutoff = CutoffState::default();
    for _ in 0..30 {
        let decision = decide(&input(0, Mode::Always, &sessions, &power, cutoff, &cfg));
        cutoff = decision.new_cutoff;
        assert!(decision.hold);
    }
    assert!(!cutoff.thermal_latched);
    assert!(cutoff.samples_c.is_empty());
}

#[test]
fn min_hold_delays_work_grace_and_reachable_end_only() {
    let cfg = policy();
    let power = plugged();
    let idle = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Idle,
        waiting_since_ms: None,
    }];
    let waiting_done = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Waiting,
        waiting_since_ms: Some(0),
    }];

    let mut work_ended = input(
        4_999,
        Mode::WhileWorking,
        &idle,
        &power,
        CutoffState::default(),
        &cfg,
    );
    work_ended.hold_since_ms = Some(0);
    work_ended.held_reason = Some(HoldReason::Working(1));
    let decision = decide(&work_ended);
    assert!(decision.hold);
    assert_eq!(decision.reason, HoldReason::MinHold);
    assert_eq!(decision.next_deadline_ms, Some(5_000));

    // Grace has elapsed, but the hold is still inside the 5 s acquire window.
    let mut grace_ended = input(
        GRACE_MS + 4_999,
        Mode::WhileWorking,
        &waiting_done,
        &power,
        CutoffState::default(),
        &cfg,
    );
    grace_ended.hold_since_ms = Some(GRACE_MS);
    grace_ended.held_reason = Some(HoldReason::WaitingGrace(1));
    assert_eq!(decide(&grace_ended).reason, HoldReason::MinHold);

    let mut reachable_ended = input(
        4_999,
        Mode::WhileWorking,
        &idle,
        &power,
        CutoffState::default(),
        &cfg,
    );
    reachable_ended.hold_since_ms = Some(0);
    reachable_ended.held_reason = Some(HoldReason::Reachable);
    assert_eq!(decide(&reachable_ended).reason, HoldReason::MinHold);

    let mut released = work_ended;
    released.now_ms = 5_000;
    let decision = decide(&released);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::NothingToDo);

    let mut left_always = input(
        1_000,
        Mode::WhileWorking,
        &idle,
        &power,
        CutoffState::default(),
        &cfg,
    );
    left_always.hold_since_ms = Some(0);
    left_always.held_reason = Some(HoldReason::AlwaysMode);
    let decision = decide(&left_always);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::NothingToDo);
}

#[test]
fn min_hold_does_not_delay_cutoff_pause_or_mode_off() {
    let cfg = policy();
    let sessions = [];
    let low = PowerSnapshot {
        battery_present: true,
        discharging: true,
        battery_percent: Some(5.0),
        temp_c: None,
        lid_closed: None,
    };
    let mut cutoff = input(
        1_000,
        Mode::WhileWorking,
        &sessions,
        &low,
        CutoffState::default(),
        &cfg,
    );
    cutoff.hold_since_ms = Some(0);
    cutoff.held_reason = Some(HoldReason::Working(1));
    let decision = decide(&cutoff);
    assert!(!decision.hold);
    assert_eq!(decision.reason, HoldReason::CutoffBattery);

    let power = plugged();
    let working = [coffeegent_core::session::SessionView {
        phase: SessionPhase::Working,
        waiting_since_ms: None,
    }];
    let mut paused = input(
        1_000,
        Mode::WhileWorking,
        &working,
        &power,
        CutoffState::default(),
        &cfg,
    );
    paused.paused_until_ms = Some(9_000);
    paused.hold_since_ms = Some(0);
    paused.held_reason = Some(HoldReason::Working(1));
    assert_eq!(decide(&paused).reason, HoldReason::Paused);

    let mut off = input(
        1_000,
        Mode::Off,
        &working,
        &power,
        CutoffState::default(),
        &cfg,
    );
    off.hold_since_ms = Some(0);
    off.held_reason = Some(HoldReason::Working(1));
    assert_eq!(decide(&off).reason, HoldReason::ModeOff);
}

#[test]
fn saved_mode_wins_over_config_mode() {
    assert_eq!(effective_mode(Some(Mode::Off), Mode::Always), Mode::Off);
    assert_eq!(
        effective_mode(Some(Mode::Always), Mode::WhileWorking),
        Mode::Always
    );
    assert_eq!(effective_mode(None, Mode::Always), Mode::Always);
    assert_eq!(Mode::Always.toggle(), Mode::Off);
    assert_eq!(Mode::Off.toggle(), Mode::WhileWorking);
    assert_eq!(Mode::WhileWorking.toggle(), Mode::Off);
}

#[test]
fn duration_parses_humantime_tomorrow_off_and_indefinite() {
    assert_eq!(
        duration::parse_timer("30m").expect("30m"),
        TimerToken::After(Duration::from_secs(30 * 60))
    );
    assert_eq!(
        duration::parse_timer("1h").expect("1h"),
        TimerToken::After(Duration::from_secs(60 * 60))
    );
    assert_eq!(
        duration::parse_timer("90s").expect("90s"),
        TimerToken::After(Duration::from_secs(90))
    );
    assert_eq!(duration::parse_timer("Off").expect("off"), TimerToken::Off);
    assert_eq!(
        duration::parse_timer("indefinite").expect("indefinite"),
        TimerToken::Indefinite
    );
    assert!(duration::parse_timer("nope").is_err());

    let tomorrow = duration::parse_timer("tomorrow").expect("tomorrow");
    let at = TimeOfDay::parse("06:00").expect("06:00");
    // 1970-01-01 12:00:00 UTC -> 1970-01-02 06:00:00 UTC.
    let now_ms = 12 * 3_600 * 1_000;
    let resolved = duration::resolve_timer(&tomorrow, now_ms, at, 0).expect("resolve");
    assert_eq!(resolved.as_until_ms(), Some((86_400 + 6 * 3_600) * 1_000));
    assert_eq!(
        duration::resolve_timer(&TimerToken::Indefinite, now_ms, at, 0)
            .expect("indefinite")
            .as_until_ms(),
        Some(i64::MAX)
    );
    assert_eq!(
        duration::resolve_timer(&TimerToken::Off, now_ms, at, 0)
            .expect("off")
            .as_until_ms(),
        None
    );
}

#[test]
fn config_defaults_sample_and_unknown_keys() {
    let parsed = config::parse_config("").expect("empty");
    assert!(parsed.warnings.is_empty());
    assert_eq!(parsed.config.mode, Mode::WhileWorking);
    assert_eq!(parsed.config.timing.working_ttl_secs, 600);
    assert_eq!(parsed.config.timing.waiting_grace_secs, 300);
    assert_eq!(parsed.config.timing.min_hold_secs, 5);
    assert_eq!(parsed.config.cutoff.battery_percent, 20.0);
    assert_eq!(parsed.config.cutoff.thermal_c, 85.0);
    assert_eq!(parsed.config.cutoff.thermal_rearm_c, 70.0);
    assert!(parsed.config.cutoff.thermal_only_when_lid_closed);
    assert!(!parsed.config.agents.codex);

    let sample = r#"
mode = "always"

[timing]
waiting_grace_secs = 300
working_ttl_secs   = 600
min_hold_secs      = 5
tomorrow_at        = "06:00"

[cutoff]
battery_percent            = 20
thermal_c                  = 85
thermal_window_secs        = 20
thermal_rearm_c            = 70
thermal_rearm_secs         = 30
thermal_only_when_lid_closed = true
apply_to_reachable         = true

[lid]
lid_inhibit   = "when_holding"
lock_on_close = true

[thermal]
sensor_types = ["x86_pkg_temp"]
ignore_types = ["INT3400", "SEN1", "SEN3"]

[notifications]
waiting   = true
finished  = true
finished_min_work_secs = 30
cooldown_secs = 10

[agents]
claude = true
cursor = true
codex  = false
"#;
    let parsed = config::parse_config(sample).expect("sample");
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    assert_eq!(parsed.config.mode, Mode::Always);
    assert_eq!(
        parsed.config.thermal.sensor_types,
        vec!["x86_pkg_temp".to_owned()]
    );
    assert_eq!(parsed.config.policy().waiting_grace_ms, 300_000);

    let messy = "mode = \"while-working\"\nextra = 1\n\n[timing]\nwaiting_grace_secs = 12\nmystery = true\n";
    let parsed = config::parse_config(messy).expect("warnings");
    assert_eq!(parsed.config.mode, Mode::WhileWorking);
    assert_eq!(parsed.config.timing.waiting_grace_secs, 12);
    assert!(parsed.warnings.iter().any(|w| w.contains("`extra`")));
    assert!(parsed.warnings.iter().any(|w| w.contains("timing.mystery")));
    assert!(config::parse_config("mode = \"sometimes\"\n").is_err());
}
