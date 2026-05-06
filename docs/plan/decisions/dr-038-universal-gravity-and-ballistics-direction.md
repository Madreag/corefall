---
type: decision
id: DR-038
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Per-cell gravity sampling cannot meet active-region perf budget at 60Hz/120Hz on Steam Deck floor; per-cell override scheme produces nondeterministic ballistics across replay; gravity-field network sync produces visible client/server divergence in PvP; or universal-gravity grammar conflicts with a future cinematic gameplay surface (e.g., procedural rotation gravity) such that the project owner amends the field shape."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/gravity-and-ballistics-model|gravity/ballistics spec]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry spec]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction|DR-037]]

# DR-038: Universal Gravity And Ballistics Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> The game ships **universal gravity**: one `GravityField` records the local gravity vector everywhere; every system that has any physics-y behavior reads from it. Materials, projectiles, actors, equipment, debris, gases, liquids, sparks, casings — all read the same field. Per-planet ambient g; per-cell or per-region override for special zones (gravity wells, low-g labs, magnetic boots, damaged grav generators). Deterministic ballistics with atmosphere-density-aware drag. Lands at extended M5.5 (collision integration) + M5.6 (material kernel density layering) + new M5.9 (atmospherics kernel coupling). M0/M1 carry placeholder `gravity_g` field on scenario manifest only.

## Decision

**Gravity is one field, every system subscribes.** No subsystem computes its own gravity. No hardcoded `9.81 m/s²` in production code. Every per-tick physics integration reads `GravityField::sample(pos)` to get the local `(direction, magnitude)`. Per-planet ambient comes from the scenario manifest; per-cell or per-region overrides come from gameplay (gravity wells, base grav generators, magnetic boots, low-g labs).

This DR ratifies what [[spec/gravity-and-ballistics-model]] specifies, elevates universal gravity from "implicit in M5.5 collision" to "core direction with locked grammar", and threads it through the roadmap.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Universal field | `cf_physics::gravity::GravityField` is the single source of truth. Layered: cell overrides > region overrides > ambient. Sampled per-tick per-entity. |
| Per-planet ambient | Scenario manifest field `gravity_g` is the multiplier of Earth standard 9.81 m/s². Locked defaults: Earth 1.000, Mars 0.378, Moon 0.166, Europa 0.134, Mimas 0.0064, Vulcan 0.910, Venus 0.904, zero-g 0.0, reverse-g 1.0 with direction (0, +1). Modders add new ambients via data row. |
| Per-cell override | Sparse map of cell → `GravityVec` for special zones (gravity wells, low-g labs, magnetic boots, damaged grav generators, scripted cinematics). Time-stable until gameplay event toggles them. Replicated server-authoritative. |
| What reads it | Actor controller (walking gait, jump apex, fall acceleration, fall damage threshold per [[spec/origin-reaction-and-resource-model]]); projectile system (ballistic arcs); equipment items / debris / gibs / casings (kinematics); material kernel (density layering for liquids and sand); atmospherics density layering ([[spec/atmospherics-and-chemistry-model]] gas stratification by molar mass × g); body damage (fall damage per local g); liquid pipes / tanks (gravity-driven flow); visual effects (sparks, embers, droplets); AI doctrine (jump/fall/grenade-arc planning); net code (server authoritative). |
| Ballistic math | `a = (F_gravity + F_drag + F_collision) / m`; `F_drag = -0.5 · ρ_local · v · |v| · C_d · A`. ρ_local from atmospherics; in vacuum, drag ≈ 0 → projectiles fly farther; in dense atmospheres (Venus 239 kPa), heavy projectiles tunnel and light ones tumble. |
| Atmospherics coupling | Density layering in materials and atmospherics reads g per cell. CO2 sinks; H2 rises; oil floats on water under positive g; everything mixes uniformly at 0g; everything flips at reverse g. Stratification kernel runs on sealed atmospheres with multi-gas mixes and significant ΔM. |
| Per-cell override classes | Gravity Generator (base module: ambient + 1g down inside region); Gravity Well (anomaly / weapon: per-cell vector toward center point); Low-g Lab (per-region 0.1g down); Magnetic Boots (per-actor override: 1g toward surface normal in actor frame); Damaged Gravity Generator (intermittent toggle); Reverse-g Chamber (puzzle / cinematic). All emit replay events on activate/deactivate. |
| Replay determinism | Same seed + same actor inputs + same gravity field + same atmosphere = byte-identical event stream + final state. Integration order is fixed; no platform-specific atomics in the inner loop. |
| Observation API | `cfctl observe --gravity` returns ambient + active overrides; `cfctl inspect gravity <region-id>` shows per-region vector and source. New `gravity` and `ballistics` event categories extending DR-002 schema. |
| Performance posture | `GravityField::sample(pos)` is hot-path; SoA storage; cache-friendly; SIMD-friendly per-cell array. Per-projectile integration runs at fixed-tick alongside M5.5 contact solver. Stratification kernel runs sleeping-aware (only dirty atmospheres). |
| Modding | New gravity override class / new planet ambient / new projectile drag profile are all data-driven schemas validated by `cargo run -p cf-mod -- validate content/`. |

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| `const GRAVITY_MS2: f32 = 9.81` anywhere in production code | Breaks per-planet scenarios; breaks per-cell overrides; breaks the "universal" promise. |
| Different gravity for actors vs projectiles vs debris | Inconsistent feel; debris falls "wrong" relative to bullets; players notice; AI plans break. |
| Different gravity for material density layering vs object physics | Breaks the "oil floats on water and dropped weapons fall the same way" intuition. |
| Hardcoded arcade arcs for grenades | Players who learn grenade arc on Earth must re-learn on Mars; defeats the per-planet feel. Use the same ballistic integrator. |
| Gravity ignored for gases | Gas stratification (CO2 at floor, H2 at ceiling) is a Stationeers-grade signal we want; flat-mix atmospheres feel wrong in 1g. |
| Per-frame gravity recomputation | Determinism breaks; integration order matters. Per-fixed-tick only. |
| Net replication of ambient gravity per tick | Wasteful; ambient is set on scenario load and changes only via override events. |
| Magnetic boots that "just feel sticky" with no real frame change | Breaks the universal-field promise; feels arbitrary. Real per-actor override applied each tick. |
| GPU-only gravity field | Determinism is harder; cross-platform parity is harder. CPU deterministic field first. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| One global gravity constant (1g down everywhere) | Loses the per-planet feel that DR-036 + DR-037 + scenario diversity all promise. |
| Per-system gravity (actors read 1g, projectiles read tuned arc, debris reads spline) | Loses the "everything reads the same field" promise the user explicitly requested. |
| Per-region only (no per-cell override) | Sufficient for most gameplay but kills certain anomaly weapons (gravity grenades) that need a small radius vector field. |
| Real-life Earth = 9.806 m/s² always | We use 9.81 as the named constant for clarity; tuning happens through `gravity_g` multiplier, not by changing the name. |
| Procedurally rotated gravity (rotating ship interiors) | Possibly cool for a future expansion but adds simulation complexity beyond launch scope; flagged as a revisit trigger only. |

## Evidence Trail

- Project owner direction (2026-05-06): "gravity should be a thing that affects materials, bullets, entities, everything really".
- [[spec/gravity-and-ballistics-model]] — canonical contract with locked field shape, per-planet defaults, ballistic math, atmospherics coupling, override classes, GRAV-A acceptance tests.
- Cross-DR coherence:
  - DR-007 (terrain/material model) — material density layering reads gravity from one source.
  - DR-033 (full collision physics) — M5.5-008 impulse-to-damage routing now includes wind force from atmospherics ΔP and ballistic drag from atmospherics ρ_local; both depend on gravity for actor / debris kinematics.
  - DR-036 (systemic material simulation) — material density layering and granular flow read gravity per cell.
  - DR-037 (Stationeers-grade atmospherics) — atmospherics density layering reads gravity for gas stratification by molar mass.
  - DR-027 (deep combat-base) — base gravity generator is a base module; damaged grav generator is a mission consequence.
  - DR-022 (humanlike AI) — AI plans jumps/falls/grenade arcs against sampled g; reads override regions before walking heavy equipment into low-g zones.
  - DR-002 (replay/event architecture) — new `gravity` and `ballistics` event categories.
  - DR-005 / DR-034 / DR-035 — server-authoritative gravity field; deltas replicated.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Per-cell sampling becomes hot-path bottleneck | SoA storage; cache-friendly; SIMD-friendly per-cell array; layered lookup with cell-override sparsity check first; stratification kernel runs only on dirty atmospheres. |
| Per-cell override produces nondeterminism across replay | Override toggles are events with parent_event_id chains; integration order pinned; no platform-specific atomics. Run-bundle ATOM-15 + GRAV-A-10 prove byte-identity. |
| Net sync cost in MMO shards | Ambient changes rarely; overrides replicate as events not as per-tick state; server authoritative. |
| Magnetic boots produce per-actor frame inversion bugs | Per-actor override applied each tick at integrator boundary; replay records every override state change with `gravity.entity_entered_region` / `_exited_region` events. |
| Gravity wells trivialize all combat | Per-weapon balance review; spawn rate gates; mission-only or rare-loot status; M5.5 + M5.5 collision must enforce limits. |
| Per-planet g feels weird in default Earth scenarios | Tutorial scenarios stay on Earth ambient; alternate-planet scenarios introduce g-shift gradually with HUD readouts. |
| Vacuum projectile ranges trivialize cover-based gameplay | Mission-design responsibility; vacuum scenarios use tighter sightlines and movable cover; M8.5 lab calibration. |
| Implementation in M5.5 underestimates scope | Full integration spans M5.5 + M5.6 + M5.9; M5.5 lands the GravityField struct + actor/projectile sampling; later milestones land density-layering and atmospherics coupling. |

## Prototype / Validation Plan

| Test Pack | Milestone | What It Proves |
|---|---|---|
| GRAV-A-01..GRAV-A-03 | Extended M5.5 (Full Collision Gauntlet) | Drop tests on Earth / Mars / Moon: per-planet g_factor reflected in fall acceleration and impact velocity; replay determinism. |
| GRAV-A-04..GRAV-A-05 | Extended M5.5 + M5.9 | Projectile arc tests on Earth + Mars + vacuum (Moon); drag-aware integration; per-projectile event stream. |
| GRAV-A-06..GRAV-A-07 | Extended M5.5 + M7.5 | Per-cell override regions: low-g lab, magnetic boots; replay logs `gravity.entity_entered_region` and per-actor override state changes. |
| GRAV-A-08 | M5.6 Material Kernel | Liquid layering at 1g vs 0g: oil-on-water settles vs mixed; kernel reads g per cell. |
| GRAV-A-09 | M5.9 Atmospherics-Grade Kernel | Gas stratification: CO2 at floor / H2 at ceiling at 1g; uniform at 0g; per-tick partial-pressure deltas reflect local g × ΔM. |
| GRAV-A-10 | Extended M5.5 + M5.6 + M5.9 + M3 | Determinism replay across full mixed-gravity scenario for 10000+ ticks; byte-identical event stream and final state. |
| Gravity regression suite | T-PHYS lifelong | All GRAV-* slices keep passing as new override classes / new planet ambients / new projectile drag profiles land. |

## Cross-DR Anchors

| DR | Tie |
|---|---|
| DR-007 terrain/material model | Density layering / settling / flow read gravity. |
| DR-027 combat-base scope | Base gravity generator module; damaged grav generator mission consequence. |
| DR-033 full collision physics | Universal gravity is the integration step inside the M5.5 contact solver; ballistic drag couples to atmospherics. |
| DR-036 systemic material simulation | Material density / granular flow / liquid settling read gravity per cell. |
| DR-037 Stationeers-grade atmospherics | Gas stratification by molar mass × local g; wind force on entities from ΔP at proportional impulse with gravity-affected mass. |
| DR-022 humanlike AI bar | AI plans jumps / falls / grenade arcs / equipment routes against sampled g. |
| DR-002 replay/event architecture | New `gravity` + `ballistics` event categories. |
| DR-006 modding data model | Override class / planet ambient / projectile drag profile schemas are first-class moddable surfaces. |
| DR-005 / DR-013 / DR-034 / DR-035 | Server-authoritative gravity field with override deltas replicated. |
| DR-031 content economy | Gravity-anomaly content packs follow DR-031 monetization rules. |

## Revisit Trigger

- Per-cell gravity sampling cannot meet active-region perf budget at 60Hz/120Hz on Steam Deck floor after M5.5 evidence.
- Per-cell override scheme produces nondeterministic ballistics across replay (any first-divergence is a hard halt).
- Gravity-field network sync produces visible client/server divergence in PvP after M11/M12 evidence.
- Universal-gravity grammar conflicts with a future cinematic gameplay surface (e.g., procedural rotation gravity for ring-station interiors) such that the project owner amends the field shape.
- A future "real-life Earth = 9.806 m/s²" toggle for realism players that needs schema migration.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/gravity-and-ballistics-model]] — canonical contract.
- [[research-log/2026-05-06-origin-reaction-and-resource-design-intent]]
- [[research-log/2026-05-06-atmospherics-and-chemistry-stationeers-research]]
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-007-terrain-material-model]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[decisions/dr-037-stationeers-grade-atmospherics-direction]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/full-collision-physics-plan]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
