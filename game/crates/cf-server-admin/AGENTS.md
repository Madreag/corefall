# cf-server-admin — AGENTS.md

## Owns
- Admin tooling for dedicated servers (real implementation at M9).
- Player management (kick, ban, mute, whitelist).
- Server configuration hot-reload.
- Match/session management (start, stop, restart scenarios).
- RCON-style remote admin interface.

## Public API Boundary
- (Stub until M9.)

## Does NOT Own
- Server logic → `cf-server`.
- Ops metrics → `cf-server-ops`.
- Anti-cheat → `cf-server-anti-cheat`.

## Test Surface
- (Stub.) Coverage lands at M9.

## Source Trail
- DR-034 (dedicated server).
- DR-013 (backend service scope).
