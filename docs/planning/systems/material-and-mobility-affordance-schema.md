← [[systems/index|systems index]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]] · [[systems/physics-and-destruction-models|physics/destruction]] · [[engine/terrain-materials|Cortex terrain materials]] · [[comparables/the-powder-toy-local-audit|Powder Toy audit]] · [[comparables/openlierox-local-audit|OpenLieroX audit]] · [[comparables/noita-grade-material-simulation-research|noita-grade material research]] · [[decisions/dr-007-terrain-material-model|DR-007]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]]

# Material And Mobility Affordance Schema

> [!warning] Proposal, not locked spec
> This is the first synthesis layer from Cortex material code, Powder Toy's material sandbox, and OpenLieroX's destructive arena/material flags. Use it for prototype requirements and spec growth. Do not treat every field as a launch commitment until [[decisions/dr-007-terrain-material-model]] closes. Implementation specifics now live under [[decisions/dr-036-systemic-material-simulation-direction]] (chunked CA + reaction table + atmosphere networks + curated 17-material launch set + AI affordance/affliction layer + material lab).

## Why This Exists

Cortex-style terrain cannot be just "solid or empty." The future game needs material data that explains:

- What weapons and tools do to terrain.
- How actors, ropes/tethers, projectiles, debris, and AI routes interact with terrain.
- Which terrain states are readable in the HUD/material overlay.
- Which events are serializable for replay, saves, and future networking.
- Which fields are safe for modders and which fields should stay in lab/prototype scope.

The goal is a **small launch schema with strong affordances**, plus optional lab/mod fields that can grow without breaking the core game.

## Current Prototype Target

[[spec/terrain-material-sandbox-slice-a]] is the first buildable test for this schema. It reduces the field set to eight materials, five overlay modes, terrain/path/replay events, and MAT-T-01..MAT-T-10 acceptance tests so DR-007 can gather runtime evidence instead of staying theoretical.

## Evidence Inputs

| Source | Evidence | Schema Pressure |
|---|---|---|
| Cortex `Material` | `Index`, `Priority`, `Piling`, `Integrity`, `Restitution`, `Friction`, `Stickiness`, `Density`, `SettleMaterial`, `SpawnMaterial`, `IsScrap`, color/texture. | Keep material identity, penetration resistance, collision response, loose-pixel/debris behavior, and visual identity. |
| Cortex terrain lifecycle | `SLTerrain::EraseSilhouette`, `SceneMan::TryPenetrate`, `DislodgePixel`, dirty material areas, pathfinding refresh budget. | Terrain edits need dirty regions, cause/event records, path invalidation, and AI debug. |
| Powder Toy | Data-first `Element`, compact `Particle`, air/heat/gravity fields, transition rules, Lua hooks, snapshots/deltas. | Add lab/mod-only environment fields carefully; expose them in tools before missions. |
| OpenLieroX | `Material` flags for worm/projectile passability, flow, breathability, destroyability, light blocking, water behavior, damage, hookability; mask-based carving; rope attach rules. | Add material affordances for movement, tools, hooks/tethers, hazards, visibility, and networking. |
| UX brief | Material/path overlay requires integrity, pathability, hazard, ownership/build modes. | Every field that affects play needs a player-facing overlay or feedback path. |
| AI trust suite | Bots need terrain awareness, blocked reasons, dig/breach plans, hazard avoidance, and stuck recovery. | AI must read the same affordance profile players can inspect. |

## Schema Layers

| Layer | Purpose | Launch Status |
|---|---|---|
| Identity | Stable id/index, name, color, textures, tags. | Required. |
| Physical | Integrity, density, friction, restitution, stickiness, priority, piling. | Required; inherited from Cortex. |
| Tool affordance | Diggable, drillable, beam-cuttable, explosive-carvable, repairable, reinforceable. | Required for core tools. |
| Mobility affordance | Actor passability, projectile passability, anchorable/tetherable, climbable, slippery, landing-safe. | Required for actor feel and AI. |
| Hazard | Damage-on-touch, fire, smoke/gas, electric, heat/cold, toxic/corrosive. | Curated launch subset only. |
| Visibility/support | Blocks light, blocks line of sight, cover value, support value/collapse hint. | Required for overlays; support can start approximate. |
| Event/replay/network | Semantic event kind, dirty rect behavior, deterministic payload, snapshot policy. | Required for any mutable material. |
| Lab/mod extension | Heat capacity, pressure interaction, reaction table, flow update, custom Lua callbacks. | Lab/mod-only until proven readable. |

## Proposed Material Record

| Field | Type | Source | Launch? | Used By |
|---|---|---|---|---|
| `id` | string | New schema | Yes | Saves, mods, replay, UI. |
| `paletteIndex` | uint8 or internal id | Cortex | Yes | Terrain grid, legacy import. |
| `displayName` | localized string | New schema | Yes | UI/workbench. |
| `tags` | string list | New schema | Yes | Loadout UI, mod search, AI. |
| `color` | RGB/RGBA | Cortex/OpenLieroX/Powder Toy | Yes | Terrain preview, particles, overlay legend. |
| `fgTexture`, `bgTexture` | asset refs | Cortex | Yes | Rendering/workbench. |
| `integrity` | float | Cortex | Yes | Penetration, digging, breach time. |
| `density` | float | Cortex/Powder Toy | Yes | Debris mass, particles, support. |
| `priority` | int | Cortex | Yes | Settling/layer conflict. |
| `piling` | int | Cortex | Yes | Loose material behavior. |
| `restitution` | float 0..1 | Cortex | Yes | Projectile/body collision response. |
| `friction` | float 0..1 | Cortex/OpenLieroX | Yes | Actor movement, projectile response. |
| `stickiness` | float 0..1 | Cortex | Yes | Stuck projectiles/debris. |
| `spawnMaterial` | material id | Cortex | Yes | Debris/material particles. |
| `settleMaterial` | material id | Cortex | Yes | Loose fill/scrap behavior. |
| `isScrap` | bool | Cortex | Yes | Gib/debris cleanup and compaction. |
| `actorPassable` | bool | OpenLieroX | Yes | Movement/pathing. |
| `projectilePassable` | bool | OpenLieroX | Yes | Bullet/projectile collision. |
| `diggable` | bool or tier | Cortex/OpenLieroX | Yes | Digger tools, AI breach plans. |
| `drillable` | bool or tier | New | Yes | Engineer tools. |
| `beamCuttable` | bool or tier | OpenLieroX beam carving | Yes | Lasers/drills/cutting weapons. |
| `explosiveCarvable` | bool or tier | Cortex/OpenLieroX | Yes | Grenades, rockets, charges. |
| `repairable` | bool or method | New | Yes | Foam/concrete/panels. |
| `anchorable` | bool or tier | OpenLieroX `can_hook` | Yes if mobility tool ships | Rope/tether/grapple/zipline. |
| `climbable` | bool or coefficient | New | Prototype | Actor movement/mobility. |
| `landingSafe` | bool or damage modifier | New | Prototype | Drops, craft, AI landing plans. |
| `slipperiness` | float | Cortex friction + hazard plan | Yes if wet/ice ships | Movement feedback. |
| `damageOnTouch` | amount/channel | OpenLieroX damage material | Curated | Lava/electric/thorns/traps. |
| `blocksLight` | bool | OpenLieroX | Yes | Lighting, fog-of-war, stealth. |
| `blocksLineOfSight` | bool | New | Yes | AI target logic, command overlay. |
| `coverValue` | float | New | Yes | AI and player overlay. |
| `supportValue` | float | New | Prototype | Collapse-lite/support hints. |
| `flammable` | bool/tier | Powder Toy/Noita | Curated | Fire hazard. |
| `conductive` | bool/tier | Powder Toy | Curated | Electric hazard. |
| `toxic` | bool/tier | New/Noita-like | Curated | Gas/acid. |
| `heatCapacity` | float | Powder Toy | Lab/mod | Heat simulation. |
| `airPressureEffect` | struct | Powder Toy | Lab/mod | Pressure/smoke/gas. |
| `flowModel` | enum | Powder Toy/OpenLieroX water | Lab/mod | Sand/liquid/gas. |
| `reactionTable` | list | Powder Toy/Noita | Lab/mod | Chemistry. |
| `luaCallbacks` | capability-declared refs | Powder Toy/OpenLieroX/Gusanos | Mod-only | Custom behavior with validation. |

## Launch vs Lab vs Mod

| Bucket | Allowed Fields | Rule |
|---|---|---|
| Launch core | Identity, physical, terrain mutation, tool affordance, passability, visibility, small hazard subset. | Must be readable in HUD/material overlay and measurable in AI tests. |
| Launch hidden/internal | Dirty rect policy, replay snapshot policy, deterministic payload flags. | Must be stable for saves/replay/network even if not shown to players. |
| Material lab | Heat, pressure, flow, reaction tables, custom spawn/transition experiments. | Can be wild and expressive; label as lab/prototype until playtested. |
| Public mods | Most fields plus capped custom callbacks and capability declarations. | Validator must enforce budgets and show risk labels. |
| Private prototype mods | Anything useful. | Track copied/reused sources in [[references/usage-ledger]] if moved into the future project. |

## UX Overlay Contract

Every player-relevant field needs a visual language. Avoid exposing raw tables in combat unless the player asks.

| Overlay Mode | Shows | Fields |
|---|---|---|
| Integrity | "Can I break this, and with what?" | `integrity`, `diggable`, `drillable`, `beamCuttable`, `explosiveCarvable`. |
| Pathability | "Can I or my bots pass this?" | `actorPassable`, `slipperiness`, `landingSafe`, `pathCost`, door/build modifiers. |
| Mobility | "Can I anchor, climb, land, or tether here?" | `anchorable`, `climbable`, `landingSafe`, `projectilePassable`. |
| Hazard | "Will this hurt or disable me?" | `damageOnTouch`, `flammable`, `conductive`, `toxic`, temperature/electric tags. |
| Visibility/Cover | "Can I see, shoot, or hide through this?" | `blocksLight`, `blocksLineOfSight`, `coverValue`. |
| Build/Repair | "Can I fix or reinforce this?" | `repairable`, `supportValue`, ownership/build state. |

Acceptance test: MAT-01 should expand into material-specific subtasks:

| Test | Pass Criteria |
|---|---|
| MAT-01A Breakability | Player picks the right tool for dirt, concrete, metal, and reinforced wall in under 2 seconds each. |
| MAT-01B Mobility | Player identifies valid vs invalid grapple/tether/landing material in under 2 seconds. |
| MAT-01C Hazard | Player identifies damaging/electric/toxic material before contact. |
| MAT-01D AI Path | Player understands why a bot route is blocked or unsafe from overlay labels. |

## AI Contract

AI should not have secret terrain knowledge that the player cannot inspect. It can compute deeper scores, but the core reasons must map to overlay labels.

| AI Need | Material Fields | Event/Debug Label |
|---|---|---|
| Select breach route | `integrity`, `diggable`, `explosiveCarvable`, `coverValue` | `breach_route_chosen`, `material_resistance`. |
| Avoid hazard | `damageOnTouch`, `flammable`, `toxic`, `conductive` | `hazard_avoided`, `hazard_too_risky`. |
| Use mobility tool | `anchorable`, `climbable`, `actorPassable`, `landingSafe` | `anchor_valid`, `anchor_invalid`, `mobility_refused`. |
| Recover from stuck | `actorPassable`, `diggable`, `drillable`, nearby empty regions | `stuck_recovery_dig`, `stuck_waiting_for_path`. |
| Choose weapon/tool | `beamCuttable`, `explosiveCarvable`, `repairable`, `projectilePassable` | `tool_selected_for_material`. |
| Find cover | `coverValue`, `blocksLineOfSight`, `supportValue` | `cover_selected`, `cover_rejected`. |

## Replay And Networking Contract

Material mutations must be replayable and eventually networkable. The event should explain what happened without replaying every cosmetic particle.

| Event | Minimum Payload | Snapshot Need |
|---|---|---|
| `terrain_carve_mask` | cause id, source actor/item, material ids affected, mask id, position, dirty rect, removed count. | Periodic terrain chunk snapshot. |
| `terrain_penetration` | projectile id, material id, impulse, threshold, result, debris count. | Actor/projectile snapshot around impact. |
| `terrain_fill_or_settle` | source material, settle material, area/points, cause. | Dirty rect snapshot. |
| `hazard_started` | hazard type, material, bounds, source. | Hazard field snapshot if dynamic. |
| `anchor_attached` | actor/tool id, material id, point, success/failure reason. | Actor/mobility state snapshot. |
| `ai_material_decision` | actor id, order id, material id, score/reason. | Debug/replay event only; can be compressed. |

Networking rule: prototype semantic events first. Raw bitmap/chunk deltas are fallback snapshots, not the primary design language.

## First Prototype Requirements

The actor-feel sandbox should include a tiny material set that exercises every critical category without requiring a full material simulation.

| Material | Purpose | Required Fields |
|---|---|---|
| Air | Empty/passable baseline. | `actorPassable`, `projectilePassable`. |
| Dirt | Basic digging/carving material. | `integrity`, `diggable`, `explosiveCarvable`, `anchorable`. |
| Concrete | Hard bunker wall. | `integrity`, `beamCuttable`, `explosiveCarvable`, `coverValue`, `blocksLineOfSight`. |
| Metal | High resistance/ricochet. | `integrity`, `restitution`, `friction`, `beamCuttable`, `conductive`. |
| Loose sand/rubble | Piling/fill behavior. | `piling`, `settleMaterial`, `actorPassable` maybe slow, `supportValue` low. |
| Nohook rock | Mobility affordance negative case. | `anchorable = false`, `diggable = false`, `coverValue` high. |
| Hazard tile | Damage/electric/fire proof point. | one curated hazard flag plus visible overlay. |
| Repair foam/panel | Build/repair test. | `repairable`, `supportValue`, `integrity`, ownership/build tag. |

## Decisions This Feeds

| Decision | How This Helps |
|---|---|
| [[decisions/dr-004-first-playable-slice]] | Defines the minimum material set and mobility affordance tests for slice A. |
| [[decisions/dr-005-multiplayer-posture]] | Defines terrain and anchor events that can be serialized before promising online play. |
| [[decisions/dr-006-modding-data-model]] | Defines schema fields, validation targets, and workbench effect/material previews. |
| [[decisions/dr-007-terrain-material-model]] | Turns "curated hazards first" into concrete field groups and launch/lab/mod boundaries. |
| [[decisions/dr-008-ai-architecture]] | Gives AI readable terrain/tool affordances and debug labels. |
| [[decisions/dr-009-command-ux-style]] | Defines overlay modes that make material decisions visible. |

## Open Questions

| Question | Cheapest Test |
|---|---|
| Is `anchorable` important enough for launch? | Add a simple tether/grapple prototype to slice A; compare player retention/feel with and without it. |
| Is `supportValue` worth implementing early? | Fake collapse/support overlay first; only simulate if players use it tactically. |
| Should hazards be per-pixel, cell-grid, or volume overlays? | Prototype smoke/fire/electric as coarse cells over pixel terrain. |
| How much field complexity can modders handle? | Workbench should show beginner/advanced tabs and generated examples. |
| Which fields must be deterministic? | Replay prototype should record/restore every field that changes combat outcomes. |

## Source Trail

- `../Cortex-Command-Community-Project/Source/Entities/Material.h`
- `../Cortex-Command-Community-Project/Data/Base.rte/Materials.ini`
- `../Cortex-Command-Community-Project/Source/Entities/SLTerrain.cpp`
- `../Cortex-Command-Community-Project/Source/Managers/SceneMan.cpp`
- [[engine/terrain-materials]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[systems/physics-and-destruction-models]]
- [[systems/ux-overlay-screen-brief]]
- [[systems/ai-trust-test-suite]]
- [[systems/replay-event-architecture]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
