---
type: system
status: research-bridge
feeds:
  - DR-002
  - DR-004
  - DR-005
  - DR-007
  - DR-008
---

<- [[systems/index|systems]] · [[systems/replay-event-architecture|replay/event architecture]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[spec/prototype-roadmap|prototype roadmap]]

# Replay Determinism And Run Evidence

> [!summary] Slice A recommendation
> Do not bet the first prototype on pure deterministic replay. Build a hybrid evidence model: input intent, semantic events, actor/inventory snapshots, terrain chunk snapshots, checksums, dropped-event counters, and human notes in a validated run bundle. If a subsystem later proves deterministic, promote it as a deterministic island.

## Why This Matters

The game we want is physics-heavy, terrain-mutating, AI-driven, moddable, and readable under pressure. That combination makes replay and debugging infrastructure product-critical:

| Need | Evidence Requirement |
|---|---|
| Player trust | A death recap must explain input, weapon, projectile, terrain, wound, status, and item fallout. |
| AI trust | Bot choices and refusals need reason labels, rejected alternatives, and outcome events. |
| Equipment balancing | Loadout tests must show what item was selected, why it was safe or unsafe, what it changed, and whether the result matched the role card. |
| Terrain/design tuning | Diggers, explosives, fill tools, and hazards need dirty-region, material, path, and snapshot evidence. |
| Future networking | Event volume, snapshot size, and checksum stability are the cheapest early indicators for co-op/PvP/MMO implementation risk. DR-005/DR-034/DR-035 close the direction; evidence still decides whether scope is promoted, adjusted, or reopened at M9-M12. |
| Modding/workbench | Reused/copied/private experiments are allowed, but package behavior has to be traceable through source ids, package ids, and run evidence. |

## Research Comparison

| Source | Replay Model | Useful Lesson | Risk For This Game |
|---|---|---|---|
| Unreal Replay System | Network replication stream recorded by `DemoNetDriver`; replays can pause, scrub, change speed, and carry metadata/version fields. | Reuse the same authority/event surface for replay and future networking where possible. | Replication-log replay is powerful, but it assumes an engine architecture designed around replicated state. |
| Photon Quantum replay docs | Deterministic replay from input history, deterministic config, runtime config, asset DB, initial frame/tick data, and optional checksums. | If we want deterministic islands, record content hashes, config, input history, and checksums from the start. | Full deterministic replay only works when the simulation, assets, physics, and RNG are controlled tightly. |
| Gaffer deterministic lockstep | Send per-frame inputs, not state; require bit-identical results and per-frame checksums; buffer inputs to hide jitter. | Input traces are worth recording even before networking because they become determinism probes. | Floating-point physics, collision ordering, scripts, and random effects can break bit identity fast. |
| Gaffer snapshot interpolation | Send snapshots with sequence numbers and interpolate between them when determinism is not practical. | Snapshot/state streaming is the fallback for mutable physics, many actors, and non-deterministic effects. | Bandwidth and storage can explode unless snapshots are chunked and semantic. |
| YellowAfterlife deterministic-netcode prep | Build a replay first; centralize input polling; support multiple logic frames per render frame; rollback needs save/load on demand. | Slice A should record input intent and build snapshot restore smoke tests before any multiplayer promise. | Engine-provided physics and scripts may remain non-deterministic even with clean input capture. |
| GameDeveloper replay-system article | Deterministic replays are compact but hard to skip and fragile under physics drift; full-state snapshots are larger but robust. | Use checkpoints and an assert/compare mode to find first divergent tick. | A replay that cannot report the first divergence becomes a time sink. |
| OpenSoldat `Demo.pas` | Demo files store a header, tick count, next-frame markers, movement snapshots, camera changes, and replay through the network message path. | A combat game can make demo playback cheaper by reusing existing net/message handling. | It is not enough for rich terrain/body causality; our recorder needs semantic events too. |
| The Powder Toy snapshots | `Snapshot` stores particles, air pressure/velocity, heat, gravity maps, block maps, fan velocities, stickmen/signs, frame count, and RNG state; `SnapshotDelta` forwards/restores deltas. | Rich simulation snapshots should include RNG state and enough material fields to restore or compare. | Full snapshots of every terrain/material field are expensive; dirty chunks and deltas matter. |
| OpenLieroX NewNet | Attempts fixed 10 ms simulation, save/restore, net-synced random, and checksums; restore path is explicitly outdated. | Fixed-step physics and checksums help, but stale restore paths are a warning. | A half-built rollback/determinism layer can mislead planning if not validated continuously. |
| CCCP local code | `MovableMan::Update()` orders travel, Lua callbacks, pre-controller update, controller/AI update, scripts, actor/item/particle update, transfer/deletion, settle, seeing rays, and drawing layers. | Recorder hooks need to respect update order and avoid mutating sim state while collision/script code runs. | Existing `MovableObject::Save()` is not a robust replay snapshot API; it even notes scene save special-cases this path. |

## Decision Matrix

| Option | Description | Pros | Cons | Use In Slice A |
|---|---|---|---|---|
| A. Pure deterministic input replay | Record seed/config and per-tick input; replay by re-running sim. | Small files, ideal for rollback/netcode, clean acceptance evidence. | Fragile with physics, Lua, RNG, floating point, content changes, and object-order drift. | Probe only. Do not make it the main recorder promise. |
| B. Event log only | Record semantic events such as fire, hit, wound, carve, item choice, objective result. | Great for recaps, debugging, AI reason labels, and analytics. | Cannot restore arbitrary mutable terrain/body state by itself. | Required, but insufficient alone. |
| C. Snapshot/state stream | Record actor, inventory, projectile, and terrain state at cadence. | Robust playback and scrub anchors; tolerates nondeterminism. | Large files, serialization work, possible stale schema. | Required for actor/terrain anchors. |
| D. Hybrid events + snapshots + checksums | Record inputs, semantic events, periodic snapshots, content/config hashes, and compare checksums. | Best balance for physics-heavy prototype evidence. Supports death recap, AI, equipment, terrain, and networking estimates. | More schema work and UI/tooling burden. | Recommended default. |
| E. Network-replication replay backend | Design event/replay stream as future server-authoritative replication surface. | Efficient later if networking chooses server authority. | Premature if it blocks local fun and evidence capture. | Keep fields serializable and measured; do not force launch networking yet. |

## Slice A Run-Evidence Contract

[[references/prototype-run-bundle-schema]] already defines `run_manifest.json`, `events.jsonl`, `summary.json`, and `notes.md`. This note adds the semantic expectations for replay/determinism claims.

| File | Add / Require | Why |
|---|---|---|
| `run_manifest.json` | `determinism_mode`, `sim_tick_rate`, `rng_policy`, `content_hashes`, `engine_build`, `feature_flags` through schema `extensions` until stabilized. | Prevents a replay from claiming reproducibility without saying what was controlled. |
| `events.jsonl` | `input_intent`, `sim_checksum`, `snapshot_actor`, `snapshot_inventory`, `snapshot_terrain_chunk`, `recorder_dropped`, `ai_item_choice`, `ai_item_refusal`, `ai_item_result`. | Gives enough evidence to explain causality and measure drift. |
| `summary.json` | `checksum_mismatch_count`, `first_divergent_tick`, `snapshot_bytes`, `event_bytes`, `dropped_total`, `determinism_claim`, `largest_event_types` through extensions until stabilized. | Feeds DR-002, DR-005, DR-007, DR-008, and performance budgets. |
| `notes.md` | Under `Assumptions Tested`, state whether the run tested deterministic replay, hybrid replay, snapshot restore, or event-only recap. | Human notes must not overclaim what the run proved. |

### Event Families To Treat As First-Class

| Event Family | Required Payload | Main Consumer |
|---|---|---|
| `input_intent` | actor id, player/controller id, input mode, move/aim, selected item, buttons, tick. | Actor feel, replay, determinism probes, future input prediction. |
| `sim_checksum` | checksum scope, tick, value, optional actor/material/chunk split. | Divergence detection and future network experiments. |
| `snapshot_actor` | transform, velocity, status, health, selected item, inventory summary. | Death recap, scrub anchors, AI failure reports. |
| `snapshot_inventory` | actor id, item ids, ammo/condition/script state summary, selected slot. | Equipment/loadout debugging and replay restore. |
| `snapshot_terrain_chunk` | chunk id, bbox, material/version checksum, optional compact payload or diff id. | Destruction replay and path invalidation evidence. |
| `recorder_dropped` | dropped count, category/type, buffer depth, reason. | Performance and trust. Dropped evidence must be visible. |
| `ai_item_choice/refusal/result` | item id, target context, score inputs, reason label, result, claim-state delta. | Equipment AI contract, workbench diagnostics, replay/debug. |

## Local Hook Map

| Hook Target | Local Evidence | Why It Belongs In The Recorder |
|---|---|---|
| `MovableMan::Update()` | `Source/Managers/MovableMan.cpp:1269-1328` increments the sim frame, transfers alarm buffers, runs travel, Lua callbacks, pre-controller update, then controller/AI update. | This is the top-level ordering anchor for per-tick recorder flushing. |
| `MovableMan::UpdateControllers()` | `MovableMan.cpp:1754-1790` updates controllers, runs threaded AI, then `UpdateAI`. | This is where player/AI intent should be captured before consequences are ambiguous. |
| `MovableMan::PreControllerUpdate()` | `MovableMan.cpp:1795-1812` updates actors/items/particles before controller input. | Useful boundary for previous-tick state snapshots versus new intent. |
| `Controller::Update()` | `Source/System/Controller.cpp:147-174` branches player, AI, and reset paths. | Record `input_intent` and `ai_intent` through one control surface. |
| `Controller::ShouldUpdateAIThisFrame()` | `Controller.cpp:196-208` throttles AI by contiguous actor id and sim update count. | AI replay evidence must say when a bot did not think because the update was throttled. |
| `MovableObject::Save()` | `Source/Entities/MovableObject.cpp:402-430` serializes some physical fields but notes scene save special-cases this path. | Do not treat existing save methods as a complete replay snapshot layer. |
| Terrain mutation hooks | `SceneMan::TryPenetrate`, `SLTerrain::EraseSilhouette`, dirty-region/path notes. | Terrain snapshots should be dirty-region/chunk based, not every pixel event forever. |
| Equipment/AI hooks | [[references/equipment-ai-behavior-contract]], [[spec/equipment-loadout]] | Item choice/refusal/result events must share role-card ids, warning ids, and package ids with loadout/workbench tooling. |

## Comparable Lessons To Borrow

| Comparable | Borrow | Avoid |
|---|---|---|
| OpenSoldat demos | Store a header, version, map, tick count, and process playback through a message path. | Do not rely only on movement/network packets; Cortex-like destruction needs richer semantic events. |
| Powder Toy snapshots | Include RNG state, material fields, particles, environment fields, and deltas/restore paths. | Do not snapshot everything every frame. Use dirty chunks and cadence budgets. |
| OpenLieroX NewNet | Fixed-step simulation, save/restore boundaries, and checksums as desync signals. | Do not let an outdated restore path become a false proof of rollback feasibility. |
| Unreal replays | Replay/replication metadata and version fields; scrub/speed/pause as user-facing expectations. | Do not assume an engine without replicated state can copy the same architecture directly. |
| Photon Quantum | Checksums and deterministic config/assets/input history for proof. | Do not claim full determinism until content, scripts, physics, RNG, and state restore are controlled. |

## Acceptance Tests

| ID | Test | Pass Condition |
|---|---|---|
| DET-A-01 | Input replay probe | A 30-second fixed-seed actor run records input intent and can re-run with identical control commands through the same tick sequence. |
| DET-A-02 | Checksum surface | Run emits `sim_checksum` at a fixed cadence and summary reports zero or named mismatch count. |
| DET-A-03 | First divergence report | If checksum mismatch is injected or observed, report `first_divergent_tick`, category, and nearest parent event. |
| DET-A-04 | Snapshot restore smoke | Actor/inventory snapshot can restore enough state for a death recap or replay viewer anchor. |
| DET-A-05 | Terrain chunk evidence | One dig/explosion run emits terrain dirty chunk checksum/payload and byte count. |
| DET-A-06 | Equipment causality | A loadout run links item role id -> selected/refused reason -> fire/support result -> replay/death/diagnostic surface. |
| DET-A-07 | Run-bundle hygiene | `prototype_run_check.py` passes and all acceptance tests cite real event ids. |

## Decision Updates

| Decision | Update |
|---|---|
| [[decisions/dr-002-replay-event-architecture]] | Lean should stay hybrid event log + snapshots + checksums until deterministic islands are proven. |
| [[decisions/dr-005-multiplayer-posture]] | Direction is now server-authoritative solo/LAN/online/PvP/MMO-ready via `cx-server`; use event volume, snapshot size, and checksum stability to prove M9-M12 gates or reopen the DR explicitly. |
| [[decisions/dr-007-terrain-material-model]] | Terrain model tests must report dirty-region size, chunk snapshot bytes, and material/path invalidation events. |
| [[decisions/dr-008-ai-architecture]] | AI trust harness should consume `ai_item_*`, `ai_intent`, and `sim_checksum` events so bot failures are reproducible. |

## Source Trail

### Local

- `../Cortex-Command-Community-Project/Source/Managers/MovableMan.cpp:1269`
- `../Cortex-Command-Community-Project/Source/Managers/MovableMan.cpp:1754`
- `../Cortex-Command-Community-Project/Source/Managers/MovableMan.cpp:1795`
- `../Cortex-Command-Community-Project/Source/System/Controller.cpp:147`
- `../Cortex-Command-Community-Project/Source/System/Controller.cpp:196`
- `../Cortex-Command-Community-Project/Source/Entities/MovableObject.cpp:402`
- `../comparables_repos/opensoldat/shared/Demo.pas:40`
- `../comparables_repos/opensoldat/shared/Demo.pas:129`
- `../comparables_repos/opensoldat/shared/Demo.pas:275`
- `../comparables_repos/opensoldat/shared/Demo.pas:368`
- `../comparables_repos/the-powder-toy/src/simulation/Snapshot.h:11`
- `../comparables_repos/the-powder-toy/src/simulation/SnapshotDelta.h:6`
- `../comparables_repos/the-powder-toy/src/gui/game/GameModel.cpp:419`
- `../comparables_repos/openlierox/src/common/NewNetEngine.cpp:28`
- `../comparables_repos/openlierox/src/common/NewNetEngine.cpp:117`
- `../comparables_repos/openlierox/src/common/NewNetEngine.cpp:148`
- `../comparables_repos/openlierox/src/client/CClient_Game.cpp:230`
- [[systems/replay-event-architecture]]
- [[spec/replay-recorder-slice-a]]
- [[references/prototype-run-bundle-schema]]
- [[references/equipment-ai-behavior-contract]]

### Public

- Unreal Engine Replay System: https://dev.epicgames.com/documentation/en-us/unreal-engine/using-the-replay-system-in-unreal-engine
- Photon Quantum Replay: https://doc.photonengine.com/quantum/current/manual/replay
- Gaffer On Games, Deterministic Lockstep: https://gafferongames.com/post/deterministic_lockstep/
- Gaffer On Games, Snapshot Interpolation: https://gafferongames.com/post/snapshot_interpolation/
- YellowAfterlife, Preparing Your Game For Deterministic Netcode: https://yal.cc/preparing-your-game-for-deterministic-netcode/
- GameDeveloper, Developing Your Own Replay System: https://www.gamedeveloper.com/programming/developing-your-own-replay-system

## Change Log

- 2026-05-04: Added after targeted replay/determinism research across Unreal, Photon Quantum, Gaffer, YellowAfterlife, GameDeveloper, OpenSoldat, Powder Toy, OpenLieroX, and CCCP update/save hooks.
