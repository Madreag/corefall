# cf-server-ops — AGENTS.md

## Owns
- Ops dashboards + observability for dedicated servers (real implementation at M9).
- Server health metrics, player count, tick budget, memory usage.
- Prometheus/OpenTelemetry exporter surface.
- Admin dashboard backend (JSON API for ops tooling).

## Public API Boundary
- (Stub until M9.)

## Does NOT Own
- Server logic → `cf-server`.
- Anti-cheat → `cf-server-anti-cheat`.
- Persistence → `cf-server-persistence`.

## Test Surface
- (Stub.) Coverage lands at M9.

## Source Trail
- DR-034 (dedicated server).
- DR-047 (launch and live operations).
