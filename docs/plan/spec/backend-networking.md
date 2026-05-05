---
type: spec
status: stub
ready_when: "DR-005 closes; bandwidth prototype meets target."
---

← [[spec/index|spec section]] · [[spec/backend-service-hub-slice-a|backend service/hub Slice A]] · [[systems/networking-backend-frontend|networking]] · [[engine/network-terrain-replication-lifecycle|terrain replication]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[comparables/opensoldat-satellites-local-audit|OpenSoldat satellites]]

# Backend / Networking Posture

> [!info] Current posture
> [[decisions/dr-013-backend-service-scope]] sets the backend service boundary: build a local-first service spine that supports play, package compatibility, replay/debug, diagnostics, hub UX, and future co-op experiments. Keep accounts, matchmaking, public PvP service commitments, anti-cheat enforcement, and economy/gacha layers as research/prototype tracks until DR-005 and real runs justify them.

## Service Scope Summary

| Layer | Status | Examples |
|---|---|---|
| Core local spine | Build in Slice A | `/v1/health`, schema versions, local package registry, join eligibility, deep-link parser, local server supervisor, replay/report index, diagnostics export, privacy redaction. |
| Fixture-backed backend | Build in Slice A | Static `servers.json`, `packages.json`, `replays.json`, resolver fixtures, fake supervisor, package mismatch rows, stale heartbeat cases. |
| Optional online prototypes | Prototype when useful | Static/heartbeat server directory, manifest registry, replay upload/share sandbox, consented telemetry summaries, daily seed/challenge metadata. |
| Platform adapters | Research behind the shared contract | Steam server browser/SDR/GameNetworkingSockets, EOS lobbies/sessions, PlayFab lobby/server services, self-hosted directory, LAN discovery. |
| Not launch commitments | Require future evidence/DR | Matchmaking, accounts/profiles, cloud save, leaderboards, relay allocation, anti-cheat/trust enforcement, moderation/admin. |
| Research/later economy | Research freely, do not couple to Slice A | Account economy, gacha/collection inventory, cosmetics marketplace, paid mod storefront. |

## What goes here when ready

- Launch posture (solo + local co-op; co-op-ready arch) once DR-005 has evidence.
- Server authority model + bandwidth budget.
- Backend service implementation details from [[decisions/dr-013-backend-service-scope]] and [[spec/backend-service-hub-slice-a]].
- Server discovery schema: version, region, rules, content hashes, required packages, mod trust, player/bot counts, replay compatibility, and join eligibility.
- Launcher/hub role: local game, server browser, replays, settings, mods/workbench, diagnostics.
- Replay/network evidence bridge: event volume, snapshot size, dirty terrain chunk checksums, content hashes, replay schema, and first-divergence reports from [[systems/replay-determinism-and-run-evidence]].
- Slice A implementation requirements for the backend/hub live in [[spec/backend-service-hub-slice-a]].
- Tracks not yet promoted to launch (prototype-only): live PvP, matchmaking, account economy. Each has its own DR or moonshot register entry when it matures.

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
