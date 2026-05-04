---
type: spec
status: prototype-requirements
ready_when: "A minimal scenario runner executes AI-H-01..AI-H-06 against the actor-feel sandbox with recorder output, reason labels, and pass/fail reports."
feeds:
  - DR-002
  - DR-004
  - DR-008
  - DR-009
---

← [[spec/index|spec section]] · [[spec/ai-and-command|AI and command spec stub]] · [[systems/ai-trust-test-suite|AI trust suite]] · [[engine/ai-order-lifecycle|AI order lifecycle]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[decisions/dr-008-ai-architecture|DR-008]]

# AI Trust Harness Slice A

> [!danger] Solo-first infrastructure
> This is the build checklist for turning AI trust from a document into a regression system. It is not the final AI architecture. It is the minimum harness that lets us see, replay, and measure whether bots are useful in a destructible scene.

## Purpose

The future game can only make a strong solo-first promise if bot failures are visible, replayable, and fixable. The first AI harness should answer:

> Can a small bot scenario prove what the bot intended, what it saw, why it chose an action, whether the terrain changed under it, and how it recovered?

This harness should run alongside [[spec/actor-feel-sandbox-slice-a]] and [[spec/replay-recorder-slice-a]]. It should produce replay artifacts and structured reports before the team commits to larger squad AI, commander AI, campaign objectives, or multiplayer AI sync.

## Inputs

| Source | Requirement Pulled Forward |
|---|---|
| [[systems/ai-trust-test-suite]] | AI-01..AI-12 scenarios, metrics, and debug overlay requirements. |
| [[engine/ai-order-lifecycle]] | C++ controller/path plumbing, Lua behavior coroutines, AI update throttle, alarm processing. |
| [[spec/replay-recorder-slice-a]] | Event envelope, stable ids, cause chains, snapshots, JSONL/export, death/failure recap. |
| [[spec/actor-feel-sandbox-slice-a]] | Test scene, actor/material/equipment baseline, serializable intent/control surface. |
| [[systems/material-and-mobility-affordance-schema]] | AI-visible material/tool/mobility/hazard fields and refusal labels. |
| [[comparables/opensoldat-local-audit]] | Bots use gameplay control states; demos and HUD make bot behavior inspectable. |
| [[comparables/openlierox-local-audit]] | Bot stuck recovery, rope/movement heuristics, weapon/path decisions, terrain clearing, and network caution. |
| Unreal AI Debugging docs | Centralized runtime debug surfaces for behavior, path/nav, perception, and query state. |
| CRYENGINE Modular Behavior Tree Debugging docs | Per-agent execution history and active branch visualization are useful even when final AI is not behavior-tree based. |
| Recast Navigation README | Navigation systems need debug visualization; tile/cache/crowd concepts are inspiration, not a drop-in fit for pixel terrain. |

## Harness Scope

| Area | Slice A Must Have | Later |
|---|---|---|
| Runner | Loads a scenario manifest, seed, map fixture, actors, orders, mutation script, and timeout. | Full CI farm, campaign-scale fixtures, random fuzzing. |
| Bots | One or two AI-controlled actors using the same intent/control layer as players. | Full squad, commander, personalities, learned preferences. |
| Terrain | Static bunker/door/dirt wall plus one scripted terrain mutation. | Procedural maps, collapse-lite, Noita-grade materials. |
| Recorder | Requires [[spec/replay-recorder-slice-a]] events plus AI reason events. | Polished replay browser and long-term replay compatibility. |
| Assertions | Scenario-level pass/fail metrics and failure reasons. | Statistical confidence across hundreds of seeds. |
| Debug UI | AI overlay and event report with current order, tactic, target, path, stuck state, and tool/material reason. | Full tactical map, scrubbable behavior timeline, visual authoring editor. |

## Scenario Manifest

Use a small, plain manifest so scenarios can be copied, tweaked, and reviewed in the vault.

| Field | Purpose |
|---|---|
| `scenario_id` | Stable id like `AI-H-02` or `AI-04`. |
| `seed` | Reproducibility for actor jitter, projectiles, and random AI choices. |
| `fixture_map` | Test room or bunker fixture. |
| `actors` | Actor presets, team, role, loadout, starting position, AI mode/job. |
| `orders` | Initial command: defend, breach, dig, rescue, retreat, pickup, suppress. |
| `terrain_mutations` | Optional scripted changes during the run: collapse, new breach, blocked tunnel. |
| `threats` | Enemies, alarm events, hazards, delivery warning, turret, door, blocked path. |
| `success_assertions` | Must-pass measurable outcomes. |
| `failure_assertions` | Instant fail conditions: self-kill, friendly fire, idle loop, no reason label. |
| `telemetry_required` | Required event types and overlay fields. |
| `timeout_ms` | Hard stop for the scenario. |

## AI Event Contract

These events extend [[spec/replay-recorder-slice-a]]. They should use the same envelope and stable `record_id` rules.

| Event | Required Payload | Why |
|---|---|---|
| `ai_update_allowed` | actor id, update interval, contiguous id, tick modulo, skipped/allowed. | Exposes throttle effects from `Controller::ShouldUpdateAIThisFrame`. |
| `ai_order_issued` | issuer, target actor/squad, order type, target, priority, timeout. | Separates persistent order from tactic. |
| `ai_behavior_selected` | actor id, `AIMode`, behavior name, previous behavior, reason. | Mirrors Cortex coroutine selection and prevents hidden mode flips. |
| `ai_tactic_scored` | actor id, tactic, score, top rejected options, reason labels. | Required if we pursue DR-008 utility scoring. |
| `ai_target_acquired` | actor id, target id, target type, threat score, line-of-sight or alarm source. | Explains "why did the bot shoot/move?" |
| `ai_perception_signal` | signal type, source id/pos, confidence, occlusion/ray result, decay. | Makes alarms and sight readable. |
| `ai_path_requested` | actor id, start, goal, jump height, dig strength, team, request id. | Path failures need a cause. |
| `ai_path_completed` | request id, status, length, total cost, waypoint count. | Uses `PathRequest` fields exposed to Lua. |
| `ai_path_invalidated` | actor id, path id, dirty bbox, terrain/material cause. | Destructible terrain must update bot beliefs. |
| `ai_stuck_state_changed` | actor id, stuck time, average velocity, blocker id/material, old/new state. | Core trust metric. |
| `ai_recovery_action` | actor id, action, reason, retry time, expected outcome. | Failures are acceptable only if recovery is legible. |
| `ai_tool_choice` | actor id, tool/item id, target material, expected effect, rejected tools. | Breaching/digging is a first-class action. |
| `ai_hazard_reflex` | actor id, hazard id/type, risk score, action/refusal. | Prevents "solo AI walks into obvious death" failures. |
| `ai_friendly_fire_check` | actor id, weapon id, target id, blocked actors, decision. | Makes squad trust testable. |
| `ai_test_assertion` | scenario id, assertion id, status, measured value, threshold. | Machine-readable pass/fail report. |
| `ai_test_result` | scenario id, status, duration, failures, replay path. | CI/dashboard friendly. |

## Local Hook Map

| Harness Need | Hook Candidate | Evidence |
|---|---|---|
| AI tick allowed/skipped | `Controller::ShouldUpdateAIThisFrame()` and `MovableMan::UpdateControllers()`. | `Controller.cpp:196-208`; `MovableMan.cpp:1754-1792`. |
| Threaded/main AI phase | C++ dispatches `ThreadedUpdateAI` before `UpdateAI`. | `MovableMan.cpp:1763-1789`; script function names in `Actor.h:82`. |
| AIMode and path surface | Lua bindings expose `AIMode`, `MovePathSize`, `MovePath`, `UpdateMovePath`, `SetAlarmPoint`, `GetAlarmPoint`. | `LuaBindingsEntities.cpp:198-267`. |
| Path request completion | Async path callback moves from path thread into main Lua callback; `PathRequest` exposes path, length, status, total cost. | `LuaAdapters.cpp:288-310`; `LuaBindingsSystem.cpp:236-246`. |
| Behavior creation | `NativeHumanAI` chooses behavior coroutine from `Owner.AIMode`. | `NativeHumanAI.lua:194-237`. |
| Target acquisition | `NativeHumanAI` looks for targets, compares threat, stores offsets, and starts attack behavior. | `NativeHumanAI.lua:273-314`. |
| Behavior run/completion | Current behavior coroutine resumes and is cleared; missing weapon/tool behavior can be queued. | `NativeHumanAI.lua:521-546`. |
| Alarm and medikit logic | Alarm point reactions, medikit use, team block handling, weapon fire state. | `NativeHumanAI.lua:558-615`; `SharedBehaviors.lua:25-102`. |
| Door/breach/weapon choice | Attack behavior checks door material, breaching tools, firearm penetration, grenades, diggers, weapon search. | `NativeHumanAI.lua:758-817`; `HumanBehaviors.lua:1500-1580`. |
| Path follow, stale path, stuck, digging | GoTo path loop uses update timers, stuck timer, path refresh, obstacle scans, digging, and waypoint removal. | `SharedBehaviors.lua:379-435`, `520-580`, `653-743`, `778-815`. |
| Weapon/tool pickup | Searches nearby devices, scores path length, queues pickup path. | `HumanBehaviors.lua:568-698`, `703-817`. |
| Squad/task layer | `TacticsHandler` exposes Attack/Defend/PatrolArea/Brainhunt/Sentry tasks, squad membership, retask, save/load. | `TacticsHandler.lua:1-23`, `68-95`, `223-237`. |

## Slice A Scenario Set

These are harness bootstrap scenarios. They do not replace the full AI-01..AI-12 suite; they make it runnable.

| ID | Scenario | Maps To | Setup | Pass Criteria |
|---|---|---|---|---|
| AI-H-01 | Sentry hears threat | AI-01, AI-05 | One defender, one offscreen enemy shot/alarm, simple cover. | Bot records alarm, turns or suppresses with reason, does not abandon defend order. |
| AI-H-02 | GoTo path with new blockage | AI-04, AI-12 | Bot moving to waypoint; terrain mutation blocks path halfway. | Bot emits path invalidation/stuck/recovery events and replans/digs/retreats within timeout. |
| AI-H-03 | Breach material gate | AI-02, AI-03, AI-11 | Door or concrete barrier; bot has firearm + digger/breaching tool. | Bot chooses effective tool or reports missing capability; no infinite shooting at unbreakable material. |
| AI-H-04 | Weapon/tool pickup | AI-10 | Unarmed bot near weapon and digger under light threat. | Bot evaluates reachable item, moves to it, picks it up, resumes useful behavior. |
| AI-H-05 | Medikit/reflex interrupt | AI-09 | Hurt bot with medikit, then threat appears. | Bot heals only when safe and interrupts healing when threat crosses threshold. |
| AI-H-06 | Friendly obstruction | AI-08 | Two friendly bots in narrow bunker path. | Bot emits blocked/yield/recovery labels and avoids permanent clumping. |

## Full Suite Mapping

| Existing Test | Harness Requirement Before It Can Run |
|---|---|
| AI-01 Defend brain room | Orders, alarm reaction, current intent overlay, defend-zone assertions. |
| AI-02 Breach door | Tool/material affordance check, target material, rejected option labels. |
| AI-03 Dig to objective | Path request, dig plan, terrain-carve events, self-trap assertion. |
| AI-04 Recover from collapsed tunnel | Terrain mutation script, path invalidation event, stuck/recovery timer. |
| AI-05 Suppress alarm | Perception signal, occlusion/ray result, suppression reason. |
| AI-06 Rescue downed/veteran unit | Role/job manifest, threat score, rescue/extract/cover/refuse labels. |
| AI-07 Avoid dropship crush | Hazard/reflex event, delivery warning marker, avoidance assertion. |
| AI-08 Hold formation through bunker | Squad slots, blocker/yield labels, obstruction duration metric. |
| AI-09 Use medikit | Health threshold, safety check, interrupt condition. |
| AI-10 Weapon pickup | Item value, path score, pickup target, resume behavior. |
| AI-11 Retreat from unwinnable breach | Missing capability detection and refusal/retool order. |
| AI-12 Enemy flanking through new hole | Terrain dirty region -> path update latency -> new route choice. |

## Report Format

| Section | Contents |
|---|---|
| Header | scenario id, seed, build id, map fixture, actor presets, recorder schema version. |
| Result | pass/fail/invalid, runtime, first failing assertion, replay/export path. |
| Metrics | time to first useful action, path recovery time, unexplained idle time, obstruction duration, wasted shots, tool selection correctness. |
| Event Coverage | required events present/missing; dropped event count. |
| Timeline | condensed event chain with order, behavior, target, path, perception, stuck/recovery, tool/fire decisions. |
| Regression Delta | previous run comparison once history exists. |

## Debug Overlay

| Overlay Field | Source Event / State |
|---|---|
| Current order | `ai_order_issued`, scenario manifest. |
| Behavior/tactic | `ai_behavior_selected`, `ai_tactic_scored`. |
| Target/perception | `ai_target_acquired`, `ai_perception_signal`. |
| Path | `ai_path_requested`, `ai_path_completed`, `ai_path_invalidated`. |
| Stuck/recovery | `ai_stuck_state_changed`, `ai_recovery_action`. |
| Tool/material | `ai_tool_choice`, terrain material events. |
| Fire safety | `ai_friendly_fire_check`, `weapon_fired`, `projectile_spawned`. |
| Assertion status | `ai_test_assertion`, `ai_test_result`. |

## Acceptance Tests

| ID | Test | Pass Criteria |
|---|---|---|
| AI-HARNESS-01 | Scenario manifest load | Runner loads AI-H-01 from data and spawns actors/map/orders with a fixed seed. |
| AI-HARNESS-02 | AI telemetry coverage | Every run includes tick allowance, behavior selected, path state, target/perception, and test result events. |
| AI-HARNESS-03 | Recorder integration | Harness output links to JSONL/replay data from [[spec/replay-recorder-slice-a]]. |
| AI-HARNESS-04 | First useful action metric | AI-H-01 records a useful action within a threshold and labels the reason. |
| AI-HARNESS-05 | Stuck recovery metric | AI-H-02 detects blocked/stuck state and emits a recovery action or impossible label within timeout. |
| AI-HARNESS-06 | Tool/material decision | AI-H-03 records target material, chosen tool, rejected options, and outcome. |
| AI-HARNESS-07 | No silent idle | Any idle longer than threshold requires a current reason label. |
| AI-HARNESS-08 | Debug overlay smoke | Viewer/overlay can show current order, behavior, target, path, stuck state, and latest assertion. |
| AI-HARNESS-09 | Report export | Runner emits a machine-readable result plus a human-readable summary table. |
| AI-HARNESS-10 | Regression repeatability | Same seed produces comparable event categories and assertion outcomes across three runs; exact physics determinism is not required yet. |

## First Build Tickets

| Order | Ticket | Done When |
|---|---|---|
| 1 | Define scenario manifest schema and AI-H-01 fixture. | Manifest can spawn one defender and one alarm/threat. |
| 2 | Add `ai_test_result` and assertion events to recorder schema. | Reports can fail loudly without parsing logs. |
| 3 | Emit AI tick, behavior, target, and path events. | AI-HARNESS-02 has minimum coverage. |
| 4 | Add runner command or debug menu action. | AI-H-01 can run repeatedly from a clean state. |
| 5 | Build simple report writer. | JSON + markdown report lists metrics, failures, and replay path. |
| 6 | Add AI-H-02 path blockage fixture. | Stuck/path invalidation and recovery are testable. |
| 7 | Add AI-H-03 breach fixture. | Tool/material decision is testable. |
| 8 | Add overlay panel for current order/behavior/path/stuck/assertion. | AI-HARNESS-08 is testable. |
| 9 | Add run-history comparison. | Same-seed regression deltas become visible. |
| 10 | Expand to AI-H-04..AI-H-06. | Pickup, medikit, and obstruction tests become automated. |

## Failure Modes

| Failure | Mitigation |
|---|---|
| Harness only proves flat-ground pathing. | Every serious scenario includes destructible terrain, material gates, hazards, or mutation. |
| AI passes but player cannot read it. | Pass requires event labels and overlay fields, not only objective success. |
| Too many AI events flood the recorder. | Store high-rate internals as debug-only channels; publish condensed reason events. |
| Exact deterministic replay is impossible. | Compare assertions and event categories first; use seeds to reduce variance, not to promise full determinism. |
| Scenario authoring gets slow. | Keep manifest small and copyable; build editor/workbench support later. |
| Utility scores become magic numbers. | Require `ai_tactic_scored` top alternatives and reason labels. |
| Lua hook surface breaks invariants. | Engine owns reflex/order contracts; Lua can add tactics/reasons inside typed fields. |

## Decisions Fed

| Decision | Evidence This Harness Should Produce |
|---|---|
| [[decisions/dr-002-replay-event-architecture]] | Whether AI reason events fit recorder budgets and death/failure recaps. |
| [[decisions/dr-004-first-playable-slice]] | Whether the first playable can support bot-driven scenarios after actor feel works. |
| [[decisions/dr-008-ai-architecture]] | Whether jobs + utility + script hooks are necessary or too heavy. |
| [[decisions/dr-009-command-ux-style]] | Which AI reasons must appear in HUD/overlay/tactical map. |

## Source Trail

- `Cortex-Command-Community-Project/Source/Managers/MovableMan.cpp:1754`, `1763`, `1786`
- `Cortex-Command-Community-Project/Source/System/Controller.cpp:196`
- `Cortex-Command-Community-Project/Source/Entities/Actor.h:82`, `270`
- `Cortex-Command-Community-Project/Source/Lua/LuaBindingsEntities.cpp:198`, `206`, `240`, `264`
- `Cortex-Command-Community-Project/Source/Lua/LuaAdapters.cpp:288`
- `Cortex-Command-Community-Project/Source/Lua/LuaBindingsSystem.cpp:236`
- `Cortex-Command-Community-Project/Data/Base.rte/AI/NativeHumanAI.lua:83`, `194`, `273`, `521`, `558`, `758`
- `Cortex-Command-Community-Project/Data/Base.rte/AI/SharedBehaviors.lua:25`, `379`, `520`, `653`, `778`
- `Cortex-Command-Community-Project/Data/Base.rte/AI/HumanBehaviors.lua:568`, `703`, `1500`
- `Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/TacticsHandler.lua:1`, `68`, `223`
- Unreal AI Debugging: `https://dev.epicgames.com/documentation/en-us/unreal-engine/ai-debugging-in-unreal-engine`
- CRYENGINE Modular Behavior Tree Debugging: `https://www.cryengine.com/docs/static/engines/cryengine-5/categories/23756813/pages/23310620`
- Recast Navigation: `https://github.com/recastnavigation/recastnavigation`
- GDC Vault, AI Behavior Editing and Debugging in The Division: `https://gdcvault.com/play/1023382/AI-Behavior-Editing-and-Debugging`
