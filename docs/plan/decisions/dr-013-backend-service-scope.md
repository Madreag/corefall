---
type: decision
id: DR-013
status: open
priority: P1
revisit_trigger: "Reopen when DR-005 produces co-op/PvP bandwidth evidence, a public playtest is scheduled, package/replay sharing becomes a core loop, or the project chooses a platform backend."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[dashboards/research-readiness|readiness]] · [[spec/backend-networking|backend/networking posture]] · [[spec/backend-service-hub-slice-a|backend service/hub Slice A]]

# DR-013: Backend Service Scope

> [!info] Status: OPEN; LEAN: local-first service spine + optional adapters
> Build the backend only where it makes the game, modding, replay/debug, hub UX, diagnostics, and future co-op experiments better. Start with a local-first service spine and schema-compatible adapters. Do not couple early backend work to accounts, matchmaking, anti-cheat, monetization, or a public PvP promise.

## Context

[[spec/backend-service-hub-slice-a]] already defines a buildable first backend/hub slice. This record decides what belongs in the durable backend spine versus what should stay as optional platform/community/live-service research until prototypes prove the need.

The decision matters because a Cortex-like game has unusual backend pressure: mod/package compatibility, replay evidence, AI failure reports, server discovery, terrain-sync metadata, and deep links all need shared schemas even if the game launches solo-first.

## Options

| Option | Summary | Best Case | Worst Case |
|---|---|---|---|
| A | No backend until online is committed. | Fastest short-term path; no ops burden. | Hub, package compatibility, replay sharing, diagnostics, and co-op experiments all reinvent data shapes later. |
| B | Local-first service spine + static/heartbeat adapters. | Gives the game a durable service contract without live-service lock-in. | Some backend work ships before online fun is proven. |
| C | Public community backend from prototype start. | Early server list, replay sharing, and mod registry create community energy. | Moderation, abuse, privacy, uptime, and compatibility costs distract from actor feel and AI. |
| D | Platform-first backend (Steam/EOS/PlayFab/Unity/etc.). | Faster public multiplayer/lobby integration if a platform is chosen. | Platform lock-in and account assumptions leak into core schema too early. |
| E | Live-service/account-economy-first backend. | Monetization, inventory, and events can be researched quickly. | The game becomes service-led before the core physics/AI loop is excellent. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Maximum focus on simulation and AI; no server maintenance. | Weakens package, replay, diagnostics, hub, and online prototype readiness. | Whether local-only tooling can remain coherent without shared schemas. |
| B | Supports solo/local/offline flows; keeps platform choice open; feeds package builder, replay recorder, hub, and diagnostics. | Requires disciplined scope control and schema maintenance. | Exact API/process boundary after DR-001 engine choice. |
| C | Tests real community behavior early; makes server discovery tangible. | Requires auth, moderation, rate limits, privacy, logs, abuse handling, deployment, and support. | Whether public multiplayer is fun or viable yet. |
| D | Reuses proven lobby, relay, matchmaking, server query, and identity systems. | Can force Steam/EOS/PlayFab/Unity concepts into a game still proving its local loop. | Target stores, platform terms, SDK fit, and transport model. |
| E | Useful later for cosmetics, retention, events, inventory, and experiments. | High risk to fairness, modding trust, and design focus. | Whether monetization is wanted at all. |

## Evaluation

| Lens | A: no backend | B: local-first spine | C: public backend early | D: platform-first | E: live-service first |
|---|---|---|---|---|---|
| Player value | Low until online exists | High: better hub, replays, packages, diagnostics | Medium/high if community online works | Medium if platform flow is smooth | Low before core loop is proven |
| Readability | Weak join/error explanations | Strong join blockers and diagnostics | Strong if maintained | Strong but platform-shaped | Risky: economy UI noise |
| AI burden | No direct help | Strong AI failure reports and replay indexes | Strong, but noisy public reports | Depends on platform telemetry | Distracts from AI trust |
| UX burden | Hidden compatibility failures | Manageable dense hub and resolver | Higher moderation/account surfaces | SDK/platform UX constraints | High economy/account UI |
| Performance risk | None | Low local API/event overhead | Medium deployment/event volume | Medium SDK/transport integration | Medium/high service coupling |
| Modding impact | Weak registry story | Strong package manifest spine | Strong but moderation-heavy | Workshop/platform dependent | Risky if inventory gates mods |
| Networking/replay impact | Late integration risk | Strong metadata, hashes, run evidence | Strong but public risk | Strong for platform networks | Misaligned with sync evidence |
| Content cost | Low now, higher later | Moderate fixtures/schemas | High live data + moderation | Medium integration docs/tools | High economy/content treadmill |
| Retention upside | Local only | Replays, challenges, package discovery | Community servers/replays | Friends/lobbies/invites | High but ethically sensitive |
| Ethics/fairness | Clean | Clean if privacy/redaction stays first | Needs moderation/privacy policy | Platform terms apply | Highest risk |

## Service Tier Matrix

| Tier | Services | Scope Decision | Why |
|---|---|---|---|
| Slice A local/core | `/v1/health`, schema/version report, local package registry, join eligibility, deep-link parser, local server supervisor, local replay/report index, diagnostics export, privacy redaction. | Build now. | Directly supports solo/local play, package compatibility, workbench UX, recorder evidence, and future co-op tests. |
| Slice A fixtures | Static `servers.json`, `packages.json`, `replays.json`, resolver fixtures, fake process adapter, compatibility failure rows. | Build now with fixtures. | Lets UX/backend tests run before real public hosting or transport exists. |
| Optional online prototype | Static or heartbeat server directory, package manifest registry, content dependency resolver, replay upload/share sandbox, consented telemetry summaries, daily seed/challenge metadata. | Prototype when useful; keep adapters swappable. | Tests community value without making a launch promise. |
| Platform adapter candidates | Steam server browser, Steam Datagram Relay/GameNetworkingSockets, EOS lobbies/sessions, PlayFab lobbies/servers, self-hosted directory, LAN discovery. | Research/prototype behind the same service contract. | The game should not choose a platform by accident through schema drift. |
| Not a launch commitment yet | Matchmaking, accounts/profiles, cloud save, leaderboards, relay allocation, anti-cheat/trust enforcement, moderation/admin tools. | Design interfaces only when a prototype asks for them. | They imply public service responsibilities and product promises. |
| Research/later | Account economy, gacha/collection inventory, cosmetics marketplace, paid mod storefront. | Research freely; do not couple to Slice A. | These are product/economy decisions, not prerequisites for the best first playable. |

## Backend Objects That Must Stay Shared

| Object | Required Consumers | Minimum Fields |
|---|---|---|
| `ServerSummary` | Hub, deep links, join resolver, replay recorder, future networking tests. | Version, protocol, mode, map/hash, terrain-sync profile, players/bots, package set, content hash, trust/mod safety, join state, heartbeat/expiry, warnings. |
| `PackageManifestSummary` | Package builder, workbench, server browser, replay viewer, diagnostics. | Package id/version, manifest hash, source/provenance, dependencies, script capability flags, trust tier, compatible game/schema versions. |
| `JoinEligibilityResult` | Hub, deep links, package builder, recorder, support diagnostics. | `can_join`/`needs_action`/`blocked`, reason codes, repair/download/workbench route, redaction state, event id. |
| `ReplaySummary` | Replay browser, AI trust harness, death recap, bug reports, future sharing. | Run id, map/hash, package hash, actors, tags, duration, schema version, failure markers, privacy flags. |
| `DiagnosticsReport` | Developer tools, support, AI/replay debugging. | Redacted environment summary, schema versions, latest join failure, package mismatch, crash/exit state, replay/report pointers. |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| OpenSoldat separates engine, base content, launcher, and lobby; its launcher/server rows prove server discovery is UX, not just networking plumbing. | [[comparables/opensoldat-satellites-local-audit]], [[spec/backend-service-hub-slice-a]] | High |
| OpenSoldat base content uses deterministic package hashes for pure server compatibility. | `../comparables_repos/opensoldat-base/README.md`, `../comparables_repos/opensoldat-base/create_smod.py`, [[comparables/opensoldat-satellites-local-audit]] | High |
| Steamworks treats community/dedicated game servers and a unified server browser as first-class, with API access via `ISteamMatchmakingServers`. | Steamworks Game Servers and `ISteamMatchmakingServers` docs in [[references/sources]] | High |
| Steam server queries support map, tags/data, dedicated/secure, not-full/has-player/no-player, address, and direct server rules queries. | Steamworks `ISteamMatchmakingServers` docs in [[references/sources]] | High |
| Steam Datagram Relay can hide IPs and uses game-coordinator/ticket/certificate flows for stronger dedicated-server admission. | Steam Datagram Relay docs in [[references/sources]] | Medium/high; platform-specific |
| Unity Lobby marks inactive lobbies after heartbeat/update gaps and hides inactive public lobbies from query/quick-join results. | Unity Lobby heartbeat docs in [[references/sources]] | High |
| Unity Multiplay server readiness explicitly separates process start from "ready for players" and uses readiness/health checks for allocations. | Unity Multiplay server-readiness docs in [[references/sources]] | High, but service is in transition after March 31, 2026 |
| PlayFab multiplayer services are modular: lobbies, matchmaking, party, and servers can be used independently or combined. | PlayFab multiplayer overview in [[references/sources]] | Medium/high; platform-specific |
| EOS exposes lobbies, sessions, P2P, voice, anti-cheat, metrics, and game services as separable modules. | EOS docs/search results in [[references/sources]] | Medium; needs direct implementation audit before platform choice |

## Current Recommendation

Recommendation: **Option B: local-first service spine + optional adapters.**

Build the service contract now, but keep the runtime deployment local/file-backed until an online prototype proves public service value. The backend should be boring infrastructure that makes the best game easier to build: package compatibility, clear join blockers, replay/debug indexing, local host lifecycle, diagnostics, and hub navigation.

This is not a "small ambition" choice. It preserves optionality for Steam/EOS/PlayFab/self-hosted/LAN later, while avoiding the trap of building accounts, matchmaking, anti-cheat, and economy before actor feel, AI, destruction, replay, and equipment workbench quality are proven.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| BACK-SCOPE-01 | Static/local service can serve health, servers, packages, replays, and join eligibility from fixtures. | Pass if all endpoints load and all schemas are versioned. |
| BACK-SCOPE-02 | Join blockers are concrete and action-oriented. | Pass if every disabled server row maps to a specific package/update/trust/password/replay/action route. |
| BACK-SCOPE-03 | Heartbeat/expiry behavior works without public ops. | Pass if fixture/local rows expire or degrade deterministically. |
| BACK-SCOPE-04 | Local supervisor readiness is structured. | Pass if process states never depend on parsing stdout text. |
| BACK-SCOPE-05 | Package builder and server browser share manifest identity. | Pass if the same package hash fields drive workbench diagnostics and join eligibility. |
| BACK-SCOPE-06 | Replay summaries remain useful offline. | Pass if the hub can browse local replays/reports with backend offline. |
| BACK-SCOPE-07 | Deep links are safe. | Pass if no plain password, invite token, absolute local path, or private IP leaks into events or reports. |
| BACK-SCOPE-08 | Recorder events cover backend UX. | Pass if fetch, heartbeat, join decision, content resolve, deep link, local health, and diagnostics events export through the run-bundle event envelope. |
| BACK-SCOPE-09 | Adapter boundary stays clean. | Pass if Steam/EOS/PlayFab/self-hosted/LAN concepts map into the shared objects without changing core UI components. |
| BACK-SCOPE-10 | Public-service escalation is explicit. | Pass if adding matchmaking/accounts/leaderboards requires a new DR or a reopened DR-013 entry. |

## Risks

| Risk | Mitigation |
|---|---|
| The backend becomes a live-service project too early. | Keep Slice A file-backed/local and ban accounts/matchmaking/economy from the core path until a prototype proves need. |
| Platform docs seduce the design into platform lock-in. | Treat Steam/EOS/PlayFab/Unity as adapters behind shared objects, not the source of truth. |
| Server browser UI becomes noisy. | Use [[spec/ux-wireframes-slice-a]] and [[decisions/dr-012-accessibility-comfort-readability]]: dense table, explicit blockers, scalable text, no color-only state. |
| Mod/package hashes confuse players. | Route issues into workbench actions with readable package names, provenance, and repair/download choices. |
| Diagnostics leak private data. | Keep redaction tests in [[spec/backend-service-hub-slice-a]] and prototype run bundles. |
| The service spine distracts from actor feel. | Sequence implementation after A0/A1 or build only fixture schemas until the first sandbox exists. |

## Revisit Trigger

Reopen this decision when:

- DR-005 produces bandwidth/authority evidence for co-op or PvP.
- A public playtest/server directory is scheduled.
- Package registry or replay sharing becomes a primary retention loop.
- The project chooses a platform backend or store.
- User explicitly decides accounts, leaderboards, gacha/economy, or public matchmaking should move from research into product scope.
- New source evidence contradicts this recommendation.

## Source Trail

- [[spec/backend-service-hub-slice-a]]
- [[systems/networking-backend-frontend]]
- [[comparables/opensoldat-satellites-local-audit]]
- [[systems/replay-determinism-and-run-evidence]]
- [[spec/package-builder-workbench-slice-a]]
- [[spec/ux-wireframes-slice-a]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-010-license-reuse-matrix]]
- Steamworks Game Servers: `https://partner.steamgames.com/doc/features/multiplayer/game_servers`
- Steamworks `ISteamMatchmakingServers`: `https://partner.steamgames.com/doc/api/ISteamMatchmakingServers`
- Steam Datagram Relay: `https://partner.steamgames.com/doc/features/multiplayer/steamdatagramrelay`
- Unity Lobby heartbeat: `https://docs.unity.com/lobby/heartbeat-a-lobby`
- Unity Multiplay server readiness: `https://docs.unity.com/en-us/multiplay-hosting/concepts/server-readiness`
- PlayFab multiplayer overview: `https://learn.microsoft.com/en-us/gaming/playfab/multiplayer/mpintro`
- EOS game services overview: `https://dev.epicgames.com/docs/epic-online-services/index`
