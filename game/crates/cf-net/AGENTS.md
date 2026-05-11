# cf-net — AGENTS.md

## Owns
- Client/server transport (real implementation pending M9 / DR-005).
- Transport library selection (Lightyear / renet / quinn — deferred to M9/M10).
- Client prediction + server reconciliation.
- Network event serialization for co-op and PvP.

## Public API Boundary
- (Stub until M9.)

## Does NOT Own
- Server logic → `cf-server`.
- Game state → `cf-sim-core`, `cf-actor`, `cf-terrain`, etc.
- Replay/event recording → `cf-replay`.

## Test Surface
- (Stub.) Real coverage lands at M9.

## Cross-Crate Contracts
- Will depend on: `cf-sim-core`, `cf-replay`.
- Will be depended on by: `cf-server`, `cf-app`.

## Source Trail
- DR-005 (multiplayer posture; CLOSED direction).
- DR-052 (network sync / rollback; CLOSED direction).
