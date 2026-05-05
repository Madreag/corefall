---
type: spec
status: planning-anchor-v0
authority: "Direction and contract for command-core base power and mobile/avatar-core tradeoffs. Specific numbers and module catalog remain open."
ready_when: "A prototype can root the command core in a base, power at least three base modules, uproot it, embed it into one body/chassis, and replay the base-power/avatar tradeoff with clear events."
feeds:
  - DR-003
  - DR-008
  - DR-009
  - DR-011
  - DR-015
---

<- [[spec/index|spec section]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-015-player-identity-control-posture|DR-015 player identity]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs]] · [[spec/progression-retention|progression]]

# Command Core, Base Power, And Avatar Uproot

> [!summary] What this page is
> The command core is not only a loss-condition object. It is a physical strategic asset that can power the base while rooted, or be uprooted and embedded into a body/chassis to create a powerful but risky mobile avatar.

## Core Thesis

The command core should create a hard strategic choice:

| State | Player Fantasy | Main Benefit | Main Risk |
|---|---|---|---|
| `rooted_base` | I am the commander inside the fortress. | Powers base modules, shields, sensors, doors, repair platforms, turrets, command relays, and passive/control boosts. | Core location is predictable and must be defended. |
| `portable_core` | I am moving the heart of the base. | Can relocate, escape, or prepare to embed into a body/chassis. | Base drops to reserve power; core is vulnerable in transit. |
| `embedded_avatar` | I put the command core into a unit and personally enter the battlefield. | Body/chassis becomes a stronger core-bearer with better armor/health/energy/equipment output/abilities/control aura. | Base loses core power; losing the avatar can lose the run/campaign. |

This turns the old Cortex brain-in-a-bunker idea into a richer game mechanic: the player can turtle, raid, evacuate, gamble, or make a last stand.

## Rooted Base Power

When rooted in a base, the command core is the primary power/control source for important base systems.

| Base System | Powered Behavior | If Core Is Uprooted / Offline |
|---|---|---|
| Shields | Base shield envelope, local shield doors, directional shield projectors. | Collapse, weaken, or drain reserve batteries. |
| Powered turrets | Automated defense, target sharing, friend/foe filtering, ammo/heat telemetry. | Go offline, switch to dumb local mode, or need manual crew. |
| Sensors | Radar, motion sensors, wall/terrain scans, enemy approach warnings, LZ warnings. | Fog increases; targeting and commander AI confidence drop. |
| Doors / locks | Powered blast doors, smart locks, pressure gates, access routing. | Fail safe by faction/design: lock, open, jam, or require manual tool. |
| Repair platforms | Heal/repair actors, androids, robots, armor, weapons, tools, and mech modules. | Slow down, lose advanced repair, or require portable repair tools. |
| Charging / energy pads | Recharge energy weapons, shields, drones, powered armor, and mech modules. | Recharge rate drops or stops. |
| Command relays | Improve AI coordination, order propagation, squad intent sharing, and tactical overlays. | AI still acts, but loses local boost, shared sensor certainty, and some command bandwidth. |
| Logistics beacons | Improve delivery accuracy, craft landing safety, cargo routing, and emergency extraction. | Delivery risk rises; LZ scoring gets worse. |

## Embedded Avatar Core

The command core can be uprooted and planted into an eligible body, android shell, robot frame, powered armor, or mech. That unit becomes a **core-bearer** or **avatar chassis**.

| Boost Type | Possible Effects | Design Rule |
|---|---|---|
| Durability | More armor, more health, better shock resistance, emergency sealing. | Must be readable as armor/core state, not invisible HP inflation. |
| Mobility | Faster movement, stronger jump/jet, better recovery, higher carry capacity. | Must still respect mass, terrain, recoil, and route-fit tradeoffs. |
| Energy | Larger battery, faster recharge, stronger shields, more ability uptime. | Must expose heat/overload/energy warnings. |
| Equipment output | Higher power budget for heavy weapons, tools, shields, sensors, or repair modules. | Must not make every weapon a universal solution. |
| Abilities | Command pulse, rally, local repair burst, shield flare, emergency extraction beacon, overclock. | Ability costs and cooldowns must be replay/event-visible. |
| Control aura | Stronger command radius, faster AI response, better squad sensor sharing near the avatar. | This is a commander fantasy, not just a personal DPS buff. |

The avatar state should be tempting but dangerous. It can be the right play for a breakthrough, rescue, evacuation, boss fight, or desperate last stand, but it should not be the default answer to every mission.

## Strategic Tradeoff

| Choice | Good When | Bad When |
|---|---|---|
| Keep core rooted | You need shields/turrets/sensors/repair platforms, are defending, or expect a siege. | You need to evacuate, push deep, rescue a valuable actor, or recover from a collapsing base. |
| Uproot core and move it | Base is compromised, relocation is possible, or mission asks for core extraction. | Enemy pressure is high, route is unsafe, or base modules are carrying the fight. |
| Embed core into infantry/android | Need a small fast avatar, stealth rescue, tunnel movement, or emergency direct intervention. | Open-field heavy combat or base defense depends on core power. |
| Embed core into powered armor/mech | Need a heavy breakthrough, shielded rescue, boss duel, or last-stand power spike. | Terrain is tight, delivery route is unsafe, or losing base shields is fatal. |

## UX Requirements

| Surface | Must Show |
|---|---|
| Base power panel | Core state, available power, reserve power, powered/offline modules, shield/sensor/turret/repair status. |
| Core action prompt | Root, uproot, carry, embed, extract, repair, shield, emergency eject. |
| Avatar HUD | Core integrity, avatar bonuses, energy/heat, base-offline warning, extraction route. |
| Tactical map | Base power radius, command relay coverage, sensor coverage, powered doors, turret arcs, shield coverage. |
| Squad panel | Which actors are boosted by relay/avatar aura and which are outside command support. |
| Replay/debrief | When the core moved, what base systems went dark, what avatar boosts were active, and whether the gamble paid off. |

## AI Contract

AI must understand command-core state instead of treating it as a passive objective.

| AI Behavior | Required Reason Label |
|---|---|
| Defend core room | "command core rooted; shield grid depends on it" |
| Repair powered module | "turret offline; core power available; mechanic in range" |
| Escort portable core | "core uprooted; base reserve power low" |
| Refuse unsafe embed | "avatar chassis damaged; core loss risk too high" |
| Push with avatar | "core embedded; shield burst ready; objective window open" |
| Retreat avatar | "core integrity critical; base power offline" |

Enemy AI should also understand the core: it can raid power modules, breach shield generators, bait an avatar deployment, or cut off extraction.

## Replay And Event Contract

| Event | Payload Highlights |
|---|---|
| `command_core_state_changed` | old_state, new_state, actor_or_base_id, cause_event_id, reason_label. |
| `base_power_changed` | available_power, reserve_power, lost_modules, restored_modules, cause_event_id. |
| `base_module_power_changed` | module_id, module_type, old_state, new_state, power_draw, reason_label. |
| `core_embedded` / `core_extracted` | core_id, actor_id/chassis_id, valid/invalid reason, time_to_complete. |
| `avatar_boost_changed` | actor_id, boost_type, old_value, new_value, source_core_id. |
| `core_damaged` | core_id, damage_type, integrity_remaining, shield_state, cause_event_id. |
| `base_reserve_depleted` | base_id, reserve_remaining, systems_failed. |

## Progression Hooks

| Object | Fields To Track |
|---|---|
| `command_core_record` | core id, origin/flavor, integrity, upgrades, scars, rooted/embedded history, near-loss events. |
| `base_power_grid` | generator modules, reserve batteries, shield emitters, turret links, sensor relays, repair pads, door controllers. |
| `avatar_chassis_history` | which chassis held the core, mission outcomes, damage, abilities used, extraction result. |
| `base_module_record` | installed module, power draw, condition, repair history, mod provenance, tactical role. |

## Prototype Acceptance Tests

| ID | Test | Pass Criteria |
|---|---|---|
| CORE-A-01 | Rooted base power | Rooted command core powers shield, turret, and repair platform; UI shows all three. |
| CORE-A-02 | Uproot tradeoff | Uprooting the core visibly weakens/offlines base modules and emits replay events. |
| CORE-A-03 | Embedded avatar | Core embeds into one body/chassis and changes armor/energy/mobility or ability output with clear HUD labels. |
| CORE-A-04 | Last-resort decision | A scenario makes embedding the core tactically tempting but risky; replay explains the result. |
| CORE-A-05 | AI understands core state | Friendly AI defends, escorts, retreats, or repairs based on core/base power state with reason labels. |
| CORE-A-06 | Enemy pressure | Enemy commander targets a powered module or core route and emits a reason. |

## Open Questions

| Question | Next Evidence |
|---|---|
| Does the core physically move as an item, carried actor, vehicle cargo, or special chassis? | CORE-A prototype. |
| Should base modules have independent generators or only reserve batteries when the core leaves? | Base-power lab. |
| Is avatar embedding instant, channelled, or equipment/tool-dependent? | UX and balance prototype. |
| Can the command core split into lesser relays, or is it always singular? | Later campaign/progression DR. |
| What happens in co-op if multiple players have command cores? | DR-005 follow-up once co-op is real. |

## Source Trail

- [[decisions/dr-015-player-identity-control-posture]]
- [[game/player-loop-and-ux]]
- [[spec/authoritative-game-spec-v0]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/progression-retention]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
