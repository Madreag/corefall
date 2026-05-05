---
type: spec
status: prototype-reqs
ready_when: "A role-tagged equipment/loadout model, field atlas, comparison UI, AI item metadata, delivery-risk model, mod schema, loadout test fixtures, and source-position drill-downs pass LOAD-A-01..LOAD-A-16, LOAD-FIELD-01..06, and LOAD-FIELD-SOURCE-01..06."
feeds:
  - DR-003
  - DR-004
  - DR-006
  - DR-008
  - DR-009
  - DR-014
---

← [[spec/index|spec section]] · [[spec/prototype-implementation-backlog-slice-a|implementation backlog A5]] · [[spec/equipment-role-card-renderer-slice-a|role-card renderer Slice A]] · [[spec/equipment-loadout-workbench-slice-a|loadout workbench Slice A]] · [[spec/chassis-armor-mechs-and-origins|chassis/armor/mechs/origins]] · [[spec/accessibility-comfort-slice-a|accessibility/comfort Slice A]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[references/equipment-role-card-renderer-view-slice-a|renderer view]] · [[references/equipment-corpus-cccp|CCCP equipment corpus]] · [[references/equipment-cccp-field-map|CCCP field map]] · [[references/equipment-device-loadout-field-atlas|device/loadout field atlas]] · [[references/equipment-source-anchored-device-snapshots|source-anchored snapshots]] · [[references/equipment-comparable-design-patterns|comparable design patterns]] · [[references/equipment-role-design-deep-dive|role design deep dive]] · [[references/equipment-capability-authoring-matrix|capability matrix]] · [[references/equipment-ai-behavior-contract|AI behavior contract]] · [[references/equipment-ai-summary-seed-slice-a|AI summary seed]] · [[references/equipment-role-records-slice-a|role records]] · [[references/equipment-consumer-traceability-matrix|consumer traceability]] · [[references/equipment-consumer-traceability-slice-a|generated trace report]] · [[references/equipment-trace-tab-view-slice-a|trace-tab view]] · [[references/equipment-source-trace-slice-a|source trace]] · [[references/equipment-role-cards-slice-a|role cards]] · [[references/equipment-overlap-audit-slice-a|overlap audit]] · [[references/equipment-overlap-resolution-worksheet-slice-a|overlap worksheet]] · [[references/equipment-schema-and-overlays|schema/overlays]] · [[references/equipment-overlay-review-matrix|review matrix]] · [[references/equipment-manual-overlay-patches|manual patches]] · [[references/equipment-overlay-merged-preview|merged preview]] · [[references/equipment-provenance-workbench-view|provenance view]] · [[references/equipment-loadout-fixtures-slice-a|LOAD-A fixtures]] · [[references/equipment-ai-scenarios-slice-a|AI scenarios]] · [[references/equipment-package-diagnostics-slice-a|package diagnostics]] · [[references/content-loader-graph-cccp|loader graph]] · [[engine/content-module-loading-lifecycle|content/module lifecycle]] · [[engine/loadout-delivery-economy-lifecycle|loadout/delivery]] · [[systems/damage-equipment-and-items|damage/equipment]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[systems/material-and-mobility-affordance-schema|material/mobility schema]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[spec/package-builder-workbench-slice-a|package-builder Slice A]]

# Equipment And Loadout Model

> [!summary] Purpose
> Define equipment roles, loadout slots, AI-readable item metadata, delivery-risk fields, UI comparison fields, mod/schema requirements, and acceptance tests. This page turns the old stub into a build-facing bridge between item design, bot competence, buy/loadout UX, replay/debug, and modding.

> [!important] Product stance
> Equipment should create tactics, stories, and expressive problem solving. The goal is not a spreadsheet-perfect armory. The goal is a readable sandbox where every item changes combat, terrain, squad behavior, delivery risk, or recovery options in a way players and bots can understand.
>
> Armor, powered armor, mechs, robot frames, android shells, and species/origin gear extend the same equipment contract. They must have local slots, mass/protection tradeoffs, condition stages, AI reason labels, UX warnings, replay events, and workbench provenance instead of becoming passive RPG stat bonuses.

> [!tip] Implementation handoff
> [[spec/prototype-implementation-backlog-slice-a]] makes this an explicit A5 milestone. The first implementation must consume the CCCP field atlas, source trace, generated role cards, fixture loadouts, package diagnostics, AI behavior labels, and overlap audit rather than rebuilding a separate item taxonomy.

## Slice A Question

Can a player build a small squad for a destructible mission, understand what each actor can solve, predict delivery risk, and trust bots to use their tools without memorizing hidden `.ini` groups or engine quirks?

## Shared Consumer Contract

Equipment data has to serve the whole game, not just the catalog screen. A field is only worth making canonical when at least one runtime system and one review/tooling surface can use it.

| Contract Object | Owns | Primary Consumers | Hard Rule |
|---|---|---|---|
| `item_definition` | Immutable authored facts: identity, role tags, handling, projectile/effect links, source package, declared capabilities. | AI seed, buy/loadout UI, package builder, balance review, backend compatibility. | Never store mutable ammo/condition/owner state here. |
| `runtime_item_instance` | Owner, ammo, heat/cooldown, condition, current script state, attachments, dropped/pickup state. | Actor controller, AI runtime, replay recorder, save/load, networking probes. | Every stateful change that affects play needs an event or snapshot path. |
| `chassis_definition` | Origin compatibility, body/chassis sockets, armor layers, module hardpoints, pilot capacity, mass, mobility profile, damage-stage vocabulary. | Actor controller, loadout UI, body model, AI, mission director, replay, package builder. | Mechs, robots, powered armor, and species bodies cannot bypass the shared loadout/damage contract. |
| `equipment_condition` | Intact/impaired/critical/disabled/destroyed state, source event, repairability, smoke/spark/audio state, behavior penalty. | HUD, body silhouette, AI refusal, replay/debrief, salvage/repair loop. | Damaged gear becomes invisible frustration instead of tactical information. |
| `loadout_template` | Delivery craft, explicit actors, slots, package ids, budget, mission-role intent, legacy source order. | Buy/loadout frontend, mission preflight, backend/session compatibility, replay manifest. | Do not rely on hidden ordered `AddCargoItem` semantics after import; preserve order only as provenance. |
| `role_card` | Human-readable tactical meaning: best at, bad at, range, terrain consequence, support value, handling, risk. | Catalog row, detail drawer, squad summary, balance review, tutorials, death recap. | Every catalog-visible item must say what decision it changes or be marked skin/legacy/internal. |
| `ai_summary` | Target classes, range model, danger model, material fit, reason labels, claim state, scenario refs. | Bot runtime, AI harness, debug overlay, package diagnostics, replay reports. | A fun manual item can stay playable; a bot-default item needs harness-backed evidence. |
| `ui_projection` | Dense scan fields: icon, cost/mass, range band, role chips, bot trust, warning badges, accessibility labels. | Frontend catalog, HUD item panel, controller/keyboard navigation, workbench trace tab. | No important item meaning can be color-only or hidden inside raw stats. |
| `package_diagnostic` | Source path, field provenance, warning ids, mode verdicts, first fix action, manual patch id. | Modding workbench, package builder, release checklist, usage ledger. | Private experiments warn; published/bot-default claims can block until required evidence exists. |
| `balance_row` | Role overlap, cost/mass/supply pressure, terrain power, handling cost, AI confidence, mission impact. | Design review, overlap worksheet, playtest reports, progression tuning. | Cost alone is not a valid balance answer for a dominant tool. |
| `replay_event` | Item id, actor owner, action, reason label, target/material context, package hash, result/checksum. | Recorder, death recap, bug reports, deterministic probes, backend compatibility. | If item behavior can cause confusion, death, terrain change, or desync, it needs a replay label. |
| `mission_requirement` | Required/recommended capabilities and bot-safe/manual policy for breach, heal, scout, anti-craft, delivery, recovery. | Mission director, buy/loadout preflight, AI commander, tutorial/challenge generator. | Missions ask for capabilities, not hard-coded item names. |

This is the rule behind the generated trace-tab and source-trace work: one discovered CCCP field should not become five unrelated meanings. It should flow from source evidence to canonical item data, then into AI, UI, modding, balance, replay, backend, and mission requirements with provenance attached.

## Implementation-Facing Role Record

The role card cannot be only prose. The first buildable `role_record` should be a joinable data row that starts from a source-backed item definition, carries explicit runtime and UI meanings, and gives every consumer the same facts. Generated role cards, source traces, AI summaries, and fixture imports already prove the shape; the next workbench and AI harness should consume this record directly.

| Record Slice | Required Fields | Source Pressure | Primary Consumers | Failure If Shallow |
|---|---|---|---|---|
| Identity and provenance | `item_id`, `package_id`, `display_name`, `catalog_visibility`, `source_path`, `inheritance_chain`, `source_confidence`, `manual_patch_id`, `warning_ids`. | `PresetName`, `CopyOf`, loader graph rows, generated source trace, manual overlay patches. | UI detail drawer, package diagnostics, backend/session manifest, usage ledger, balance review. | A designer cannot tell whether a role claim is direct, inherited, inferred, scripted, or hand-patched. |
| Handling and slot pressure | `mass`, `bulk`, `hands_required`, `dual_wield_policy`, `support_requirement`, `support_offset`, `grip_strength`, `held_hitability`, `drop_fragility`, `actor_body_constraints`. | `HeldDeviceType`, `OneHanded`, `DualWieldable`, `Supportable`, `NoSupportFactor`, `GripStrengthMultiplier`, `GetsHitByMOsWhenHeld`. | Actor controller, HUD slot warnings, buy/loadout comparison, bot stance, replay item-loss labels. | Heavy weapons, sidearms, shields, and one-arm fallback collapse into damage/cost rows. |
| Chassis and armor pressure | `origin_compatibility`, `armor_slot`, `coverage_arc`, `coverage_part`, `protection_profile`, `module_socket`, `pilot_state`, `route_profile`, `repair_tags`, `condition_stage`. | Future CHASSIS-A prototype, body part contracts, armor/mech design, powered-armor/mech module slots. | Loadout builder, body model, AI rescue/repair, HUD/squad panel, replay/debrief, progression. | Armor and mechs become unreadable stat bundles or dominant upgrades. |
| Action and effect model | `primary_verb`, `secondary_verb`, `target_rule`, `activation_context`, `effect_profile`, `scripted_verbs`, `failure_reasons`. | `HDFirearm` firing loop, Lua `ScriptPath`, Medikit/Grapple/Scanner/Constructor scripts, Unreal GAS ability/effect/task split. | AI evaluator, command overlay, mod schema, package workbench, replay event taxonomy. | Support, mobility, sensor, revive, and construction items become special cases that only humans understand. |
| Projectile and area shape | `projectile_model`, `projectile_count`, `muzzle_velocity`, `lifetime`, `spread_shape`, `blast_radius`, `falloff`, `target_filters`, `friendly_fire_policy`. | `Magazine`, `Round`, `AIBlastRadius`, `AILifeTime`, `AIFireVel`, `AIPenetration`; OpenRA `Weapon -> Projectile -> Warheads`. | Bot target choice, danger overlays, balance overlap, replay causality, networking byte budget. | Explosive/refusal logic and visible danger UI drift apart from the actual projectile data. |
| Terrain and material consequence | `dig_profile`, `fill_profile`, `material_affordances`, `dirty_region_policy`, `path_invalidation`, `collapse_risk`, `actor_collision_effect`. | Digger particles, Concrete Sprayer fill, Constructor modes, A1 material-grid/checksum/probe events. | Terrain sandbox, mission director, AI pathing, buy/loadout role coverage, replay/debug. | Destructible-space tools are balanced like weapons instead of route-making/building tools. |
| AI policy | `bot_claim_state`, `target_classes`, `range_model`, `material_fit`, `danger_radius`, `utility_inputs`, `blackboard_keys`, `reason_labels`, `scenario_refs`, `harness_status`. | CCCP AI projectile summaries, Lua item-choice behavior, generated AI summary seed, AI-H scenario fixtures. | Bot runtime, AI harness, debug overlay, bot-trust badge, package diagnostics. | Bots misuse gear silently, or everything risky gets mislabeled as permanently manual-only. |
| UI projection | `short_role_text`, `best_at`, `bad_at`, `icon_tags`, `comparison_fields`, `warning_badges`, `accessibility_labels`, `same_input_actions`, `source_tab_rows`. | Buy menu category/cost/mass state, LOAD-W renderer rows, trace-tab rows, accessibility Slice A. | Catalog row, detail drawer, actor columns, squad summary, controller/keyboard navigation. | The player gets names and stat bars without knowing what decision the item changes. |
| Balance and overlap | `role_signature`, `overlap_group_id`, `cost_pressure`, `mass_pressure`, `ammo_pressure`, `terrain_power`, `delivery_burden`, `mission_fit`, `counterplay`. | Generated overlap audit, role-card renderer view, source snapshots for rifles/sidearms/explosives/tools. | Design review, tuning sheets, mission fixtures, progression rewards. | Cost becomes the only balancing knob for dominant tools. |
| Replay, backend, and session | `event_families`, `causality_parent`, `source_snapshot_id`, `determinism_class`, `sync_relevance`, `package_hash_relevance`, `loadout_snapshot_ref`. | Run-bundle schema, replay recorder Slice A, LOAD-A/LOAD-W smoke events, Factorio prototype/runtime split. | Recorder, death recap, bug reports, session compatibility, package mismatch UI. | Item-caused deaths, desyncs, and package errors cannot be reproduced. |
| Mission capability | `capability_tags`, `required_by_missions`, `recommended_for_missions`, `bot_safe_for_missions`, `manual_only_policy`, `delivery_role`. | DeliveryCreationHandler roles, mission director Slice A, loadout fixtures. | Mission preflight, commander AI, tutorial/challenge generator, buy/loadout workbench. | Missions ask for hidden item names instead of visible capabilities. |

### Consumer Acceptance Matrix

Every consumer should be able to prove the same `role_record` is usable. This is the minimum acceptance language for the next spec/prototype pass.

| Consumer | Must Read | Must Emit Or Show | Acceptance Test Hook |
|---|---|---|---|
| AI runtime and harness | Role record, AI summary seed, current actor/order/target/material context, source confidence. | `ai_item_choice`, `ai_item_refusal`, `ai_item_result`, top rejected alternatives, claim-state delta. | `AI-EQ-SUMMARY-*`, `AI-H-LOAD-*`, `LOAD-FIELD-05`. |
| Buy/loadout UI | Role record, UI projection, fixture slots, diagnostics, trace rows, overlap rows. | Catalog row, detail drawer, actor slot column, squad capability strip, bot trust badge, source tab. | `LOAD-A-*`, `LOAD-R-*`, `LOAD-W-*`, `ACC-A-*`. |
| Modding/package workbench | Source path, field provenance, schema version, warning ids, first fix action, package mode. | Dev/local/published verdicts, bot-default blockers, manual patch suggestions, open source target. | `PACK-014C`, `LOAD-FIELD-01`, `LOAD-FIELD-SOURCE-*`. |
| Balance review | Role signature, overlap group, cost/mass/ammo/delivery pressure, terrain power, AI confidence. | Keep/split/skin/legacy/internal decision, fixture request, tuning note. | `LOAD-010`, `LOAD-COMP-*`, overlap worksheet rows. |
| Replay/debug | Stable item id, owner, action, reason label, source snapshot id, material/body/delivery result, checksum. | Replay event row, death recap cause, source-jump/debug marker, deterministic probe output. | `REC-A-LOAD`, `MAT-TOOL-01`, run-bundle checker. |
| Backend/session | Package id/hash, schema version, loadout snapshot, item visibility, bot-default eligibility. | Join eligibility verdict, package mismatch warning, replay compatibility note. | `BACK-A-*`, `MOD-A-EQUIP-01`, `LOAD-FIELD-06`. |
| Mission director | Capability tags, bot-safe/manual policy, delivery role, risk and missing-capability labels. | Required/recommended/missing capability strip, commander warning, fixture request. | `MISSION-A-*`, `LOAD-A-03`, `LOAD-FIELD-03`. |

### Role Record Questions

Every catalog-visible row should answer these questions before it becomes a settled spec commitment. A private prototype can skip answers temporarily, but the workbench must label the missing pieces.

| Question | Required Evidence | First Place To Surface It |
|---|---|---|
| What player decision changes if this item exists? | `best_at`, `bad_at`, role signature, overlap status. | Catalog row and detail drawer. |
| What body or handling constraint does it impose? | Hands/support/mass/grip/hitability fields and actor body state. | Actor slot warning and HUD item panel. |
| What map/material consequence can it create? | Dig/fill/breach/profile fields, material fit, path invalidation policy. | Material preview and mission preflight. |
| When should a bot choose or refuse it? | AI summary seed, target/material/danger data, reason labels, harness refs. | Bot trust badge, debug overlay, replay event. |
| What should a modder fix first? | Missing required fields, source confidence, package-mode verdict, first fix action. | Package diagnostics and source tab. |
| What proves it worked or failed? | Item event family, result checksum, source snapshot id, causality parent. | Replay/debug report and run bundle. |
| What backend/session facts must survive? | Package hash, schema version, resolved loadout snapshot, item visibility. | Session manifest and compatibility warning. |

## Evidence Stack

### Local Cortex / CCCP Evidence

| Evidence | Local Path | Design Lesson |
|---|---|---|
| Held devices are typed as weapon/tool/shield/bomb and expose one-hand, dual-wield, support, grip, pickup, loudness, explosive, and hitability fields. | `Source/Entities/HeldDevice.h:16-21`, `:136-219`, `:238-366`, `:373-418` | Role tags should not be just UI labels. They must map to physical handling, arm requirements, pickup rules, loudness, and damage survivability. |
| Firearms read magazine, sounds, rate of fire, activation/reload timing, full-auto, spread, no-support factor, recoil, muzzle, shell, and offsets. | `Source/Entities/HDFirearm.cpp:171-248`, `:650-960` | Weapon cards need handling stats and failure explanations, not just damage/cost. |
| Rounds and magazines cache AI velocity, penetration, blast radius, lifetime, tracer cadence, particle count, and dig strength estimates. | `Source/Entities/Magazine.cpp:37-55`, `:76-88`, `:118-168`; `Source/Entities/Round.cpp:37-50`, `:74-91` | Future items need explicit bot summaries. Bots should not infer everything from simulation at runtime. |
| Loadouts are ordered cargo lists; first actor gets items before/after it until the next actor boundary. | `Source/Entities/Loadout.cpp:40-88`, `:122-200` | Future UI should use explicit actor columns/slots instead of order-sensitive hidden assignment. |
| Buy menu tracks category tabs, saved loadouts, craft, cost, mass, passenger count, allowed/always/prohibited items, and cart actions. | `Source/Menus/BuyMenuGUI.h:123-157`, `:181-246`; `BuyMenuGUI.cpp:180-251` | Loadout UX already has the raw mechanics. It needs role filters, compatibility badges, and risk previews. |
| Activity delivery helper defines infantry roles and group names: Light, Medium, Heavy, CQB, Scout, Sniper, Grenadier, Engineer; weapon/tool groups include primary, secondary, light, heavy, sniper, CQB, explosive, diggers, breaching. | `Data/Base.rte/Activities/Utility/DeliveryCreationHandler.lua:46-55`, `:136-149`, `:181-197`, `:712-937` | Role kits should graduate from implicit group names into a formal schema shared by UI, AI, missions, and mods. |
| Device group scan shows the active corpus is already role-tagged: 84 `Weapons`, 61 `Weapons - Primary`, 36 `Weapons - Light`, 24 `Weapons - Heavy`, 14 `Weapons - Sniper`, 14 `Weapons - Explosive`, 24 `Tools`, 7 `Tools - Diggers`, 7 `Tools - Breaching`, 5 `Shields`. | `rg "AddToGroup =" Cortex-Command-Community-Project/Data --glob '*.ini'` | Existing content can seed the first taxonomy, but it is too coarse for AI and UX. |
| Generated equipment corpus scanned the active CCCP data tree: 576 `.ini` files, 3512 top-level preset blocks, 184 device blocks, 138 buyable/implicit-buyable devices, 90 loadout blocks, 107 magazines, 111 rounds. It also exposes metadata coverage: descriptions 89%, gold 90%, mass 99%, groups 90%, loudness 4%, inherited explicit AI round fields 11%, plus direct/inherited/inferred/missing field provenance. | [[references/equipment-corpus-cccp]], `research_tools/equipment_corpus.py` | LOAD-001 has a first pass with common `CopyOf` inheritance and warning details. Next work should extend toward fuller PresetMan parity and turn fixture results into stricter schema rules. |
| First schema/overlay pass now exists for Base, Coalition, and Ronin: JSON Schema, corpus JSON, and 72-item overlay seed with explicit internal review includes. | [[references/equipment-schema-and-overlays]], `cortext_command_vault/references/equipment.schema.json`, `cortext_command_vault/references/equipment-overlay-seed.base-coalition-ronin.json` | LOAD-002/LOAD-003 have seed artifacts. Next work is manual review, package-builder validation, AI harness use, and prototype-backed field tightening. |
| The generated overlay now has a manual review matrix: warning severity, critical item triage, internal/payload classification queue, bot competence gates, package-builder diagnostic mapping, and nine LOAD-A fixture candidates. | [[references/equipment-overlay-review-matrix]] | LOAD-003 has an actionable review queue. The next concrete output is fixture JSON/YAML plus manual overlay patches for inherited items, internal payloads, support tools, and explosives. |
| The first fixture file now instantiates nine LOAD-A loadouts with actual Base/Coalition/Ronin item IDs, expected warnings, manual badges, and test coverage. | [[references/equipment-loadout-fixtures-slice-a]], [equipment-loadout-fixtures.slice-a.json](../references/equipment-loadout-fixtures.slice-a.json) | LOAD-004/005/006/007 can start from shared data instead of each prototype inventing its own item set. |
| The first manual patch layer now covers 14 high-priority records: replacement/catalog policy, internal turret/payload classification, scripted support/mobility/sensor contexts, and launcher bot-label gating. | [[references/equipment-manual-overlay-patches]], [equipment-overlay-manual-patch.base-coalition-ronin.json](../references/equipment-overlay-manual-patch.base-coalition-ronin.json), `research_tools/equipment_overlay_check.py` | Prototypes should load generated seed, then manual patch, then fixture data; generated files stay reproducible. |
| A merged preview now applies the manual patch and produces fixture diagnostic reports for all nine LOAD-A fixtures. | [[references/equipment-overlay-merged-preview]], [equipment-overlay-merged.preview.json](../references/equipment-overlay-merged.preview.json), `research_tools/equipment_overlay_merge.py` | LOAD-A consumers can use one patch-applied artifact while the seed and manual patch remain separately reviewable. |
| The first provenance workbench view turns fixture item rows into consumer-facing diagnostics: 33 LOAD-A item rows, 25 unique fixture items, 0 missing fixture items, attention-field counts, warning fix queue, source highlights, and AI/UI/modding/balancing/replay implications. | [[references/equipment-provenance-workbench-view]], [equipment-provenance-workbench-view.slice-a.json](../references/equipment-provenance-workbench-view.slice-a.json), `research_tools/equipment_provenance_workbench_view.py` | LOAD-001C gives the buy/loadout UI and package workbench an immediate provenance panel instead of asking designers to read raw JSON. |
| AI-H equipment scenario seeds now convert the nine LOAD-A fixtures into bot tests. | [[references/equipment-ai-scenarios-slice-a]], [equipment-ai-scenarios.slice-a.json](../references/equipment-ai-scenarios.slice-a.json), `research_tools/equipment_ai_scenarios.py` | AI work can test weapon choice, breach tools, support use, explosive refusal, target classes, pickup value, and negative loadout preflight using the same item IDs and diagnostics as the loadout UI. |
| Generated AI summary seeds now turn all 106 role-card rows into bot decision contracts. | [[references/equipment-ai-summary-seed-slice-a]], [equipment-ai-summary-seed.slice-a.json](../references/equipment-ai-summary-seed.slice-a.json), `research_tools/equipment_ai_summary_seed.py` | AI, UI, package diagnostics, balance, replay, and backend/session code now share `claim_state`, decision inputs, blackboard keys, reason labels, event families, scenario links, and source confidence before any item is promoted to bot-safe. |
| Generated role records now join those separate artifacts into one implementation-facing item object. | [[references/equipment-role-records-slice-a]], [equipment-role-records.slice-a.json](../references/equipment-role-records.slice-a.json), `research_tools/equipment_role_record_projection.py`, [[research-log/2026-05-04-equipment-role-record-projection]] | LOAD-W-21/22 now have 106 checkable role records carrying identity/provenance, handling, action/effect, projectile/area, terrain/material, AI policy, UI projection, balance/overlap, replay/backend/session, and mission capability slices. |
| Package-builder diagnostic expected output now turns fixture warnings into dev/local/published mode verdicts and bot-assignment verdicts. | [[references/equipment-package-diagnostics-slice-a]], [equipment-package-diagnostics.slice-a.json](../references/equipment-package-diagnostics.slice-a.json), `research_tools/equipment_package_diagnostics.py` | Workbench tests can prove warnings are visible, package-mode gates differ, and unsafe bot defaults are blocked without blocking private player experimentation. |
| Generated role cards translate the merged overlay and selected non-catalog CCCP examples into 106 system-readable item cards. | [[references/equipment-role-cards-slice-a]], [equipment-role-cards.slice-a.json](../references/equipment-role-cards.slice-a.json), `research_tools/equipment_role_cards.py` | LOAD-009 now has a repeatable artifact for buy/loadout UI, AI harness, package-builder diagnostics, balance, replay/backend, and modding consumers. |
| Generated overlap audit finds 10 player-catalog role-signature overlap groups covering 42 catalog items; 3 are high-risk. | [[references/equipment-overlap-audit-slice-a]], [equipment-overlap-audit.slice-a.json](../references/equipment-overlap-audit.slice-a.json), `research_tools/equipment_role_cards.py` | LOAD-010 now has concrete balance/UX pressure: medium assault rifles and sidearms need stronger handling, recoil, ammo, AI-policy, economy, mission-role, or cosmetic/skin decisions. |
| Role-card renderer requirements translate those artifacts into build-facing UI/workbench/AI/replay/balance tests. | [[spec/equipment-role-card-renderer-slice-a]] | LOAD-009/LOAD-010 now have a renderer contract, visibility policy, special-case item checks, overlap-resolution rules, and LOAD-R-01..LOAD-R-12 acceptance tests. |
| Renderer view model generates concrete LOAD-R fixture output. | [[references/equipment-role-card-renderer-view-slice-a]], [equipment-role-card-renderer-view.slice-a.json](../references/equipment-role-card-renderer-view.slice-a.json), `research_tools/equipment_role_card_renderer_view.py` | The first renderer fixture exposes 63 unique player-catalog rows, 106 workbench rows, 5 detail drawer examples, 10 overlap rows, 9 fixture summaries, and a LOAD-R coverage map. |
| Equipment loadout workbench requirements turn generated artifacts into an interactive prototype target. | [[spec/equipment-loadout-workbench-slice-a]], [[research-log/2026-05-04-equipment-loadout-workbench-slice-a]], [[prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke]], [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]] | LOAD-W-01..LOAD-W-22 now define fixture selector, catalog browser, actor columns, detail drawers, diagnostics, trace tab, role-record projection, overlap compare, bot trust, export preview, keyboard/controller navigation, and accessibility checks. First input evidence covers fixture-tab click, focused-button keyboard activation, Tab-to-fixture focus, ArrowRight focus movement, Enter/Space activation, fixture-control focus restore, and selected-item role-record state; full workbench traversal remains open. |
| Overlap worksheet turns duplicate-role pressure into candidate role splits and fixture queue. | [[references/equipment-overlap-resolution-worksheet-slice-a]] | LOAD-010 now has item-by-item next identities for assault rifles and sidearms plus medium-risk differentiator queues for explosives, shotguns, heavy automatics, digger/shovel, snipers, and shields. |
| Digger rounds use terrain-removal particles, orphan-terrain cleanup fields, shell damage particles, and high round counts. | `Data/Base.rte/Devices/Tools/Digger/Digger.ini:33-178`, `:185-220` | Terrain tools are weapons in the data model; the spec needs terrain affordances and material fit. |
| Concrete Sprayer fires wet concrete particles and has builder-style use cases. | `Data/Base.rte/Devices/Tools/ConcreteSprayer/ConcreteSprayer.ini:5-125`, `:132-220` | Support tools can add terrain, not just remove it. Buy/loadout UX must show repair/fill capability. |
| Grapple Gun and Constructor are tool outliers with mobility/build-order behavior. | `Data/Base.rte/Devices/Tools/GrappleGun/GrappleGun.ini`; `Data/Base.rte/Devices/Tools/Constructor/Constructor.ini` | Future taxonomy needs mobility and construction roles, not only combat roles. |
| AI Lua already queries weapon/tool status, blast radius, penetration, dig strength, trajectories, and leader weapon comparison. | `Data/Base.rte/AI/SharedBehaviors.lua:1089-1098`; `NativeHumanAI.lua:386-400`; `HumanBehaviors.lua:623`; `AHuman.cpp:925-959` | AI metadata should be made explicit and testable through [[spec/ai-trust-harness-slice-a]]. |
| Deeper AI/equipment behavior trace shows current AI already switches by item group, projectile summary, blast safety, material/door context, pickup path score, support timing, and leader weapon similarity. | [[references/equipment-ai-behavior-contract]], `Data/Base.rte/AI/NativeHumanAI.lua:386`, `:540`, `:572`, `:771`; `HumanBehaviors.lua:569`, `:1014`, `:1543`; `SharedBehaviors.lua:457`, `:1062` | LOAD-006 should become a shared behavior contract: item choice, refusal, reason labels, UI badges, package diagnostics, replay events, and balance rows must all read the same fields. |
| CCCP field map translates C++/Lua/INI fields into future schema consumers. | [[references/equipment-cccp-field-map]] | Use the field map when deciding whether an item field belongs to immutable item definitions, runtime state, loadout templates, mission requirements, UI summaries, AI summaries, package diagnostics, replay, or backend compatibility. |
| Device/loadout field atlas converts exact CCCP fields into a build-facing consumer contract. | [[references/equipment-device-loadout-field-atlas]], [[research-log/2026-05-04-equipment-device-loadout-field-atlas]] | Use the atlas when deciding how `HeldDevice`, `HDFirearm`, `Magazine`, `Round`, `Loadout`, delivery-role, and buy/loadout UI fields should appear in AI reason labels, UI/workbench trace tabs, balance rows, replay events, backend compatibility, and mod schemas. |
| Source-anchored device snapshots show literal CCCP values for representative devices and loadouts. | [[references/equipment-source-anchored-device-snapshots]], [[research-log/2026-05-04-equipment-source-anchored-device-snapshots]] | Use the snapshots when a design or implementation pass needs concrete values for item detail drawers, source inspectors, AI reason labels, material-tool events, replay rows, or balance comparisons. |
| Content module loading lifecycle now explains how `.rte` loader behavior produces trustworthy item facts: official/mod/userdata load order, include stack, `CopyOf`, source paths, preset collisions, script reload, zip import, and scan-folder caveats. | [[engine/content-module-loading-lifecycle]], [[research-log/2026-05-04-content-module-loading-lifecycle]] | Equipment role cards, AI item claims, loadout UI badges, package diagnostics, balancing rows, replay events, and backend compatibility should consume resolved item definitions with source provenance instead of raw filenames or group strings. |
| Equipment role deep dive maps concrete CCCP devices and durable design references into role-card rules. | [[references/equipment-role-design-deep-dive]] | Role taxonomy must be based on gameplay behavior, terrain consequence, handling, risk, bot policy, and evidence, not only item names or catalog groups. |
| Equipment capability authoring matrix synthesizes CCCP fields, generated role-card coverage, Soldat/OpenSoldat, Arma Reforger, Ravenfield, and Core equipment references into authoring tiers and role rules. | [[references/equipment-capability-authoring-matrix]], [[research-log/2026-05-04-equipment-capability-authoring-matrix]] | Item roles should be usable by four consumers at once: AI choosing safely, UI explaining tradeoffs, mod/workbench validation, and balancing/replay analysis. Private experiments stay unblocked; published/bot-default claims need evidence. |
| Equipment consumer traceability matrix maps CCCP/comparable fields into AI, UI, workbench/package, balancing, replay/debug, backend, and prototype-test obligations. | [[references/equipment-consumer-traceability-matrix]], [[research-log/2026-05-04-equipment-consumer-traceability]] | LOAD-A/LOAD-R/LOAD-W/AI-H/PACK/REC work should share item ids, warning ids, claim states, consumer-impact labels, and event contracts instead of inventing separate equipment meanings. |
| Generated LOAD-011 traceability report proves where each role-card row currently reaches downstream consumers. | [[references/equipment-consumer-traceability-slice-a]], [equipment-consumer-traceability.slice-a.json](../references/equipment-consumer-traceability.slice-a.json), `research_tools/equipment_consumer_traceability.py` | The equipment spec can now name real coverage gaps: AI is mostly partial until harness-backed summaries exist, replay/backend is partial until item reason events exist, and fixture breadth still needs more catalog coverage. |
| Generated LOAD-W-010 trace-tab view turns that traceability into UI-ready workbench data. | [[references/equipment-trace-tab-view-slice-a]], [equipment-trace-tab-view.slice-a.json](../references/equipment-trace-tab-view.slice-a.json), `research_tools/equipment_trace_tab_view.py` | The future loadout/workbench can render item trace rows, fixture tabs, package diagnostic trace rows, gap badges, and open targets without inventing a separate item meaning layer. |
| Generated content loader graph turns CCCP module/include/source context into a machine fixture: 10 official modules, 508 INI files, 498 include edges, 3064 top-level preset blocks, 48085 `CopyOf` refs, 458 script paths, 0 missing includes, 0 wrong-case include/script paths, and 5 duplicate same-module preset keys. | [[references/content-loader-graph-cccp]], [content-loader-graph-cccp.json](../references/content-loader-graph-cccp.json), `research_tools/content_loader_graph.py` | The equipment workbench should join role cards to loader source positions so AI/UI/modding/balancing/replay rows can explain package, include, source line, collision pressure, and confidence. |
| Generated LOAD-FIELD-SOURCE trace now joins 106 role-card rows to corpus field provenance, loader file/include context, duplicate preset pressure, trace-tab refs, open targets, and source confidence. | [[references/equipment-source-trace-slice-a]], [equipment-source-trace.slice-a.json](../references/equipment-source-trace.slice-a.json), `research_tools/equipment_source_trace.py` | The workbench can open exact source lines, explain why 104 rows remain medium confidence, and route missing critical fields or duplicate-source pressure into AI, package-builder, replay, and balance follow-up queues. |

### Comparable And External Evidence

See [[references/equipment-comparable-design-patterns]] for the source-backed comparison matrix that ties CCCP, OpenSoldat, OpenRA, Unreal GAS, Godot, Unity, and Factorio into one equipment-field contract.

| Source | Pattern / Lesson | Requirement Added Here |
|---|---|---|
| [[comparables/opensoldat-local-audit]] plus local `weapons.ini` pass | Fast shooter weapons need clear roles, readable reticle/state, bot weapon decisions, and demo/replay hooks. OpenSoldat also makes feel fields explicit: speed-sensitive damage, hitbox modifiers, fire interval, reload, bink, movement accuracy, spread, recoil, push, inherited velocity, and bullet style. | Role cards must show handling/feel fields and not rely on DPS. Add `LOAD-COMP-01` to compare handling identity across rifles, sidearms, shotguns, launchers, and melee. |
| [[comparables/openlierox-local-audit]] | Compact destructible arenas get longevity from distinct weapons, modded content, rope/mobility tools, and effect-chain authoring. | Mobility and effect-chain tools need explicit role cards, not just weapon slots. |
| [[comparables/the-powder-toy-local-audit]] | Material behavior is most useful when tools and UI expose what will happen to matter, heat, pressure, and terrain. | Terrain tools need dig/fill/build/scan previews, dirty-region events, and material checksum output. |
| GameDeveloper, "7 Suggestions for Having Interesting Weapons in Shooters" | Avoid overlapping weapons; different weapons should require different tactics/skills. Limited loadouts are useful only when encounter design benefits from them. | Every item card needs "best at" and "bad at"; overlap warnings require a visible reason to keep both items. |
| Bohemia Interactive CfgAmmo / CfgWeapons / Targeting references | Mature AI weapon data separates ammo usage, strict target filters, min/mid/max ranges, threat classes, lock/targeting systems, and player-facing visibility. | `ai_summary` fields are required before bot-safe claims. |
| OpenRA Weapons docs | Weapon definitions separate range, min range, valid/invalid targets, projectile types, warheads, damage falloff, relationships, and debug overlay colors. | Split projectile, effect, target filters, terrain/resource effects, and debug overlays into typed item fields instead of one overloaded damage row. Add `LOAD-COMP-02`. |
| Unreal Gameplay Ability System | Actor-owned abilities, effects, attributes, tags, and tasks are separate concepts with explicit activation/effect lifecycles, prediction, and replication constraints. | Scripted support, medikit, grapple, shield, deployable, and command tools should be actions/effects with event contracts, not only weapon stat rows. |
| Godot Resources and Unity ScriptableObject Manual | Shared item data should be authored once, reused by runtime objects, and edited/inspected by tools. | Split immutable item definitions from mutable runtime item instances. |
| Factorio data lifecycle and prototype docs | Startup prototype data, runtime state, mod load order, migrations, and machine-readable docs are distinct layers. | Package/import tooling should separate resolved item definitions from runtime state, preserve source/provenance, and generate schema docs for creators. Add `MOD-A-EQUIP-01`. |
| Steam Inventory Schema and Item Tags | Item definitions, bundles, dynamic properties, and tags should be structured separately so economy/frontend/backend systems do not scrape gameplay text. | Economy/frontend tags stay separate from simulation fields; monetization must not distort the core item model. |

## CCCP Field Families To Preserve

The future spec should not flatten CCCP equipment into generic `damage`, `range`, and `cost` stats. The current code/data already separates handling, firing, projectile, AI, loadout, buy menu, and scripted behavior. Our schema should preserve those seams so AI, UI, modding, balance, replay, and backend systems can consume the same facts.

| Field Family | Actual CCCP Fields / Behaviors | Future Canonical Fields | Why It Matters |
|---|---|---|---|
| Physical handling | `Mass`, `OneHanded`, `DualWieldable`, `Supportable`, `SupportOffset`, `UseSupportOffsetWhileReloading`, `GripStrengthMultiplier`, `GetsHitByMOsWhenHeld`, `SharpLength`. | `mass`, `hands_required`, `support_requirement`, `support_offset`, `reload_handling`, `grip_strength`, `hitbox_policy`, `aim_reveal_bonus`. | Actor feel, arm-loss fallback, bot stance, UI slot compatibility, and replay explainers all depend on handling, not only damage. |
| Firearm cadence | `RateOfFire`, `ActivationDelay`, `DeactivationDelay`, `ReloadTime`, `FullAuto`, `Reloadable`, `DualReloadable`, `OneHandedReloadTimeMultiplier`, `NoSupportFactor`. | `cadence`, `windup_ms`, `winddown_ms`, `reload_ms`, `fire_mode`, `one_hand_reload_penalty`, `unsupported_accuracy_penalty`. | Distinct weapons need different timing windows and bot confidence states; overlap warnings should test cadence, not only role names. |
| Firearm offsets and feedback | `MuzzleOffset`, `EjectionOffset`, `ShellEjectAngle`, `ShellSpreadRange`, `ShellAngVelRange`, `ShellVelVariation`, `RecoilScreenShakeAmount`, `ShakeRange`, `SharpShakeRange`. | `muzzle_socket`, `shell_socket`, `shell_feedback`, `screen_shake`, `hip_spread`, `aimed_spread`, `camera_feedback`. | Reticle/HUD and replays need visible reasons for misses, recoil, shell clutter, and control feel. |
| Magazine/ammo | `Magazine`, `RoundCount`, `RTTRatio`, `RegularRound`, `TracerRound`, `Discardable`, negative/infinite ammo behavior, full-capacity cache. | `ammo_source`, `magazine_capacity`, `tracer_pattern`, `ammo_mode`, `discard_policy`, `resupply_policy`. | Loadout UI and AI need ammo pressure, reload windows, and supply burden. Backend/replay needs deterministic magazine state. |
| Projectile/round behavior | `Particle`, `ParticleCount`, `FireVelocity`, `InheritsFirerVelocity`, `Separation`, `LifeVariation`, `Shell`, `ShellVelocity`. | `projectile_prefab`, `projectile_count`, `muzzle_velocity`, `inherits_actor_velocity`, `spread_separation`, `lifetime_variance`, `shell_output`. | Weapon cards should expose spread shape, projectile travel, and effect-chain risk. AI should not guess whether a weapon is hitscan-like, arcing, or swarm-like. |
| AI projectile summary | `AIBlastRadius`, `AILifeTime`, `AIFireVel`, `AIPenetration`, `EstimateDigStrength`, `GetBulletAccScalar`, Lua trajectory comparison. | `ai_blast_radius`, `ai_projectile_lifetime`, `ai_velocity`, `ai_penetration`, `ai_dig_strength`, `ai_gravity_scalar`, `trajectory_class`. | Bots need explicit target/safety/material summaries and reason labels; this is the cheapest path to great solo AI. |
| Groups and delivery roles | `AddToGroup`, `Weapons - Primary`, `Weapons - Secondary`, `Weapons - Light`, `Weapons - Heavy`, `Weapons - Explosive`, `Tools - Diggers`, `Tools - Breaching`, delivery helper roles `Light`, `Medium`, `Heavy`, `CQB`, `Scout`, `Sniper`, `Grenadier`, `Engineer`. | `role_tags`, `slot_tags`, `mission_role_tags`, `capability_tags`, `delivery_role`, `bot_default_policy`. | Existing content already has a vocabulary, but it must be normalized so loadout UX, mission needs, and bot loadouts agree. |
| Ordered loadout cargo | `DeliveryCraft` plus ordered `AddCargoItem` entries; first actor receives following items until next actor boundary. | `delivery.craft`, explicit `actors[]`, explicit `slots`, `slot_role`, `assignment_policy`, `fallback_policy`. | Future UI should avoid hidden order-sensitive assignment; every actor column should show why an item belongs in that slot. |
| Buy/loadout UX state | Category tabs, craft rows, passenger count, mass, cost, saved loadouts, cart actions, allowed/always/prohibited item sets. | `catalog_filter`, `craft_capacity`, `passenger_count`, `mass_budget`, `gold_cost`, `saved_loadout`, `eligibility_policy`, `cart_event`. | The buy screen should become a planning tool: mission fit, delivery burden, bot trust, and package warnings must sit next to cost and mass. |
| Scripted equipment | Lua items such as Medikit, Grapple Gun, Constructor, Disarmer, Scanner, concrete/digger tools, pie actions, scripted target checks. | `scripted_use_context`, `target_rule`, `failure_reason`, `bot_support_level`, `replay_state`, `manual_only_reason`. | Support/mobility/build tools are where the game can become more than a shooter. They need first-class cards even when their behavior lives in scripts. |
| Package/workbench provenance | `PresetName`, `CopyOf`, source `.ini` path, inherited fields, inferred fields, manual overlay patches, warning details. | `source_path`, `field_provenance`, `inheritance_chain`, `manual_patch_id`, `warning_id`, `first_fix_action`, `package_mode_verdict`. | Modding and future backend/package compatibility depend on knowing which facts are direct, inherited, inferred, missing, or manually reviewed. |

Design rule: role tags are allowed to be broad, but the stored data cannot be vague. If the item is a digger, its card should name material fit, tunnel profile, stand-off, path invalidation, AI confidence, and replay events. If it is a sidearm, its card should name one-arm reliability, draw/fallback use, ammo pressure, and target class. If it is a scripted support tool, its card should name target rules, failure cases, bot support, and package diagnostics.

## Concrete CCCP Role Cards

Use [[references/equipment-source-anchored-device-snapshots]] for concrete source values, [[references/equipment-role-design-deep-dive]] for the hand-authored design pass, [[references/equipment-capability-authoring-matrix]] for the cross-source capability/authoring matrix, and [[references/equipment-role-cards-slice-a]] for the generated role-card dataset. The short version is that several CCCP items prove why equipment cards need behavior, terrain, AI, provenance, authoring-tier, and overlap fields.

| CCCP Example | Why A Simple Category Fails | Spec Translation |
|---|---|---|
| Light/Medium/Heavy Diggers | They are `HDFirearm` devices, but their real value is controlled terrain removal through particles, sharpness, spread, ammo count, and orphan-terrain cleanup. | Treat as route-making terrain tools with material fit, tunnel profile, path invalidation, bot stand-off, and material overlay preview. |
| Concrete Sprayer | It is authored like a firearm but functions as a build/fill/reinforcement tool and is not normally buyable in the scanned Base catalog. | Treat support/build tools as first-class loadout roles, not hidden special cases. |
| Grapple Gun | It uses scripts, guide arrows, pie actions, infinite claw ammo, and player-control-specific input behavior. | Treat mobility tools as scripted role cards with anchor rules, failure states, bot confidence, and replay state. |
| Medikit | It is a limited-use scripted firearm shell that raycasts to self/ally targets, heals wounds/health, can revive a dead actor clone, and refunds ammo on failed use. | Treat medical support as target-rule plus rescue policy metadata, not just "healing item." |
| Rocket Launcher | It combines heavy handling, projectile/emitter behavior, `AIBlastRadius`, slow reload, craft/armor threat, and terrain blast risk. | Treat heavy explosive weapons as breach/craft-threat/danger-radius items with bot refusal reasons and reticle danger overlays. |

## Role Card Requirements

Every catalog-visible item should generate a role card that can be read by humans and systems.

| Role Card Area | Required Fields | Why |
|---|---|---|
| Decision changed | `best_at`, `bad_at`, `primary_verb`, `secondary_verb`, `skill_hook`. | Prevents filler items and overlapping weapons. |
| Tactical shape | range band, setup time, cadence, area shape, line-of-sight rule, counterplay. | Connects item design to encounter design and AI choices. |
| Terrain consequence | material affordances, dig/fill/breach profile, collapse risk, path update policy. | Makes destructible-space tools visible and testable. |
| Handling | mass, bulk, one/two-hand rules, support requirement, recoil, movement impact, arm-loss fallback. | Keeps physical bodies and item choices coupled. |
| AI policy | target classes, min/max range, danger radius, friendly-fire policy, bot confidence, refusal labels. | Makes bots trustworthy and debuggable. |
| Mod/package policy | schema version, source paths, scripted-use context, package mode, warning ids, fixture requirements. | Lets creators validate items without custom code. |
| Replay/backend | event tags, determinism class, sync relevance, package hash relevance, causality parents. | Keeps replay, sessions, and package compatibility explainable. |

Every role card detail drawer should expose a compact source-snapshot tab for reviewed items. That tab should show the literal source fields that justify the role claim, the normalized fields they become, whether each value was direct/inherited/scripted/manual, and which consumers use it. Without this, future AI/UI/balance agents will keep re-reading raw `.ini` and Lua files or trusting stale summaries.

The first generated role-card artifact currently includes 106 cards: 72 merged-overlay cards plus 34 important non-catalog examples such as Concrete Sprayer, payloads, turret guns, and hidden/internal explosive devices. The non-catalog rows are intentional; the buy menu should hide them, but package diagnostics, replay/event labeling, migration tooling, and modding workbench UX still need to know what they are.

## Overlap Audit Requirements

LOAD-010 is now concrete enough to treat overlap as a build-facing design check.

| Current Finding | Examples | Required Response |
|---|---|---|
| High-risk medium assault overlap. | Old Stock Battle Rifle, Coalition Assault Rifle, Compact Assault Rifle, Ronin AK-47, Ronin M16A2. | Give each a visible handling/recoil/cadence/ammo/economy/mission role, make some skins/legacy variants, or remove them from default catalog. |
| High-risk sidearm overlap. | .357 Magnum, Desert Eagle, MP5K, UZI, Old Stock Pistol, Auto Pistol, Beretta 93R, Luger P08. | Make backup roles distinct: quick draw, armor punch, suppressive panic fire, stealth, one-arm reliability, ammo economy, or dual-wield identity. |
| Medium-risk explosive overlap. | Grenade/mine/bomb bandoliers across Base, Coalition, and Ronin. | Add fuse, throw arc, terrain channel, hazard output, bot refusal, and material-affordance differences before default bot use. |
| Medium-risk digger/tool overlap. | Medium Digger and Shovel. | Compare tunnel width, dig rate, material fit, melee value, bot stance, and delivery/loadout burden in the terrain sandbox. |

Design rule: a role-overlap warning is not a ban. It is a demand for a visible reason to keep both items. If the reason is faction flavor, cosmetic history, or nostalgia, mark it honestly so the player and future spec do not mistake it for a new tactical role.

Use [[references/equipment-overlap-resolution-worksheet-slice-a]] as the working LOAD-010 sheet. It assigns initial statuses such as `needs_role_split`, `skin_or_legacy_candidate`, `manual_only_until_ai`, and `mission_fixture_needed` so high-risk overlap can move into tests instead of staying as a vague design warning.

## Equipment Design Reference Matrix

| Reference | Takeaway | Requirement Added Here |
|---|---|---|
| Weapon design patterns research | Classify weapons by gameplay behavior, affordances, consequences, level patterns, and NPC relationships. | Role tags need behavioral meaning, not just real-world weapon labels. |
| GameDeveloper weapon-design article | Limited loadouts, rockets, overlap, ability depth, handling depth, and reliable weapons all have design consequences. | Every item card needs "best at" and "bad at"; no universal reliable weapon without an explicit design reason. |
| Arma 3 AI config references | AI equipment use benefits from explicit ammo usage, target classes, threat values, suppression/danger, sight/hearing, and skill data. | `ai_summary` fields are required before bot-safe claims. |
| Unreal Gameplay Tags | Tags need a dictionary, hierarchy, and query rules. | Tags are namespaced and validated; numeric safety fields remain typed. |
| Unity ScriptableObject | Shared item data should be authored once and referenced by many runtime objects. | Split immutable item definitions from mutable item instances. |
| Steam Inventory Schema / Tags | Item definitions and tags can support frontend/economy/backend properties, bundles, and extended game fields. | Economy/profile data stays separate from simulation fields; monetized inventory must not drive launch gameplay. |

## Product Rules

| Rule | Implication |
|---|---|
| Every item must change a decision. | If two items only differ by damage number, merge them or give one a distinct terrain, handling, AI, delivery, or support role. |
| Player choice must stay legible. | The buy/loadout screen shows why a squad can breach, heal, scout, survive, carry, or fail. |
| Bots need item summaries. | AI uses explicit role/range/risk/material fields before falling back to expensive simulation. |
| Delivery is part of balance. | Mass, passenger count, craft footprint, LZ risk, cargo fragility, and exit safety are visible before purchase. |
| Mods inherit the same contract. | Modded items must declare enough metadata for UI, AI, replay, package compatibility, and validation. |
| Do not lock fun behind grind. | Progression can add variety and mastery goals, but core combat/dig/heal/delivery verbs must work early. |

## Mission Capability Consumer

[[spec/mission-director-slice-a]] now treats missions as consumers of equipment data. Mission manifests should request capabilities such as `dig.soft`, `breach.door`, `fight.medium`, `support.heal`, `build.fill`, `craft.capacity`, or `fallback.sidearm` instead of hard-coded item names.

| Mission Requirement | Equipment Model Response |
|---|---|
| Required capability | Loadout workbench shows covered, missing, dangerous, manual-only, and bot-safe states. |
| Recommended capability | UI explains likely advantage without blocking private experiments. |
| Bot-safe capability | Requires reviewed item role fields plus AI-H/AI-EQ evidence before default bot assignment. |
| Destruction capability | Links material fit, tool profile, path invalidation, danger radius, and replay events. |
| Delivery capability | Links craft capacity, cargo mass, LZ footprint, exit risk, cargo fragility, and cost. |
| Modded capability | Package diagnostics verify tags, typed fields, provenance, and declared script behavior. |

This is the practical reason for the field atlas and trace-tab work: AI, UI, modding, balancing, replay, backend, and mission authoring all need the same item facts. A breach mission should be able to say "the squad lacks reviewed hard-breach capability" and point to the exact item fields or missing overlay review that caused the warning.

## Role Taxonomy

### Actor Roles

| Role | Primary Job | Required Gear | Optional Gear | AI Must Know |
|---|---|---|---|---|
| Assault | Fight through mixed infantry. | Reliable primary, sidearm, light explosive. | Shield, medikit. | Engage range, reload window, cover preference, friendly-fire risk. |
| Breacher | Open doors/walls and clear hardpoints. | Breaching weapon/tool, explosive, short weapon. | Scanner, shield. | Which materials can be breached, danger radius, where to stand before firing. |
| Engineer | Dig, repair, reinforce, build routes. | Digger/constructor/concrete or fill tool. | Light weapon, mobility tool. | Material fit, support/collapse risk, path update after edit. |
| Medic | Recover wounded actors and stabilize veterans. | Medikit or medical dart/tool. | Smoke/cover item, sidearm. | Target priority, danger threshold, rescue path, when to retreat. |
| Scout | Reveal terrain, find threats, mark routes. | Scanner, mobility/light weapon. | Grapple, suppressed sidearm. | Sensor range, stealth/noise, path confidence, report events. |
| Sniper | Long-range precision and overwatch. | High sharp length/range weapon. | Spotter sensor, sidearm. | Line of sight, overpenetration, target priority, relocation timer. |
| Grenadier | Area denial and terrain shock. | Launcher/grenades/mines. | Sidearm, shield. | Blast radius, throw arc, friendly-fire exclusion, collapse risk. |
| Heavy | Suppression and craft/armor threat. | Heavy weapon, high-stability actor. | Shield, ammo pack. | Setup time, recoil/stability, target class, ammo economy. |
| Pilot | Use powered armor, mechs, drones, and vehicle-scale tools without trapping the squad in bad terrain. | Compatible chassis/mech, sidearm, ejection/rescue plan. | Repair kit, sensor, beacon. | Entry/exit state, route fit, cockpit risk, module damage, abandon/rescue rules. |
| Mechanic | Repair armor, weapons, tools, mech modules, robots, androids, and delivery craft under pressure. | Repair tool, diagnostic sensor, sidearm. | Spare module, smoke/cover, light digger. | Repair target priority, reactor/EMP risk, when to tow/abandon. |
| Commander | Control, buff, rally, order clarity. | Sensor/command device, reliable sidearm. | Drone, beacon, shield. | Order propagation, priority labels, retreat/rescue triggers. |
| Salvager | Recover enemy/dropped equipment and resources. | Scanner, carry pack, light weapon. | Grapple, repair tool. | Pickup value, carry risk, route to safe zone. |

### Item Archetypes

| Archetype | Examples | Required Metadata | UX Card Focus |
|---|---|---|---|
| Primary firearm | SMG, rifle, shotgun, sniper, launcher. | role tags, range, penetration, spread, recoil, reload, ammo, loudness. | "Best at", "Bad at", range band, handling, noise, bot skill. |
| Sidearm | pistol, compact SMG, melee backup. | one-hand, dual-wield, quick draw, minimum range. | Backup reliability, arm-loss fallback, low-mass value. |
| Digger | light/medium/heavy diggers, constructor dig mode. | material strength, tunnel profile, orphan cleanup, heat/noise. | Material fit, tunnel width, speed, bot confidence. |
| Breacher | rocket, charge, grenade launcher, door tool. | blast radius, structural damage, fuse/arming, friendly-fire rules. | Breach rating, danger radius, collapse risk. |
| Repair/fill | concrete sprayer, constructor spray, foam. | fill material, support strength, cure time, hazard block. | Can bridge, reinforce, seal, trap, or block. |
| Mobility | grapple, jetpack assist, rope/tether. | anchor materials, max length, retract, actor mass limits. | Traversal value, failure cases, AI competence. |
| Shield | riot/combat shield, deployable barrier. | coverage arc, durability, one-hand/two-hand, mass, stance. | Protection vs mobility and weapon compatibility. |
| Armor layer | helmet, torso plate, arm guard, leg armor, powered armor segment. | coverage part/arc, protection profile, durability stages, mass, repair tags, mobility/handling penalty. | Protection vs speed, local body consequence, damaged-stage warning. |
| Mech / chassis module | mech arm weapon, leg actuator, sensor head, reactor, cockpit, cargo clamp. | socket, pilot requirement, module condition, mass, route profile, heat/power, repair/salvage tags. | Capability gain, module failure, pilot risk, route/delivery burden. |
| Sensor | scanner, noise detector, material probe. | reveal radius, target types, line-of-sight, stealth impact. | What it reveals and how long info persists. |
| Medical/support | medikit, medical dart, stim, revive kit. | heal type, target rules, cooldown, danger constraints. | Stabilize, revive, patch limb, AI rescue priority. |
| Consumable explosive | grenade, mine, bomb, remote charge. | trigger mode, fuse, radius, terrain channels, pickup/drop safety. | Throw arc, fuse state, safe range, bot avoidance. |
| Delivery craft | dropship, rocket, crate, pod. | mass capacity, passenger count, footprint, exit mode, crash profile. | ETA, LZ risk, cargo survival, queue state. |
| Deployable | turret, beacon, bunker system, supply cache. | placement rules, terrain support, team ownership, package trust. | Placement preview, power/ammo, pickup/reclaim. |

## Loadout Slot Model

Future loadouts should be explicit actor-slot documents, not an order-sensitive cart.

| Slot | Required? | Examples | Constraint |
|---|---|---|---|
| `actor` | Yes | Coalition Light, Dummy Heavy, custom bot. | Defines body mass, inventory capacity, arm count, faction, AI skill. |
| `origin` | Yes | human, android, robot frame, augmented, alien/biological prototype. | Defines wound/repair model, vulnerabilities, story hooks, and compatible chassis/armor. |
| `chassis` | Yes | light body, heavy body, android shell, robot frame, powered armor base, mech hull. | Defines sockets, mass, mobility, collision/profile, pilot capacity, and body/module damage model. |
| `head_armor` | Optional | helmet, sensor hood, mech head shell. | Protects head/sensors; may affect visibility, hearing, or sensor fidelity. |
| `torso_armor` | Optional | vest, plate carrier, reactor casing, powered chest. | Protects core/life support/power; adds mass/heat/profile. |
| `arm_armor` | Optional | bracer, servo sleeve, manipulator guard. | Protects grip/tool/weapon use; may slow aim/reload. |
| `leg_armor` | Optional | greave, powered leg, track pod. | Protects mobility; affects speed, jump, terrain sinking, route fit. |
| `primary` | Role-dependent | rifle, shotgun, heavy gun, launcher. | One two-handed or compatible one-handed pair. |
| `secondary` | Recommended | pistol, compact weapon, rescue sidearm. | Must remain usable if primary arm/support is lost. |
| `terrain_tool` | Mission-dependent | digger, constructor, breacher, concrete sprayer. | Must declare material affordances. |
| `support_tool` | Optional | medikit, scanner, shield, beacon. | Must declare AI use context. |
| `consumables` | Optional | grenades, mines, charges. | Limited by safety, mass, and inventory handling. |
| `mobility` | Optional | grapple, jump assist, tether. | Must declare anchor/material and AI confidence. |
| `mech` | Optional / mission-dependent | scout walker, breach mech, cargo exosuit, repair frame. | Pilot/entry state, module slots, route profile, delivery burden, damage stages. |
| `mech_modules` | Mech-dependent | arm weapon, drill, shield, sensor, reactor, cargo clamp. | Must declare socket, power/heat, condition stages, repair/salvage rules. |
| `delivery` | Squad-level | dropship, rocket, pod, crate. | Capacity, footprint, queue, LZ risk. |

### Loadout Document Shape

```yaml
loadout_id: base.engineer.breach.v1
display_name: Breach Engineer
mission_role_tags: [Engineer, Breach, Repair]
budget:
  gold: 210
  mass_kg: 165
  passenger_slots: 1
delivery:
  craft: Base.rte/Drop Ship MK1
  risk_profile: standard
actors:
  - actor: Coalition.rte/Light Soldier
    doctrine: follow_then_breach
    slots:
      primary: Coalition.rte/Compact Assault Rifle
      secondary: Base.rte/Pistol
      terrain_tool: Base.rte/Light Digger
      support_tool: Base.rte/Concrete Sprayer
      consumables:
        - Coalition.rte/Frag Grenade
```

## Item Metadata Contract

| Field Group | Fields | Consumers |
|---|---|---|
| Identity | `item_id`, `display_name`, `package_id`, `role_tags`, `faction_tags`, `tech_tier`, `rarity_or_availability`. | Buy/loadout UI, package builder, backend compatibility. |
| Physical handling | `mass`, `one_handed`, `dual_wieldable`, `supportable`, `grip_strength`, `stance_offsets`, `sharp_length`, `inventory_bulk`, `drop_fragility`. | Actor controller, HUD, loadout comparison, delivery risk. |
| Chassis / armor | `origin_id`, `chassis_id`, `armor_slots`, `coverage_parts`, `protection_profile`, `damage_stages`, `repair_tags`, `module_sockets`, `pilot_capacity`, `route_profile`, `power_heat_profile`. | Body damage, armor/mech HUD, AI rescue/repair, mission fit, workbench. |
| Condition / degradation | `condition_stage`, `behavior_penalties`, `smoke_spark_state`, `jam_or_fault_rules`, `repairability`, `salvage_state`, `source_event_id`. | Runtime item instance, HUD warnings, AI refusal/switching, replay/debrief, salvage economy. |
| Offensive effect | `damage_channels`, `range_band`, `rate_of_fire`, `reload_time`, `ammo_model`, `projectile_count`, `penetration`, `blast_radius`, `recoil`, `spread`, `loudness`. | Combat sim, AI, HUD, replay, balance. |
| Terrain effect | `material_affordances`, `dig_strength`, `fill_material`, `repair_strength`, `collapse_risk`, `orphan_cleanup`, `hazard_output`. | Material overlay, AI pathing, mission design. |
| Support effect | `heal_type`, `revive_rules`, `sensor_types`, `shield_coverage`, `deployable_rules`, `mobility_anchor_rules`. | Squad AI, command overlay, buy/loadout UI. |
| AI summary | `ai_min_range`, `ai_max_range`, `ai_target_classes`, `ai_use_contexts`, `ai_danger_radius`, `ai_friendly_fire_policy`, `ai_material_fit`, `ai_competence_required`, `claim_state`, `blackboard_keys`, `required_reason_labels`, `required_events`, `scenario_refs`, source confidence. | AI trust harness, bot runtime, debug overlay, buy/loadout trust badges, replay/backend, package diagnostics. |
| UX summary | `short_role_text`, `warnings`, `comparison_bars`, `icon_tags`, `color_independent_symbols`, `explain_failure_messages`. | HUD, buy/loadout, workbench diagnostics. |
| Replay/network | `event_tags`, `determinism_class`, `sync_relevance`, `package_hash_relevance`, `causality_parent_events`. | Recorder, backend, package builder. |
| Economy | `gold_cost`, `owned_item_policy`, `salvage_value`, `repair_cost`, `resupply_cost`, `delivery_risk_weight`. | Campaign/economy, buy menu, retention. |

## Role Data By Consumer

Item roles are not only category labels. Each role field must be consumable by runtime code, tools, and design review without rewriting the meaning in five places.

| Consumer | Reads | Must Decide | Failure If Missing |
|---|---|---|---|
| AI runtime and harness | `ai_summary`, role-card `claim_state`, `role_tags`, range band, projectile velocity, blast/danger radius, material fit, friendly-fire policy, blackboard keys, reason labels, event families, bot competence. | Which item to use, when to switch, when to refuse, what target class is valid, whether pickup is worth it, and which harness/replay evidence can promote the claim. | Bots silently misuse gear or designers mark everything manual-only. |
| Buy/loadout UI | `short_role_text`, icon tags, mass, cost, range, terrain/support effects, bot competence, warning details, provenance labels. | How to filter, compare, explain missing squad roles, and preview delivery risk. | Player sees names and numbers but not why a loadout works. |
| Modding/workbench | `field_provenance`, source paths, warning ids, schema version, package diagnostics, manual overlay policy. | Whether a creator must add tags, AI summaries, source evidence, package compatibility, or validation tests. | Mods require one-off fixes and cannot be trusted by UI or AI. |
| Balancing | role tags, damage/terrain channels, ammo model, mass, gold cost, reload/supply, delivery risk, role overlap. | Whether an item has a unique job, whether a kit is too broad, whether cost/mass/ammo pressure matches its power. | A universal weapon or universal kit flattens the sandbox. |
| Mission/scenario authoring | mission role requirements, material affordances, delivery constraints, threat classes, recovery/support coverage. | What the mission expects the player to bring and how to warn about missing capabilities. | Failure feels hidden because the mission asked for an ability the UI never surfaced. |
| Replay/backend/package compatibility | stable `item_id`, package id/hash, slot labels, event tags, determinism/sync class, schema version. | How to reproduce a loadout, diagnose a replay, reject incompatible packages, or explain a desync. | Replays and online/session tooling cannot explain item-caused failures. |

> [!important] Provenance rule
> The UI can show an `inferred`, `manual`, or generated AI-summary value, but the AI harness and balance tests cannot treat that value as settled. Use [[references/equipment-ai-summary-seed-slice-a]] for bot decision seeds, [[references/equipment-provenance-workbench-view]] for fixture-level warning/provenance rows, and [[references/equipment-source-trace-slice-a]] for role-card-to-source confidence, loader context, include parents, duplicate-source pressure, and exact source jump targets before any item role becomes an authoritative spec commitment.

## Equipment Run-Evidence Requirements

The equipment page must stay deeper than taxonomy. Actual CCCP device/loadout fields, durable design references, and generated role cards are useful only if future runs prove how those fields reach AI, UI, modding, balancing, replay, and backend consumers.

| Evidence Need | Required Run Evidence | Linked Contract |
|---|---|---|
| AI item choice | `ai_item_choice` event with actor id, order context, selected item id, role-card id, score inputs, target context, selected reason, and rejected alternatives. | [[references/equipment-ai-behavior-contract]], [[systems/replay-determinism-and-run-evidence]] |
| AI item refusal | `ai_item_refusal` event with refused item id, reason label, source field or missing field, package mode, and first fix action. | [[references/equipment-ai-behavior-contract]], [[references/equipment-package-diagnostics-slice-a]] |
| AI summary seed parity | Every role-card row has a generated AI summary seed with `claim_state`, decision inputs, blackboard keys, reason labels, required events, source confidence, and required tests. | [[references/equipment-ai-summary-seed-slice-a]], [[references/equipment-consumer-traceability-slice-a]] |
| Item result | `ai_item_result` or player item-result event with expected effect, actual result, terrain/body/logistics consequence, interruption/failure reason, and claim-state delta. | [[spec/ai-trust-harness-slice-a]], [[spec/replay-recorder-slice-a]] |
| Loadout ownership | Run manifest and events identify explicit actor slots, item ids, package ids, source fixture id, and selected delivery craft. | [[references/equipment-loadout-fixtures-slice-a]], [[references/prototype-run-bundle-schema]] |
| Terrain tools | Digger/breacher/fill tools emit terrain material probe, dirty chunk, material changed, path invalidation, bounded route/collision query, bounded actor-hull response, overlay, and byte-count evidence. | [[systems/material-and-mobility-affordance-schema]], [[systems/replay-determinism-and-run-evidence]] |
| Mod/package traceability | Workbench diagnostics name source path, field provenance, warning ids, package mode verdict, affected consumers, and any manual patch id. | [[references/equipment-consumer-traceability-matrix]], [[references/equipment-trace-tab-view-slice-a]] |
| Balance overlap | Similar item runs record cadence, range, handling, ammo pressure, terrain effect, bot confidence, and role-overlap status. | [[references/equipment-overlap-audit-slice-a]], [[references/equipment-overlap-resolution-worksheet-slice-a]] |
| Source-position confidence | Workbench rows expose source state, loader module/order, include depth, source fields, provenance counts, duplicate source hits, and open targets for source/loader/role-card/trace-tab views. | [[references/equipment-source-trace-slice-a]], [[references/content-loader-graph-cccp]] |

Design rule: an item role is not ready to become authoritative just because it appears in a role card. It becomes spec-grade when the run bundle can show the item entering a loadout, being selected or refused, producing an effect, surfacing the right UI/debug labels, and keeping enough replay/backend metadata to reproduce the outcome.

### Current Prototype Evidence

[[prototypes/actor-feel-lab-a1-ui-smoke]] is the first checked LOAD-A runtime-smoke bridge. It imports the generated `engineer_breach` fixture, selects actual fixture rows, mutates a tiny material grid, and emits item/tool/refusal/checksum/path-refresh/path-probe/collision-probe/actor-collision-response events that future AI, UI, modding, balance, replay, backend, and mission systems can consume.

| Fixture Item | CCCP-Derived Fields Used | Runtime Smoke Output | Still Missing |
|---|---|---|---|
| `Base.rte/Light Digger` | `item_id`, `slot`, `archetype`, `primary_verb`, `material_fit`, `bot_claim_state`, `required_reason_labels`, `source_path`. | `ai_item_selected`, `ai_tool_choice`, `ai_material_probe`, `terrain_dirty_region`, `terrain_carve_mask`, `path_material_refresh`, `path_probe_result`, `collision_probe_result`, `actor_collision_response`, `ai_item_result`; 9 dirt cells removed; checksum `fnv1a32:d3ee7cf9` -> `fnv1a32:cc173c72`; bounded route flips blocked -> open; bounded collision flips solid -> clear; actor hull flips stopped -> moved; visible `dig.soft` mark and probe overlay. | Integrated movement collision, chunk snapshots, global path planner result, dig-rate balance, AI-H scoring. |
| `Base.rte/Constructor` | `item_id`, `slot`, `archetype`, `material_fit`, `review_status`, `bot_claim_state`, `source_path`. | `ai_item_selected`, `ai_tool_choice`, `terrain_dirty_region`, `path_material_refresh`, `path_probe_result`, `collision_probe_result`, `actor_collision_response`, `terrain_fill_applied`; 12 air cells become repair foam; checksum `fnv1a32:cc173c72` -> `fnv1a32:07df70ba`; bounded route flips open -> blocked; bounded collision flips clear -> solid; actor hull flips moved -> stopped; visible `fill_gap` mark and probe overlay. | Integrated support collision response, cover value, global path planner result, scripted-use confidence. |
| `Coalition.rte/Timed Explosive Bandolier` | `item_id`, `slot`, `archetype`, `material_fit`, `bot_claim_state`, `review_status`, refusal labels. | `ai_item_selected`, `ai_item_refusal` with `manual_recommended_required`, `friendly_fire_risk`, and `collapse_risk_high`; no terrain cells change; visible manual-warning mark. | Fuse/placement/throw simulation, danger overlay, ally/self/collapse scoring, bot-safe harness proof. |
| `Base.rte/Old Stock Pistol` | `item_id`, `slot`, `archetype`, sidearm/fallback role, bot claim state, source path. | Available in fixture inventory and selection list; sidearm reload/fire path exists in prototype. | Role-specific sidearm handling, one-arm fallback, ammo pressure, weapon-switch AI event, overlap balance. |

This is intentionally not enough to promote the fixture to bot-safe or balance-ready. It is enough to lock one implementation rule: equipment pages, workbenches, AI harnesses, replay events, backend schemas, mission requirements, and material/path/collision systems must use the same item ids, role fields, warning labels, material deltas, checksums, bounded query/response results, and source/provenance rows.

## AI Item Contract

Use [[references/equipment-ai-behavior-contract]] as the detailed source of truth. The short rule: AI item choice must be explainable in the debug overlay, buy/loadout UI, package diagnostics, replay reports, and balance review.

[[references/equipment-ai-summary-seed-slice-a]] is now the generated bridge from role cards into bot decision data. Runtime code should consume it as a seed contract, then replace `seeded_risky_pending_harness`, `manual_or_supervised_until_harness`, or `not_default_catalog` states with harness-backed results only after replay evidence exists.

| AI Question | Required Data | Failure / Refusal Label |
|---|---|---|
| Can this weapon affect the current target? | `target_classes`, range band, projectile velocity/lifetime, trajectory type, line of sight, penetration/material fit. | `target_out_of_range`, `trajectory_blocked`, `no_valid_target_class`. |
| Can this item breach, dig, fill, scan, heal, anchor, or support the current objective? | `material_affordances`, `scripted_use_context`, target rule, support/fill/dig profile, current order. | `material_too_hard`, `needs_breacher`, `wrong_tool_for_order`, `scripted_tool_unproven`. |
| Is this safe near allies, self, craft, or critical terrain? | Danger radius, friendly-fire policy, fuse/arming window, muzzle clearance, self-risk, ally/craft positions. | `friendly_fire_risk`, `target_too_close_for_explosive`, `blast_self_risk`, `no_safe_line_of_fire`. |
| Should this actor switch tools now? | Current order, equipped item state, ammo/reload window, threat level, role fit, target material, path state. | `reload_window`, `ammo_empty`, `wrong_tool_for_order`. |
| Should the bot pick up a dropped item? | Pickupability, item visibility, role upgrade, path length/risk, current equipment score, hazard/owner state. | `pickup_path_too_long`, `pickup_low_value`, `pickup_not_allowed`. |
| Should the bot use support/rescue equipment? | Medical/support item target rule, actor wound state, route risk, threat level, interruption rules, failed-use policy. | `support_use_unsafe`, `rescue_threat_too_high`, `needs_medic`. |
| Is this item eligible for bot default loadouts? | `bot_claim_state`, source confidence, package mode, harness refs, warning ids, scripted-tool proof. | `package_blocks_bot_default`, `missing_ai_summary`, `manual_recommended_item`. |

The future item evaluator should output three event families:

| Event | Required Payload | Consumer |
|---|---|---|
| `ai_item_choice` | actor, order, selected item, target context, score inputs, selected reason, top rejected items. | AI harness, replay/debug, HUD. |
| `ai_item_refusal` | actor, refused item, reason label, source field or missing field, first fix action. | Buy/loadout UI, package diagnostics, replay. |
| `ai_item_result` | item, expected effect, actual result, interruption/failure reason, claim-state change if in harness. | AI harness, balance review, replay/backend. |

### Equipment AI Contract Deltas

| Delta | Destination |
|---|---|
| Add shared reason-label enum/schema from [[references/equipment-ai-behavior-contract]]. | [[spec/ai-trust-harness-slice-a]], [[references/equipment-ai-scenarios-slice-a]], [[references/equipment-package-diagnostics-slice-a]], [[spec/replay-recorder-slice-a]] |
| Add `bot_claim_state` values: `untested`, `manual_only`, `risky`, `scenario_passed`, `regression_failed`, `bot_default_allowed`. | `equipment.schema.json`, [[references/equipment-capability-authoring-matrix]], [[spec/package-builder-workbench-slice-a]] |
| Add utility-score inputs for range, material fit, danger, pickup value, support risk, and handling penalty. | [[spec/ai-trust-harness-slice-a]], [[references/equipment-ai-scenarios-slice-a]] |
| Consume generated `claim_state`, blackboard keys, reason labels, event families, source confidence, and scenario refs from [[references/equipment-ai-summary-seed-slice-a]]. | [[spec/ai-trust-harness-slice-a]], [[spec/equipment-loadout-workbench-slice-a]], [[spec/replay-recorder-slice-a]] |
| Add a "why bot blocked" buy/loadout panel. | [[spec/equipment-loadout-workbench-slice-a]], [[spec/ux-wireframes-slice-a]] |
| Add item decision/refusal events to recorder Slice A. | [[spec/replay-recorder-slice-a]], [[systems/replay-event-architecture]] |

## Buy / Loadout UX Contract

The buy/loadout screen from [[spec/ux-wireframes-slice-a]] should compare decisions, not raw stat dumps.

| Surface | Required Fields | Acceptance |
|---|---|---|
| Catalog row | role icon, cost, mass, range band, terrain fit, bot skill, warning badge. | Player can scan for "breacher", "medic", or "safe AI weapon" without opening every item. |
| Detail drawer | behavior summary, pros/cons, material effects, failure modes, replay examples. | Player understands how the item changes play. |
| Actor column | actor role, slots, assigned items, total mass, missing-role warnings. | Assignment is explicit; no cargo-order surprises. |
| Squad summary | total cost, total mass, roles covered, breach/heal/scout/delivery capability. | Player can see what the squad cannot solve. |
| Delivery preview | craft capacity, ETA, LZ footprint, LZ risk, cargo fragility, exit safety. | Player predicts delivery risk before queueing. |
| Bot competence badge | `Good`, `Risky`, `Manual Recommended`, `No AI Support Yet`. | Bot trust is set before the mission, not discovered after failure. |

## Delivery Risk Model

| Risk Input | Why It Matters | Source / Future Hook |
|---|---|---|
| Craft footprint and width | Determines whether the LZ physically fits. | `BuyMenuGUI::GetDeliveryWidth`, craft bounds. |
| Cargo mass and passenger count | Affects craft selection, crash risk, and exit delay. | `GetTotalOrderMass`, `GetTotalOrderPassengers`. |
| LZ terrain stability | Fresh craters, overhangs, and narrow shafts can kill cargo. | [[systems/material-and-mobility-affordance-schema]]. |
| Enemy threat near LZ | AA, grenadiers, turret arcs, and alarm state change delivery safety. | Mission/AI threat map. |
| Cargo fragility | Heavy tools and veterans should not be dropped into a crush zone. | Actor/item physical metadata. |
| Exit mode | Dropship doors, rocket unload, crate burst, pod hatch. | `ACraft` lifecycle and delivery AI. |
| Queue pressure | Multiple deliveries to one zone can collide or block exits. | `GameActivity` delivery queue events. |

## Balance Axes

| Axis | Good Pressure | Bad Pressure |
|---|---|---|
| Range | Forces position and cover decisions. | One weapon dominates all distances. |
| Terrain effect | Weapons/tools change routes and objectives. | Terrain destruction becomes a universal skip button. |
| Handling | Recoil/support/mass make bodies matter. | Sluggishness hides fun behind input delay. |
| AI competence | Some items are bot-safe; some are manual-expert tools. | Bots silently misuse expensive gear. |
| Delivery cost/risk | Logistics is tactical. | Players feel punished by invisible landing math. |
| Supply model | Ammo/heat/charges create timing. | Resupply chores dominate battles. |
| Friendly fire | Explosives are powerful but scary. | AI self-kills without preview or warning. |
| Mod schema | Weird items are supported. | Every mod requires bespoke UI/AI code. |

## Loadout Test Fixtures

| Fixture | Contents | Purpose |
|---|---|---|
| `assault_basic` | Light actor, SMG/rifle, sidearm, grenade. | Baseline combat readability. |
| `engineer_breach` | Light actor, rifle, digger, breaching charge, concrete/fill tool. | Terrain and delivery UI coverage. |
| `medic_rescue` | Light actor, medikit/dart, smoke/cover item, sidearm. | AI rescue and support priority. |
| `sniper_overwatch` | Sniper actor, long-range weapon, sensor, sidearm. | Range, line-of-sight, target priority. |
| `grenadier_risky` | Grenadier, launcher, explosives. | Friendly-fire and danger-radius tests. |
| `heavy_craft_killer` | Heavy actor, rocket/heavy gun, shield. | Recoil/stability and craft threat. |
| `scout_salvager` | Scout, scanner, grapple, sidearm. | Pickup, reveal, mobility. |
| `bad_loadout_missing_breach` | Assault-only squad on hard-material mission. | UI must warn "no breacher". |
| `bad_loadout_bot_unsafe` | Bot assigned high-risk explosive without safety metadata. | Validator must mark manual-only or blocked. |

## Acceptance Tests

LOAD-A, LOAD-R, LOAD-W, LOAD-FIELD, AI-EQ, PACK, and REC equipment runs should cite artifacts from [[references/prototype-run-bundle-schema]] once a prototype emits real run folders.

| ID | Test | Pass Condition |
|---|---|---|
| LOAD-A-01 | Corpus classification | At least the Base/Coalition/Ronin sample corpus maps existing groups into the new archetypes with no unclassified buyable item above warning severity. |
| LOAD-A-02 | Explicit actor assignment | UI/data model preserves which actor receives which item without relying on loadout cargo order. |
| LOAD-A-03 | Role coverage summary | Squad summary correctly flags missing breach, heal, scout, anti-craft, and support roles for mission fixtures. |
| LOAD-A-04 | Item comparison | Player can compare two weapons/tools by range, terrain effect, handling, risk, bot skill, cost, and mass. |
| LOAD-A-05 | Material fit | Digger/breacher/fill tools expose material affordance results from [[systems/material-and-mobility-affordance-schema]]. |
| LOAD-A-06 | Bot-safe weapon choice | AI selects a sane weapon/tool for target range/material and emits a reason label. |
| LOAD-A-07 | Friendly-fire guard | Bot refuses or warns before explosive/area item use when allies are inside danger radius. |
| LOAD-A-08 | Pickup value | Unarmed bot evaluates nearby dropped weapons/tools and explains pickup or ignore. |
| LOAD-A-09 | Delivery risk preview | Buy/loadout screen predicts low/medium/high LZ risk and logs the inputs used. |
| LOAD-A-10 | Owned item economy | Owned/recovered items reduce cost visibly without hiding delivery/craft cost. |
| LOAD-A-11 | Saved templates | Player can save, load, duplicate, and edit named loadouts with role tags. |
| LOAD-A-12 | Mod validation | Package builder rejects or warns for missing role/AI/UX metadata according to package mode. |
| LOAD-A-13 | Replay event coverage | Purchase, delivery, pickup, weapon fire, reload, tool switch, item drop, and support use emit replayable events. |
| LOAD-A-14 | Accessibility/readability | Item role, warning, and risk state are not color-only and remain readable at Slice A text scale per [[spec/accessibility-comfort-slice-a]]. |
| LOAD-A-15 | No universal weapon | Test mission set has no item that is optimal for every range/material/threat/delivery condition. |
| LOAD-A-16 | Manual-only honesty | Items too complex for bots are labeled `Manual Recommended` until AI tests prove competence. |
| AI-EQ-SUMMARY-01 | AI seed coverage | Every role-card row has a generated AI summary seed in [[references/equipment-ai-summary-seed-slice-a]]. |
| AI-EQ-SUMMARY-02 | Bot claim-state honesty | Every item exposes candidate/risky/manual/blocked/not-catalog state before the UI or package builder allows bot-default claims. |
| AI-EQ-SUMMARY-03 | Reason/event parity | Every item has selection/refusal reason labels and `ai_item_candidate`, `ai_item_selected`, `ai_item_refusal`, and `ai_item_result` event families. |
| LOAD-FIELD-01 | Field trace drill-down | Selecting any fixture item shows source path, CCCP field, normalized field, direct/inherited/inferred/manual status, downstream consumers, and first fix action. |
| LOAD-FIELD-02 | Ordered loadout import | Legacy `DeliveryCraft` plus ordered `AddCargoItem` entries convert into explicit actor slots while preserving original source order and warning on ambiguous ownership. |
| LOAD-FIELD-03 | Delivery role reproduction | Light, Heavy, CQB, Scout, Sniper, Grenadier, and Engineer kits can be represented from explicit role fields without relying on hidden Lua group assumptions. |
| LOAD-FIELD-04 | Scripted item declaration | Medikit, Grapple Gun, Concrete Sprayer, Scanner, and Constructor-like tools declare target rules, failure states, side effects, replay events, and bot confidence. |
| LOAD-FIELD-05 | AI reason source | AI item-choice/refusal events cite normalized fields such as `danger_radius`, `support_required`, `material_fit`, `range_band`, `ammo_pressure`, or `bot_unproven`. |
| LOAD-FIELD-06 | Durable schema audit | The schema covers type/prototype identity, component capabilities, hidden/internal items, stack/weight/cost, slot models, interaction flags, ability/effect hooks, and backend loadout compatibility. |
| LOAD-FIELD-SOURCE-01 | Source state coverage | Every role-card row has `source_linked`, `source_not_in_loader_graph`, `role_card_only`, or `missing_source_path` from [[references/equipment-source-trace-slice-a]]. |
| LOAD-FIELD-SOURCE-02 | Source jump targets | Every row exposes open targets for source path, loader graph, role card, and trace-tab refs. |
| LOAD-FIELD-SOURCE-03 | Loader context | Source-linked rows show package/module order, include depth, file stats, include parents, and duplicate preset pressure from [[references/content-loader-graph-cccp]]. |
| LOAD-FIELD-SOURCE-04 | Provenance confidence | Workbench rows expose direct/inherited/inferred/manual/missing counts and critical missing fields before AI or balance treats a value as settled. |
| LOAD-FIELD-SOURCE-05 | Source-driven next actions | Medium/low confidence rows generate first actions for LOAD-FIELD, PACK-014C, AI-H-LOAD-010, duplicate review, or LOAD-W-010. |
| LOAD-FIELD-SOURCE-06 | Checker-backed join | `research_tools/equipment_overlay_check.py` validates the source-trace row set against role cards and trace-tab rows. |

## First Implementation Tickets

| Ticket | Output |
|---|---|
| LOAD-001 | Extend `research_tools/equipment_corpus.py` and [[references/equipment-corpus-cccp]] beyond common `CopyOf` inheritance into fuller PresetMan parity, broader scripted/runtime provenance, JSON/CSV export, and package-builder-owned warning policy. |
| LOAD-002 | Iterate first-pass `equipment.schema.json` from [[references/equipment-schema-and-overlays]] using package-builder, AI harness, and prototype evidence. |
| LOAD-002B | Add `capability.*`, `authoring_tier`, `ai_claim_status`, `role_overlap_resolution`, and source-confidence fields from [[references/equipment-capability-authoring-matrix]] to the schema backlog before freezing any item-role contract. |
| LOAD-002C | Add consumer-traceability fields from [[references/equipment-consumer-traceability-matrix]]: `consumer_visibility`, field-level `source_confidence`, per-capability `claim_state`, `event_contract`, `debug_overlay_contract`, `frontend_priority`, and `package_mode_policy`. |
| LOAD-003 | Review generated Base/Coalition/Ronin overlay seed using [[references/equipment-overlay-review-matrix]] and add manual fixes for inherited fields, internal payloads, role/material/AI fields, and package diagnostics. |
| LOAD-003G | Consume [[references/equipment-overlay-merged-preview]] in prototype loaders so fixture/UI/package/AI work shares the same patch semantics. |
| LOAD-001C | Consume [[references/equipment-provenance-workbench-view]] in buy/loadout and package-builder workbench prototypes as the first fixture-level provenance/diagnostics panel. |
| LOAD-004 | Implement loadout document model with explicit actor slots and delivery block. |
| LOAD-005 | Load [equipment-loadout-fixtures.slice-a.json](../references/equipment-loadout-fixtures.slice-a.json) into [[spec/ux-wireframes-slice-a]] buy/loadout prototype screens. |
| LOAD-006 | Add AI harness scenarios for weapon selection, breach tool selection, friendly-fire refusal, and pickup value. |
| LOAD-006A | First scenario seed done in [[references/equipment-ai-scenarios-slice-a]]; next wire it into the runner. |
| LOAD-006B | Use [[references/equipment-ai-behavior-contract]] to implement item choice/refusal reason labels, `ai_item_choice`, `ai_item_refusal`, `ai_item_result`, and bot-claim-state updates. |
| LOAD-006C | Consume [[references/equipment-ai-summary-seed-slice-a]] as the generated bot item-use contract for `claim_state`, blackboard keys, reason labels, events, source confidence, and scenario links; promote claims only through AI-H/AI-EQ harness evidence. |
| LOAD-007 | Add package-builder validation rules for missing AI/UX/material metadata. |
| LOAD-007A | First expected output exists in [[references/equipment-package-diagnostics-slice-a]]; next compare real workbench diagnostics against it. |
| LOAD-008 | Emit replay events for loadout, purchase, delivery, pickup, switch, fire, reload, support use, and item loss. |
| LOAD-009 | First generated artifact exists in [[references/equipment-role-cards-slice-a]]; renderer requirements now live in [[spec/equipment-role-card-renderer-slice-a]]. Next build LOAD-009A..LOAD-009F from `equipment-role-cards.slice-a.json`. |
| LOAD-009A | First renderer view model exists in [[references/equipment-role-card-renderer-view-slice-a]] and [equipment-role-card-renderer-view.slice-a.json](../references/equipment-role-card-renderer-view.slice-a.json); next consume it in an actual UI/workbench prototype. |
| LOAD-009B | Use [[references/equipment-capability-authoring-matrix]] as the renderer/workbench rubric so item rows show capability fit, authoring tier, AI claim state, balance risk, and modding evidence instead of only catalog category. |
| LOAD-W | First interactive equipment/loadout/workbench prototype requirements now live in [[spec/equipment-loadout-workbench-slice-a]]. Build LOAD-W-001..LOAD-W-010 and run LOAD-W-01..LOAD-W-17 against the generated fixtures and trace-tab view. |
| LOAD-FIELD | Use [[references/equipment-device-loadout-field-atlas]] to add field drill-downs, legacy loadout conversion, delivery-role reproduction, scripted item declarations, AI reason-source labels, and durable schema audit rows to the equipment workbench. |
| LOAD-FIELD-SOURCE | Consume [[references/equipment-source-trace-slice-a]] so every role-card/detail/workbench row can jump to source line, loader module, role card, and trace-tab refs while showing source confidence and package/source next actions. |
| LOAD-011 | Keep generated [[references/equipment-consumer-traceability-slice-a]] and [equipment-consumer-traceability.slice-a.json](../references/equipment-consumer-traceability.slice-a.json) current as LOAD-W, AI-H-LOAD, PACK-014C, LOAD-010C, and REC-A-LOAD add real consumer evidence. |
| LOAD-W-010A | Consume [[references/equipment-trace-tab-view-slice-a]] and [equipment-trace-tab-view.slice-a.json](../references/equipment-trace-tab-view.slice-a.json) in the workbench prototype as the source for item trace rows, fixture tabs, diagnostic trace rows, gap badges, and open targets. |
| LOAD-010 | First generated artifact exists in [[references/equipment-overlap-audit-slice-a]]; overlap-resolution requirements now live in [[spec/equipment-role-card-renderer-slice-a]]. Next turn high/medium overlap groups into visible handling, terrain, AI, economy, mission, or catalog/skin decisions. |
| LOAD-010B | Apply the matrix's capability-role rubric to overlap groups so duplicate items are either visibly differentiated, honestly marked as skin/legacy/history, or kept out of bot-default/published-quality claims until proven. |

## Open Questions

| Question | Why It Matters |
|---|---|
| Should actors have hard inventory slots, soft bulk/mass limits, or both? | Hard slots improve UI clarity; soft limits preserve sandbox weirdness. |
| Should bot competence be per item, per role, or learned from tests? | Static tags are easy; harness-derived confidence is more honest. |
| Should modded items default to manual-only until metadata/tests exist? | Safer for AI trust, but can slow creator iteration if too strict. |
| How much delivery-risk math should be visible? | Exact math helps experts; too much can make the buy screen feel like accounting. |
| Do role kits live in package data, mission data, or player profile data? | Affects mod compatibility, campaign economy, and replay reproducibility. |

## Source Trail

### Local

- `../Cortex-Command-Community-Project/Source/Entities/HeldDevice.h:16`
- `../Cortex-Command-Community-Project/Source/Entities/HDFirearm.cpp:171`
- `../Cortex-Command-Community-Project/Source/Entities/HDFirearm.cpp:650`
- `../Cortex-Command-Community-Project/Source/Entities/Magazine.cpp:37`
- `../Cortex-Command-Community-Project/Source/Entities/Magazine.cpp:143`
- `../Cortex-Command-Community-Project/Source/Entities/Round.cpp:37`
- `../Cortex-Command-Community-Project/Source/Entities/Loadout.cpp:40`
- `../Cortex-Command-Community-Project/Source/Entities/Loadout.cpp:122`
- `../Cortex-Command-Community-Project/Source/Menus/BuyMenuGUI.h:123`
- `../Cortex-Command-Community-Project/Source/Menus/BuyMenuGUI.cpp:180`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/DeliveryCreationHandler.lua:46`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/DeliveryCreationHandler.lua:181`
- `../Cortex-Command-Community-Project/Data/Base.rte/Activities/Utility/DeliveryCreationHandler.lua:712`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Tools/Digger/Digger.ini:33`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Tools/ConcreteSprayer/ConcreteSprayer.ini:5`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Tools/GrappleGun/GrappleGun.ini:1`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Tools/GrappleGun/GrappleGun.lua:1`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Special/Medikit/Medikit.ini:1`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Special/Medikit/Medikit.lua:1`
- `../Cortex-Command-Community-Project/Data/Base.rte/Devices/Weapons/RocketLauncher/RocketLauncher.ini:1`
- `../Cortex-Command-Community-Project/Data/Base.rte/AI/SharedBehaviors.lua:1089`
- `../Cortex-Command-Community-Project/Data/Base.rte/AI/NativeHumanAI.lua:386`
- `../Cortex-Command-Community-Project/Data/Base.rte/AI/HumanBehaviors.lua:569`
- `../Cortex-Command-Community-Project/Data/Base.rte/AI/HumanBehaviors.lua:1014`
- `../research_tools/equipment_corpus.py`
- `../research_tools/equipment_ai_summary_seed.py`
- `../research_tools/equipment_trace_tab_view.py`
- `../research_tools/content_loader_graph.py`
- `../research_tools/equipment_source_trace.py`
- [[references/equipment-cccp-field-map]]
- [[references/equipment-role-design-deep-dive]]
- [[references/equipment-capability-authoring-matrix]]
- [[references/equipment-ai-behavior-contract]]
- [[references/equipment-ai-summary-seed-slice-a]]
- [equipment-ai-summary-seed.slice-a.json](../references/equipment-ai-summary-seed.slice-a.json)
- [[references/equipment-consumer-traceability-matrix]]
- [[references/equipment-consumer-traceability-slice-a]]
- [equipment-consumer-traceability.slice-a.json](../references/equipment-consumer-traceability.slice-a.json)
- [[references/equipment-trace-tab-view-slice-a]]
- [equipment-trace-tab-view.slice-a.json](../references/equipment-trace-tab-view.slice-a.json)
- [[references/equipment-source-trace-slice-a]]
- [equipment-source-trace.slice-a.json](../references/equipment-source-trace.slice-a.json)
- [[references/equipment-role-cards-slice-a]]
- [[references/equipment-role-card-renderer-view-slice-a]]
- [[references/equipment-overlap-audit-slice-a]]
- [[references/equipment-overlap-resolution-worksheet-slice-a]]
- [[spec/equipment-role-card-renderer-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[references/equipment-corpus-cccp]]
- [[references/content-loader-graph-cccp]]
- [[references/equipment-schema-and-overlays]]
- [[references/equipment-overlay-review-matrix]]
- [[references/equipment-loadout-fixtures-slice-a]]
- [[references/equipment-manual-overlay-patches]]
- [[references/equipment-overlay-merged-preview]]
- [[references/equipment-provenance-workbench-view]]
- [[references/equipment-ai-scenarios-slice-a]]
- [[references/equipment-package-diagnostics-slice-a]]
- [[systems/replay-determinism-and-run-evidence]]
- [[research-log/2026-05-04-equipment-role-record-contract]]

### Public

- Giusti, Hullett, Whitehead, "Weapon Design Patterns in Shooter Games": https://users.soe.ucsc.edu/~ejw/papers/giusti-weapon-design-patterns-dpg2012.pdf
- Ozzie Smith, "7 Suggestions for Having Interesting Weapons in Shooters", GameDeveloper: https://www.gamedeveloper.com/design/7-suggestions-for-having-interesting-weapons-in-shooters
- Trent Polack, "A Systemic Approach to Game Design", GameDeveloper: https://www.gamedeveloper.com/business/a-systemic-approach-to-game-design
- Bohemia Interactive Community, "AI Config Reference - Arma 3": https://community.bistudio.com/wiki/Arma_3:_AI_Config_Reference
- Bohemia Interactive Community, "CfgAmmo Config Reference": https://community.bistudio.com/wiki/CfgAmmo_Config_Reference
- Bohemia Interactive Community, "CfgWeapons Config Reference": https://community.bistudio.com/wiki/CfgWeapons_Config_Reference
- Bohemia Interactive Community, "Arma 3 Targeting Config Reference": https://community.bistudio.com/wiki/Arma_3:_Targeting_Config_Reference
- GameDev.net forum thread, "Loadout System Design?": https://gamedev.net/forums/topic/694975-loadout-system-design/
- Brandon "Wayward" Casteel, "How do you create a good loadout system in RTS games?": https://waywardstrategy.com/2022/09/13/loadout-systems-in-rts/
- Randy Smith, "Systems, Game Systems, and Systemic Games", GameDeveloper: https://www.gamedeveloper.com/design/systems-game-systems-and-systemic-games
- Steamworks Documentation, "Steam Inventory Schema": https://partner.steamgames.com/doc/features/inventory/schema
- Steamworks Documentation, "Steam Inventory Item Tags": https://partner.steamgames.com/doc/features/inventory/itemtags
- Epic Developer Community, "Using Gameplay Tags in Unreal Engine": https://dev.epicgames.com/documentation/unreal-engine/using-gameplay-tags-in-unreal-engine
- Epic Developer Community, "Understanding the Unreal Engine Gameplay Ability System": https://dev.epicgames.com/documentation/unreal-engine/understanding-the-unreal-engine-gameplay-ability-system
- Epic Developer Community, "Gameplay Effects for the Gameplay Ability System": https://dev.epicgames.com/documentation/unreal-engine/gameplay-effects-for-the-gameplay-ability-system-in-unreal-engine
- Unity Manual, "ScriptableObject": https://docs.unity3d.com/6000.4/Documentation/Manual/class-ScriptableObject.html
- Godot Engine Docs, "Resources": https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html
- Factorio API Docs: https://lua-api.factorio.com/latest/
- Factorio Prototype Docs: https://lua-api.factorio.com/latest/index-prototype.html
- OpenRA, "Weapons": https://docs.openra.net/en/release/weapons/
- Soldat Wiki, "Weapon Mod": https://wiki.soldat.pl/index.php/Weapon_Mod
- Soldat Manual: https://static.soldat.pl/man/manual-en.html
- Bohemia Interactive Community, "Arma Reforger: Weapon Components": https://community.bistudio.com/wiki/Arma_Reforger:Weapon_Components
- Ravenfield RavenScript, "WeaponEntry": http://ravenfieldgame.com/ravenscript/api/WeaponEntry.html
- Ravenfield RavenScript, "WeaponRole": http://ravenfieldgame.com/ravenscript/api/WeaponRole.html
- Ravenfield RavenScript, "LoadoutType": http://ravenfieldgame.com/ravenscript/api/LoadoutType.html
- Core API, "Equipment": https://docs.coregames.com/api/equipment/
- Core API, "Weapon": https://docs.coregames.com/api/weapon/
- Jeff Orkin, "Three States and a Plan: The A.I. of F.E.A.R.": https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf
- GameDeveloper, "Building the AI of F.E.A.R. with Goal Oriented Action Planning": https://www.gamedeveloper.com/design/building-the-ai-of-f-e-a-r-with-goal-oriented-action-planning
- Game AI Pro 3, "Choosing Effective Utility-Based Considerations": http://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter13_Choosing_Effective_Utility-Based_Considerations.pdf
- Epic Developer Community, "Behavior Tree in Unreal Engine - Overview": https://dev.epicgames.com/documentation/unreal-engine/behavior-tree-in-unreal-engine---overview
- Bohemia Interactive Community, "CfgAmmo Config Reference": https://community.bistudio.com/wiki/CfgAmmo_Config_Reference
- Unreal Engine Replay System: https://dev.epicgames.com/documentation/en-us/unreal-engine/using-the-replay-system-in-unreal-engine
- Photon Quantum Replay: https://doc.photonengine.com/quantum/current/manual/replay

## Change Log

- 2026-05-04: Promoted from stub to prototype requirements after local CCCP device/loadout code pass, device group scan, and external weapon/loadout/AI metadata research.
- 2026-05-04: Added generated schema/overlay seed, review matrix link, warning severity queue, and stronger tag/schema references for LOAD-A fixture work.
- 2026-05-04: Added [[references/equipment-cccp-field-map]] as the field-level bridge from CCCP C++/Lua/INI mechanics into AI, UI, modding, balance, replay, backend, and package-builder consumers.
- 2026-05-04: Added [[research-log/2026-05-04-equipment-role-record-contract]] and the implementation-facing `role_record` contract so item roles are directly consumable by AI, UI, modding, balance, replay/debug, backend/session, and mission systems.
- 2026-05-04: Added [[references/equipment-provenance-workbench-view]] and the role-data-by-consumer table so equipment roles stay usable by AI, UI, modding/workbench, balancing, mission design, replay, and backend tooling.
- 2026-05-04: Added [[references/equipment-role-design-deep-dive]], concrete CCCP device role cards, durable design-reference translation, and LOAD-009/LOAD-010 role-card/overlap tests.
- 2026-05-04: Generated [[references/equipment-role-cards-slice-a]] and [[references/equipment-overlap-audit-slice-a]] so role-card rendering and overlap detection are repeatable artifacts instead of prose-only recommendations.
- 2026-05-04: Added [[spec/equipment-role-card-renderer-slice-a]] so generated role cards and overlap groups become concrete buy/loadout, workbench, AI/debug, replay, accessibility, and balance-review requirements.
- 2026-05-04: Added [[references/equipment-overlap-resolution-worksheet-slice-a]] so LOAD-010 has concrete role-split, skin/legacy, mission-fixture, and AI/manual decisions to test.
- 2026-05-04: Added [[references/equipment-role-card-renderer-view-slice-a]] so LOAD-R has a generated JSON/Markdown fixture for catalog rows, workbench rows, detail drawers, overlap rows, fixture summaries, and acceptance coverage.
- 2026-05-04: Added [[references/equipment-capability-authoring-matrix]] and [[research-log/2026-05-04-equipment-capability-authoring-matrix]] so item roles are described in a way AI, UI, modding/workbench, balancing, replay, and package policy can all consume.
- 2026-05-04: Added [[spec/equipment-loadout-workbench-slice-a]] and [[research-log/2026-05-04-equipment-loadout-workbench-slice-a]] as the concrete interactive UI/workbench prototype target for LOAD-W.
- 2026-05-04: Added [[references/equipment-consumer-traceability-matrix]] and [[research-log/2026-05-04-equipment-consumer-traceability]] so each equipment field can be traced into AI, UI, workbench/package, balance, replay/debug, backend, and prototype-test consumers.
- 2026-05-04: Added generated [[references/equipment-consumer-traceability-slice-a]] so LOAD-011 mechanically checks 106 role-card rows against downstream consumer coverage and gap queues.
- 2026-05-04: Added generated [[references/equipment-trace-tab-view-slice-a]] and [[research-log/2026-05-04-equipment-trace-tab-view]] so LOAD-W-010 has a UI-ready view model with trace rows, fixture tabs, diagnostic rows, gap badges, and open targets.
- 2026-05-04: Added [[references/equipment-device-loadout-field-atlas]] and [[research-log/2026-05-04-equipment-device-loadout-field-atlas]] so exact CCCP device/loadout fields now drive LOAD-FIELD tests, AI reason labels, UI/workbench trace rows, balancing rows, replay events, backend compatibility, and mod schema rules.
- 2026-05-04: Added [[references/equipment-ai-behavior-contract]] and [[research-log/2026-05-04-equipment-ai-behavior-contract]] so actual CCCP item-choice, projectile-summary, breaching, pickup, support, and explosive-safety behavior becomes a shared AI/UI/workbench/replay contract.
- 2026-05-04: Added generated [[references/equipment-ai-summary-seed-slice-a]] and `equipment-ai-summary-seed.slice-a.json` so all 106 role-card rows now have bot decision seeds with claim state, decision inputs, blackboard keys, reason labels, event families, scenario refs, source confidence, and consumer impacts.
- 2026-05-04: Added equipment run-evidence requirements that tie actual CCCP device/loadout fields and role-card claims into [[systems/replay-determinism-and-run-evidence]], run bundles, AI item events, terrain chunks, package diagnostics, and overlap/balance tests.
- 2026-05-04: Added generated [[references/equipment-source-trace-slice-a]] and LOAD-FIELD-SOURCE tests so role cards now join to exact source paths, loader graph context, field provenance, trace-tab refs, source confidence, and source-driven next actions.
- 2026-05-04: Added [[prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke]] and [[research-log/2026-05-04-a1-load-w-fixture-switch-smoke]] so all nine LOAD-A fixture imports and a runtime switch from `engineer_breach` to `medic_rescue` have checked smoke evidence before deeper workbench/AI/replay tests.
- 2026-05-04: Added [[prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke]] and [[research-log/2026-05-04-a1-load-w-fixture-tab-input-smoke]] so fixture-tab click and focused-button keyboard activation now have checked browser input evidence.
- 2026-05-04: Added [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]] and [[research-log/2026-05-04-a1-load-w-fixture-traversal-smoke]] so fixture controls now have checked Tab traversal, ArrowRight controller-equivalent focus movement, Enter/Space activation, and focus-restore evidence.
