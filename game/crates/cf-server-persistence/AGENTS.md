# cf-server-persistence — AGENTS.md

## Owns
- MMO shard persistence layer (real implementation at M12 / DR-035).
- Persistent terrain state, base state, faction state, veteran actor state.
- Save/load for server-side world snapshots.
- Database adapter (SQLite for dev, PostgreSQL for production shards).

## Public API Boundary
- (Stub until M12.)

## Does NOT Own
- Client-side saves → `cf-save`.
- Server logic → `cf-server`.
- Shard topology → `cf-server` mode selection.

## Test Surface
- (Stub.) Coverage lands at M12.

## Source Trail
- DR-035 (persistent MMO architecture).
- DR-029 (save-game model — client-side complement).
