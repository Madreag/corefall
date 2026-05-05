# AI And Bots

> [!tip] New code pass
> For the engine/Lua order flow, see [[engine/ai-order-lifecycle]]. For measurable solo-bot quality gates, see [[systems/ai-trust-test-suite]].

## Cortex Command AI Layers

Cortex AI is not one monolithic bot brain. It is distributed across engine actor state, pathfinding, activity rules, and Lua behavior scripts.

Local source trail:

- `Source/Entities/Actor.h` defines AI modes, AI script hooks, move path methods, alarm points, path update flags, and controller integration.
- `Source/Managers/ActivityMan.cpp` updates active game mode logic.
- `Source/Activities/GAScripted.cpp` calls Lua activity hooks such as `StartActivity`, `UpdateActivity`, and global scripts.
- `Source/System/PathFinder.h` wraps MicroPather for synchronous and asynchronous path calculation.
- `Source/Entities/Scene.cpp` updates pathfinding area costs after terrain/material changes.
- `Data/Base.rte/AI/SharedBehaviors.lua` includes alarm handling, line-of-sight checks, brain searching, patrol behavior, and path updates.
- `Data/Base.rte/AI/PathFinder.lua` shows script-level path commands.
- Activity scripts such as `Siege.lua`, `BunkerBreach.lua`, and `LandingZoneMap.lua` call pathfinding functions in mission context.

## AI Responsibility Map

| Layer | Responsibility | Example Behavior |
|---|---|---|
| Actor engine state | Current AI mode, controller state, path ownership, alarm point, movement capabilities. | Switch from sentry to patrol to brain hunt. |
| Scene/pathfinder | Path cost graph, route calculation, terrain-cost updates. | Recalculate passability after tunnel or crater changes. |
| Lua behavior | Tactical decisions and scriptable routines. | Process alarm events, cast obstacle rays, search for enemy brain. |
| Activity script | Scenario goals, spawns, economy, win/loss rules. | Invasion waves, bunker defense, landing zones. |
| Player command layer | Orders, direct control, actor switching. | Hold this bunker, dig here, defend brain, assault route. |

## Existing Cortex-Style AI Strengths

- AI behavior can be scripted and extended through Lua.
- Scene pathfinding can account for terrain updates instead of assuming static maps.
- Actors can have AI modes instead of only autonomous/free behavior.
- Alarm events and ray checks let AI react to sensed threats.
- Activity scripts can create mission-specific rules.

## Existing Cortex-Style AI Weaknesses To Solve

| Weakness | Player Symptom | Future Fix |
|---|---|---|
| Terrain changes invalidate intent. | Bots stare at walls, fail to dig, or take absurd routes. | Give AI explicit dig/build/path-repair goals, not just path requests. |
| Equipment use is inconsistent. | Bots waste explosives or fail to use specialist tools. | Item files need AI usage metadata and training scenarios. |
| Orders are too coarse. | Player cannot express "hold, mine, retreat, breach, repair, rescue" cleanly. | Command palette and squad roles with visible intent markers. |
| AI lacks self-preservation clarity. | Units stand in blast zones or fire lanes. | Danger fields for explosives, friendly fire, collapse, fire, acid, dropships. |
| Debuggability is low. | Designers cannot easily see why AI made a choice. | AI overlay: current goal, path, blocked reason, threat, target, next fallback. |

## Target AI Architecture

Use a layered model:

| Layer | Purpose | Implementation Direction |
|---|---|---|
| Reflex | Very short reactions. | Dodge explosive, brace, stop walking into lava, avoid muzzle obstruction. |
| Tactic | Current fight. | Pick cover, fire, reload, retreat, throw grenade, suppress, flank. |
| Navigation | Movement through changing terrain. | Pathfinding plus local steering plus dig/build plans. |
| Job | Role behavior. | Miner mines, engineer repairs, medic rescues, guard patrols, scout marks targets. |
| Commander | Squad/side objectives. | Decide assault route, allocate actors, reinforce brain, attack enemy economy. |
| Personality/noise | Variety and believability. | Courage, discipline, aggression, panic, loyalty. |

## Bot Behavior Checklist

For a Cortex-like game to be satisfying solo, bots need to handle:

- Move to a point through broken terrain.
- Decide when to dig a new path.
- Decide when not to dig because it exposes the brain or collapses cover.
- Use jetpacks/jumps safely around pits and dropships.
- Avoid standing beneath incoming craft.
- Avoid firing explosives at nearby allies or walls.
- Pick up useful dropped weapons.
- Retreat when badly wounded.
- Call for medic/repair support.
- Defend a brain without all units clustering on it.
- Attack an enemy brain while maintaining supply routes.
- Mine resources without blocking tactical units.
- Stop pushing through a path that is no longer viable.
- React to sound/alarm events without omniscience.

## Rain World Lessons

Rain World is the best comparable reference for autonomous ecosystem AI. Its creatures are not merely placed enemies waiting for the player. They move, hunt, fight, shelter, and pursue goals in the world. The designers also had to constrain the chaos with progression gates, room attractiveness, geometry tweaks, and guidance.

Design lessons for our game:

- AI should appear to have its own agenda.
- Autonomy must be bounded by fairness.
- If the AI can create unwinnable states, the game needs recovery valves.
- World geometry and objective placement are AI design tools.
- Player-readable intent matters more than raw cleverness.

## Soldat/OpenSoldat Lessons

Soldat and OpenSoldat are useful for combat bots and online shooter structure:

- Fast 2D movement needs responsive target prediction.
- Bots must understand weapon ranges and reload windows.
- Navigation over irregular maps must account for jump/jet movement.
- Server/client architecture pressures AI determinism and bandwidth decisions.
- Cvars and server configs are valuable for tuning bot difficulty.

## AI Debug And Authoring Tools

A future game should include developer and modder AI tools early:

| Tool | Use |
|---|---|
| Path overlay | Shows current path, blocked segment, path cost, and fallback route. |
| Goal stack inspector | Shows bot current goal, parent order, and timeout. |
| Threat heatmap | Visualizes explosive, line-of-fire, hazard, and enemy visibility zones. |
| Equipment affordance viewer | Shows which actors know how to use which tools. |
| Behavior replay | Stores last N decisions for postmortem. |
| Scenario test harness | Runs bots through standard dig/defend/assault/rescue tests. |

## AI Design For Retention

The user's end goal emphasizes great AI so players do not need other humans. That requires more than aim skill. Good solo AI should create:

- Stories: the medic dragged a miner out; the demo unit accidentally opened a new flank; a scout escaped through a crater.
- Rivalry: enemy commanders adapt over a campaign.
- Trust: friendly bots follow orders well enough that the player delegates.
- Surprise: AI finds a route the player did not see.
- Fairness: AI mistakes are legible and recoverable.

## Open Questions

- Should campaign enemies have persistent commanders with learned preferences?
- Should friendly bots use the same AI stack as enemies, or get extra assist logic?
- How much tactical command should be available while directly controlling an actor?
- Should AI be script-first for moddability or behavior-tree/utility-first for tooling?
- Should bot difficulty alter aim/reaction only, or also strategy depth?
