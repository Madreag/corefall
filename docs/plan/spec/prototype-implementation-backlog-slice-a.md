---
type: spec
status: implementation-backlog
ready_when: "A0..A7 have task ownership, run-bundle evidence, validation notes, and DR handoff links; no milestone can claim readiness without linked prototype artifacts."
feeds:
  - DR-001
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-006
  - DR-007
  - DR-008
  - DR-009
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|prototype roadmap]] · [[prototypes/actor-feel-lab-a0-bootstrap|A0 bootstrap evidence]] · [[prototypes/actor-feel-lab-a1-runtime-smoke|A1 runtime smoke]] · [[prototypes/actor-feel-lab-a1-ui-smoke|A1 UI smoke]] · [[prototypes/actor-feel-lab-a1-load-w-workbench-smoke|LOAD-W workbench smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke|LOAD-W fixture-switch smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke|LOAD-W fixture-tab input smoke]] · [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke|LOAD-W fixture traversal smoke]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[spec/equipment-loadout-workbench-slice-a|equipment workbench Slice A]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[spec/mission-director-slice-a|mission director Slice A]] · `cortex-command-repos-all/VAULT_PLAN.md` (research vault root)

# Prototype Implementation Backlog Slice A

> [!summary] Purpose
> Convert the Slice A roadmap into implementable task cards. This is the handoff surface for a future implementing agent: what to build first, what evidence to emit, which vault pages supply the requirements, and what each task must avoid growing into.

> [!warning] Backlog, not final spec
> This page is allowed to be practical and opinionated. It does not make launch promises. A task can prototype freely, copy/adapt private experiments, and test moonshots, but final spec claims still need evidence and decision-record closure.

> [!important] Equipment reminder carried forward
> Equipment/loadouts are not a shallow taxonomy or future nice-to-have. A5 must consume actual CCCP-derived device/loadout fields, generated role cards, source trace, field atlas, package diagnostics, AI behavior labels, fixture loadouts, and overlap audits. The goal is one item-role model that AI, UI, modding, balancing, replay, backend, and mission authoring can all use.

## Milestone Map

| Milestone | Main Question | Minimum Build Output | Evidence Gate |
|---|---|---|---|
| A0 Lab shell | Can the team run, reset, tune, and capture a tiny sandbox quickly? | Prototype repo/workspace, one-room scene, run manifest, seed/config dump. | A0 run bundle passes [[references/prototype-run-bundle-schema]]. |
| A1 Actor feel | Is one actor fun and readable for five minutes? | Movement, aim, rifle, reload, recoil, status, item strip. | A-FEEL-01..06 cite events/screenshots. |
| A2 Terrain/material | Do material rules, carving, filling, hazards, and mobility validity make sense under pressure? | Material grid, digger, grenade/charge, repair/fill lane, dirty-region events. | MAT-T-01..10 cite terrain events, overlay evidence, and cost counters. |
| A3 Recorder/viewer | Can failures be reconstructed without guesswork? | Ring buffer, JSONL export, snapshots, event tail, death/failure recap. | REC-A-01..07 and DET-A-01..07 cite event ids. |
| A4 UX comprehension | Can the player tell what happened and what a tool will do? | HUD, material overlay, failure labels, recap panel, accessibility pass. | UX-W/HUD/MAT screenshots and notes. |
| A5 Equipment/loadout | Do item roles survive real CCCP field data, fixtures, bot labels, and source drill-downs? | Mini workbench with mission strip, catalog, actor columns, detail drawer, trace tab, source inspector. | LOAD-A, LOAD-R, LOAD-W, LOAD-FIELD, LOAD-FIELD-SOURCE, AI-H-LOAD, and REC-A-LOAD evidence. |
| A6 AI trust bootstrap | Can a simple bot use the same surfaces and explain decisions/refusals? | Scenario runner, bot intent events, item choice/refusal/result labels, report output. | AI-H-01..06 and AI-EQ labels backed by run evidence. |
| A7 Breach Contract | Can the parts form one repeatable destructible mission? | Typed mission manifest, commander reasons, objective states, LZ scorer, debrief/replay roundtrip. | MISSION-A-01..18, save/replay evidence, loadout mission strip. |

## First 48 Hours

| Order | Action | Done When | Write Results To |
|---|---|---|---|
| 1 | Create a prototype workspace outside canonical reference repos. | It can run a blank/lab scene and has a visible reset loop. | `GOAL_PROGRESS.md`, dated [[research-log/index|research log]]. |
| 2 | Add run-bundle skeleton. | `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`, and captures folder exist. | [[references/prototype-run-bundle-schema]]. |
| 3 | Emit one `system.run_started`, one `input_intent`, and one `system.run_finished` event. | `research_tools/prototype_run_check.py <run-dir>` passes basic coherence. | Prototype run folder and [[systems/replay-determinism-and-run-evidence]]. |
| 4 | Add actor movement/aim/rifle loop. | Browser runtime smoke exists; next a player can move, aim, shoot, reload, reset, export a checked bundle, and produce notes/captures. | [[prototypes/actor-feel-lab-a1-runtime-smoke]], [[spec/actor-feel-sandbox-slice-a]]. |
| 5 | Add one HUD/event-tail screenshot artifact. | Done for UI smoke; next connect screenshots to manual play and runtime item/tool behavior. | [[prototypes/actor-feel-lab-a1-ui-smoke]], [[spec/ux-wireframes-slice-a]]. |
| 6 | Import LOAD-A equipment fixtures and use them in the runtime/workbench. | Done for `engineer_breach` selected-item state, Light Digger/Constructor material-grid edits, checksum/path-refresh events, bounded route/collision probes, bounded actor-hull responses, probe overlay, Timed Explosive refusal, all nine LOAD-A fixture imports, query-param switch to `medic_rescue` Medikit, fixture-tab click/focused-keyboard switching to `medic_rescue`/`sniper_overwatch`, and fixture-control Tab/Arrow/Enter/Space traversal; next connect this to integrated movement collision, global path planner output, full workbench traversal, physical gamepad input, and AI-H evidence. | [[prototypes/actor-feel-lab-a1-ui-smoke]], [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]], [[spec/equipment-loadout-workbench-slice-a]]. |

## Task Board

### A0 Lab Shell

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A0-001 | Create prototype workspace outside reference repos. | `AGENTS.md`, `cortex-command-repos-all/VAULT_PLAN.md` (research vault root), [[spec/prototype-roadmap]]. | Prototype has its own folder/repo and does not dirty CCCP/comparable reference repos. | [[prototypes/actor-feel-lab-a0-bootstrap]] records `prototype_workspaces/actor_feel_lab/`; git status for reference repos stays clean. | Unblocks all A tasks. | Full production repo architecture. |
| A0-002 | One-room lab scene and reset loop. | [[spec/actor-feel-sandbox-slice-a]]. | Run, reset, seed, and reload config are one-command or one-click operations. | Screenshot/capture plus `system.run_started` and `system.run_finished`. | Unblocks A1/A2/A3. | Campaign shell or level editor. |
| A0-003 | Tunable config dump. | [[spec/prototype-roadmap]]. | Movement, weapon, material, recorder, and UI constants serialize into the run manifest. | A0 bootstrap records `config_hash` and `config/a0_lab_config.json`; A1 must add real runtime constants. | Unblocks fair comparison of feel iterations. | Full settings app. |
| A0-004 | Run-bundle checker path. | [[references/prototype-run-bundle-schema]]. | Every serious run can be validated with `research_tools/prototype_run_check.py <run-dir>`. | A0 bootstrap checker output reports `errors 0`; future runs must keep passing. | Unblocks DR evidence. | CI platform build matrix. |

### A1 Actor Feel

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A1-001 | Explicit `control_intent` surface. | [[engine/direct-control-and-actor-feel-lifecycle]], [[spec/replay-recorder-slice-a]]. | Movement, aim, selected item, fire, jump, crouch/stance, and tool intent serialize before simulation consequences. | `input_intent` events precede resulting events. | Unblocks AI driver and replay probes. | Netcode/rollback implementation. |
| A1-002 | Movement and recovery loop. | [[spec/actor-feel-sandbox-slice-a]], [[comparables/opensoldat-local-audit]], [[comparables/openlierox-local-audit]]. | Actor can move, jump/fall, recover from recoil/impact, and restart quickly. | A-FEEL-01 notes with config values and event ids. | Unblocks all feel tests. | Full body animation system. |
| A1-003 | Rifle/reload/recoil/reticle loop. | [[engine/projectile-to-impact-lifecycle]], [[spec/actor-feel-sandbox-slice-a]]. | Player can explain misses from motion, recoil, reload, stance, range, or spread. | A-FEEL-02 screenshot plus `weapon_fired` and reload events. | Unblocks damage, recorder, and UX tests. | Large arsenal. |
| A1-004 | Status/stability strip. | [[spec/body-damage-model]], [[spec/ux-wireframes-slice-a]]. | Actor state shows stable/unstable/dying/dead or equivalent readable states. | HUD capture and `actor_status_changed` events. | Unblocks death recap and body model tests. | Final body-part UI. |

### A2 Terrain And Material Lab

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A2-001 | Material grid with probe/readout. | [[systems/material-and-mobility-affordance-schema]], [[spec/terrain-material-sandbox-slice-a]]. | Air, dirt, concrete, metal, nohook, hazard, loose fill, and repair material can be queried. | `terrain_material_probe` events and overlay capture. | Unblocks A2-002..A2-006. | Noita-scale chemistry. |
| A2-002 | Digger/carve tool. | [[spec/equipment-loadout]], [[spec/terrain-material-sandbox-slice-a]]. | Digger removes dirt and struggles/fails against harder material with a readable label. | MAT-T evidence plus `terrain_carve_mask`, dirty rect, material ids. | Unblocks equipment terrain roles. | Full mining economy. |
| A2-003 | Grenade/charge consequence chain. | [[spec/replay-recorder-slice-a]], [[systems/physics-and-destruction-models]]. | One blast records parent fire/throw/place, terrain edit, impulse/damage, camera/audio cue placeholder, and recap bookmark. | A-FEEL-04 and REC-A-02/03 event chain. | Unblocks body/replay stress tests. | Full explosives catalogue. |
| A2-004 | Repair/fill action. | [[spec/actor-feel-sandbox-slice-a]], [[systems/material-and-mobility-affordance-schema]]. | Player can place or spray one repair/fill material and see path/cover consequence. | `terrain_fill_or_repair` event and overlay screenshot. | Unblocks support/build item roles. | Full construction system. |
| A2-005 | Mobility validity lane. | [[research-log/moonshot-register]], [[spec/actor-feel-sandbox-slice-a]]. | Optional grapple/tether/jet lane either proves valid/nohook feedback or is explicitly split to A.1. | A-FEEL-05 result or split decision. | Unblocks mobility item role evidence. | Let mobility block core gun/dig loop. |
| A2-006 | Dirty-region and path placeholder events. | [[systems/replay-determinism-and-run-evidence]], [[spec/ai-trust-harness-slice-a]]. | Terrain edits emit enough dirty/path data for future AI and network byte-budget probes. | MAT-T and DET-A counters in summary. | Unblocks DR-005/DR-007 evidence. | Full pathfinding rewrite. |

### A3 Recorder And Viewer

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A3-001 | Stable event envelope and ids. | [[spec/replay-recorder-slice-a]], [[references/prototype-run-bundle-schema]]. | Events include run id, tick, event id, category, event type, parent id when applicable, actor/source ids, and payload. | JSONL validates and event ids are unique/monotonic. | Unblocks all event consumers. | Final replay format. |
| A3-002 | Ring buffer and dropped-event accounting. | [[spec/replay-recorder-slice-a]]. | Recorder cannot block simulation and visible counters report drops. | REC-A-06 and summary counters. | Unblocks stress runs. | Streaming analytics service. |
| A3-003 | Snapshots and checksums. | [[systems/replay-determinism-and-run-evidence]]. | Actor/inventory/terrain snapshot or checksum emits at defined cadence. | DET-A-02..05 evidence. | Unblocks determinism and networking DRs. | Pure deterministic replay claim. |
| A3-004 | Event tail and filterable viewer. | [[spec/replay-recorder-slice-a]], [[spec/ux-wireframes-slice-a]]. | Recent important events can be filtered by actor/category/type/cause chain. | Screenshot and REC-A-01/03 evidence. | Unblocks UX comprehension. | Polished replay browser. |
| A3-005 | Death/failure recap. | [[spec/body-damage-model]], [[systems/replay-event-architecture]]. | A forced death/failure shows a last-3-to-5-second chain with input, hit/hazard/terrain, status, and inventory consequences. | REC-A-04 and BODY-A evidence. | Unblocks DR-003. | Cinematic killcam. |

### A4 UX Comprehension

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A4-001 | Minimal tactical HUD. | [[spec/ux-wireframes-slice-a]], [[spec/actor-feel-sandbox-slice-a]]. | Current actor, item, ammo/cooldown, status, reticle state, and last event are visible. | HUD-01..03 captures. | Unblocks player comprehension tests. | Final art direction. |
| A4-002 | Material/tool overlay. | [[spec/terrain-material-sandbox-slice-a]], [[systems/material-and-mobility-affordance-schema]]. | Player can identify breakability, hazard, mobility validity, and repair/fill state before action. | MAT-01A..D screenshots and notes. | Unblocks terrain role confidence. | Full tactical map. |
| A4-003 | Failure labels. | [[spec/replay-recorder-slice-a]], [[references/equipment-ai-behavior-contract]]. | Tool fails with a stable reason label: wrong material, cooldown, no ammo, no anchor, danger, bot unsupported, or unknown. | Event ids linked to visible labels. | Unblocks AI/refusal vocabulary. | Dialogue/tutorial system. |
| A4-004 | Accessibility floor. | [[spec/ux-wireframes-slice-a]], [[spec/accessibility-comfort-slice-a]], [[decisions/dr-012-accessibility-comfort-readability]]. | 200 percent text scale, keyboard/controller navigation, color-independent state labels, captions, reduced motion/shake/flash, and setting events work in debug UI. | ACC-A/UX-W accessibility screenshots/notes and run-bundle setting evidence. | Unblocks workbench UI later. | Full public certification pass. |

### A5 Equipment And Loadout Mini-Workbench

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A5-001 | Equipment artifact loader. | [[references/equipment-role-card-renderer-view-slice-a]], [[references/equipment-loadout-fixtures-slice-a]], [[references/equipment-package-diagnostics-slice-a]], [[references/equipment-ai-scenarios-slice-a]], [[references/equipment-ai-summary-seed-slice-a]]. | Prototype loads renderer rows, nine fixture loadouts, diagnostic rows, AI scenario links, and 106 AI summary seed rows by stable ids. | LOAD-W-01/02/03 output plus JSON parse/checker result. | Unblocks all A5 UI. | Full inventory economy. |
| A5-002 | Mission strip and actor columns. | [[spec/equipment-loadout]], [[spec/mission-director-slice-a]]. | One fixture shows mission requirements, actor role, explicit slots, missing capabilities, bot-safe/manual/risky counts, mass/cost, and delivery notes. | LOAD-A and LOAD-W-05/06/08 evidence. | Unblocks Breach Contract planning. | Full campaign store. |
| A5-003 | Catalog rows and role-card detail drawer. | [[spec/equipment-role-card-renderer-slice-a]], [[references/equipment-role-cards-slice-a]]. | Player mode renders catalog-visible rows; designer mode exposes hidden/internal rows; drawer shows best/bad at, role, handling, terrain, bot, warning, package, replay, source. | LOAD-R-01..08 and LOAD-W-02..04. | Unblocks item comprehension. | Final armory art/pass. |
| A5-004 | Field atlas drill-down. | [[references/equipment-device-loadout-field-atlas]], [[references/equipment-cccp-field-map]]. | Selecting a fixture item shows CCCP field, normalized field, direct/inherited/inferred/manual status, consumers, and first fix action. | LOAD-FIELD-01..06. | Unblocks mod/schema accuracy. | Full schema editor. |
| A5-005 | Source inspector and trace tab. | [[references/equipment-source-trace-slice-a]], [[references/equipment-trace-tab-view-slice-a]], [[references/content-loader-graph-cccp]]. | Workbench can jump from item row to source path/module/include context, source confidence, consumer gaps, package diagnostics, and open targets. | LOAD-W-17/18 and LOAD-FIELD-SOURCE-01..06. | Unblocks package-builder diagnostics. | Full IDE integration. |
| A5-006 | Bot trust and AI item labels. | [[references/equipment-ai-behavior-contract]], [[references/equipment-ai-summary-seed-slice-a]], [[references/equipment-ai-scenarios-slice-a]], [[spec/ai-trust-harness-slice-a]]. | Every risky/manual/bot-blocked item displays claim state, reason labels, blackboard/source-confidence drilldown, scenario reference, first fix action, and selected/refused event families. | AI-H-LOAD, AI-EQ, AI-EQ-SUMMARY, LOAD-W-07/11/12/19/20. | Unblocks AI trust harness. | Full bot behavior implementation. |
| A5-007 | Overlap compare and balance prompts. | [[references/equipment-overlap-audit-slice-a]], [[references/equipment-overlap-resolution-worksheet-slice-a]]. | High-risk assault rifle and sidearm overlap groups show role split, skin/legacy, mission fixture, or manual-only candidate decisions. | LOAD-010 and LOAD-W-10. | Unblocks balance/spec cleanup. | Final balance pass. |
| A5-008 | Runtime fixture import into actor lab. | [[spec/actor-feel-sandbox-slice-a]], [[spec/equipment-loadout-workbench-slice-a]], [[references/equipment-loadout-fixtures-slice-a]], [[prototypes/actor-feel-lab-a1-ui-smoke]], [[prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke]]. | All nine LOAD-A fixture imports exist; `engineer_breach` drives selected item/tool events; query-param fixture switch reaches `medic_rescue` Medikit trace/source/AI rows; fixture-tab mouse click and focused-button keyboard activation reach `medic_rescue` and `sniper_overwatch` trace/source/AI rows; Tab-to-fixture focus, ArrowRight movement, Enter/Space activation, and render focus restore are checked. Remaining pass requires full workbench traversal, physical gamepad input, bot labels, warnings, squad comparison, and replay/export fields. | REC-A-LOAD, DET-A-06, full LOAD-W traversal evidence. | Unblocks real item-role proof. | Full dropship/buy flow. |

### A6 AI Trust Bootstrap

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A6-001 | AI scenario manifest runner. | [[spec/ai-trust-harness-slice-a]], [[references/equipment-ai-scenarios-slice-a]]. | AI-H-01..06 and AI-H-LOAD seeds can run or report a structured blocker. | Scenario report artifact. | Unblocks DR-008. | Full campaign AI. |
| A6-002 | Bot control-intent driver. | A1 `control_intent`, [[spec/replay-recorder-slice-a]]. | Bot can drive the same movement/aim/tool intent surface as player or fail with explicit blocker. | `ai_intent` plus `input_intent` events. | Unblocks bot feel comparison. | Strategic commander AI. |
| A6-003 | Item choice/refusal/result events. | [[references/equipment-ai-behavior-contract]], [[references/equipment-ai-summary-seed-slice-a]], [[spec/equipment-loadout]]. | Bot item decisions emit selected reason, rejected alternatives, missing fields, source confidence, summary row id, claim state, and result. | `ai_item_choice`, `ai_item_refusal`, `ai_item_result`, AI-EQ-SUMMARY. | Unblocks AI-EQ labels and replay. | Machine learning planner. |
| A6-004 | Path/material failure reasons. | [[spec/terrain-material-sandbox-slice-a]], [[systems/material-and-mobility-affordance-schema]]. | Bot or scripted probe can say blocked path, unsafe terrain, wrong tool, or stale path. | AI-H path/material evidence. | Unblocks solo trust tests. | Full squad pathfinding. |
| A6-005 | AI report and overlay. | [[spec/ux-wireframes-slice-a]], [[systems/ai-trust-test-suite]]. | Run summary lists pass/fail, reason labels, event ids, screenshots, and next fixes. | AI harness report linked from run bundle. | Unblocks DR-008 updates. | Designer-only AI IDE. |

### A7 Breach Contract Proof Mission

| ID | Build | Inputs | Done When | Evidence Required | Blocks / Unblocks | Do Not Grow Into |
|---|---|---|---|---|---|---|
| A7-001 | Typed mission manifest. | [[spec/mission-director-slice-a]], [[systems/destruction-objective-mission-patterns]]. | Manifest defines objective, seed, material constraints, required/recommended capabilities, commander text, and replay/save fields. | MISSION-A-01/02. | Unblocks mission strip. | Campaign generator. |
| A7-002 | Capability strip wired to loadout workbench. | [[spec/equipment-loadout-workbench-slice-a]], [[references/equipment-consumer-traceability-matrix]]. | Mission says covered/missing/manual/bot-safe for breach, dig, support, scout, anti-craft, mobility, delivery. | MISSION-A + LOAD-W evidence. | Unblocks one-contract playtest. | Full store/meta economy. |
| A7-003 | Objective state and commander reasons. | [[engine/activity-scenario-lifecycle]], [[spec/ai-trust-harness-slice-a]]. | Objective changes and commander prompts include reason labels, not vague flavor text. | `mission.objective_state_changed` and commander events. | Unblocks recap/debrief. | Full narrative system. |
| A7-004 | LZ/delivery risk scorer. | [[engine/loadout-delivery-economy-lifecycle]], [[spec/backend-service-hub-slice-a]]. | Mission can warn about craft capacity, LZ hazard, delivery footprint, or cargo survival at prototype level. | MISSION-A delivery events and UI capture. | Unblocks logistics spec proof. | Final dropship AI. |
| A7-005 | Debrief/save/replay roundtrip. | [[spec/replay-recorder-slice-a]], [[spec/progression-retention]]. | One run produces debrief with result, key events, actor fate, equipment used/refused, salvage placeholder, and replay/export link. | MISSION-A debrief artifact and checker pass. | Unblocks retention tests. | Campaign persistence system. |

## Acceptance Test Mapping

| Test Family | Primary Milestone | Evidence Must Include |
|---|---|---|
| A-FEEL | A1 | Input events, feel notes, HUD capture, config values. |
| MAT-T / MAT-01 | A2/A4 | Material probe, carve/fill/hazard events, overlay screenshots, dirty-region costs. |
| REC-A / DET-A | A3 | Event chains, snapshots, checksums, dropped counters, viewer artifact, run-bundle checker pass. |
| UX-W / HUD | A4/A5 | Screenshots at normal and 200 percent text, keyboard/controller route, color-independent labels. |
| LOAD-A | A5 | Fixture id, actor slots, item ids, mission capabilities, expected warnings, manual/bot states. |
| LOAD-R | A5 | Catalog rows, detail drawer, role signatures, overlap rows, renderer coverage. |
| LOAD-W | A5 | Workbench surfaces, state transitions, diagnostics, trace tab, source inspector, export preview. |
| LOAD-FIELD / LOAD-FIELD-SOURCE | A5 | Exact CCCP field mapping, normalized field, provenance state, source path/module/include context, open targets. |
| AI-H / AI-EQ | A6 | Bot intent, item choice/refusal/result events, scenario report, reason labels. |
| MISSION-A | A7 | Manifest, objective state, capability strip, commander reasons, delivery risk, debrief/replay roundtrip. |
| PACK / CONTENT | A5/A7 | Package diagnostics, loader graph/source confidence, package mode verdicts, provenance. |

## Run Bundle Evidence By Milestone

| Milestone | Required Run Files | Required Captures | Required Summary Rows |
|---|---|---|---|
| A0 | Manifest, empty/minimal events, summary, notes. | Lab shell or console proof. | Build id, seed, scene id, reset/config. |
| A1 | Input, weapon, reload, status events. | HUD/reticle/item strip. | A-FEEL results, latency/feel notes. |
| A2 | Terrain probe, carve/fill, dirty region, hazard events. | Material overlay and terrain edit before/after. | MAT-T results, dirty rect counts, frame cost. |
| A3 | Snapshots, checksums, parent chains, dropped counters. | Event tail, death/failure recap, viewer filter. | REC/DET results, bytes/sec, dropped_total. |
| A4 | UX visibility events where practical. | HUD, overlay, failure labels, accessibility proof. | UX confusion, text-fit, navigation issues. |
| A5 | Loadout, item role, AI label, package diagnostic, source trace refs. | Workbench catalog/detail/trace/source/export screens. | LOAD/AI-EQ/PACK/REC-A-LOAD results. |
| A6 | AI intent/choice/refusal/result events. | AI overlay or report screen. | Scenario pass/fail with reason labels. |
| A7 | Mission, objective, commander, delivery, debrief events. | Mission strip, objective HUD, debrief. | MISSION-A results and replay/debrief links. |

## Gates Before Moving On

| Move | Gate |
|---|---|
| A0 -> A1 | Reset loop and run-bundle skeleton work. |
| A1 -> A2 | Player can move/aim/shoot/reload for five minutes and explain basic reticle/status states. |
| A2 -> A3 | Terrain edits produce compact, inspectable dirty-region events. |
| A3 -> A4 | REC-A can reconstruct at least one weapon/terrain/death or forced-failure chain. |
| A4 -> A5 | HUD/overlay/failure labels are readable enough that workbench language can reuse them. |
| A5 -> A6 | One fixture reaches runtime/debug UI with item id, role signature, bot label, source trace, package warning, and replay/export fields. |
| A6 -> A7 | Bot harness either runs the basic scenarios or reports blockers with reason labels and event ids. |
| A7 -> Slice B | One Breach Contract run has manifest, objective events, equipment capability strip, debrief, and checked run bundle. |

## Where To Write Results

| Result Type | Destination |
|---|---|
| Daily/substantial pass note | `cortext_command_vault/research-log/YYYY-MM-DD-*.md` and [[research-log/index]]. |
| Current goal checkpoint | `GOAL_PROGRESS.md`. |
| Completion or remaining blockers | `GOAL_COMPLETION_AUDIT.md`. |
| Prototype evidence | `prototype_runs/`, `prototype_workspaces/`, and [[prototypes/index]]. |
| Decision implications | Relevant DR in [[decisions/index]] plus [[dashboards/decision-tracker]]. |
| Spec claim promoted from evidence | Relevant page in [[spec/index]] with backlinks to run evidence and DR. |
| Reused/copied code, data, asset, UI pattern, or content | [[references/usage-ledger]]. |
| Equipment field/schema/workbench discovery | [[spec/equipment-loadout]], [[spec/equipment-loadout-workbench-slice-a]], and the relevant `references/equipment-*` page. |

## Implementation Guardrails

| Guardrail | Practical Rule |
|---|---|
| Preserve reference repos | Build prototypes outside `Cortex-Command-Community-Project/` and `comparables_repos/` unless the user explicitly asks otherwise. |
| Prototype freely | Private copying/adaptation is allowed when it helps speed or quality; log provenance if it enters the future project. |
| Evidence before authority | A note can say "hypothesis" or "prototype result"; do not mark a feature as settled spec without evidence. |
| Recorder first | Every implemented mechanic should emit at least minimal events before it becomes a dependency for AI, UX, networking, or mission logic. |
| Equipment data is shared | AI, UI, modding, balancing, replay, backend, and mission systems consume the same role-card/source-trace fields; do not fork item meanings per subsystem. |
| Debug UI may be plain | It still must be readable, keyboard reachable, color independent, and fit at 200 percent text scale. |
| Scope discipline | If a feature does not test the current milestone question, isolate it as a moonshot or A.1 split. |

## Open Blockers

| Blocker | Impact | Cheapest Next Action |
|---|---|---|
| No human actor-feel run yet. | A1 runtime/UI smokes exist, but five-minute feel, terrain tools, AI, and equipment workbench behavior remain unproven. | Play the browser lab manually, then add terrain tools and one LOAD-A equipment fixture. |
| CCCP visual runtime proof is still partial. | DR-001 cannot lock engine strategy. | Follow [[engine/cccp-runtime-window-capture-troubleshooting]] and archive visible menu/mission proof. |
| Equipment workbench is data-ready but not interactive. | Item-role model cannot yet prove player/AI/UI comprehension. | Implement A5-001..A5-005 using generated JSON fixtures. |
| AI harness is design-ready but not runnable. | "Great solo AI" remains a target, not evidence. | Implement A6-001 with fixture scenarios and reason-label output. |
| Replay/viewer is not built. | Prototype findings would be hard to debug and promote. | Build A3 before broadening mechanics. |
| Overlap decisions are seeded but untested. | Arsenal can still collapse into false choices. | Use A5-007 and A5-008 to compare overlap items in runtime fixtures. |

## Source Trail

- [[spec/prototype-roadmap]]
- [[prototypes/actor-feel-lab-a0-bootstrap]]
- [[prototypes/actor-feel-lab-a1-runtime-smoke]]
- [[prototypes/actor-feel-lab-a1-ui-smoke]]
- [[references/prototype-run-bundle-schema]]
- [[spec/actor-feel-sandbox-slice-a]]
- [[spec/replay-recorder-slice-a]]
- [[systems/replay-determinism-and-run-evidence]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/equipment-loadout]]
- [[spec/equipment-role-card-renderer-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[references/equipment-device-loadout-field-atlas]]
- [[references/equipment-source-trace-slice-a]]
- [[references/equipment-trace-tab-view-slice-a]]
- [[references/equipment-ai-behavior-contract]]
- [[references/equipment-ai-summary-seed-slice-a]]
- [[references/equipment-consumer-traceability-matrix]]
- [[references/equipment-loadout-fixtures-slice-a]]
- [[references/equipment-role-card-renderer-view-slice-a]]
- [[references/equipment-package-diagnostics-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
- [[spec/mission-director-slice-a]]
- [[decisions/index]]

## Change Log

- 2026-05-04: A0 bootstrap run validated the separate workspace and run-bundle checker path; A1 interactive runtime remains open.
- 2026-05-04: A1 runtime smoke added a browser actor-feel lab and checked movement/aim/rifle/reload/status/snapshot event bundle; human feel remains open.
- 2026-05-04: A1 UI smoke added deterministic browser capture, selected `engineer_breach` LOAD-A fixture state, Light Digger/Constructor material-grid edits, checksum/path-refresh events, bounded route/collision probes, bounded actor-hull responses, probe overlays, Timed Explosive refusal labels, and checked screenshot run bundle; manual feel, integrated movement collision, global path planner output, and AI competence remain open.
- 2026-05-04: Created as the implementation task-card handoff for Slice A, with equipment/loadout promoted to an explicit A5 milestone grounded in CCCP field/source artifacts.
