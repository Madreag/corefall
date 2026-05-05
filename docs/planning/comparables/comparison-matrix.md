# Comparable Game Matrix

## High-Level Comparison

| Game / Project | Physics Model | Destruction Model | AI Model | Networking / Backend | Tooling / Modding | Main Lesson |
|---|---|---|---|---|---|---|
| Cortex Command / CCCP | Pixel terrain plus movable object and atom/body collision. | Terrain erasure, material penetration, wounds/gibs, particles. | Actor AI modes, Lua behavior scripts, pathfinding, activity scripts. | Legacy network code exists; needs audit. | `.rte` data modules, Lua, VS Code extension, mod converter. | Strategic action sandbox where terrain, bodies, economy, and scripts interact. |
| C4 | Similar Cortex lineage. | Similar old engine primitives. | Similar actor/path/activity model. | Stronger visible multiplayer/NAT code trail. | Old-style source/data layout. | Best Cortex-family multiplayer comparison. |
| Soldat/OpenSoldat | Polygon-map 2D shooter, gostek body physics, explicit control state, bullet physics, movement-accuracy weapon feel. | Not Cortex-style terrain destruction; particle/sparks and body violence matter more. | Waypoint + line-of-sight bots write into the same control state as players; useful traits/stuck counters, but too static for mutable terrain. | Modern refactor uses GameNetworkingSockets and custom snapshots/deltas; satellite audit adds launcher/lobby/server-list/deep-link lessons. | Launcher/lobby/content ecosystem; deterministic `.smod` base archive; PolyWorks map editor still pending. | Fast readability, weapon schema, bot debug state, reticle feedback, content-purity packages, backend/frontend UX, and multiplayer caution. |
| Liero/OpenLiero | Worm body and projectile simulation in destructible arenas. | Pixel terrain destruction in small real-time arenas. | Mostly duel/local/arena AI depending fork. | OpenLiero is more faithful/local; OpenLieroX has multiplayer history. | Mods, levels, weapons. | Short sessions plus extreme weapons create replay. |
| OpenLieroX | LX56-style worm physics, rope/tether states, projectile actions, beams, mod-tuned movement. | Mask-based dirt carving, explosion/beam terrain edits, material flags for passability/hookability/damage/flow. | Large heuristic bot stack with target choice, rope/stuck recovery, carving, weapon choice, mode branches. | Legacy packet model plus unfinished NewNet save/restore/checksum effort; useful caution for authority and terrain sync. | Classic game scripts, Gusanos event weapons, Lua bindings, large mod/level/skin pack tree. | Movement mastery plus modded weapon chaos can sustain a game, but unclear assets and client-authority patterns are release risks. See [[comparables/openlierox-local-audit]]. |
| Noita | Falling-sand/cellular automata plus rigid body chunks. | Nearly every pixel can change, burn, flow, collapse, or react. | Enemy AI plus systemic world hazards; not the main research value. | Not primary. | Modding exists, but research here is simulation. | Material interactions create endless surprise if constrained by good world structure. |
| The Powder Toy | Particle/material sandbox with heat, pressure, velocity fields, electronics, gravity, and compact per-particle state. | Material transformation, pressure/heat reactions, destructive elements, and editor operations rather than mission destruction. | Not primary. | Online save/community ecosystem for shared simulations, not live combat. | Lua API, custom elements, stamps, undo snapshots/deltas, community creations. | Use as material-lab, creator-tool, and hazard-overlay inspiration; see [[comparables/the-powder-toy-local-audit]]. |
| Teardown | Voxel volumes, CPU collision, GPU rendering/ray tracing. | Voxel destruction as the core objective mechanic. | Not primary. | Not primary. | Modding/community levels. | If destruction breaks normal level design, make destruction the objective. |
| Rain World | Procedural body animation and physical movement. | Not terrain-destruction focused. | Autonomous ecosystem AI with creatures pursuing own needs. | Not primary. | Community/mod interest, but research here is AI. | AI feels alive when creatures exist beyond the player. |

## Which Comparable To Use For Which Question

| Question | Best Comparable |
|---|---|
| How should live 2D shooter movement feel? | Soldat/OpenSoldat |
| How should reticle feedback expose accuracy and friendly-fire risk? | OpenSoldat local audit |
| How do we make many weapons replayable? | Liero/OpenLiero/OpenLieroX, especially [[comparables/openlierox-local-audit]] |
| How should rope/grapple/tether mastery work? | [[comparables/openlierox-local-audit]] |
| How can pixel materials interact deeply? | Noita, The Powder Toy, especially [[comparables/the-powder-toy-local-audit]] |
| How do objectives work when the player can destroy walls? | Teardown |
| How do we build solo AI that feels alive? | Rain World |
| How should a legacy 2D shooter modernize netcode? | OpenSoldat, especially [[comparables/opensoldat-local-audit]] |
| How should creator tooling support maps, mods, launcher, and backend services? | [[comparables/opensoldat-satellites-local-audit]], PolyWorks, CCCP VS Code extension |

## Research Priority

| Priority | Target | Reason |
|---|---|---|
| Essential | CCCP unified repo | Primary engine/data/content reference. |
| Essential | C4 networking code | Best local multiplayer comparison. |
| Essential | OpenSoldat | Modern open-source 2D shooter architecture and netcode; first local audit complete for core repo. |
| High | Noita talks/articles | Best per-pixel material design reference. |
| High | Rain World AI talks/articles | Best autonomous creature AI reference. |
| High | OpenLieroX | First local audit complete; best open-source comparable for destructible arena movement, rope mastery, weapon action graphs, mod packs, bots, and network-history caution. |
| Medium | OpenLiero | Compact faithful Liero baseline still pending if we need a smaller terrain/weapon reference. |
| High | The Powder Toy | First local audit complete; best open-source source for material schema, air/heat fields, Lua tooling, save/stamp UX, and snapshot-delta undo. |
| Medium | Teardown | Destruction-first objective design. |
| Low | Additional Worms-like repos | Useful after core sources are fully audited. |
