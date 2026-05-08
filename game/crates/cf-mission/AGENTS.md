# cf-mission — AGENTS.md

## Owns
- M1.5 mission state machine: `Objective`, `ObjectiveKind` (`BreachBarrier`/`NeutralizeActor`/`ReachZone`), `ObjectiveStatus`, `MissionState`, `MissionResult`, `LossReason`, `LossConditions`.
- Pure-function `step(state, inputs) -> MissionTickReport` that the engine calls once per tick after the actor world settles.
- `MissionView`/`ObjectiveView` projections used by the engine to populate the JSON-RPC observe envelope.
- `MissionState::reset(tick)` for the engine to rewind objectives + result + timer on `scenario.reset` without rebuilding from disk.
- Anti-scope: no command-core, no commander AI, no comic-noir mission cards, no full director — those land at M7.

## Public API Boundary
- Types: `Objective`, `ObjectiveKind`, `ObjectiveStatus`, `LossReason`, `LossConditions`, `MissionResult`, `MissionState`, `MissionView`, `ObjectiveView`, `MissionTickInputs`, `MissionTickReport`.
- Functions: `step(&mut MissionState, MissionTickInputs) -> MissionTickReport`, `MissionView::from_state`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `mission.*` events from the `MissionTickReport`.
- Actor world / damage routing → `cf-actor`.
- Breach state → `cf-terrain`.
- Reactive enemies → `cf-ai`.

## Test Surface
- Unit tests: `cargo test -p cf-mission` covers first-objective activation, breach completion → neutralize advance, full clear win, player-dead loss, timer-expiry loss, terminal idempotency, reset rewind, and `MissionView` round-trip.

## Cross-Crate Contracts
- Depends on: `cf-actor` (reads `ActorState`/`Status` from inputs).
- Depended on by: `cf-control` (engine + scenario + observe envelope + run-bundle event emission).
- Events emitted by the engine from a `MissionTickReport`: `mission.objective_started`, `mission.objective_completed`, `mission.objective_failed`, `mission.mission_resolved`.

## Common Pitfalls
- Loss conditions are evaluated BEFORE objective completion in the same tick. A player who dies on the same tick they would have completed the final objective records a loss, not a win — that matches the M7 director's failure ordering.
- `step` is idempotent once `MissionResult` is terminal (`Won` / `Lost`). After a final result, `step` returns an empty report.
- Only the first un-completed required objective progresses per tick. Optional rows are not auto-skipped — they remain `Pending` unless their kind is met.

## Source Trail
- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice.
- spec/native-implementation-backlog M1.5-001..M1.5-006.
- DR-002 (replay/event architecture, OPEN — closes at M3).
- DR-004 (sequenced single-actor → squad → bunker breach lean — confirmed unchanged; M7 closes the DR).
