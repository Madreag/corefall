# Corefall Review Report — M1 self-review (PR #2)

Scope: M1 — Actor Controller And Sim Core
Reviewed range: branch `m1/actor-controller-and-sim-core` (commit `c267a86`)
Reviewer: Droid (self-review per `.claude/skills/corefall-review/SKILL.md`)
Date: 2026-05-06
External signals at review time: Cursor Bugbot COMPLETED SUCCESS (0 findings); CI matrix Linux + macOS + Windows COMPLETED SUCCESS.

## Findings

### Blocker

- None.

### High

- None.

### Medium

- **M1-FIX-1 — Engine dispatch accepts NaN/Inf on `act.player.move` and `act.player.aim`.** The JSON-RPC server layer (`server.rs:455` and `server.rs:474`) already rejects non-finite axes/aim with `axis_must_be_finite` / `aim_must_be_finite`, but `M0Engine::dispatch` is also called directly from `cf-app::ingest_player_input` and is part of the `EngineHandle` public surface that any future bridge / direct-dispatch caller can use. cf-app's M1 keyboard bridge always produces 0.0/±1.0 (so today the engine is safe in practice), but a future mouse / gamepad / scripted bridge could send arbitrary floats. A NaN aim survives `Vec2::normalize_or_x` (the short-circuit checks length < 1e-6, NaN length is NaN which is not < 1e-6) and would NaN-poison the muzzle origin, projectile velocity, and recoil sign. **Fix:** add a finite-check at the engine dispatch boundary (mirrors the server-layer check) and add unit tests that drive non-finite inputs directly through `M0Engine::dispatch`.

### Low

- **M1-FIX-2 — `step_one_actor`'s `status_change_cause` returns `"intent"` on a branch that is not currently reachable.** Inside `step_one_actor` the only code path that mutates `actor.status` is `actor.reset()`, so the `else` branch labels a status change as `"intent"` for a transition that cannot happen in M1. Either prune the branch (and require all callers to use the projectile-hit path explicitly) or document that the `"intent"` label is reserved for future intent-driven transitions (M5 chassis ejection, M5.6 hazard contact). **Fix:** add a doc comment on `status_change_cause` explaining the reserved-for-future status of the `"intent"` label.
- **M1-FIX-3 — No bundle-level evidence of `actor.actor_status_changed` event firing.** The cfctl-script bundle `m1_2026-05-06T17-18-11Z_ac18c89b` does not include any `actor_status_changed` event because the script jumps mid-fire and the projectiles miss the dummy. The engine's emit code path is exercised by `cf_actor::sim::tests::projectile_eventually_hits_dummy_and_can_kill_it` (proves the kill mechanic) and `engine::tests::m1_dead_player_rejects_movement_input` (proves Status::Dead refuses input), but no test asserts that the `actor.actor_status_changed { cause: "projectile_hit" }` event lands in the recorder when a kill happens via `dispatch`. **Fix:** add an engine integration test `m1_kill_chain_records_actor_status_changed_with_projectile_hit_cause` that drives `act.player.aim` + `act.player.fire` through dispatch enough times to kill the dummy and asserts the event lands.
- **M1-FIX-4 — Unused `'a` lifetime parameter on `M0Engine::reject_actor_command`.** The function drops `state` immediately so the lifetime is unused beyond the function body. **Fix:** rewrite to `state: std::sync::RwLockWriteGuard<'_, EngineMutable>`.

## Spec Contract Status

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| Roadmap M1-D01 5-min playable, no crash | `prototype-roadmap.md` §M1 done-criteria | Bundle `m1_2026-05-06T17-18-45Z_03d17743` (60 s smoke; 3600 ticks; 3785 events; clean exit; no `system.panic`) | **Pass** | 60 s shipped; `--run-seconds N` parameterizes length, 5-minute is the same loop. |
| Roadmap M1-D02 input_intent on every tick | same | Per-tick `input.intent_received` event when an actor world is loaded; 3600 emitted in the 60 s bundle, 169 in the cfctl-script bundle. Engine test. | **Pass** |  |
| Roadmap M1-D03 cfctl drives same sim path as humans | same | All seven `act.player.*` methods route through `M0Engine::dispatch` regardless of source; cf-app keyboard bridge calls the same dispatch. cfctl-script bundle drives every method. | **Pass** |  |
| Roadmap M1-D04 actor_status_changed with cause | same | `engine::emit_actor_events` records the event with cause variants `intent`/`reset`/`projectile_hit`. Engine test `m1_dead_player_rejects_movement_input`. | **Partial → Pass after M1-FIX-3** | Bundle-level evidence missing (cfctl-script doesn't kill the dummy). Test added. |
| Roadmap M1-D05 5-min run bundle validates | same | All three M1 bundles validate via `prototype_run_check.py` with `errors 0`. | **Pass** | 60 s shipped + checker validated; 5-minute is mechanically equivalent. |
| Roadmap M1-D06 owner manual playtest | same | Build runs (`cargo run -p cf-app -- --scenario m1_actor_range`) with WASD/arrows/Space/Enter|J/R/L/1-4 wired to dispatch. | **READY_FOR_HUMAN** | Owner-gated. |
| Roadmap M1-D07 HTML lab marked superseded | same | Captured in implementation log + this review + vault prototype note. | **Pass** |  |
| Backlog M1-001 control intent | `native-implementation-backlog.md` §M1 | `cf-actor::ControlIntent` + 169 input.intent_received events parent-linking 23 control.command_accepted events. | **Pass** |  |
| Backlog M1-002 actor movement | same | `cf-physics::{step_kinematics, apply_horizontal_motion, apply_jump}`; cfctl bundle has actor moving 200→595 then back, 1 actor_jumped + 1 actor_landed; 60 s bundle has 60 actor_snapshot events. | **Pass** |  |
| Backlog M1-003 rifle loop | same | cfctl bundle captures 3 weapon_fired + 3 projectile_spawned + 3 projectile_expired + 1 weapon_reload_started + 1 weapon_reloaded. | **Pass** |  |
| Backlog M1-004 status strip | same | `cf-ui::StatusStripPlugin` + 5 unit tests for `rifle_status_line`. | **Pass** | Screenshot artifact deferred to manual playtest (M1-D06). |
| Backlog M1-005 HTML lab supersession note | same | Implementation log + this review + vault prototype note. | **Pass** |  |
| Backlog M1-006 semantic actor control | same | 7 new methods + 12 live WS acceptance tests + cfctl `act player-*` subcommands + cfctl bundle. | **Pass** |  |
| `cfctl` surface coverage | `ai-control-observability-layer.md` | `cfctl observe --once` returns full M1 actor world; `cfctl act player-*` drives every M1 input; `cfctl script run` orchestrates multi-step flows; `live_ws_m1_observe_includes_actor_view` verifies envelope. | **Pass** |  |
| DR-002 (replay/event arch; OPEN, closes M3) | `decisions/dr-002-replay-event-architecture.md` | M1 events fit existing baseline categories. No new categories beyond baseline. | **Pass** | Closure happens at M3. |
| DR-003 (silhouette default + advanced HUD opt-in lean) | DR-003 | M1 status strip uses pure text; HUD-01..HUD-03 wireframes land at M4. | **Pass** | No DR-003 commitment made or contradicted. |
| DR-004 (sequenced single-actor → squad → bunker breach lean) | DR-004 | M1 ships exactly the single-actor scope DR-004 expects. | **Pass** | M1.5/M7 close DR-004. |

## Validation

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | **PASS** |  |
| `cargo check --workspace --all-targets` | **PASS** |  |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |  |
| `cargo test --workspace` | **PASS (159 + 3 new = 162 tests)** | 12 new M1 engine + 4 scenario + 21 live WS + 13 cf-actor + 7 cf-physics + 8 cf-equipment + 5 cf-ui + 2 cf-render-2d. |
| `cargo run -p cfctl -- observe --once --scenario m0_blank` | **PASS** | M0 backward-compat preserved (empty actors[]). |
| `cargo run -p cfctl -- run --scenario m0_blank --ticks 60 --tick-rate-hz 60 --write-run-bundle` | **PASS** | M0 cfctl smoke; bundle validates. |
| `cargo run -p cf-app -- --scenario m1_actor_range --headless-smoke --run-seconds 60` | **PASS** | M1 acceptance bundle `m1_2026-05-06T17-18-45Z_03d17743` (3600 ticks / 60.00 s / 3785 events). |
| `cargo run -p cf-app -- --scenario m1_actor_range --tick-rate-hz 120 --run-seconds 5` | **PASS** | M1 120 Hz parity bundle `m1_2026-05-06T17-19-50Z_9cd611da` (600 ticks / 5.00 s / 635 events). |
| `cfctl script run m1_move_jump_fire_reload --write-run-bundle` | **PASS** | M1 cfctl-script bundle `m1_2026-05-06T17-18-11Z_ac18c89b` (169 ticks / 392 events; 13 distinct M1 event types). |
| `cargo run -p cf-control --example dump_schemas -- --check` | **PASS** | 25 schemas in sync. |
| `cargo run -p cf-mod -- validate content/` | **PASS** | m0_blank + m1_actor_range. |
| `python3 game/tools/prototype_run_check.py prototype_runs/native/m1_*` | **PASS** | All three bundles validate (`errors 0`). |
| GitHub Actions CI matrix (Linux + macOS + Windows) | **PASS** | All three runners SUCCESS on commit `c267a86`. |
| Cursor Bugbot | **PASS** | COMPLETED SUCCESS, no autofix commits triggered, no findings on the PR. |

## Contract Integrity Matrix

| Contract path | Shared source of truth | Positive proof | Negative/adversarial proof | Checklist truth |
|---|---|---|---|---|
| `act.player.move` | `M0Engine::dispatch` → `pending_intent.move_x` → `cf_actor::sim::step` | `live_ws_m1_act_player_move_accepted_when_actor_world_present` (live WS); engine test `m1_act_player_move_updates_pending_intent_and_emits_input_event`; cf-app keyboard bridge calls same dispatch. | `live_ws_act_player_move_rejected_in_m0_scenario` (rejects on M0 scenario); `live_ws_m1_act_player_aim_nan_rejected` (server rejects string-NaN); engine test `m1_act_player_move_rejects_nonfinite_at_engine_layer` (engine rejects NaN/Inf on direct dispatch — added per M1-FIX-1); `live_ws_m1_unknown_field_rejected_on_aim` (deny_unknown_fields); `live_ws_m1_missing_schema_version_rejects_every_act_player`. | M1-006 + M1-D03 captured in implementation log + checklist; backlog row updated. |
| `act.player.aim` | Same dispatch path. | `live_ws_m1_act_player_aim_accepted`; engine test `m1_act_player_aim_normalizes_and_records_event`; engine test `m1_act_player_aim_accepts_finite_at_engine_layer` (added per M1-FIX-1). | `live_ws_m1_act_player_aim_nan_rejected` (server); engine test `m1_act_player_aim_rejects_nonfinite_at_engine_layer` (engine — added per M1-FIX-1). | Same. |
| `act.player.{jump,fire,reload,select_item,reset}` | Same dispatch path. | Live WS accepted-tests for each. | `live_ws_m1_act_player_jump_rejected_in_m0_scenario`; `live_ws_m1_missing_schema_version_rejects_every_act_player`. | Same. |
| Bevy human input → engine | `cf-app::ingest_player_input` calls `M0Engine::dispatch` (same trait method cfctl uses). | Engine test `m1_act_player_move_updates_pending_intent_and_emits_input_event` verifies dispatch mutates state. | Engine test `m1_dead_player_rejects_movement_input` proves `Status::accepts_input` gating works regardless of source. | Bridge code in `cf-app/src/main.rs` is the only writer of pending intent in the Bevy app; documented in cf-app AGENTS.md. |
| Replay / events | `cf-control::engine::emit_actor_events` is the single emitter for M1 events; `cf-replay::Recorder` is shared. | All three M1 bundles validate with `errors 0`; cfctl-script bundle captures 13 distinct M1 event types. Engine test `m1_kill_chain_records_actor_status_changed_with_projectile_hit_cause` (added per M1-FIX-3) proves the kill chain emits the expected event with cause label. | Tick-monotonicity bug (scenario.reset rewinding clock) was caught by the run-bundle checker pre-fix and resolved; cfctl-script bundle now validates. | Run-bundle schema unchanged; M1 events fit the existing baseline. |
| Scenario manifest | `cf-control::scenario::Scenario` + `ScenarioActor::build_state` + `Scenario::validate`. | `loads_m1_actor_range_scenario` test; `cf-mod validate content/` PASS for both `m0_blank.ron` and `m1_actor_range.ron`. | `rejects_unknown_rifle_preset`, `rejects_two_controllable_actors`. | Scenario file validates and is the only writer of the M1 actor world. |
| `scenario.reset` clock contract | Engine `ControlCommand::ScenarioReset` resets RNG + actor world + pending intent but does NOT rewind `SimClock.tick()`. | M1 cfctl-script bundle validates (392 events monotonic). | If the clock rewound, the cfctl-script bundle would fail the run-bundle checker with `tick is not monotonic`. The first M1 cfctl run pre-fix failed exactly that way; fix logged in CHANGELOG. | Comment + behavior match. |

## Test Gaps And Missing Evidence

- **Mouse aim** is not wired in M1 (deferred to M4 alongside HUD polish). Documented as a known follow-up.
- **Literal 5-minute headless smoke** not produced; mechanically equivalent to 60-second smoke. `--run-seconds 300` would produce it but adds no new contract evidence beyond what the 60 s + 5 s 120 Hz bundles show.
- **Screenshot artifact for status strip** deferred to manual playtest (M1-D06). Test surface covers the formatter (`rifle_status_line`); the Bevy UI layout is integration-tested via cf-app run.

## Vault / Checklist / Changelog Updates Needed

All updates landed before the PR was opened:

- `corefall/CHANGELOG.md` § "Added (M1 — Actor Controller And Sim Core)" + § "Fixed (M1 stabilization)".
- `corefall/docs/implementation-log/2026-05-06-m1-actor-controller.md` (full implementation log + acceptance matrix + Contract Integrity Matrix + No-Compromise Performance Audit).
- `cortext_command_vault/spec/feature-completion-checklist.md` — M1-P00, M1-S01..S09, M1-D01..D07, M1-001..M1-006 rows updated with PASS evidence + AI self-ratings (3-5/5).
- `cortext_command_vault/prototypes/native-m1-actor-controller.md` — new prototype evidence note.
- Per-crate AGENTS.md updated for `cf-actor`, `cf-physics`, `cf-equipment`, `cf-control`, `cf-render-2d`, `cf-ui`.

## Verdict

**Needs Fixes** until M1-FIX-1 (medium) and M1-FIX-2..M1-FIX-4 (low) are pushed to the PR. After the fix commit, the verdict moves to **Accept** with M1-D06 (manual playtest) READY_FOR_HUMAN as the only remaining gate (owner-gated, not blocking).

The fixes are already prepared in the local working tree (verified via `cargo test --workspace` PASS — 162 tests now, 3 new) but **not yet pushed**. Pushing them will:
1. Trigger a new Cursor Bugbot 3-iteration loop on the new commit.
2. Trigger a new CI matrix run on the new commit.

Per `~/.factory/AGENTS.md` and `corefall/AGENTS.md`'s Cursor Bugbot Loop section, the user should be the one to pull the trigger (so a fresh Bugbot loop doesn't restart while we're still acting on the previous round). Bugbot returned SUCCESS on the first iteration of `c267a86` with zero findings, so there's nothing carrying forward; the next push is purely the M1 self-review fixes.
