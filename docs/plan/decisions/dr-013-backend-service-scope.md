---
type: decision
id: DR-013
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "Backend cost or operator complexity exceeds the AI-augmented-solo team model (DR-026); platform certification forces a backend redesign; account/identity provider becomes contractually mandatory; or community-hosting posture proves incompatible with multiplayer scale."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/server-app-architecture|server app architecture]] · [[spec/persistent-mmo-architecture|persistent MMO architecture]] · [[spec/backend-networking|backend networking]] · [[spec/backend-service-hub-slice-a|backend service/hub Slice A]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-034-dedicated-server-application|DR-034]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]]

# DR-013: Backend Services Architecture

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-05)
> Backend services are first-class. They support a self-hostable dedicated server app (DR-034), persistent MMO shards (DR-035), package/replay sharing, server discovery, and the optional account/identity surface for public modes. **Local-first remains the default for solo/private LAN play; public-server services unlock as online modes are used and are proven by M9-M12 gates.** Any operator can run any service tier with no proprietary cloud dependency.

## Decision

**Full backend service spine + dedicated server app + community hosting + optional first-party services.**

This DR replaces the prior "local-first service spine + optional adapters" lean. The user's commitment on 2026-05-05 promotes backend services from optional-research-only to a full-product architecture surface, while preserving the local-first default for solo and private play.

## What This Locks In

| Service Tier | Status | Notes |
|---|---|---|
| Local game services (health, schema, package registry, join eligibility, deep-link parser, local replay/report index, diagnostics export, redaction) | **Required at launch.** Same shape as the prior Slice A. | Files-on-disk + in-process services for solo/private play. |
| Local server supervisor | **Required at launch.** | Drives `cx-server` lifecycle from the client when the player hosts. |
| `lobby_directory` service | Required for public server/shard discovery; optional for private deployments. | Aggregates community-hosted shards; multiple instances can exist. Also one of the `cx-server --mode` options (DR-034). |
| Account/identity adapter | Required for public shards. Optional for private play. | Plug-in: local account file (private), `lobby_directory` token (community), Steam/EOS/PlayFab adapters (post-launch ready). |
| Server discovery / browser | Required for public online modes. | Filter by mode, region, ping, player count, package set, ruleset, trust tier. |
| Package / mod registry | **Required at launch.** | Package hashes + manifest summaries + dependency graph; reused by client, server, and editor. |
| Replay / report index | **Required at launch.** | Per-run bundle metadata; queryable; redacted by default. |
| Diagnostics + telemetry | **Required at launch.** | Local diagnostics with consented exports; redaction tests required. |
| Persistent MMO shard services (snapshot store, event journal, durable storage adapter) | Required for M12 MMO shard evidence. | DR-035; community-hostable; no proprietary cloud lock-in. |
| Anti-cheat foundation (server-side validation hooks, ban list, audit log) | **Required at launch.** | Foundation only; tournament-grade is post-launch. |
| Optional first-party hosted services (cloud save sync, server browser cluster, ranked leaderboards, event aggregation) | **Optional, post-launch.** | Adapters; no v1 commitment to operate them. |
| Live-service economy / cosmetic shop / marketplace | **NOT a launch commitment.** | DR-031 anti-goal; remains rejected. |

| Architectural Pin | Commitment |
|---|---|
| One service contract | All gameplay/server/client/replay consumers share `ServerSummary`, `PackageManifestSummary`, `JoinEligibilityResult`, `ReplaySummary`, `DiagnosticsReport`, `ShardSnapshotSummary`, `LobbyEntry` shapes. |
| Schema versioning | Every service object carries `schema_version`; migrations are mandatory before bumping. |
| Transport | HTTPS REST + WebSocket for `lobby_directory`; QUIC/UDP for sim transport (per DR-005). |
| Persistence | Local filesystem default; S3-compatible / network filesystem / durable journal adapters available; no hard cloud dependency. |
| Account credentials | Token-based bearer; tokens have expiry; tokens are NEVER written to run bundles or replay events. |
| Privacy / redaction | Default-on for all run-bundle exports; tested per BACK-SCOPE-07; opt-in for diagnostics sharing. |
| Adapter posture | Steam, EOS, PlayFab, Unity Multiplay, Sony/MS/Nintendo are **adapter** layers behind shared contracts; the core never depends on any one platform. |
| Observability | Prometheus-compatible metrics endpoint per server; structured JSON logs; health/readiness endpoints. |
| Open-source posture for backend code | Friendly to community-hosted operators; documentation and reference deployments ship with the server artifact. |

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| "Local-first only" / "no backend until online is committed" | The user's 2026-05-05 commitment requires backend at launch for community hosting + MMO. |
| "Public community backend from prototype start" | Still rejected as a Slice A obligation; the local-first default + adapter pattern protects sim/AI quality during M0..M7. Public services arrive in M9..M12. |
| Platform-first backend (Steam/EOS-only) | Adapters yes, lock-in no. The core service contract is platform-neutral. |
| Live-service economy spine | DR-031 forbids. Backend is for gameplay/community surfaces, not monetization. |
| Forced account systems | Solo and private play work without accounts. |
| Forced first-party hosted MMO | Architecture supports community-hosted shards by default. |
| Hidden / non-documented hosting | Reference deployments + Docker images + hosting guide ship at launch. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Backend stays "OPEN, local-first lean" forever | Conflicts with DR-005 multiplayer launch ladder + DR-035 MMO commitment. |
| Steam-only backend | Lock-in. Linux + Windows community hosting must work without a Steam account. |
| Cloud-database-first MMO persistence | Operator cost; conflicts with community-hosted MMO. Local FS + journal is enough; cloud is an adapter. |
| Backend implies live-service / monetization | Contradicts DR-031; we explicitly separate gameplay services from economy. |
| Single first-party `lobby_directory` only | Operator monoculture risk; we ship as a multi-instance protocol. |

## Evidence Trail

- Project owner verbatim (2026-05-05): "I want to have an entire server app designed to run our app. anyone can host multiplayer games, as well as the persistent MMO mode."
- DR-005 multiplayer architecture closes with full ladder including community-hosted PvP and MMO shards.
- DR-034 dedicated server app commits to `cx-server` as a launch artifact.
- DR-035 persistent MMO architecture commits to community-hostable shards.
- Source patterns: Steamworks Game Servers + Steam Datagram Relay (community-hostable + relay-optional), EOS sessions/lobbies/relay (modular), PlayFab multiplayer (modular), Unity Multiplay readiness (process+ready separation), OpenSoldat satellites (launcher/lobby/base content separation), Project Zomboid dedicated server (community ops), Space Station 14 (round-based persistence + community).
- Cross-DR coherence: DR-013's prior "local-first + optional adapters" lean is preserved as the default solo/private posture; the closed-direction now explicitly extends scope to public services as full-product architecture proven in M9-M12.

## Service Tier Matrix (Updated)

| Tier | Services | Status | Why |
|---|---|---|---|
| Local game core (Slice A) | health, schema/version report, local package registry, join eligibility, deep-link parser, local server supervisor, local replay/report index, diagnostics export, privacy redaction. | Required at launch. | Solo/local play, package compatibility, workbench UX, recorder evidence; same as prior Slice A. |
| Server lifecycle (Slice B) | `cx-server-ops` health/readiness/metrics, log shipping, drain shutdown, restart hooks. | **Required at launch.** | DR-034 dedicated server app. |
| Server discovery (Slice B) | `lobby_directory` service: shard list, presence, package set summary, trust tier, ping. | **Required at launch.** | DR-005 community hosting + DR-035 MMO discovery. |
| Account/identity adapter (Slice B) | Local account, lobby token, Steam/EOS/PlayFab adapter shapes. | **Required at launch** for public shards. | DR-035 account model. |
| Persistence (Slice B) | `cx-server-persistence` snapshot store + event journal + durable storage adapter. | **Required at launch.** | DR-035 MMO mode. |
| Anti-cheat foundation (Slice B) | Server-side validation hooks, profiles, ban list, audit log. | **Required at launch.** | DR-005 anti-cheat foundation. |
| Replay / report sharing (Slice C) | Optional opt-in upload of run-bundle metadata; replay browser. | Optional, post-launch. | Retention/community surface. |
| First-party server browser cluster | Cloud-hosted aggregation across community lobby_directory instances. | Optional, post-launch. | Convenience; community can run without it. |
| Cloud save sync | Cross-device save sync. | Optional, post-launch. | DR-029. |
| Ranked leaderboards | PvP / contract leaderboard service. | Optional, post-launch. | Competitive; not v1. |
| Live-service economy / shop / marketplace | Cosmetic / DLC store. | **NOT a launch commitment.** | DR-031 forbids predatory; expansion DLC is a separate distribution channel. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| BACK-SCOPE-01..10 | Existing tests still apply (per [[spec/backend-service-hub-slice-a]]). |
| SERVER-001..SERVER-016 | Dedicated server lifecycle works for all modes (DR-034). |
| MMO-001..MMO-012 | MMO shard services + persistence + interest mgmt + community hosting (DR-035). |
| BACK-LOBBY-01 | `lobby_directory` lists 3 community shards across 2 operators; client browses + joins. |
| BACK-ACCOUNT-01 | Local account file works for private LAN; Steam adapter resolves identity for public shard; tokens never appear in run bundles. |
| BACK-PERSIST-01 | Snapshot store + journal restore reproduces shard state within 1 minute of crash. |
| BACK-ANTI-CHEAT-01 | Server-side rate limit + replay drift detection + ban list persists. |
| BACK-OPS-01 | Reference Docker image runs `cx-server` unchanged; `/health` + `/ready` + `/metrics` work. |

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Backend complexity overruns DR-026 team capacity | Modular crate boundaries; community-runnable defaults; first-party services are post-launch only. |
| Cloud cost spirals | Local FS + journal default; cloud adapters are operator-funded. |
| Account/identity provider lock-in | Plug-in adapter; local account works for private; `lobby_directory` token is operator-friendly. |
| Service contract drift | Schema versioning + migration handlers + cross-consumer tests at every milestone. |
| Mod compatibility chaos | Package hashes + trust tiers + clear mismatch UI; per-server admission policy. |
| Replay/report privacy leaks | Default-on redaction; BACK-SCOPE-07 tests; never log tokens or absolute local paths. |
| Anti-cheat false positives | Tiered profiles; tournament-grade is post-launch; appeal-out-of-game per operator policy. |

## Revisit Trigger

- Backend cost or operator complexity exceeds the DR-026 AI-augmented-solo team model.
- Platform certification (Steam/Sony/MS/Nintendo) forces a backend redesign.
- Account/identity provider becomes contractually mandatory in a way that conflicts with community hosting.
- Community-hosting posture proves incompatible with multiplayer scale.
- A future business decision elevates first-party hosted services to launch commitments.

## Source Trail

- Project owner direction (2026-05-05).
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[spec/backend-networking]]
- [[spec/backend-service-hub-slice-a]]
- [[spec/package-builder-workbench-slice-a]]
- [[systems/networking-backend-frontend]]
- [[systems/replay-determinism-and-run-evidence]]
- [[comparables/opensoldat-satellites-local-audit]]
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-010-license-reuse-matrix]]
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-026-team-and-repo-model]]
- [[decisions/dr-029-save-game-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- Steamworks Game Servers, Steam Datagram Relay, EOS, PlayFab, Unity Lobby + Multiplay docs.
- [[research-log/2026-05-05-multiplayer-and-mmo-direction]]
