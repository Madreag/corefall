# cf-control — AGENTS.md

## Owns
- JSON-RPC 2.0 envelope types (`JsonRpcRequest`/`JsonRpcResponse`/`JsonRpcNotification`/`JsonRpcError`).
- Method catalog for M0: `scenario.load`, `scenario.reset`, `sim.pause`, `sim.resume`, `sim.step`, `sim.run_for_ticks`, `observe.once`, `observe.subscribe`, `observe.unsubscribe`, `observe.frame` (notification), `observe.settings`, `act.settings.set`, `runbundle.write`, `system.shutdown`.
- Method catalog extension for M1: `act.player.move`, `act.player.jump`, `act.player.aim`, `act.player.fire`, `act.player.reload`, `act.player.select_item`, `act.player.reset`. All seven are gated on the loaded scenario carrying typed `actors[]`; M0 scenarios (`m0_blank`) reject every `act.player.*` with `act_player_unavailable_no_actor_world` (or the legacy `act_player_move_not_available_in_m0` for the move method).
- Local control server (`ControlServer`) on `127.0.0.1:17890` (loopback only).
- DR-012 lock: `Settings` resource (six accessibility flags) + observability via `observe.settings`. **M4A closure**: live `act.settings.set` round-trips through `apply_settings_patch`; `current_settings()` accessor for cf-app's HUD bridge; `ObserveFrame.accessibility` surfaces `ui_scale_applied` + `high_contrast_applied` + `captions_visible` + `reduced_*_applied` + 12-id `focusable_nodes` contract.
- **M4A HUD-cache surface**: `EngineMutable.hud_banners` (`VecDeque<HudBannerView>`, capped at 8) + `hud_captions` + `hud_tool_validity` + `hud_last_status` diffing cursor + `hud_last_mission_result` cursor; refreshed at end of every `drive_tick` via `refresh_hud_caches`. Banner queue raises critical/warning/info banners from status diffs (HP_LOW / ARMOR_CRACKED / EJECT_NOW), ammo state (AMMO_OUT — sticky/dedup), and mission resolution (MISSION_WON / MISSION_FAILED). Caption queue surfaces `status_changed.<actor_id>` events as text. Tool-validity tracker updates per `act.player.dig` outcome (last_carve_tick / last_refusal_tick / reason / target). cf-app reads via `hud_caches_snapshot()`.
- **M4A ActorView extensions**: `stance` + `body_silhouette` + `module_strip` mirror `cf_actor::ActorObservation` projections. `build_module_strip_view` derives the placeholder weapon_mount + jet/shield/sensor surface from the rifle state + selected slot.
- Inline `M0Engine` + `run_m0_inline` driver shared by `cf-app` and `cfctl`. M1 extends the engine with an actor world (`cf_actor::sim::ActorSimState`), per-actor rifle state, projectiles, and per-tick `input.*` / `actor.*` / `equipment.*` / `combat.*` event emission.
- Scenario loader for the M0 + M1 manifest shape (`ScenarioActor` + `ScenarioInventory`).
- Bevy-bridge actor snapshot (`actor_render_snapshot()`) returning `ActorRenderSnapshot { tick, floor_y, actors, player_actor_id, player_rifle: Option<RifleHudView> }` for `cf-app`.

## Public API Boundary
- Types: `Settings`, `ObserveFrame`, `ObserveSettings`, `ActorView`, `EngineState`, `ControlEnvelopeStatus`, `RunStatus`, `Scenario`, `ScenarioActor`, `ScenarioInventory`, `ScenarioLoadError`, `M0Engine`, `M0EngineConfig`, `M0EngineOutcome`, `InitialActorWorld`, `ActorRenderSnapshot`, `RifleHudView`.
- Functions: `run_m0_inline`.
- Server: `ControlServer`, `ControlServerConfig`, `EngineHandle`, `ControlCommand`, `CommandResult`.
- Constant: `SCHEMA_VERSION = 1`.

## Does NOT Own
- Render/UI → `cf-render-2d`/`cf-ui`.
- Sim core / RNG → `cf-sim-core`.
- Run-bundle envelope / event taxonomy → `cf-replay`.
- Network transport for multiplayer → `cf-net` (decision deferred to M9).

## Test Surface
- Unit tests: `cargo test -p cf-control`.
- Schema mismatch returns `-32602` with fix-hint.
- Unknown method returns `-32601`.
- `observe.once` returns a frame matching `SCHEMA_VERSION`.

## Cross-Crate Contracts
- Depends on: `cf-sim-core`, `cf-replay`, `cf-actor`, `cf-physics`, `cf-equipment`.
- Depended on by: `cf-app`, `cfctl` (and later `cf-server`, `cf-e2e`, `cf-tools-editor`).
- Events emitted (via injected `Recorder`):
  - M0: `control.command_accepted`, `control.command_rejected`, `control.observation_sent`, `control.settings_observed`, `control.settings_changed`, `system.run_started`, `system.run_finished`, `system.tick_sample`, `system.panic`, `determinism.sim_checksum`.
  - M1: `input.intent_received` (every tick when an actor world is loaded), `actor.actor_status_changed`, `actor.actor_reset`, `actor.actor_jumped`, `actor.actor_landed`, `actor.actor_snapshot` (cadence 60), `equipment.selected_item_changed`, `equipment.weapon_reload_started`, `equipment.weapon_reloaded`, `equipment.weapon_dry_fire`, `equipment.weapon_fired`, `combat.projectile_spawned`, `combat.projectile_hit`, `combat.projectile_expired`.

## Common Pitfalls
- Every request param object MUST carry `schema_version: 1`. Mismatches return `-32602`.
- The control server binds to loopback by default. Remote bind must require an auth token (see DR-005 / DR-013) — NOT in M0.
- The `EngineHandle` trait is async. Adding sync calls inside it will deadlock the WebSocket runtime.
- `scenario.reset` does NOT rewind `SimClock.tick()`. Rewinding would violate `events.jsonl` monotonicity once any events were already recorded at higher ticks. Reset is a content reload (RNG + actor world + pending intent), not a time-warp.
- Edge-triggered `act.player.*` fields (`jump`, `fire`, `reload`, `selected_item`, `reset`) live on `EngineMutable.pending_intent`; `drive_tick` consumes them and calls `clear_edges()`. Continuous fields (`move_x`, `aim`) persist tick-to-tick.

## Source Trail
- spec/ai-control-observability-layer.
- spec/prototype-roadmap §Control Transport And Envelope.
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core.
- DR-012 (accessibility floor; OPEN — closes at M4).
- docs/implementation-log/2026-05-05-m0-engine-bootstrap.md §DR-012 floor lock.
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
