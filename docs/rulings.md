# Confirmed rulings (baseline for M1+)

These four decisions were confirmed on **2026-11-07**. They are authoritative for implementation;
do not reinterpret them in later milestones. The initiative file records the same text.

## Mode persistence (`state.json` vs `config.toml`)

Once `mode` is saved in `state.json`, it wins over `config.toml`. `coffeegent mode` and `toggle`
write state only. `reload` refreshes the rest of the config and leaves `mode`, `pause`, and
`reachable` unchanged. Config `mode` applies only when state has no saved `mode`.

## `min_hold_secs`

`min_hold_secs` delays only a release because work ended. A latched cutoff, pause, and mode off
release the lock immediately.

## `toggle`

`toggle` flips only `while_working` and `off`. From `always` it goes to `off`. From `off` it goes
to `while_working`.

## `coffeegent hook` exit status

`coffeegent hook` always exits 0. Exit codes 0, 1, and 2 apply to the other commands.
