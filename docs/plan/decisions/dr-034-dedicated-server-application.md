---
type: decision
id: DR-034
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "Single-binary multi-mode server proves infeasible; community hosting demand is too low to justify the per-mode investment; or platform-cert process forces a fork between client and server."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/server-app-architecture|server app architecture]] · [[spec/persistent-mmo-architecture|persistent MMO architecture]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-013-backend-service-scope|DR-013]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]]

# DR-034: Dedicated Server Application And Community Hosting

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-05)
> Ship a dedicated server binary `cf-server` as a full-product artifact, designed from day one to host LAN co-op, online co-op, PvP arena, persistent MMO shard, and lobby-directory modes. Same sim core, terrain, physics, equipment, chassis, AI, replay, and modding crates as the client. Community-hostable by default; platform adapters (Steam/EOS/PlayFab/Unity) are optional. See [[spec/server-app-architecture]] for the full architecture.

## Decision

**One binary, multiple modes, community-hosted by default.** `cf-server` is a Rust binary in the same workspace as the client (per DR-024); selected mode determines lifecycle, persistence, transport, and anti-cheat profile. There is no "server-only" sim path; the server omits render/audio/UI crates only.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Binary | `cf-server` is a full-product artifact and ships on Linux + Windows. macOS server is nice-to-have, not required. |
| Modes | `coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`, `ranked_arena` (post-launch), `lobby_directory`. Mode is a CLI flag + config file. |
| Authority | 100% server-authoritative simulation (per DR-005). |
| Sim parity | Same `cf-sim-core`, `cf-terrain`, `cf-physics`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-ai`, `cf-mission`, `cf-replay`, `cf-control`, `cf-net`, `cf-save`, `cf-mod` crates. No fork. |
| Crates added | `cf-server` (bin), `cf-server-ops`, `cf-server-persistence`, `cf-server-anti-cheat`, `cf-server-admin`. Detailed in [[spec/server-app-architecture]]. |
| Configuration | RON config file; validated by `cf-mod validate`; schema-versioned; migrations registered. |
| Hosting | Community-hostable by default. Reference Docker image + Linux + Windows hosting guide ship at launch. |
| Platform adapters | Steam Datagram Relay, EOS, PlayFab, Unity Multiplay are optional adapters behind the same trait. No platform lock-in. |
| Modding | Server runs the same `cf-mod` packages as the client. Server-only mods marked `server_only: true`. Mod hash sync is mandatory; trust tiers gate per-server admission. |
| Anti-cheat foundation | Server-authoritative input validation; profiles `casual`, `competitive`, `tournament_strict`; ban list persisted; audit log appended. Tournament-grade is post-launch. |
| Observability | Structured JSON logs, Prometheus-compatible metrics endpoint, `/health` and `/ready` endpoints, replay archive directory. |
| Admin API | `cfctl --capability admin` over the same JSON-RPC envelope as the client; capability-gated; opt-in. |
| LLM mind | Mind workers may run server-side per DR-032; clients never see prompts. |
| Documentation | Hosting guide, ops runbook, anti-cheat profile reference, mod compatibility playbook all ship with the server artifact. |

## What This Does NOT Lock

- Specific networking transport library (lightyear / renet / quinn — decided in M9/M10).
- Specific account/identity provider (plug-in; defaults to local + lobby token; Steam/EOS/PlayFab adapters post-launch).
- First-party hosted shards (optional; community-hosted is the default).
- Ranked PvP service (post-launch).
- Voice chat (out of scope; external services).
- Cross-shard live combat or seamless world (post-launch open question; would require DR-035 amendment).

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| Different sim logic for server vs client | Mod compatibility hell; replay determinism risk. |
| A "lite" server stripped of mod support | Defeats the point of community hosting. |
| Forced first-party hosting | DR-031 economy + DR-026 team model both push toward community-hostable. |
| Forced account systems for private LAN/co-op rooms | Solo and private play work without accounts. |
| Naive "trust the client" anywhere beyond explicit debug capability | Cheating risk; replay/determinism risk. |
| Steam-only or platform-only server | Lock-in. Adapters yes, lock-in no. |
| Multiple server binaries per mode | Operational complexity. One binary, multiple modes. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Use Steam dedicated server only | Lock-in; cuts off Linux + Windows community hosting outside Steam. |
| Use a third-party MMO middleware | Cost; lock-in; doesn't fit Cortex sim. |
| Server is a different binary with different sim | Replay/mod chaos; double-implementation cost. |
| Two binaries (one for co-op, one for MMO) | Operational complexity; mode is a config concern, not a binary concern. |
| Peer-to-peer with elected host | Anti-cheat too weak; persistence too fragile. |
| Headless-only (`cf-headless`) without dedicated lifecycle | Misses health/readiness/metrics/anti-cheat foundations operators need. |

## Evidence Trail

- Project owner verbatim (2026-05-05): "I want to have an entire server app designed to run our app. anyone can host multiplayer games, as well as the persistent MMO mode."
- Source patterns: Source dedicated server, Quake/Quake-Live community hosting, Project Zomboid dedicated server, Space Station 13/14 round-based persistence, Minecraft Realms-vs-Bukkit, Steam Datagram Relay + Steam Game Servers, EOS sessions/lobbies, PlayFab modular multiplayer, Unity Multiplay readiness.
- Cross-DR coherence: DR-024 native stack already includes `cf-net` + `cf-headless`; this DR formalizes `cf-server` as the launch artifact built on those.
- DR-026 team/repo model: modular crate boundaries + community-runnable defaults match the AI-augmented-solo team's capacity.
- DR-031 content economy: premium + free modding posture supports community hosting; no marketplace cut.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Operational complexity for community operators | Reference Docker images + minimal config + explicit hosting guide + community templates. |
| Anti-cheat false positives | Tiered profiles; competitive default is `competitive`, not `tournament_strict`; operators can tune. |
| Mod compatibility chaos | Hash sync; trust tiers; clear UI; auto-download off by default. |
| Server lifecycle bugs (drain shutdown, restart hooks) | SERVER-015 acceptance test; reference systemd/Docker configs. |
| Different OS quirks (Linux/Windows path separators, file watching) | T-PLATFORM portability tests apply; CI matrix; Docker abstracts most differences. |
| Cloud-cert tax (Steam/EOS/Sony/MS/Nintendo) | Adapters; can ship Steam without locking out Linux community hosting. |
| AI mind layer cost on server | DR-032 mock-default; operators opt-in to cloud providers per shard config. |
| Replay drift between server and clients | Stable pair ordering; deterministic islands; per-tick checksums; first-divergence reports. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M9 server-core subset | Core server lifecycle, config, replay, health/readiness, admin capability gates, drain shutdown, Docker smoke, and mode boot/config paths. |
| M10 LAN co-op via `cf-server --mode lan_room` | LAN auto-discovery; ready-up; mission completion; per-client bundles align. |
| M11 online co-op via `cf-server --mode coop_room` over NAT/relay | Mod hash sync; latency masking; clean mismatch error. |
| M12 PvP via `cf-server --mode pvp_arena` | 4-8 players; server-authoritative; bandwidth + cheat profile tested. |
| M12 MMO via `cf-server --mode mmo_shard` | 50 simulated clients for 1 hour; persistence; restart resume; interest management. |
| Reference Docker image | Operator pulls image, fills 5 config fields, runs server; friends join. |
| Anti-cheat profile validation | Input-rate-spike client kicked; ban list persists across restart; audit log written. |

## Revisit Trigger

- Single-binary multi-mode server proves infeasible (e.g. `mmo_shard` mode requires structurally different scheduler).
- Community hosting demand is too low to justify the per-mode investment (post-launch evidence).
- Platform certification process forces a fork between client and server (anti-cheat or sandboxing requirements).
- A first-party MMO hosting business model is required for revenue.

## Source Trail

- Project owner direction (2026-05-05).
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-025-target-platforms]]
- [[decisions/dr-026-team-and-repo-model]]
- [[decisions/dr-029-save-game-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-032-hybrid-llm-ai-direction]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[research-log/2026-05-05-multiplayer-and-mmo-direction]]
