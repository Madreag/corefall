---
type: spec
status: prototype-reqs
ready_when: "A destructible proof mission runs with a typed manifest, serializable director/commander state, objective events, equipment capability requirements, replay export, and MISSION-A-01..18 acceptance evidence."
feeds:
  - DR-002
  - DR-003
  - DR-004
  - DR-007
  - DR-008
  - DR-009
  - DR-039
  - DR-040
  - DR-042
  - DR-043
---

<- [[spec/index|spec section]] · [[spec/missions-and-objectives|missions and objectives]] · [[systems/destruction-objective-mission-patterns|destruction-objective patterns]] · [[engine/activity-scenario-lifecycle|activity lifecycle]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/ai-trust-harness-slice-a|AI harness]] · [[spec/replay-recorder-slice-a|replay recorder]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/game-modes-and-match-grammar|game modes / match grammar]] · [[spec/comms-voice-and-radio-model|comms/voice/radio]]

# Mission Director Slice A

> [!summary] Purpose
> Define the first build-facing mission/director contract: typed mission manifests, destruction-aware objective grammar, commander AI state, director pacing, equipment capability requirements, save/replay state, UI obligations, and MISSION-A acceptance tests.

> [!important] Product stance
> Missions are where physics, AI, loadouts, UX, replay, and retention become a game. A mission is not just a Lua script and a win condition. It is a repeatable contract that tells the player what matters, tells AI what to do, tells the UI what to explain, tells mods how to validate, and tells the recorder what happened.

## Slice A Question

Can we build one compact bunker-breach contract where a player can buy a small capability-based squad, breach destructible terrain, read objective and commander intent, watch an enemy commander adapt, save/replay the result, and diagnose every AI/loadout failure from structured events?

## Evidence Stack

### Local Cortex / CCCP Evidence

| Evidence | Local Path | Design Lesson |
|---|---|---|
| `Activity` is the base game-mode contract: teams, funds, player config, state, save flags, and generic saved values. | `Source/Entities/Activity.cpp:231-283`, `Source/Entities/Activity.h:537-563` | Our mission manifest needs typed state and typed save keys. Generic key/value save is useful, but the spec should not hide schema drift. |
| `GAScripted` enables Lua activities and treats scripts with `OnSave` as user-saveable by default. | `Source/Activities/GAScripted.cpp:64-67`, `:109-116`, `:171-187` | Scripted missions are powerful, but save support must be explicit and testable. |
| `ActivityMan::Save` writes the active activity, then asks scene objects to save themselves, then writes scene state into `Save.ini`. | `Source/Managers/ActivityMan.cpp:145-199` | Mission state, terrain/object state, movable-object state, and replay state must be versioned together. |
| `MetaFight.lua` has no custom `OnSave`, `SaveNumber`, or `SaveString` schema; metagame battles are rebuilt/programmed by `MetagameGUI` and resumed in memory when unfinished. | `Data/Base.rte/Activities/MetaFight.lua`; `Source/Menus/MetagameGUI.cpp:2513-2514`, `:3686`, `:4356-4358` | Campaign/offensive battles still need explicit serializable director state in our game. In-memory resume is not enough for long-running contracts or replay evidence. |
| `BunkerBreach.lua` declares scene-area contracts, places brains, initializes attacker/defender AI, checks win conditions, emits objective arrows, and controls reinforcements. | `Data/Base.rte/Activities/BunkerBreach.lua:1-19`, `:21-44`, `:243-285`, `:348-399`, `:401-620` | This is the best CCCP proof-mission reference: scene areas, teams, brains, reinforcements, objective text, and destruction pressure all exist in one script. |
| `BunkerBreach.lua` uses `OnSave`/`ResumeLoadedGame`, but also shows schema risks: major attack timer values appear to save from `spawnTimer`, and internal reinforcement budget reloads as boolean. | `Data/Base.rte/Activities/BunkerBreach.lua:287-332` | Future mission save schemas need typed validators, roundtrip tests, and migration warnings. |
| `LandingZoneMap.lua` scores LZs using terrain altitude, enemy LOS, fog, occupied/craft avoidance, async path requests, path length, and obstacle height. | `Data/Base.rte/Activities/Utility/LandingZoneMap.lua:59-137`, `:441-557`, `:607-756` | Enemy commander spawn logic should be terrain-aware and explainable, not "random dropship here." |
| `TacticsHandler.lua` provides squads and tasks: Attack, Defend, PatrolArea, Brainhunt, Sentry, with retask timers and optional save/load. | `Data/Base.rte/Activities/Utility/TacticsHandler.lua:14-22`, `:126-220`, `:299-349`, `:443-510` | Commander AI should own squads/tasks as serializable intent, not only per-actor mode flags. |
| `GameActivity::CreateDelivery` turns override purchases or buy-menu orders into craft, cargo, actor teams, AI modes, LZ assignment, delay, and funds deduction. | `Source/Activities/GameActivity.cpp:399-640` | Missions must request capabilities and delivery policies, then let loadout/craft systems instantiate them with visible cost and risk. |
| Lua exposes `AddObjectivePoint`, `SetLandingZone`, `CreateDelivery`, `SetOverridePurchaseList`, `SaveNumber`, `LoadNumber`, `SaveString`, and `LoadString`. | `Source/Lua/LuaBindingsActivities.cpp:61-164` | Our mission scripting API should keep these verbs but wrap them in typed events and manifest validation. |

### External Mission / Director Evidence

| Source | Durable Lesson For This Spec |
|---|---|
| Michael Booth, Valve, "The AI Systems of Left 4 Dead" | Use a simple pacing state machine and emotional-intensity metric. Adjust pacing frequency and population timing before silently changing difficulty amplitude. |
| Michael Booth, Valve, "Replayable Cooperative Game Design: Left 4 Dead" | Treat the team as the player, require cooperation through situation design, and use structured unpredictability instead of fixed memorized scripts. |
| Daniel Brewer, "Managing Pacing in Procedural Levels in Warframe" | Use a coarse flow graph/TacMap, influence maps, active areas, visibility filters, objective distance, and mission-script overrides to spawn threats without hand-placed trigger dependence. |
| Travis Hoffstetter, GameDeveloper, "The Art and Science of Pacing and Sequencing Combat Encounters" | Intensity is how often the player must re-plan. Enemy type, count, spawn timing, spawn location, destruction events, dialogue, and music are all pacing knobs. |
| Dennis Gustafsson, "Teardown design notes" | Fully destructible missions cannot rely on normal walls as obstacles. Plan/action phases, optional targets, compact vertical spaces, one prep save, and tool upgrades turn destruction into repeatable gameplay. |
| GameDeveloper, "How beautiful voxels laid the way for Teardown's heist-y framework" | When destruction is the core technology, objectives must exploit freedom instead of fighting it. |
| 80.lv, "Teardown Developer Breaks Down Multiplayer And Voxel Destruction Tech" | Networked destruction needs deterministic destruction commands plus state sync, not raw voxel dumps. That should inform future mission replay/network event shape. |

## What CCCP Actually Does

| Layer | CCCP Shape | What To Preserve | What To Fix |
|---|---|---|---|
| Activity lifecycle | `ActivityMan` owns one active activity; `GAScripted` delegates lifecycle to Lua. | Scriptable missions with C++ manager support. | Require typed manifests, validation, event export, and save roundtrip tests. |
| Campaign battle wrapper | `MetaFight` is created by metagame code, has no custom save schema, and can be resumed while still in memory. | Strategic/campaign systems can generate tactical battles. | Campaign-generated battles need serializable mission/director state and replay/export identity. |
| Proof mission | `BunkerBreach` has named scene-area requirements, brain placement, defender/attacker AI, reinforcements, win checks, and objective arrows. | Scene contracts plus scriptable mission rules. | Move hidden string contracts into manifest validation and UI-readable mission requirements. |
| Spawn/LZ logic | `LandingZoneMap` asynchronously scores LZs using terrain/LOS/path/fog/craft collision. | Terrain-aware spawn selection. | Emit LZ score reasons and support replay/debug panels. |
| Squad/task helper | `TacticsHandler` owns task types, squads, retask timers, and optional save/load. | Task vocabulary and squad grouping. | Promote tasks to commander intent with reason strings, priorities, state, and replay events. |
| Delivery/economy | `CreateDelivery` turns orders into craft/cargo/AI modes/funds changes. | Delivery as an economy and risk surface. | Expose capability fit, cargo survival risk, LZ danger, and item role constraints before purchase. |

## MetaFight Save Schema Finding

`MetaFight.lua` does not define a mission-specific `OnSave` block. It also does not use `SaveNumber`, `SaveString`, `LoadNumber`, or `LoadString`. `GAScripted` only assumes a script can be user-saved when `OnSave` exists, and `Activity::CanBeUserSaved()` blocks saving for metagame/internal contexts. The metagame UI handles an unfinished `MetaFight` by marking the Continue button as Resume and resuming the still-live activity.

Design implication: campaign battles can be generated by a strategy layer, but the tactical mission must still have a durable identity:

| Required State | Why |
|---|---|
| `mission_id`, `campaign_action_id`, `seed`, `scene_id`, `manifest_version` | Replays, bug reports, save migration, and campaign rollback need a stable handle. |
| director phase, intensity, timers, spawn queues, and RNG streams | In-memory resume is not enough for a long mission or a crash-safe run. |
| commander targets, known brains, LZ candidates, budgets, and doctrine | Enemy behavior must be explainable and recoverable after load. |
| objectives, progress, failure causes, UI markers, and event parents | The player and replay viewer need the same causal story. |
| loadout package ids, capability requirements, and item role cards | AI/UI/modding/balance must agree on what a mission asked for and what the squad brought. |

## Mission Manifest Model

A mission should be a typed document with optional script hooks, not a free-form script with hidden string contracts.

| Manifest Section | Required Fields | Consumers |
|---|---|---|
| Identity | `id`, `version`, `title`, `seed`, `scene`, `campaign_action_id`, `provenance`. | Save/load, replay, backend, mod packages, crash reports. |
| Scene contract | named areas, spawn anchors, brain vaults, LZ bands, destructible zones, critical objects, forbidden spawn zones. | Editor, validator, commander AI, UI map, replay. |
| Teams | factions, human/CPU slots, starting funds, brain policy, commander profile, tech whitelist. | Buy/loadout UI, AI, economy, backend session. |
| Objectives | objective id/type, owner, target selector, progress sensor, fail sensor, UI marker, replay event, director effect. | HUD, replay, AI, retention/challenges. |
| Director profile | phase graph, intensity inputs, spawn budget, peak/relax timings, pressure caps, recovery windows. | AI director, telemetry, balance. |
| Commander profile | doctrine, aggression, target preferences, LZ risk tolerance, reserve ratio, breach policy, retreat policy. | Enemy AI, debug overlay, replays. |
| Equipment requirements | capability tags, minimum role coverage, forbidden/optional roles, delivery footprint, bot-safety level. | Loadout workbench, AI, package validator, balancing. |
| Destruction policy | critical-object rules, terrain-edit budget, path refresh policy, collapse/hazard permissions, repair/fill support. | Simulation, AI, replay, networking. |
| Save schema | typed fields, migration version, timer/RNG fields, actor references, terrain digest, delivery queues. | Save/load tests, package migrations. |
| Event schema | required event families and payloads. | Recorder, AI harness, UX recap, backend analytics. |
| Script hooks | allowed hooks, permissions, deterministic class, save obligations. | Modding/workbench, replay, package diagnostics. |

### Example Manifest Sketch

```yaml
id: mission.breach_contract.slice_a
version: 0.1.0
seed: "${run_seed}"
scene: scene.slice_a_bunker
teams:
  player:
    faction: coalition
    funds: 2400
    brain_policy: protected_commander
    loadout_requirements:
      required_capabilities: [breach.light, fight.medium, dig.soft, fallback.sidearm]
      recommended_capabilities: [heal.basic, scout.sensor]
      bot_default_level: reviewed
  defender:
    faction: dummy
    funds: 1800
    commander_profile: defender_breach_response_v1
objectives:
  - id: obj_destroy_brain
    type: brain_hunt
    target_selector: enemy.primary_brain
    ui_marker: destroy
    director_effects: [increase_pressure_on_breach]
  - id: obj_optional_salvage
    type: recover_object
    optional: true
    target_selector: loot_cache.closest_to_main_bunker
director:
  profile: cortex_contract_director_v1
  phases: [setup, prep, launch, build_up, sustain_peak, peak_fade, relax, objective_push, extraction, debrief]
destruction:
  critical_objects: [enemy.primary_brain, player.commander]
  terrain_policy: destructible_except_bedrock
  path_refresh_budget_ms: 3
replay:
  required_events: [mission_start, objective_update, commander_decision, lz_scored, delivery_created, terrain_changed, director_intensity_sample, mission_end]
```

## Director State Machine

| Phase | What It Does | Exit Trigger | Required Events |
|---|---|---|---|
| `setup` | Validate scene, teams, packages, area names, critical objects, mission variables. | All required contracts present. | `mission_validate`, `mission_start`. |
| `prep` | Player buys loadout, studies terrain, places initial route/markers where allowed. | Player launches or timer expires if mission wants pressure. | `loadout_locked`, `route_marker_added`. |
| `launch` | Initial craft/actors enter, LZ risk is sampled, first objectives become visible. | First actor is controllable or first breach occurs. | `delivery_created`, `objective_start`. |
| `build_up` | Populate threats until intensity reaches target. | Team intensity crosses peak threshold or objective is close. | `director_phase_change`, `spawn_request`. |
| `sustain_peak` | Hold pressure briefly so the fight peaks instead of instantly relaxing. | Minimum peak time expires. | `director_intensity_sample`. |
| `peak_fade` | Stop new major pressure while existing combat resolves. | Natural break: no active close threats, or intensity decays. | `director_phase_change`. |
| `relax` | Minimal threats, recovery, salvage, route repair, squad regroup. | Enough travel/objective progress/time. | `recovery_window_start`, `recovery_window_end`. |
| `objective_push` | Focus pressure around an active objective or breach. | Objective completes/fails or pressure budget expires. | `objective_pressure`, `commander_decision`. |
| `emergency` | Rescue stuck/softlocked mission: replacement craft, alternate target, extraction route. | Recovery succeeds or fail condition is final. | `mission_recovery_action`. |
| `extraction` | Bring survivors/salvage out, resolve late threats, stop new major spawns. | Extraction complete or all recoverable actors lost. | `extract_start`, `extract_complete`. |
| `debrief` | Summarize causes, optional objectives, loadout performance, AI failures, replay links. | Player exits/deploys next contract. | `mission_end`, `debrief_generated`. |

Director rule: difficulty can change budgets and available enemy quality, but the pacing director should primarily change spawn timing, spawn location, recovery windows, and focus. Hidden difficulty rubber-banding makes the simulation feel fake.

## Intensity Inputs

The first director should compute transparent intensity from signals that matter in a Cortex-like game.

| Signal | Positive Pressure | Recovery / Relief |
|---|---|---|
| Actor damage | wounds, limb loss, status, knockdown, brain damage. | healing, regroup, safe cover, actor revived. |
| Terrain pressure | bunker breach, path collapse, route blocked, exposed brain, falling debris. | new route opened, repair/fill completed, safe tunnel established. |
| Enemy proximity | enemies near brain, enemies with LOS, explosives in danger radius. | enemies killed, no LOS, distance restored. |
| Delivery risk | dropship under fire, cargo lost, LZ compromised, craft crash. | craft exits safely, cargo delivered, LZ cleared. |
| Resource pressure | low ammo, no digger, no med tool, funds below threshold. | pickup/salvage, resupply, optional extraction. |
| Objective progress | player close to win, optional target taken, alarm triggered. | objective complete, optional target skipped, failover path exists. |
| Player command load | too many simultaneous alerts or bots asking for help. | clear order state, bots executing correctly, no stuck labels. |

Every intensity sample must be exportable to replay and debug overlays. If the player feels punished, the mission should show the cause.

## Commander AI Contract

The commander is the per-team planner that turns director pressure into concrete actions. The director asks "how much pressure and where"; the commander asks "what target, which squad, which route, which loadout, which delivery, and why."

| State | Required Fields | Why |
|---|---|---|
| Doctrine | `aggression`, `risk_tolerance`, `breach_preference`, `defend_ratio`, `reserve_ratio`, `retreat_threshold`. | Lets designers create enemy personalities without rewriting AI. |
| Knowledge | known brains, last seen actors, alarms, LZ scores, breaches, blocked paths, route confidence. | Prevents omniscient AI while keeping decisions explainable. |
| Budget | funds, spawn budget, reserve floor, delivery cooldown, package options. | Connects AI pressure to economy and fairness. |
| Targets | current target id, target type, confidence, priority, time since chosen, fallback targets. | Makes commander intent visible in replay/debug. |
| Squads | squad id, members, assigned task, task reason, retask timer, stuck state, equipment capability summary. | Bridges `TacticsHandler` style tasks to AI trust tests. |
| Delivery | chosen craft, cargo, LZ, LZ score reasons, path obstacle estimate, landing risk. | Makes reinforcements feel like logistics, not spawn magic. |

### Commander Decision Verbs

| Verb | Inputs | Output Event |
|---|---|---|
| `choose_target` | objective pressure, known enemies, brain positions, damage done, distance/path score. | `commander_decision.target_selected` |
| `score_lz` | LZ terrain, LOS, fog, occupied/craft risk, path length, obstacle height, delivery footprint. | `commander_decision.lz_scored` |
| `build_package` | mission capability need, funds, role cards, craft capacity, bot-safe item flags. | `commander_decision.package_built` |
| `assign_squad` | squad capabilities, task priority, path confidence, stuck state. | `commander_decision.squad_tasked` |
| `breach` | material affordance, dig strength, blast radius, friendly danger, objective value. | `commander_decision.breach_ordered` |
| `defend` | brain threat, breach openings, route confidence, reserve budget. | `commander_decision.defend_ordered` |
| `retreat_or_rescue` | wounds, route danger, player orders, med/support availability. | `commander_decision.recovery_ordered` |

Every commander decision needs a short reason string and structured fields. Copy the spirit of L4D-style action debug: the replay/debug view should answer "why did the AI do that?" without reading source.

## Objective Grammar

| Objective Type | Sensors | UI Marker | Director Effect | Equipment / AI Needs |
|---|---|---|---|---|
| `brain_hunt` | target brain alive/dead, route confidence, last seen position. | Destroy / Protect. | Increase pressure near brain, relax after kill. | Assault, breacher/digger, target priority, friendly-fire safety. |
| `breach_route` | material opened, path exists, door/wall destroyed, tunnel width. | Breach / Open Route. | Spawn defenders near new opening; mark route event. | Digger, breacher, material affordance, path refresh. |
| `hold_area` | actor count, time in zone, enemies contesting, terrain integrity. | Hold. | Sustain pressure, then relax. | Defender squads, repair/fill, shield/support. |
| `recover_object` | object carried, dropped, destroyed, extracted. | Recover. | Spawn pursuit; optional route pressure. | Carry capacity, escort, safe extraction. |
| `sabotage` | target damage, noise/alarm, witnesses, timer. | Sabotage. | Alarm escalation and patrol response. | Low loudness, precision explosives, scanner. |
| `extract` | extraction zone reached, craft landed/exited, cargo survival. | Extract. | Stop major spawns after exit begins, unless chase mission. | LZ score, craft capacity, suppression. |
| `survive_wave` | wave timer, active enemy count, safe brain, debris budget. | Survive. | Build/sustain/relax cycles by wave. | Repair, ammo, med, area denial. |
| `salvage` | item/source recovered, value, risk, optional status. | Salvage. | Optional pressure; feeds retention/debrief. | Salvager role, carry risk, loadout UI. |
| `rescue` | friendly state, route open, treatment, extraction. | Rescue. | Emergency/recovery branch. | Medic, digger, shield, AI rescue policy. |

Objective rule: objective state is not just screen text. Each objective owns progress, fail/win, replay events, UI markers, director effects, save fields, telemetry, and validation tests.

## Destruction-Aware Mission Requirements

| Requirement | Design Rule | Test |
|---|---|---|
| Terrain cannot be the only lock. | If a wall matters, assume the player can dig, blast, fill, collapse, or route around it. | Add a 16-pixel breach between named regions and confirm mission still works. |
| Use true obstacles sparingly. | Elevation, distance, water, alarms, defended spaces, timers, and critical machinery are more honest than arbitrary invincible walls. | Show every non-destructible or critical object in an overlay. |
| Make destruction useful. | Digging, breaching, collapsing, filling, and building should change route, objective, or AI decisions. | `terrain_changed` events must have at least one consumer: path, AI, objective, replay, or UI. |
| Protect the promise. | If an object is critical, tell the player and the validator. Do not secretly make random walls invulnerable. | Critical-object audit in mission validator. |
| Give recovery paths. | A broken route should create a new decision, not a softlock. | Emergency phase can request alternate LZ, replacement tool, or objective fallback. |
| Compact spaces beat empty sprawl. | Cortex-like chaos is most readable in dense, vertical, inspectable maps. | Mission map should fit a playable camera/command overview without hiding key state. |

Teardown's plan/action split is directly useful here: Slice A should support a short prep/buy/route phase, then a player-chosen launch into active combat. One prep-only save slot is worth prototyping because it encourages experimentation without turning combat into rewind spam.

## Equipment Capability Contract

The mission manifest must ask for capabilities, not hard-coded item names. This is where the equipment work connects.

| Mission Need | Capability Tags | Source Notes |
|---|---|---|
| Open soft terrain | `dig.soft`, `dig.tunnel_narrow`, `material.soil`, `bot_use.reviewed`. | [[references/equipment-device-loadout-field-atlas]], [[systems/material-and-mobility-affordance-schema]] |
| Breach hardpoint | `breach.door`, `breach.concrete`, `blast.radius.medium`, `friendly_fire.strict`. | [[references/equipment-ai-behavior-contract]], [[references/equipment-role-cards-slice-a]] |
| Fight mixed infantry | `fight.medium`, `range.mid`, `reload.acceptable`, `fallback.sidearm`. | [[references/equipment-capability-authoring-matrix]], [[references/equipment-overlap-resolution-worksheet-slice-a]] |
| Rescue wounded actor | `support.heal`, `target.ally`, `bot_use.conditional`, `replay.stateful`. | [[spec/body-damage-model]], [[references/equipment-ai-scenarios-slice-a]] |
| Preserve route | `build.fill`, `repair.terrain`, `material.support`, `path.invalidate_on_use`. | [[spec/terrain-material-sandbox-slice-a]], [[spec/equipment-loadout-workbench-slice-a]] |
| Survive delivery risk | `craft.capacity`, `cargo.mass`, `lz.footprint`, `egress.safe`, `cargo_fragility`. | [[engine/loadout-delivery-economy-lifecycle]], [[references/equipment-consumer-traceability-matrix]] |

Loadout UI implication: mission cards should show `required`, `recommended`, `dangerous`, and `manual-only` equipment capabilities. If the player brings no breacher to a breach mission, that is allowed for private play, but the UI must say what consequence they accepted.

AI implication: commander packages and friendly bot assignments can only claim competence for reviewed capability tags. Prototype freely; only mark bot-safe after harness evidence.

## Proof Mission: Breach Contract Slice A

| Component | Slice A Scope |
|---|---|
| Scene | One compact bunker with `LZ Attacker`, `Main Bunker`, `Brain`, `Internal Reinforcements`, one optional salvage cache, and at least two breach routes. |
| Player | One commander/brain actor plus one or two controllable actors; explicit loadout requirements for fight, breach/dig, fallback sidearm. |
| Defender | One commander profile, one protected brain, one guard squad, one reinforcement delivery or internal door response. |
| Objectives | Destroy/protect brain, open breach route, optional salvage, extraction/debrief if recorder is ready. |
| Director | Setup -> prep -> launch -> build_up -> peak_fade/relax -> objective_push -> debrief. |
| Equipment | Use LOAD-A fixtures and role cards. Mission asks for capabilities; UI explains missing capability risks. |
| Replay | Export objective, commander, LZ, delivery, terrain, intensity, AI order, and mission end events. |
| UX | HUD objective marker, command reason panel, LZ danger marker, loadout capability strip, debrief recap. |

## World, Match Grammar, Weather, and Comms Policy Hooks

> [!note] Cross-reference
> Slice A Breach Contract is the simplest mission. The full match grammar (Bunker Defence, Symmetric Arena, FFA, Asymmetric N-Team, Coop-vs-AI, Campaign) lands at later milestones (M7 + M11 + M12) per [[spec/game-modes-and-match-grammar]] / DR-042. Mission director hooks below are the integration anchors.

| Hook | Source-of-truth | Mission director responsibility |
|---|---|---|
| **World binding** | `Mission.world_id` → [[spec/celestial-bodies-and-worlds-model]] | Resolve world manifest at scenario load; pass per-world ambient + gravity + day/night + ore deposits to runtime kernels (`cf-atmos`, `cf-physics`, `cf-mining`, `cf-environment`). |
| **Match grammar** | `Match.kind` (`BunkerDefence`, `SymmetricArena`, `FFA`, `AsymmetricNTeam`, `CoopVsAI`, `Campaign`) → [[spec/game-modes-and-match-grammar]] | Wire team-config-flexibility (1v1 through NvN, FFA, asymmetric, coop); fire `match.victory_condition_met` event chain. AI fills empty player slots per `Match.ai_fill_policy`. |
| **Weather policy** | `Mission.weather_policy` → [[spec/environmental-conditions-model]] / DR-040 | Optional weather override (force dust storm; force RF silence solar flare; force clear). Per-mission scripted weather events. Wire M7.7 weather kernel events into mission objectives (e.g., "extract before dust storm peak"). |
| **Comms policy** | `Mission.comms_policy` → [[spec/comms-voice-and-radio-model]] / DR-043 | Per-team default frequencies; per-mission radio bans (RF silence campaign mission); jamming overlays. Fire `mission.comms_policy_changed` events. Match grammar declares default frequencies + encryption keys; mission director may modify per-phase. |
| **Hazard escalation** | M5.7 hazard package + M5.10 environment aggregation | Mission director can author objectives like `evacuate before radiation peak`, `breach hull while contained`, `extinguish atmosphere fire before reactor breach`. Reasons cause-chained from `EnvironmentSignal.hazard_detected`. |

## Save And Replay Contract

### Save Fields

| Field Group | Required Fields |
|---|---|
| Mission identity | `mission_id`, `manifest_version`, `scene_id`, `run_seed`, `campaign_action_id`, `started_at`. |
| Director | phase, phase timers, intensity samples, spawn budget, pressure caps, recovery windows, RNG stream ids. |
| Objectives | id, status, progress, target ids, last event id, failure cause, UI marker state. |
| Commander | doctrine, known targets, selected LZs, squad tasks, budgets, delivery cooldowns, reason history pointer. |
| Actors | stable ids, team, brain assignment, squad id, role summary, loadout instance ids, script ownership. |
| Equipment | loadout package id, item ids, role-card versions, mutable ammo/state references, dropped item refs. |
| Terrain | terrain digest, dirty regions, mission-critical object state, route/breach flags, path refresh queue. |
| Delivery | queued craft/cargo, ETA, LZ, craft state, cargo ownership, cost debit state. |
| Script hooks | script state blob, deterministic class, declared save keys, migration version. |

### Replay Events

| Event | Required Payload |
|---|---|
| `mission_start` | mission id, seed, scene, teams, package hashes, manifest version. |
| `director_phase_change` | previous phase, next phase, reason, intensity, objective context. |
| `director_intensity_sample` | samples by signal, total pressure, cap, decay state. |
| `objective_start/update/complete/fail` | objective id, status, progress, cause, target refs. |
| `commander_decision.*` | team, decision verb, target, alternatives, reason string, confidence. |
| `lz_scored` | candidate x/y, terrain score, LOS, path length, obstacle height, chosen flag. |
| `delivery_created/landed/lost` | craft, cargo, LZ, ETA, cost, result. |
| `terrain_changed` | region, tool/source, material delta, path invalidation, objective/director consumers. |
| `loadout_capability_check` | mission requirements, squad capability coverage, missing/dangerous/manual-only flags. |
| `mission_end` | winner, cause, objective table, actor losses, optional rewards, replay ids. |

## Consumer Matrix

| Consumer | Mission Director Must Provide |
|---|---|
| AI | Commander reason strings, squad tasks, target/LZ scores, capability summaries, path/terrain events. |
| UI/HUD | Objective markers, mission phase, LZ danger, capability fit, AI intent, failure causes, debrief recap. |
| Modding/workbench | Manifest schema, scene-area validator, capability requirements, script-hook save obligations, package diagnostics. |
| Balancing | Intensity samples, spawn budget, delivery losses, objective timings, item role usage, retry/optional-target stats. |
| Replay/debug | Stable event families, causality parents, snapshots/checksums, terrain deltas, commander decisions. |
| Backend/networking | Mission/package hashes, event-volume estimates, join eligibility, deterministic command candidates. |
| Retention | Same-seed challenge fields, optional objective stats, veteran/salvage outcomes, replay share metadata. |

## Risks And CCCP Bugs To Watch

| Risk | Evidence | Future Guard |
|---|---|---|
| Mission state is implicit script state. | `MetaFight` has no custom save schema. | Every mission manifest declares save fields and passes roundtrip tests. |
| Save fields drift or typo silently. | `BunkerBreach` appears to save `majorAttackTimer` values from `spawnTimer`; `internalReinforcementBudget` reloads as boolean. | Typed schema, migration tests, timer field assertions. |
| Objective UI is transient only. | `BrainCheck` clears and re-adds objective points per frame. | Objectives are persistent entities with UI views and replay events. |
| Scene area strings fail late. | `BunkerBreach` comments list required areas. | Editor/workbench validates named areas before launch. |
| AI appears to cheat or flail. | LZ/path/target logic is mostly hidden. | Export candidate scores, reason strings, and rejected alternatives. |
| Equipment mismatch blocks mission fun. | CCCP loadouts are ordered cargo with implicit item assignment. | Capability-based mission checks plus explicit actor slots. |
| Destruction softlocks the mission. | Any destructible objective can change routes and actor access. | Emergency phase, alternate objectives, path refresh budget, critical-object overlay. |

## Acceptance Tests

| ID | Test | Pass Criteria |
|---|---|---|
| MISSION-A-01 | Manifest validation | Missing scene areas, objectives, teams, capability tags, or save fields fail with actionable diagnostics. |
| MISSION-A-02 | Capability fit | Loadout screen shows required/recommended/missing/dangerous/manual-only mission capabilities before launch. |
| MISSION-A-03 | Prep to launch | Player can buy/lock loadout, choose/confirm LZ, and start mission with a stable `mission_start` event. |
| MISSION-A-04 | Objective lifecycle | Every objective start/update/complete/fail emits HUD marker and replay event with same objective id. |
| MISSION-A-05 | Destructible route | Opening a route changes path/AI/objective state and emits `terrain_changed` with at least one consumer. |
| MISSION-A-06 | LZ scoring | Commander LZ choice records candidate scores, chosen reason, path length, LOS, terrain, and obstacle fields. |
| MISSION-A-07 | Commander target choice | Enemy target selection emits reason string, alternatives, confidence, and doctrine inputs. |
| MISSION-A-08 | Commander package choice | Reinforcement package uses role-card/capability fields and records why each actor/item/craft was chosen. |
| MISSION-A-09 | Squad tasking | Attack/defend/breach/rescue tasks show squad id, members, target, reason, retask timer, and result. |
| MISSION-A-10 | Director pacing | Build/peak/fade/relax transitions are visible in event log and do not secretly modify difficulty amplitude. |
| MISSION-A-11 | AI fairness | Commander knowledge can be inspected; no decision uses hidden omniscient data without a debug flag. |
| MISSION-A-12 | Save roundtrip | Save/reload preserves director phase, objectives, commander state, delivery queues, actors, and mission outcome. |
| MISSION-A-13 | Replay recap | Replaying the run shows mission start, objective changes, commander decisions, LZ decisions, terrain changes, delivery outcomes, and mission end. |
| MISSION-A-14 | Failure explanation | Brain loss, softlock recovery, delivery failure, objective failure, and loadout mismatch each produce player-readable cause text. |
| MISSION-A-15 | Emergency recovery | Destroying/closing the intended route triggers an alternate route, replacement option, or explicit final failure cause. |
| MISSION-A-16 | UI readability | Objective, AI intent, loadout fit, LZ danger, and critical-object markers do not overlap or hide core play. |
| MISSION-A-17 | Mod/workbench roundtrip | Mission manifest opens in workbench, validates scene contracts, and exports unchanged except normalized ordering. |
| MISSION-A-18 | Run bundle | Prototype run exports manifest/events/summary/notes accepted by [[references/prototype-run-bundle-schema]]. |

## First Implementation Tickets

| Ticket | Output |
|---|---|
| MISSION-001 | Mission manifest schema with teams/objectives/director/commander/equipment/save/event sections. |
| MISSION-002 | Scene-area validator and critical-object overlay for the proof bunker scene. |
| MISSION-003 | Director intensity sampler and phase machine with debug/event output. |
| MISSION-004 | LZ scorer sandbox using terrain/LOS/path/obstacle fields and event export. |
| MISSION-005 | Commander AI reason strings for target, LZ, package, squad task, breach, defend, and recovery decisions. |
| MISSION-006 | Breach Contract Slice A scene and mission script/manifest. |
| MISSION-007 | Save/load roundtrip test for mission/director/commander/objectives/delivery. |
| MISSION-008 | Replay/debrief view that groups mission cause, objective outcomes, commander decisions, and loadout performance. |
| MISSION-009 | Loadout-workbench mission strip using capability requirements from [[spec/equipment-loadout-workbench-slice-a]]. |

## Source Trail

- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/MetaFight.lua`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/BunkerBreach.lua`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/LandingZoneMap.lua`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/TacticsHandler.lua`
- `../Cortex-Command-Community-Project/Source/Activities/GAScripted.cpp`
- `../Cortex-Command-Community-Project/Source/Activities/GameActivity.cpp`
- `../Cortex-Command-Community-Project/Source/Entities/Activity.cpp`
- `../Cortex-Command-Community-Project/Source/Entities/Activity.h`
- `../Cortex-Command-Community-Project/Source/Managers/ActivityMan.cpp`
- `../Cortex-Command-Community-Project/Source/Menus/MetagameGUI.cpp`
- Valve, "The AI Systems of Left 4 Dead": https://steamcdn-a.akamaihd.net/apps/valve/2009/ai_systems_of_l4d_mike_booth.pdf
- Valve, "Replayable Cooperative Game Design: Left 4 Dead": https://cdn.fastly.steamstatic.com/apps/valve/2009/GDC2009_ReplayableCooperativeGameDesign_Left4Dead.pdf
- Daniel Brewer, "Managing Pacing in Procedural Levels in Warframe": http://www.gameaipro.com/GameAIProOnlineEdition2021/GameAIProOnlineEdition2021_Chapter07_Managing_Pacing_in_Procedural_Levels_in_Warframe.pdf
- Travis Hoffstetter, "The Art and Science of Pacing and Sequencing Combat Encounters": https://www.gamedeveloper.com/design/the-art-and-science-of-pacing-and-sequencing-combat-encounters
- Dennis Gustafsson, "Teardown design notes": https://blog.voxagon.se/2020/11/05/teardown-design-notes.html
- GameDeveloper, "How beautiful voxels laid the way for Teardown's heist-y framework": https://www.gamedeveloper.com/design/how-beautiful-voxels-laid-the-way-for-i-teardown-s-i-heist-y-framework
- 80.lv, "Teardown Developer Breaks Down Multiplayer And Voxel Destruction Tech": https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech
