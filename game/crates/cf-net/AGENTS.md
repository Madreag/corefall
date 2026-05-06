# cf-net — AGENTS.md

## Owns
- (M0 stub) Will own authority, snapshot/event hybrid replication, transport adapter for the dedicated server.

## Common Pitfalls
- Networking transport library (lightyear vs renet vs quinn) is OPEN. Do NOT pick one before M9/M10 prototyping; confirm with user before locking.

## Source Trail
- spec/server-app-architecture.
- spec/persistent-mmo-architecture.
- DR-005 / DR-013 / DR-034 / DR-035.
