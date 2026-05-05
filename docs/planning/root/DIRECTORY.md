# Cortex Command Repository Directory

This root directory is a local research host for the Cortex Command repositories and comparable open-source games. The Obsidian-style research vault is in `cortext_command_vault/`; start at `cortext_command_vault/index.md`.

The vault includes dedicated systems research for physics, destruction, damage, equipment, chassis/armor/mechs/origins, AI, networking/backend/frontend, UX/UI, accessibility/comfort/readability, retention, modding workbench, replay/event architecture, destructible-objective mission patterns, body damage / wound / gib lifecycle, activity/scenario lifecycle, systemic material simulation (chunked CA + reactions + atmospheres + affordance/affliction layer), and comparable games (Soldat/OpenSoldat, Liero/OpenLiero/OpenLieroX, Noita, The Powder Toy, Barotrauma, Oxygen Not Included, Stationeers, Teardown, Rain World). It also includes decision records through DR-036 and a usage ledger for tracking external reuse during prototyping.

> [!info] Posture
> This is a personal project. Research, prototyping, and reuse are encouraged. License/reuse tracking lives in `cortext_command_vault/references/usage-ledger.md` and `cortext_command_vault/decisions/dr-010-license-reuse-matrix.md`; it is documentation, not a gate. See `AGENTS.md`.

## Repository Inventory

| Directory | Upstream | Priority | Role | What is special |
| --- | --- | --- | --- | --- |
| `Cortex-Command-Community-Project` | `https://github.com/cortex-command-community/Cortex-Command-Community-Project.git` | Essential | Active CCCP unified game repo | Current community continuation. This is the main research target because it merges source and data, carries the active engine, active `.rte` content modules, current build system, Lua scripts, activities, factions, renderer work, SDL3 migration, and modernized dependency stack. |
| `Cortex-Command-Community-Project-Source` | `https://github.com/cortex-command-community/Cortex-Command-Community-Project-Source.git` | Optional, history only | Archived old source repo | Important for pre-2024 source history and archaeology. Its README says it was archived when the source and data repos were merged into the unified CCCP repo on January 5, 2024. |
| `Cortex-Command-Community-Project-Data` | `https://github.com/cortex-command-community/Cortex-Command-Community-Project-Data.git` | Optional, history only | Archived old data repo | Important for pre-2024 content history, old `.rte` module evolution, faction/assets history, and migration comparisons. Also archived after the 2024 unified repo merge. |
| `Cortex-Command-Community-Project-VSCode-Extension` | `https://github.com/cortex-command-community/Cortex-Command-Community-Project-VSCode-Extension.git` | Nice-to-have | Modding language tooling | A practical map of the CC INI dialect. It contains grammar, snippets, filepath validation, language server pieces, and editor affordances for `.ini`/`.rte` mod authoring. Useful if we build our own modding UX. |
| `Cortex-Command-Legacy-Mod-Converter` | `https://github.com/cortex-command-community/Cortex-Command-Legacy-Mod-Converter.git` | Nice-to-have | Legacy mod migration tool | A GUI wrapper around rule-driven conversion. Useful for understanding what changed between classic CC mod formats and current CCCP expectations, and for designing import/migration tooling in a fork. |
| `cortex-command-community.github.io` | `https://github.com/cortex-command-community/cortex-command-community.github.io.git` | Low | Website source | The public face of CCCP. Useful for messaging, downloads, contribution flow, and how the community explains the project to players and contributors. |
| `Cortex-Command-Community-Continuation-Engine` | `https://github.com/Cortex-Command-Center/Cortex-Command-Community-Continuation-Engine.git` | Comparison | C4 alternative fork engine | Alternative continuation from the Cortex Command Center group. It is useful as a comparison fork, especially around older source layout, Allegro-era dependencies, and its multiplayer/networking emphasis. |

## Sibling Workspaces

| Path | Role |
|---|---|
| `cortext_command_vault/` | Obsidian research vault (knowledge base + decisions + spec section). |
| `comparables_repos/` | Workspace for cloning comparable open-source games (OpenSoldat, OpenLiero, OpenLieroX, Powder Toy). OpenSoldat core, The Powder Toy, and OpenLieroX are cloned and audited first-pass; see `comparables_repos/README.md`. |
| `prototype_workspaces/actor_feel_lab/` | Separate prototype workspace for A0/A1 actor-feel, LOAD-A equipment runtime, and LOAD-W workbench smoke work. |
| `prototype_runs/` | Checked prototype run bundles with manifest/events/summary/notes evidence. |

## Fast Read

Start with:

- `cortext_command_vault/index.md` — vault map.
- `cortext_command_vault/spec/authoritative-game-spec-v0.md` — current canonical v0 game spec: product promise, player fantasy, core loop, first playable scope, system contracts, launch commitments, prototype tracks, moonshots, and open research.
- `cortext_command_vault/research-log/2026-05-04-goal-completion-audit-snapshot.md` — captured snapshot of the Codex agent's `/goal` completion audit (kept for audit trail; not a current project status).
- `cortext_command_vault/dashboards/index.md` — dashboard hub.
- `cortext_command_vault/dashboards/navigation-map.md` — find a note by question.
- `cortext_command_vault/dashboards/system-heatmap.md` — risk/value/readiness flags.
- `cortext_command_vault/dashboards/research-readiness.md` — spec readiness gates.
- `cortext_command_vault/decisions/index.md` — DR-001..DR-036 plus open topics.
- `cortext_command_vault/decisions/dr-036-systemic-material-simulation-direction.md` — closed-direction systemic material simulation (DR-036): bounded active-region per-pixel CA kernel (`cx-material`) + Barotrauma-style hull/atmosphere networks (`cx-atmos`) + reaction table + affordance/affliction layer; curated 17-material launch set; T-MAT side track + M5.6/M5.7/M6.6/M7.5/M8.5 + 4 new run-bundle event categories (`material`/`reaction`/`atmosphere`/`affliction`).
- `cortext_command_vault/comparables/noita-grade-material-simulation-research.md` — Noita + Powder Toy + Barotrauma + ONI + Stationeers research synthesis (50 sources) feeding DR-036.
- `cortext_command_vault/decisions/dr-014-tone-player-promise.md` — tactical pulp sci-fi disaster sandbox tone/player-promise decision.
- `cortext_command_vault/decisions/dr-015-player-identity-control-posture.md` — command-core operator identity; strategy-first play is valid and direct possession is optional intervention.
- `cortext_command_vault/spec/command-core-base-power.md` — rooted command core base power, base shields/turrets/sensors/doors/repair platforms, uproot tradeoff, embedded avatar boosts, and CORE-A tests.
- `cortext_command_vault/spec/index.md` — spec section index and supporting subsystem pages.
- `cortext_command_vault/spec/chassis-armor-mechs-and-origins.md` — armor layers, powered armor, enterable mechs, android/robot/origin choices, staged equipment damage, AI labels, replay events, and CHASSIS-A tests.
- `cortext_command_vault/spec/prototype-roadmap.md` — native Rust + Bevy/wgpu M0..M12 roadmap with agent implementation contract, T-CONTROL AI/dev observability, M1.5 Micro Breach fun proof, validation command matrix, bug-hunt checklist, and definition of done.
- `cortext_command_vault/spec/native-implementation-backlog.md` — primary AI-agent handoff backlog: M0..M12 task cards with crate ownership, implementation scope, tests, E2E commands, `cxctl`/control-observation obligations, run-bundle evidence, and anti-scope.
- `cortext_command_vault/spec/feature-completion-checklist.md` — live completion/rating checklist for roadmap features, milestone scope/done-criteria, side tracks, task cards, validation checks, bug hunts, and definition-of-done rows.
- `cortext_command_vault/spec/ai-control-observability-layer.md` — built-in eyes/ears/hands layer for Codex, automated tests, accessibility tooling, and future bot authors: `cx-control`, `cxctl`, structured observations, collision observations, semantic actions, UI tree, local control server, and capability gates.
- `cortext_command_vault/spec/hybrid-llm-ai-plan.md` — async LLM mind-layer roadmap extension: local reflex/tactical AI keeps acting while optional OpenAI/Anthropic/local LLM workers propose doctrine, memory, personality, commander adaptation, debriefs, and profile changes through strict schemas, validation, replay events, and deterministic fallback.
- `cortext_command_vault/spec/full-collision-physics-plan.md` — full physical collision roadmap extension: collision classes/matrix/proxies, projectile-projectile rules, CCD tiers, impulse-to-damage routing, `collision` events, T-PHYS, and M5.5 COLL-001..COLL-012.
- `cortext_command_vault/spec/server-app-architecture.md` — `cx-server` dedicated server binary architecture (DR-034): single binary with `coop_room`/`pvp_arena`/`lan_room`/`mmo_shard`/`lobby_directory` modes; same Rust workspace as the client; community-hostable; reference Docker image; SERVER-001..SERVER-016 acceptance suite split across M9-M12.
- `cortext_command_vault/spec/persistent-mmo-architecture.md` — persistent MMO shard architecture (DR-035): bounded shard-with-portal model; 50-200 concurrent target; community-hostable; persistent terrain/bases/veterans/factions/commander memory; account required for public shards; not subscription-funded; MMO-001..MMO-012 acceptance suite.
- `cortext_command_vault/spec/prototype-implementation-backlog-slice-a.md` — historical browser/HTML A0..A7 implementation task cards; use only for old actor-feel-lab context unless explicitly assigned.
- `cortext_command_vault/references/prototype-run-bundle-schema.md` — manifest/events/summary schemas, notes headings, cross-file consistency rules, and `prototype_run_check.py` command for validating future prototype run evidence.
- `cortext_command_vault/prototypes/index.md` — prototype evidence index.
- `cortext_command_vault/prototypes/actor-feel-lab-a0-bootstrap.md` — A0 bootstrap run evidence for the separate actor-feel workspace and run-bundle checker path.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-runtime-smoke.md` — A1 browser/canvas runtime and checked movement/aim/rifle/reload/status/snapshot event bundle.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-ui-smoke.md` — A1 deterministic browser capture, selected `engineer_breach` LOAD-A fixture state, placeholder tool/refusal events, and checked screenshot run bundle.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-workbench-smoke.md` — A1 LOAD-W workbench smoke: generated coverage counts, fixture tabs, selected Light Digger trace/source/AI/diagnostic panel, source inspector, screenshot, and checked run bundle.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-switch-smoke.md` — A1 LOAD-W fixture-switch smoke: all nine LOAD-A fixture imports, `engineer_breach` to `medic_rescue` runtime switch, Medikit trace/source/AI joins, screenshot, and checked run bundle.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-tab-input-smoke.md` — A1 LOAD-W fixture-tab input smoke: Chrome DevTools click and focused-button keyboard activation switch fixtures, preserve trace/source/AI joins, and produce a checked 55-event run bundle.
- `cortext_command_vault/prototypes/actor-feel-lab-a1-load-w-fixture-traversal-smoke.md` — A1 LOAD-W fixture traversal smoke: Tab reaches fixture controls, ArrowRight moves focus, Enter/Space activate fixtures, focus survives render restore, trace/source/AI joins persist, and a checked 139-event run bundle proves fixture-control traversal.
- `cortext_command_vault/research-log/2026-05-04-a0-prototype-bootstrap.md` — dated log for the A0 prototype bootstrap pass.
- `cortext_command_vault/research-log/2026-05-04-a1-actor-feel-runtime-smoke.md` — dated log for the A1 runtime-smoke pass.
- `cortext_command_vault/research-log/2026-05-04-a1-ui-smoke-capture.md` — dated log for the A1 UI-smoke screenshot/capture pass.
- `cortext_command_vault/research-log/2026-05-04-a1-load-w-workbench-smoke.md` — dated log for the LOAD-W browser workbench smoke pass.
- `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-switch-smoke.md` — dated log for the LOAD-W fixture-switch smoke pass.
- `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-tab-input-smoke.md` — dated log for the LOAD-W fixture-tab input smoke pass.
- `cortext_command_vault/research-log/2026-05-04-a1-load-w-fixture-traversal-smoke.md` — dated log for the LOAD-W fixture-control traversal smoke pass.
- `cortext_command_vault/spec/progression-retention.md` and `cortext_command_vault/decisions/dr-011-progression-retention-loop.md` — intrinsic-first progression/retention model, RET-A tests, UI/telemetry requirements, and monetization guardrails.
- `cortext_command_vault/spec/actor-feel-sandbox-slice-a.md` — buildable requirements for the first actor-feel sandbox.
- `cortext_command_vault/engine/cccp-build-run-audit.md` — DR-001 build/run audit: native macOS dependency/install result, Meson/GCC 13 configure pass, compile/link pass, bounded startup result, and remaining interactive mission proof.
- `cortext_command_vault/engine/cccp-runtime-window-capture-troubleshooting.md` — DR-001 runtime proof follow-up: WindowServer sees a Cortex window, but screenshot capture still does not show game pixels.
- `cortext_command_vault/repos/index.md` — Cortex repo atlas.
- `cortext_command_vault/comparisons/cccp-vs-c4.md` — fork strategy implications.
- `cortext_command_vault/systems/mechanics-matrix.md` — all-inclusive comparison tables.
- `cortext_command_vault/systems/physics-and-destruction-models.md` — terrain/materials/body/damage models.
- `cortext_command_vault/spec/body-damage-model.md` — build-facing body damage requirements: body parts, wounds, status, gibs, equipment fallout, treatment, AI reason labels, replay/death recap events, HUD rules, and BODY-A acceptance tests.
- `cortext_command_vault/research-log/2026-05-04-tone-chassis-mech-promise.md` — dated pass adding tone/player promise, mechs, armor, origins, and staged equipment/module damage.
- `cortext_command_vault/systems/material-and-mobility-affordance-schema.md` — material/tool/mobility/hazard/replay schema proposal.
- `cortext_command_vault/spec/terrain-material-sandbox-slice-a.md` — buildable terrain/material lab requirements, overlay/path/replay/AI tests, and MAT-T acceptance suite.
- `cortext_command_vault/systems/ai-and-bots.md` and `cortext_command_vault/systems/ai-trust-test-suite.md` — AI direction + measurable trust.
- `cortext_command_vault/spec/ai-trust-harness-slice-a.md` — buildable AI scenario harness, event contract, local AI hook map, reports, and bootstrap tests.
- `cortext_command_vault/systems/networking-backend-frontend.md`, `cortext_command_vault/spec/backend-service-hub-slice-a.md`, `cortext_command_vault/decisions/dr-013-backend-service-scope.md`, `cortext_command_vault/systems/replay-event-architecture.md`, and `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` — online posture, backend/hub Slice A, local-first backend service scope, recorder design, hybrid replay/determinism posture, checksums, snapshots, and run-evidence tests.
- `cortext_command_vault/spec/replay-recorder-slice-a.md` — buildable recorder/viewer requirements and local CCCP hook map.
- `cortext_command_vault/systems/destruction-objective-mission-patterns.md` — destruction-aware missions.
- `cortext_command_vault/spec/mission-director-slice-a.md` — typed mission manifest, director pacing, commander AI, destruction-aware objective grammar, equipment capability contract, save/replay events, UI/workbench obligations, and MISSION-A acceptance tests.
- `cortext_command_vault/spec/missions-and-objectives.md` — high-level missions/objectives spec index and Breach Contract proof-mission target.
- `cortext_command_vault/systems/ux-overlay-screen-brief.md` — screen inventory + acceptance tests.
- `cortext_command_vault/spec/ux-wireframes-slice-a.md` — build-facing HUD, squad, command, buy/loadout, material overlay, replay/death recap, hub, workbench, accessibility, UX telemetry, and UX-W acceptance-test requirements.
- `cortext_command_vault/spec/accessibility-comfort-slice-a.md` — text scale/reflow, contrast, no-color-only states, same-input navigation, remap/holds, captions, reduced motion/shake/flash, equipment workbench ACC-A tests, and run-bundle evidence additions.
- `cortext_command_vault/spec/equipment-loadout.md` — actor role taxonomy, item archetypes, explicit loadout slots, AI item metadata, buy/loadout UX, delivery risk, mod validation, replay hooks, and LOAD-A acceptance tests.
- `cortext_command_vault/spec/equipment-role-card-renderer-slice-a.md` — LOAD-009/LOAD-010 role-card renderer, catalog visibility, detail drawers, actor slot cards, squad summaries, workbench drill-downs, AI/replay labels, overlap-resolution rules, and LOAD-R tests.
- `cortext_command_vault/spec/equipment-loadout-workbench-slice-a.md` — interactive equipment/loadout/workbench prototype requirements, fixture routes, state machine, LOAD-W tests, and implementation tickets.
- `cortext_command_vault/references/equipment-cccp-field-map.md` — field-level bridge from CCCP C++/Lua/INI equipment/loadout mechanics into future AI, UI, modding, balance, replay, backend, and package-builder consumers.
- `cortext_command_vault/references/equipment-device-loadout-field-atlas.md` — consumer-ready atlas mapping exact held-device/firearm/magazine/round/loadout/delivery fields into AI reason labels, UI/workbench trace rows, balance rows, replay events, backend compatibility, and mod schema rules.
- `cortext_command_vault/references/equipment-source-anchored-device-snapshots.md` — literal CCCP field values for representative devices/loadouts translated into source-inspector rows, AI reason labels, material-tool events, replay/backend obligations, and balance comparisons.
- `cortext_command_vault/references/equipment-comparable-design-patterns.md` — CCCP/OpenSoldat/OpenRA/Unreal/Godot/Unity/Factorio equipment patterns translated into item-definition/runtime/effect/AI/UI/modding/replay requirements and LOAD-COMP tests.
- `cortext_command_vault/references/equipment-role-design-deep-dive.md` — concrete CCCP device role cards, durable weapon/loadout/tag/data design references, role-card requirements, pros/cons matrices, and LOAD-009/LOAD-010 follow-ups.
- `cortext_command_vault/references/equipment-capability-authoring-matrix.md` — cross-source capability/authoring matrix for AI, UI, modding/workbench, balancing, replay, schema, authoring-tier, and bot-default item-role decisions.
- `cortext_command_vault/references/equipment-ai-behavior-contract.md` — bot item-choice/refusal contract grounded in CCCP AI code, with reason labels, event families, scenario mapping, package/UI/replay obligations, and AI-EQ tickets.
- `cortext_command_vault/references/equipment-ai-summary-seed-slice-a.md` — generated bot-facing item-use seed for all 106 role-card rows: claim state, blackboard keys, required reason labels/events, source confidence, scenario refs, and first fix actions.
- `cortext_command_vault/references/equipment-consumer-traceability-matrix.md` — field-to-consumer matrix for AI, UI, workbench/package diagnostics, balancing, replay/debug, backend/session compatibility, and prototype-test obligations.
- `cortext_command_vault/references/equipment-consumer-traceability-slice-a.md` — generated LOAD-011 report tracing 106 role-card rows into UI, AI, workbench/package, balance, replay/backend, fixtures, and prioritized consumer gaps.
- `cortext_command_vault/references/equipment-trace-tab-view-slice-a.md` — generated LOAD-W-010 trace-tab view model with 106 item trace rows, 9 fixture tabs, 39 diagnostic trace rows, 80 gap badges, and open targets for workbench UI.
- `cortext_command_vault/references/equipment-source-trace-slice-a.md` — generated LOAD-FIELD-SOURCE source-inspector fixture joining 106 role cards to loader graph context, source confidence, field provenance, duplicate source pressure, trace-tab refs, and open targets.
- `cortext_command_vault/references/equipment-role-records-slice-a.md` — generated LOAD-W role-record projection joining role cards, source trace, AI summaries, diagnostics, overlap, fixture refs, replay/backend fields, and mission capability tags into one item object for AI/UI/modding/balance/replay consumers.
- `cortext_command_vault/references/equipment-role-cards-slice-a.md` — generated LOAD-009 role cards for 106 catalog/internal/non-catalog item records, with AI/UI/modding/balance/replay consumer fields.
- `cortext_command_vault/references/equipment-role-card-renderer-view-slice-a.md` — generated LOAD-R renderer view with 63 catalog rows, 106 workbench rows, 5 detail drawers, 10 overlap rows, 9 fixture summaries, and acceptance coverage.
- `cortext_command_vault/references/equipment-overlap-audit-slice-a.md` — generated LOAD-010 overlap audit: 10 role-signature groups covering 42 player-catalog items, including 3 high-risk duplicate-role groups.
- `cortext_command_vault/references/equipment-overlap-resolution-worksheet-slice-a.md` — LOAD-010 worksheet for turning duplicate-role groups into role splits, skin/legacy candidates, mission fixtures, and AI/UI/modding/balance/replay implications.
- `cortext_command_vault/references/equipment-corpus-cccp.md` — generated CCCP device/loadout corpus: counts, groups, archetypes, metadata coverage, gap sample, loadout sample, and extractor caveats.
- `cortext_command_vault/research-log/2026-05-04-equipment-copyof-resolution.md` — common device/magazine/round `CopyOf` cleanup and current equipment overlay counts.
- `cortext_command_vault/research-log/2026-05-04-equipment-provenance-warning-metadata.md` — field provenance and structured warning details for generated and patched equipment data.
- `cortext_command_vault/references/equipment-schema-and-overlays.md` — JSON Schema, corpus JSON, Base/Coalition/Ronin overlay seed, validation commands, and LOAD-002/LOAD-003 follow-ups.
- `cortext_command_vault/references/equipment-overlay-review-matrix.md` — warning severity, critical item triage, bot competence gates, LOAD-A fixture plan, package diagnostics, and AI/UX implications.
- `cortext_command_vault/references/equipment-manual-overlay-patches.md` — non-destructive manual patch layer for replacement/catalog policy, internal components/payloads, scripted tool contexts, and launcher bot-label gating.
- `cortext_command_vault/references/equipment-overlay-merged-preview.md` — patch-applied equipment preview and fixture diagnostic reports for UI, AI harness, package-builder, and prototype-loader work.
- `cortext_command_vault/references/equipment-provenance-workbench-view.md` — LOAD-A fixture provenance rows, attention fields, warning fix queue, and source highlights for workbench/UX/AI consumers.
- `cortext_command_vault/research-log/2026-05-04-equipment-provenance-workbench-view.md` — dated log for the provenance view and its AI/UI/modding/balancing/replay consumer implications.
- `cortext_command_vault/references/equipment-loadout-fixtures-slice-a.md` — nine LOAD-A fixture loadouts with actual item IDs, expected warnings, manual badges, and test coverage.
- `cortext_command_vault/references/equipment-ai-scenarios-slice-a.md` — nine AI-H-LOAD equipment scenarios and schema generated from fixtures for bot trust tests.
- `cortext_command_vault/references/equipment-package-diagnostics-slice-a.md` — PACK-014 expected package-builder diagnostics, consumer-impact labels, mode verdicts, and bot assignment verdicts for the equipment fixtures.
- `cortext_command_vault/research-log/2026-05-04-equipment-role-card-renderer-slice-a.md` — dated log for turning generated role-card and overlap artifacts into a product/workbench renderer contract.
- `cortext_command_vault/research-log/2026-05-04-equipment-renderer-view-model.md` — dated log for the generated LOAD-R renderer view model and checker coverage.
- `cortext_command_vault/research-log/2026-05-04-equipment-overlap-resolution-worksheet.md` — dated log for the overlap worksheet and its high/medium-risk resolution queue.
- `cortext_command_vault/research-log/2026-05-04-equipment-capability-authoring-matrix.md` — dated log for the equipment capability/authoring matrix research pass.
- `cortext_command_vault/research-log/2026-05-04-equipment-ai-behavior-contract.md` — dated log for turning actual CCCP item-choice, projectile-summary, breaching, pickup, support, and explosive-safety behavior into a shared AI/UI/workbench/replay contract.
- `cortext_command_vault/research-log/2026-05-04-equipment-ai-summary-seed.md` — dated log for generating the AI summary seed that bridges role cards into bot claim states, blackboard fields, reason labels, event families, and harness/package/replay follow-ups.
- `cortext_command_vault/research-log/2026-05-04-equipment-loadout-workbench-slice-a.md` — dated log for the interactive equipment/loadout/workbench prototype requirements.
- `cortext_command_vault/research-log/2026-05-04-equipment-consumer-traceability.md` — dated log for the cross-consumer equipment traceability pass.
- `cortext_command_vault/research-log/2026-05-04-equipment-trace-tab-view.md` — dated log for the generated LOAD-W-010 trace-tab view model and checker coverage.
- `cortext_command_vault/research-log/2026-05-04-equipment-source-trace.md` — dated log for the source-inspector join across role cards, corpus provenance, loader graph context, and trace-tab rows.
- `cortext_command_vault/research-log/2026-05-04-equipment-role-record-projection.md` — dated log for generating and prototyping the shared role-record projection consumed by the LOAD-W browser seed.
- `cortext_command_vault/research-log/2026-05-04-equipment-device-loadout-field-atlas.md` — dated log for the deeper CCCP device/loadout field atlas and durable schema reference pass.
- `cortext_command_vault/research-log/2026-05-04-equipment-source-anchored-device-snapshots.md` — dated log for the literal CCCP equipment/source-value snapshot pass.
- `cortext_command_vault/research-log/2026-05-04-equipment-comparable-design-patterns.md` — dated log for the OpenSoldat/OpenRA/Unreal/Godot/Unity/Factorio equipment-pattern comparison pass.
- `cortext_command_vault/research-log/2026-05-04-mission-director-slice-a.md` — dated log for the mission/director code and web-source pass.
- `cortext_command_vault/systems/modding-package-and-workbench.md` — package format + workbench scope.
- `cortext_command_vault/engine/content-module-loading-lifecycle.md` — `.rte` discovery/import, official/mod/userdata order, include stack, `CopyOf`, preset source paths, script reload, zip import, CONTENT-A tests, and package/equipment diagnostics.
- `cortext_command_vault/references/content-loader-graph-cccp.md` — generated loader graph and JSON fixture: module order, include edges, script paths, preset/source counts, duplicate preset pressure, and CONTENT-A coverage.
- `cortext_command_vault/research-log/2026-05-04-content-loader-graph-generated.md` — dated log for the generated loader graph pass.
- `cortext_command_vault/spec/modding-model.md` — exploratory package modes, loader parity, provenance, script capability, equipment metadata, migration, and MOD-A tests.
- `cortext_command_vault/spec/package-builder-workbench-slice-a.md` — deterministic package builder, manifest/provenance validation, diagnostics, graphs, migration preview, and test-launch requirements.
- `cortext_command_vault/engine/body-damage-wound-gib-lifecycle.md` — wound/gib/death/inventory chain.
- `cortext_command_vault/research-log/2026-05-04-body-damage-model-spec.md` — dated pass for promoting the body damage model from stub to prototype requirements.
- `cortext_command_vault/engine/direct-control-and-actor-feel-lifecycle.md` — local CCCP input/control/actor/firearm/terrain feel loop.
- `cortext_command_vault/engine/activity-scenario-lifecycle.md` — activity/scenario contract.
- `cortext_command_vault/engine/projectile-to-impact-lifecycle.md` — trigger to impact.
- `cortext_command_vault/engine/terrain-mutation-and-pathfinding-lifecycle.md` — terrain edit + path invalidation.
- `cortext_command_vault/engine/ai-order-lifecycle.md` — engine + Lua AI order flow.
- `cortext_command_vault/engine/loadout-delivery-economy-lifecycle.md` — buy menu + delivery.
- `cortext_command_vault/engine/network-terrain-replication-lifecycle.md` — terrain replication code trace.
- `cortext_command_vault/comparables/comparison-matrix.md` — outside-game research.
- `cortext_command_vault/comparables/opensoldat-local-audit.md` — first local comparable source audit.
- `cortext_command_vault/comparables/opensoldat-satellites-local-audit.md` — OpenSoldat base/launcher/lobby audit for content purity, launcher UX, server discovery, and backend/frontend scope.
- `cortext_command_vault/comparables/the-powder-toy-local-audit.md` — material simulation/tooling comparable source audit.
- `cortext_command_vault/comparables/openlierox-local-audit.md` — destructible arena, rope movement, weapons, bots, modding, and network-caution source audit.
- `cortext_command_vault/references/sources.md` — source URLs + local code paths.
- `cortext_command_vault/references/usage-ledger.md` — tracker for actually-used external code/assets.
- `cortext_command_vault/strategy/research-to-spec-roadmap.md` — next research steps.
- `cortext_command_vault/design/opportunities-for-our-fork.md` — build/fork ideas.
- `cortext_command_vault/research-log/moonshot-register.md` — wild ideas + ambitious bets, never gated by readiness.

## Local Snapshot Notes

These clones are a research snapshot of the upstream state available at clone time. The active CCCP repo is the one to inspect first for current implementation details. The old Source/Data repos should be treated as historical context unless you are deliberately tracing old behavior or asset lineage.
