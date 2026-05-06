# cf-actor — AGENTS.md

## Owns
- M1 actor primitives: `ActorId`, `Status`, `Inventory`, `InventoryItem`, `ItemSlot`, `Vec2`, `IntentSource`, `ControlIntent`, `ActorState`, `ActorWorld`, `ActorObservation`.
- `sim` module: per-tick `step` function that consumes a `BTreeMap<ActorId, ControlIntent>`, drives `cf-physics::{step_kinematics, apply_horizontal_motion, apply_jump, apply_recoil}`, ticks `cf-equipment::RifleState` per actor, spawns + flies projectiles, and emits a `StepReport`.
- `ActorSimState`: actor world + per-actor rifle state + projectile pool. Cloned cheaply into `cf-control`'s engine.

## Public API Boundary
- Types: `ActorId`, `Status`, `Inventory`, `InventoryItem`, `ItemSlot`, `Vec2`, `IntentSource`, `ControlIntent`, `ActorState`, `ActorWorld`, `ActorObservation`.
- Module: `sim::{ActorSimState, ActorTickOutcome, ActorTuning, ExpiredProjectile, HitOutcome, Projectile, RifleStates, SpawnedProjectile, StepDeps, StepReport, step}`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `input.*` / `actor.*` / `equipment.*` / `combat.*` events from the `StepReport`; the recorder lives in `cf-replay`.
- Bevy ECS components → `cf-render-2d` / `cf-app` wrap these types in components.
- Chassis layers (armor zones, modules, pilot binding) → `cf-chassis` (lands at M5).
- Scenario manifest types → `cf-control::scenario::ScenarioActor` builds an `ActorState` via `build_state`.

## Test Surface
- Unit tests: `cargo test -p cf-actor` covers status thresholds, reset, inventory selection, intent edge clearing, checksum byte stability, sim step idle/move/jump/fire/projectile-hit/dead-actor/reset/determinism.

## Cross-Crate Contracts
- Depends on: `cf-physics`, `cf-equipment`.
- Depended on by: `cf-control`, `cf-render-2d`, `cf-ui`, `cf-app`.
- The `step` function is the single sim-side entry point; `cf-control` calls it once per tick on the player's pending intent.

## Common Pitfalls
- Edge-triggered intent fields (`jump`, `fire`, `reload`, `selected_item`, `reset`) MUST be cleared by the engine after a tick consumes them; continuous fields (`move_x`, `aim`) persist. The engine handles this in `EngineMutable::pending_intent.clear_edges()`.
- `Status::accepts_input` returns false for `Downed`/`Dead`. The sim still runs physics (so a downed actor falls); it just ignores movement/fire/reload intent.
- `Vec2::normalize_or_x` returns `(1, 0)` on a zero input so muzzle origin / projectile velocity never produce NaNs.

## Source Trail
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core.
- spec/native-implementation-backlog M1-001..M1-006.
- DR-003 (body damage readability; OPEN — closes at M4).
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
