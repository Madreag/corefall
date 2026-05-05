---
type: spec
status: closed-direction
created: 2026-05-05
authority: "Closed-direction architecture for persistent MMO shard mode. The MMO shard is one of the supported modes of cx-server; community-hostable; not subscription-funded."
ready_when: "M12 ships an MMO shard mode capable of 50-200 concurrent players for 1+ hour with persistence snapshots and clean restart."
feeds:
  - DR-002
  - DR-005
  - DR-008
  - DR-013
  - DR-022
  - DR-024
  - DR-026
  - DR-029
  - DR-031
  - DR-032
  - DR-034
  - DR-035
---

← [[spec/index|spec section]] · [[spec/server-app-architecture|server app architecture]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-013-backend-service-scope|DR-013]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]]

# Persistent MMO Architecture

> [!summary] Direction
> A "persistent MMO shard" is one mode of `cx-server` (DR-034). A shard is a long-running world hosting 50-200 concurrent players in a contract-based frontier sandbox. Multiple shards can run independently; cross-shard travel is via lobby/portal at v1, not seamless world. Anyone can host a shard; first-party hosting is optional, not required for launch.

> [!important] Bounds
> This is **not** EVE Online, **not** WoW, **not** Star Citizen. It is **Cortex Command's contract-and-base sandbox at MMO scale** with persistent factions, named actors, salvage economies, and player-built bases. The MMO mode borrows from Project Zomboid dedicated servers, Space Station 13/14 round-based persistence, and EVE-class server architecture, **without** committing to seamless single-shard world or full simulation of every shard tick.

## What Persistent Means Here

| Surface | Persistence Granularity |
|---|---|
| World terrain | Per-region chunk; carved/repaired terrain persists between session/restart. Material state matches the deterministic-island contract (DR-002). |
| Bases | Full base layouts + module HP/ammo/power state per faction (DR-027). Player-built bases survive reboot. |
| Player veterans | Per-account roster: names, traits, injuries, equipment, AI doctrines, kill/save histories. Cross-mission per DR-018 + DR-029. |
| Faction state | Reputation, contract pool, enemy commander memory, doctrine evolution (DR-022). |
| Mission director memory | Cross-mission adaptation to player tactics; LLM memory writes from DR-032. |
| Mech/chassis state | Damage history, salvageable modules, paint/identity, crew slots (DR-021). |
| Salvage / inventory | Per-account materials, parts, recovered modules. |
| Replay archives | Per-mission run bundles archived; queryable by player/faction/timeframe. |
| Audit log | Anti-cheat events, admin actions, config changes — append-only with retention policy. |

## Shard Topology

Each shard is one process running `cx-server --mode mmo_shard`. A shard owns:

- One persistent world manifest (region map, materials, hazards, faction territories).
- One persistent state store (snapshot files + durable event journal).
- A configurable concurrent-player target (default 50, soft cap 200).
- A configurable contract director (mission generator pulling from per-faction pools).
- A configurable persistence cadence (default snapshot every 10 minutes; journal on every objective change).

Shards do not share world state. Players have one account per `lobby_directory` (or per community list); accounts can travel between shards via lobby/portal. Cross-shard live trade or live combat is **not** v1.

## Player Count Targets

| Tier | Concurrent | Hardware Floor | Notes |
|---|---|---|---|
| `intimate` | 4-16 | 4-core / 8 GB / 100 Mbps | LAN/online co-op-style; simulated as a bigger `coop_room` with persistence on. |
| `community` | 16-50 | 8-core / 16 GB / 1 Gbps | Default community shard. |
| `regional` | 50-100 | 16-core / 32 GB / 1 Gbps + relay | Mid-size public shard with anti-cheat on. |
| `flagship` | 100-200 | 32-core / 64 GB / 10 Gbps + relay tier | Dense world; first-party or partner-operated. |
| `experimental` | 200+ | TBD | R&D only; targets that require sharding the sim itself. |

The **launch target** is `community` + `regional`. `flagship` is a stretch target proven post-launch. `experimental` is moonshot.

## Persistence Model

Persistence uses two stores:

| Store | Format | Cadence | Purpose |
|---|---|---|---|
| Snapshot store | Compressed binary (bincode + zstd) per region | Configurable: default every 10 min | Fast restore on shard restart. |
| Event journal | Append-only `events.jsonl` per shard tick | Continuous (per tick, batched) | Tick-level audit + replay reconstruction; supports point-in-time recovery. |

Recovery: on restart, load most recent snapshot, replay journal forward, reach live state. Schema-version-aware migration handlers run during recovery (matches DR-029 save model).

Storage: defaults to local filesystem (`./shard-state/<shard-id>/`). Operators can mount network storage, S3-compatible object stores, or remote durable journals via adapters. **No** hard dependency on a proprietary cloud database.

## Account Model

| Surface | Pin |
|---|---|
| Accounts required for shards | **Yes** for public shards; **no** for private LAN/co-op rooms. |
| Account provider | Plug-in. Defaults: local account file (private shards), `lobby_directory` adapter (community), Steam/EOS/PlayFab adapter (post-launch). |
| Authentication | Token-based; tokens are bearer credentials with expiry; never plaintext passwords. Replay/run-bundles redact tokens by default per DR-013. |
| Cross-shard identity | Same account id resolves across shards a player has been admitted to. |
| Privacy | Account id is opaque to other players unless they share. Display names are per-shard, not global. |
| Free vs paid | The base game is premium one-time purchase per DR-031. Public shard hosting is community-runnable (free) or operator-funded. **No subscription** for the base game; an MMO shard *operator* may charge for their hosted shard, but that's an operator decision, not the base SKU. |

## Server-Authoritative MMO Loop

Per fixed tick (60 Hz default; 30 Hz acceptable for MMO mode if perf demands):

1. Drain client `cx-control` actions; rate-limit per anti-cheat profile.
2. Tick sim systems for all actors in active region(s).
3. Run mission director and faction commander logic.
4. Process LLM mind proposals (background; never blocking).
5. Emit events to clients in interest range; persist to journal; snapshot if cadence due.
6. Update visibility/interest sets; cull events to clients out of range.

Performance: MMO shard at 50-100 concurrent players + 200 AI actors targets 30 Hz sim, 60 Hz client interpolation. T-PERF tracks this.

## Interest Management

Clients only receive events/snapshots for entities in their interest set:

| Entity | Default Interest Range |
|---|---|
| Actors | Visual range + audible range; mission allies always; faction commander always. |
| Terrain | Chunk + 1-chunk halo; dirty regions broadcast in this halo. |
| Base modules | Visible range; owner's faction always; allied factions per relationship. |
| Audio captions | Caption events delivered for player-audible sources only. |
| AI reason labels | Allied AI always; visible enemy AI when they take an action; persisted offline cognition only at debrief time. |

Interest is computed server-side; clients do not request entities they shouldn't see (anti-information-leak).

## Mission / Contract Loop In MMO Mode

| Aspect | Pin |
|---|---|
| Contract source | Per-faction contract pool generated by mission director per DR-017. |
| Contract triggers | Player accepts via base/HUB UI; contract director spawns enemy/objective/event sequence. |
| Persistence | Contract state persists across sessions; players can log out mid-contract and resume (within a configurable timeout). |
| Failure | Per DR-018 consequence ladder; named actor injuries persist; salvage/loss applies. |
| Sharing | Players can party-up; contracts can be shared/co-op'd; rewards split per scenario rules. |
| Discovery | Lobby/portal lists active shards with their contract pool summary. |

## Anti-Cheat And Operator Posture

MMO shards default to anti-cheat profile `competitive` (stricter than `casual`, less strict than `tournament_strict`). Operators can tune:

- Input rate caps per actor.
- Capability set (e.g. disallow modding higher-trust packages on competitive shards).
- Replay drift thresholds.
- Auto-kick / auto-ban thresholds.

Audit logs are append-only and operator-readable. Player appeals are out-of-game per operator policy.

## Modding In MMO Mode

| Aspect | Pin |
|---|---|
| Server-required mods | Operator declares; clients see manifest before join; mismatch produces clean error. |
| Server-only mods | Allowed for admin tools, tournament rules, shard-specific quests, special events. Clients never see source. |
| Persistence migration | Mod schema bumps require server-side migration handlers; shards refuse to load incompatible mod versions without registered migrations. |
| Trust | Operators pin a max trust tier accepted from clients (`vanilla`, `verified`, `community`, `experimental`). |
| Sandbox | Mod scripts run in `cx-mod` deterministic island per DR-006; non-deterministic ops forbidden. |

## Cross-Shard, Lobby, And Discovery

| Surface | Pin |
|---|---|
| Lobby/portal | Players discover shards via a `lobby_directory` instance (also a `cx-server` mode). Multiple lobby instances can exist. |
| Shard browse | Filter by mode, region, ping, player count, package set, ruleset, trust tier. |
| Cross-shard travel | Player logs out on Shard A, logs in on Shard B; no live cross-shard combat or trade in v1. |
| Identity persistence | Account id + per-shard veteran roster persist; cross-shard veteran transfer is **not** v1. |
| Federation | Operator-of-operators model: a `lobby_directory` aggregates shards but doesn't move state between them. Future federation is open research. |

## What MMO Mode Is NOT

| Anti-Goal | Why |
|---|---|
| Seamless single-shard world | Sim cost; out of scope for v1; multi-shard is enough. |
| Cross-shard live combat | Latency / authority complexity; v1 keeps shards independent. |
| Subscription-funded MMO | Conflicts with DR-031 content economy (premium + free modding). |
| Live cash shop / pay-to-win | Conflicts with DR-031. |
| Mandatory account at all multiplayer modes | Private LAN/co-op rooms must work without accounts. |
| Operator-imposed publisher hosting | Architecture must support community-hosted shards by default. |
| Real-money trading of in-game items | Out of scope for v1; future business decision, not technical. |
| Auto-population (server bots dressed as players) | Forbidden. NPCs and AI actors are visibly AI; player count metric is humans only. |

## Acceptance Suite

`MMO-001..MMO-012` (M12 task cards):

| ID | What It Proves |
|---|---|
| MMO-001 | `cx-server --mode mmo_shard` boots with default config; 1 player connects and roams the world. |
| MMO-002 | Snapshot persists every 10 minutes; shard restart resumes from snapshot in <30 s with no state loss. |
| MMO-003 | Event journal supports point-in-time recovery; crash/kill -9 + restart resumes within 1 minute with at most 1 minute of journal replay. |
| MMO-004 | 50 simulated clients (headless `cxctl` puppets) connect and play for 1 hour without desync; sim stays at 30 Hz target. |
| MMO-005 | 100 simulated clients sustained for 30 minutes; perf report records frame budgets and degraded modes. |
| MMO-006 | Two shards run concurrently on different ports; lobby/portal lists both; player log-out from Shard A and log-in on Shard B works. |
| MMO-007 | Mod hash mismatch on join produces actionable diff; auto-download disabled by default. |
| MMO-008 | Anti-cheat profile `competitive` rejects input-rate-spike client; logs `system.anti_cheat_kicked`; ban list persists. |
| MMO-009 | Interest management: client receives only events/snapshots for entities in interest set; verified by event-volume audit. |
| MMO-010 | LLM mind workers run server-side; clients never see prompts; mind events redacted in client-visible event stream per DR-032. |
| MMO-011 | Schema migration: a v0.1 shard state loads on v0.2 with declared migration handlers. |
| MMO-012 | Operator can run a shard with no proprietary cloud dependency (filesystem snapshot store, local lobby_directory, no Steam/EOS adapters). |

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Persistence corruption | Atomic snapshot writes (temp + rename); journal replay validates on restore; rolling backups. |
| Sim cost at 100+ concurrent players | Interest management; sub-region tick budgets; offload AI/LLM mind to background; degrade to 30 Hz. |
| Mod mismatch hell | Pinned package set per shard; clear diff UI; trust tiers; version migrations. |
| Bot/cheater wave | Anti-cheat profiles; input-rate limits; replay drift detection; ban list with appeal-out-of-game. |
| Operator burnout | Reference Docker images; minimal config requirements; community templates. |
| Account compromise | Token-based auth; rotation; rate limits on join; never log raw tokens. |
| MMO scope creep | Anti-goals enforced via this spec; cross-shard combat / seamless world require explicit DR amendment. |
| Cost spirals for big shards | First-party hosting optional; operator-funded model preserved; we provide reference deployments. |

## Cross-DR Anchors

| DR | Tie |
|---|---|
| DR-005 multiplayer architecture | MMO is one of the modes; ladder culminates here. |
| DR-013 backend services | Lobby/portal, account adapter, anti-cheat foundation all live in the backend service spine. |
| DR-018 death meaning | MMO mode persists named actor injuries/death per scenario policy; cross-mission consequence ladder applies. |
| DR-022 humanlike AI | Faction commander cross-mission memory + adaptation persists in shard state. |
| DR-027 combat-base scope | Bases are persistent in MMO mode; deep combat-base mechanics scale to shard. |
| DR-029 save game model | Shard state uses the same versioned format + migration handlers as solo saves. |
| DR-031 content economy | MMO is community-runnable; no subscription; operators may monetize hosting independently. |
| DR-032 hybrid LLM AI | Mind workers run server-side; per-shard cost cap. |
| DR-033 full collision physics | Server-authoritative collision; clients consume `collision.*` events. |
| DR-034 dedicated server app | The `cx-server` binary's `mmo_shard` mode is the implementation surface for this spec. |
| DR-035 persistent MMO architecture | The closed-direction commitment captured by this spec. |

## Source Trail

- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-018-death-meaning-and-consequence-ladder]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-029-save-game-model]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-032-hybrid-llm-ai-direction]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[spec/server-app-architecture]]
- [[spec/backend-networking]]
- [[spec/backend-service-hub-slice-a]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[research-log/2026-05-05-multiplayer-and-mmo-direction]]
