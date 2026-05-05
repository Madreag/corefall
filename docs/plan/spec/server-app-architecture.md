---
type: spec
status: closed-direction
created: 2026-05-05
authority: "Closed-direction architecture for the dedicated server application. Anyone can host. Same binary serves co-op, PvP, and persistent MMO shards."
ready_when: "M9 ships cf-server with a working dedicated build and core server suite, M11 proves a community member can host an internet-reachable co-op session, and M12 proves PvP + MMO shard modes with the same binary."
feeds:
  - DR-002
  - DR-005
  - DR-006
  - DR-008
  - DR-013
  - DR-022
  - DR-024
  - DR-025
  - DR-026
  - DR-029
  - DR-031
  - DR-032
  - DR-033
  - DR-034
  - DR-035
---

← [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/persistent-mmo-architecture|persistent MMO architecture]] · [[spec/backend-networking|backend networking]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-013-backend-service-scope|DR-013]] · [[decisions/dr-034-dedicated-server-application|DR-034]]

# Server App Architecture (`cf-server`)

> [!summary] Direction
> A single dedicated server binary (`cf-server`) is a full-product artifact. Anyone can host any supported game mode (co-op, PvP arena, persistent MMO shard) with no proprietary dependency. The server is a first-class native artifact alongside the client app, modding tools, and scenario editor.

> [!important] Hard rules
> Every gameplay mode is server-authoritative. The client app and `cf-server` share the **same** sim core, terrain, physics, equipment, chassis, AI, replay, mod loader, and `cf-control` schemas. There is no "server-only" branch of game logic. Public dedicated servers do not require an Anthropic/OpenAI/Steam/Epic account; cloud features are optional adapters.

## Purpose

The dedicated server app exists so that:

- The game's social layer is durable, community-runnable, and not gated by our hosting cost.
- Multiplayer in any form (LAN co-op, online co-op, public PvP arena, persistent MMO shard) uses the same authoritative sim path.
- Modding extends to **server-side gameplay**, not just client cosmetics.
- Replay/event determinism (DR-002) is reproducible from event streams the server emits.
- Persistent MMO shards (DR-035) are operationally feasible without a publisher-scale ops team (DR-026).

## Single Binary, Multiple Modes

`cf-server` runs in one of these modes selected at launch:

| Mode | Purpose | Player Count Target | Persistence |
|---|---|---|---|
| `coop_room` | Private/public co-op session with one mission/contract. | 2-4 (configurable up to 8). | Per-session save; archive run bundle on close. |
| `pvp_arena` | Server-authoritative PvP with anti-cheat foundation. | 2-8 (configurable). | Per-match save; replay archive only. |
| `lan_room` | LAN co-op (auto-discovered). | 2-4. | Same as `coop_room`. |
| `mmo_shard` | Persistent MMO shard with a long-running world (DR-035). | 50-200 concurrent (target); soft cap configurable. | Continuous; periodic snapshot to durable storage. |
| `ranked_arena` | Future: ranked PvP arena with leaderboard adapter. | 2-8. | Optional; opt-in account adapter. |
| `lobby_directory` | Public server/shard browser and presence directory. | N/A. | Required for public discovery; can be disabled in private deployments. |

Mode selection is a single `--mode` flag; per-mode config files live in `content/server/<mode>/`. Adding a new mode is a data + config addition, not a fork.

## Crate / Binary Layout

| Crate / Binary | Role |
|---|---|
| `cf-server` (binary) | Dedicated server entry point; pulls `cf-sim-core`, `cf-terrain`, `cf-physics`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-ai`, `cf-mission`, `cf-replay`, `cf-net`, `cf-control`, `cf-save`, `cf-mod`. No `cf-render-2d`, `cf-ui`, `cf-audio`. |
| `cf-server-ops` (library) | Lifecycle: config loader, mode selector, health/readiness, metrics, log shipping, shutdown drain, restart hooks. |
| `cf-server-persistence` (library) | MMO shard persistence: snapshot/restore, durable event store, cross-tick journaling, migration handlers. (M12 / DR-035) |
| `cf-server-anti-cheat` (library) | Server-side validation of client inputs, replay-driven anomaly detection, rate limits, capability gates. Extension trait + cargo features. |
| `cf-server-admin` (library) | Admin/console API: kick, ban, save, restart, mode switch, scenario load. JSON-RPC over the same `cf-control` envelope, behind capability gate `admin`. |
| `cf-headless` (existing) | Stays as the headless sim runner used by replay verification and CI; `cf-server` consumes it for the deterministic island. |

> The dedicated server keeps the **exact same** `cf-control` envelope as the client. `cfctl` can drive a `cf-server` instance for testing, automation, MMO ops, and anti-cheat audits.

## Configuration Model

Server config is RON, validated by `cf-mod validate`:

```ron
ServerConfig(
  schema_version: 1,
  mode: "coop_room",
  bind: "0.0.0.0:0",
  public_address: None,
  max_clients: 4,
  scenario: "breach_contract",
  package_set: ["base", "official_dlc_a"],
  mod_packs: [],
  capabilities: (
    admin: false,
    debug: false,
    control_api: true,
    metrics: true,
    profiling: false,
  ),
  persistence: (
    enabled: false,
    interval_ticks: 600,
    storage_dir: "./shard-state/",
    retain_snapshots: 30,
  ),
  anti_cheat: (
    enabled: true,
    profile: "competitive",
    log_dir: "./anti-cheat/",
  ),
  ai_mind: (
    enabled: false,
    config_path: "content/config/mind.ron",
  ),
  ops: (
    log_level: "info",
    log_format: "json",
    metrics_bind: "127.0.0.1:9090",
    health_path: "/health",
    ready_path: "/ready",
  ),
  rate_limits: (
    inputs_per_tick_per_client: 4,
    chat_per_minute: 30,
    join_per_minute: 6,
  ),
)
```

Adding a new field bumps `schema_version`; older configs migrate via registered handlers (matches DR-029 save model).

## Core Loop

1. Parse CLI/config; validate against schema.
2. Load `package_set` + `mod_packs` via `cf-mod`; refuse on hash mismatch unless `--allow-package-mismatch` (debug only).
3. Initialize `cf-sim-core` fixed-tick loop and the chosen mode's scenario manifest.
4. Open `cf-net` listener (transport per DR-005); admit clients per capability gates and rate limits.
5. Per tick: drain `cf-control` actions from clients, validate via anti-cheat, run sim, emit events, broadcast snapshots/event deltas, write replay events.
6. Periodically: persist (MMO mode), rotate logs, expose metrics, run health/readiness probes.
7. On shutdown: drain clients with reason, finalize replay archive, flush persistence, signal exit code.

The sim tick rate matches the client (60 Hz default; 120 Hz option). Render is absent; physics/AI/replay budgets stay deterministic.

## Authority Model

| Domain | Authority |
|---|---|
| Player input | Client sends `cf-control` actions; server validates against capability + rate limit + anti-cheat profile; only accepted actions enter the sim. |
| Sim state | 100% server-authoritative. Clients receive snapshots + event deltas; clients use prediction + reconciliation only for player-driven actor (per DR-005). |
| Terrain mutation | Server-authoritative. Clients render dirty regions delivered by snapshot/event deltas. |
| AI decisions | Server-authoritative. Clients see reason labels via event stream. |
| Mission director | Server-authoritative. Clients see commander events with reason strings. |
| Save / persistence | Server-authoritative for MMO shards; client-authoritative for solo + private LAN sessions; mixed for online co-op (host server holds the save). |
| Anti-cheat | Server-authoritative. Server-side validators are mandatory; client-side hints are not trusted. |

## Networking Transport

Transport is decided per-mode (per DR-005):

| Mode | Default Transport | Fallback / Adapter |
|---|---|---|
| `lan_room` | UDP via `lightyear` or equivalent; LAN broadcast discovery. | TCP fallback for restricted environments. |
| `coop_room` | UDP via `lightyear` / `renet` / `quinn` (decided in M9); NAT punch-through; STUN-style relay. | Steam Datagram Relay or EOS adapter optional. |
| `pvp_arena` | Same as `coop_room` with stricter authority + anti-cheat profile. | TLS over QUIC for tournament servers. |
| `mmo_shard` | QUIC-based (`quinn`); long-lived connections; per-region UDP relay if needed. | Steam Datagram Relay / EOS / PlayFab as optional adapters. |
| `lobby_directory` | HTTPS REST + WebSocket for live presence. | Steam server browser / EOS lobby adapters. |

Final transport library selection is the open follow-up tracked under [[decisions/index]] still-open topics; `cf-net` exposes adapters behind a single trait so swapping transports is local to one crate (per DR-024).

## Modding And Package Compatibility

| Aspect | Pin |
|---|---|
| Server-side mods | Yes. The same `cf-mod` package format runs on both client and server. |
| Hash sync | Mandatory. On client join, the server requires a matching package set hash (chassis, equipment, materials, scenarios, AI doctrines, mind packs). Mismatch produces a clean error with downloadable manifest of differences. |
| Auto-download | Optional per server config; off by default for production servers. Dev workflows can enable it. |
| Server-only mods | Allowed (admin tools, tournament rules). Marked with `server_only: true` in the package manifest; clients see a per-server policy summary, never raw mod code. |
| Trust tiers | Mods declare trust tier (`vanilla`, `verified`, `community`, `experimental`); servers can pin a maximum trust tier accepted from clients. |
| Sandbox | Mod scripts run in `cf-mod`'s sandboxed deterministic island per DR-006; non-deterministic ops are forbidden in sim-tick scope on both client and server. |

## Anti-Cheat Foundation

| Layer | What It Does |
|---|---|
| Input validation | Reject inputs outside declared rate limits, capability set, or per-actor authority window. |
| Replay correlation | Compare client-claimed actor state vs server-authoritative snapshots; flag drift for review. |
| Capability gates | `admin`, `debug`, `god`, `teleport`, `force_damage`, `reveal_map` all off by default; require server config opt-in. |
| Anomaly profiles | `casual`, `competitive`, `tournament_strict`. Thresholds for input rate, snapshot drift, modding, and reason-label coverage. |
| Audit log | Every rejection writes a `system.anti_cheat_*` event in the replay/run-bundle for offline review. |
| Bans | Server-side ban list (steam id / account id / IP / hash); persisted via `cf-server-persistence`. |

This is **a foundation**, not a full anti-cheat product. Tournament-grade anti-cheat is post-launch; the foundation is sufficient for community servers and the MMO mode default profile.

## Observability

Every server build ships with:

- Structured logs (JSON over stderr by default).
- Prometheus-compatible metrics endpoint (`/metrics`); per-mode default scrape interval.
- Health endpoint (`/health`) and readiness endpoint (`/ready`) per DR-013.
- Replay archive directory; per-session run bundles, retention configurable.
- `cfctl` admin commands (capability-gated) for live introspection: tick rate, client list, anti-cheat events, mod hash status, persistence status, AI mind queue depth.

## Hosting Posture

| Surface | Pin |
|---|---|
| First-party hosted servers | Optional, post-launch. Not required for v1 launch. |
| Community-hosted servers | First-class. Documented hosting guide (Linux + Windows) ships with the dedicated server build. |
| Server browser | Decoupled. Anyone can run a `lobby_directory` instance; default community list points at a self-hostable list backed by adapters. |
| Steam Game Servers / Steam Datagram Relay | Optional adapter; ship-ready config when Steam launch arrives. |
| EOS / PlayFab / Unity Multiplay | Optional adapters; same shape as Steam. |
| Cloud one-click deploy | Future. We provide Docker images and reference Terraform/systemd/launchd configs; partners or community build the rest. |
| MMO publisher hosting | Optional. We architect for community hosting; first-party MMO hosting is a business decision, not a technical lock-in. |

The default ship state is: **a player downloads `cf-server`, fills in 5 config fields, runs it, opens a port, and friends join.** No account required for private servers.

## Cross-DR Anchors

| DR | Tie |
|---|---|
| DR-005 multiplayer architecture | Defines the modes ladder; this spec implements it. |
| DR-013 backend services | Defines services architecture; this spec is the gameplay-server portion. |
| DR-024 native engine stack | `cf-server` is a Rust binary in the same workspace. |
| DR-026 team/repo model | Crates per concern; modular for AI agents. |
| DR-029 save game model | Server-side persistence reuses save schema + migration handlers. |
| DR-031 content economy | Community hosting fits the premium + free modding posture; no marketplace cut. |
| DR-032 hybrid LLM AI | Mind workers can run server-side; clients see reason labels only. |
| DR-033 full collision physics | Server-authoritative collision; clients consume `collision.*` events. |
| DR-034 dedicated server application | The closed-direction commitment captured by this spec. |
| DR-035 persistent MMO architecture | The MMO shard mode; this spec hosts it. |

## Acceptance Suite

`SERVER-001..SERVER-016` are split across milestones. M9 requires the core server lifecycle subset only; PvP/MMO scale tests belong to M12.

| ID | Milestone Gate | What It Proves |
|---|---|---|
| SERVER-001 | M9 core | `cf-server --mode coop_room --scenario breach_contract` boots, accepts 2 clients, runs the mission to completion, archives a run bundle. |
| SERVER-002 | M12 PvP | `cf-server --mode pvp_arena` runs a 4-player match server-authoritatively; client-side prediction + reconciliation visible in events. |
| SERVER-003 | M10 LAN | `cf-server --mode lan_room` is auto-discovered by client on the same LAN; ready-up + start works. |
| SERVER-004 | M12 MMO | `cf-server --mode mmo_shard` runs for 1 hour with simulated 50 clients (headless); persistence snapshot every 10 minutes; restart resumes from snapshot. |
| SERVER-005 | M10/M11 | Mod hash mismatch on join produces actionable error: client sees package diff and a download/repair route. |
| SERVER-006 | M9 core | Server replay verifies headlessly with `cf-headless replay --verify-checksums`. |
| SERVER-007 | M10/M11 | Per-client run bundles align tick-for-tick (`cf-headless replay-compare`). |
| SERVER-008 | M11/M12 | Anti-cheat profile `competitive` rejects an input-rate-spike client and logs `system.anti_cheat_kicked` event with reason; `tournament_strict` is opt-in for ranked/tournament later. |
| SERVER-009 | M9 core | Admin command via `cfctl --capability admin` kicks a client, saves the shard/session, hot-loads a scenario. |
| SERVER-010 | M9 core | Health + readiness endpoints work in container mode; metrics endpoint exposes per-tick budget, client count, replay queue depth. |
| SERVER-011 | M9 core | `cf-server` binary launches and exits cleanly on Linux + Windows. macOS support is nice-to-have. |
| SERVER-012 | M12 integration | LLM mind layer runs server-side per DR-032; clients never see prompts; replay records hashed prompts only. |
| SERVER-013 | M9/M11 | Server-side mod (admin tool / tournament ruleset) loads with `server_only: true` and is invisible to clients. |
| SERVER-014 | M9 core | Capability `god`/`debug` is off by default; requires explicit config opt-in; opt-in is recorded in run-bundle manifest. |
| SERVER-015 | M9 core | Drain shutdown: SIGTERM produces graceful client disconnect with reason, replay flush, persistence save, exit code 0 within 10 seconds. |
| SERVER-016 | M9 core | Reference Docker image runs the dedicated server unmodified; documented in `docs/server-hosting.md`. |

## Anti-Goals

- A Black Mesa "open the firewall and trust" model. Server admins must declare anti-cheat profile, capability set, and persistence policy.
- A proprietary launch-only multiplayer that locks out community hosting.
- Forced account systems for private servers.
- Different sim logic for server vs client.
- A "lite" headless server stripped of mod support.
- A naive "trust the client" model in any mode beyond debug.
- An MMO mode that requires a publisher-scale ops team to operate; the architecture must be community-hostable in degraded mode.

## Source Trail

- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-024-native-engine-stack]]
- [[decisions/dr-026-team-and-repo-model]]
- [[decisions/dr-029-save-game-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-032-hybrid-llm-ai-direction]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[spec/backend-networking]]
- [[spec/backend-service-hub-slice-a]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[systems/networking-backend-frontend]]
- [[engine/network-terrain-replication-lifecycle]]
- [[references/prototype-run-bundle-schema]]
- [[research-log/2026-05-05-multiplayer-and-mmo-direction]]
