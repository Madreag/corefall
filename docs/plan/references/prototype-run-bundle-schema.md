---
type: reference
status: prototype-evidence-contract
feeds:
  - DR-002
  - DR-004
  - DR-005
  - DR-007
  - DR-008
  - DR-009
  - DR-012
  - DR-013
  - DR-024
  - DR-033
---

← [[references/sources|sources]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/prototype-implementation-backlog-slice-a|historical Slice-A backlog]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[spec/accessibility-comfort-slice-a|accessibility/comfort Slice A]] · [[decisions/dr-013-backend-service-scope|DR-013 backend scope]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]]

# Prototype Run Bundle Schema

> [!summary] Why this exists
> The prototype roadmap requires every serious prototype/native milestone run to emit a reproducible bundle. This page turns that prose into a concrete contract so implementation agents, planning agents, vault maintainers, replay tooling, AI harnesses, and decision records all read the same evidence.

## Contract Files

| File | Role | Schema / Checker |
|---|---|---|
| `run_manifest.json` | Identifies the build, slice, scene, seed, material schema, config hash, assumptions tested, and expected tests. | `prototype-run-manifest.schema.json` |
| `events.jsonl` | Ordered recorder events and snapshots from the run. | `prototype-recorder-event.schema.json` per line |
| `summary.json` | Results, event counts, byte volume, performance counters, artifacts, blockers, and next actions. | `prototype-run-summary.schema.json` |
| `notes.md` | Human observation layer: assumptions, Good/Bad/Meh moments, evidence links, and next actions. | Heading checks in `prototype_run_check.py` |
| `screenshots/` or `captures/` | HUD, overlay, event tail, replay viewer, workbench, or failure screenshots. | Listed in `summary.json.artifacts` |

Validation command:

```bash
python3 research_tools/prototype_run_check.py <run-dir>
```

The checker intentionally avoids third-party dependencies. JSON Schema files are the canonical field reference; `prototype_run_check.py` enforces the cross-file rules that plain schema validation cannot catch.

## Required `notes.md` Headings

Use these headings in every meaningful run note:

```markdown
## Assumptions Tested
## Good
## Bad
## Meh
## Evidence Links
## Next Actions
```

Good/Bad/Meh should be about observed play and debug evidence, not only personal taste. Evidence links should point to event ids, screenshot paths, config names, issue ids, or follow-up vault pages.

## Cross-File Consistency Rules

| Rule | Why It Matters |
|---|---|
| `summary.json.manifest_run_id` must equal `run_manifest.json.run_id`. | Prevents pasted summaries from being attached to the wrong run. |
| Every event `run_id` must equal the manifest run id. | Keeps replay/debug tooling from mixing traces. |
| Event ids must be unique within a run. | Death recaps, AI failures, and terrain cause chains need stable references. |
| Event ticks must be monotonic. | Recorder output should be inspectable without reconstructing hidden ordering. |
| Event `parent_event_id` should reference an earlier event unless it is explicitly external. | Cause chains are the foundation for replay, AI trust, UX recaps, and networking analysis. |
| Determinism claims must be backed by manifest/summary extensions and event evidence. | [[systems/replay-determinism-and-run-evidence]] requires input traces, checksums, content/config hashes, and snapshot evidence before a run can claim deterministic behavior. |
| `summary.json.event_counts.total` must equal the number of parsed JSONL events. | Prevents stale counters from feeding decision records. |
| `summary.json.event_counts.by_category` and `by_type` must match actual events. | Event volume and category budgets feed DR-002, DR-005, and DR-007. |
| Test evidence ids in `summary.json.tests[*].evidence_event_ids` must exist in `events.jsonl`. | Acceptance tests should cite real captured evidence. |
| `summary.json.event_counts.dropped_total` must be at least the sum of per-event `dropped_count`. | Recorder backpressure must stay visible. |

## Event Category Baseline

| Category | Typical Event Types | Primary Consumers |
|---|---|---|
| `input` | `input_intent`, `tool_selected_for_material` | Actor feel, replay, future net prediction, AI harness. |
| `control` | `control_command_received`, `control_command_accepted`, `control_command_rejected`, `control_observation_sent`, `control_assertion_result` | AI/Codex automation, E2E tests, future bot SDK, replay/debug evidence. |
| `mind` | `mind.task_created`, `mind.prompt_recorded` (hashes by default; raw text only when `manifest.capabilities.debug` is true), `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written` | Async LLM mind layer (DR-032 / [[spec/hybrid-llm-ai-plan]]); audit prompt/response provenance, validator decisions, applied patches, structured memory writes; secrets redacted by default. |
| `collision` | `collision_pair_created`, `collision_contact_started`, `collision_contact_persisted`, `collision_contact_ended`, `contact_impulse_applied`, `projectile_deflected`, `projectile_projectile_contact`, `collision_filter_applied`, `collision_damage_applied`, `collision_budget_degraded`, `collision_first_divergence` | T-PHYS / DR-033 full collision evidence; inspect contact pairs, filter reasons, projectile-projectile results, impulse-to-damage routing, and replay divergence. |
| `combat` | `weapon_fired`, `projectile_spawned`, `projectile_hit_mo`, `weapon_reloaded` | Damage readability, replay, equipment balance. |
| `body` | `wound_added`, `actor_status_changed`, `body_gibbed`, `inventory_dropped` | HUD, death recap, UX trust. |
| `terrain` | `terrain_material_probe`, `terrain_penetration_threshold`, `terrain_carve_mask`, `terrain_fill_or_repair`, `path_material_refresh` | Terrain model, AI path trust, networking bandwidth. |
| `ai` | `ai_intent`, `ai_item_choice`, `ai_item_refusal`, `ai_item_result` | Solo trust, AI harness, equipment workbench diagnostics. |
| `logistics` | loadout, delivery, build, salvage, economy events. | Backend/hub and campaign-loop research. |
| `mission` | objective, squad, commander, fail-state, win-state events. | First playable and future campaign spec. |
| `system` | run lifecycle, recorder health, config changes, dropped-event batches. | Reproducibility and tooling. |
| `snapshot` | `snapshot_actor`, `snapshot_terrain_chunk`, `snapshot_inventory` | Replay anchors and mutable terrain fallback truth. |
| `determinism` | `sim_checksum`, `first_divergence`, `replay_probe_result` | DR-002/DR-005 evidence, future netcode experiments, and deterministic-island promotion. |
| `ux` | HUD visibility, overlay mode, failure label, death recap shown. | UX/UI comprehension tests. |
| `accessibility` | `ux_accessibility_setting_changed`, text scale applied, contrast mode, focus path tested, caption shown, flash suppressed, screen shake scaled. | ACC-A evidence, comfort/readability regression, workbench accessibility, run-bundle audits. |
| `performance` | frame cost, dirty rect cost, event volume, path refresh cost. | DR-002/DR-005/DR-007 risk budgets. |

## Native Milestone Acceptance Gates

| Milestone | Run Bundle Must Prove |
|---|---|
| M0 engine bootstrap | Native app starts/ends cleanly, seed/config/build metadata are captured, fixed-tick smoke evidence exists, `cxctl observe/run` evidence exists, and the bundle validates with `prototype_run_check.py`. |
| M1 actor controller | Input, movement, aim, weapon, reload, status, HUD, semantic control actions, actor/equipment observations, and recorder events are captured from the native controller scene. |
| M1.5 micro breach fun slice | Win/loss state, objective timer, reactive enemy behavior, temporary soft-breach surface edits, control-driven win/loss scripts, observation stream freshness, and HUD objective readability are captured. |
| M2 terrain/materials | Material probe, penetration, carve/fill, dirty-region refresh, path refresh hooks, and performance counters are captured from mutable terrain actions. |
| M3 replay/event recorder | Event cause chains, snapshots, dropped-event counters, deterministic replay checks, and viewer artifacts are present enough to debug a run without watching it live. |
| M4 HUD/comic-noir UI | HUD, overlays, death/material explanations, accessibility settings, caption evidence, and screenshots/captures show the player-facing state clearly. |
| M5 equipment/chassis | Item role labels, damage-stage state, armor/chassis effects, bot-usable fields, loadout validation, repair/salvage, and ejection/disable evidence are captured. |
| M5.5 full collision gauntlet | `collision.*` events captured for collision matrix coverage, limb/body/equipment/mech/base/projectile contacts, projectile-projectile deflection/fuze/detonation cases, CCD/tunneling fixtures, impulse-to-damage routing, collision-filter reasons, `cxctl observe --collisions`, perf counters, and headless replay checksums. |
| M6 AI trust harness | Bot intent, perception facts, doctrine/personality labels, mistakes, recovery actions, blocked-path reasons, and explanation overlays are captured by AI-H scenarios. |
| M6.5 LLM mind lab | `mind.*` events captured for every task: prompt hash (raw text only when `debug` capability is on), response hash, validator result with reasons, applied patch ids, rejected proposals, memory writes; mock-provider runs are deterministic; live provider runs are flagged but never required for CI. |
| M7 mission director | Manifest-driven objectives, director events, command-core/base-power state, debrief/retry state, and scenario completion/failure evidence are captured. |
| M8 editor/mod tools | Edited scenario/package data, validation diagnostics, content hashes, sample mod load evidence, and workbench screenshots are captured. |
| M9+ networking/headless tracks | Headless replay, authority/replication events, config hashes, divergence reports, and bandwidth/performance counters are captured before any network posture can close. |

## Historical Slice-A Acceptance Gates

| Slice | Run Bundle Must Prove |
|---|---|
| A0 lab shell | Manifest exists, run starts/ends cleanly, reset/config/seed data are captured. |
| A1 actor feel | A-FEEL tests cite input, movement, aim, weapon, reload, and status evidence. |
| A2 terrain/material lab | MAT-T tests cite material probe, penetration, carve/fill, dirty region, path refresh, and performance events. |
| A3 replay/debug viewer | REC-A tests cite event cause chains, snapshots, dropped-event counters, and viewer artifacts. |
| A4 UX comprehension | HUD/material/death recap screenshots are listed, and UX events record which explanation surface was shown. |
| A4 accessibility/comfort | ACC-A evidence records text scale, contrast, no-color-only state, same-input navigation, remapping, captions, reduced motion/shake/flash, screenshots, and setting values. |
| A5 equipment/loadout | LOAD-A/LOAD-R evidence includes item role labels, refusal labels, fixture ids, and bot-usable fields. |
| A5 implementation backlog | [[spec/prototype-implementation-backlog-slice-a]] defines the concrete workbench/source-trace/item-role task cards and evidence destinations. |
| A6 AI trust bootstrap | AI-H/AI-EQ evidence includes bot intent, item choice/refusal/result, blocked-path reasons, and explanation labels. |

## Checker Scope

The checker is a gate for evidence hygiene, not a declaration that a prototype is fun or shippable.

| It Checks | It Does Not Check |
|---|---|
| Required files exist. | Final game quality. |
| JSON parses and schema-version constants match. | Full JSON Schema compliance. |
| Run ids, event ids, parent ids, counts, and test evidence references are coherent. | Physics correctness or determinism. |
| Notes include required headings. | Whether tester notes are insightful. |
| Summary counters match the raw events. | Whether the decision record should close. |

## Source Trail

- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[spec/ai-control-observability-layer]]
- [[spec/prototype-implementation-backlog-slice-a]]
- [[spec/replay-recorder-slice-a]]
- [[spec/accessibility-comfort-slice-a]]
- [[systems/replay-determinism-and-run-evidence]]
- [[spec/actor-feel-sandbox-slice-a]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[spec/ai-trust-harness-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- `prototype-run-manifest.schema.json`
- `prototype-recorder-event.schema.json`
- `prototype-run-summary.schema.json`
- `../../research_tools/prototype_run_check.py`
