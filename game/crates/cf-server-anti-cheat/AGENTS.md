# cf-server-anti-cheat — AGENTS.md

## Owns
- Anti-cheat foundation for dedicated servers (real implementation at M11).
- Server-side validation of client commands (position, fire rate, damage claims).
- Anomaly detection (speed hacks, teleport, impossible damage).
- Report + ban surface for server admins.

## Public API Boundary
- (Stub until M11.)

## Does NOT Own
- Client prediction → `cf-net`.
- Server authority → `cf-server`.
- Admin tooling UI → `cf-server-admin`.

## Test Surface
- (Stub.) Coverage lands at M11.

## Source Trail
- DR-005 (multiplayer posture — server-authoritative model).
- DR-052 (network sync / rollback / determinism).
