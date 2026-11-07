# coffeegent: Specification v0.2

> Status: **v0.2 = v0.1 draft + Phase 0 findings.** Phase 0 was stopped early by the maintainer; its remaining unknowns are now **gates** inside the milestones (section 15), to be resolved while building.
>
> Evidence tags: **OBSERVED** (measured on the maintainer's laptop), **DOCUMENTED** (from docs, not tested), **[VERIFY]** (still unconfirmed; the code agent must stop and ask the human instead of working around it).

---

## 0. Phase 0 outcomes (authoritative facts)

Target machine: ASUS laptop, Pop!_OS 24.04, COSMIC on Wayland, systemd 255, logind with default settings, Rust 1.95.

**OBSERVED**
1. A **user-space, root-less** logind lock `sleep:idle:handle-lid-switch` (mode `block`) keeps the laptop awake with the lid closed, on AC and on battery (runs of 61-139 s, screen locked and unlocked). No polkit prompt.
2. **`handle-lid-switch` is mandatory.** With only `sleep:idle` the laptop suspended on lid close. The lid action on COSMIC is logind's own (journal: "Lid closed" then "Suspending...").
3. Lock lifetime follows the file descriptor: after SIGTERM and after SIGKILL the lock was gone within 1 s. **No watchdog or stale-lock handling is needed.**
4. **Closing the lid does not lock the screen** while our lock is held. logind `LockedHint` stays `no` even when the COSMIC screen is locked, so it must not be used to detect lock state.
5. Baseline: nobody else held `handle-lid-switch` or `idle`. `cosmic-comp` contains "Inhibiting lid switch" strings (see D1).
6. Power: `BAT0` plus `AC0` (mains); a separate USB-C source device (`ucsi-source-psy`) exists and was offline. sysfs shows "Not charging" at 100% on AC. UPower CLI text uses locale decimal commas.
7. Thermal: zones `acpitz`, `TCPU`, `x86_pkg_temp`, `SEN2`, `SEN4` are useful; `INT3400`, `SEN1`, `SEN3` are constant; NVMe "Sensor 2" is constant (unusable); no GPU temp via sysfs. 8-core load on AC: package 53 -> 91 C in 10 s, plateau 91-93 C ~58 s, firmware step-down to ~73 C. Idle spikes of 70-78 C occur. Cooldown: 50 C after 15 s. Closed-lid idle: 6-7 W, 48-58 C.
8. zbus **6.0.0**: async API with `zbus::block_on`, no `blocking-api` feature; use `zbus::OwnedFd` (the `zvariant::OwnedFd` path is deprecated). `signal-hook` 0.4.5 worked for signals. Reference implementation: `docs/reference/inhibit_spike.rs` (~30 lines; copy the maintainer's `main.rs` there).
9. COSMIC: `org.kde.StatusNotifierWatcher` is owned by `cosmic-applet-status-area`. `cosmic-idle` owns idle timers (its suspend/screen-off settings were `None` on this machine).

**DOCUMENTED (not tested)**
- Hook mechanisms for Claude Code, Cursor and Codex (section 9).
- Upstream `pop-os/cosmic-comp#19`: no suspend on lid close with external monitors, suggesting COSMIC holds its own lid lock when a monitor is attached.
- After a real resume, logind ignores lid events for 30 s (`HoldoffTimeoutSec`); NetworkManager takes Wi-Fi down on suspend.
- UPower's critical-action is HybridSleep.

**Not tested** (see gates and open items): external display, low-battery behavior, closed-lid runs longer than ~2 min, lock-session command, panel physical state, live agent hook payloads, tray, autostart.

---

## 1. Summary

**coffeegent** is a Linux user-level daemon that keeps the machine awake, **including with the laptop lid closed**, only while AI coding agents are actively working, and releases automatically when they are idle, when the battery is low, or when the machine is too hot.

- Name: `coffeegent` (coffee + agent). Binary: `coffeegent`. License: **MIT**, open source on GitHub.
- Language: **Rust** (stable via rustup, edition 2021). The author is new to Rust: prefer simple, readable code with short "why" comments.
- Primary target: **Pop!_OS 24.04 + COSMIC (Wayland)**. Secondary: GNOME (Wayland), KDE Plasma. Everything must degrade gracefully without tray support.
- **v0.1 agents: Claude Code and Cursor** (hooks). **Codex is deferred to M7** (not installed on the dev machine, and needs a fallback signal).

Clean-room note: inspired by the behavior described on a commercial macOS product's marketing page. Do not copy its name, artwork or text.

## 2. Goals and non-goals

### Goals (v0.1)
1. Hold a systemd-logind inhibitor while any tracked agent session is working (or waiting within a grace period).
2. Keep running with the lid closed, root-less, using `handle-lid-switch`.
3. **Lock the screen when the lid closes while we hold the lock** (security; section 6.1).
4. Safety cutoffs: low battery and high temperature.
5. Manual controls: pause, stay-reachable timer, mode switch, via CLI and tray.
6. Notifications: "agent waiting for you", "agent finished", "cutoff".
7. Crash safety: dead daemon means lock released by the OS; dead agent means its session expires.
8. Diagnosable: `coffeegent doctor`.

### Non-goals (v0.1)
- Codex support; heuristic detection of arbitrary agents; other agents.
- Display control (the panel's physical behavior with the lid closed is unknown; see open items).
- Handling docked-with-external-monitor semantics beyond documenting them.
- Preferences GUI, automatic hotkey registration, Flatpak/Snap, X11-specific features, wake-on-LAN/RTC wake, non-Linux platforms.
- Any network access: no telemetry, updater or license check.

## 3. Architecture

```
 agent hooks ─▶ `coffeegent hook <agent>` ─▶ Unix socket ─▶ ┌─────────────────────────┐
 (Claude/Cursor)                                            │ coffeegent daemon       │
 optional file-activity detector (M7) ──────────────────────▶ │  session registry+FSM   │
                                                            │  policy engine (pure)   │
 `coffeegent <cmd>` CLI ◀──────────── Unix socket ─────────▶ │  inhibitor (logind fd)  │
                                                            │  power: UPower + sysfs  │
 tray (SNI) + notifications ◀──────────────────────────────▶ │  thermal: sysfs         │
                                                            └─────────────────────────┘
```

### 3.1 Process model
- **One binary** `coffeegent` with subcommands. One daemon per user session, **single-instance**: if the socket is live, a second `coffeegent daemon` logs "already running" and exits 0 (needed because both systemd and XDG autostart may be present).
- The `hook` subcommand path uses **only `std`** (no tokio, no D-Bus): connect, write one JSON line, exit. Budget **<= 150 ms** total, connect timeout 50 ms, **always exit 0**, print nothing to stdout, never block an agent.
- The daemon is async (tokio). All OS access sits behind traits so logic is testable (section 11).

### 3.2 Workspace layout
```
coffeegent/
├─ Cargo.toml                 # workspace
├─ LICENSE  README.md  CONTRIBUTING.md  CHANGELOG.md  SPEC.md
├─ crates/
│  ├─ coffeegent-core/        # pure logic, #![forbid(unsafe_code)], no I/O, no tokio
│  │   ├─ src/{lib,types,session,policy,config,duration}.rs
│  │   └─ tests/
│  └─ coffeegent/             # the binary
│      ├─ src/main.rs  cli.rs  hook.rs  doctor.rs
│      ├─ src/adapters/{mod,claude,cursor}.rs      # codex.rs arrives in M7
│      ├─ src/daemon/{mod,ipc,runtime}.rs
│      ├─ src/backends/{logind,upower,thermal,fake}.rs
│      ├─ src/ui/{tray,notify}.rs
│      └─ src/install/{mod,claude,cursor}.rs
├─ packaging/{systemd/coffeegent.service, autostart/coffeegent.desktop, icons/, deb/}
├─ tests/fixtures/<agent>/*.json        # sanitized REAL payloads (gate G1)
├─ docs/{agents.md, desktops.md, hotkeys.md, troubleshooting.md, manual-test.md, ideas.md}
├─ docs/reference/inhibit_spike.rs      # the working Phase 0 spike
└─ .github/workflows/ci.yml
```

### 3.3 Dependencies (verify APIs against current docs when scaffolding)
`tokio`, **`zbus` 6.x** (see section 0, item 8), `ksni` (tray) [VERIFY compatibility with the chosen runtime], `serde`, `serde_json`, `toml`, `toml_edit`, `clap` (derive), `humantime`, `tracing` + `tracing-subscriber`, `anyhow`, `thiserror`, `notify-rust` or direct zbus for notifications, `nix` (peer credentials, signals; `signal-hook` is acceptable if tokio signals are inconvenient), `directories` or manual XDG. M7 only: `notify` (inotify). Dev: `tempfile`, `assert_cmd`, `proptest` (optional). Keep the dependency count small; justify additions.

## 4. Domain model

### 4.1 Agents
`enum Agent { Claude, Cursor, Codex }` (lowercase on the wire). `Codex` exists in the enum from M0 but has no adapter or installer until M7; `agents.codex` in config defaults to `false`.

### 4.2 Events
```rust
pub enum EventKind { Start, Working, Waiting, Idle, End }
pub struct Event {
    pub agent: Agent,
    pub session_id: String,       // agent-provided; if missing, "pid:<pid>"
    pub kind: EventKind,
    pub ts_ms: i64,               // assigned by the daemon on receipt
    pub pid: Option<u32>,         // agent process pid if determinable (hook's ppid chain)
    pub cwd: Option<String>,      // path only; never prompt content
}
```
`Working` doubles as a **heartbeat**.

### 4.3 Session state machine
States: `Working`, `Waiting`, `Idle`. (`End` removes the session.)

| Current | Event | Next | Notes |
|---|---|---|---|
| (none) | Start | Idle | create |
| (none) | Working/Waiting/Idle | per event | create lazily (hooks may be installed mid-session) |
| Idle | Working | Working | |
| Working | Working | Working | refresh `last_activity` |
| Working | Waiting | Waiting | record `waiting_since` |
| Waiting | Working | Working | user answered |
| Waiting | Waiting | Waiting | `waiting_since` unchanged |
| Idle | Waiting | Idle | **ignored**: waiting only counts if work was in progress (neutralizes idle-prompt notifications) |
| Working/Waiting | Idle | Idle | emits "finished" transition |
| any | End | removed | |

Timers (evaluated each 1 s tick):
- **working_ttl** (default **600 s**): a `Working` session with no event for this long becomes `Idle`. This is the guard for **lost or never-sent Stop events** (for example, a user interrupt may not fire Stop [VERIFY in G1]). Rationale for 600 over the report's provisional 120: a long shell command can run for minutes between PreToolUse and PostToolUse with no events, and a too-short TTL would let the laptop suspend mid-task. **Tune in G1** from measured event gaps.
- **waiting_grace** (default 300 s): a `Waiting` session stops contributing to the hold after this long (it stays `Waiting` for display).
- **liveness**: every 10 s, if a session has a `pid` and `/proc/<pid>` is gone, remove the session.
- **session_expiry**: `Idle` sessions untouched for 24 h are pruned.

## 5. Policy engine (`coffeegent-core::policy`)

Pure function; no clock or I/O; time injected as `i64` unix millis.

```rust
pub fn decide(input: &PolicyInput) -> Decision;

pub struct PolicyInput<'a> {
    pub now_ms: i64,
    pub mode: Mode,                       // WhileWorking | Always | Off
    pub paused_until_ms: Option<i64>,     // i64::MAX = indefinite
    pub reachable_until_ms: Option<i64>,
    pub sessions: &'a [SessionView],
    pub power: &'a PowerSnapshot,
    pub cutoff: CutoffState,              // latched state + thermal sample window, owned by caller
    pub cfg: &'a PolicyConfig,
}
pub struct PowerSnapshot {
    pub battery_present: bool,
    pub discharging: bool,                // true ONLY for state "Discharging"
    pub battery_percent: Option<f32>,
    pub temp_c: Option<f32>,              // one sample per second from the selected sensor(s)
    pub lid_closed: Option<bool>,         // None = no lid / unknown
}
pub struct Decision {
    pub hold: bool,
    pub reason: HoldReason,
    pub new_cutoff: CutoffState,
    pub next_deadline_ms: Option<i64>,
}
pub enum HoldReason { Working(n), WaitingGrace(n), Reachable, AlwaysMode,
                      Paused, ModeOff, CutoffBattery, CutoffThermal, NothingToDo }
```

**Precedence (highest first):**
1. **Cutoff latched** (battery or thermal): `hold=false` (also overrides Reachable and Always unless `cutoff.apply_to_reachable=false`).
2. **Paused** or **Mode::Off**: `hold=false`.
3. **Reachable** active: `hold=true`.
4. **Mode::Always**: `hold=true`.
5. **Mode::WhileWorking**: `hold = any(Working) || any(Waiting && now - waiting_since < waiting_grace)`.
6. Otherwise `hold=false`.

Pause and reachable are mutually exclusive; the last command wins.

**Battery cutoff** (OBSERVED lessons: "Not charging" is not discharging; "any line power" is unreliable because of the USB-C source device):
- Applies only when `battery_present && discharging`. Trigger at `battery_percent <= cutoff.battery_percent` (default 20). Clear when `!discharging` **or** `battery_percent >= battery_percent + 5`.
- Never derive "on battery" from UPower text or from "any line-power device online".

**Thermal cutoff** (debounced because idle spikes to 78 C were observed):
- The core keeps a ring of the last `thermal_rearm_secs` per-second samples.
- Trigger: **every** sample over the last `thermal_window_secs` (default 20) is `>= cutoff.thermal_c` (default **85**). Clear: every sample over the last `thermal_rearm_secs` (default 30) is `< cutoff.thermal_rearm_c` (default **70**).
- `cutoff.thermal_only_when_lid_closed` (default **true**): the thermal cutoff only evaluates while `lid_closed == Some(true)`; if `lid_closed` is `None` (no lid sensor, desktop) it always evaluates. Rationale: with the lid open the user can see and act, and ventilation is better; under all-core builds this machine legitimately plateaus at 91-93 C. [VERIFY: design choice, untested with a closed lid under sustained load.]
- Hardware critical on the target is 100 C; our limit must stay well below it.

If power or thermal data is unavailable, the corresponding cutoff never triggers; log once at startup what is available.

**Debounce:** never release within `min_hold_secs` (default 5) of acquiring.

**Known limitations (document in README):**
- When a cutoff releases our lock but the OS does not then suspend (for example, docked with an external monitor, see D1), the machine stays awake; coffeegent can only notify.
- UPower's own critical-action (HybridSleep) runs as root and may ignore our block lock; our 20% default cutoff is meant to act before it.

## 6. Backends (traits with real and fake implementations)

```rust
trait Inhibitor { fn set(&mut self, want: Option<LockSet>) -> Result<()>; } // None = release
struct LockSet { sleep_idle: bool, lid: bool, why: String }
trait PowerSource { fn snapshot(&self) -> PowerSnapshot; /* + change notifications */ }
trait Thermal { fn temp_c(&self) -> Option<f32>; }
trait Notifier { fn notify(&self, n: Notification); }
trait SessionLocker { fn lock(&self) -> Result<()>; }
```
The `fake` backend is in-memory and scriptable; used by tests and `coffeegent daemon --backend fake`.

### 6.1 Logind inhibitor and lid handling (real)
- Call `org.freedesktop.login1.Manager.Inhibit(what, who, why, mode)` on the **system bus** with `what = "sleep:idle:handle-lid-switch"`, `who = "coffeegent"`, `mode = "block"`. Hold the returned fd (`zbus::OwnedFd`) while `hold` is true; dropping it releases the lock. See `docs/reference/inhibit_spike.rs`.
- **`handle-lid-switch` must always be included while holding** (OBSERVED: required). `lid.lid_inhibit = "never"` exists only for diagnostics and is documented as breaking the main feature.
- `why` is human-readable, for example `"Claude Code is working (project: myapp)"`.
- On Inhibit failure: log, show the tray error state, retry with backoff (1, 2, 4 ... max 30 s). Never crash.
- **Lock on lid close** (`lid.lock_on_close`, default **true**): when `lid_closed` flips false -> true **while we hold the lock**, call `SessionLocker::lock()`, implemented as logind `Manager.LockSession(<XDG_SESSION_ID>)` (or `Session.Lock`), [VERIFY: untested on COSMIC; the maintainer's report suggested `loginctl lock-session`]. If it fails or the screen does not lock, send a notification ("lid closed but screen could not be locked") and keep running. Rationale: OBSERVED, closing the lid with our lock held does not lock the screen, so a laptop in a bag would sit unlocked. Do not use `LockedHint` to detect lock state.
- Suspend/resume: subscribe to `PrepareForSleep(bool)`; on resume recompute immediately. Note logind ignores lid events for 30 s after resume.

### 6.2 Power (real)
- Preferred: UPower D-Bus (DisplayDevice `Percentage` and `State`: 1 charging, **2 discharging**, 4 fully charged, 5 pending charge, 6 pending discharge) with `PropertiesChanged` signals. `discharging = (State == 2)`.
- Fallback without UPower: `/sys/class/power_supply/BAT*/{capacity,status}` polled every 30 s; `discharging = (status == "Discharging")`.
- Never parse UPower CLI output (locale decimal commas). `OnBattery` may be used for display only.
- `lid_closed`: logind `LidClosed` property (+ `PropertiesChanged`), or `None` if absent.

### 6.3 Thermal (real)
- Poll **every 1 s**. Select the sensor **by `type` name, not index**: default `thermal.sensor_types = ["x86_pkg_temp"]`. Fallback if absent: max of all valid thermal-zone and hwmon temperatures excluding `thermal.ignore_types` (default `["INT3400","SEN1","SEN3"]`) and known-bad channels (NVMe "Sensor 2"). Discard readings < -10 or > 150 C. No GPU temperature (not exposed; polling `nvidia-smi` may wake the dGPU).
- `doctor` prints which sensor(s) were selected.

## 7. IPC

- **Socket:** `$XDG_RUNTIME_DIR/coffeegent/coffeegent.sock`, dir 0700, socket 0600. Check `SO_PEERCRED`; reject any other uid.
- **Protocol:** newline-delimited JSON, `"v":1`, unknown fields ignored, unknown versions rejected. Max 64 KiB per message.

```json
{"v":1,"type":"event","agent":"claude","session_id":"abc","kind":"working","pid":1234,"cwd":"/home/u/proj"}
{"v":1,"type":"cmd","cmd":"status"}
{"v":1,"type":"cmd","cmd":"pause","for_secs":1800}          // or "until_ts_ms"; "indefinite":true
{"v":1,"type":"cmd","cmd":"resume"}
{"v":1,"type":"cmd","cmd":"reachable","for_secs":7200}      // "off":true to cancel
{"v":1,"type":"cmd","cmd":"mode","mode":"while_working"}    // | "always" | "off"
{"v":1,"type":"cmd","cmd":"toggle"}
{"v":1,"type":"cmd","cmd":"sessions"}
{"v":1,"type":"cmd","cmd":"reload"}
{"v":1,"type":"cmd","cmd":"quit"}
```
Responses: `{"v":1,"ok":true,"data":{...}}` or `{"v":1,"ok":false,"error":"..."}`.

`status` includes: `holding`, `reason`, `mode`, `paused_until`, `reachable_until`, `cutoff` (none|battery|thermal), `battery_percent`, `discharging`, `temp_c`, `lid_closed`, `session_counts`, `uptime_secs`, `version`.

**Persistence:** `$XDG_STATE_HOME/coffeegent/state.json` stores `mode`, `paused_until`, `reachable_until` (expired values dropped on load). Sessions are not persisted.

## 8. Configuration

`$XDG_CONFIG_HOME/coffeegent/config.toml`. Missing file means defaults; unknown keys warn; `reload` re-reads.

```toml
mode = "while_working"          # while_working | always | off

[timing]
waiting_grace_secs = 300
working_ttl_secs   = 600        # tune in gate G1
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
lid_inhibit   = "when_holding"  # when_holding | never (diagnostic only)
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
codex  = false                  # until M7
```

## 9. Agent adapters and installers

### 9.1 Principles
- **Never forward prompt content.** Adapters extract only session id, event name, cwd, pid; everything else is dropped inside the hook process.
- Adapters are pure `fn parse(input) -> Option<Event>`, tested against sanitized **real** fixtures captured in gate G1.
- **PID discovery:** the hook walks its parent-process chain (`/proc/<pid>/stat`) to find the agent process; send the best candidate as `pid`, or omit if uncertain [VERIFY in G1].
- Installers: idempotent, back up to `<file>.coffeegent.bak.<timestamp>` before the first change, preserve all unrelated content, tag our entries so uninstall removes only ours, support `--dry-run` (diff, no write), print what changed. Hook command: `<abs path>/coffeegent hook <agent>`.
- Hooks must be fast and synchronous; do not background work in them (a third-party claim says Claude kills backgrounded work in Stop hooks; irrelevant to us, because we do nothing slow, but do not try).

### 9.2 Mappings (DOCUMENTED, to be confirmed by captured fixtures in G1)

**Claude Code** (`~/.claude/settings.json`, JSON payload on stdin):
| Event | Kind |
|---|---|
| SessionStart | Start |
| UserPromptSubmit | Working |
| PreToolUse, PostToolUse | Working (heartbeat) |
| Notification | Waiting, **unless** the payload subtype indicates an idle/auth notice; subtype to be checked in G1 (FSM already ignores Waiting from Idle). Also check in G1 whether newer, more specific events (for example a permission-request event) exist. |
| Stop | Idle |
| SessionEnd | End |
| SubagentStop and others | ignored |

**Cursor** (`~/.cursor/hooks.json`, shape `{"version":1,"hooks":{"<event>":[{"command":"..."}]}}`, stdin JSON):
| Event | Kind |
|---|---|
| sessionStart | Start |
| beforeSubmitPrompt | Working |
| beforeShellExecution, afterShellExecution, afterFileEdit, afterAgentResponse | Working (heartbeat) |
| stop | Idle |
| sessionEnd | End |
| (permission/input) | **No documented event: GAP.** Cursor never produces "waiting"; document that the "waiting" notification and grace period do not apply to Cursor. |
Unknowns for G1: whether the `cursor-agent` CLI also runs these hooks; user-level hooks are documented as not applying to cloud agents (out of scope).

**Codex** (M7): `~/.codex/config.toml` `notify = [argv...]`, a **single global** notify, JSON as the last argv; documented only for turn completion (`agent-turn-complete`, approval event disputed). No work-started event, so M7 needs a file-activity fallback. The installer must **not clobber an existing `notify`**: chain via `coffeegent hook codex -- <original argv...>`, which sends our event then spawns the original command detached with the same arguments.

### 9.3 File-activity detector (M7)
Generic inotify detector: each watched file is a pseudo-session (`file:<path>`); a modification emits `Working`; `quiet_secs` without modification emits `Idle`. Paths and write cadence of Codex rollout files are unknown and must be observed first.

## 10. User interface

### 10.1 CLI
```
coffeegent daemon [--backend real|fake]
coffeegent status [--json]
coffeegent sessions [--json]
coffeegent toggle                            # flip mode while_working <-> off
coffeegent pause <30m|1h|tomorrow|DURATION>
coffeegent resume
coffeegent reachable <1h|2h|4h|DURATION|off>
coffeegent mode <while-working|always|off>
coffeegent install-hooks <claude|cursor|all> [--dry-run] [--yes]     # codex in M7
coffeegent uninstall-hooks <claude|cursor|all> [--dry-run]
coffeegent hook <claude|cursor>              # internal
coffeegent doctor [--json]
```
Exit codes: 0 success, 1 error, 2 daemon not running. Durations use `humantime`.

### 10.2 Tray (StatusNotifierItem)
- Icons (own monochrome SVGs, symbolic): `coffeegent-holding`, `-idle`, `-paused`, `-cutoff`, `-error`.
- Tooltip: reason, battery, session summary.
- Menu: status line, **Stay reachable** (1h/2h/4h/Off), **Pause** (30 min/1 hour/Until tomorrow/Resume), **Mode** radio, **Sessions** (`agent: project (state)`), battery line, **Quit**.
- **The tray is optional.** On COSMIC the watcher exists only while the Status Area applet runs, so the daemon must handle a missing or restarting watcher: warn once, retry registration with backoff, never fail startup. Gate G2 validates ksni on COSMIC.

### 10.3 Notifications (freedesktop Notifications)
`waiting` (Claude only): "<Agent> is waiting for you (<project>)"; `finished`: on Working -> Idle if worked >= `finished_min_work_secs`; `cutoff`: battery or too hot; lid-lock failure (6.1). Throttled by `cooldown_secs` per session; best effort.

### 10.4 Hotkeys
Not implemented; `docs/hotkeys.md` explains binding `coffeegent toggle` and `coffeegent pause 30m` as custom shortcuts in COSMIC, GNOME and KDE.

### 10.5 Autostart (decided by gate G3)
Unknown on COSMIC: the report only assumes (from binary strings) that `cosmic-session` starts `cosmic-session.target` through `systemctl --user`, and it is untested whether `graphical-session.target` is reached. Therefore: ship **both** a systemd user unit (`Type=simple`, `ExecStart=/usr/bin/coffeegent daemon`, `Restart=on-failure`; not auto-enabled) and an XDG autostart `.desktop` file; the single-instance guard (3.1) makes double start harmless. G3 picks the default and `doctor` reports the active mechanism.

## 11. Testing strategy (required)

1. **Core unit tests:** every FSM row (4.3), every precedence rule (5), battery latch (trigger, clear on AC and on +5%), thermal window and re-arm (including a single idle spike not tripping), `thermal_only_when_lid_closed` (closed, open, unknown), pause/reachable exclusion, Waiting-from-Idle ignored. Include a regression case for "Not charging at 100% is not discharging".
2. **Adapter tests:** every sanitized fixture parses to the expected `Event`; malformed input returns `None` without panic.
3. **Installer tests:** on temp copies of sample configs: idempotency, backup, preservation, clean uninstall (byte-identical round trip), `--dry-run` writes nothing.
4. **Daemon integration tests** with `--backend fake` on a temp `XDG_RUNTIME_DIR`: work -> idle release after `min_hold`; waiting grace; pid disappearance; battery cutoff; reachable beats idle; cutoff beats reachable; lid-close triggers `SessionLocker` exactly once per close while holding and never when not holding; second daemon instance exits 0.
5. **Hook shim tests:** daemon absent -> exit 0 within 150 ms; garbage or oversized stdin -> exit 0.
6. **Manual checklist** (`docs/manual-test.md`): (a) lid closed on battery for 10+ minutes with a sustained agent task, watching temperature; (b) lock-on-close actually locks; (c) low-battery cutoff releases and the laptop suspends; (d) external monitor attached (document the result); (e) idle-timer suspend: configure `cosmic-idle` suspend timers and confirm they do not suspend while we hold the `sleep` lock; (f) tray on COSMIC; (g) install/uninstall hooks on a real agent; (h) suspend/resume.
7. Optional (v0.2): `dbusmock` under `dbus-run-session`.

CI: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deb` on tags.

## 12. Quality, security, privacy

- `#![forbid(unsafe_code)]` in core; minimal commented `unsafe` elsewhere (prefer `nix`). No `unwrap`/`expect` outside tests. `anyhow` in the binary, `thiserror` in core.
- `tracing` logs to stdout/stderr (journald); level via `COFFEEGENT_LOG`. **Never log prompt content, file contents or environment variables.**
- No network. IPC socket user-only with peer-credential check. Daemon unprivileged: no setuid, no polkit rules, no root helper (OBSERVED: not needed).
- Installers touch only the agent's own config file.
- **README Security section:** explain that the lid-close lock keeps the machine running and, if `lock_on_close` fails, leaves the screen unlocked.
- Budget: idle CPU ~0%, RSS < 25 MB.

## 13. Packaging and release
- v0.1: `cargo install --path crates/coffeegent` and a **`.deb`** (`cargo-deb`) for Ubuntu/Pop!_OS 24.04: `/usr/bin/coffeegent`, `/usr/lib/systemd/user/coffeegent.service`, `/etc/xdg/autostart/coffeegent.desktop` (or per G3), icons, docs. Post-install message tells the user to run `coffeegent install-hooks` and enable autostart. **Never modify agent configs at install time.**
- Later: AUR, tarball, Nix. **Not** Flatpak/Snap.
- SemVer; Keep a Changelog.

## 14. Roadmap beyond v0.1
v0.2: Codex hardened, heuristic detector with a TOML agent registry, more agents, D-Bus control interface, shortcut helper, socket activation. Later: preferences GUI, COSMIC (libcosmic) applet if SNI proves poor, display control, docked-mode handling.

## 15. Implementation plan (for the code agent)

Work milestone by milestone. After each: tests green, clippy clean, short summary of changes and of remaining [VERIFY] items. **Stop and ask the human** if a gate fails or reality contradicts this spec.

| M | Scope | Acceptance criteria |
|---|---|---|
| **M0** | Workspace scaffold, CI, MIT license, README skeleton, `coffeegent --version`, tracing; copy the spike to `docs/reference/` | CI green |
| **M1** | `coffeegent-core`: types, config, session FSM, `decide()`, both cutoff latches, duration parsing | All 11.1 tests pass; core has no I/O deps |
| **M2** | Daemon runtime: IPC, fake backends, state persistence, all commands, CLI client, single-instance guard | 11.4 tests pass on `--backend fake` |
| **M3** | Real backends: logind lock (fd), UPower + sysfs fallback, thermal poller (by type name), lid-closed tracking, `SessionLocker`, suspend/resume | On the target laptop: `reachable 10m` shows the lock in `systemd-inhibit --list`; SIGKILL drops it; lid-closed run behaves as in section 0; lock-on-close result recorded (works, or notification fallback documented) |
| **G1 (start of M4)** | **Fixture capture.** Using the maintainer's spike tools (`hook-log.py`, `cfg.py`, `summarize.py` in `~/coffeegent-spike`) or a minimal logging hook, record real Claude Code and Cursor sessions: prompt, tool use, permission prompt, Esc-interrupt, normal finish, quit. Produce **sanitized** fixtures (human reviews before commit). Answer: Is Stop sent on interrupt? Notification subtypes? Event gaps during long tool runs (set `working_ttl`)? Is a PID derivable? Do `cursor-agent` hooks fire? | Fixtures committed; mapping tables in 9.2 corrected to OBSERVED; `working_ttl_secs` default set from data |
| **M4** | Hook shim, adapters (Claude, Cursor), installers with backup/dry-run/uninstall | Real sessions drive Working -> Waiting (Claude) -> Idle; install/uninstall round trip is byte-identical |
| **G2 (start of M5)** | **Tray gate.** Minimal ksni test on COSMIC: icon visible, menu works, notification, icon survives a Status Area applet restart, daemon started before the watcher | Result recorded; if ksni fails, M5 ships notifications + CLI only and logs a tray issue |
| **M5** | Tray, notifications, icons, `doctor` | Reviewed on COSMIC; notifications throttled |
| **G3 (start of M6)** | **Autostart gate.** Create a throwaway user unit and a `.desktop` autostart entry that log start time plus `WAYLAND_DISPLAY`, session bus and watcher presence; one logout/login | Default mechanism chosen and documented |
| **M6** | Packaging (`.deb`), docs, README with support matrix and Security section, manual checklist run, release `v0.1.0` | Fresh install on target passes the checklist (11.6) |
| **M7** | Codex: observe real `notify` payloads and rollout-file behavior (requires Codex installed by the maintainer), adapter + chained installer + file-activity detector | Codex session drives Working/Idle; existing `notify` preserved |

**Definition of done (each milestone):** fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.

**Style for the agent:** plain structs/enums and small functions; explain *why* in comments (the maintainer is learning Rust); keep policy and FSM side-effect free; no features outside the milestone (ideas go to `docs/ideas.md`).

## 16. Open items
- **D1. Docked behavior:** with an external monitor, `cosmic-comp` may hold its own lid lock (upstream #19, ASSUMED). Releasing ours then would not restore suspend. Test in the manual checklist and document.
- **D2. Idle:** `cosmic-idle` owns idle timers and serves `org.freedesktop.ScreenSaver`; whether the logind `sleep` lock blocks its suspend is untested (manual checklist 11.6e). If it does not, add a `org.freedesktop.ScreenSaver.Inhibit` backend (note this also keeps the screen on, so it would be opt-in).
- **D3. Panel state:** the compositor keeps the internal output enabled with the lid closed; whether the physical panel stays lit (battery and heat) is unknown.
- **D4. Longer closed-lid runs, low-battery behavior, and UPower critical-action interplay** are untested.
- **D5. `lock_on_close`** mechanism on COSMIC (M3).
- **D6. Thermal design choice** (`thermal_only_when_lid_closed`) is untested under closed-lid load.
