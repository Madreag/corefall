# Cortex Command Vault Plan

This is the execution plan for growing `cortext_command_vault/` into a durable Cortex Command knowledge base. The game spec will be one curated section inside the vault, not the destination that replaces the vault.

> [!info] Top priority
> Build the best possible game, UX/UI, frontend/backend, systems, features, and player experience. The vault is the planning surface, not a brake. Research, prototyping, and reuse are encouraged. License/reuse is a documentation surface (`cortext_command_vault/references/usage-ledger.md`), not a gate. See `AGENTS.md`.

> [!warning] Spec-section rule
> The full authoritative spec section is not ready to finalize yet — but only because the **evidence** isn't there yet (lifecycle traces, prototype results, comparable code audits, decision records). Exploratory spec stubs, ambitious feature notes, and private prototypes are allowed now. Once the gates below are met, we promote proven directions into the spec. The vault is bigger than the spec; the spec lives inside it.

## Planning Docs (Open These First)

> [!important] Daily-use docs for planning + handoff + status
> Vault home + roadmap + backlog + checklist + reading list + decision tracker drive every implementation pass. The first stop for any planning question is `cortext_command_vault/index.md` (Planning Docs panel).

| Doc | Path | Use For |
|---|---|---|
| **Vault home** | `cortext_command_vault/index.md` | Single-pane dashboard with the Planning Docs panel + Fast Routes. |
| **Native build roadmap** | `cortext_command_vault/spec/prototype-roadmap.md` | M0..M12 + 8 sub-milestones, side tracks, Open Decision Gates Protocol, CLI Reference, validation matrix, definition of done. |
| **Native task-card backlog** | `cortext_command_vault/spec/native-implementation-backlog.md` | Concrete task cards per milestone — primary AI handoff file. |
| **Feature completion checklist** | `cortext_command_vault/spec/feature-completion-checklist.md` | Live completion + rating rows; Open Decision Gates Checklist; Server/MMO + Material/T-MAT addenda. |
| **AI-coder reading list** | `cortext_command_vault/spec/ai-coder-reading-list.md` | Required reading order for AI workers before starting any milestone or feature task. |
| **AI control / observability layer** | `cortext_command_vault/spec/ai-control-observability-layer.md` | Eyes/ears/hands rule: every player surface reachable from `cxctl`. |
| **Authoritative game spec v0** | `cortext_command_vault/spec/authoritative-game-spec-v0.md` | Canonical product direction + launch commitments. |
| **Decision tracker** | `cortext_command_vault/dashboards/decision-tracker.md` | Live DR status (DR-001..DR-036) + open topics. |
| **Research readiness** | `cortext_command_vault/dashboards/research-readiness.md` | Spec readiness gates + progress bars + next four artifacts. |
| **System heatmap** | `cortext_command_vault/dashboards/system-heatmap.md` | System priority by player value, risk, evidence. |
| **Run-bundle schema** | `cortext_command_vault/references/prototype-run-bundle-schema.md` | Run-bundle contract + acceptance gates per milestone. |
| **Prototype evidence** | `cortext_command_vault/prototypes/index.md` | Run bundles + workspaces. |

## Navigation

| Destination | Path |
|---|---|
| Vault home | `cortext_command_vault/index.md` |
| Goal completion audit (snapshot) | `cortext_command_vault/research-log/2026-05-04-goal-completion-audit-snapshot.md` |
| Dashboard hub | `cortext_command_vault/dashboards/index.md` |
| Navigation map | `cortext_command_vault/dashboards/navigation-map.md` |
| System heatmap | `cortext_command_vault/dashboards/system-heatmap.md` |
| Research readiness gates | `cortext_command_vault/dashboards/research-readiness.md` |
| Decision records | `cortext_command_vault/decisions/index.md` |
| AI trust source of truth | `cortext_command_vault/systems/ai-trust-test-suite.md` |
| AI trust harness Slice A requirements | `cortext_command_vault/spec/ai-trust-harness-slice-a.md` |
| CCCP build/run audit | `cortext_command_vault/engine/cccp-build-run-audit.md` |
| CCCP runtime window capture troubleshooting | `cortext_command_vault/engine/cccp-runtime-window-capture-troubleshooting.md` |
| Direct actor-control lifecycle | `cortext_command_vault/engine/direct-control-and-actor-feel-lifecycle.md` |
| Body damage lifecycle | `cortext_command_vault/engine/body-damage-wound-gib-lifecycle.md` |
| Body damage model requirements | `cortext_command_vault/spec/body-damage-model.md` |
| Tone/player promise decision | `cortext_command_vault/decisions/dr-014-tone-player-promise.md` |
| Chassis, armor, mechs, and origins spec | `cortext_command_vault/spec/chassis-armor-mechs-and-origins.md` |
| Activity/scenario lifecycle | `cortext_command_vault/engine/activity-scenario-lifecycle.md` |
| Missions/objectives spec index | `cortext_command_vault/spec/missions-and-objectives.md` |
| Mission director Slice A requirements | `cortext_command_vault/spec/mission-director-slice-a.md` |
| Networking terrain lifecycle | `cortext_command_vault/engine/network-terrain-replication-lifecycle.md` |
| Backend service scope decision | `cortext_command_vault/decisions/dr-013-backend-service-scope.md` |
| Backend service/hub Slice A requirements | `cortext_command_vault/spec/backend-service-hub-slice-a.md` |
| Replay/event architecture | `cortext_command_vault/systems/replay-event-architecture.md` |
| Replay recorder Slice A requirements | `cortext_command_vault/spec/replay-recorder-slice-a.md` |
| Replay determinism/run evidence | `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` |
| UX overlay/screen brief | `cortext_command_vault/systems/ux-overlay-screen-brief.md` |
| UX wireframes Slice A requirements | `cortext_command_vault/spec/ux-wireframes-slice-a.md` |
| Accessibility/comfort Slice A requirements | `cortext_command_vault/spec/accessibility-comfort-slice-a.md` |
| Equipment/loadout model requirements | `cortext_command_vault/spec/equipment-loadout.md` |
| Equipment shared consumer contract pass | `cortext_command_vault/research-log/2026-05-04-equipment-shared-consumer-contract.md` |
| Equipment role-record contract pass | `cortext_command_vault/research-log/2026-05-04-equipment-role-record-contract.md` |
| Equipment generated role records | `cortext_command_vault/references/equipment-role-records-slice-a.md`; `cortext_command_vault/references/equipment-role-records.slice-a.json` |
| Equipment role-record projection pass | `cortext_command_vault/research-log/2026-05-04-equipment-role-record-projection.md` |
| Equipment role-card renderer Slice A | `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md` |
| Equipment loadout workbench Slice A | `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md` |
| Equipment CCCP field map | `cortext_command_vault/references/equipment-cccp-field-map.md` |
| Equipment device/loadout field atlas | `cortext_command_vault/references/equipment-device-loadout-field-atlas.md` |
| Equipment source-anchored device snapshots | `cortext_command_vault/references/equipment-source-anchored-device-snapshots.md` |
| Equipment comparable design patterns | `cortext_command_vault/references/equipment-comparable-design-patterns.md` |
| Equipment role design deep dive | `cortext_command_vault/references/equipment-role-design-deep-dive.md` |
| Equipment capability authoring matrix | `cortext_command_vault/references/equipment-capability-authoring-matrix.md` |
| Equipment AI behavior contract | `cortext_command_vault/references/equipment-ai-behavior-contract.md` |
| Equipment AI summary seed | `cortext_command_vault/references/equipment-ai-summary-seed-slice-a.md`; `cortext_command_vault/references/equipment-ai-summary-seed.slice-a.json` |
| Equipment consumer traceability matrix | `cortext_command_vault/references/equipment-consumer-traceability-matrix.md` |
| Equipment generated consumer traceability report | `cortext_command_vault/references/equipment-consumer-traceability-slice-a.md` |
| Equipment generated trace-tab view | `cortext_command_vault/references/equipment-trace-tab-view-slice-a.md` |
| Equipment generated source trace | `cortext_command_vault/references/equipment-source-trace-slice-a.md`; `cortext_command_vault/references/equipment-source-trace.slice-a.json` |
| Equipment generated role records | `cortext_command_vault/references/equipment-role-records-slice-a.md`; `cortext_command_vault/references/equipment-role-records.slice-a.json` |
| Equipment generated role cards | `cortext_command_vault/references/equipment-role-cards-slice-a.md` |
| Equipment role-card renderer view | `cortext_command_vault/references/equipment-role-card-renderer-view-slice-a.md` |
| Equipment generated overlap audit | `cortext_command_vault/references/equipment-overlap-audit-slice-a.md` |
| Equipment overlap resolution worksheet | `cortext_command_vault/references/equipment-overlap-resolution-worksheet-slice-a.md` |
| Generated CCCP equipment corpus | `cortext_command_vault/references/equipment-corpus-cccp.md` |
| Equipment schema and overlay seed | `cortext_command_vault/references/equipment-schema-and-overlays.md` |
| Equipment overlay review matrix | `cortext_command_vault/references/equipment-overlay-review-matrix.md` |
| Equipment manual overlay patches | `cortext_command_vault/references/equipment-manual-overlay-patches.md` |
| Equipment merged overlay preview | `cortext_command_vault/references/equipment-overlay-merged-preview.md` |
| Equipment provenance workbench view | `cortext_command_vault/references/equipment-provenance-workbench-view.md` |
| Equipment loadout fixtures | `cortext_command_vault/references/equipment-loadout-fixtures-slice-a.md` |
| Equipment AI scenario seeds/schema | `cortext_command_vault/references/equipment-ai-scenarios-slice-a.md` |
| Equipment package diagnostics expected output | `cortext_command_vault/references/equipment-package-diagnostics-slice-a.md` |
| Content module loading lifecycle | `cortext_command_vault/engine/content-module-loading-lifecycle.md` |
| Generated content loader graph | `cortext_command_vault/references/content-loader-graph-cccp.md`; `cortext_command_vault/references/content-loader-graph-cccp.json` |
| Progression/retention decision | `cortext_command_vault/decisions/dr-011-progression-retention-loop.md` |
| Progression/retention spec | `cortext_command_vault/spec/progression-retention.md` |
| Material/mobility schema proposal | `cortext_command_vault/systems/material-and-mobility-affordance-schema.md` |
| Terrain/material Slice A requirements | `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md` |
| Native prototype roadmap / M0..M12 build order | `cortext_command_vault/spec/prototype-roadmap.md` |
| Native implementation backlog / M0..M12 task cards | `cortext_command_vault/spec/native-implementation-backlog.md` |
| Feature completion/rating checklist | `cortext_command_vault/spec/feature-completion-checklist.md` |
| AI control/observability layer | `cortext_command_vault/spec/ai-control-observability-layer.md` |
| Hybrid LLM AI plan / async mind-layer roadmap extension | `cortext_command_vault/spec/hybrid-llm-ai-plan.md` |
| Full collision physics plan / T-PHYS + M5.5 | `cortext_command_vault/spec/full-collision-physics-plan.md` |
| Server app architecture / `cx-server` / T-SERVER | `cortext_command_vault/spec/server-app-architecture.md` |
| Persistent MMO architecture / M12 MMO mode | `cortext_command_vault/spec/persistent-mmo-architecture.md` |
| Historical HTML/Slice-A backlog / A0..A7 task cards | `cortext_command_vault/spec/prototype-implementation-backlog-slice-a.md` |
| Authoritative game spec v0 | `cortext_command_vault/spec/authoritative-game-spec-v0.md` |
| Prototype run-bundle schema/checker | `cortext_command_vault/references/prototype-run-bundle-schema.md`; `research_tools/prototype_run_check.py` |
| Prototype evidence index | `cortext_command_vault/prototypes/index.md` |
| Actor-feel A0 bootstrap evidence | `cortext_command_vault/prototypes/actor-feel-lab-a0-bootstrap.md`; `prototype_workspaces/actor_feel_lab/`; `prototype_runs/actor_feel_lab/a0_bootstrap_2026-05-04_1256/` |
| Actor-feel A1 runtime smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-runtime-smoke.md`; `prototype_workspaces/actor_feel_lab/web/`; `prototype_runs/actor_feel_lab/a1_runtime_smoke_2026-05-04_1309/` |
| Actor-feel A1 UI + LOAD-A smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-ui-smoke.md`; `prototype_runs/actor_feel_lab/a1_ui_smoke_2026-05-04_1322/`; `prototype_runs/actor_feel_lab/a1_ui_smoke_2026-05-04_1322/captures/a1-ui-smoke.png` |
| Actor-feel A1 LOAD-W workbench smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md`; `prototype_runs/actor_feel_lab/a1_load_w_workbench_smoke_2026-05-04_1503/`; `prototype_runs/actor_feel_lab/a1_load_w_workbench_smoke_2026-05-04_1503/captures/a1-load-w-workbench-smoke.png` |
| Actor-feel A1 LOAD-W fixture-switch smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_switch_smoke_2026-05-04_1517/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_switch_smoke_2026-05-04_1517/captures/a1-load-w-fixture-switch-smoke.png` |
| Actor-feel A1 LOAD-W fixture-tab input smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_tab_input_smoke_2026-05-04_1558/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_tab_input_smoke_2026-05-04_1558/captures/a1-load-w-fixture-tab-input-smoke.png` |
| Actor-feel A1 LOAD-W fixture traversal smoke evidence | `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_traversal_smoke_2026-05-04_1610/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_traversal_smoke_2026-05-04_1610/captures/a1-load-w-fixture-traversal-smoke.png` |
| Actor-feel Slice A requirements | `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md` |
| Destruction objective patterns | `cortext_command_vault/systems/destruction-objective-mission-patterns.md` |
| Modding workbench brief | `cortext_command_vault/systems/modding-package-and-workbench.md` |
| Modding model requirements | `cortext_command_vault/spec/modding-model.md` |
| Package builder/workbench Slice A requirements | `cortext_command_vault/spec/package-builder-workbench-slice-a.md` |
| Comparable source pass | `cortext_command_vault/comparables/research-pass-2-open-source-systems.md` |
| OpenSoldat satellite audit | `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md` |
| Comparables workspace | `comparables_repos/README.md` |
| Usage ledger (track reuse) | `cortext_command_vault/references/usage-ledger.md` |
| Moonshot register (wild ideas) | `cortext_command_vault/research-log/moonshot-register.md` |
| Fork opportunities | `cortext_command_vault/design/opportunities-for-our-fork.md` |
| Game spec section | `cortext_command_vault/spec/authoritative-game-spec-v0.md`; `cortext_command_vault/spec/index.md` |

## North Star

Create the best Cortex Command-like game: solo-first, AI-rich, moddable, physics/destruction-driven, readable under pressure, and built around battles that players want to replay because the systems create stories.

The plan must answer this before we write the spec:

> What exact mechanics, tools, UI, AI, and technical architecture would make a modern Cortex-like game more replayable, more legible, and more maintainable than the original?

## Vault Operating Model

| Layer | Role | Rule |
|---|---|---|
| `repos/` | What each cloned Cortex repo is and why it matters. | Keep factual, source-backed, and version-aware. |
| `engine/` | Code-level mechanics and lifecycle archaeology. | Prefer code paths, diagrams, and observed behavior over design wishes. |
| `systems/` | Design translation across physics, AI, UX, networking, equipment, retention, and tooling. | Convert evidence into design implications, but mark assumptions. |
| `comparables/` | Similar games, open-source references, and outside design lessons. | Capture lessons without treating other games as blueprints. |
| `decisions/` | Build-time and spec-time decision records. | Compare options, pros/cons, risks, evidence, and revisit triggers before committing. |
| `design/` and `strategy/` | Direction, opportunities, decisions, and planning. | Keep tradeoffs explicit and revisit when evidence changes. |
| `spec/` | The future game spec section. | Curate decisions from the vault and link back to evidence instead of duplicating every research note. |
| `references/` and `research-log/` | Source trail and chronological audit log. | Preserve provenance so future decisions remain checkable. |

## Execution Board

| Priority | Workstream | Output | Status | Primary Links |
|---|---|---|---|---|
| P0 | Body damage model requirements | Body parts + wounds + status + gibs + equipment fallout + treatment + AI reason labels + replay/death recap + HUD rules + BODY-A tests | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | `cortext_command_vault/spec/body-damage-model.md`, `cortext_command_vault/engine/body-damage-wound-gib-lifecycle.md`, `cortext_command_vault/engine/combat-actors-gibbing.md`, `cortext_command_vault/research-log/2026-05-04-body-damage-model-spec.md` |
| P0 | Chassis, armor, mechs, and origins | Armor layers + powered armor + enterable mechs + android/robot/origin choices + staged equipment/module damage + pilot rescue/ejection + CHASSIS-A tests | <span class="cc-flag cc-blue">EXPLORATORY REQS</span> | `cortext_command_vault/spec/chassis-armor-mechs-and-origins.md`, `cortext_command_vault/decisions/dr-014-tone-player-promise.md`, `cortext_command_vault/research-log/2026-05-04-tone-chassis-mech-promise.md` |
| P0 | Command core / base power model | Rooted command core + base shields/turrets/sensors/doors/repair platforms + uproot tradeoff + embedded avatar boosts + CORE-A tests | <span class="cc-flag cc-blue">PLANNING ANCHOR</span> | `cortext_command_vault/spec/command-core-base-power.md`, `cortext_command_vault/decisions/dr-015-player-identity-control-posture.md` |
| P0 | Activity/scenario lifecycle | C++ + Lua activity contract note | <span class="cc-flag cc-green">DONE</span> | `cortext_command_vault/engine/activity-scenario-lifecycle.md` |
| P0 | Replay/event architecture | Event model brief | <span class="cc-flag cc-green">DONE</span> | `cortext_command_vault/systems/replay-event-architecture.md` |
| P0 | Replay recorder Slice A requirements | Event envelope + hook map + viewer/export tests | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/replay-recorder-slice-a.md`, `cortext_command_vault/decisions/dr-002-replay-event-architecture.md` |
| P0 | Replay determinism/run evidence | Hybrid replay posture + deterministic-island gates + checksums/snapshots + DET-A tests | <span class="cc-flag cc-blue">RESEARCH BRIDGE</span> | `cortext_command_vault/systems/replay-determinism-and-run-evidence.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md`, `cortext_command_vault/research-log/2026-05-04-replay-determinism-run-evidence.md` |
| P0 | Destruction-objective brief | Mission pattern catalog | <span class="cc-flag cc-green">DONE</span> | `cortext_command_vault/systems/destruction-objective-mission-patterns.md` |
| P0 | Mission director Slice A requirements | Typed mission manifest + director pacing + commander AI + destruction-aware objective grammar + equipment capability contract + save/replay events + UI/workbench obligations + MISSION-A tests | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/mission-director-slice-a.md`, `cortext_command_vault/spec/missions-and-objectives.md`, `cortext_command_vault/research-log/2026-05-04-mission-director-slice-a.md` |
| P0 | UX overlay/screen brief | Screen inventory + overlay states | <span class="cc-flag cc-green">DONE</span> | `cortext_command_vault/systems/ux-overlay-screen-brief.md` |
| P0 | UX wireframes Slice A requirements | HUD/squad/command/buy/replay/hub/workbench/accessibility wireframes + UX-W tests | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/ux-wireframes-slice-a.md`, `cortext_command_vault/spec/accessibility-comfort-slice-a.md`, `cortext_command_vault/systems/ux-overlay-screen-brief.md` |
| P0 | Accessibility/comfort Slice A requirements | Text scale/reflow + contrast + no-color-only states + same-input navigation + remap/holds + captions + reduced motion/shake/flash + equipment workbench ACC-A tests + run-bundle evidence additions | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/accessibility-comfort-slice-a.md`, `cortext_command_vault/decisions/dr-012-accessibility-comfort-readability.md`, `cortext_command_vault/spec/ux-wireframes-slice-a.md`, `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md` |
| P0 | Equipment/loadout model requirements | Actor roles + item archetypes + shared consumer contract + implementation-facing role-record contract + exact CCCP device/loadout field atlas + source-anchored device snapshots + comparable design patterns + capability matrix + AI behavior contract + generated AI summary seed + consumer traceability + generated consumer traceability report + generated trace-tab view + generated source trace + generated role cards + renderer view + overlap audit + overlap worksheet + role-card renderer + interactive workbench spec + first LOAD-W render smoke + first fixture-switch smoke + fixture-tab input smoke + fixture-control traversal smoke + explicit slots + AI/UX/mod/replay/backend metadata + delivery risk + LOAD-A/LOAD-R/LOAD-W/AI-EQ/AI-EQ-SUMMARY/LOAD-FIELD/LOAD-FIELD-SOURCE/LOAD-COMP tests | <span class="cc-flag cc-blue">WORKBENCH + FIXTURE TRAVERSAL SMOKE PASS</span> | `cortext_command_vault/spec/equipment-loadout.md`, `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md`, `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md`, `cortext_command_vault/research-log/2026-05-04-a1-load-w-workbench-smoke.md`, `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-switch-smoke.md`, `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-tab-input-smoke.md`, `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-traversal-smoke.md`, `cortext_command_vault/research-log/2026-05-04-equipment-shared-consumer-contract.md`, `cortext_command_vault/research-log/2026-05-04-equipment-role-record-contract.md`, `cortext_command_vault/references/equipment-source-trace-slice-a.md`, `cortext_command_vault/references/equipment-role-card-renderer-view-slice-a.md`, `cortext_command_vault/references/equipment-cccp-field-map.md`, `cortext_command_vault/references/equipment-device-loadout-field-atlas.md`, `cortext_command_vault/references/equipment-source-anchored-device-snapshots.md`, `cortext_command_vault/references/equipment-comparable-design-patterns.md`, `cortext_command_vault/references/equipment-role-design-deep-dive.md`, `cortext_command_vault/references/equipment-capability-authoring-matrix.md`, `cortext_command_vault/references/equipment-ai-behavior-contract.md`, `cortext_command_vault/references/equipment-ai-summary-seed-slice-a.md`, `cortext_command_vault/references/equipment-consumer-traceability-matrix.md`, `cortext_command_vault/references/equipment-consumer-traceability-slice-a.md`, `cortext_command_vault/references/equipment-trace-tab-view-slice-a.md`, `cortext_command_vault/references/equipment-role-cards-slice-a.md`, `cortext_command_vault/references/equipment-overlap-audit-slice-a.md`, `cortext_command_vault/references/equipment-overlap-resolution-worksheet-slice-a.md`, `cortext_command_vault/references/equipment-corpus-cccp.md`, `cortext_command_vault/references/equipment-schema-and-overlays.md`, `cortext_command_vault/references/equipment-overlay-review-matrix.md`, `cortext_command_vault/references/equipment-manual-overlay-patches.md`, `cortext_command_vault/references/equipment-overlay-merged-preview.md`, `cortext_command_vault/references/equipment-provenance-workbench-view.md`, `cortext_command_vault/references/equipment-loadout-fixtures-slice-a.md`, `cortext_command_vault/references/equipment-ai-scenarios-slice-a.md`, `cortext_command_vault/references/equipment-package-diagnostics-slice-a.md`, `cortext_command_vault/engine/loadout-delivery-economy-lifecycle.md`, `cortext_command_vault/systems/damage-equipment-and-items.md` |
| P0 | Content module loading lifecycle | `.rte` discovery/import + official/mod/userdata load order + include stack + `CopyOf` + source paths + script reload + generated loader graph + CONTENT-A tests | <span class="cc-flag cc-green">DONE + GENERATED GRAPH</span> | `cortext_command_vault/engine/content-module-loading-lifecycle.md`, `cortext_command_vault/references/content-loader-graph-cccp.md`, `cortext_command_vault/spec/modding-model.md`, `cortext_command_vault/spec/package-builder-workbench-slice-a.md` |
| P0 | Modding workbench brief | Package format + validation + loader graph + workbench | <span class="cc-flag cc-green">DONE</span> | `cortext_command_vault/systems/modding-package-and-workbench.md`, `cortext_command_vault/engine/content-module-loading-lifecycle.md` |
| P0 | Material/mobility schema proposal | Launch/lab/mod material fields + affordances | <span class="cc-flag cc-yellow">DRAFT</span> | `cortext_command_vault/systems/material-and-mobility-affordance-schema.md` |
| P0 | Terrain/material Slice A requirements | Eight-material lab + overlays + dirty-region/path/replay/AI MAT-T tests | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md`, `cortext_command_vault/decisions/dr-007-terrain-material-model.md` |
| P0 | Actor-feel Slice A requirements | Prototype scope + tests + event hooks + kill criteria | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md`, `cortext_command_vault/decisions/dr-004-first-playable-slice.md` |
| P0 | Native prototype roadmap / M0..M12 build order | Native Rust + Bevy/wgpu roadmap + agent implementation contract + T-CONTROL AI/dev observability + M1.5 Micro Breach fun proof + validation matrix + bug-hunt checklist + definition of done | <span class="cc-flag cc-blue">IMPLEMENTATION BRIDGE</span> | `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/spec/ai-control-observability-layer.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md` |
| P0 | Native implementation backlog / M0..M12 task cards | Crate ownership + implementation steps + tests + E2E commands + `cxctl`/control-observation evidence + run-bundle evidence + anti-scope for every native milestone | <span class="cc-flag cc-blue">PRIMARY IMPLEMENTATION BACKLOG</span> | `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/ai-control-observability-layer.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md` |
| P0 | Feature completion/rating checklist | Live checkoff surface for every roadmap feature, milestone scope/done-criterion, side-track obligation, task card, validation check, bug-hunt prompt, and definition-of-done row; agents update evidence and AI self-ratings after implementation, while human ratings stay owner-controlled | <span class="cc-flag cc-green">LIVE CHECKLIST</span> | `cortext_command_vault/spec/feature-completion-checklist.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/prototypes/index.md` |
| P0 | AI control/observability layer | Structured eyes/ears/hands layer for Codex + tests + future bot authors: `cx-control`, `cxctl`, observation stream, collision observation, semantic actions, UI tree, capability gates | <span class="cc-flag cc-blue">CROSS-CUTTING REQUIREMENT</span> | `cortext_command_vault/spec/ai-control-observability-layer.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/spec/full-collision-physics-plan.md` |
| P0 | Hybrid LLM AI plan | Async LLM mind layer that never blocks reflex/tactical AI: provider adapters, strict schemas, proposal validation, replay events, memory, doctrine patches, personality, debriefs, and M6.5 Mind Lab tests | <span class="cc-flag cc-blue">ROADMAP EXTENSION</span> | `cortext_command_vault/spec/hybrid-llm-ai-plan.md`, `cortext_command_vault/spec/ai-control-observability-layer.md`, `cortext_command_vault/spec/ai-trust-harness-slice-a.md`, `cortext_command_vault/decisions/dr-008-ai-architecture.md`, `cortext_command_vault/decisions/dr-022-ai-humanlike-bar.md` |
| P0 | Full collision physics plan | Everything physical collides by default unless explicit tested filters say otherwise: collision matrix/proxies, CCD tiers, projectile-projectile contacts, impulse-to-damage, `collision` events, T-PHYS, and M5.5 COLL-001..COLL-012 | <span class="cc-flag cc-blue">ROADMAP EXTENSION</span> | `cortext_command_vault/spec/full-collision-physics-plan.md`, `cortext_command_vault/decisions/dr-033-full-collision-physics-direction.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md` |
| P0 | Systemic material simulation direction (T-MAT) | Hybrid systemic materials: bounded active-region per-pixel CA kernel (`cx-material`) + Barotrauma-style hull/atmosphere networks (`cx-atmos`) + data-driven reaction table + per-actor affordance/affliction layer; curated 17-material launch set; expansion via material lab; AI material competence + replay determinism + server-authoritative; T-MAT side track + M5.6/M5.7/M6.6/M7.5/M8.5 milestones + 4 new run-bundle event categories | <span class="cc-flag cc-green">DIRECTION CLOSED</span> | `cortext_command_vault/decisions/dr-036-systemic-material-simulation-direction.md`, `cortext_command_vault/comparables/noita-grade-material-simulation-research.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/spec/native-implementation-backlog.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md` |
| P0 | Historical HTML/Slice-A backlog / A0..A7 task cards | Browser-era First 48 Hours checklist + task cards + evidence gates + A5 equipment/loadout workbench handoff | <span class="cc-flag cc-yellow">HISTORICAL BACKLOG</span> | `cortext_command_vault/spec/prototype-implementation-backlog-slice-a.md`, `cortext_command_vault/spec/prototype-roadmap.md`, `cortext_command_vault/references/prototype-run-bundle-schema.md`, `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md` |
| P0 | Prototype run-bundle schema/checker | Manifest/event/summary schemas + cross-file checker for future prototype evidence | <span class="cc-flag cc-blue">READY + A1 UI/LOAD-A/MATERIAL/PROBE/LOAD-W/FIXTURE TRAVERSAL SMOKE PASS</span> | `cortext_command_vault/references/prototype-run-bundle-schema.md`, `research_tools/prototype_run_check.py`, `cortext_command_vault/prototypes/actor-feel-lab-a0-bootstrap.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-runtime-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-ui-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md`, `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md`, `prototype_runs/actor_feel_lab/a1_ui_smoke_2026-05-04_1322/`, `prototype_runs/actor_feel_lab/a1_load_w_workbench_smoke_2026-05-04_1503/`, `prototype_runs/actor_feel_lab/a1_load_w_fixture_switch_smoke_2026-05-04_1517/`, `prototype_runs/actor_feel_lab/a1_load_w_fixture_tab_input_smoke_2026-05-04_1558/`, `prototype_runs/actor_feel_lab/a1_load_w_fixture_traversal_smoke_2026-05-04_1610/` |
| P0 | Backend service/hub Slice A requirements | Server browser schema + join eligibility + local supervisor + backend events | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/backend-service-hub-slice-a.md`, `cortext_command_vault/decisions/dr-013-backend-service-scope.md`, `cortext_command_vault/decisions/dr-005-multiplayer-posture.md`, `cortext_command_vault/decisions/dr-006-modding-data-model.md` |
| P0 | Package builder/workbench Slice A requirements | Deterministic package builder + diagnostics + provenance + loader graph + migration + test-launch | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/package-builder-workbench-slice-a.md`, `cortext_command_vault/engine/content-module-loading-lifecycle.md`, `cortext_command_vault/decisions/dr-006-modding-data-model.md`, `cortext_command_vault/decisions/dr-010-license-reuse-matrix.md` |
| P0 | Decision records | DR-001..DR-036 | <span class="cc-flag cc-green">DONE / ACTIVE</span> | `cortext_command_vault/decisions/index.md` |
| P0 | Comparable repo workspace | `comparables_repos/` workspace + plan | <span class="cc-flag cc-green">STARTED</span> | `comparables_repos/README.md`; OpenSoldat core/satellites, The Powder Toy, and OpenLieroX cloned. |
| P0 | Runtime AI trust harness requirements | Replayable bot test harness manifest + events + reports | <span class="cc-flag cc-blue">READY TO BUILD</span> | `cortext_command_vault/spec/ai-trust-harness-slice-a.md`, `cortext_command_vault/systems/ai-trust-test-suite.md`, `cortext_command_vault/engine/ai-order-lifecycle.md` |
| P1 | First playable slice prototype | Working sandbox per DR-004 | <span class="cc-flag cc-orange">PENDING</span> | `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md`, `cortext_command_vault/decisions/dr-004-first-playable-slice.md` |
| P1 | Comparable repo audits | Per-repo audit notes from cloned sources | <span class="cc-flag cc-yellow">PARTIAL</span> | `cortext_command_vault/comparables/opensoldat-local-audit.md`, `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md`, `cortext_command_vault/comparables/the-powder-toy-local-audit.md`, `cortext_command_vault/comparables/openlierox-local-audit.md`, `cortext_command_vault/comparables/audit-template.md` |
| P1 | UX wireframe prototypes | Low-fidelity HUD/squad/command/buy/replay/hub/workbench prototypes | <span class="cc-flag cc-orange">PENDING</span> | `cortext_command_vault/spec/ux-wireframes-slice-a.md`, `cortext_command_vault/systems/ux-overlay-screen-brief.md` |
| P1 | Progression/retention loop requirements | Intrinsic-first retention DR + progression objects + RET-A tests | <span class="cc-flag cc-blue">EXPLORATORY REQS</span> | `cortext_command_vault/decisions/dr-011-progression-retention-loop.md`, `cortext_command_vault/spec/progression-retention.md`, `cortext_command_vault/systems/ux-ui-and-retention.md` |
| P2 | Game spec section | Authoritative v0 spec plus supporting subsystem pages | <span class="cc-flag cc-green">AUTHORITATIVE V0</span> | `cortext_command_vault/spec/authoritative-game-spec-v0.md`, `cortext_command_vault/spec/index.md`, `cortext_command_vault/dashboards/research-readiness.md` |

## Current Status

| Area | Status | Evidence | Next Gap |
|---|---|---|---|
| Repo inventory | DONE | `cortext_command_vault/repos/index.md` | Keep commit snapshots current if repos change. |
| Source/reference list | DONE enough | `cortext_command_vault/references/sources.md` | Add article snapshots only when a spec claim needs exact citation. |
| Direct actor-control lifecycle | DONE | `cortext_command_vault/engine/direct-control-and-actor-feel-lifecycle.md` | Use it while building Slice A control/replay hooks. |
| Projectile lifecycle | DONE | `cortext_command_vault/engine/projectile-to-impact-lifecycle.md` | Tied to body damage downstream. |
| Body damage / wound / gib model | PROTOTYPE REQS | `cortext_command_vault/spec/body-damage-model.md`, `cortext_command_vault/engine/body-damage-wound-gib-lifecycle.md`, `cortext_command_vault/research-log/2026-05-04-body-damage-model-spec.md` | Run HUD-01..HUD-03 and BODY-A-01..BODY-A-12 against actor-feel, replay, equipment, AI, and UX prototypes. |
| Chassis/armor/mechs/origins | EXPLORATORY REQS | `cortext_command_vault/spec/chassis-armor-mechs-and-origins.md`, `cortext_command_vault/decisions/dr-014-tone-player-promise.md`, `cortext_command_vault/research-log/2026-05-04-tone-chassis-mech-promise.md` | Add CHASSIS-A prototype tasks after actor/body/equipment basics: local armor stages, equipment degradation, enterable mech, android/robot origin, AI labels, and replay cause chains. |
| Terrain/pathfinding lifecycle | DONE | `cortext_command_vault/engine/terrain-mutation-and-pathfinding-lifecycle.md` | Prototype dirty-region/path invalidation behavior. |
| Activity/scenario lifecycle | DONE | `cortext_command_vault/engine/activity-scenario-lifecycle.md`, `cortext_command_vault/research-log/2026-05-04-mission-director-slice-a.md` | MetaFight save-schema finding is resolved; use it when building the typed mission manifest. |
| AI order lifecycle | DONE | `cortext_command_vault/engine/ai-order-lifecycle.md` | Build runtime test harness and failure replay flow. |
| Loadout/delivery lifecycle | DONE | `cortext_command_vault/engine/loadout-delivery-economy-lifecycle.md`, `cortext_command_vault/spec/ux-wireframes-slice-a.md`, `cortext_command_vault/spec/equipment-loadout.md` | Run UX-W-06/UX-W-07 and LOAD-A-09 once buy/loadout/delivery UI exists. |
| Equipment/loadout model requirements | READY TO BUILD | `cortext_command_vault/spec/equipment-loadout.md`, `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md`, `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md`, `cortext_command_vault/references/equipment-source-trace-slice-a.md`, `cortext_command_vault/references/equipment-role-card-renderer-view-slice-a.md`, `cortext_command_vault/references/equipment-cccp-field-map.md`, `cortext_command_vault/references/equipment-role-design-deep-dive.md`, `cortext_command_vault/references/equipment-capability-authoring-matrix.md`, `cortext_command_vault/references/equipment-ai-behavior-contract.md`, `cortext_command_vault/references/equipment-ai-summary-seed-slice-a.md`, `cortext_command_vault/references/equipment-consumer-traceability-matrix.md`, `cortext_command_vault/references/equipment-consumer-traceability-slice-a.md`, `cortext_command_vault/references/equipment-trace-tab-view-slice-a.md`, `cortext_command_vault/references/equipment-role-cards-slice-a.md`, `cortext_command_vault/references/equipment-overlap-audit-slice-a.md`, `cortext_command_vault/references/equipment-overlap-resolution-worksheet-slice-a.md`, `cortext_command_vault/references/equipment-corpus-cccp.md`, `cortext_command_vault/references/equipment-schema-and-overlays.md`, `cortext_command_vault/references/equipment-overlay-review-matrix.md`, `cortext_command_vault/references/equipment-manual-overlay-patches.md`, `cortext_command_vault/references/equipment-overlay-merged-preview.md`, `cortext_command_vault/references/equipment-provenance-workbench-view.md`, `cortext_command_vault/references/equipment-loadout-fixtures-slice-a.md`, `cortext_command_vault/references/equipment-ai-scenarios-slice-a.md`, `cortext_command_vault/references/equipment-package-diagnostics-slice-a.md`, `cortext_command_vault/research-log/2026-05-04-equipment-role-record-contract.md`, `cortext_command_vault/research-log/2026-05-04-equipment-source-trace.md`, `cortext_command_vault/research-log/2026-05-04-equipment-loadout-model.md`, `cortext_command_vault/research-log/2026-05-04-equipment-field-map.md`, `cortext_command_vault/research-log/2026-05-04-equipment-ai-behavior-contract.md`, `cortext_command_vault/research-log/2026-05-04-equipment-ai-summary-seed.md`, `cortext_command_vault/research-log/2026-05-04-equipment-consumer-traceability.md`, `cortext_command_vault/research-log/2026-05-04-equipment-trace-tab-view.md`, `cortext_command_vault/research-log/2026-05-04-equipment-loadout-workbench-slice-a.md`, `cortext_command_vault/research-log/2026-05-04-equipment-capability-authoring-matrix.md`, `cortext_command_vault/research-log/2026-05-04-equipment-renderer-view-model.md`, `cortext_command_vault/research-log/2026-05-04-equipment-overlap-resolution-worksheet.md`, `cortext_command_vault/research-log/2026-05-04-equipment-role-card-renderer-slice-a.md`, `cortext_command_vault/research-log/2026-05-04-equipment-role-cards-overlap-audit.md`, `cortext_command_vault/research-log/2026-05-04-equipment-copyof-resolution.md`, `cortext_command_vault/research-log/2026-05-04-equipment-provenance-warning-metadata.md`, `cortext_command_vault/research-log/2026-05-04-equipment-provenance-workbench-view.md`, `cortext_command_vault/research-log/2026-05-04-equipment-role-design-deep-dive.md` | Field-level CCCP translation, implementation-facing role-record contract, concrete role-card design, capability authoring matrix, AI behavior contract, generated AI summary seed, consumer traceability matrix/report, generated trace-tab view, generated source trace, generated 106-card role dataset, generated LOAD-R renderer view, generated 10-group overlap audit, overlap resolution worksheet, role-card renderer contract, interactive LOAD-W workbench requirements, common `CopyOf` cleanup, field provenance, structured warning details, patch application preview, provenance workbench view, AI scenario seeds, and package diagnostic expected output with consumer-impact labels now exist; next implement interactive LOAD-W/LOAD-R/LOAD-A fixture-backed AI/UX/package/balancing/replay/backend hooks, LOAD-FIELD-SOURCE source inspector, AI-EQ item-choice/refusal labels, AI-EQ-SUMMARY harness promotion, and resolve high/medium overlap groups. |
| Terrain networking lifecycle | DONE | `cortext_command_vault/engine/network-terrain-replication-lifecycle.md` | Measure or prototype authority, bandwidth, and replay/event implications. |
| Backend service/hub Slice A requirements | READY TO BUILD | `cortext_command_vault/spec/backend-service-hub-slice-a.md`, `cortext_command_vault/decisions/dr-013-backend-service-scope.md` | Implement static/heartbeat backend, hub browser, join resolver, package compatibility, local supervisor, backend events, and BACK-SCOPE checks under the local-first service-spine boundary. |
| Replay/event architecture | DONE | `cortext_command_vault/systems/replay-event-architecture.md`, `cortext_command_vault/spec/replay-recorder-slice-a.md`, `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` | Implement recorder + viewer prototype with hybrid events/snapshots/checksums; prove deterministic islands before claiming them. |
| Destruction-objective patterns | DONE | `cortext_command_vault/systems/destruction-objective-mission-patterns.md` | Build first proof mission. |
| Mission/director Slice A | READY TO BUILD | `cortext_command_vault/spec/mission-director-slice-a.md`, `cortext_command_vault/spec/missions-and-objectives.md`, `cortext_command_vault/research-log/2026-05-04-mission-director-slice-a.md` | Implement Breach Contract manifest, commander reason strings, director intensity sampler, LZ scorer, save/replay roundtrip, and mission strip in the loadout workbench. |
| Content module loading lifecycle | DONE + GENERATED GRAPH | `cortext_command_vault/engine/content-module-loading-lifecycle.md`, `cortext_command_vault/references/content-loader-graph-cccp.md`, `cortext_command_vault/references/equipment-source-trace-slice-a.md`, `cortext_command_vault/spec/modding-model.md` | Add CONTENT-A fixture modules and consume generated equipment source-position joins in LOAD-W/source-inspector and package-builder diagnostics. |
| Modding workbench brief | DONE | `cortext_command_vault/systems/modding-package-and-workbench.md`, `cortext_command_vault/engine/content-module-loading-lifecycle.md` | Build workbench V1 prototype against loader graph requirements. |
| Package builder/workbench Slice A requirements | READY TO BUILD | `cortext_command_vault/spec/package-builder-workbench-slice-a.md`, `cortext_command_vault/engine/content-module-loading-lifecycle.md` | Build deterministic package output, validation, provenance scanner, loader graph, graphs, migration preview, registry summary, and test-launch loop. |
| Material/mobility schema proposal | DRAFT | `cortext_command_vault/systems/material-and-mobility-affordance-schema.md`, `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md` | Implement MAT-T-01..MAT-T-10 and feed DR-007. |
| UX overlay/screen brief | DONE + requirements ready | `cortext_command_vault/systems/ux-overlay-screen-brief.md`, `cortext_command_vault/spec/ux-wireframes-slice-a.md` | Build low-fidelity prototypes and run UX-W-01..UX-W-16 against actor/replay/material/backend/package slices. |
| Decision records | DONE / ACTIVE (DR-001..DR-036) | `cortext_command_vault/decisions/index.md` | Resolve DRs as evidence accumulates. |
| Comparable web/source pass | DONE | `cortext_command_vault/comparables/research-pass-2-open-source-systems.md` | Clone and audit local comparable code. |
| Comparable repo workspace | STARTED | `comparables_repos/README.md`; `comparables_repos/opensoldat`; `comparables_repos/opensoldat-base`; `comparables_repos/opensoldat-launcher`; `comparables_repos/opensoldat-lobby`; `comparables_repos/the-powder-toy`; `comparables_repos/openlierox` | PolyWorks/OpenLiero only when needed. |
| Comparable repo audits | PARTIAL | `cortext_command_vault/comparables/opensoldat-local-audit.md`; `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md`; `cortext_command_vault/comparables/the-powder-toy-local-audit.md`; `cortext_command_vault/comparables/openlierox-local-audit.md`; `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md`; `cortext_command_vault/spec/backend-service-hub-slice-a.md`; `cortext_command_vault/spec/package-builder-workbench-slice-a.md`; `cortext_command_vault/spec/ux-wireframes-slice-a.md` | Convert remaining audit findings into implementation tickets and prototype results. |
| AI trust suite | DRAFT + requirements ready | `cortext_command_vault/systems/ai-trust-test-suite.md`, `cortext_command_vault/spec/ai-trust-harness-slice-a.md` | Implement runnable harness. |
| Usage ledger (reuse tracking) | EMPTY | `cortext_command_vault/references/usage-ledger.md` | Add entries as reuse happens. |
| Native prototype roadmap | IMPLEMENTATION BRIDGE | `cortext_command_vault/spec/prototype-roadmap.md` | Use it as the strategic build ladder: M0 engine bootstrap, T-CONTROL observability, M1 actor, M1.5 Micro Breach fun proof, M2 terrain, M3 replay, M4 HUD, M5 chassis/equipment, M5.5 collision, M5.6/M5.7 materials/hazards, M6 AI, M6.5 LLM mind, M6.6 AI material competence, M7 Breach Contract, M7.5 atmospherics, M8 editor/mods, M8.5 material lab, M9 `cx-server`, M10 LAN, M11 self-hosted online co-op, M12 PvP/MMO readiness. |
| Native implementation backlog | PRIMARY IMPLEMENTATION BACKLOG | `cortext_command_vault/spec/native-implementation-backlog.md` | Use it as the AI-agent task board: M0..M12 task cards, crate ownership, validation commands, E2E proof, `cxctl`/control-observation evidence, run-bundle evidence, anti-scope, and result-writing rules. |
| Feature completion/rating checklist | LIVE CHECKLIST | `cortext_command_vault/spec/feature-completion-checklist.md` | Use it after each implementation pass to check exact rows, link evidence, record AI self-ratings, and leave human rating columns for the user. |
| AI control/observability layer | CROSS-CUTTING REQUIREMENT | `cortext_command_vault/spec/ai-control-observability-layer.md` | Keep this built in from M0: structured observations, collision observations, semantic actions, UI tree, local control server, CLI scripts, future bot API. |
| Hybrid LLM AI plan | ROADMAP EXTENSION | `cortext_command_vault/spec/hybrid-llm-ai-plan.md` | Add as T-LLM plus M6.5 after local M6 AI: optional cloud/local LLM mind workers propose doctrine, memory, profile, debrief, and commander-adaptation patches through strict schemas, validation, replay logging, and deterministic fallback. |
| Full collision physics plan | ROADMAP EXTENSION | `cortext_command_vault/spec/full-collision-physics-plan.md` | Add as T-PHYS plus M5.5 between chassis/equipment and AI: collision matrix, limb/body/equipment/mech/base/projectile contacts, projectile-projectile, CCD tiers, impulse-to-damage, collision observation, replay, and perf gates. |
| Historical HTML/Slice-A implementation backlog | HISTORICAL BACKLOG | `cortext_command_vault/spec/prototype-implementation-backlog-slice-a.md` | Browser-era A0..A7 task cards; use only for old actor-feel-lab context unless explicitly assigned. |
| Prototype run-bundle schema/checker | READY + A1 UI/LOAD-A/MATERIAL/PROBE/RESPONSE/LOAD-W/FIXTURE TRAVERSAL SMOKE PASS | `cortext_command_vault/references/prototype-run-bundle-schema.md`; `research_tools/prototype_run_check.py`; `cortext_command_vault/prototypes/actor-feel-lab-a1-ui-smoke.md`; `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md`; `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md`; `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md`; `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md`; `prototype_runs/actor_feel_lab/a1_ui_smoke_2026-05-04_1322/`; `prototype_runs/actor_feel_lab/a1_load_w_workbench_smoke_2026-05-04_1503/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_switch_smoke_2026-05-04_1517/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_tab_input_smoke_2026-05-04_1558/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_traversal_smoke_2026-05-04_1610/` | A1 smokes validate event-family, screenshot-artifact, selected LOAD-A item state, material-grid edits, checksum/path-refresh events, bounded path/collision query events, bounded actor-hull response events, probe overlay visibility, refusal plumbing, generated LOAD-W trace/source/AI/diagnostic render plumbing, all-fixture imports, query-param fixture-switch data routing, fixture-tab click/focused-keyboard input routing, and fixture-control Tab/Arrow/Enter/Space traversal; next keep schemas/checker aligned with manual runs, integrated movement collision, full workbench traversal, physical gamepad input, 200% text scale, AI-H scoring, and replay/export output. |
| Actor-feel prototype workspace | A1 UI/LOAD-A/MATERIAL/PROBE/RESPONSE/LOAD-W/FIXTURE TRAVERSAL SMOKE PASS | `prototype_workspaces/actor_feel_lab/`; `prototype_runs/actor_feel_lab/a1_ui_smoke_2026-05-04_1322/`; `prototype_runs/actor_feel_lab/a1_load_w_workbench_smoke_2026-05-04_1503/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_switch_smoke_2026-05-04_1517/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_tab_input_smoke_2026-05-04_1558/`; `prototype_runs/actor_feel_lab/a1_load_w_fixture_traversal_smoke_2026-05-04_1610/`; `cortext_command_vault/prototypes/index.md`; `cortext_command_vault/research-log/2026-05-04-a1-ui-smoke-capture.md`; `cortext_command_vault/research-log/2026-05-04-a1-load-w-workbench-smoke.md`; `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-switch-smoke.md`; `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-tab-input-smoke.md`; `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-traversal-smoke.md` | Run manual feel notes, turn bounded actor-hull responses into integrated movement collision and global path planner evidence, expand from fixture-control traversal into full workbench traversal and physical gamepad proof, and feed imported LOAD-A/LOAD-W events into AI-H/replay/export. |

## Current Execution Order

1. **Use the native roadmap, native backlog, feature checklist, AI control layer, full collision plan, hybrid LLM AI plan, and run-bundle schema as the implementation bridge**: follow `cortext_command_vault/spec/prototype-roadmap.md` for strategic M0..M12 build order, `cortext_command_vault/spec/native-implementation-backlog.md` for AI-agent task cards, `cortext_command_vault/spec/feature-completion-checklist.md` for completion/rating updates, `cortext_command_vault/spec/ai-control-observability-layer.md` for `cxctl`/structured observation requirements, `cortext_command_vault/spec/full-collision-physics-plan.md` for the T-PHYS/M5.5 physical consequence contract, `cortext_command_vault/spec/hybrid-llm-ai-plan.md` for the optional T-LLM/M6.5 async mind-layer extension after local M6 AI, and `research_tools/prototype_run_check.py` for run-folder validation. The old A0..A7 browser backlog remains available only for historical actor-feel-lab context.
2. **Build M0 -> M1 -> M1.5 first**: create the native Rust/Bevy/wgpu workspace plus `cx-control`/`cxctl`, make one actor playable and controllable through semantic actions, then build the Micro Breach fun slice with one reactive enemy, one soft breach surface, one 60-90s objective, HUD, control-driven E2E win/loss scripts, and checked run bundles before broadening into full M2 terrain.
3. **Treat DR-001 as direction-closed and build greenfield native first**: CCCP remains a read-only reference lab, not the implementation target. `cortext_command_vault/engine/cccp-build-run-audit.md` confirms the active repo builds/runs far enough for reference work; `cortext_command_vault/engine/cccp-runtime-window-capture-troubleshooting.md` records the remaining visual-proof gap. Do not block M0/M1 on more CCCP runtime proof unless a worker is explicitly assigned reference-validation work.
4. **Convert comparable source audits into prototype requirements**: OpenSoldat core/satellites, The Powder Toy, and OpenLieroX first passes are done; actor-feel, recorder, AI harness, terrain/material, backend service/hub, package-builder/workbench, UX wireframes, and equipment/loadout requirements now exist; next begin implementation or split these requirements into tickets.
5. **Stand up the recorder + viewer** from `cortext_command_vault/spec/replay-recorder-slice-a.md` and `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` for the replay/event architecture (DR-002) so AI/UX/death/mission tests are debuggable from day one and deterministic claims are backed by checksums/snapshots.
6. **Build the Breach Contract proof mission** from `cortext_command_vault/spec/mission-director-slice-a.md`: typed manifest, commander reason strings, director intensity sampler, LZ scorer, mission capability strip, save/replay events, and MISSION-A-01..18.
7. **Run AI trust scenarios** using `cortext_command_vault/spec/ai-trust-harness-slice-a.md`: first AI-H-01..AI-H-06 against actor-feel + recorder, then AI-01..AI-12 and commander scenarios against the minimal breach mission; close DR-008 when evidence supports it.
8. **Build UX wireframe prototypes** from `cortext_command_vault/spec/ux-wireframes-slice-a.md` and run UX-W-01..UX-W-16 plus HUD-01..HUD-03, ORDER-01, BUY-01, LOAD-A, LOAD-W mission strip, and LOAD-R/LOAD-009/LOAD-010 buy/loadout tests from `cortext_command_vault/spec/equipment-loadout.md`, `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md`, `cortext_command_vault/spec/mission-director-slice-a.md`, `cortext_command_vault/references/equipment-role-cards-slice-a.md`, `cortext_command_vault/references/equipment-overlap-audit-slice-a.md`, and `cortext_command_vault/references/equipment-overlay-review-matrix.md`.
9. **Run progression/retention RET-A tests** from `cortext_command_vault/spec/progression-retention.md` once actor-feel, recorder, loadout, and one repeatable contract exist.
10. **Resolve remaining DRs as evidence accumulates**; promote claims into the `spec/` section as their gates close.
11. **Track reuse in the usage ledger** as it happens — never blocks work, just keeps the public-release option open.

## Research Tracks

| Track | Questions To Answer | Output | Status |
|---|---|---|---|
| Core feel | What makes direct actor control satisfying or frustrating? | Controller feel memo and prototype checklist. | Local control lifecycle now captured in `cortext_command_vault/engine/direct-control-and-actor-feel-lifecycle.md`; requirements captured in `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md`; A1 runtime/UI smokes exist in `cortext_command_vault/prototypes/actor-feel-lab-a1-runtime-smoke.md` and `cortext_command_vault/prototypes/actor-feel-lab-a1-ui-smoke.md`; needs manual feel run and terrain/equipment behavior expansion. |
| Terrain/destruction | How do terrain pixels, materials, penetration, debris, hazards, air/heat fields, path updates, and movement/tool affordances actually work? | Terrain/destruction sequence and replacement-design options. | Draft schema and buildable terrain/material Slice A requirements now exist in `cortext_command_vault/systems/material-and-mobility-affordance-schema.md` and `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md`; implementation still needed. |
| Damage/body model | How do wounds, gibs, impulses, limbs, inventory, treatment, AI rescue/refusal, replay/death recap, and death states interact? | Damage-channel design brief, body-state UI proposal, event contract, AI reason-label contract, and BODY-A acceptance tests. | Requirements ready in `cortext_command_vault/spec/body-damage-model.md`; HUD/body-state/replay/AI prototype tests still needed. |
| Equipment/loadouts | What item roles exist and what is missing? | Device taxonomy, exact field atlas, source-anchored device snapshots, comparable design patterns, capability matrix, AI behavior contract, generated AI summary seed, consumer traceability, generated consumer traceability report, generated trace-tab view, generated source trace, generated role cards, renderer view, renderer contract, interactive workbench spec, first LOAD-W browser smoke, first fixture-switch smoke, first fixture-tab input smoke, first fixture-control traversal smoke, overlap audit, overlap worksheet, loadout UX model, role tags, AI metadata, delivery risk, mod validation. | Requirements ready in `cortext_command_vault/spec/equipment-loadout.md`, `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md`, and `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md`; field map, device/loadout field atlas, source-anchored snapshots, comparable design patterns, role design deep dive, capability authoring matrix, AI behavior contract, generated AI summary seed, consumer traceability matrix, generated LOAD-011 report, generated LOAD-W-010 trace-tab view, generated LOAD-FIELD-SOURCE source trace, role-card dataset, renderer view, overlap audit, overlap resolution worksheet, first generated corpus, schema/overlay seed, review matrix, manual patch layer, merged preview, provenance view, fixture data, AI scenario seeds, package expected diagnostics, first browser LOAD-W render smoke in `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md`, first query-param fixture-switch smoke in `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md`, first fixture-tab click/keyboard smoke in `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md`, and first fixture-control Tab/Arrow/Enter/Space traversal smoke in `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md`; full workbench traversal, physical gamepad input, AI-EQ item-choice/refusal labels, AI-EQ-SUMMARY harness promotion, LOAD-FIELD/LOAD-FIELD-SOURCE/LOAD-COMP drill-downs, 200% text-scale checks, replay/export preview, and overlap-resolution decisions still needed. |
| AI/bots | What must friendly/enemy bots do to make solo play strong? | AI trust test suite, goal architecture, debug overlay requirements, runnable harness requirements. | Requirements ready; implementation missing. |
| Activities/objectives | Which mission structures embrace destructible terrain, commander AI, and equipment capability planning? | Destruction-objective brief, typed mission/director Slice A requirements, mission pattern catalog, and Breach Contract proof target. | Requirements ready; first proof mission implementation still needed. |
| Networking | What can be reused or learned from CCCP/C4/OpenSoldat/OpenLieroX? | Multiplayer/server feasibility memo and architecture recommendation. | Partial; OpenSoldat adds snapshot/delta caution plus launcher/lobby/package-hash lessons; OpenLieroX adds carve/rope/NewNet caution; [[spec/server-app-architecture]], [[spec/persistent-mmo-architecture]], [[spec/backend-service-hub-slice-a]], and [[systems/replay-determinism-and-run-evidence]] now feed event-volume/checksum/snapshot/server evidence; PvP/MMO readiness is proven by M12 gates. |
| UX/UI | What information does the player need while controlling, commanding, buying, and building? | Screen inventory, overlay spec, command UX brief, wireframe requirements, accessibility/comfort floors. | Requirements ready in `cortext_command_vault/spec/accessibility-comfort-slice-a.md`; prototypes missing. |
| Modding/tools | How do creators author, validate, migrate, and test content? | Mod package model, schema plan, editor/workbench spec. | Package-builder/workbench Slice A requirements now exist; implementation missing. |
| Progression/retention | What keeps players coming back without relying on obligation or gacha-led pressure? | DR-011 + exploratory spec: intrinsic-first hybrid, campaign, daily seeds, veterans, salvage, replays, creator challenges, RET-A tests. | Exploratory requirements ready; prototype tests missing. |
| Production feasibility | Which systems are expensive, risky, or must be scoped? | Risk register and prototype kill criteria. | Needs estimates. |

## Code Archaeology Checklist

| System | Files / Areas | Current State | Next Deliverable |
|---|---|---|---|
| Main update loop | CCCP `Source/Main.cpp`, managers, `Actor::FullUpdate` | Direct actor control lifecycle now has a local update-order slice. | Broader frame/update order diagram. |
| Data loading | `PresetMan`, `.rte`, `Index.ini`, `DataModule`, `Reader`, `Entity::CopyOf`, `LuaMan`, zipped modules | Content module loading lifecycle and generated loader graph now capture active module order, include/script/source counts, duplicate preset pressure, CONTENT-A coverage, and package/equipment implications. | Add package-builder parity fixtures and join equipment source positions to role cards. |
| Terrain/materials | `SLTerrain`, `SceneMan`, `Material`, `Materials.ini`, `PathFinder`, `ADoor` | Lifecycle note and Slice A requirements exist. | Implement terrain/material sandbox and log MAT-T results. |
| Collision/physics | `Atom`, `AtomGroup`, `MOSRotating`, `MOPixel` | Partial through projectile/physics notes. | Collision and body interaction map. |
| Weapons/damage | `HDFirearm`, `ThrownDevice`, `TDExplosive`, wounds/gibs | Projectile note exists; body trace incomplete. | Wound/gib/death lifecycle. |
| Actor/inventory | `AHuman`, `Actor`, `Loadout`, `MovableMan`, `HeldDevice`, `HDFirearm`, `Magazine`, `Round`, device `.ini` groups | Loadout lifecycle, equipment/loadout requirements, role-card renderer requirements, CCCP field map, device/loadout field atlas, source-anchored snapshots, role design deep dive, capability authoring matrix, AI behavior contract, generated AI summary seed, consumer traceability matrix, generated source trace, generated role cards, generated overlap audit, first generated corpus, schema/overlay seed, review matrix, manual patch layer, merged preview, provenance view, fixture data, AI scenario seeds, package diagnostics, and first LOAD-A runtime/material-grid/probe/actor-response event-contract smoke exist. | Turn LOAD-A bounded actor-hull responses into integrated movement collision, global path planner, and AI-H evidence, then consume field map/field atlas/source snapshots/capability matrix/AI behavior contract/AI summary seed/traceability matrix/source trace/role cards/overlap audit/merged preview/provenance view/scenarios/diagnostics in LOAD-R/LOAD-W/LOAD-FIELD/LOAD-FIELD-SOURCE/LOAD-009/LOAD-010/AI-H-LOAD/AI-EQ/AI-EQ-SUMMARY/PACK-014/REC-A-LOAD tests. |
| AI/pathfinding | `Actor`, `PathFinder`, `Scene`, `Data/Base.rte/AI`, activities | AI lifecycle exists. | Runtime harness and path failure replay. |
| Activities | `ActivityMan`, `GAScripted`, Lua activities, `MetaFight.lua`, `BunkerBreach.lua`, `LandingZoneMap.lua`, `TacticsHandler.lua` | Activity lifecycle, MetaFight save finding, destruction-objective patterns, and mission/director Slice A requirements now exist. | Implement Breach Contract and run MISSION-A-01..18 with recorder/AI/loadout evidence. |
| Networking | CCCP/C4 `NetworkClient`, `NetworkServer`, message structs, C4 NAT punch | Terrain replication note exists. | Authority/bandwidth/replay feasibility memo. |
| Tooling | VS Code extension, legacy converter, website | Repo notes exist. | Creator tooling and migration plan. |

## Comparable Repo Workspace

The sibling directory `comparables_repos/` exists with a `README.md` plan and `.gitignore`. OpenSoldat core, The Powder Toy, and OpenLieroX are cloned and have first-pass local audits. Remaining clone targets and order are documented there. The vault stays notes-only; the comparable workspace stays code-only.

| Comparable | Clone Target | Inspect First | Why |
|---|---|---|---|
| OpenSoldat | `comparables_repos/opensoldat` | First pass: `shared/mechanics`, `shared/network`, `shared/AI.pas`, `shared/Demo.pas`, `client/InterfaceGraphics.pas` | Shooter feel, bots, netcode modernization. See `cortext_command_vault/comparables/opensoldat-local-audit.md`. |
| OpenSoldat base | `comparables_repos/opensoldat-base` | First pass: `README.md`, `create_smod.py`, content layout, `shared/mod.ini` | Content/package structure and licensing boundary. See `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md`. |
| OpenSoldat launcher | `comparables_repos/opensoldat-launcher` | First pass: Electron/React shell, lobby/local/demos/settings, process spawn, `soldat://`, mods/interfaces | Frontend/launcher reference. See `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md`. |
| OpenSoldat lobby | `comparables_repos/opensoldat-lobby` | First pass: Go HTTP API, static server JSON, CORS, TODOs | Backend/lobby reference. See `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md`. |
| PolyWorks | `comparables_repos/polyworks` | map editor workflow | Creator-tool UX. |
| OpenLiero | `comparables_repos/openliero` | weapons, terrain, movement | Compact destructible arena baseline. |
| OpenLieroX | `comparables_repos/openlierox` | First pass: `src/common/PhysicsLX56.cpp`, `src/common/CMap.cpp`, `src/client/CClient_Game.cpp`, `src/common/CGameScript.cpp`, `src/common/CWormBot.cpp`, `include/Protocol.h`, `src/common/NewNetEngine.cpp`, `src/gusanos/*`, `share/gamedir/` | Long-lived modded multiplayer reference. See `cortext_command_vault/comparables/openlierox-local-audit.md`. |
| The Powder Toy | `comparables_repos/the-powder-toy` | First pass: `src/simulation`, element update rules, Lua API, save format, snapshots/deltas, community backend | Material simulation and sandbox UI reference. See `cortext_command_vault/comparables/the-powder-toy-local-audit.md`. |

Each local audit should produce one note with:

- Code paths inspected.
- Mechanics learned.
- Architecture learned.
- License/reuse caution.
- What to copy as a design idea.
- What not to copy.
- Prototype implication.

## Decision Records Needing Evidence

The actual decision records live in `cortext_command_vault/decisions/`. This table lists records that still need prototype/build evidence before they can close. Closed-direction records DR-001, DR-005, DR-013, and DR-014..DR-036 are summarized in `cortext_command_vault/decisions/index.md` and `cortext_command_vault/dashboards/decision-tracker.md`.

| ID | Title | Lean | Status |
|---|---|---|---|
| DR-002 | Replay/event architecture | Hybrid event log + snapshots | Open |
| DR-003 | Body damage readability | Hybrid: silhouette default + advanced HUD opt-in | Open |
| DR-004 | First playable slice | Sequenced single actor → squad → bunker breach | Open |
| DR-006 | Modding data model | Schema-first + Lua escape hatches + workbench | Open |
| DR-007 | Terrain/material model | Prototype solids + curated hazards first; systemic direction now active per DR-036 (chunked CA + reaction table + atmospheres) | Open |
| DR-008 | AI architecture | Hybrid jobs + utility scoring + scripted hooks | Open |
| DR-009 | Command UX style | Direct control + slowdown overlay + optional tactical map | Open |
| DR-010 | License/reuse posture | Personal use unrestricted; ledger tracks usage; legal review only at public-release time | Open |
| DR-011 | Progression/retention loop | Intrinsic-first hybrid: mastery, autonomy, veterans, salvage, replays, creator challenges | Open |
| DR-012 | Accessibility, comfort, and readability floor | Slice A accessibility/comfort floor, not late compliance | Open |

Still pending topics without a DR: localization plan, networking transport choice, modding script host, and cloud-save backend. Monetization/economy and audio identity are already direction-closed in DR-031 and DR-020.

## AI Trust Gates

The detailed AI test list lives in `cortext_command_vault/systems/ai-trust-test-suite.md`. This plan should not duplicate it.

The spec cannot claim "great solo AI" as a settled promise until these gates are satisfied. This does not block ambitious AI research or private prototypes.

| Gate | Minimum Requirement |
|---|---|
| Scenario coverage | Defend, breach, dig, recover from blocked path, rescue, retreat, avoid blast/delivery danger, use equipment, avoid friendly fire. |
| Replay/debug | Every failed test can be replayed with current order, intent, target, path state, tool score, alarm confidence, stuck timer, and recovery action. |
| Terrain awareness | Tests include doors, destructible walls, collapsed tunnels, fresh breaches, falling/landing hazards, and blocked paths. |
| Player trust | Failures are explained in UI, not hidden in logs only. |
| Regression loop | AI changes can be compared across builds with metrics, not anecdotes. |

## UX/UI Work Items

| Surface | Needed Research | Acceptance Test |
|---|---|---|
| Direct HUD | Health/wounds, ammo, stance, item state, danger feedback. | Player can understand why an actor is weak, unsafe, or unable to act within one glance. |
| Squad panel | Roles, jobs, current intent, alerts, quick switch. | Player can find the bot that needs help without cycling randomly. |
| Command overlay | Move/dig/defend/rescue/repair orders, visible path and blocked reason. | Player can see what the bot will try before committing the order. |
| Buy/loadout UI | Role filters, cost, weight, terrain effect, AI competence, delivery risk. | Player can build a squad for a mission without memorizing item names. |
| Material overlay | Integrity, hazard, pathability, support/collapse risk. | Player can tell which terrain can be dug, breached, burned, reinforced, or crossed. |
| Event/replay viewer | Explains major deaths, breaches, collapses, friendly-fire incidents. | Player can answer "why did I lose this unit/objective?" after the fact. |
| Mod workbench | Preset graph, `CopyOf`, validation, sprite/sound preview, test launch. | Creator can detect broken content before booting a full mission. |

## Design Briefs To Write

| Brief | What It Should Decide | Blocks |
|---|---|---|
| AI Trust Brief | What bots must do, how the player commands them, and how failures are explained. | AI architecture, UX overlays, solo-first promise. |
| Destruction Objectives Brief | Mission patterns that assume walls and terrain can be destroyed. | Campaign loop, level design, AI breach tests. |
| Damage And Body Brief | Damage channels, wounds, armor, gibbing, control states, readability. | Combat feel, UI, retention/veteran systems. |
| Equipment Role Brief | Weapon/tool taxonomy, exact CCCP field atlas, source-anchored snapshots, comparable design patterns, concrete role cards, loadout roles, item AI metadata. | Promoted to `cortext_command_vault/spec/equipment-loadout.md`; field map, device/loadout field atlas, source-anchored snapshots, comparable design patterns, role design deep dive, capability authoring matrix, generated role cards, generated overlap audit, first corpus, schema/overlay seed, review matrix, manual patch layer, merged preview, provenance view, fixture data, AI scenario seeds, package expected diagnostics, and interactive workbench requirements in `cortext_command_vault/references/` and `cortext_command_vault/spec/`; next build renderer/workbench and resolve overlap groups. |
| Material System Brief | Material properties, hazards, digging, repair, collapse-lite scope. | Terrain prototype, networking/replay event model. |
| UX Overlay Brief | What simulation info is exposed, when, and how. | AI trust, material readability, replay viewer. |
| Replay/Event Brief | Which outcomes are events, what is deterministic, what needs snapshots. | AI harness, networking, debug tooling, player learning. |
| Modding Tool Brief | Package format, validation, editor features, migration support. | Creator workflow, community longevity. |
| Retention Brief | Campaign loop, contracts, veterans, daily seeds, salvage, replay sharing. | Promoted to `cortext_command_vault/decisions/dr-011-progression-retention-loop.md` and `cortext_command_vault/spec/progression-retention.md`; next run RET-A tests. |
| Multiplayer Feasibility Brief | What online model is realistic, what gets deferred, what must be architected now. | Live online promise and backend scope. |

## Prototype Milestones

The canonical native build order now lives in `cortext_command_vault/spec/prototype-roadmap.md`, the AI-agent task-card handoff lives in `cortext_command_vault/spec/native-implementation-backlog.md`, completion/rating tracking lives in `cortext_command_vault/spec/feature-completion-checklist.md`, the full collision contract lives in `cortext_command_vault/spec/full-collision-physics-plan.md`, and the built-in automation/bot control contract lives in `cortext_command_vault/spec/ai-control-observability-layer.md`. The older Slice-A backlog remains useful as historical HTML-lab evidence, but new implementation agents should start from the native roadmap/backlog/checklist/full-collision/control-layer set. The table below is the higher-level milestone/risk view; use the native backlog for M0..M12 task ownership, validation commands, run-bundle gates, control-observation proof, and final-audit requirements, and use the feature checklist for closeout rows, evidence links, and ratings.

| Milestone | Depends On | Success Criteria | Effort | Risk | Kill Criteria |
|---|---|---|---|---|---|
| Actor feel sandbox | None | One actor is fun to move, aim, fire, fall, recover, dig, repair, trigger explosions, and emit replayable events. See `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md`. | Medium | High | Movement/combat remains unreadable or sluggish after two control iterations. |
| Terrain/material sandbox | Actor feel sandbox, material schema | Bullets, drills, explosions, repair/fill, hazard, and optional tether/grapple pass MAT-T-01..MAT-T-10 from `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md`. | Medium | High | Terrain edits cannot stay performant with AI/path dirty-region hooks or material overlays are unreadable. |
| Replay/event capture | Actor feel sandbox, terrain damage sandbox, `cortext_command_vault/spec/replay-recorder-slice-a.md` | Major outcomes can be reviewed and explained. | Medium | High | Events cannot reconstruct enough context to debug AI/death/terrain failures. |
| Damage/body sandbox | Actor feel sandbox, replay/event capture | Wounds, knockdown, equipment damage, and gibbing are readable. | Medium | Medium | Players cannot tell whether a unit is hurt, stunned, dead, or recoverable. |
| AI breach scenario | Terrain damage sandbox, replay/event capture | A small squad can assault or defend through mutable terrain. | Large | Critical | Bots loop, idle, self-kill, or fail silently without useful debug labels. |
| Loadout/delivery loop | Actor feel sandbox, AI breach scenario | Player can buy a squad, choose delivery, and understand risk. | Medium | Medium | Loadout choices feel like spreadsheet work instead of tactical planning. |
| Mod validation slice | Damage/body sandbox, loadout/delivery loop | A tiny mod can be authored, validated, loaded, and tested. | Large | Medium | Schema/editor path cannot represent core item and actor relationships. |
| Terrain authority slice | Terrain damage sandbox, replay/event capture | Server or host applies semantic terrain events with snapshots as fallback. | Large | Critical | Bandwidth or reconciliation is impossible at target combat density. |

## Evidence Standards

Exploratory ideas, moonshots, copied experiments, and private prototypes may be captured at any time if they are labeled clearly. Do not promote an idea into the spec as a settled commitment unless it has at least one of:

- Local Cortex code evidence.
- Direct comparable repo code evidence.
- Public developer/source citation.
- Prototype result.
- Explicit design assumption marked as unproven.

Every spec claim should link back to the vault note, code path, source URL, or prototype artifact that supports it.

## Risk Register

| Risk | Why It Matters | Mitigation |
|---|---|---|
| AI is not trustworthy | Solo-first game fails if bots need babysitting. | Build AI tests before content scale. |
| Simulation becomes unreadable | Players blame randomness instead of learning. | Overlays, event attribution, material lab. |
| Terrain sync blocks multiplayer | Destruction plus physics is expensive to network. | Prototype networking freely; avoid promising launch PvP until authority/bandwidth tests pass. |
| Replay/event model is added too late | AI debugging, player learning, networking, and support all need event history. | Treat replay/events as core infrastructure, not polish. |
| Scope expands into Noita-scale materials | Material depth can overwhelm production. | DR-036 direction is closed: curated 17-material launch set + active-region budgets + per-milestone perf gates (M5.6/M5.7/M7.5/M8.5); expansion materials gated behind material lab; reject "hundreds of launch materials" pattern. |
| Body physics becomes spectacle without readability | Gibs are memorable but can hide useful state. | Pair wounds/gibs with readable states, alerts, and replay explanations. |
| Modding is too fragile | Community longevity suffers. | Schema validation, editor tooling, migration support. |
| UI slows the game | Strategy depth becomes menu friction. | Saved loadouts, quick orders, direct-control priority. |
| Progression becomes grind | Retention systems can damage sandbox trust. | Favor contracts, veterans, salvage, challenge seeds. |
| Gacha corrupts design | Pay randomness can conflict with modding/fairness if it drives core design too early. | Research retention and collection freely; do not make monetization commitments until core loop and ethics are settled. |
| License/IP assumptions are wrong | Reuse may require cleanup before a public release. | Track reuse in the usage ledger; treat license work as release preparation, not a private-prototype blocker. |
| Engine modernization dominates schedule | Forking old code can consume the project before gameplay improves. | Build/run audit before engine choice; prototype risky systems independently. |

## Definition Of Ready For Spec Section

The vault is ready to finalize the first authoritative spec section only when the gates below are satisfied or explicitly deferred with a reason. Exploratory spec stubs, idea notes, and prototypes can continue before these gates close. The rest of the vault remains the ongoing knowledge base after the spec exists.

### Already Done

| Gate | Evidence |
|---|---|
| Requested Cortex repos cloned and documented | `cortext_command_vault/repos/index.md` |
| CCCP vs C4 comparison exists | `cortext_command_vault/comparisons/cccp-vs-c4.md` |
| Online comparable research pass exists | `cortext_command_vault/comparables/research-pass-2-open-source-systems.md` |
| Vault navigation/dashboards | `cortext_command_vault/index.md`, `cortext_command_vault/dashboards/` |
| Body damage / wound / gib lifecycle | `cortext_command_vault/engine/body-damage-wound-gib-lifecycle.md` |
| Activity/scenario lifecycle | `cortext_command_vault/engine/activity-scenario-lifecycle.md` |
| Replay/event architecture brief | `cortext_command_vault/systems/replay-event-architecture.md` |
| Replay recorder Slice A requirements | `cortext_command_vault/spec/replay-recorder-slice-a.md` |
| AI trust harness Slice A requirements | `cortext_command_vault/spec/ai-trust-harness-slice-a.md` |
| Equipment/loadout model requirements | `cortext_command_vault/spec/equipment-loadout.md` |
| Equipment role-card renderer Slice A | `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md` |
| Equipment loadout workbench Slice A | `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md` |
| Equipment CCCP field map | `cortext_command_vault/references/equipment-cccp-field-map.md` |
| Equipment device/loadout field atlas | `cortext_command_vault/references/equipment-device-loadout-field-atlas.md` |
| Equipment source-anchored device snapshots | `cortext_command_vault/references/equipment-source-anchored-device-snapshots.md` |
| Equipment comparable design patterns | `cortext_command_vault/references/equipment-comparable-design-patterns.md` |
| Equipment capability authoring matrix | `cortext_command_vault/references/equipment-capability-authoring-matrix.md` |
| Equipment consumer traceability matrix | `cortext_command_vault/references/equipment-consumer-traceability-matrix.md` |
| Equipment generated consumer traceability report | `cortext_command_vault/references/equipment-consumer-traceability-slice-a.md` |
| Equipment generated trace-tab view | `cortext_command_vault/references/equipment-trace-tab-view-slice-a.md` |
| Equipment generated source trace | `cortext_command_vault/references/equipment-source-trace-slice-a.md` |
| Equipment generated role records | `cortext_command_vault/references/equipment-role-records-slice-a.md` |
| Equipment generated AI summary seed | `cortext_command_vault/references/equipment-ai-summary-seed-slice-a.md`; `cortext_command_vault/references/equipment-ai-summary-seed.slice-a.json` |
| CCCP equipment corpus first pass | `cortext_command_vault/references/equipment-corpus-cccp.md` |
| Equipment schema/overlay seed | `cortext_command_vault/references/equipment-schema-and-overlays.md` |
| Equipment overlay review matrix | `cortext_command_vault/references/equipment-overlay-review-matrix.md` |
| Equipment manual overlay patches | `cortext_command_vault/references/equipment-manual-overlay-patches.md` |
| Equipment merged overlay preview | `cortext_command_vault/references/equipment-overlay-merged-preview.md` |
| Equipment provenance workbench view | `cortext_command_vault/references/equipment-provenance-workbench-view.md` |
| Equipment loadout fixtures | `cortext_command_vault/references/equipment-loadout-fixtures-slice-a.md` |
| Equipment AI scenario seeds | `cortext_command_vault/references/equipment-ai-scenarios-slice-a.md` |
| Equipment package diagnostic expected output | `cortext_command_vault/references/equipment-package-diagnostics-slice-a.md` |
| Destruction-objective brief | `cortext_command_vault/systems/destruction-objective-mission-patterns.md` |
| UX overlay/screen brief | `cortext_command_vault/systems/ux-overlay-screen-brief.md` |
| UX wireframes Slice A requirements | `cortext_command_vault/spec/ux-wireframes-slice-a.md` |
| Modding workbench brief | `cortext_command_vault/systems/modding-package-and-workbench.md` |
| Content module loading lifecycle | `cortext_command_vault/engine/content-module-loading-lifecycle.md` |
| Modding model requirements | `cortext_command_vault/spec/modding-model.md` |
| Package builder/workbench Slice A requirements | `cortext_command_vault/spec/package-builder-workbench-slice-a.md` |
| Decision records DR-001..DR-036 | `cortext_command_vault/decisions/` |
| Comparables workspace skeleton | `comparables_repos/README.md` |
| Usage ledger ready | `cortext_command_vault/references/usage-ledger.md` |

### Remaining Gates

| Gate | Required Before Spec? | Evidence Needed |
|---|---|---|
| Local comparable code audits | Yes | OpenSoldat core/satellites, Powder Toy, and OpenLieroX first passes are complete; use deeper targeted audits when a spec claim needs more evidence. |
| AI trust runnable harness | Yes | A working scenario runner that exercises AI-01..AI-12 and produces replays. |
| Replay/event recorder + viewer prototype | Yes | A recorder + viewer that handles a 5-minute battle and reproduces death/breach causes. |
| First playable prototype (DR-004 slice A) | Yes | Single-actor + small destructible scene playable for 5 minutes. |
| Body damage HUD acceptance tests | Yes | HUD-01..HUD-03 from `cortext_command_vault/systems/ux-overlay-screen-brief.md` and UX-W-01..UX-W-16 from `cortext_command_vault/spec/ux-wireframes-slice-a.md` pass. |
| Engine strategy resolved (DR-001) | Yes | DR-001 closes with a chosen path and revisit trigger; [[engine/cccp-build-run-audit]] now proves native configure/compile plus bounded startup, but it is not the final interactive mission proof. |
| Multiplayer posture frozen for launch (DR-005) | Yes | DR-005 closes with launch posture. |
| Material/damage/equipment taxonomies | Yes | Equipment/loadout taxonomy, field map, device/loadout field atlas, source-anchored snapshots, capability authoring matrix, AI summary seed, consumer traceability matrix/report, trace-tab view, generated source trace, generated role-record projection, metadata contract, role-card renderer, interactive loadout workbench requirements, generated role cards, renderer view, generated overlap audit, overlap worksheet, first generated corpus, schema/overlay seed, review matrix, manual patch layer, merged preview, provenance view, fixture data, AI scenario seeds, and package expected diagnostics now exist in `cortext_command_vault/spec/equipment-loadout.md`, `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md`, `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md`, and `cortext_command_vault/references/`; material/damage need prototype feedback and HUD tests. |
| Prototype effort/risk estimates | Yes | Milestone table with effort, risk, dependency order, and kill criteria. |

> [!info] License/reuse no longer a gate
> Per AGENTS.md and DR-010, license/reuse posture is **documentation, not a gate**. Track reuse in the usage ledger; revisit only when public release is imminent.

### Must Not Finalize Settled Spec Commitments Until

- No P0 row in [Execution Board](#execution-board) is `MISSING` (yellow/orange/draft is fine).
- The AI trust suite has either a runnable harness or a concrete implementation plan with replay/debug data fields.
- OpenSoldat core/satellites, Powder Toy, and OpenLieroX are locally audited; deeper follow-up is targeted to specific spec questions.
- DR-001 (engine), DR-002 (replay), DR-008 (AI architecture) have either a chosen direction or a documented prototype-pending status.
- Replay/event capture has a working recorder.

## Open Questions Triage

| Must Clarify Before Assigning Broad AI Worker Milestones | Why |
|---|---|
| Exact M0 repository bootstrap command and toolchain pin | Needed so every worker creates the same Rust/Bevy workspace and CI surface. Roadmap has the recipe; the first implementation worker should execute and lock it. |
| Networking transport selection timing | DR-005/DR-034 close the authority model, but Lightyear vs renet vs quinn remains a M9/M10 evidence decision. Do not let M0-M7 workers choose it ad hoc. |
| Modding script host | DR-006 leans schema-first + Lua escape hatch; mlua vs Rhai should be chosen when M5/M8 needs real scripts. |
| Localization scope | English-first is accepted, but string extraction/font/language-pack policy needs a small DR before UI text explodes. |
| Cloud-save backend | DR-029 and DR-013 make local-first authoritative; provider choice stays post-launch/adapter until a cloud-sync worker is assigned. |

| Can Defer (as a launch commitment; research/prototype freely) | Why |
|---|---|
| Ranked PvP / first-party tournament service | Public PvP arena readiness is now M12; ranked ladders and tournament-grade operations are optional later layers. |
| Gacha/monetization launch commitments | Should not shape core design before fairness and modding boundaries are proven; retention/collection research is welcome (moonshot MS-12). |
| Full Noita-like everywhere-always material sim as launch baseline | DR-036 closes the systemic direction with bounded active-region kernels + curated 17-material launch set + material lab gate for expansion materials; reject "every-pixel-everywhere-always" patterns. |
| Public mod marketplace launch commitment | Creator tooling and validation must work first; private/personal mod sharing is unblocked. |
| Large campaign economy as launch commitment | Core combat, AI, delivery, and replay loops need proof first; scenario/economy prototyping is welcome. |

## Suggested Spec Section Structure

1. Product promise.
2. Target player and play modes.
3. Core gameplay loop.
4. Simulation architecture.
5. Replay/event architecture.
6. Actor/body/damage model.
7. Terrain/material/destruction model.
8. AI and command model.
9. Equipment/loadout/economy model.
10. Mission/objective/campaign model.
11. UX/UI model, including [[spec/ux-wireframes-slice-a]].
12. Modding and creator tools, including [[spec/package-builder-workbench-slice-a]].
13. Backend/networking posture and [[spec/backend-service-hub-slice-a]].
14. Retention/progression model.
15. Native prototype roadmap, implementation backlog, feature checklist, full collision plan, and control-observability layer, including [[spec/prototype-roadmap]], [[spec/native-implementation-backlog]], [[spec/feature-completion-checklist]], [[spec/full-collision-physics-plan]], and [[spec/ai-control-observability-layer]]; keep [[spec/prototype-implementation-backlog-slice-a]] as historical HTML-lab context only.
16. Risks, deferred features, and kill criteria.
