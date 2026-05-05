---
type: comparable-research
status: first-pass-centralized
created: 2026-05-05
topic: noita-grade systemic material simulation
local_repos:
  - comparables_repos/the-powder-toy
  - comparables_repos/barotrauma
  - comparables_repos/ep01-sandsim
  - comparables_repos/gpu-falling-sand-ca
feeds:
  - [[decisions/dr-007-terrain-material-model]]
  - [[spec/full-collision-physics-plan]]
  - [[systems/physics-and-destruction-models]]
  - [[systems/material-and-mobility-affordance-schema]]
---

<- [[comparables/index|comparables index]] | [[systems/physics-and-destruction-models|physics/destruction]] | [[systems/material-and-mobility-affordance-schema|material schema]] | [[decisions/dr-007-terrain-material-model|DR-007]] | [[research-log/moonshot-register|moonshots]]

# Noita-Grade Material Simulation Research

> [!summary] Central finding
> The target feel should be "Noita systemic danger in a Cortex-like combat sandbox", but the implementation should be hybrid: per-pixel active materials where direct combat, terrain, fire, fluids, acid, lava, electricity, and debris matter; room/volume networks where base pressure, oxygen, flooding, fire spread, and submarine-style disasters matter; and editor/debug overlays strong enough that AI agents, players, and modders can inspect why anything happened.

## Why This Exists

The user asked for a focused research blast on material simulation where everything affects everything:

- A tiny pixel of lava can ignite or kill.
- Water near fire becomes mist/steam.
- Toxic gas can asphyxiate.
- Oil and wood burn.
- Acid is neutralized by water.
- Kicked pebbles can damage enemies.
- Electricity flows through liquids and metals.
- Eating world materials can cause sickness and vomit, and vomit can become a usable material.
- Liquids have density and can layer instead of mixing.
- Rare reactions can turn matter into gold.
- Barotrauma-style pressure, flooding, oxygen, and submarine damage should inform bases and vehicles.

This note centralizes research for Noita, The Powder Toy, Barotrauma, Oxygen Not Included, Stationeers, and several open-source falling-sand references.

## Research Method

| Track | What Was Used |
|---|---|
| Search tools | Exa, Tavily, Brave Search, Firecrawl, Perplexity, and web search were used during the research pass. You.com/YDC returned transient server errors during this pass, so it is not counted as source evidence. |
| Local code | Existing `comparables_repos/the-powder-toy` plus newly cloned `comparables_repos/barotrauma`, `comparables_repos/ep01-sandsim`, and `comparables_repos/gpu-falling-sand-ca`. |
| Source posture | Noita/ONI/Stationeers mechanics are web/wiki/reference research. Powder Toy is open-source but GPL. Barotrauma source is public for modding/research but not FOSS. All copied code/assets still need [[references/usage-ledger]] entries before reuse. |
| Scope choice | This file is intentionally standalone for now because roadmap/navigation files are being reviewed by another agent. The roadmap integration plan is included below, but not applied here. |

## Fast Answer For Our Game

| Design Question | Recommendation |
|---|---|
| Should we chase full Noita? | Yes as a long-term headline, but not as an unbounded first milestone. Build a curated material set first and make every reaction inspectable. |
| Which game is the closest reference? | Noita for per-pixel material causality and unfair-but-readable chain reactions. |
| Which game is second most relevant? | Barotrauma for pressure, flooding, oxygen, compartmentalization, power, pumps, vents, fire, and submarine/base disaster grammar. |
| Which open source project is most useful? | The Powder Toy for material schema, air/heat fields, Lua, tools, stamps, saves, and undo. |
| Which management sims matter? | Oxygen Not Included for readable grid overlays and thermal/gas/liquid simplification; Stationeers for pipe/atmosphere networks, sensors, analyzers, phase change, and scriptability. |
| Core technical shape | Hybrid active-region pixel sim + rigid-body/limb physics + room/atmosphere network + replayable event bus + agent-control observation API. |
| Core UX shape | Every hazard needs an overlay, caption, combat-readable signal, AI reason label, and replay event. Hidden chemistry is fun only when players can learn it. |

## Source Coverage

| # | Source | Type | What It Contributes |
|---:|---|---|---|
| 1 | [GDC Vault: Exploring the Tech and Design of Noita](https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design) | Noita technical talk listing | Scaling falling-sand simulation to a large continuous world, integrating rigid-body physics, and designing around a fully destructible world. |
| 2 | [GDC YouTube: Exploring the Tech and Design of Noita](https://www.youtube.com/watch?v=prXuyMCgbTc) | Noita technical talk | Falling Everything engine, sand/liquid/gas rules, 64x64 chunks, particle droplets, Box2D rigid bodies, marching squares, multithreading. |
| 3 | [80 Level: Noita - A Game Based on Falling Sand Simulation](https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation) | Noita technical article | Cellular automata rules, density swaps, fire/water/steam, gas inverse gravity, wood burning, explosions, rigid-body pixel recomputation, dirty chunks. |
| 4 | [Jethro Braindump: GDC Vault Noita notes](https://braindump.jethro.dev/posts/gdc_vault_exploring_the_tech_and_design_of_noita/) | Noita talk notes | Concise summary of sand/liquid/gas sim, marching squares, chunking, dirty rects. |
| 5 | [Rock Paper Shotgun: making a fun game when everything is falling](https://www.rockpapershotgun.com/the-noita-devs-on-how-to-make-a-fun-game-when-everything-is-falling) | Noita design interview | The hard part is robust fun, not just tech; chain reactions need plannable anchors like oil lamps and readable consequences. |
| 6 | [Rock Paper Shotgun: from falling sand to Falling Everything](https://www.rockpapershotgun.com/from-falling-sand-to-falling-everything-the-simulation-games-that-inspired-noita) | Noita inspiration article | Material chain examples: lava, steam, oil needing oxygen, simplified temperature, emergent ecosystem behavior. |
| 7 | [Rock Paper Shotgun: Noita physics roguelite](https://www.rockpapershotgun.com/noita-physics-roguelite) | Noita preview | Liquids, stains, sparks, oil, water, slime, and ecosystem chain reactions as player-facing verbs. |
| 8 | [Eurogamer: Glorious pixel physics rule in Noita](https://www.eurogamer.net/glorious-pixel-physics-rule-in-noita) | Noita preview | Player-readable examples: water pools and extinguishes, acid eats rock, oil burns, stains persist. |
| 9 | [Noita Wiki: Materials](https://noita.wiki.gg/wiki/Materials) | Mechanics reference | Material categories, density, durability, hardness, damage materials, gases, liquids, powders, food, vomit, flasks. |
| 10 | [Noita Wiki: Alchemy](https://noita.wiki.gg/wiki/Alchemy) | Mechanics reference | Reactions in world/flasks/pouches, fire dousing, toxic sludge neutralization, lava-water rock, Midas/gold reactions. |
| 11 | [Noita Wiki: Electricity](https://noita.wiki.gg/wiki/Electricity) | Mechanics reference | Electrification pulses, conductive liquids/metals, breaking flasks, detonating explosives, water + electricity hazards. |
| 12 | [Noita Wiki: Falling Sand Game](https://noita.wiki.gg/wiki/Falling_Sand_Game) | Mechanics/context reference | Positions Noita inside falling-sand design lineage. |
| 13 | [The Powder Toy GitHub](https://github.com/The-Powder-Toy/The-Powder-Toy) | Open source repo | C++/SDL material sandbox with pressure, velocity, heat, gravity, electronics, Lua, saves, stamps, and community sharing. |
| 14 | `comparables_repos/the-powder-toy/src/simulation/Element.h` | Local source | Data-first element schema: movement, air, gravity, heat, flammability, hardness, transitions, callbacks. |
| 15 | `comparables_repos/the-powder-toy/src/simulation/Air.cpp` | Local source | Coarse pressure/velocity/heat field update, wall blocking, pressure-to-velocity coupling. |
| 16 | [[comparables/the-powder-toy-local-audit]] | Vault audit | Existing local audit of Powder Toy material schema, saves, stamps, Lua, undo, and community loops. |
| 17 | [GameEngineering EP01 SandSim](https://github.com/GameEngineering/EP01_SandSim) | Open source repo | Small Noita-inspired C/OpenGL falling-sand tutorial project with sand, water, salt, wood, fire, smoke, steam, oil, lava, acid. |
| 18 | `comparables_repos/ep01-sandsim/source/main.c` | Local source | Simple material enum and update functions for sand, salt, smoke, fire, oil/fire/water interactions. |
| 19 | [GelamiSalami GPU-Falling-Sand-CA](https://github.com/GelamiSalami/GPU-Falling-Sand-CA) | Open source repo | GPU cellular automata using Margolus/block updates to avoid race conditions and directional bias. |
| 20 | [FakeFishGames Barotrauma GitHub](https://github.com/FakeFishGames/Barotrauma) | Public source repo | C# source for submarine hulls, gaps, pumps, oxygen, fire, pressure, AI, afflictions; public source but not FOSS. |
| 21 | [Barotrauma source code announcement](https://barotraumagame.com/uncategorized/662/) | Official announcement | Source is public for modding/research, but licensing is restrictive; use as read-only reference unless separately licensed. |
| 22 | [Barotrauma Modding Guide: Submarine Editor](https://regalis11.github.io/BaroModDoc/Editors/SubmarineEditor.html) | Official modding docs | Hulls/rooms, gaps, water/oxygen separation, pumps, ballast, oxygen generators, vents, airlocks, compartmentalization. |
| 23 | [Barotrauma Modding Guide: StatusEffect](https://regalis11.github.io/BaroModDoc/Misc/StatusEffect.html) | Official modding docs | Status effects can spawn particles, fire, explosions, afflictions, and react to events such as OnFire/InWater/OnDamaged. |
| 24 | [Barotrauma Modding Guide: Character](https://regalis11.github.io/BaroModDoc/ContentTypes/Character.html) | Official modding docs | Character needs air/water, status effects, limb events, death, severing, AI targeting metadata. |
| 25 | [Barotrauma Modding Guide: Afflictions](https://regalis11.github.io/BaroModDoc/ContentTypes/Afflictions.html) | Official modding docs | Affliction prefabs, periodic effects, icons, overlays, pressure immunity and medical readability. |
| 26 | [Barotrauma Wiki: Barotrauma Affliction](https://barotraumagame.com/wiki/Barotrauma_(Affliction)) | Official wiki | High pressure damages unprotected characters; pressure protection is a key equipment gate. |
| 27 | [Barotrauma issue 10409](https://github.com/FakeFishGames/Barotrauma/issues/10409) | Developer discussion | Pressure is intentionally approximate and relative, not real-unit simulation. This is a valuable scope lesson. |
| 28 | `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/Hull.cs` | Local source | Hull water, oxygen, volume, pressure build/drop, wave smoothing, connected-hull search, water flow forces. |
| 29 | `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/Gap.cs` | Local source | Room-to-room/outside water and oxygen exchange, pressure distribution, flow force, update throttling. |
| 30 | `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/FireSource.cs` | Local source | Fire consumes oxygen, grows with hull oxygen, damages characters/items, is extinguished by water. |
| 31 | `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Items/Components/Machines/OxygenGenerator.cs` | Local source | Oxygen generation depends on voltage and condition, then distributes through linked vents by hull volume. |
| 32 | `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Items/Components/Machines/Pump.cs` | Local source | Pumps move water in/out of hulls, auto-control target water level, interact with pressure and power. |
| 33 | [Oxygen Not Included Wiki: Game Mechanics](https://oxygennotincluded.wiki.gg/wiki/Game_Mechanics) | Mechanics reference | Simplified cell simulation, conservation-ish behavior, thermal conductivity, specific heat, pressure abstractions. |
| 34 | [ONI Wiki: Thermal Conductivity](https://oxygennotincluded.wiki.gg/wiki/Thermal_Conductivity) | Mechanics reference | Heat transfer formulas, mass scaling, heat transfer caps, material thermal properties. |
| 35 | [ONI Wiki: Guide/Heat Transfer](https://oxygennotincluded.wiki.gg/wiki/Guide/Heat_Transfer) | Mechanics reference | Multiple tile layers, heat movement between cell/entity/building/pipe, phase change behavior. |
| 36 | [ONI Wiki: Gas](https://oxygennotincluded.wiki.gg/wiki/Gas) | Mechanics reference | Gas expansion, density layering, temperature movement, phase changes, deletion edge cases. |
| 37 | [ONI Wiki: Liquid](https://oxygennotincluded.wiki.gg/wiki/Liquid) | Mechanics reference | Gravity flow, drowning/burning/freezing hazards, one-liquid-per-tile, pressure damage from overmass. |
| 38 | [ONI Wiki: Units](https://oxygennotincluded.wiki.gg/wiki/Units) | Mechanics reference | ONI is approximate physics; gas pressure is represented by mass per tile, no full gas mixtures. |
| 39 | [ONI Wiki: One element per cell rule](https://oxygennotincluded.fandom.com/wiki/One_element_per_cell_rule) | Mechanics/community reference | One main material per tile creates readable simulation and emergent exploits like liquid locks. |
| 40 | [Stationeers Wiki: Atmosphere](https://stationeers-wiki.com/Atmosphere) | Mechanics reference | Atmospheres store moles, composition, volume, temperature, pressure; systems equalize through connections and can move players/objects. |
| 41 | [Stationeers Wiki: Phase Change Mechanics](https://stationeers-wiki.com/Phase_Change_Mechanics) | Mechanics reference | Gases/liquids/solids phase change by pressure and temperature; pipe stress and device safety constraints. |
| 42 | [Stationeers Wiki: Atmospheric Components Quick Reference](https://stationeers-wiki.com/Atmospheric_Components_Quick_Reference) | Mechanics reference | Practical machine list: pipes, valves, regulators, analyzers, mixers, filters, vents, AC, pumps. |
| 43 | [Stationeers Wiki: Gas](https://stationeers-wiki.com/Gas) | Mechanics reference | Gas properties, phase changes, pipe bursting/freezing consequences. |
| 44 | [Stationeers Wiki: Pressure, Volume, Quantity, and Temperature](https://stationeers-wiki.com/Pressure,_Volume,_Quantity,_and_Temperature) | Mechanics reference | Ideal gas law framing for pressure/temperature/moles/volume. |
| 45 | [Stationeers Research Wiki: Physics](https://github.com/Niilo007/Stationeers-Research/wiki/Physics) | Community reverse-engineering | Tick rate, ideal gas formulas, pipe network as a single volume, partial pressure notes. |
| 46 | [Stationeers Wiki: Advanced Furnace](https://stationeers-wiki.com/Kit_(Advanced_Furnace)) | Mechanics reference | Pressure/temp controlled machines, sensor-readable data, safe pressure constraints. |
| 47 | [BooleanCube falling-sand-sim](https://github.com/BooleanCube/falling-sand-sim) | Open source repo | Rust/WASM falling-sand reference for browser-grade material simulation patterns. |
| 48 | [m-camps sand_sim](https://github.com/m-camps/sand_sim) | Open source repo | C++/SDL falling-sand reference with common material movement patterns. |
| 49 | [tranma falling-sand-game](https://github.com/tranma/falling-sand-game) | Open source repo | JavaScript falling-sand reference for small-grid material behavior. |
| 50 | [yuzhoumo Simulake](https://github.com/yuzhoumo/simulake) | Open source repo | Browser particle simulation reference for fluid-looking behavior and interaction scope. |

## Comparative Matrix

| Reference | Simulation Granularity | Strongest System | Readability Pattern | What To Steal | What Not To Copy Blindly |
|---|---|---|---|---|---|
| Noita | Per-pixel materials plus rigid bodies and active chunks | Anything can interact with anything through simple material rules | Repeated cause/effect grammar, strong material colors, flasks, stains, chain reactions | Fire/water/steam/oil/acid/lava/electricity/alchemy as first-class verbs | Unbounded offscreen chaos without controls, especially in a tactical campaign with AI squadmates |
| The Powder Toy | Particle grid plus coarse air/heat/gravity fields | Material schema, editor tools, community saves, Lua | Tools, search, stamps, overlays, element descriptions | Data-first material registry, coarser fields, snapshot/delta undo, scripting hooks | Hundreds of launch materials or GPL code reuse without license plan |
| Barotrauma | Hull/room volumes, gaps, water/oxygen/pressure, item systems | Submarine/base disaster chain: leaks, flooding, pressure, oxygen, fire, pumps, vents, power | Status monitors, pumps, wiring, hull visibility, affliction icons | Room network model for bases/mechs/subs; damageable life support and pressure hazards | Real fluid pressure simulation. Barotrauma itself uses approximations for gameplay. |
| Oxygen Not Included | Tile grid, one main element per cell, thermal/gas/liquid layers | Heat, gas, liquid, pressure, phase change as management UX | Overlays, meters, material labels, predictable tile rules | Simplify hard physics aggressively; provide strong overlays | Full colony-management complexity in a combat-first game |
| Stationeers | Atmosphere/pipe networks with moles, pressure, temp, phase changes | Machine-readable atmospherics and player-built engineering systems | Sensors, analyzers, controllers, IC scripting, pipe devices | Base power/life-support networks that are inspectable and scriptable | Requiring spreadsheet-level engineering for all players |
| EP01 SandSim | Single C file grid CA | Educational material interaction kernel | Direct brush interaction, simple material categories | Minimal starting kernel for sand/water/fire/steam/oil/lava/acid prototypes | Treating tutorial code as production architecture |
| GPU Falling Sand CA | GPU block CA | Parallelism and race avoidance | Visual block update stability | Margolus/block update ideas for GPU experiments | GPU-only sim before determinism, replay, and gameplay needs are known |

## Noita: Target Feel

Noita's key design promise is not just "destructible terrain." It is **material causality**:

- Every visible material has a behavior.
- Hazards come from materials, not only scripted traps.
- Small particles can matter.
- Fluids, gases, fire, electricity, and bodies share the same world.
- Items are containers and conduits for material interactions.
- Players learn through repeated systemic evidence.

### Noita Material Grammar

| Material/State | Behavior To Model | Why It Matters For Us |
|---|---|---|
| Water | Falls, pools, extinguishes, evaporates to steam, conducts electricity in useful contexts, neutralizes toxic sludge/acid-like hazards. | Water becomes a universal tactical tool, not just decoration. |
| Steam/mist | Rises, occupies air space, can condense or signal heat. | Makes heat transitions visible and lets fire/water reactions become battlefield information. |
| Fire | Spreads through flammable materials, consumes burnable pixels, creates heat and smoke. | Fire should be an area denial and terrain-modification system, not just DOT. |
| Oil/fuel | Flows like a liquid, ignites, burns on surfaces, can turn harmless terrain into a trap. | Lets loadouts create trap setups and emergent accidents. |
| Wood | Structural/burnable solid that can be consumed by fire. | Base rooms, crates, doors, bridges, and props can become tactical liabilities. |
| Acid/toxic sludge | Damages materials/actors, reacts with water, creates neutralization opportunities. | Creates "bring counter-materials" equipment choices. |
| Lava | Extreme heat/damage material that converts with water/blood/mud into rock variants. | A single particle can be scary; gives terrain deep risk. |
| Toxic gas | Occupies air, damages/asphyxiates, must be ventilated or avoided. | Connects material sim to base ventilation, masks, AI danger reasoning, and captions. |
| Electricity | Conducts through water/metals, detonates or breaks some objects, damages actors. | Makes wet metal rooms, powered doors, mechs, and cables tactically dangerous. |
| Food/vomit | Ingestible materials can affect actor state, and vomit becomes world material. | Supports grotesque survival comedy and systemic item loops. |
| Alchemic precursor/Midas | Rare recipes can convert matter to gold. | Rare discoverable chemistry supports retention and player stories. |

### Noita Technical Lessons

| Area | Evidence | Lesson |
|---|---|---|
| Core CA | GDC/80.lv describe simple local rules for sand/liquid/gas. | Start with simple movement classes: powder, liquid, gas, static solid, rigid-body pixel, fire/effect. |
| Density | Noita liquids compare density and swap pixels. | Liquids should layer by density where it is player-visible: oil on water, heavy toxic sludge, lava below lighter fluids, gas rising. |
| Fire/water | Fire can turn water into steam; water can extinguish fire. | Treat state transitions as explicit reaction table entries and replay events. |
| Gas | Steam/gases use inverted gravity-like movement. | Gases need ceiling pooling and ventilation paths, not just particle VFX. |
| Rigid bodies | Noita uses rigid-body physics plus pixel-derived shapes. | Terrain/props/limbs/mech armor should share collision/damage channels without forcing every object to be a CA cell. |
| Chunking | Noita uses 64x64 chunks and dirty rectangles. | Active-region budgets and chunk dirty flags are non-negotiable for 4K/120 ambitions. |
| Multithreading | Noita partitions chunks and uses phased updates. | Sim jobs should be deterministic and partition-friendly from the start. |
| Design | RPS interview emphasizes robust fun over raw chaos. | Chain reactions need predictable anchors, counters, and debug explainability. |

### Noita Design Warnings

| Risk | Why It Matters | Mitigation |
|---|---|---|
| Offscreen unfairness | Chain reactions can kill from outside player attention. | Limit high-lethality propagation outside alert zones; provide warning audio/captions/replay reasons. |
| AI confusion | AI squadmates may walk through invisible gas or ignite oil without understanding it. | AI hazard perception must read material fields and explain choices. |
| Performance explosion | Active pixels can grow without bound. | Dirty chunks, material budgets, sleeping rules, LOD, and event throttles. |
| UI overload | Too many hidden materials make the game feel random. | Overlay modes, inspect cursor, hazard captions, material dictionary, replay event causes. |
| Design invisibility | Cool natural sim may go unnoticed. | Make high-effort systems player-facing through missions, tools, and objectives. |

## The Powder Toy: Source-Readable Material Lab

The Powder Toy is the strongest open-source reference for turning a material simulation into a creator-facing product.

### Local Repo Snapshot

| Field | Value |
|---|---|
| Local path | `comparables_repos/the-powder-toy` |
| Local HEAD | `e005c55` |
| License | GPL-3.0 |
| Reuse posture | Study freely; log copied code/assets in [[references/usage-ledger]]. Public code reuse implies GPL compatibility unless separately licensed/reimplemented. |

### Powder Toy Patterns To Reuse

| Pattern | Source Evidence | Adaptation |
|---|---|---|
| Material schema | `Element.h` fields for air, heat, gravity, hardness, burn/explode/melt, transitions, callbacks. | Our material registry should be data-first, inspectable, testable, and usable by editor/UI/AI/network/replay. |
| Coarse fields | `Air.cpp` stores pressure/velocity/ambient heat separately from particles. | Use cell/chunk fields for gas, heat, smoke, pressure, visibility, electricity potential, and AI hazard cost. |
| Particle state | `Particle` stores compact typed values rather than full objects. | Keep pixels compact; put rare behavior in material definitions or chunk metadata. |
| Tools | Tools and brushes are first-class and searchable. | Our material lab, base editor, and scenario editor need stamps, brush tools, undo, search, and test-run. |
| Lua/hooks | Lua can create particles and hook element behavior. | Provide mod scripting, but with capability gates for deterministic/authoritative sessions. |
| Saves/stamps | Saves include particles and environmental fields; stamps can preserve regions. | Shared bunkers, hazard rooms, breach scenarios, and material puzzles should be saveable chunks. |
| Snapshot/delta undo | Editor history uses snapshots and deltas. | Use the same thinking for replay checkpoints, simulation regression tests, and AI run bundles. |

### Powder Toy Lesson

The important lesson is not "ship hundreds of materials." It is: **make the simulation a product surface**. Players and AI agents should be able to inspect, search, stamp, save, undo, replay, and script it.

## Barotrauma: Pressure, Flooding, Oxygen, And Compartment Disasters

Barotrauma matters because our setting has base power, shields, powered doors, turrets, sensors, repair platforms, mechs, command cores, and disaster contracts. A base or mech should not just have HP. It should have systems that fail in readable chains.

### Local Repo Snapshot

| Field | Value |
|---|---|
| Local path | `comparables_repos/barotrauma` |
| Local HEAD | `7446dc1` |
| License posture | Public source for modding/research, but not FOSS. Treat as read-only unless separately licensed. |
| Highest-value code | `Hull.cs`, `Gap.cs`, `FireSource.cs`, `OxygenGenerator.cs`, `Pump.cs`, `Character.cs` |

### Barotrauma Systems

| System | How Barotrauma Models It | Adaptation For Our Game |
|---|---|---|
| Hulls/rooms | Hull objects define volumes, water, oxygen, pressure, fire state, and connected rooms. | Bases, bunkers, ships, caves, mech interiors, and sealed chambers can be room volumes over pixel terrain. |
| Gaps | Gaps connect hulls and allow water/oxygen exchange with open/closed state and flow force. | Breaches, doors, vents, holes, cracked bulkheads, and blasted terrain can become explicit connection edges. |
| Flooding | Water volume changes hull state; pumps push water in/out; flow forces move items. | Flooding should push actors/items, disable gear, affect electricity, extinguish fire, and pressure rooms. |
| Oxygen | Oxygen generators distribute oxygen through vents based on linked hull volume, voltage, and condition. | Base modules need power, condition, linked vents/doors, and visible oxygen maps. |
| Fire | Fire consumes oxygen, grows with oxygen availability, damages characters/items, and water suppresses it. | Fire should interact with ventilation, smoke, sprinklers, water, oxygen, and AI panic/routing. |
| Pressure | Pressure is approximate and relative, not real-unit simulation. | Use pressure as readable game pressure, not strict engineering. Prioritize fun and causality. |
| Afflictions | Pressure, oxygen, wounds, burns, poisons, and other conditions are visible afflictions. | Actor health should expose material-caused effects with captions, icons, and body/equipment damage chains. |
| Power/system condition | Pumps and generators depend on power/condition. | Damageable base modules and mech parts can degrade function gradually instead of binary destroyed/working. |

### Barotrauma Lessons For Bases And Mechs

| Feature | Design Translation |
|---|---|
| Command-core base power | A rooted command core can boost modules, stabilize shields, power turrets, improve sensors, run oxygen/pressure systems, and support AI control bandwidth. |
| Uprooted command-core risk | Moving the core into a body/mech creates a strong avatar but weakens base power, shields, sensors, doors, pumps, repair systems, and automated defenses. |
| Mech internals | Medium/heavy mechs can have internal compartments, cooling, battery/fuel lines, pilot pod, oxygen/cockpit pressure, hydraulic limbs, and breach/fire/flood risks. |
| Disaster contracts | Missions can be generated from room-network failures: stop flooding, rescue crew, restart pumps, vent toxic gas, seal breach, reroute power, fight boarders. |
| AI trust | AI teammates must understand "seal the door", "vent gas", "repair pump", "avoid electrified water", "carry oxygen", and "pull survivor out of pressure". |

## Oxygen Not Included: Readable Grid Thermodynamics

Oxygen Not Included is not an action game, but it is a strong reference for making heat, gas, liquid, and phase changes readable.

### ONI Patterns

| Pattern | Source Evidence | Adaptation |
|---|---|---|
| One main element per cell | ONI's one-element-per-cell rule keeps simulation explainable. | Our room/atmosphere grids can be simplified even if active combat chunks use richer per-pixel materials. |
| Thermal properties | Thermal conductivity, specific heat, mass, and phase changes drive heat transfer. | Material schema should include heat capacity/conductivity and readable phase thresholds. |
| Gas layering | Gases layer/settle by density/molar mass in simplified ways. | Toxic gas, smoke, oxygen, steam, and coolant vapor should move predictably through rooms and caves. |
| Liquid pressure | Excess liquid mass can create pressure and damage tiles. | Flooded rooms, sealed tanks, and pressure traps can damage doors, glass, actors, and weak terrain. |
| Overlays | ONI survives complexity through overlays and meters. | Our material/heat/gas/electricity overlays are not optional. |

### ONI Lesson

Approximation is allowed if it is **consistent and legible**. ONI does not simulate real fluids, but it creates a reliable mental model. Our game should prefer player-learnable laws over physically pure laws.

## Stationeers: Atmospheres, Pipes, Machines, And Scriptability

Stationeers is the strongest reference for base engineering and machine-readable atmospherics.

### Stationeers Patterns

| Pattern | Source Evidence | Adaptation |
|---|---|---|
| Atmosphere networks | Atmospheres store gases/liquids with moles, pressure, volume, temperature, and composition. | Base rooms, pipes, tanks, mech cooling loops, oxygen loops, and hazard systems can share a network model. |
| Pipes/devices | Pumps, valves, filters, regulators, mixers, vents, analyzers, AC, and furnaces operate on networks. | Base equipment should be systemic, repairable, scriptable, and readable by AI and players. |
| Phase change | Pressure/temp can condense or evaporate gases/liquids; wrong pipes fail. | Coolants, steam, fuel vapor, toxic condensate, and cryo hazards can create engineering gameplay. |
| Sensors/analyzers | Players can inspect pressure/temp/composition and automate devices. | Debug overlays, in-game terminals, and AI control APIs should read the same state. |
| Flow force | Pressure differences can move players/objects. | Breaches, decompression, flooding, and high-pressure vents should physically affect bodies/items. |

### Stationeers Lesson

If a system is complex, make it **instrumented and programmable**. That applies to players, modders, and AI agents. The same telemetry that powers tests should power in-game readouts.

## User-Requested Interactions Mapped To Implementation

| Interaction | Best Reference | Implementation Pattern | UI/AI Requirement | Prototype Test |
|---|---|---|---|---|
| Tiny lava pixel ignites/kills | Noita | `contact_damage`, high temperature, heat emission, lava-water transition. | Actor yelps/caption; AI avoids even small lava fields; replay records `hazard_contact`. | Place one lava pixel on boot, limb, wood, water, and armor. Verify different outcomes. |
| Water near fire becomes mist/steam | Noita, ONI | Reaction table: water + heat/fire -> steam/mist; optional condensation. | Steam visible; heat overlay shows cause; AI treats steam as visibility/temperature hazard if needed. | Heat water pool with fire and track mass transition budget. |
| Toxic gas asphyxiates | Noita, Barotrauma, ONI | Gas field with toxicity, oxygen displacement, mask/filter protection. | Gas overlay, caption, AI "holding breath / need filter" reason. | Flood room with gas, measure actor oxygen/poison ticks and AI path avoidance. |
| Oil and wood flammable | Noita, EP01, Powder Toy | Flammable materials with ignition temp, burn rate, oxygen requirement, burn products. | Fire warnings and material inspect text. | Shoot spark into oil/wood room; verify spread and extinguish options. |
| Acid neutralized by water | Noita Alchemy | Reaction table with stoichiometry/priority and resulting neutral liquid/sediment/gas. | Inspect says "water neutralizes"; AI can use water bottle/tool. | Pour water into acid and verify reaction, damage reduction, and byproduct. |
| Pebble kick damages monster | Noita, collision roadmap | Debris rigid body with mass, velocity, sharpness, impact damage, bounce/deflect. | Hit indicator and AI can consider thrown/kicked debris as weapon. | Kick pebble at body/armor/head and compare damage thresholds. |
| Electricity flows through liquids/metals | Noita Electricity, Stationeers | Conductivity field over wet/liquid/metal pixels; pulses/arcs; powered equipment sources. | Electric overlay, audio crackle captions, AI avoids electrified water/metal. | Energize puddle touching metal door and actor; verify conduction and grounding. |
| Eating ground material causes sickness/vomit | Noita materials/food, Barotrauma status effects | Ingestion effects table; vomit material spawn; container/bottle compatibility. | Consumable inspect text; AI can refuse unsafe food unless desperate. | Eat toxic sludge/meat/vomit and verify affliction plus spawned vomit material. |
| Vomit can be bottled and used | Noita | Materials can enter containers; vomit has reactions/uses. | Bottle UI lists material, purity, hazard, known recipes. | Spawn vomit, bottle it, pour on target material, log reaction. |
| Liquids layer by density | Noita, ONI | Density compare/swap in liquid update; immiscible material groups. | Inspect column shows density; visible stratification. | Pour oil/water/toxic sludge/acid in same shaft and verify stable layers. |
| Random mixtures make gold | Noita Alchemy | Rare recipe system with seed-dependent or manifest-defined recipes; high excitement, strict logs. | Discovery UI and replay cause chain. | Mix recipe materials and verify Midas/gold conversion in a bounded lab. |
| Pressure/flooding kills or moves actors | Barotrauma, Stationeers | Hull/room pressure, breach/gap flow, force impulses on actors/items. | Pressure overlay; warning sirens; AI seals doors/evacuates. | Blast hull, flood room, close/open doors, test actor pull and pressure affliction. |
| Fire consumes oxygen | Barotrauma | Fire uses hull oxygen; lower oxygen slows fire or creates smoke/asphyxiation tradeoff. | Fire/oxygen overlay; AI can vent or starve fire. | Burn sealed room and compare fire growth with oxygen generator on/off. |
| Pumps/vents affect survival | Barotrauma, Stationeers | Powered components move water/gas and fail by damage/power. | Status panel; AI repair tasks; command-core boosts. | Damage pump, restore power, verify water/oxygen recovery. |

## Recommended Hybrid Architecture

### Layer Stack

| Layer | Responsibility | Notes |
|---|---|---|
| Active material grid | Per-pixel or sub-tile sand/liquid/gas/fire/acid/lava/electricity in active chunks. | Used near players, projectiles, explosions, fires, breaches, lab tests, and watched rooms. |
| Rigid body physics | Limbs, bodies, armor plates, weapons, pebbles, doors, props, mech parts, projectiles. | Full collision from [[spec/full-collision-physics-plan]] should generate material events and receive material damage. |
| Terrain solidity grid | Diggable terrain, hardness, support, collapse, material ownership. | Bridges pixel sim and collision bodies. |
| Room/volume network | Base/sub/ship/mech interior pressure, oxygen, toxic gas, smoke, flooding, temperature. | Barotrauma/Stationeers style. Room graph uses gaps/doors/vents/breaches. |
| Pipe/power/signal networks | Pumps, vents, generators, batteries, shields, sensors, turrets, doors, repair platforms. | Works with command-core rooted/unrooted tradeoff. |
| Reaction engine | Material pair/triple reactions with priority, temperature, catalysts, containers, byproducts. | Needs deterministic event records and human-readable explanations. |
| Hazard/affliction layer | Fire, poison, pressure, asphyxia, burns, corrosion, sickness, wetness, shock. | Actor/equipment/body-part specific effects. |
| AI perception layer | AI-readable hazard maps, material affordances, task causes, confidence, route costs. | Prevents "humanlike AI" from looking foolish around systemic hazards. |
| Observation/control layer | `cxctl`/JSON-RPC events for material frames, hazard frames, pressure frames, reaction logs. | Lets AI agents test without screenshots. |
| Replay/event layer | Deterministic seed, material events, reaction causes, pressure deltas, collision impulses. | Required for debugging, bug hunts, AI regression, and community replays. |

### Material Schema Fields

| Field | Purpose |
|---|---|
| `id`, `display_name`, `category` | Stable content identity and UI labeling. |
| `movement_class` | Static solid, powder, liquid, gas, fire/effect, rigid-pixel, field-only. |
| `density`, `viscosity`, `surface_tension` | Flow, layering, splash, puddle behavior. |
| `mass_per_pixel` | Collision impulse, pressure, thrown debris, projectile interaction. |
| `hardness`, `durability`, `fracture_behavior` | Digging, bullet penetration, explosions, collapse, damage to tools. |
| `heat_capacity`, `thermal_conductivity`, `temperature` | Heating/cooling, phase change, burn propagation. |
| `ignition_temperature`, `burn_rate`, `oxygen_requirement`, `burn_products` | Fire system and smoke/gas byproducts. |
| `phase_changes` | Water/steam/ice, lava/rock, fuel vapor, toxic condensate. |
| `toxicity`, `asphyxiation`, `corrosiveness`, `radioactivity` | Actor/body/equipment hazard effects. |
| `conductivity`, `grounding`, `arc_threshold` | Electricity through liquids/metals/mechs/base systems. |
| `wetting`, `stain_effects`, `cleaning_reactions` | Noita-style robes/armor wetness, fire resistance, contamination. |
| `ingestion_effects`, `vomit_products` | Eat/sickness/vomit loop. |
| `container_rules` | Bottles, tanks, pouches, canisters, fuel cells, mech reservoirs. |
| `reaction_tags` | Acid, base, water, oil, blood, organic, metal, catalyst, Midas, coolant, explosive. |
| `ai_affordances` | Avoid, seek, use-as-weapon, extinguish-with, neutralize-with, collect, vent, pump. |
| `ui_overlay_color`, `caption_priority` | Readability and accessibility. |
| `performance_tier` | Full active sim, simplified sim, sleeping, or decorative. |
| `network_replay_mode` | Deterministic event, snapshot delta, approximate cosmetic, forbidden in authoritative play. |

## Launch Material Set

Start small enough that AI, UI, replay, and balance can keep up.

| Material | Category | Essential Interactions |
|---|---|---|
| Air/empty | Field | Oxygen/gas occupancy, pressure, smoke, toxic gas, steam. |
| Dirt/sand | Powder/terrain | Diggable, collapses, absorbs liquids, can bury actors/items. |
| Rock/concrete | Solid terrain | Hardness, fracture, projectile impact, lava-water rock byproduct. |
| Metal | Solid/conductor | Conducts electricity, armor, doors, mech parts, sparks. |
| Wood/organic | Solid/flammable | Burns, breaks, fuel, creates smoke/char. |
| Water | Liquid | Extinguishes, conducts, neutralizes acid/toxic sludge, becomes steam. |
| Steam/mist | Gas | Rises, condenses, obscures, indicates heat. |
| Smoke | Gas | Asphyxiation/visibility, fire byproduct. |
| Fire/heat | Effect | Ignites, damages, consumes oxygen, creates smoke/steam. |
| Oil/fuel | Liquid/flammable | Floats/layers, burns, spreads fire, fuels machines. |
| Acid | Liquid/corrosive | Damages terrain, armor, bodies; neutralized by water/base. |
| Toxic sludge/liquid | Liquid/toxic | Poison/contact hazard; water neutralizes or dilutes. |
| Toxic gas | Gas/toxic | Asphyxiation/poison; vents/filters/masks matter. |
| Lava | Liquid/hot | Extreme heat/damage, rock byproducts with water/blood/mud. |
| Blood/vomit | Liquid/organic | Wetness, reactions, sick/gross systems, bottleable material. |
| Electricity charge | Field/effect | Conducts through water/metals, arcs, damages, detonates, disables electronics. |
| Pebble/debris | Rigid body | Kick/throw impact damage, blocks mechanisms, sparks on metal. |

## Research/Expansion Material Set

| Material | Why It Is Interesting | When To Promote |
|---|---|---|
| Slime | Sticky movement, fire suppression, toxic variants. | When mobility/status systems are stable. |
| Brine/saltwater | Density, conductivity, corrosion, freezing/boiling differences. | When liquid density and electricity are proven. |
| Coolant | Mech/base thermal management, leaks, freezing hazards. | When mechs and power systems exist. |
| Cryo liquid/gas | Freezing, brittle armor, fog. | When temperature pipeline is useful in missions. |
| Fuel vapor | Explosive gas, ventilation stakes. | When pressure/ventilation has readable UI. |
| Foam | Fire suppression, sealant, movement slowdown. | When rescue/fire missions need tools. |
| Nanogel/repair fluid | Repair and contamination tradeoffs. | When damageable equipment/mechs are in prototypes. |
| Alchemic precursor | Rare recipe system. | After a material lab and replay cause chain exist. |
| Midas/gold-maker | High-retention discovery and stories. | Late; needs strict containment and balance rules. |
| Biological acid/blood variants | Alien factions/races have unique body fluids. | When origins/races are implemented. |

## Prototype Plan

These are not roadmap edits yet; they are candidate slices to pull into the roadmap when the other agent's review settles.

| ID | Prototype | Core Work | Done When |
|---|---|---|---|
| MAT-01 | Active Material Kernel | Chunk grid, material IDs, dirty rects, deterministic update order, powder/liquid/gas movement. | Sand falls, water pools, steam rises, sleeping chunks stay stable, replay checksum matches. |
| MAT-02 | Reaction Table | Data-driven pair/triple reactions with priority, heat threshold, catalysts, byproducts. | Water+fire -> steam, water+acid -> neutralized, lava+water -> rock, reactions emit cause events. |
| MAT-03 | Fire Package | Fire, heat, smoke, wood, oil, oxygen cost, water extinguish. | Oil trail burns, wood chars/breaks, sealed room consumes oxygen, water/sprinkler suppresses. |
| MAT-04 | Corrosion/Toxic Package | Acid, toxic sludge, toxic gas, water neutralization, armor/body damage. | Acid damages terrain/armor, water neutralizes, toxic gas asphyxiates unmasked actor, AI avoids. |
| MAT-05 | Electricity Package | Conductive materials, wetness, puddle conduction, arcs, grounding, device shock. | Electrified water shocks actor, metal door conducts, grounded path reduces hazard, replay explains chain. |
| MAT-06 | Density/Layering | Liquid densities, immiscible groups, surface behavior. | Oil floats on water, heavy sludge sinks, stable layers persist without jitter. |
| MAT-07 | Debris Impact | Pebbles/rocks as rigid bodies with mass, velocity, impact damage, deflection. | Kicked pebble damages enemy at speed, bounces/deflects at low speed, armor reduces damage. |
| MAT-08 | Ingestion/Vomit/Containers | Eat material, sickness effects, vomit material spawn, bottle/pour loop. | Actor eats unsafe material, becomes sick, vomits, vomit can be bottled and poured. |
| MAT-09 | Hull/Room Network | Room volumes, gaps/doors, water, oxygen, pressure, toxic gas, fire. | Breach floods room, pressure moves items, oxygen changes, doors/gaps alter flow, pump repairs outcome. |
| MAT-10 | Base Equipment Loop | Pumps, vents, oxygen generator, filters, sensors, powered doors, alarms. | Command-core power affects modules; damaged pump/vent has visible failure; AI can repair. |
| MAT-11 | Overlays/Inspection | Material, gas, heat, pressure, electricity, reaction, AI hazard overlays. | Player/AI/test agent can inspect "why did this hurt me?" without screenshots. |
| MAT-12 | AI Hazard Competence | Hazard map, affordance tags, route costs, tactical material use. | AI avoids electrified water, uses water against fire/acid, vents gas, kicks debris opportunistically. |
| MAT-13 | Replay/Determinism | Material event logs, reaction causes, checksums, rollback checkpoints. | Same seed/input run produces same result; bug report bundle identifies reaction chain. |
| MAT-14 | Material Lab | Brush tools, material search, recipe tests, stamps, save/load, human/AI tests. | A designer can build and share a tiny reaction puzzle in minutes. |

## How This Feeds Existing Vault Decisions

| Vault Area | Update Needed Later | Why |
|---|---|---|
| [[decisions/dr-007-terrain-material-model]] | Keep Noita-grade as moonshot, but add "curated material kernel" as active prototype path. | Avoid turning full chemistry into a launch promise while still moving toward it. |
| [[spec/full-collision-physics-plan]] | Ensure material contacts can damage bodies/equipment and collision impulses can spawn material reactions. | Collision without material causality misses the user-requested fantasy. |
| [[systems/material-and-mobility-affordance-schema]] | Add full material schema fields above and AI affordance tags. | AI and UI need the same material semantics. |
| [[spec/prototype-roadmap]] | Add a T-MAT or T-CHEM side track after current review finishes. | Roadmap should include implementation/test slices, not just inspiration. |
| [[spec/ai-control-observability-layer]] | Add material/hazard/pressure/electricity observation frames. | AI agents must test and play without relying on screenshots. |
| [[references/prototype-run-bundle-schema]] | Add material event category if not already present. | Bug hunting needs reaction chains and checksums. |

## Roadmap Integration Recommendation

Add a side track called **T-MAT: Systemic Materials, Chemistry, Atmospheres**.

| Milestone | Scope | Dependencies |
|---|---|---|
| M5.6 Material Kernel | MAT-01, MAT-02, MAT-03, MAT-06, MAT-13 minimal. | Terrain/collision architecture, replay checksums. |
| M5.7 Hazard Package | MAT-04, MAT-05, MAT-07, material damage to armor/limbs/equipment. | Full collision milestone, body damage readability. |
| M7.5 Base Atmospherics | MAT-09, MAT-10, oxygen/flooding/pressure/fire/pumps/vents. | Base power, command-core tradeoff, UI overlays. |
| M8.5 Material Lab | MAT-11, MAT-14, recipe/stamp/test editor. | Editor/workbench, AI control layer. |
| M6.6 AI Material Competence | MAT-12 plus LLM/behavior-tree explanation hooks. | AI architecture, hazard observation frames. |

## Testing Requirements

| Test Type | Required Tests |
|---|---|
| Unit | Material schema validation, reaction priority, density compare, conductivity, phase threshold, damage modifiers. |
| Integration | Fire-water-steam, oil-wood-fire-smoke, acid-water-neutralization, lava-water-rock, electric-water-metal, toxic-gas-asphyxia. |
| Collision/material | Bullet into liquid, pebble into body, limb through fire, armor into acid, mech foot into lava, projectile through steam/smoke. |
| AI | Avoid hazard, use counter-material, repair pump, close breach door, rescue downed actor from gas/flooding, explain action. |
| Replay | Same seed and inputs produce same material checksums and reaction event sequence. |
| Performance | Active chunk budget at 4K/120 target, worst-case fire/liquid/gas stress maps, sleeping chunks, GPU/CPU profile. |
| UX/accessibility | Captions for gas hiss, fire spread, electricity, pressure warning, material reaction; overlays readable in colorblind modes. |
| Scenario | Breach room, burning oil tunnel, acid spill rescue, electrified water trap, lava leak, toxic gas evacuation, Midas lab accident. |

## Risk Register

| Risk | Severity | Why | Mitigation |
|---|---:|---|---|
| Simulation becomes the whole project | High | Noita-grade materials can consume years. | Curated launch set, material lab for experiments, promote only proven fun. |
| 4K/120 performance failure | High | Pixel sim, collisions, AI, and overlays are expensive together. | Dirty chunks, active radius, sleeping, LOD, fixed budgets, profiler milestones. |
| Unfair invisible deaths | High | One lava pixel or toxic gas can feel cheap. | Warnings, readable effects, grace windows, replay cause, AI captions, scenario tuning. |
| AI looks stupid | High | Humanlike AI must handle hazards. | Hazard affordance map, explicit counter-material actions, forced AI regression scenarios. |
| Network/replay nondeterminism | High | Material sim can diverge across platforms. | Deterministic kernels, seeded RNG, checksum frames, event logs, authoritative snapshots where needed. |
| Material explosion in content | Medium | Hundreds of materials dilute readability. | Launch material set plus experimental lab-only set. |
| Chemistry is too hidden | Medium | Players may not discover interactions. | Recipe journal, lab, mission hints, inspect tool, debrief cause chains. |
| Strict realism fights fun | Medium | Stationeers/ONI-style physics can become work. | Use game abstractions and expose them consistently. |
| Licensing contamination | Medium | Barotrauma source is not FOSS; Powder Toy is GPL. | Read-only study unless explicitly licensed/logged; use own implementation. |

## Design Principles

| Principle | Meaning |
|---|---|
| Every material is a verb | If a material exists, it should do something the player, AI, or environment can use. |
| Hidden depth, visible causes | Reactions can be surprising, but the cause chain must be inspectable after the fact. |
| Small particles can matter | Pebbles, droplets, sparks, and gas pockets should have tactical consequences. |
| Counters beat punishments | Fire needs water/foam/venting; acid needs neutralizer; gas needs masks/vents; electricity needs grounding/dryness. |
| AI sees the same game | AI hazard maps and player overlays should share data, so agents feel fair and competent. |
| Approximation is allowed | Noita, ONI, Barotrauma, and Stationeers all simplify. Consistency matters more than real physics. |
| Debuggability is a feature | The material sim must be replayable, inspectable, and controllable by AI agents from early prototypes. |

## Open Questions

| Question | Suggested Next Step |
|---|---|
| CPU, GPU, or hybrid material kernel? | Prototype CPU deterministic kernel first, then GPU CA stress test using GPU-Falling-Sand-CA patterns. |
| How many materials are allowed in active combat? | Start with the launch material set above; measure readability and performance before adding more. |
| How exact should pressure be? | Use Barotrauma-style approximate pressure for gameplay; use Stationeers-like values only where instrumentation benefits. |
| Should chemistry be seed-random or fixed? | Fixed core reactions for learnability; rare alchemy recipes can be seed/scenario-specific with strong logging. |
| How much should actors ingest? | Keep player-facing ingest/vomit systems in lab/comedy/survival contexts first; avoid forcing it into every mission. |
| How does this interact with MMO architecture? | Use server-authoritative material events and bounded active regions; avoid full freeform pixel sim everywhere online. |

## Source Trail

### Noita

- [GDC Vault: Exploring the Tech and Design of Noita](https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design)
- [GDC YouTube: Exploring the Tech and Design of Noita](https://www.youtube.com/watch?v=prXuyMCgbTc)
- [80 Level: Noita - A Game Based on Falling Sand Simulation](https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation)
- [Jethro Braindump: GDC Vault Noita notes](https://braindump.jethro.dev/posts/gdc_vault_exploring_the_tech_and_design_of_noita/)
- [Rock Paper Shotgun: The Noita devs on making a fun game when everything is falling](https://www.rockpapershotgun.com/the-noita-devs-on-how-to-make-a-fun-game-when-everything-is-falling)
- [Rock Paper Shotgun: From falling sand to Falling Everything](https://www.rockpapershotgun.com/from-falling-sand-to-falling-everything-the-simulation-games-that-inspired-noita)
- [Rock Paper Shotgun: Noita physics roguelite](https://www.rockpapershotgun.com/noita-physics-roguelite)
- [Eurogamer: Glorious pixel physics rule in Noita](https://www.eurogamer.net/glorious-pixel-physics-rule-in-noita)
- [Noita Wiki: Materials](https://noita.wiki.gg/wiki/Materials)
- [Noita Wiki: Alchemy](https://noita.wiki.gg/wiki/Alchemy)
- [Noita Wiki: Electricity](https://noita.wiki.gg/wiki/Electricity)
- [Noita Wiki: Falling Sand Game](https://noita.wiki.gg/wiki/Falling_Sand_Game)

### The Powder Toy And Open Falling-Sand Repos

- [The Powder Toy GitHub](https://github.com/The-Powder-Toy/The-Powder-Toy)
- `comparables_repos/the-powder-toy` at `e005c55`
- `comparables_repos/the-powder-toy/src/simulation/Element.h`
- `comparables_repos/the-powder-toy/src/simulation/Air.cpp`
- [[comparables/the-powder-toy-local-audit]]
- [GameEngineering EP01 SandSim](https://github.com/GameEngineering/EP01_SandSim)
- `comparables_repos/ep01-sandsim` at `7fd7d5a`
- [GelamiSalami GPU-Falling-Sand-CA](https://github.com/GelamiSalami/GPU-Falling-Sand-CA)
- `comparables_repos/gpu-falling-sand-ca` at `2a29a4c`
- [BooleanCube falling-sand-sim](https://github.com/BooleanCube/falling-sand-sim)
- [m-camps sand_sim](https://github.com/m-camps/sand_sim)
- [tranma falling-sand-game](https://github.com/tranma/falling-sand-game)
- [yuzhoumo Simulake](https://github.com/yuzhoumo/simulake)

### Barotrauma

- [FakeFishGames Barotrauma GitHub](https://github.com/FakeFishGames/Barotrauma)
- `comparables_repos/barotrauma` at `7446dc1`
- [Barotrauma source code announcement](https://barotraumagame.com/uncategorized/662/)
- [Barotrauma Modding Guide: Submarine Editor](https://regalis11.github.io/BaroModDoc/Editors/SubmarineEditor.html)
- [Barotrauma Modding Guide: StatusEffect](https://regalis11.github.io/BaroModDoc/Misc/StatusEffect.html)
- [Barotrauma Modding Guide: Character](https://regalis11.github.io/BaroModDoc/ContentTypes/Character.html)
- [Barotrauma Modding Guide: Afflictions](https://regalis11.github.io/BaroModDoc/ContentTypes/Afflictions.html)
- [Barotrauma Wiki: Barotrauma affliction](https://barotraumagame.com/wiki/Barotrauma_(Affliction))
- [Barotrauma issue 10409](https://github.com/FakeFishGames/Barotrauma/issues/10409)
- `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/Hull.cs`
- `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/Gap.cs`
- `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Map/FireSource.cs`
- `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Items/Components/Machines/OxygenGenerator.cs`
- `comparables_repos/barotrauma/Barotrauma/BarotraumaShared/SharedSource/Items/Components/Machines/Pump.cs`

### Oxygen Not Included

- [Oxygen Not Included Wiki: Game Mechanics](https://oxygennotincluded.wiki.gg/wiki/Game_Mechanics)
- [ONI Wiki: Thermal Conductivity](https://oxygennotincluded.wiki.gg/wiki/Thermal_Conductivity)
- [ONI Wiki: Guide/Heat Transfer](https://oxygennotincluded.wiki.gg/wiki/Guide/Heat_Transfer)
- [ONI Wiki: Gas](https://oxygennotincluded.wiki.gg/wiki/Gas)
- [ONI Wiki: Liquid](https://oxygennotincluded.wiki.gg/wiki/Liquid)
- [ONI Wiki: Units](https://oxygennotincluded.wiki.gg/wiki/Units)
- [ONI Wiki: One element per cell rule](https://oxygennotincluded.fandom.com/wiki/One_element_per_cell_rule)
- [ONI Wiki: Elements](https://oxygennotincluded.wiki.gg/wiki/Elements)
- [ONI Wiki: Guide/Temperature Management](https://oxygennotincluded.wiki.gg/wiki/Guide/Temperature_Management)

### Stationeers

- [Stationeers Wiki: Atmosphere](https://stationeers-wiki.com/Atmosphere)
- [Stationeers Wiki: Phase Change Mechanics](https://stationeers-wiki.com/Phase_Change_Mechanics)
- [Stationeers Wiki: Atmospheric Components Quick Reference](https://stationeers-wiki.com/Atmospheric_Components_Quick_Reference)
- [Stationeers Wiki: Gas](https://stationeers-wiki.com/Gas)
- [Stationeers Wiki: Phase Change guide](https://stationeers-wiki.com/Phase_Change_guide)
- [Stationeers Wiki: Pressure, Volume, Quantity, and Temperature](https://stationeers-wiki.com/Pressure,_Volume,_Quantity,_and_Temperature)
- [Stationeers Wiki: Temperature independent fuel mixing](https://stationeers-wiki.com/Temperature_independent_fuel_mixing)
- [Stationeers Research Wiki: Physics](https://github.com/Niilo007/Stationeers-Research/wiki/Physics)
- [Stationeers Wiki: Advanced Furnace](https://stationeers-wiki.com/Kit_(Advanced_Furnace))

## Research Verdict

The best path is not pure Noita, pure Barotrauma, pure Oxygen Not Included, or pure Stationeers. The best path is:

1. Noita for the fantasy: every material is dangerous/useful and small particles matter.
2. Powder Toy for the tooling and schema: materials are editable, inspectable, scriptable, saveable, and replayable.
3. Barotrauma for room disasters: pressure, flooding, oxygen, fire, pumps, vents, breaches, and equipment condition.
4. Oxygen Not Included for overlays and simplified thermodynamics that players can reason about.
5. Stationeers for machine-readable atmosphere/power networks and player/AI automation.

For our game, material simulation should become a **combat grammar, base-disaster grammar, AI-competence test, and modding/editor surface**. It should not be a hidden tech demo. The decisive milestone is a small, deterministic, inspectable material lab that can produce replay bundles and AI-control observations before it tries to simulate the whole world.
