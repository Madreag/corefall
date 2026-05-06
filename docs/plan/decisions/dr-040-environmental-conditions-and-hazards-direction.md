---
type: decision
id: DR-040
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Aggregator runs out of perf budget on Steam Deck floor at 50-actor scenarios; per-tick bundle determinism breaks across replay; modder signal extensions produce unbounded compute cost; or the aggregation contract proves to add zero value over per-kernel direct queries after M5.10 evidence."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|tracker]] · [[spec/environmental-conditions-model|environmental conditions spec]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction|DR-037]] · [[decisions/dr-038-universal-gravity-and-ballistics-direction|DR-038]] · [[decisions/dr-039-celestial-bodies-and-worlds-direction|DR-039]]

# DR-040: Environmental Conditions And Hazards Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06; chose **add EnvironmentSignal aggregation layer** option)

## Decision

A per-tick per-actor `EnvironmentSignal` struct aggregates atmospheric (DR-037), gravitational (DR-038), thermal, radiation, photic, EM, weather, water, acoustic, day/night, and comms slices into a single source of truth. Every consumer (AI doctrine, HUD, accessibility caption, replay event recorder, audio mixer, mission director) reads from the bundle. No consumer queries individual kernels for environmental data.

Hazard taxonomy is closed-enum: `hypoxic`, `combustible_atmosphere`, `toxic_atmosphere`, `breach_decomp`, `hyperthermic`, `hypothermic`, `radiation`, `low_visibility`, `glare`, `em_disruption`, `wind_force`, `drowning_hazard`, `vacuum_no_voice`, `comms_blackout`, `gravity_shift`. Origin gating per [[spec/origin-reaction-and-resource-model]].

## What This Locks In

| Aspect | Commitment |
|---|---|
| Aggregation layer | `cf-environment` crate; runs after all signal-producing kernels per [[spec/environmental-conditions-model#Tick Schedule (where the aggregation runs)]]. |
| Bundle struct | Locked per [[spec/environmental-conditions-model#The EnvironmentSignal Struct]]. |
| Hazard taxonomy | Closed-enum; modders extend via data row. |
| Origin gating | Lives at consumer (AI doctrine, HUD); producer reports raw signal. |
| Replay | Bundle deltas (sparse), full snapshot per scenario-second for debug scrub. |
| Performance | SoA actors; SIMD-friendly; sleeping actors skip; per-tick budget bounded. |

## What This Explicitly REJECTS

- Per-system parallel queries for environmental data ("AI reads atmosphere from cf-atmos AND weather from cf-mission AND gravity from cf-physics directly"). CI grep gate enforces.
- Hidden environment signals that don't emit replay events.
- Per-frame aggregation (must be per-tick deterministic).

## Why Not The Alternatives

- **Keep subsystems independent**: failure mode where AI gets stale views is structural, not a tooling problem. Aggregator pattern eliminates the failure forever.
- **Defer decision**: the aggregator is needed for AI Environmental Competence (M6.6) and for the per-actor HUD chip rendering (M4 placeholder → M5.10 real).

User chose **Yes — add EnvironmentSignal aggregation layer (M5.10)** explicitly.

## Cross-DR Anchors

- DR-022 humanlike AI — AI doctrine reads EnvironmentSignal as primary perception input.
- DR-036, DR-037, DR-038, DR-039 — produce slices that feed the bundle.
- DR-043 voice/radio — produces acoustic + EM + comms slices.
- DR-002 replay/event architecture — adds `environment` event category.

## Revisit Trigger

- Aggregator runs out of perf budget on Steam Deck floor at 50-actor scenarios.
- Per-tick bundle determinism breaks across replay.
- Modder signal extensions produce unbounded compute cost.
- Aggregation contract proves to add zero value over per-kernel direct queries after M5.10 evidence.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/environmental-conditions-model]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]
