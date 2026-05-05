---
type: decision
id: DR-007
status: open
priority: P0
revisit_trigger: "M5.6/M5.7 active-material kernel + hazard package prototypes settle implementation specifics (CPU vs GPU kernel, exact material count, chunk size, snapshot cadence) at perf target; or M5.6 evidence forces a different architecture choice; or DR-036 direction is amended."
---

← [[decisions/index|decision records]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]] · [[comparables/noita-grade-material-simulation-research|Noita-grade material research]] · [[systems/physics-and-destruction-models|physics/destruction]] · [[systems/material-and-mobility-affordance-schema|material schema]] · [[engine/terrain-mutation-and-pathfinding-lifecycle|terrain mutation]] · [[engine/terrain-materials|terrain/materials]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# DR-007: Terrain And Material Model

> [!info] Status: OPEN; LEAN updated 2026-05-05: hybrid active-region pixel sim + curated launch material set + Noita-grade systemic ambition (per DR-036). Implementation specifics remain open until M5.6/M5.7 prototype evidence.

> [!note] Direction parent
> The architecture-level direction (Noita-grade systemic causality, hybrid active-region + room/atmosphere + reaction engine) is closed in [[decisions/dr-036-systemic-material-simulation-direction]]. This DR remains OPEN to track implementation specifics (CPU vs GPU kernel, exact material count, chunk size, snapshot cadence) per M5.6/M5.7 evidence.

## Context

The terrain/material model is the simulation backbone. It controls performance, AI pathfinding, networking, UX overlays, and modding. Cortex uses pixel terrain + material integrity + atom collision; Noita uses falling-sand cellular automata; Teardown uses voxels. We must choose a launch scope. See [[systems/physics-and-destruction-models]] and [[engine/terrain-mutation-and-pathfinding-lifecycle]].

## Options

| Option | Summary |
|---|---|
| A. Erasure-only terrain | Pixels are present/absent; no material. |
| B. Cortex-style materials (solids only) | Material id + integrity + priority + piling; no liquids/gases. |
| C. Cortex + curated hazards | Solids + small set of hazards (fire, gas, electric, water). |
| D. Noita-grade simulation | Per-pixel material reactions; liquids, gases, heat. |
| E. Hybrid: Cortex solids + tile-based hazards (not per-pixel) | Hazard tiles, not pixel-by-pixel. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Cheap; deterministic; trivial sync. | Loses Cortex identity; flat tactical depth. | None. |
| B | Restores Cortex identity; manageable cost. | Less "wow" than D. | Path-cost performance under heavy edits. |
| C | Adds tactical hazard layer; reasonable cost. | Authoring + AI cost for hazards. | Number of hazards before readability collapses. |
| D | Maximum spectacle; emergent strategy potential. | Production-heavy at solo-first scope; AI is hard. | Whether moonshot prototype changes the launch calculus. |
| E | Hazards without per-pixel cost. | Hybrid edges feel artificial. | Whether players notice the seam. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Lowest | Strong | Strongest | Spectacle | Strong |
| Readability | High | High | Medium | Lowest | Medium |
| AI burden | Lowest | Medium | Medium-high | Highest | Medium |
| UX burden | Low | Medium | Medium-high | Highest | Medium |
| Performance risk | Lowest | Medium | Medium | Highest | Medium |
| Modding impact | Low | Medium | High | Highest (and chaotic) | High |
| Networking/replay impact | Lowest | Medium | Medium | Highest | Medium |
| Content cost | Lowest | Medium | High | Highest | Medium |
| Retention upside | Low | Medium | High | Highest if shippable | Medium-high |
| Ethics/fairness | High | High | High | Medium | High |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Cortex material model with integrity/priority/piling already exists and works. | [[engine/terrain-materials]] | High |
| Path-cost recalc has budget caps and can be starved. | [[engine/terrain-mutation-and-pathfinding-lifecycle]] | High |
| Noita scaling required chunked dirty regions and checker-pattern updates. | [[comparables/noita-powder-toy-teardown-rain-world]] | Medium |
| Powder Toy source shows a broad data-first `Element` schema, compact particle state, air/heat/gravity side fields, Lua hooks, and snapshot/delta undo. | [[comparables/the-powder-toy-local-audit]] | High |
| Powder Toy depth comes from hundreds of materials and editor-facing tools; production cost is high if treated as a campaign baseline instead of a material-lab/workbench reference. | [[comparables/research-pass-2-open-source-systems]] | Medium |
| OpenLieroX/Gusanos material flags include actor/projectile passability, flow, breathability, destroyability, light blocking, water behavior, damage, and hookability; nohook rock is a concrete movement-affordance material. | [[comparables/openlierox-local-audit]] | High |
| OpenLieroX terrain carving is mask-based, dirt-only, dirty-region saved, and tied to explosions/beam weapons/rope particles. | [[comparables/openlierox-local-audit]] | High |
| The first schema synthesis separates identity, physical, tool affordance, mobility affordance, hazard, visibility/support, replay/network, and lab/mod extension fields. | [[systems/material-and-mobility-affordance-schema]] | Medium |
| Terrain/material Slice A turns that schema into a concrete eight-material lab with overlay, dirty-region, path, AI, replay, and performance tests. | [[spec/terrain-material-sandbox-slice-a]] | Medium until implemented |
| Teardown shows destruction-as-objective is more important than per-pixel chemistry. | [[comparables/noita-powder-toy-teardown-rain-world]] | Medium |

## Current Recommendation

Recommendation for the **settled launch model**: **C. Cortex-style solids + curated hazards (small launch set)**.

Launch hazard set (proposed):
- Fire (spreads on flammable materials).
- Smoke/gas (vision/health debuff).
- Electric (stuns devices/robots).
- Slippery/wet (movement modifier).
- Hot/cold (gradual damage; structural implications).

Launch affordance columns (proposed):
- Actor passability.
- Projectile passability.
- Dig/drill/carve allowed.
- Anchor/grapple/tether allowed.
- Blocks light/vision.
- Deals contact damage.
- Supports pathfinding cost.
- Produces debris/particles/sound when hit.

Moonshot research (prototype freely; promote to launch via DR if it proves fun and readable):
- Liquids/gases as per-pixel simulation.
- Material chemistry (acids, alkalis, oxidation).
- Plasma/laser melt.
- Full Noita-grade D path (a side-prototype that may be more important than expected — find out, don't preemptively ban).

Why: preserves Cortex identity, supports destruction-objective patterns, keeps AI/path/networking workloads manageable for the launch promise, while keeping the door open for moonshot materials to either ship later or re-shape the launch via a future DR.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| Heavy combat scene (20 actors, 5 explosives) sustains 60 FPS on baseline hardware. | Performance budget. | Pass = > 55 FPS p95. |
| Path-cost recalc finishes within 100ms for the same scene. | AI is not starved. | Pass = under deadline. |
| MAT-T-01..MAT-T-10 terrain material sandbox tests pass. | Minimum material set is readable, replayable, path-aware, and AI-usable. | Pass = [[spec/terrain-material-sandbox-slice-a]] results logged. |
| 5 hazards readable in 1 mission (player identifies each in playtest). | UX coverage. | Pass = > 80% identification. |
| 5 affordance flags readable in material overlay. | Movement/tool rules are learnable. | Pass = players can tell diggable, anchorable, damaging, passable, and blocks-vision material states at a glance. |
| Terrain edits replicate in co-op prototype within 200ms. | Networking compatibility. | Pass = under deadline. |
| Mod adds a custom material; the engine accepts it. | Mod hook works. | Pass = playable; Fail = needs schema fix. |

## Risks

| Risk | Mitigation |
|---|---|
| Hazard count creeps until readability collapses. | Cap *launch* hazards at 5; new ones go through a DR. Mods/sandbox can register more freely. |
| Path recalc starves under heavy edits. | Coalesce dirty regions; force-refresh deadline; fallback "ignore stale costs" mode. |
| Liquids/gases prototype is fun and we underbudget for it. | Treat moonshot prototype results as input to a follow-up DR; don't bolt them onto launch without that DR. |
| Per-pixel modding lures into infinite scope. | Modding manifest requires explicit hazard registration; mods can ship complex chemistry without forcing it onto base. |

## Revisit Trigger

Reopen this decision when:

- Terrain backend prototype is benchmarked at 1280x720 with 20 actors.
- A specific launch hazard fails its readability test.
- Modding ecosystem requests new hazard categories.
- A networking benchmark reveals terrain replication issues.

## Source Trail

- [[systems/physics-and-destruction-models]]
- [[systems/material-and-mobility-affordance-schema]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[engine/terrain-materials]]
- [[engine/projectile-to-impact-lifecycle]]
- [[comparables/noita-powder-toy-teardown-rain-world]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
- [[systems/destruction-objective-mission-patterns]]
