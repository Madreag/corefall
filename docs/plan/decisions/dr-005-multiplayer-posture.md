---
type: decision
id: DR-005
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "Networking transport library proves infeasible after M9/M10 prototyping; bandwidth budgets cannot be hit at MMO scale; community-hosting posture conflicts with platform certification; or post-launch evidence shows competitive PvP is incompatible with the chassis/destruction sim."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/server-app-architecture|server app architecture]] · [[spec/persistent-mmo-architecture|persistent MMO architecture]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-013-backend-service-scope|DR-013]] · [[decisions/dr-034-dedicated-server-application|DR-034]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]]

# DR-005: Multiplayer Architecture And Launch Scope

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-05)
> Multiplayer/PvP/MMO capability is an architecture commitment from day one, not an afterthought: **solo + split-screen-ready local play + LAN co-op + online co-op + community-hostable PvP arenas + persistent MMO shards** all route through one server-authoritative `cf-server` design (DR-034). PvP arenas and MMO shards are full-product targets proven at M12; if their evidence gates fail, DR-005/DR-035 reopen instead of silently blocking earlier first-playable milestones.

## Decision

**Server-authoritative simulation; one dedicated server binary; full multiplayer ladder architected from day one and evidence-gated through M12.**

This DR replaces the prior "solo-first + co-op-ready architecture; no launch PvP promise yet" posture. The user's commitment on 2026-05-05 is to plan multiplayer, PvP, MMO capability, and a hostable server app as first-class product architecture. It does **not** mean M9 must pass M12-scale MMO/PvP soak tests; those remain M12 evidence gates.

## What This Locks In

| Mode | Status | First Proof |
|---|---|---|
| Solo (offline) | Required at launch. No account needed. | M0..M7 progression. |
| Split-screen local | Optional at launch (input remap UX cost). | Post-M7 evaluation. |
| LAN co-op | Full-product target; evidence-gated. | M10 LAN Co-op milestone. |
| Online co-op (private) | Full-product target; evidence-gated. | M11 Online Co-op milestone. |
| Public online co-op (community-hosted) | Full-product target; evidence-gated. | M11 + M12. |
| PvP arena (community-hosted) | Full-product target; not a M9 gate. | M12 PvP milestone. |
| Persistent MMO shards (community-hostable) | Full-product target; not a M9 gate. | M12 MMO milestone. |
| Ranked PvP / first-party MMO hosting | Optional, post-launch. | Post-launch evaluation. |

| Architectural Pin | Commitment |
|---|---|
| Authority | 100% server-authoritative for sim state, terrain mutation, AI decisions, mission director, save persistence. Clients use prediction + reconciliation only for player-driven actor. |
| Server binary | One `cf-server` binary with `--mode <coop_room\|pvp_arena\|lan_room\|mmo_shard\|lobby_directory\|ranked_arena>`. Same sim, terrain, physics, equipment, chassis, AI, replay, mod, control as the client. See [[spec/server-app-architecture]]. |
| Hosting | Community-hostable by default. Steam Datagram Relay / EOS / PlayFab / Unity Multiplay are **adapters**, not requirements. |
| Account | **Not required** for solo or private LAN/co-op rooms. Required for public shards (per DR-035). |
| Anti-cheat | Server-authoritative validation as a foundation; tournament-grade is post-launch. Anti-cheat profiles: `casual`, `competitive`, `tournament_strict`. |
| Modding | Server runs the same `cf-mod` package format as the client; mod hash sync is mandatory; trust tiers gate per-server admission (DR-006). |
| Replay determinism | Server-authoritative replay; per-client run bundles align tick-for-tick (DR-002). |
| LLM mind | Mind workers may run server-side (DR-032); clients see reason labels only, never prompts. |
| Networking transport | Decided in M9/M10: candidate trait-bound implementations of `lightyear`, `renet`, `quinn`. Selection committed before M11 begins. |
| Bandwidth target | TBD per T-PERF; per-client floor 64 KB/s in dense combat; MMO mode 30 Hz acceptable to fit budgets. |

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| "No PvP/MMO architecture until later" | The user's 2026-05-05 direction requires multiplayer, PvP, MMO capability, and hostable server architecture to be designed now. |
| "MMO mode is post-launch-only research" | DR-035 promotes MMO shard mode to a full-product target with M12 acceptance gates. Community-hosted shards remove the operator-cost objection, but M12 evidence still decides readiness. |
| Client-authoritative sim state | Cheating risk; replay/determinism risk; networking instability. Server is authoritative for everything that matters. |
| Forced first-party hosting | Architecture supports community hosting first; first-party is optional adapter. |
| Forced account systems for private play | Solo and LAN/private co-op work without any account. |
| Different sim logic for server vs client | One `cf-sim-core`; server omits render/audio crates only. |
| Brute-force replication | Use snapshot/event hybrid + interest management per DR-002 + persistent MMO architecture. |
| Subscription-funded MMO | Conflicts with DR-031 content economy. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Solo + LAN only at launch | Underdelivers on the contract-and-base sandbox social promise; the user wants community to be first-class. |
| Solo + LAN + online co-op (no PvP, no MMO) | Same as above; misses the unique community hosting opportunity. |
| Single-shard seamless world MMO | Sim cost; out of scope. Shard-with-portal is enough. |
| Publisher-only hosted MMO | Operator cost; conflicts with DR-026 solo-with-AI team model. |
| Fully decentralized P2P | Authority/anti-cheat too weak; rejected. Server-authoritative is non-negotiable. |
| Different server binaries per mode | Operational complexity; mod compatibility hell. One binary, multiple modes. |

## Evidence Trail

- Project owner verbatim (2026-05-05): "I think it makes sence to plan out Multiplayer and PVP and MMO capability. I want to have an entire server app designed to run our app. anyone can host multiplayer games, as well as the persistent MMO mode."
- Interpretation: this closes the architecture direction and roadmap capability target. It does not collapse M9/M10/M11/M12 into one release gate; PvP/MMO readiness is proven by the M12 suites.
- Source patterns: Source dedicated server, Quake/Quake-Live community model, Project Zomboid dedicated server, Space Station 13/14 round-based persistence, Minecraft Realms-vs-Bukkit split, EVE Online single-shard architecture (anti-pattern; we're not doing that), Steam Game Servers + Steam Datagram Relay, EOS sessions, Unity Lobby + Multiplay readiness, PlayFab modular multiplayer.
- Cross-DR coherence: DR-024 already commits to "MMO-ready architecture from day one"; this DR closes the open posture so DR-024 isn't aspirational.
- DR-026 team model + DR-031 economy + DR-013 backend scope all support community-hosted servers as the default; DR-005 ratifies this.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Bandwidth at peak combat density | Snapshot/event hybrid; interest management; per-region replication; T-PERF gates at M10/M11/M12. |
| Anti-cheat is hard with destructible sim | Server-authoritative simulation; replay drift detection; tiered profiles; tournament-grade is post-launch. |
| Mod compatibility across servers | Hash sync; trust tiers; clear mismatch UI; auto-download off by default for production. |
| MMO persistence corruption | Atomic snapshot writes; journal replay validation; rolling backups (DR-035). |
| Community hosting cost / availability | First-party Docker images + reference deployments; `lobby_directory` aggregates community shards; we don't take responsibility for uptime of community shards. |
| Networking transport churn | Trait-bound `cf-net` adapter; selection committed before M11; library swap is local to one crate. |
| Platform certification (Steam/EOS) | Adapters; can launch on Steam without locking to Steam-only. |
| Replay drift across clients | Stable pair ordering, deterministic islands, per-client checksums in M10/M11 acceptance tests. |
| Operator burnout | Minimal-config dedicated server; reference Docker; community templates; T-SERVER side track tracks ops UX. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M9 server-core subset | Dedicated server binary works for core lifecycle, replay, health/readiness, admin capability gates, drain shutdown, Docker smoke, and mode boot/config paths. |
| M10 LAN co-op | Two LAN clients survive a 5-min Breach Contract; per-client bundles align. |
| M11 online co-op | Two friends in different cities co-op a Breach Contract through NAT/relay; mod hash sync works. |
| M12 PvP arena | 4-8 players in a small destructible map; server-authoritative; bandwidth + cheat models tested. |
| M12 MMO-001..MMO-012 | MMO shard mode boots, persists, restarts cleanly; 50 simulated clients for 1 hour; 100 for 30 min. |
| Headless replay verification | `cf-headless replay --verify-checksums` per-client and per-server bundles align tick-for-tick. |
| Anti-cheat profile validation | Input-rate-spike client kicked; ban list persists across restart. |

## Revisit Trigger

- Networking transport library proves infeasible after M9/M10 prototyping.
- Bandwidth budgets cannot be hit at MMO scale.
- Community-hosting posture conflicts with platform certification (Steam/Sony/MS/Nintendo).
- Post-launch evidence shows competitive PvP is incompatible with the chassis/destruction sim.
- A subscription/free-to-play business decision conflicts with DR-031.
- Cross-shard live combat or seamless world becomes a strategic priority (would require DR-035 amendment).

## Source Trail

- Project owner direction (2026-05-05).
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[spec/backend-networking]]
- [[spec/backend-service-hub-slice-a]]
- [[systems/networking-backend-frontend]]
- [[engine/network-terrain-replication-lifecycle]]
- [[comparables/soldat-and-opensoldat]]
- [[comparables/opensoldat-local-audit]]
- [[comparables/opensoldat-satellites-local-audit]]
- [[comparables/openlierox-local-audit]]
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-026-team-and-repo-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-032-hybrid-llm-ai-direction]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[research-log/2026-05-05-multiplayer-and-mmo-direction]]
