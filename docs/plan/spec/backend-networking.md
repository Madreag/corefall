---
type: spec
status: stub
ready_when: "DR-005/DR-013/DR-034/DR-035 stay coherent; M9-M12 bandwidth/server evidence meets target."
---

← [[spec/index|spec section]] · [[spec/backend-service-hub-slice-a|backend service/hub Slice A]] · [[systems/networking-backend-frontend|networking]] · [[engine/network-terrain-replication-lifecycle|terrain replication]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[comparables/opensoldat-satellites-local-audit|OpenSoldat satellites]]

# Backend / Networking Posture

> [!info] Current posture
> [[decisions/dr-013-backend-service-scope]] sets the backend service boundary: build a local-first service spine for solo/private play, then extend the same contracts into `cf-server`, `lobby_directory`, public-server discovery, account adapters for public shards, anti-cheat foundation, and MMO persistence as M9-M12 evidence matures. Optional economy/gacha-like layers remain separate, dormant, default-off research tracks per [[decisions/dr-057-optional-gacha-battle-pass-and-private-prototype-license-posture|DR-057]].

## Service Scope Summary

| Layer | Status | Examples |
|---|---|---|
| Core local spine | Build in Slice A | `/v1/health`, schema versions, local package registry, join eligibility, deep-link parser, local server supervisor, replay/report index, diagnostics export, privacy redaction. |
| Fixture-backed backend | Build in Slice A | Static `servers.json`, `packages.json`, `replays.json`, resolver fixtures, fake supervisor, package mismatch rows, stale heartbeat cases. |
| Public server services | Build as M9-M12 online modes mature | `lobby_directory`, server browser, account adapter for public shards, anti-cheat foundation, persistence/journal services, server observability. |
| Platform adapters | Research behind the shared contract | Steam server browser/SDR/GameNetworkingSockets, EOS lobbies/sessions, PlayFab lobby/server services, self-hosted directory, LAN discovery. |
| Still not launch commitments | Require future evidence/DR | Ranked matchmaking, cloud save, leaderboards, first-party relay allocation, tournament-grade anti-cheat, moderation/admin product. |
| Research/later economy | Research freely, do not couple to Slice A | Account economy, gacha-like collection inventory, cosmetic battle pass, cosmetics marketplace. Must default off and pass DR-057 activation gates before release-facing use. |

## What goes here when ready

- Launch/product posture from [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-034-dedicated-server-application]], and [[decisions/dr-035-persistent-mmo-architecture]].
- Server authority model + bandwidth budget.
- Backend service implementation details from [[decisions/dr-013-backend-service-scope]] and [[spec/backend-service-hub-slice-a]].
- Server discovery schema: version, region, rules, content hashes, required packages, mod trust, player/bot counts, replay compatibility, and join eligibility.
- Launcher/hub role: local game, server browser, replays, settings, mods/workbench, diagnostics.
- Replay/network evidence bridge: event volume, snapshot size, dirty terrain chunk checksums, content hashes, replay schema, and first-divergence reports from [[systems/replay-determinism-and-run-evidence]].
- Slice A implementation requirements for the backend/hub live in [[spec/backend-service-hub-slice-a]].
- Tracks not yet promoted to product commitments: ranked matchmaking, account economy, cosmetic battle pass, gacha-like collection, first-party cloud hosting, tournament anti-cheat product, and live monetization. Optional economy hooks follow DR-057 and need a future activation DR when they mature.

## Inputs

- [[systems/networking-backend-frontend]]
- [[engine/network-terrain-replication-lifecycle]]
- [[systems/replay-event-architecture]]
- [[systems/replay-determinism-and-run-evidence]]
- [[comparables/soldat-and-opensoldat]]
- [[comparables/opensoldat-satellites-local-audit]]
- [[spec/backend-service-hub-slice-a]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-013-backend-service-scope]]
