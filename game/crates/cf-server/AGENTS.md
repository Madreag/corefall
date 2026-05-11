# cf-server — AGENTS.md

## Owns
- Multi-mode dedicated server binary (real implementation pending M9).
- Modes: `coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`, `lobby_directory`.
- Server-authoritative fixed-tick simulation.
- Community-hostable single binary.

## Public API Boundary
- (Stub until M9.)

## Does NOT Own
- Transport → `cf-net`.
- Persistence → `cf-server-persistence`.
- Anti-cheat → `cf-server-anti-cheat`.
- Admin tooling → `cf-server-admin`.
- Ops dashboards → `cf-server-ops`.

## Test Surface
- (Stub.) SERVER-001..016 acceptance suite begins at M9.

## Source Trail
- DR-005 (multiplayer posture).
- DR-034 (dedicated server).
