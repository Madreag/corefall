# cf-mission — AGENTS.md

## Owns
- **M1.5 mission state machine**: `Objective`, `ObjectiveStatus`, `MissionState`, `MissionResult`, `LossConditions`.
- **`ObjectiveKind` variants**:
  - `BreachBarrier { target }` — break the breach strip with the given id (M1.5).
  - `NeutralizeActor { target }` — drive the named actor to `Status::Dead` (M1.5).
  - `ReachZone { min, max }` — the player's position lies inside the AABB (M1.5).
  - `DefendReactor { target }` — survive the mission timer with the named reactor still alive (M2.5).
- **`LossReason` variants**: `PlayerDead`, `TimerExpired`, `ReactorDestroyed` (M2.5), `ObjectiveFailed` (reserved).
- **M2.5 reactor primitives**: `Reactor` (id + position + half_extents + hp + max_hp + destroyed flag) with `apply_damage` (latched: a destroyed reactor cannot un-destroy and ignores subsequent damage), `is_destroyed`, `aabb_contains`, `reset`, `checksum_bytes`. `ReactorWorld` (BTreeMap<id, Reactor>) with `iter`, `iter_mut`, `is_destroyed`, `destroyed_map`, `reset`, `checksum_bytes`.
- Pure-function `step(state, inputs) -> MissionTickReport` that the engine calls once per tick after the actor world settles. Evaluates: (1) `DefendReactor` reactor-destroyed loss BEFORE the timer-expiry check so simultaneous-tick destruction wins record the right loss reason; (2) timer-expired branch handles mixed-objective scenarios (DefendReactor + others) by resolving on the timer-expired tick — Won if all required objectives complete, TimerExpired loss otherwise; (3) per-objective progress in declaration order; (4) win-condition check.
- `MissionView` / `ObjectiveView` projections (with `target_actor`, `target_breach`, `target_reactor`, `zone_min`, `zone_max`) used by the engine to populate the JSON-RPC observe envelope.
- `MissionState::reset(tick)` and `ReactorWorld::reset` for the engine to rewind on `scenario.reset` without rebuilding from disk.
- Anti-scope: no command-core, no commander AI, no comic-noir mission cards, no full director — those land at M7.

## Public API Boundary
- Types: `Objective`, `ObjectiveKind` (with the four BP2 variants above), `ObjectiveStatus`, `LossReason`, `LossConditions`, `MissionResult`, `MissionState`, `MissionView`, `ObjectiveView`, `MissionTickInputs`, `MissionTickReport`, `Reactor`, `ReactorWorld`.
- Functions: `step(&mut MissionState, MissionTickInputs) -> MissionTickReport`, `MissionView::from_state`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `mission.*` events from the `MissionTickReport`.
- Actor world / damage routing → `cf-actor`.
- Reactor projectile-collision routing → `cf-control::M0Engine::drive_tick` (per-hit AABB test against the `ReactorWorld` after the actor step).
- Breach state → `cf-terrain` (M1.5 BreachStrip + M2 ChunkedTerrain).
- Reactive enemies → `cf-ai`.

## Test Surface
- Unit tests: `cargo test -p cf-mission` covers:
  - **M1.5 paths**: first-objective activation, breach completion → neutralize advance, full clear win, player-dead loss, timer-expiry loss, terminal idempotency, reset rewind, MissionView round-trip.
  - **M2.5 reactor paths**: defend_reactor loses when reactor destroyed (and `state.objectives[0].status` flips to `Failed` — Devin regression), defend_reactor wins when timer expires with reactor alive, mixed-objective DefendReactor + ReachZone resolves as TimerExpired loss when other objectives are still pending (Devin regression for the timer-expiry / next-tick race), `Reactor::apply_damage` drives destruction and is no-op once destroyed, `ReactorWorld::destroyed_map` round-trip, `Reactor::aabb_contains` inside + outside.

## Cross-Crate Contracts
- Depends on: `cf-actor` (reads `ActorState` / `Status` from inputs).
- Depended on by: `cf-control` (engine + scenario + observe envelope + run-bundle event emission).
- Events emitted by the engine from a `MissionTickReport`: `mission.objective_started`, `mission.objective_completed`, `mission.objective_failed`, `mission.mission_resolved`.
- Events emitted by the engine from `ReactorWorld` damage routing (after the actor step): `combat.projectile_hit { target_kind: reactor }`, `actor.reactor_damaged { hp, hp_max, destroyed, damage_applied }`, `actor.actor_status_changed { actor_kind: reactor, new_status: destroyed, cause: projectile_hit }` (emitted EXACTLY ONCE per reactor on the hit that flipped destroyed; per-hit state captured in the retain loop, not read post-loop, to avoid duplicate destruction events when multiple projectiles hit the same reactor in a single tick).

## Common Pitfalls
- Loss conditions are evaluated BEFORE objective completion in the same tick. A player who dies on the same tick they would have completed the final objective records a loss, not a win — that matches the M7 director's failure ordering.
- `step` is idempotent once `MissionResult` is terminal (`Won` / `Lost`). After a final result, `step` returns an empty report.
- Only the first un-completed required objective progresses per tick. Optional rows are not auto-skipped — they remain `Pending` unless their kind is met.
- `DefendReactor` only completes via the timer-expired branch — passive ticks DO NOT auto-complete it. The reactor-destroyed branch sets the objective's `status` field to `Failed` (mutating through index after the immutable scan that found the failing index) so `MissionView::from_state` reports `failed` correctly.
- The reactor-destroyed branch runs BEFORE the timer-expiry branch so a same-tick "reactor destroyed exactly when timer expires" resolves as `ReactorDestroyed` loss, not `TimerExpired`.
- Mixed-objective scenarios with `DefendReactor + (ReachZone | NeutralizeActor | BreachBarrier)` resolve on the timer-expired tick itself (not the next one) to avoid the latent bug where a completed DefendReactor on tick T followed by another tick T+1 with no defend_active_alive would route to `TimerExpired` loss inappropriately. Resolved by completing DefendReactor + then immediately checking the win condition at the same tick — Won iff all required complete; TimerExpired otherwise.

## Source Trail
- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice.
- spec/prototype-roadmap §M2.5 — Micro Reactor Defense Fun Slice.
- spec/native-implementation-backlog M1.5-001..M1.5-006 + M2.5-001..M2.5-006.
- DR-002 (replay/event architecture, OPEN — closes at M3B).
- DR-004 (sequenced single-actor → squad → bunker breach lean — confirmed unchanged; M7 closes the DR).
- corefall/docs/implementation-log/2026-05-08-bp2-terrain-replay-build.md.
