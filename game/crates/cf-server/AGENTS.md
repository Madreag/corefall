# cf-server — AGENTS.md

## Owns
- (M0 stub) Will own the dedicated server binary (`--mode coop_room|pvp_arena|lan_room|mmo_shard|lobby_directory`).

## Common Pitfalls
- Same Rust workspace as the client; do NOT fork sim logic for server vs client (DR-034 anti-goal).

## Source Trail
- spec/server-app-architecture.
- DR-034 (CLOSED-DIRECTION) / DR-005 (CLOSED-DIRECTION).
