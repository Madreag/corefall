---
type: decision
id: DR-036
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "Active material kernel cannot meet 4K/120 active-region perf budget; AI hazard perception cannot route around systemic hazards reliably; reaction engine produces unfair invisible deaths the player cannot debug; or Barotrauma-style room/atmosphere model conflicts structurally with DR-027 base scope."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[comparables/noita-grade-material-simulation-research|Noita-grade material research]] · [[spec/prototype-roadmap|native roadmap]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-027-combat-base-scope|DR-027]]

# DR-036: Systemic Material Simulation Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-05)
> The game ships **systemic, Noita-grade material simulation** as a core feel pillar. Implementation is **hybrid**: active-region per-pixel material sim (Noita) + rigid-body collision (DR-033) + room/atmosphere networks (Barotrauma) + reaction engine (Powder Toy / Noita Alchemy) + AI hazard perception + replay/event audit. Not unbounded freeform Noita; not pure tile grids. **Curated launch material set** with material lab as expansion surface. See [[comparables/noita-grade-material-simulation-research]] for the 50-source synthesis.

## Decision

**Material simulation is a launch product surface, not a moonshot.** Every material is a verb. Every reaction has a cause chain. Every hazard has an overlay, caption, and replay event. The implementation is staged across a new T-MAT side track and five new milestones (M5.6, M5.7, M6.6, M7.5, M8.5).

This DR ratifies what [[comparables/noita-grade-material-simulation-research]] recommends, elevates it from research to direction, and threads it through the roadmap.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Layer stack | Active material grid + rigid-body physics + terrain solidity grid + room/volume network + pipe/power/signal networks + reaction engine + hazard/affliction layer + AI perception layer + observation/control layer + replay/event layer. See [[comparables/noita-grade-material-simulation-research]] § Recommended Hybrid Architecture. |
| Material schema | Data-first; every material has `id`, `movement_class`, density, viscosity, mass-per-pixel, hardness, thermal/fire fields, phase changes, toxicity/asphyxiation/corrosiveness, conductivity, wetting/stain effects, ingestion effects, container rules, reaction tags, AI affordances, UI overlay, performance tier, network/replay mode. |
| Launch material set | 17 materials: air/empty, dirt/sand, rock/concrete, metal, wood/organic, water, steam/mist, smoke, fire/heat, oil/fuel, acid, toxic sludge/liquid, toxic gas, lava, blood/vomit, electricity charge, pebble/debris. |
| Expansion materials | Slime, brine, coolant, cryo, fuel vapor, foam, nanogel, alchemic precursor, Midas/gold-maker, biological acid/blood variants — gated behind material lab + balance review. |
| Reaction engine | Data-driven pair/triple reactions with priority, temperature thresholds, catalysts, byproducts. Every reaction emits a replay event with cause chain. |
| Room/atmosphere model | Barotrauma-style hulls/gaps/pumps/vents/oxygen/pressure/fire networks for bases, mechs, ships, sealed chambers. Approximate (not real-unit) per Barotrauma's own scope lesson. |
| Pipe/power/signal networks | Stationeers-style atmosphere/pipe/power networks for base equipment (oxygen generators, pumps, vents, filters, sensors, doors, alarms). Sensor-readable + AI-controllable. |
| AI hazard perception | AI reads material/hazard/pressure/electricity fields with the same data players see. AI affordance tags (avoid, seek, use-as-weapon, extinguish-with, neutralize-with, vent, pump). |
| Replay determinism | Material kernel is deterministic; same seed/inputs produce identical material checksums. Same authoritative server-replay model as DR-005 / DR-034. |
| Observation API | `cxctl observe --materials`, `cxctl observe --atmospheres`, `cxctl observe --reactions` for AI agents and tests. New `material`, `reaction`, `atmosphere`, and `affliction` event categories. |
| Performance posture | Active-region budgets per chunk; sleeping chunks; LOD; dirty rects; material kernel chunked at 64×64 minimum (Noita pattern). |
| Modding | Material schema is moddable per DR-006; mods declare reactions, materials, ingestion effects, pipe devices, AI affordances. Server-side mod compatibility enforced per DR-034. |
| Anti-cheat / fairness | Server-authoritative material state in multiplayer per DR-005. Bounded active regions; material events from outside interest range are not delivered. |

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| Pure freeform Noita (every-pixel-everywhere always active) | Performance + AI competence at 4K/120 + 50-100 concurrent players. |
| Hidden chemistry without inspect/replay | Players can't learn what they can't see; AI can't trust what it can't query. |
| Tile-grid hazards only (no per-pixel materials) | Loses the Cortex+Noita physical fantasy; lava droplets / acid splashes / spark trails matter. |
| Real-unit physics simulation | Stationeers-grade engineering pulls focus from combat genre; approximations are fine if consistent. |
| Hundreds of launch materials | Readability collapses; AI can't reason; mod authors get overwhelmed. Curated launch set + lab promotion path is the right shape. |
| Different sim logic for client vs server | Replay/multiplayer determinism breaks; matches DR-034 same-binary policy. |
| AI walking through systemic hazards blindly | Fails DR-022 humanlike bar. AI hazard perception is non-negotiable. |
| Subscription-funded material content packs | Conflicts with DR-031. Material lab + community packs are free; expansions/DLC are content-pack-shape per DR-031. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Stay at DR-007 LEAN ("curated hazards only; Noita-grade as moonshot") | The user's design intent (per [[comparables/noita-grade-material-simulation-research]]) is explicitly Noita-grade systemic causality. Moonshot framing under-promises. |
| Pure Noita (no Barotrauma room model) | Misses the base/mech/sub disaster grammar that DR-027 deep combat-base requires. |
| Pure Barotrauma (room-only, no per-pixel) | Loses the lava-droplet / acid-splash / fire-on-oil-trail combat verbs. |
| Pure ONI / Stationeers (full engineering sim) | Wrong genre; pulls focus from combat. |
| GPU-only kernel | Determinism is harder; cross-platform parity is harder; CI cost is higher. CPU deterministic kernel first; GPU stress test later. |
| Hidden chemistry (Noita-style "discover by accident") | OK for *rare* alchemy/Midas recipes, NOT OK for core combat reactions. Core verbs must be inspectable; rare recipes can be discoverable. |

## Evidence Trail

- Project owner direction (2026-05-05): elevate Noita-grade material simulation from moonshot to active direction; integrate full research synthesis into the roadmap.
- [[comparables/noita-grade-material-simulation-research]] — 558-line research note with 50-source synthesis covering Noita (GDC + 80.lv + RPS + Eurogamer + wiki), Powder Toy (open source + local audit), Barotrauma (public source + modding docs + local code), Oxygen Not Included (mechanics wiki), Stationeers (mechanics + atmosphere + physics wiki), open-source falling-sand projects (EP01 SandSim, GPU-Falling-Sand-CA, BooleanCube, m-camps, tranma, simulake).
- Cross-DR coherence:
  - DR-007 (terrain/material model) LEAN updated to match this direction.
  - DR-033 (full collision physics) projectile-deflection / collision-damage already produces material consequences; DR-036 specifies WHAT consequences.
  - DR-027 (deep combat-base) gets Barotrauma-style hull/gap/pump/vent/oxygen/pressure model.
  - DR-022 (humanlike AI) gets material hazard perception requirement.
  - DR-002 (replay/event architecture) gets `material`, `reaction`, `atmosphere`, and `affliction` event categories.
  - DR-006 (modding data model) gets material schema as a first-class moddable surface.
  - DR-005 / DR-034 / DR-035 (multiplayer/server/MMO) get server-authoritative material state.

## Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Sim cost explodes at 4K/120 | Active-region budgets; sleeping chunks; dirty rects; LOD; perf gates at every milestone. T-PERF + T-MAT track. |
| Unfair invisible deaths | Hazard overlays mandatory; warning audio + captions; replay cause chain; grace windows; AI captions. |
| AI looks stupid around systemic hazards | Hazard perception map + affordance tags + forced AI regression scenarios. AI-MAT acceptance suite. |
| Replay nondeterminism | CPU deterministic kernel; seeded RNG; chunk update order pinned; checksum events; first-divergence reports. |
| Material count balloons | Curated launch set (17); expansion gated behind material lab + balance review. |
| Hidden chemistry feels random | Recipe journal; inspect tool; debrief cause chains; mission hints. |
| Stationeers-style engineering pulls focus | Use approximate consistent rules; expose telemetry only where it serves combat genre. |
| Licensing contamination | Powder Toy is GPL-3 (study-only); Barotrauma source is public-but-not-FOSS (study-only); custom implementation; usage-ledger entries required for any reuse. |
| Community-hosted MMO shards diverge on materials | Server-authoritative; mod hash sync; material schema migration handlers. |

## Prototype / Validation Plan

| Test Pack | Milestone | What It Proves |
|---|---|---|
| MAT-01..MAT-03, MAT-06, MAT-13 minimal | M5.6 Material Kernel | Active material grid + reactions + density + replay determinism; sand/water/steam/oil/fire baseline. |
| MAT-04, MAT-05, MAT-07 | M5.7 Hazard Package | Acid/toxic/electricity/debris damage routes through armor/limbs/equipment. |
| MAT-12 | M6.6 AI Material Competence | AI avoids/uses materials; reason labels for hazard interactions. |
| MAT-09, MAT-10 | M7.5 Base Atmospherics | Hull/gap/pump/vent/oxygen/pressure network; flooding/fire/smoke through rooms. |
| MAT-11, MAT-14 | M8.5 Material Lab | Recipe/stamp/test editor; designer authors a tiny reaction puzzle. |
| MAT-08 | M5.7 + M8.5 | Ingestion/vomit/container loop. |
| Material lab regression suite | T-MAT lifelong | All MAT-* slices keep passing as new materials/reactions land. |

## Cross-DR Anchors

| DR | Tie |
|---|---|
| DR-007 terrain/material model | LEAN amended to match this direction; remains OPEN until M5.6/M5.7 prototype evidence settles implementation specifics. |
| DR-027 combat-base scope | Barotrauma-style room/atmosphere model lands as base infrastructure. |
| DR-033 full collision physics | Collision impulse → material reaction → damage chain. |
| DR-022 humanlike AI bar | Hazard perception is testable per AI-H-MAT scenarios. |
| DR-002 replay/event architecture | New `material`, `reaction`, `atmosphere`, and `affliction` event categories. |
| DR-006 modding data model | Material schema is a first-class moddable surface. |
| DR-005 / DR-013 / DR-034 / DR-035 | Server-authoritative material state in multiplayer/MMO modes. |
| DR-031 content economy | Material lab + community packs are free; commercial expansions follow DR-031. |

## Revisit Trigger

- Active material kernel cannot meet 4K/120 active-region perf budget after M5.6/M5.7 evidence.
- AI hazard perception cannot route around systemic hazards reliably (AI-MAT regression failures).
- Reaction engine produces unfair invisible deaths the player cannot debug.
- Barotrauma-style room/atmosphere model conflicts structurally with DR-027 base scope.
- A future Stationeers-grade engineering direction would amend this DR (open follow-up).

## Source Trail

- Project owner direction (2026-05-05).
- [[comparables/noita-grade-material-simulation-research]] (50-source research synthesis).
- [[decisions/dr-002-replay-event-architecture]]
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[decisions/dr-007-terrain-material-model]]
- [[decisions/dr-013-backend-service-scope]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-033-full-collision-physics-direction]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[systems/material-and-mobility-affordance-schema]]
- [[systems/physics-and-destruction-models]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[spec/full-collision-physics-plan]]
- [[research-log/2026-05-05-systemic-material-simulation-direction]]
