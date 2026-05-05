# Physics And Destruction Models

> [!tip] New code pass
> For the material edit and pathfinding invalidation trace, see [[engine/terrain-mutation-and-pathfinding-lifecycle]]. For projectile penetration into terrain, see [[engine/projectile-to-impact-lifecycle]]. For comparable material/tool fields, see [[comparables/the-powder-toy-local-audit]] and [[comparables/openlierox-local-audit]].

> [!important] Current implementation direction
> Full collision is now a P0 direction in [[decisions/dr-033-full-collision-physics-direction]] and the implementation-facing contract lives in [[spec/full-collision-physics-plan]]. Use that plan for collision classes, projectile-projectile rules, CCD tiers, impulse-to-damage routing, `collision` events, T-PHYS, and M5.5 COLL-001..COLL-012. Systemic material simulation (active-region CA + reaction table + atmospheres + affordance/affliction layer) is now a parallel P0 direction in [[decisions/dr-036-systemic-material-simulation-direction]]; implementation lands in T-MAT + M5.6/M5.7/M6.6/M7.5/M8.5 with `material.*`/`reaction.*`/`atmosphere.*`/`affliction.*` events. See [[comparables/noita-grade-material-simulation-research]] for the 50-source synthesis.

## Cortex Command Baseline

Cortex Command's simulation is built around a hybrid of pixel terrain, material properties, movable objects, and atom-based collision. It is not a pure falling-sand game, not a rigid-body-only game, and not a tilemap shooter. The special feel comes from these systems overlapping:

- Terrain pixels have material identity.
- Materials expose gameplay properties such as structural integrity, priority, and piling behavior.
- Projectiles and objects query whether they can penetrate terrain.
- Terrain can be erased by silhouettes, blasts, digging, and penetration.
- Dislodged terrain can become movable material particles.
- Actors are composed of movable objects, limbs, wounds, held devices, and gib definitions.
- Blast impulses and wound thresholds can destroy actors into authored gibs.

Local source trail:

- `Source/Entities/Material.h` defines terrain material properties such as priority and piling.
- `Data/Base.rte/Materials.ini` documents content-side material properties including `StructuralIntegrity` and `Priority`.
- `Source/Entities/SLTerrain.h` and `.cpp` expose `EraseSilhouette`.
- `Source/Managers/SceneMan.cpp` contains penetration, terrain dislodging, and terrain particle spawning logic.
- `Source/Entities/Atom.h`, `AtomGroup.h`, and `AtomGroup.cpp` drive atom travel and limb/body collision.
- `Source/Entities/MOSRotating.cpp` handles wounds, impulse/wound limits, gibbing, and gib creation.

## Terrain And Material Rules

The material model is important because terrain is not just "solid or empty." Material definitions can carry:

| Property | Design Meaning | Player-Facing Effect |
|---|---|---|
| Structural integrity | Resistance to penetration/digging/damage. | Concrete resists bullets and digging better than dirt. |
| Priority | Layering/replacement behavior when terrain materials overlap. | Stronger materials can preserve bunker walls or affect settlement. |
| Piling | Whether loose material accumulates or behaves as fill. | Soil and rubble can rebuild, clog, or reshape spaces. |
| Color/index | Rendering and material lookup identity. | Visual terrain feedback maps to mechanical properties. |

For a future game, the material model should become a design surface, not just an engine table. Each material should have a compact, inspectable profile:

| Material Axis | Possible Values | Design Use |
|---|---|---|
| Hardness | soil, packed dirt, rock, metal, shielded alloy | Weapon penetration, digging tool tier, cover value. |
| Cohesion | loose, packed, brittle, elastic | Collapse behavior, debris generation, tunneling risk. |
| Hazard | inert, hot, toxic, electric, corrosive, explosive | Environmental tactics and AI avoidance. |
| Flow | static, granular, liquid, gas | Noita/Powder Toy style material interactions. |
| Repairability | natural fill, foam, concrete, deployable panels | Base-building and battlefield restoration. |
| Tool affordances | diggable, drillable, anchorable, nohook, blast-resistant | OpenLieroX shows `can_hook`, passability, destroyability, and damage flags as gameplay-critical material data. |
| Visibility/light | opaque, translucent, smoke-blocking, light-blocking | Helps command overlays, stealth, target clarity, and AI line-of-sight. |

## Penetration, Digging, And Terrain Change

Cortex's terrain change loop can be understood as:

1. A moving object, projectile, actor tool, or explosion tests material along a path or area.
2. The engine compares projectile/object force against material integrity.
3. If penetration succeeds, terrain pixels are removed or transformed.
4. Some pixels can dislodge into movable particles.
5. Scene pathfinding and gameplay state eventually need to reflect the changed terrain.

This is a strong foundation for gameplay because it makes terrain a resource. A player can:

- Dig toward gold.
- Breach into a bunker.
- Build or reinforce a defensive route.
- Destroy cover under an enemy.
- Open a tunnel that helps allies but also exposes the brain.
- Create falling debris or loose obstacles.

The weakness is readability. In many Cortex-like moments, the player sees "stuff exploded" but not why a weapon succeeded or failed. Our game should make terrain state visible through optional overlays:

| Overlay | Shows | Why It Helps |
|---|---|---|
| Integrity | Remaining resistance/hardness. | Players choose proper digging or breaching tools. |
| Stability | Collapse, loose fill, unsupported spans. | Makes destruction tactical, not random. |
| Pathability | AI and actor path costs. | Explains why bots refuse or choose routes. |
| Hazard | Fire, gas, electricity, acid, vacuum, toxic dust. | Supports material-system depth. |
| Ownership/build | Claimed, fortified, damaged, rebuildable. | Supports bunker defense and strategic construction. |

## Actor Physics And Body Collision

Cortex actors use a body-part and atom-based approach instead of simple hitboxes. This gives:

- Tangible limbs and devices.
- Impacts that can throw, rotate, and destabilize actors.
- Wounds attached to body parts.
- Gibbing into authored pieces.
- Held-device interactions that feel physical.

The design challenge is agency. Physical bodies are interesting when they create near-misses, knockdowns, limp escapes, falling dropships, and desperate crawls. They are frustrating when the player cannot tell whether they are stunned, blocked, clipped, or dead.

Future design should separate physical simulation from control state:

| State | Simulation | UX Feedback |
|---|---|---|
| Standing | Full control, animation/physics blended. | Stable reticle and stance indicator. |
| Braced | Lower movement, higher recoil control. | Footing icon or weapon-ready pose. |
| Knocked | Physics-driven body, reduced control. | Clear recovery timer and ragdoll highlight. |
| Crippled | Limb or wound penalties. | Body-part damage indicator. |
| Dead/gibbed | No control, pieces persist according to budget. | Fast state confirmation and camera behavior. |

## Wounds, Gibs, And Blast Damage

The engine's wound/gib model is one of Cortex Command's signature systems. Entities can have wound counts, impulse thresholds, blast strength, and authored gib pieces. The result is not generic hitpoint subtraction. Damage becomes visual and physical.

Recommended future damage channels:

| Channel | Use | Expected Feedback |
|---|---|---|
| Piercing | Bullets, shrapnel, needles. | Entry wounds, armor penetration, bleeding/pressure leak. |
| Blunt | Falls, collisions, debris. | Knockdown, stun, limb damage. |
| Explosive | Grenades, rockets, craft crashes. | Blast impulse, terrain crater, gib risk. |
| Thermal | fire, lasers, lava, plasma. | Burning, heat haze, material ignition. |
| Chemical | acid, toxins, corrosive gas. | Armor degradation, area denial. |
| Electric | shock, EMP, powered traps. | Stun, device malfunction, robot weakness. |

The important design decision: wounds should matter before death. A player should see a soldier limping, losing aim, dropping a weapon, struggling under inventory weight, or needing a medikit. This turns damage into stories rather than a hidden health number.

## Comparable Destruction Models

### Noita

Noita is the strongest reference for per-pixel material simulation as spectacle and systemic gameplay. Its developers describe a falling-sand/cellular-automata model where pixels follow simple local rules. Liquids fall and spread, denser materials can displace lighter ones, fire and gases follow probabilistic rules, and rigid-body chunks can split when their pixels are destroyed.

Research takeaways:

- Per-pixel simulation is compelling when materials interact, not just when terrain disappears.
- Update performance depends on chunking, dirty regions, and careful update ordering.
- World generation must constrain chaos with authored macro-structure.
- Spell/wand design turns simulation into repeatable progression and experimentation.

For our game:

- Use Cortex-style physical actors and strategic brain stakes, but borrow Noita's material vocabulary for hazards and chain reactions.
- Do not copy "everything is simulated everywhere." Use simulation budgets and sleep/dirty-chunk rules.

### The Powder Toy

The Powder Toy is useful as a pure material-interaction reference. It simulates pressure, velocity, heat, gravity, electronics, and many material reactions. It shows the long-tail power of a sandbox material palette and Lua extensibility.

For our game:

- Treat it as a material laboratory reference, not a combat design template.
- Prototype material reactions in isolated test scenes before integrating them into missions.
- Use modding hooks for material experiments, but keep campaign materials curated.

### Teardown

Teardown's voxel destruction is less similar technically, but extremely relevant as design proof. Dennis Gustafsson and Tuxedo Labs made destruction central to the objective structure. Since players could break direct routes through walls, the game shifted toward planning heists where destruction is a route-building tool.

For our game:

- If bunker walls can be breached, objectives must assume breach.
- Use planning, alarms, reinforcements, evacuation, extraction, and route design rather than fixed hallway defense.
- Let tools be generous enough to enable creativity, while objectives create pressure.

### Liero / OpenLiero / OpenLieroX

Liero-like games prove that small destructible arenas plus a large weapon roster can support enormous replayability. Their destruction is simpler than Cortex's strategic layer, but the moment-to-moment loop is clear:

- Learn movement.
- Learn weapons.
- Shape the arena.
- Create sudden reversals.

The OpenLieroX local audit adds concrete source-level lessons:

| Source Pattern | Future Design Use |
|---|---|
| Mask-based dirt-only `CarveHole` with dirty-region save. | Start terrain mutation as semantic events plus dirty rectangles; keep authored hole/drill/beam masks readable. |
| Explosion combines dirt color sampling, particles, terrain carve, item/bonus consequences, camera shake, and actor damage. | One explosion event should fan out into replayable child consequences. |
| Gusanos materials distinguish passability, flow, destroyability, damage, blocks-light, and hookability. | Material schema should include movement/tool affordances, not only hardness/integrity. |
| Ninja rope checks material hookability and subdivides high-speed hook motion. | Grapple/tether/jet mobility needs material validity, collision precision, and visible failure reasons. |
| Projectile actions can carve, bounce, explode, spawn children, alter speed/radius, or home. | The mod workbench needs effect graphs and runtime budgets so chaos remains debuggable. |

For our game:

- Add small-arena challenge modes for fast combat testing.
- Treat rope/jetpack/terrain movement mastery as a retention pillar.
- Keep weapons mechanically distinct by how they change space.

### Soldat / OpenSoldat

Soldat is more polygon-map shooter than terrain destruction sandbox, but it is directly relevant for 2D action feel, networked multiplayer, bots, and tooling. Its "gostek" body physics, bullet mechanics, sparks, weapons, server/client split, cvars, and editor ecosystem are all worth studying.

For our game:

- Borrow the clarity of weapon roles and fast combat readability.
- Study OpenSoldat's refactor away from old networking dependencies.
- Use map/editor/launcher/lobby architecture as a community tooling reference.

## Recommended Physics Architecture For A Future Game

| Layer | Responsibility | Determinism Requirement | Notes |
|---|---|---|---|
| Terrain grid | Material id, integrity, temperature/hazard flags, ownership/build state. | High if multiplayer/replay. | Chunked and dirty-region updated. |
| Terrain events | Erase, fill, fracture, burn, freeze, dissolve, reinforce, carve mask, beam cut, drill path. | High. | Events should be serializable for saves/network and replay child consequences. |
| Movable objects | Actors, devices, craft, gibs, debris. | Medium to high. | Server authority likely required for online. |
| Particle effects | Sparks, smoke, dust, blood, small debris. | Low. | Mostly cosmetic; budget aggressively. |
| AI navigation | Path cost field, local steering, dig/rebuild plans, anchor/tether affordance checks. | Medium. | Must update after terrain changes and explain invalid material/tool choices. |
| UI overlays | Integrity/path/hazard/damage intent. | Low. | Derived from sim; can be client-side. |

## Open Questions

- Should terrain support liquids/gases at launch, or should launch focus on granular solids plus authored hazards?
- Should body physics be deterministic enough for multiplayer, or should online focus on co-op with server authority?
- How many materials can be active before readability collapses?
- Should digging tools create clean tunnels, noisy rubble, or both depending material/tool?
- Should structural collapse be simulated, faked, or limited to local support checks?
