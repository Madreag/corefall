---
type: spec-research-plan
status: closed-direction
created: 2026-05-05
topic: full collision physics and physical consequence
ready_when: "T-PHYS exists in the roadmap, M5.5 full-collision gauntlet is in the native backlog, and COLL-001..COLL-012 can be assigned to an implementation agent."
feeds:
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-007
  - DR-008
  - DR-014
  - DR-018
  - DR-021
  - DR-024
  - DR-028
  - DR-033
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/body-damage-model|body damage]] · [[spec/chassis-armor-mechs-and-origins|chassis/mechs/origins]] · [[systems/physics-and-destruction-models|physics/destruction systems]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]]

# Full Collision Physics Plan

> [!summary] Direction
> The game should feel like everything physical matters. Weapons, limbs, bodies, armor, mechs, objects, debris, projectiles, shields, base parts, and terrain all need collision or an explicit, tested reason not to collide. This is a core feel pillar, not a polish item.

> [!warning] Engineering boundary
> "Full collision" does not mean brute-force all-pairs every tick. It means every physical gameplay object has a collision identity, a collision proxy, a material/impulse response, and replay-visible contact events. Performance comes from broadphase partitioning, filters, CCD tiers, contact budgets, and simplified proxies.

## Design Promise

| Promise | Player-Facing Result | Implementation Contract |
|---|---|---|
| Bodies are physical | Actors bump, knock down, pin, trip, crush, and shove each other. | Actor bodies, limbs, armor zones, and held items register contact manifolds and impulse events. |
| Equipment is physical | A rifle can be hit, jammed, knocked away, bent, blocked by a door, or used as a collision object. | Held and dropped equipment has collision proxies, mass, durability, damage stages, and contact reasons. |
| Projectiles are physical where it matters | Bullets hit armor, terrain, shields, weapons, debris, and other bullets. Ricochets and deflections are explainable. | Fast projectiles use swept ray/capsule/shape casts; projectile-projectile pairs are selective but real for non-cosmetic rounds. |
| Terrain is physical | Terrain blocks, carves, crumbles, catches bodies, creates hazards, and changes AI path affordances. | Chunk proxies update from dirty terrain regions; material contacts feed damage, pathing, and replay. |
| Mechs are physical systems | Heavy chassis mass matters. A mech leg can crush infantry, catch on terrain, or take module damage from impact. | Mech parts have separate proxies, mass/inertia tiers, armor layers, and contact-to-damage thresholds. |
| Friendly collision is part of tactics | Allies can block, bump, shield, shove, and accidentally hit each other unless a scenario disables it. | Player/player, ally/ally, enemy/enemy, AI/AI, and mixed limb contacts are in the matrix by default. |
| Collision causes stories | The replay can explain "why did my soldier die?" and "why did that shot miss?" | Every meaningful contact emits `collision.*`, `combat.*`, `body.*`, `terrain.*`, or `equipment.*` events with cause chains. |

## Collision Classes

Every class below needs a stable collision class id, one or more collision proxies, a default collision layer, material response data, and event hooks.

| Class | Examples | Proxy Shape | Notes |
|---|---|---|---|
| `actor_core` | Organic body trunk, android torso, pilot body | capsule/convex compound | Owns high-level status and damage routing. |
| `actor_limb` | Head, upper arm, forearm, hand, thigh, shin, foot | capsule/convex | Limbs collide with bodies, limbs, terrain, weapons, doors, debris, and projectiles. |
| `armor_zone` | Helmet, chest plate, arm plate, shield plate | convex/capsule overlay | Takes impact before body part; can crack, spall, detach, or jam movement. |
| `held_weapon` | Rifle, digger, sword, launcher, shield projector | convex/capsule | Physical while held; may filter self-collision against owner bones but not against world/other actors. |
| `loose_item` | Dropped gun, ammo, battery, medkit, salvage part | convex/circle/AABB | Can be kicked, crushed, blocked, damaged, picked up, or destroyed. |
| `projectile_kinetic` | Bullet, slug, needle, flechette | swept segment/capsule | Collides with bodies, armor, terrain, objects, shields, weapons, and selected projectile classes. |
| `projectile_explosive` | Rocket, grenade, shell, micro-missile | swept capsule/shape | Projectile-projectile hit can detonate, deflect, or damage fuze. |
| `beam_or_trace` | Laser, cutting beam, instant rail trace | segment/shape cast plus event | Gameplay may be instant, but the trace still has collision, material, and replay semantics. |
| `terrain_pixel` | Dirt, concrete, metal, shielded alloy | chunked pixel grid | Authoritative material store; not every pixel is a rigid body. |
| `terrain_proxy` | Collision mesh/outline for dirty chunks | polyline/heightfield/convex decomposition | Generated from terrain chunks for broad/narrowphase contact. |
| `debris_chunk` | Falling rubble, broken armor, gib, crate shard | circle/convex | Budgeted lifetime; may damage by impulse and sharpness. |
| `mech_part` | Leg, foot, arm, cockpit, weapon pod, reactor, shield emitter | compound convex/capsule | Heavy contact can crush, pin, shear, or disable modules. |
| `base_object` | Door, turret, sensor mast, shield gate, repair pad | static/kinematic/dynamic compound | Powered state changes collision and damage response. |
| `force_field` | Shield bubble, door shield, personal shield | sensor + solid proxy | May block projectiles and objects without being a normal rigid body. |
| `sensor_trigger` | Objective volume, pickup radius, repair pad field | sensor only | No impulse, but emits events and must be visibly labeled in debug. |
| `cosmetic_particle` | Spark, dust, smoke fleck | none or sensor-lite | No gameplay collision unless promoted to debris/projectile. |

## Default Collision Matrix

Default rule: **collide unless explicitly exempted**. Exemptions must have a `collision_filter_reason`, tests, and replay visibility.

| Pair | Default | Response |
|---|---:|---|
| Player body <-> AI body | collide | Push, block, knockdown, crush, rescue/carry hooks. |
| Player body <-> enemy body | collide | Push, tackle, crush, melee contact, pin, fall damage. |
| Ally <-> ally | collide | Friendly body blocking is tactical; scenario can soften push but not hide it. |
| Enemy <-> enemy | collide | Enemy crowding and pileups are real; AI must recover and explain path changes. |
| AI <-> AI | collide | Same as unit-unit; doctrine can request spacing. |
| Player limb <-> unit limb | collide | Limb hits, blocks, grabs, crushes, and wound routing. |
| Player limb <-> AI limb | collide | Same as above, regardless of controller. |
| Limb <-> held weapon | collide | Blocks, weapon knock, jam/damage, melee parry. Owner self-collision may be filtered locally. |
| Held weapon <-> held weapon | collide | Parry/block/jam/knockaway; relevant for doors, shields, melee, gun barrels. |
| Loose item <-> body/limb | collide | Kick, trip, crush, pickup obstruction. |
| Loose item <-> projectile | collide | Damage or move item; may block or deflect projectile depending material. |
| Projectile <-> body/limb | collide | Penetration, wound, blunt impulse, armor routing. |
| Projectile <-> armor/equipment | collide | Ricochet, spall, crack, jam, detach, module damage. |
| Projectile <-> projectile | collide selectively | Kinetic rounds deflect/fragment/lose energy; explosive rounds may fuze/detonate/deflect. |
| Projectile <-> terrain | collide | Penetrate, ricochet, crater, embed, spawn debris. |
| Projectile <-> shield/force field | collide | Absorb, reflect, overload, punch through, detonate. |
| Debris <-> body/limb | collide | Blunt/sharp damage, knockdown, pinning. |
| Debris <-> terrain/base | collide | Settle, bounce, pile, block, damage. |
| Mech part <-> infantry | collide | Crush/push/pin; readable danger zone. |
| Mech part <-> terrain/base | collide | Footing, stumbling, scrape damage, structural impact. |
| Base object <-> body/projectile/debris | collide | Doors block bodies and shots; turrets/sensors can be damaged or shielded. |
| Force field <-> body/projectile/debris | collide per field config | Some shields block only projectiles; others block actors too. Rule must be visible. |

## Allowed Exemptions

| Exemption | Why It Can Exist | Required Proof |
|---|---|---|
| Connected self-collision filter | Prevents a forearm from constantly colliding with its own upper arm during normal animation. | Filter is local to connected owner bones and disabled for detached/destroyed parts. |
| Sensor-only volumes | Objectives, detection, repair fields, and pickups should not shove bodies. | Emits `sensor_contact` or equivalent; debug overlay labels it sensor-only. |
| Cosmetic particles | Sparks/smoke/dust cannot all be physical at 4K/120. | Cosmetic class is visually distinct and never used for gameplay damage. |
| Scenario softening | Training missions may demote lethal damage or friendly push. | Scenario policy recorded; replay says what was softened. |
| Perf budget degradation | Extreme particle/debris counts may reduce low-value collisions. | Degradation is deterministic, logged, and never applies to actors, limbs, armor, weapons, key projectiles, terrain, or mission-critical objects. |

## Projectile And Bullet Collision

The rule is not "hitscan everything." The rule is: projectiles have enough physical representation to create fair, inspectable outcomes.

| Projectile Field | Purpose |
|---|---|
| `radius_px` | Swept ray/capsule thickness; enables bullet-bullet and bullet-limb contacts. |
| `mass_kg_like` | Relative impulse and deflection response. |
| `velocity_px_per_s` | Used by CCD, energy, and replay. |
| `kinetic_energy` | Damage/penetration input; derived from mass and velocity unless authored. |
| `armor_penetration` | Material-specific penetration. |
| `restitution` | Bounce/ricochet behavior. |
| `fragmentation` | Whether collision spawns fragments. |
| `explosive_profile` | Fuze/detonation behavior for rockets, grenades, shells. |
| `ccd_class` | Discrete, sweep ray, sweep capsule, sweep shape, or TOI substep. |
| `collides_with_projectiles` | Boolean or class mask; false only for explicit low-value tracer/cosmetic classes. |
| `collision_group` | Filtering for owner-safe arming delay, training rounds, shields, or scenario rules. |

Projectile-projectile policy:

| Contact | Expected Result |
|---|---|
| Kinetic <-> kinetic | Deflect, fragment, tumble, or lose energy based on relative angle, radius, mass, and material. No explosion unless authored. |
| Kinetic <-> explosive | May deflect, damage fuze, prematurely detonate, or destabilize depending explosive profile. |
| Explosive <-> explosive | Detonate, chain, disable fuze, or bounce depending arming state and impact. |
| Beam/trace <-> projectile | If enabled, uses segment/shape intersection and material response; often vaporize/deflect/detonate. |
| Cosmetic tracer <-> anything | No gameplay collision; actual projectile record still collides if it exists. |

## CCD And Contact Tiers

Use the cheapest tier that preserves the promised behavior. Fast/small objects need CCD; slow debris usually does not.

| Tier | Use For | Technique | Notes |
|---|---|---|---|
| `Discrete` | Slow bodies, settled debris, heavy static contacts | normal fixed-tick broad/narrowphase | Cheap but can tunnel. |
| `Speculative` | Rotating limbs, doors, moving platforms, some debris | expanded future AABB/contact prediction | Handles angular motion better, but can ghost collide. |
| `SweepRay` | Tiny high-speed bullets and beams | ray cast or segment TOI | Cheapest projectile CCD; lacks thickness unless radius added. |
| `SweepCapsule` | Physical bullets, slugs, limbs at speed | swept circle/capsule against shapes | Good default for Cortex-like rounds and body parts. |
| `SweepShape` | Rockets, thrown items, weapons, shields | convex shape cast | More expensive, used for larger bodies. |
| `TOISubstep` | Critical high-value contacts | time-of-impact solve + substep | Reserved for player body, pilot, command core, major projectile, mech foot crush. |

## Broadphase And Terrain Proxies

Full collision needs a hybrid broadphase:

| Layer | Role |
|---|---|
| Dynamic tree | Moving bodies, limbs, weapons, projectiles, mechs, debris. |
| Chunk spatial hash | Terrain chunks, base modules, static objects, dense projectile lanes. |
| Dirty chunk proxy builder | Rebuilds collision outlines only for changed terrain regions. |
| Projectile lane cache | Groups high-speed projectiles by swept AABB for bullet-bullet and bullet-world tests. |
| Contact pair cache | Keeps stable pairs warm, reduces churn, and improves replay/debug labels. |
| Budget governor | Caps low-value debris pairs before sim stalls; never drops critical contacts silently. |

Terrain is authoritative as pixels/material cells. Collision uses generated proxies:

- Per-chunk outlines for solid terrain.
- Material tags attached to proxy spans.
- Dirty-region invalidation when terrain is carved, filled, melted, or repaired.
- Optional high-detail sample at the exact contact point when damage/penetration depends on material.
- Chunk-boundary tests so bullets/limbs do not snag or tunnel through seams.

## Contact Event Contract

The run bundle gets a `collision` category so collision is debuggable without watching a video.

| Event | Purpose |
|---|---|
| `collision_pair_created` | New candidate pair entered narrowphase. |
| `collision_contact_started` | First touching contact with classes, ids, materials, normal, and TOI fraction. |
| `collision_contact_persisted` | Contact remained; includes accumulated impulse when relevant. |
| `collision_contact_ended` | Pair separated. |
| `contact_impulse_applied` | Normal/tangent impulse applied; parent links to damage/shove/knockdown. |
| `projectile_deflected` | Ricochet/deflection/tumble/fragment result with reason. |
| `projectile_projectile_contact` | Bullet-bullet or projectile-projectile contact result. |
| `collision_filter_applied` | A pair was skipped; must include `collision_filter_reason`. |
| `collision_damage_applied` | Contact produced body/equipment/chassis/terrain damage. |
| `collision_budget_degraded` | Low-value contacts were culled; includes count, class, and deterministic rule. |
| `collision_first_divergence` | Replay found first contact mismatch. |

Minimum contact payload:

| Field | Meaning |
|---|---|
| `body_a`, `body_b` | Stable entity/proxy ids. |
| `class_a`, `class_b` | Collision class names. |
| `material_a`, `material_b` | Material response names if known. |
| `point_world`, `normal_world` | Contact point and normal. |
| `penetration_depth` | For overlap contacts. |
| `toi_fraction` | For swept contacts. |
| `relative_velocity` | Input to impulse/damage. |
| `normal_impulse`, `tangent_impulse` | Solver result or estimated impulse. |
| `parent_event_id` | Cause chain to shot, movement, explosion, terrain edit, or AI action. |

## Impulse-To-Damage Model

Physics can damage limbs, body armor, equipment, mechs, and terrain. Damage is not only projectile damage.

| Input | Effect |
|---|---|
| Contact impulse | Blunt trauma, knockdown, crush, armor denting, shield overload. |
| Contact sharpness | Cutting/piercing from debris, blades, broken metal, spikes. |
| Relative velocity | Fall damage, vehicle/mech impact, ricochet severity. |
| Contact area | Wide impact bruises/stuns; small impact penetrates/cracks. |
| Material pair | Rubber bounces, metal sparks/ricochets, concrete crushes, flesh wounds. |
| Armor layer | Absorbs, cracks, spalls, transfers blunt force, jams limb movement. |
| Module binding | Contact damage can disable jet, sensor, weapon mount, shield emitter, repair drone. |
| Actor origin | Organic/android/robot/mech bodies translate impulse differently. |

Damage routing:

```text
contact -> impulse/material response -> armor layer -> body part/module -> status/equipment event -> HUD/replay/debrief
```

## Acceptance Suite

These tests become M5.5 and T-PHYS requirements.

| ID | Test | Proves |
|---|---|---|
| COLL-001 | Collision matrix generator loads every physical class and fails if a pair has no rule. | No silent missing collision pairs. |
| COLL-002 | Player, ally, enemy, and AI units shove/block/knock each other in a crowded corridor. | Unit-unit collision exists for all controller/faction combinations. |
| COLL-003 | Limbs collide with limbs, bodies, terrain, and doors; connected self-filter does not hide detached limb collision. | Limb-level physicality. |
| COLL-004 | Held weapons collide with limbs, terrain, doors, and other held weapons; owner self-filter is scoped. | Weapons are physical objects. |
| COLL-005 | Bullets hit body, armor, weapon, dropped item, terrain, shield, and mech module with distinct events. | Projectile-to-everything consequence. |
| COLL-006 | Two bullets cross; kinetic rounds deflect/fragment, explosive rounds can detonate or fuze-fail based on profile. | Projectile-projectile collision and authored exceptions. |
| COLL-007 | High-speed projectile and falling body cross terrain chunk boundaries, tiny holes, and edge contacts without tunneling. | CCD plus terrain proxy seams. |
| COLL-008 | Debris/mech/base-object impact can damage armor, limbs, equipment, and modules by impulse. | Physics-caused damage. |
| COLL-009 | Full Collision Gauntlet replays headlessly with identical contact ids/checksums. | Replay/determinism readiness. |
| COLL-010 | `cxctl observe --collisions` streams current contact pairs, collision filters, and last 30 collision events. | AI/dev eyes-and-ears can inspect physics directly. |
| COLL-011 | Perf run hits target budgets at 1080p/60 and records 4K/120/Deck status. | Performance guardrail. |
| COLL-012 | AI pathing and behavior reacts to body blocking, debris, locked doors, and new terrain contacts with reason labels. | AI does not ignore the physical world. |

## Roadmap Integration

| Roadmap Place | Change |
|---|---|
| T-PHYS side track | Cross-cutting physics/collision track from M0..M12. |
| M1 | Actor controller owns simple ground/body/projectile contacts and first contact events. |
| M2 | Terrain/material chunk proxies and dirty collision updates. |
| M3 | Replay schema reserves `collision` category and contact payload. |
| M5 | Chassis, armor, equipment, and limb proxy ownership. |
| M5.5 | Full Collision Gauntlet: matrix, projectile-projectile, limb/equipment/body/debris/mech/base contacts, CCD, impulse damage, replay/perf proof. |
| M6 | AI harness consumes collision affordances and reason-labels collision-aware choices. |
| M7 | Mission director and base systems rely on doors/shields/turrets/repair pads being real collision objects. |
| M9+ | Headless/server deterministic islands decide which collision pairs are authoritative for multiplayer. |

## Research Synthesis

| Source | Useful Finding | Applied Decision |
|---|---|---|
| [Box2D Collision docs](https://box2d.org/documentation/md_collision.html) | Practical 2D collision primitives, ray/shape casts, TOI, dynamic tree, and manifold concepts. | Use capsules/convex shapes, shape casts, dynamic tree, and contact manifolds as the conceptual baseline. |
| [Box2D Dynamics docs](https://box2d.org/doc_version_2_4/md__e_1_2github_2box2d__24_2docs_2dynamics.html) | Body types, bullet flag, filtering, joints, and contact control. | Require explicit filter reasons and selective CCD for bullet-like bodies. |
| [Erin Catto GDC 2013 Continuous Collision](https://box2d.org/files/ErinCatto_ContinuousCollision_GDC2013.pdf) | Discrete collision tunnels; CCD options trade accuracy/perf; TOI/substepping is expensive. | Use tiered CCD, not universal TOI. |
| [Rapier advanced collision detection](https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/) | Broadphase/narrowphase, contact graphs, intersection graphs, hooks, events. | Model contact/event graph explicitly for replay and debug. |
| [Rapier colliders](https://rapier.rs/docs/user_guides/javascript/colliders) | Sensors, collision/solver groups, active collision types/events, contact forces. | Separate sensor-only from solid contacts and expose solver groups. |
| [Avian Physics GitHub](https://github.com/avianphysics/avian) | ECS-first physics model for Bevy projects. | Keep Bevy/ECS integration modular even if custom physics owns hot paths. |
| [Avian CCD docs](https://idanarye.github.io/bevy-tnua/avian2d/dynamics/ccd/index.html) | CCD/speculative collision concepts in Bevy-facing ecosystem. | Use Bevy-compatible CCD vocabulary for implementation agents. |
| [Unity sweep-based CCD](https://docs.unity3d.com/Manual/sweep-based-ccd.html) | Sweep CCD is accurate for linear high-speed bodies but expensive and angular-limited. | Use sweep ray/capsule/shape selectively. |
| [Unity speculative CCD](https://docs.unity3d.com/2022.3/Documentation/Manual/speculative-ccd.html) | Speculative CCD handles angular motion but can create ghost contacts. | Use speculative contacts for limbs/doors with debug visibility. |
| [Unity choose collision mode](https://docs.unity3d.com/2022.3/Documentation/Manual/choose-collision-detection-mode.html) | Collision modes should vary by body and need profiling. | Add per-class `ccd_class` and perf tests. |
| [Godot RigidBody2D docs](https://docs.godotengine.org/en/stable/classes/class_rigidbody2d.html) | Rigid bodies, contact monitors, and CCD mode are explicit body settings. | Make contact monitoring/event emission explicit on relevant objects. |
| [Godot physics introduction](https://docs.godotengine.org/en/stable/tutorials/physics/physics_introduction.html) | Layers/masks and body types are foundational. | Matrix/layer policy belongs in data and validation. |
| [Godot CollisionShape2D docs](https://docs.godotengine.org/en/stable/tutorials/physics/collision_shapes_2d.html) | Primitive shapes are preferred for dynamic objects; concave shapes are costly/static. | Use simple proxies for limbs/items/mechs; terrain uses chunk proxies. |
| [Panda3D Bullet CCD docs](https://docs.panda3d.org/1.9/python/programming/physics/bullet/ccd) | Swept sphere CCD prevents fast bodies passing through thin geometry. | Use swept spheres/capsules for bullets and small fast objects. |
| [Coumans continuous collision paper](https://www.gamedevs.org/uploads/continuous-collision-detection-and-physics.pdf) | Conservative advancement, ray casts, convex casts, and swept-sphere CCD solve common tunneling cases. | Use hybrid CCD tiers and reserve exact methods for critical objects. |
| [DigitalRune CCD](https://digitalrune.github.io/DigitalRune-Documentation/html/138fc8fe-c536-40e0-af6b-0fb7e8eb9623.htm) | Discrete sampling misses fast objects; bullets often use ray casting; CCD is expensive. | Do not simulate every bullet as heavy rigid body; use swept tests. |
| [DigitalRune motion clamping](https://digitalrune.github.io/DigitalRune-Documentation/html/b52c86be-d31e-4c6f-9892-331af87a0703.htm) | Motion clamping is a practical CCD compromise. | Allow deterministic clamping/degradation for low-value bodies. |
| [dyn4j advanced docs](https://dyn4j.org/pages/advanced.html) | Broadphase, narrowphase, SAT/GJK, CCD, and TOI listeners are a normal staged pipeline. | Roadmap splits broadphase/narrowphase/CCD/contact events. |
| [dyn4j GitHub](https://github.com/dyn4j/dyn4j) | Mature Java 2D collision/physics library with continuous collision. | Confirms 2D full collision can be staged as data + pipeline. |
| [Chipmunk docs](https://chipmunk-physics.net/release/ChipmunkLatest-Docs/) | Collision handlers, groups/categories, sensors, arbiters, impulses, friction, elasticity. | Event callbacks and arbiter-like impulse data become the run-bundle contract. |
| [Jolt Physics GitHub](https://github.com/jrouwe/JoltPhysics) | High-performance collision/rigid-body library with character, ragdoll, sensors, CCD. | Use "full collision with filters and listeners" as normal engine architecture, not a fantasy feature. |
| [Jolt docs](https://jrouwe.github.io/JoltPhysicsDocs/5.0.0/index.html) | Contact listeners, filters, motion quality, and body management are documented engine-level concepts. | Contact filtering and motion quality are first-class data. |
| [GPU Gems 3 broad phase collision](https://developer.nvidia.com/gpugems/gpugems3/part-v-physics-simulation/chapter-32-broad-phase-collision-detection-cuda) | Brute force is O(n^2); broadphase uses sweep/spatial subdivision and can be parallelized. | Full collision must start with broadphase and pair budgets. |
| [GPU Gems physics simulation part](https://developer.nvidia.com/gpugems/gpugems3/part-v-physics-simulation) | Rigid-body pipelines split broadphase, narrowphase, and resolution. | Keep collision pipeline layered and measurable. |
| [Toptal collision detection overview](https://www.toptal.com/game/video-game-physics-part-ii-collision-detection-for-solid-objects) | Broadphase, bounding volumes, GJK/EPA, and CCD as time-of-impact/root finding. | Implementation notes name GJK/EPA/TOI where appropriate, but keep v1 proxies simple. |
| [Newcastle collision detection tutorial PDF](https://research.ncl.ac.uk/game/mastersdegree/gametechnologies/physicstutorials/4collisiondetection/Physics%20-%20Collision%20Detection.pdf) | Tunneling and projectile collision are standard game-physics failure cases. | COLL-007 specifically hunts tiny holes, chunk boundaries, and high-speed tunnels. |
| [Real-Time Collision Detection](https://realtimecollisiondetection.net/books/rtcd/) | Authoritative reference for real-time collision primitives, broadphase, robustness. | Add as recommended technical reference for implementation agents. |
| [Gaffer On Games, Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/) | Fixed/semi-fixed timesteps avoid unstable physics and replay drift. | Keep fixed 60/120 Hz sim islands and deterministic run evidence. |
| [Mirtich impulse-based dynamics thesis](https://people.eecs.berkeley.edu/~jfc/mirtich/thesis/mirtichThesis.pdf) | Impulse/contact modeling can unify collisions and resting/rolling contact. | Contact impulse is the common bridge to damage, knockdown, and replay. |
| [van den Bergen ray casting PDF](http://dtecta.com/papers/jgt04raycast.pdf) | GJK-based ray casting gives earliest contact for convex objects. | Shape/sweep casts should produce TOI fractions for fast convex proxies. |
| [Noita GDC Vault talk](https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design) | Falling-sand worlds can combine pixel simulation and rigid body proxies. | Terrain pixels stay authoritative while proxies serve collision/physics. |
| [Noita 80 Level article](https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation/) | "Every pixel simulated" creates systemic magic but needs careful constraints. | We borrow material systemic depth, not unbounded every-pixel rigid bodies. |
| [[comparables/the-powder-toy-local-audit]] | Material flags and particle properties are powerful data surfaces. | Material response data feeds collision and damage. |
| [[comparables/openlierox-local-audit]] | Worm-like games need material passability/damage/hookability flags. | Terrain/contact affordances become AI/UI-readable fields. |

## Implementation-Agent Prompt

```markdown
Goal: Implement M5.5 Full Collision Gauntlet from [[spec/native-implementation-backlog]] and [[spec/full-collision-physics-plan]].

Context:
- Read [[decisions/dr-033-full-collision-physics-direction]].
- Read [[spec/prototype-roadmap]] T-PHYS and M5.5.
- Read [[references/prototype-run-bundle-schema]] for `collision` event requirements.

Hard rules:
- Everything physical collides by default unless a tested `collision_filter_reason` says otherwise.
- Do not brute-force all-pairs; use broadphase, collision layers, proxies, CCD tiers, and budgets.
- Projectiles must collide with bodies, armor, equipment, terrain, shields, and selected projectile classes.
- Kinetic bullet-bullet contacts deflect/fragment/lose energy unless the projectile profile says explosive/fuze behavior.
- Physics impulse can damage limbs, armor, equipment, chassis modules, terrain, and base objects.
- Every meaningful contact must be observable through `cxctl`, replay events, and run bundles.

Done when:
- COLL-001..COLL-012 pass.
- `cargo run -p cx-e2e -- --scenario m5_5_full_collision_gauntlet --suite COLL-001..COLL-012 --write-run-bundle` passes.
- `cargo run -p cx-headless -- replay <m5_5_run_bundle> --verify-checksums` passes.
- Perf report records 1080p/60 pass and 4K/120 + Deck status.
- Vault prototype note contains final audit, bug log, known issues, and links to run bundle.
```
