---
type: decision
id: DR-039
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Orbital math kernel cannot meet performance budget; per-shard world catalog conflicts with MMO bandwidth; modder schema produces unbounded world counts that break determinism replay; or full-astrography scope (orbital position + comms light-lag) proves to drive zero observable gameplay value after M5.10/M7.7 evidence."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|tracker]] · [[spec/celestial-bodies-and-worlds-model|worlds spec]] · [[decisions/dr-016-setting-and-world-frame|DR-016]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction|DR-037]] · [[decisions/dr-038-universal-gravity-and-ballistics-direction|DR-038]]

# DR-039: Celestial Bodies And Worlds Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06; chose **full astrography with orbital math + comms latency** option)

## Decision

A canonical `World` record is the single source of truth for every world-level signal: classification (planet/moon/asteroid/sun/station/anomaly), per-world atmosphere ambient, per-world `gravity_g`, surface terrain template, day length / rotation period / axial tilt / solar distance / orbital period (full astrography with simplified circular Keplerian orbital math), magnetic field, ambient radiation, ore deposits, weather variation table, lore tags, visual palette. **All other subsystems read from this record and stop carrying their own copies.**

12-world launch catalog: `sol`, `earth`, `earth_moon`, `mars`, `phobos`, `deimos`, `europa`, `mimas`, `vulcan`, `venus`, `belt_asteroid` (representative), `orbital_station` (representative). Modders add more via data row.

Light-lag comms latency is computed deterministically from orbital positions; mission director declares per-scenario comms policy.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Schema | `World` record with required fields per [[spec/celestial-bodies-and-worlds-model#World Schema]]. |
| Atmosphere ambient ownership | World DECLARES; cf-atmos kernel IMPLEMENTS. Atmospherics page's per-planet table moves to a cross-link. |
| Gravity_g ownership | World DECLARES; cf-physics::gravity IMPLEMENTS. Gravity page's per-planet table moves to a cross-link. |
| Surface terrain template | World references; M2 + M5.6 kernels resolve. |
| Orbital math | Simplified circular Keplerian (parent + semi-major axis + period + phase + rotation + tilt). Closed-form per-tick position + distance + comms-latency lookup. Deterministic across replay. |
| Comms latency | Computed from orbital positions; mission director declares per-scenario policy (`earth_anchored` / `local_authority` / `full_realtime` / `scripted_lag_band`). |
| MMO shards | Each shard declares which subset of worlds it hosts; cross-shard travel is a portal, not seamless. |
| Modder extensibility | Schema validates; `cargo run -p cf-mod -- validate content/worlds/` enforces. |
| Replay determinism | Same scenario time + same world records = byte-identical orbital positions + comms-latency values. |

## What This Explicitly REJECTS

- Full real-life Keplerian elliptical math at launch (deferred to post-launch toggle).
- N-body perturbation simulation (out of scope forever).
- Real-time free-roam multi-world travel (deferred to post-launch).
- Hand-coded per-system queries for "what's Mars's gravity?" — must read from World.
- Per-subsystem world tables that drift over time.

## Why Not The Alternatives

- **Worlds catalog only (no astrography)**: cuts orbital math + comms latency. Insufficient for "current planet" + multi-world campaigns the user committed to.
- **Worlds + light astrography (no comms latency)**: gives orbital position but skips the light-lag mission flavor. Missed opportunity for tactical depth.
- **Full Keplerian elliptical at launch**: too expensive for game; no observable gameplay value over circular for combat-tier missions.

User chose **Full astrography with orbital math + comms latency** explicitly.

## Cross-DR Anchors

- DR-007 terrain/material model — World.surface_template feeds M2 + M5.6 generation.
- DR-016 setting and world frame — World catalog implements the lore frame DR-016 declared.
- DR-027 combat-base scope — Bunker scenarios live on World instances.
- DR-035 persistent MMO architecture — shards declare world subsets.
- DR-036 systemic material simulation — material kernel reads world ambient.
- DR-037 Stationeers-grade atmospherics — atmospherics kernel reads world atmosphere ambient.
- DR-038 universal gravity — gravity field reads world gravity_g.
- DR-040 environmental conditions — World data feeds the EnvironmentSignal aggregator.
- DR-041 mining and extraction — World ore deposits feed mining kernel.

## Revisit Trigger

- Orbital math kernel cannot meet perf budget on Steam Deck floor.
- Per-shard world catalog conflicts with MMO bandwidth at 50-200 concurrent.
- Modder schema produces unbounded world counts that break determinism replay.
- Full-astrography scope produces zero observable gameplay value (in which case demote to "worlds + light").

## Source Trail

- Project owner direction (2026-05-06). User chose option "Full astrography with orbital math + comms latency" via AskUser.
- [[spec/celestial-bodies-and-worlds-model]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]
