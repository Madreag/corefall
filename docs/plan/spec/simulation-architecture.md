---
type: spec
status: stub
ready_when: "DR-001 and DR-007 close; terrain prototype meets perf budget."
---

← [[spec/index|spec section]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]] · [[engine/architecture|engine architecture]] · [[systems/physics-and-destruction-models|physics/destruction]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# Simulation Architecture

> [!warning] Stub

## What goes here when ready

- Update order (frame, sim sub-tick, AI, terrain, audio).
- Entity model and core data shapes.
- Material/integrity model, curated hazard set, and tool/movement affordance columns.
- Path-cost graph and invalidation contract.
- Determinism boundary (where we promise it, where we don't).

## Exploratory Schema Pressure

> [!info] Not authoritative yet
> These are research-derived requirements to test in the terrain/actor sandbox before DR-007 closes.

| Field Family | Candidate Fields | Evidence |
|---|---|---|
| Physical resistance | hardness, integrity, cohesion, density, debris type | Cortex materials; [[comparables/the-powder-toy-local-audit]] |
| Tool affordance | diggable, drillable, beam_cuttable, explosive_carvable, repairable | Cortex terrain tools; [[comparables/openlierox-local-audit]] |
| Mobility affordance | anchorable, nohook, slippery, climbable, jet_safe, path_cost | OpenLieroX rope/material flags; AI path notes |
| Hazard behavior | flammable, hot, toxic, electric, corrosive, damaging_on_touch | [[decisions/dr-007-terrain-material-model]] |
| Visibility/support | blocks_light, blocks_line_of_sight, supports_structure, collapse_hint | OpenLieroX blocks-light flag; UX overlay needs |
| Replay/network | semantic_event_kind, dirty_rect, snapshot_frequency, deterministic_rule | [[systems/replay-event-architecture]], [[decisions/dr-005-multiplayer-posture]] |

Prototype rule: every field must either affect player decisions, AI decisions, mod validation, replay/network serialization, or visible feedback. Otherwise it stays out of the launch schema.

## Current Prototype Target

[[spec/terrain-material-sandbox-slice-a]] is the buildable test for this stub. It defines the eight-material fixture, three-lane terrain lab, event contract, MAT-T-01..MAT-T-10 acceptance tests, dirty-region/path metrics, and kill criteria that should feed DR-007 before any terrain backend claim becomes authoritative.

## Inputs

- [[engine/architecture]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[engine/physics-destruction]]
- [[engine/projectile-to-impact-lifecycle]]
- [[systems/physics-and-destruction-models]]
- [[decisions/dr-001-engine-strategy]]
- [[decisions/dr-007-terrain-material-model]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
