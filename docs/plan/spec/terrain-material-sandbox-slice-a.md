---
type: spec
status: prototype-reqs
ready_when: "Terrain/material sandbox runs MAT-T-01..MAT-T-10 with overlay, dirty-region, path, recorder, and perf outputs."
---

← [[spec/index|spec section]] · [[systems/material-and-mobility-affordance-schema|material schema]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[engine/terrain-mutation-and-pathfinding-lifecycle|terrain mutation lifecycle]] · [[decisions/dr-007-terrain-material-model|DR-007]]

# Terrain Material Sandbox Slice A

> [!summary] Buildable prototype target
> This Slice A turns [[decisions/dr-007-terrain-material-model|DR-007]] and [[systems/material-and-mobility-affordance-schema]] into a small terrain lab. It does **not** lock the final backend. It tests whether Cortex-style solids plus curated hazards can be readable, AI-usable, replayable, and performant before we decide how ambitious the launch terrain model should be.

## Purpose

The sandbox should answer five questions before the future game spec commits to a material model:

| Question | Prototype Signal |
|---|---|
| Can players predict which materials can be dug, shot, blasted, anchored, repaired, crossed, or avoided? | Material overlay and tool feedback pass MAT-T-01..MAT-T-07. |
| Can bots make the same terrain decisions the player can inspect? | AI emits material decision events and blocked-path reasons during MAT-T-08. |
| Can terrain mutation stay debuggable under combat load? | Every carve/fill/hazard event has a cause, dirty rect, material summary, and replay id. |
| Can path invalidation keep up with edit bursts? | Dirty regions coalesce and path refresh metrics stay inside budget during MAT-T-08 and MAT-T-10. |
| Do hazards add tactical value without visual noise? | Hazard tile, repair/fill, and breach tests are legible in play and replay. |

## Scope

| In Scope | Out Of Scope For This Slice |
|---|---|
| One compact terrain lab with three lanes: soft breach, hard breach, hazard/repair lane. | Final campaign terrain generator. |
| Eight minimum materials from [[systems/material-and-mobility-affordance-schema]]. | Full Noita-grade liquids/gases/chemistry as a launch promise. Prototype moonshots separately via [[research-log/moonshot-register]]. |
| Rifle/projectile, digger/drill, grenade/charge, repair/fill tool, optional tether/grapple. | Final weapon roster or full workbench UI. |
| Material overlay modes: integrity, pathability, mobility, hazard, build/repair. | Complete tactical map UX. |
| Semantic terrain events and periodic chunk snapshots for [[spec/replay-recorder-slice-a]]. | Perfect deterministic replay of every cosmetic particle. |
| AI path/material debug labels for [[spec/ai-trust-harness-slice-a]]. | Full commander AI. |

## Inputs

| Input | What It Contributes |
|---|---|
| [[engine/terrain-mutation-and-pathfinding-lifecycle]] | Cortex mutation/path invalidation flow: material bitmap changes, dirty boxes, path refresh. |
| [[systems/material-and-mobility-affordance-schema]] | Candidate launch/lab/mod material fields and overlay contract. |
| [[spec/actor-feel-sandbox-slice-a]] | Control feel, tool, material, HUD, and event hooks that this sandbox should reuse. |
| [[spec/replay-recorder-slice-a]] | Event envelope, snapshot cadence, viewer/export expectations. |
| [[systems/replay-determinism-and-run-evidence]] | Dirty terrain chunk snapshots, checksum/byte-count evidence, and deterministic-island gates for terrain mutation. |
| [[spec/ai-trust-harness-slice-a]] | AI event contract, blocked-path reasons, material decision reports. |
| [[comparables/the-powder-toy-local-audit]] | Data-first materials, side fields, Lua/tooling, snapshot-delta lessons. |
| [[comparables/openlierox-local-audit]] | Hookability, passability, damage material flags, and mask-based carving lessons. |
| Noita public sources | Dirty 64x64 chunk updates, checker-pattern update caution, rigid-body collapse, and "slow phenomena may be invisible" caution. |
| Company of Heroes destruction AI source | Dynamic destruction is an AI/navigation problem, not only a rendering feature. |

## Local CCCP Hook Map

| Hook | Local Path | What The Sandbox Should Observe |
|---|---|---|
| Material fields | `Cortex-Command-Community-Project/Source/Entities/Material.h:63-117`, `Material.cpp:58-90` | Integrity, friction, restitution, stickiness, density, piling, settle/spawn material, scrap, color/texture. `StructuralIntegrity = -1` becomes effectively unbreakable in `Material.cpp:67-70`. |
| Base material values | `Data/Base.rte/Materials.ini` | Air, sand, earth, stone, concrete, metal, door metal, rubble/scrap values. Concrete is integrity 200, metal 400, stone 140, earth rubble 25. |
| Single-pixel material set | `SLTerrain.h:141-168` | `SetMaterialPixel` directly edits the bitmap; `AddUpdatedMaterialArea` is separate. Any future backend should avoid untracked edits. |
| Mask carving | `SLTerrain.cpp:397-481` | `EraseSilhouette` clears material/color pixels, optionally emits dislodged `MOPixel`s, and appends one updated material area box. |
| Projectile penetration | `SceneMan.cpp:544-686` | `TryPenetrate` compares impulse against material integrity, spawns debris on early penetrations, clears pixels to air, handles scrap compaction/orphan cleanup, but does not itself append an updated material area. |
| Door material override | `ADoor.cpp:259-299` | Doors draw/erase material footprints and append dirty boxes, then team path updates temporarily erase friendly doors. |
| Path dirty-region refresh | `Scene.cpp:2384-2426`, `PathFinder.cpp:275-303`, `PathFinder.cpp:562-590` | Pathfinding uses a 100-node update budget, falls back when updated-box count exceeds 1000, skips updates while path requests are active, and fans shared updates into team pathfinders. |

## Minimum Material Set

| Material | Player Test | Required Fields | Local Reference |
|---|---|---|---|
| Air | Empty/passable baseline. | `actorPassable`, `projectilePassable`, zero integrity. | `Materials.ini:38-61` |
| Dirt / earth | Basic digging and bullet chip behavior. | `integrity`, `diggable`, `explosiveCarvable`, `anchorable`, debris material. | Earth around `Materials.ini:213-230` |
| Concrete | Bunker wall; slower breach, high cover. | `integrity`, `explosiveCarvable`, `beamCuttable`, `coverValue`, `blocksLineOfSight`. | `Materials.ini:883-899` |
| Metal | Hard/ricochet/conductive case. | `integrity`, `restitution`, `friction`, `beamCuttable`, `conductive`, high debris density. | `Materials.ini:900-914` |
| Loose sand/rubble | Fill, settling, and low-support behavior. | `piling`, `settleMaterial`, `isScrap`, low `supportValue`, slow path cost. | Sand `Materials.ini:175-192`, earth rubble `Materials.ini:469-482` |
| Nohook rock | Mobility negative case. | `anchorable=false`, `diggable=false`, `coverValue=high`, visible refusal reason. | Stone/bedrock reference `Materials.ini:251-285`; OpenLieroX nohook concept. |
| Hazard tile | One curated hazard proof point. | `damageOnTouch` or `electric`, visible overlay, AI avoidance. | New prototype material; compare DR-007 hazard list. |
| Repair foam / panel | Build/repair proof point. | `repairable`, `supportValue`, `integrity`, ownership/build tag, fill event. | New prototype material; maps to future build/repair loop. |

## Prototype Scene

| Zone | Layout | Expected Behavior |
|---|---|---|
| Lane A: soft breach | Dirt/sand wall, thin ceiling, rubble pocket. | Rifle chips some pixels; digger opens a route; explosion produces dirty rect burst and limited debris. |
| Lane B: hard breach | Concrete wall with metal reinforcement and nohook rock anchor trap. | Player must choose drill/charge; tether refuses nohook surface with a visible reason. |
| Lane C: hazard/repair | Hazard tile blocks a shortcut; damaged bridge/panel can be filled or repaired. | AI avoids hazard unless ordered; repair/fill changes pathability and emits terrain_fill event. |
| Debug strip | Material swatches with labels and break thresholds. | Overlay modes can be tested without combat noise. |

## Event Contract Additions

Use the [[spec/replay-recorder-slice-a]] envelope. Add these event kinds before implementation:

Run artifacts should follow [[references/prototype-run-bundle-schema]] and [[systems/replay-determinism-and-run-evidence]] so MAT-T evidence includes checked event counts, dirty-region events, terrain chunk checksums, snapshot bytes, path refresh events, screenshots, and performance counters.

| Event | Required Payload | Why |
|---|---|---|
| `terrain_material_probe` | actor/tool id, material id, sampled point, overlay mode, result label. | Makes material UI and AI decisions auditable. |
| `terrain_penetration_threshold` | projectile id, material id, impulse, integrity, passed, debris count, dirty rect if any. | Explains why a bullet did or did not carve. |
| `terrain_carve_mask` | cause id, tool id, mask id/hash, position, dirty rect, removed counts by material. | Replays digger/explosion shape without storing every pixel. |
| `terrain_fill_or_repair` | actor/tool id, source material, target material, points/rect, new support/path flags. | Explains build/repair and path changes. |
| `terrain_dirty_region_batch` | frame, source event ids, rects in, rects out, coalesce cost, node budget. | Debugs path update and networking size. |
| `path_material_refresh` | frame, dirty rect count, nodes requested, nodes updated, skipped reason, team id. | Shows stale path risk and AI trust failures. |
| `hazard_contact_or_avoidance` | actor id, hazard material, contact/avoid, damage/stun, AI reason. | Makes hazards readable in replay and AI reports. |
| `anchor_material_result` | actor/tool id, material id, point, success, failure reason. | Tests mobility affordance clarity. |

## Acceptance Tests

| Test | Pass Criteria | Instrumentation |
|---|---|---|
| MAT-T-01 Overlay recognition | Player identifies diggable, hard, hazard, repairable, and nohook surfaces within 2 seconds each. | Overlay mode, material id, player selection time. |
| MAT-T-02 Projectile threshold | Rifle/projectile against dirt, concrete, metal, and nohook rock logs pass/fail with impulse vs integrity. | `terrain_penetration_threshold`, debris count, impact replay bookmark. |
| MAT-T-03 Digger carve | Digger opens a dirt route, emits bounded carve mask, and updates dirty rects. | `terrain_carve_mask`, removed material histogram. |
| MAT-T-04 Explosion burst | Grenade/charge changes terrain with coalesced dirty regions and capped debris. | Dirty rect in/out count, debris cap, frame cost. |
| MAT-T-05 Repair/fill | Repair foam/panel creates cover/pathability and is visible in overlay. | `terrain_fill_or_repair`, path refresh, overlay diff. |
| MAT-T-06 Mobility affordance | Tether/grapple succeeds on anchorable material and fails on nohook material with readable reason. | `anchor_material_result`, HUD refusal label. |
| MAT-T-07 Hazard readability | Actor and AI identify hazard before contact; contact damage/stun is visible and replayable. | `hazard_contact_or_avoidance`, damage event, replay marker. |
| MAT-T-08 Path invalidation | Opening/closing a route updates bot path reasons within 100ms target or logs a stale-path warning. | `path_material_refresh`, AI blocked reason, timer. |
| MAT-T-09 Recorder export | A 2-minute sandbox run exports terrain events plus periodic terrain chunk snapshots. | JSONL replay, chunk snapshot manifest, viewer scrub. |
| MAT-T-10 Performance budget | Burst edit test reports frame cost, event count, dirty bytes, and path update debt. | Perf counters and budget dashboard row. |

## Initial Budgets

| Budget | Target | Failure Meaning |
|---|---|---|
| Dirty rect coalescing | Burst of 5 explosions becomes fewer than 25 path/update rects. | Bitmap/event stream is too chatty for AI/networking. |
| Path refresh | Route-impacting edit produces updated path data or stale-path warning within 100ms target. | Bots cannot be trusted around mutable terrain. |
| Terrain event export | 2-minute run stays small enough to inspect manually and replay in viewer. | Event schema needs compaction/snapshots. |
| Debris | Cosmetic loose pixels capped per carve/explosion. | Spectacle is consuming readability/CPU. |
| Overlay | Material modes never require reading raw numeric values during combat. | UX contract is not player-facing enough. |

## First Tickets

| Ticket | Scope |
|---|---|
| MAT-SAN-001 | Build material fixture file with the eight minimum materials and explicit affordance tags. |
| MAT-SAN-002 | Add material overlay mode switcher: integrity, pathability, mobility, hazard, build/repair. |
| MAT-SAN-003 | Implement projectile threshold event with impulse/integrity/debris logging. |
| MAT-SAN-004 | Implement digger/beam carve mask event and dirty rect coalescing. |
| MAT-SAN-005 | Implement explosion carve burst with debris cap and dirty-region batch metrics. |
| MAT-SAN-006 | Implement fill/repair tool and overlay diff. |
| MAT-SAN-007 | Implement nohook/anchorable material test and refusal HUD label. |
| MAT-SAN-008 | Implement one hazard material with AI avoidance and contact report. |
| MAT-SAN-009 | Connect dirty-region batches to path refresh counters and stale-path warnings. |
| MAT-SAN-010 | Export terrain events and chunk snapshots through the recorder. |

## Design Rules

| Rule | Reason |
|---|---|
| Semantic terrain events first, bitmap snapshots second. | Events explain player/AI outcomes; snapshots are fallback truth. |
| Every material gameplay flag needs overlay feedback. | Hidden simulation creates confusion, not depth. |
| AI reasons must map to player-visible labels. | Bots should not look psychic or stupid for invisible reasons. |
| Hazards start coarse. | Noita/Powder Toy depth is powerful, but hazards must be readable in combat first. |
| Dirty regions are a first-class API. | Cortex shows that material edits without update tracking can leave pathfinding stale. |
| Moonshots stay alive. | If Noita-grade materials prove more fun than curated hazards, promote the result into a follow-up DR instead of suppressing it. |

## Kill Criteria

| Failure | Stop / Revise |
|---|---|
| Players cannot read material/tool outcomes from overlay labels. | Redesign material categories before adding more materials. |
| Dirty-region/event stream is too large to inspect or replay. | Coarsen events, add chunk snapshots, or simplify mutation shapes. |
| Path invalidation regularly starves while bots are active. | Change path refresh scheduling before building more AI on top. |
| Hazards are hard to distinguish under combat motion. | Reduce hazard count or separate them into clearer visual states. |
| Repair/fill does not change tactical decisions. | Cut or delay build/repair until objectives make it matter. |
| Nohook/anchor rules feel arbitrary. | Either improve visual language or do not ship tether/grapple at launch. |

## Decisions Fed

| Decision | Evidence This Slice Produces |
|---|---|
| [[decisions/dr-002-replay-event-architecture]] | Whether semantic terrain events plus snapshots explain terrain outcomes. |
| [[decisions/dr-004-first-playable-slice]] | Whether the first playable material/tool set supports satisfying actor feel. |
| [[decisions/dr-005-multiplayer-posture]] | Approximate terrain event bandwidth and snapshot fallback shape. |
| [[decisions/dr-006-modding-data-model]] | Which material fields are safe for schema/workbench exposure. |
| [[decisions/dr-007-terrain-material-model]] | Whether Cortex-style solids + curated hazards meets readability/perf needs. |
| [[decisions/dr-008-ai-architecture]] | Whether AI can consume the same material contract as the player. |
| [[decisions/dr-009-command-ux-style]] | Whether material overlays are enough for command/path planning. |

## Source Trail

- `../Cortex-Command-Community-Project/Source/Entities/Material.h`
- `../Cortex-Command-Community-Project/Source/Entities/Material.cpp`
- `../Cortex-Command-Community-Project/Data/Base.rte/Materials.ini`
- `../Cortex-Command-Community-Project/Source/Entities/SLTerrain.h`
- `../Cortex-Command-Community-Project/Source/Entities/SLTerrain.cpp`
- `../Cortex-Command-Community-Project/Source/Managers/SceneMan.cpp`
- `../Cortex-Command-Community-Project/Source/Entities/ADoor.cpp`
- `../Cortex-Command-Community-Project/Source/Entities/Scene.cpp`
- `../Cortex-Command-Community-Project/Source/System/PathFinder.cpp`
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[systems/material-and-mobility-affordance-schema]]
- [[references/prototype-run-bundle-schema]]
- [[systems/replay-determinism-and-run-evidence]]
- [[spec/actor-feel-sandbox-slice-a]]
- [[spec/replay-recorder-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
- [[comparables/the-powder-toy-local-audit]]
- [[comparables/openlierox-local-audit]]
- Noita GDC Vault talk: `https://www.gdcvault.com/play/1025695/Exploring-the-Tech-and-Design`
- Noita falling-sand interview: `https://80.lv/articles/noita-a-game-based-on-falling-sand-simulation`
- Company of Heroes destruction AI GDC Vault page: `https://www.gdcvault.com/play/765/Dealing-with-Destruction-AI-From`
