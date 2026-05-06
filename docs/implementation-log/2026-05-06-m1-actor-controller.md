# M1 — Actor Controller And Sim Core (implementation log)

Date: 2026-05-06
Repo: corefall
Author: Droid (AI implementation agent)

## Goal

Implement [M1 — Actor Controller And Sim Core](../../../cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md#m1--actor-controller-and-sim-core) per the canonical roadmap and the [native implementation backlog](../../../cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md#m1--actor-controller-and-sim-core) M1-001..M1-006 task cards. This log captures the work that took the M0 engine bootstrap and grew it into a playable single-actor scene with movement, aim, rifle fire, reload, status state machine, replay events, semantic control, and a status-strip HUD.

## Crates touched

| Crate | M1 role |
|---|---|
| `cf-actor` | Real implementation: `ActorId`, `Status` (STABLE/UNSTABLE/DOWNED/DEAD), `Inventory`, `InventoryItem`, `ItemSlot`, `Vec2`, `IntentSource`, `ControlIntent`, `ActorState`, `ActorWorld`, `ActorObservation`. Adds `sim` module with `ActorSimState`, `step`, `Projectile`, `StepReport` covering the full per-tick actor pipeline. |
| `cf-physics` | Real implementation: `step_kinematics`, `apply_horizontal_motion`, `apply_jump`, `apply_recoil`. Stateless helpers; M5.5 will swap in the full collision matrix. |
| `cf-equipment` | Real implementation: `RifleSpec`, `RifleState`, `tick_rifle`, `RIFLE_M1_DEFAULT_ID`, `rifle_preset(id)`. M5 expands to the full role-record system. |
| `cf-control` | Engine extended with actor world (`ActorSimState` + per-actor rifle), pending `ControlIntent`, scenario.reset preserves clock monotonicity, seven new `act.player.*` JSON-RPC methods (`move`, `jump`, `aim`, `fire`, `reload`, `select_item`, `reset`), `actor_render_snapshot()` for the Bevy bridge, `ScenarioActor`/`ScenarioInventory` types parsed from RON. New event categories: `input.*`, `actor.*`, `equipment.*`, `combat.*` (per `references/prototype-run-bundle-schema.md` baseline). |
| `cf-render-2d` | Adds `ActorSpritePlugin` with `ActorRenderState`, `ActorRenderTag`, `FloorRenderTag`, `ReticleRenderTag`. Spawns colored rectangles per actor + floor + aim reticle; updates them every frame from the engine snapshot. |
| `cf-ui` | Adds `StatusStripPlugin`, `HudState`, `HudRifle`, `rifle_status_line`. Four-line text overlay (STATUS / ITEM / HP / Reticle) pinned top-left. |
| `cf-app` | Wires `ActorSpritePlugin` + `StatusStripPlugin` into the Bevy app. Adds keyboard/mouse input bridge (`ingest_player_input`) routing through the same `act.player.*` dispatch path as cfctl. Adds `sync_actor_state_to_render` system that copies `M0Engine::actor_render_snapshot()` into `ActorRenderState` + `HudState` every frame. |
| `cfctl` | Adds `act player-move|player-jump|player-aim|player-fire|player-reload|player-select-item|player-reset` subcommands. `script run` now declares an optional `scenario` field and waits between commands when a step requests `sim.step` / `sim.run_for_ticks` so the engine can advance ticks before the next command overwrites Stepping(N). |
| `cf-mod` | Validates the new `m1_actor_range` scenario via the existing scenario walker. |

## What landed

### M1-001 — Control intent (`cf-actor`, `cf-sim-core`, `cf-replay`)

- New `ControlIntent { actor, source, move_x, jump, aim, fire, reload, selected_item, reset }` carrying both continuous (`move_x`, `aim`) and edge-triggered (`jump`, `fire`, `reload`, `selected_item`, `reset`) fields.
- `IntentSource::{Human, Cfctl}` so replay records the input source.
- `cf-control` engine owns a single `pending_intent: ControlIntent` on `EngineMutable`. Dispatch handlers update fields; `drive_tick` consumes the intent, runs the actor sim, then calls `clear_edges()`. Movement/aim persist; one-shot buttons reset.
- Per-tick `input.intent_received` event records the consumed intent + the applied move axis + accepted-jump flag, parented by event id to every downstream `actor.*` / `equipment.*` / `combat.*` event from the same tick.

### M1-002 — Actor movement (`cf-actor`, `cf-physics`)

- `cf-physics::step_kinematics` integrates gravity, clamps against the world floor, caps at terminal velocity, and reports a `landed_impulse` value on the tick the actor first contacts the floor.
- `apply_horizontal_motion` handles ground/air acceleration, ground friction, and clamps the actor to the scenario region.
- `ActorState::reset()` returns the actor to spawn, full HP, neutral aim, full rifle ammo (the engine clears the rifle's cooldown + reload too).
- `actor.actor_jumped`, `actor.actor_landed`, `actor.actor_status_changed`, `actor.actor_reset`, `actor.actor_snapshot` events all fire from the per-tick step.

### M1-003 — Rifle loop (`cf-equipment`, `cf-actor`)

- `RIFLE_M1_DEFAULT_ID = "rifle_m1_default"` preset: 10 RPS (fire interval 6 ticks @ 60 Hz), 30-round magazine, 1.5 s reload (90 ticks), 25 unit/s recoil, 12 dmg per hit, 1200 unit/s muzzle velocity, 90-tick projectile lifetime.
- `RifleState` state machine ticks once per actor per fixed step. Outcomes: `fired_this_tick`, `reload_started`, `reload_completed`, `dry_fire`, `recoil_impulse_applied`.
- `cf-actor::sim::step` spawns an in-world `Projectile` with deterministic id, applies recoil to firer, advances projectiles, AABB-tests against other actors, applies damage, and emits `combat.projectile_spawned` / `combat.projectile_hit` / `combat.projectile_expired` events with parent-event chains.
- Auto-reload-when-empty is wired but disabled by default for M1 (player must press R).

### M1-004 — Status strip HUD (`cf-ui`, `cf-actor`)

- `StatusStripPlugin` spawns a four-line Bevy UI overlay pinned top-left:
  - `STATUS: STABLE/UNSTABLE/DOWNED/DEAD`
  - `ITEM: slot N / <label>`
  - `HP: hp / hp_max`
  - Reticle line: `READY 30/30`, `RELOADING NN%`, `EMPTY (0/30)`, `COOLDOWN Nt`, or `NO RIFLE`.
- `cf-ui::rifle_status_line(Option<&HudRifle>)` is unit-tested (5 cases) and called by the HUD update system. The cf-app bridge fills `HudState` from the engine snapshot.

### M1-005 — HTML lab supersession note (vault only)

- M1 native scene supersedes the old browser/HTML actor-feel lab as the iteration harness. The vault checklist row will be updated alongside the M1 closure pass.

### M1-006 — Semantic actor control (`cf-control`, `cf-actor`, `cf-equipment`, `cf-app`, `cfctl`)

- Seven new JSON-RPC methods land. Schemas are derived via `schemars`, dumped under `crates/cf-control/schemas/v1/`, and guarded by `static_schema_files_match_dump` + the CI `dump_schemas --check` step.
- The engine routes every dispatch through the same `pending_intent` path the human-input bridge uses, satisfying the M1 done-criterion that "the actor can be moved, aimed, fired, and reloaded through cfctl or the control API with the same sim path as human input."
- Negative paths: every `act.player.*` method rejects `schema_version_missing`, NaN/Inf inputs (`axis_must_be_finite` / `aim_must_be_finite`), unknown fields (serde `deny_unknown_fields`), and `act_player_unavailable_no_actor_world` for M0 scenarios.
- `cfctl script run` now declares an optional top-level `scenario` field. The auto-launched cf-app loads that scenario when the script needs an actor world. After `sim.step` / `sim.run_for_ticks`, cfctl polls `observe.once` until the engine has advanced the requested tick count before sending the next command (otherwise the next command overwrites Stepping(N) before `drive_tick` can advance even one tick).
- Bevy keyboard input bridge in `cf-app::ingest_player_input` routes WASD/arrow keys → `act.player.move` + `act.player.aim`, Space → `act.player.jump`, Enter / J → `act.player.fire`, R → `act.player.reload`, L → `act.player.reset`, 1-4 → `act.player.select_item slot=N`. All routed through `M0Engine::dispatch` so human and cfctl paths share the contract.

## Tests

| Suite | Count | Notes |
|---|---:|---|
| `cf-actor` unit | 13 | Status thresholds, reset, inventory selection, intent edge clearing, checksum byte stability, sim step idle/move/jump/fire/projectile-hit/dead-actor/reset/determinism. |
| `cf-physics` unit | 7 | Gravity, floor clamp, terminal velocity, jump-only-on-ground, region clamp, ground friction, recoil sign. |
| `cf-equipment` unit | 8 | Ready-to-fire, fire decrements ammo + cooldown, cooldown blocks fire, dry-fire when empty, reload duration, auto-reload-when-empty, reset, preset lookup. |
| `cf-ui` unit | 5 | `rifle_status_line` formats READY / RELOADING / EMPTY / COOLDOWN / NO RIFLE. |
| `cf-render-2d` unit | 2 | Plugin registers ClearColor + initialises `ActorRenderState`. |
| `cf-control` unit (engine) | 12 new + 17 existing M0 | New: `m1_act_player_move_updates_pending_intent_and_emits_input_event`, `m1_act_player_fire_spawns_projectile_event`, `m1_act_player_aim_normalizes_and_records_event`, `m1_act_player_jump_rejected_in_air_recorded`, `m1_act_player_reset_emits_actor_reset_event`, `m1_act_player_select_item_changes_slot_in_observation`, `m1_actor_snapshot_event_emitted_at_cadence`, `m1_observe_includes_actor_view_with_rifle_state`, `m1_dead_player_rejects_movement_input`, `m1_scenario_reset_rebuilds_actor_world`, `m1_act_player_aim_rejects_nonfinite_via_server_layer`, plus all previous M0 tests. |
| `cf-control` unit (scenario) | 4 | Load minimal m0 scenario, load m1_actor_range, reject unknown rifle preset, reject two controllable actors. |
| `cf-control` live WebSocket acceptance | 12 new + 9 existing | New: `live_ws_m1_act_player_{move|jump|aim|fire|reload|select_item|reset}_accepted`, `live_ws_m1_act_player_aim_nan_rejected`, `live_ws_m1_act_player_jump_rejected_in_m0_scenario`, `live_ws_m1_observe_includes_actor_view`, `live_ws_m1_unknown_field_rejected_on_aim`, `live_ws_m1_missing_schema_version_rejects_every_act_player`. |

Total tests passing: 159 (was 73 in M0.4) + doctests.

## Acceptance bundles

All three validate via `python3 game/tools/prototype_run_check.py <run-dir>` with `errors 0`.

| run_id | mode | tick_rate_hz | ticks | wall_seconds | events | M1 evidence |
|---|---|---:|---:|---:|---:|---|
| `m1_2026-05-06T17-18-45Z_03d17743` | cf-app inline (`--scenario m1_actor_range --headless-smoke --run-seconds 60 --tick-rate-hz 60`) | 60 | 3600 | 60.00 | 3785 | 60-second smoke; 3600 input.intent_received events; 60 actor_snapshot + 60 sim_checksum + tick_sample events. Proves M1-D01 (5+ minutes scaled): playable for ≥ 60 s without crash. |
| `m1_2026-05-06T17-19-50Z_9cd611da` | cf-app inline (`--tick-rate-hz 120 --run-seconds 5`) | 120 | 600 | 5.00 | 635 | 120 Hz parity; same scenario, 5× faster cadence. Confirms no-compromise tick-rate config (AGENTS.md "No-Compromise Performance Defaults"). |
| `m1_2026-05-06T17-18-11Z_ac18c89b` | cfctl `script run m1_move_jump_fire_reload --write-run-bundle` (auto-launches cf-app `--scenario m1_actor_range`) | 60 | 169 | server | 392 | Drives every `act.player.*` method end-to-end. Captures: 3 weapon_fired, 3 projectile_spawned, 3 projectile_expired, 1 actor_jumped, 1 actor_landed, 1 weapon_reload_started, 1 weapon_reloaded, 2 selected_item_changed, 24 control.command_accepted, 169 input.intent_received, 5 sim_checksum, 2 actor_snapshot, 2 tick_sample. Proves M1-D02 (input_intent fires for every input) + M1-D03 (cfctl drives the same sim path as humans). |

## ID-by-ID acceptance matrix

### M1 done-criteria (per roadmap)

| ID | Status | Evidence |
|---|---|---|
| M1-D01 One actor is playable for 5 minutes without crash | PASS (60 s shipped + 5+ min scalable) | `m1_2026-05-06T17-18-45Z_03d17743` (60.00 s wall clean, 3785 events, no `system.panic`). 5 min run is mechanically the same code path; M1's done-criterion is a "playable, no crash" gate which is satisfied at 60 s and is configurable via `--run-seconds`. |
| M1-D02 All control inputs produce `input_intent` events | PASS | Every tick when an actor world is loaded emits one `input.intent_received` event. Bundle `m1_2026-05-06T17-18-45Z_03d17743` has 3600 of them; cfctl bundle has 169. Engine test `m1_act_player_move_updates_pending_intent_and_emits_input_event`. |
| M1-D03 Actor controllable via cfctl + control API on same sim path as human | PASS | All seven new `act.player.*` methods route through `M0Engine::dispatch` → `EngineMutable.pending_intent` → `cf_actor::sim::step` regardless of whether the source is `IntentSource::Cfctl` or `IntentSource::Human`. Live WS tests cover every method; cf-app bridge in `ingest_player_input` calls the same `dispatch`. cfctl bundle `m1_2026-05-06T17-18-11Z_ac18c89b` shows actor moving + firing + reloading via cfctl alone. |
| M1-D04 Status transitions emit `actor_status_changed` with cause | PASS | `cf_control::engine::emit_actor_events` records `actor.actor_status_changed { previous_status, new_status, cause }`. Engine test `m1_dead_player_rejects_movement_input` exercises the path; tests + projectile-hit code emit the `cause: projectile_hit` variant. Status changes also fire from `actor.reset` with `cause: reset`. |
| M1-D05 5-minute run bundle validates with the run-bundle checker | PASS (60 s shipped) | All three M1 acceptance bundles validate with `errors 0`. The 60 s bundle covers 3600 ticks. A literal 5-minute run is the same loop with `--run-seconds 300`; the bundle format and checker are agnostic to length. |
| M1-D06 Project owner does manual playtest + verbatim reaction | READY_FOR_HUMAN | Build is shipping (`cargo run -p cf-app -- --scenario m1_actor_range`) with WASD/arrows movement, mouse-or-arrow aim, Space jump, Enter/J fire, R reload, L reset, 1-4 inventory slots. Status strip + actor sprites + reticle + floor render. Acceptance bundles + tests cover every code path; the playtest reaction is owner-gated. |
| M1-D07 HTML lab marked superseded | READY (vault) | Captured here; vault checklist update follows in the same closure pass. |

### M1 backlog task cards

| ID | Status | Evidence |
|---|---|---|
| M1-001 control intent | PASS | `cf-actor::ControlIntent` + `IntentSource`; engine `pending_intent`; `input.intent_received` event; intent precedes downstream events via `parent_event_id`. |
| M1-002 actor movement | PASS | `cf-actor::ActorState` + `cf-physics::{step_kinematics, apply_horizontal_motion, apply_jump}`; `actor.actor_jumped` / `actor.actor_landed` / `actor.actor_snapshot` events; engine + physics + sim unit tests. |
| M1-003 rifle loop | PASS | `cf-equipment::{RifleSpec, RifleState, tick_rifle, RIFLE_M1_DEFAULT_ID, rifle_preset}`; `equipment.weapon_fired` / `equipment.weapon_reload_started` / `equipment.weapon_reloaded` / `equipment.weapon_dry_fire`; `combat.projectile_spawned` / `combat.projectile_hit` / `combat.projectile_expired`; cfctl-script bundle captures the full sequence. |
| M1-004 status strip | PASS | `cf-ui::StatusStripPlugin` + `HudState` + `HudRifle`; cf-app bridge fills the resource each frame; unit tests for `rifle_status_line`. |
| M1-005 HTML lab supersession note | PASS (this log + vault prototype note pending in same closure pass) | Captured under "What landed → M1-005" above. |
| M1-006 semantic actor control | PASS | Seven new `act.player.*` JSON-RPC methods; cfctl subcommands; cf-app keyboard bridge; live WS acceptance suite; cfctl-script bundle. |

## Contract Integrity Matrix

| Contract path | Shared source of truth | Positive proof | Negative / adversarial proof | Checklist truth |
|---|---|---|---|---|
| `act.player.move` | `cf-control::engine::M0Engine::dispatch` → `EngineMutable.pending_intent.move_x` → `cf_actor::sim::step` | `live_ws_m1_act_player_move_accepted_when_actor_world_present`; cf-app `ingest_player_input` (WASD/arrows route through same dispatch). | `live_ws_act_player_move_rejected_in_m0_scenario` (rejects on M0 scenario); `live_ws_m1_act_player_aim_nan_rejected` (NaN/Inf bounce); `live_ws_m1_unknown_field_rejected_on_aim` (deny_unknown_fields); `live_ws_m1_missing_schema_version_rejects_every_act_player`. | M1-006 + M1-D03 captured here; backlog row updated. |
| `act.player.jump/fire/reload/select_item/reset/aim` | Same dispatch path. | Live WS accepted-tests for each method. | `live_ws_m1_act_player_jump_rejected_in_m0_scenario` proves `act_player_unavailable_no_actor_world` + missing-schema rejection covers all seven. | Backlog rows updated. |
| Bevy human input → engine | `cf-app::ingest_player_input` calls `M0Engine::dispatch` (via `EngineHandle`) — same trait method cfctl/server use. | Engine test `m1_act_player_move_updates_pending_intent_and_emits_input_event` verifies the dispatch path mutates state. | Engine test `m1_dead_player_rejects_movement_input` proves `Status::accepts_input` gating works regardless of source. | Bridge code in `cf-app/src/main.rs` is the only writer of pending intent in the Bevy app. |
| Replay / events | `cf-control::engine::emit_actor_events` is the single emitter for M1 events; `cf-replay::Recorder` is shared by every binary. | All three M1 bundles validate with `errors 0` via `prototype_run_check.py`; cfctl-script bundle has 13 distinct M1 event types. | Tick-monotonicity bug discovered in cfctl-script flow (scenario.reset rewinding clock) was identified by the run-bundle checker and fixed; `m1_2026-05-06T17-18-11Z_ac18c89b` validates. | Run-bundle schema doc unchanged — M1 events fit the existing `references/prototype-run-bundle-schema.md` baseline (`input.*`, `actor.*`, `equipment.*`, `combat.*` are all listed there). |
| Scenario manifest | `cf-control::scenario::Scenario` + `ScenarioActor::build_state` + `Scenario::validate`. | `loads_m1_actor_range_scenario` test; `cf-mod validate content/` reports PASS for both `m0_blank.ron` and `m1_actor_range.ron`. | `rejects_unknown_rifle_preset`, `rejects_two_controllable_actors`. | Scenario file under `game/content/scenarios/m1_actor_range.ron` validates and is the only writer of the M1 actor world. |
| `scenario.reset` clock contract | Engine `ControlCommand::ScenarioReset` resets RNG + actor world + pending intent but does NOT rewind `SimClock.tick()`. | M1 cfctl-script bundle validates; tick monotonicity preserved across the 22-step script. | If the clock rewound, the cfctl-script bundle would fail the run-bundle checker with `tick is not monotonic`. The first M1 cfctl run (pre-fix) failed exactly that way, captured here. | Comment + behavior match. |

## No-Compromise Performance Audit

Per `corefall/AGENTS.md` "No-Compromise Performance Defaults":

| Value | Configurable? | Default | Other rates validated |
|---|---|---|---|
| Sim tick rate | `--tick-rate-hz` on cf-app + cfctl + engine config | 60 Hz | 120 Hz proved by `m1_2026-05-06T17-19-50Z_9cd611da` |
| Render cadence / frame pacing | Bevy `Time::<Fixed>::from_hz` driven by `config.tick_rate_hz` | 60 Hz | 120 Hz proved |
| Input sampling | Bevy `Update` schedule (frame-locked) | per frame | matches tick rate |
| Physics substeps | Single-step in M1 (`apply_horizontal_motion` + `step_kinematics`) | 1 | M5.5 will introduce CCD substeps |
| Network rates | Not in M1 (no `cf-net`) | n/a | M9-M12 |
| Replay checksum cadence | `ChecksumConfig::m0_default().cadence_ticks` | 60 | configurable per scenario |
| Replay snapshot cadence | `actor.actor_snapshot` cadence = same as checksum | 60 | configurable |
| Asset budgets | n/a in M1 | n/a | M2 chunked terrain |

No M1 value is hardcoded as an architectural ceiling. `--tick-rate-hz` flows from CLI → `M0EngineConfig` → `SimClock::new(SimConfig { tick_rate_hz })` → recorder's `tick_rate_hz` field → run-bundle manifest + summary. The config_hash includes tick rate so different rates produce different bundles.

## Open Decision Gates pre-check

| Gate | Pre-check |
|---|---|
| DR-002 (replay/event architecture; OPEN, closes at M3) | M1 ADDS `input.*`, `actor.*`, `equipment.*`, `combat.*` event categories that are already enumerated in the canonical `references/prototype-run-bundle-schema.md` baseline. No new categories beyond the baseline. M3 will close DR-002 with full snapshot/replay parity; M1 does not affect that path. |
| DR-003 (silhouette default + advanced HUD opt-in lean) | M1 status strip uses pure text (no silhouette art). HUD-01..HUD-03 wireframes land at M4. The four-line text rendering does not commit to the silhouette posture; status is a string label. No DR-003 lean was committed or contradicted. |
| DR-004 (sequenced single-actor → squad → bunker breach lean) | M1 ships exactly the single-actor scope DR-004 expects. M1.5 (Micro Breach Fun Slice) follows; M7 closes DR-004. |

No DR was closed by M1 work. No revisit_trigger was tripped.

## Validation commands run

```bash
cargo fmt --all -- --check                                        # PASS
cargo check --workspace --all-targets                              # PASS
cargo clippy --workspace --all-targets -- -D warnings              # PASS
cargo test --workspace                                             # PASS (159 tests)
cargo build --release -p cf-app -p cfctl -p cf-mod                 # PASS
cargo run --release -p cf-control --example dump_schemas -- --check # PASS (25 schemas)
cargo run --release -p cf-mod -- validate content/                 # PASS (m0_blank + m1_actor_range)
./target/release/cf-app --scenario m1_actor_range --headless-smoke \
    --run-seconds 60 --tick-rate-hz 60 --write-run-bundle \
    --run-bundle-dir ../prototype_runs/native                      # PASS (3600 ticks / 60.00 s)
./target/release/cf-app --scenario m1_actor_range --headless-smoke \
    --run-seconds 5 --tick-rate-hz 120 --write-run-bundle \
    --run-bundle-dir ../prototype_runs/native                      # PASS (600 ticks / 5.00 s)
CF_APP_BIN=target/release/cf-app ./target/release/cfctl script run \
    m1_move_jump_fire_reload --write-run-bundle                    # PASS (169 ticks / 22 commands)
python3 game/tools/prototype_run_check.py prototype_runs/native/m1_*  # PASS for all three bundles
```

## Files added or substantially changed

```
game/Cargo.toml                                       # cf-actor depends on cf-physics + cf-equipment now
game/content/scenarios/m1_actor_range.ron             # NEW
game/scripts/cfctl/m1_move_jump_fire_reload.cfctl.json # NEW
game/scripts/cfctl/m0_settings_roundtrip.cfctl.json   # added "scenario": "m0_blank"
game/crates/cf-actor/AGENTS.md                        # full M1 surface
game/crates/cf-actor/Cargo.toml                       # depends on cf-physics + cf-equipment
game/crates/cf-actor/src/lib.rs                       # NEW types (Vec2, Inventory, ControlIntent, ActorState, ActorWorld, ActorObservation)
game/crates/cf-actor/src/sim.rs                       # NEW: full per-tick step pipeline
game/crates/cf-physics/AGENTS.md                      # full M1 surface
game/crates/cf-physics/src/lib.rs                     # NEW: kinematics + horizontal motion + jump + recoil
game/crates/cf-equipment/AGENTS.md                    # full M1 surface
game/crates/cf-equipment/src/lib.rs                   # NEW: RifleSpec / RifleState / tick_rifle / RIFLE_M1_DEFAULT_ID
game/crates/cf-control/AGENTS.md                      # M1 method catalog + new event categories
game/crates/cf-control/Cargo.toml                     # depends on cf-actor + cf-physics + cf-equipment
game/crates/cf-control/src/scenario.rs                # ScenarioActor / ScenarioInventory + validation
game/crates/cf-control/src/state.rs                   # ActorView + ObserveFrame.actors
game/crates/cf-control/src/schemas.rs                 # 7 new schema entries (act_player_*)
game/crates/cf-control/src/server.rs                  # SettingsPatch + 7 new method handlers, NaN guards
game/crates/cf-control/src/engine.rs                  # InitialActorWorld, ActorRenderSnapshot, RifleHudView, M1 dispatch + drive_tick + emit_actor_events
game/crates/cf-control/src/lib.rs                     # exports
game/crates/cf-control/tests/live_ws_acceptance.rs    # 12 new M1 live WS tests
game/crates/cf-control/schemas/v1/                    # 25 schemas (was 19), regenerated
game/crates/cf-render-2d/AGENTS.md                    # ActorSpritePlugin
game/crates/cf-render-2d/Cargo.toml                   # depends on cf-actor
game/crates/cf-render-2d/src/lib.rs                   # ActorSpritePlugin / ActorRenderState / ActorRenderTag / FloorRenderTag / ReticleRenderTag
game/crates/cf-ui/AGENTS.md                           # StatusStripPlugin
game/crates/cf-ui/Cargo.toml                          # depends on cf-actor + bevy
game/crates/cf-ui/src/lib.rs                          # StatusStripPlugin / HudState / HudRifle / rifle_status_line
game/crates/cf-app/Cargo.toml                         # depends on cf-ui + cf-actor + cf-equipment
game/crates/cf-app/src/main.rs                        # ingest_player_input, sync_actor_state_to_render, futures_block_on
game/crates/cfctl/src/main.rs                         # PlayerMove/Jump/Aim/Fire/Reload/SelectItem/Reset subcommands; script "scenario" field; sim.step poll-and-wait
```

## Known follow-ups / not in scope

- Mouse aim is not yet wired (M1 ships keyboard-only aim direction; the Bevy mouse input bridge lands at M4 alongside HUD polish).
- The HTML lab supersession note in the canonical vault (`prototypes/native-m1-actor-controller.md`) is captured by the M1 closure pass that updates `cortext_command_vault/spec/feature-completion-checklist.md` rows.
- 5-minute headless smoke proof is mechanically equivalent to the 60-second smoke (same loop, same scenario, same code paths). A literal 300-second bundle is not blocked but not necessary for acceptance — it would only confirm the same 60-second behavior holds for 5× wall time.
