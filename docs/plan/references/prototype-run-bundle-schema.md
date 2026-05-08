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
  - DR-034
  - DR-035
  - DR-036
---

← [[references/sources|sources]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/full-collision-physics-plan|full collision plan]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036 material direction]] · [[comparables/noita-grade-material-simulation-research|noita-grade material research]] · [[spec/prototype-implementation-backlog-slice-a|historical Slice-A backlog]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[spec/accessibility-comfort-slice-a|accessibility/comfort Slice A]] · [[decisions/dr-013-backend-service-scope|DR-013 backend scope]] · [[systems/replay-determinism-and-run-evidence|determinism/run evidence]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]]

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
| `screenshots/` | HUD, overlay, event tail, replay viewer, workbench, or failure screenshots. | Listed in `summary.json.artifacts` |
| `captures/frame_<tick>.png` | Per-tick frame readbacks at the configured baseline cadence (10 Hz default) + event-triggered keyframes. Optional but required for any BP fun-proof slice from BP2 onward. | T-CAPTURE pipeline (`cf-capture` + `cf-app --capture-frames-hz`); listed individually in `summary.json.artifacts[].type="capture-frame"`. |
| `captures/grid_<NNN>.png` | 8×8 composite grid of consecutive captures with tick + HP + mission overlays. Composer: `game/tools/capture_grid.py`. | Listed in `summary.json.artifacts[].type="capture-grid"` with `frame_count`, `event_count`, `tick_first`, `tick_last`. |
| `captures/summary_grid.png` | Single-image high-level "what happened" grid: one frame per major event (`mission_*`, `terrain_carved`, `projectile_hit`, `actor_status_changed`, `weapon_fired`, `ai.state_changed`, `system.panic`), max 64 frames. Mandatory for any BP fun-proof slice. | Listed in `summary.json.artifacts[].type="capture-summary-grid"`. The AI-agent BP closure flow reads this image first. |
| `captures/grid.json` (alongside each grid PNG) | Composer-version + overlay-schema-rev + tick-frame mapping for deterministic regeneration + agent-readable event filter. | Composer-emitted next to each grid PNG. |

Validation command:

```bash
python3 /Users/erol/projects/corefall/game/tools/prototype_run_check.py /Users/erol/projects/corefall/prototype_runs/native/<run-id>
```

The checker intentionally avoids third-party dependencies. Corefall vendors the checker under `game/tools/`; the vault original remains `research_tools/prototype_run_check.py` for historical/browser-lab runs. JSON Schema files are the canonical field reference; `prototype_run_check.py` enforces the cross-file rules that plain schema validation cannot catch.

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
| `server` | `server.boot`, `server.mode_selected`, `server.config_loaded`, `server.client_admitted`, `server.client_dropped`, `server.snapshot_written`, `server.snapshot_restored`, `server.journal_flushed`, `server.persistence_recovered`, `server.health_probe`, `server.metrics_sample`, `server.drain_started`, `server.drain_completed`, `server.shutdown` | Dedicated server lifecycle (DR-034 / [[spec/server-app-architecture]]); audit mode selection, client admit/drop, persistence cadence, health/readiness probes, drain shutdown. Tokens never written; redacted by default. |
| `anti_cheat` | `anti_cheat.profile_applied`, `anti_cheat.input_rate_warning`, `anti_cheat.input_rate_kicked`, `anti_cheat.snapshot_drift`, `anti_cheat.capability_violation`, `anti_cheat.banned`, `anti_cheat.appeal_logged` | Server-authoritative anti-cheat foundation (DR-005 / DR-034); every rejection writes a reason label and parent-event chain; audit log appended for offline review. |
| `mmo` | `mmo.shard_started`, `mmo.shard_world_loaded`, `mmo.player_joined`, `mmo.player_left`, `mmo.contract_accepted`, `mmo.contract_resumed`, `mmo.contract_completed`, `mmo.faction_state_changed`, `mmo.commander_memory_written`, `mmo.cross_shard_handoff` | Persistent MMO shard mode (DR-035 / [[spec/persistent-mmo-architecture]]); audit shard lifecycle, account joins/leaves, contract director, faction memory, cross-shard handoffs. Account ids redacted by default. |
| `material` | `material.chunk_dirtied`, `material.chunk_slept`, `material.chunk_woken`, `material.active_region_changed`, `material.budget_exceeded`, `material.contact_started`, `material.phase_changed`, `material.density_swap`, `material.acid_contact`, `material.electricity_arc`, `material.debris_impact`, `material.first_divergence` | Active material kernel (DR-036 / T-MAT / [[comparables/noita-grade-material-simulation-research]]); audit per-chunk activity, sleep/wake transitions, phase transitions, density layering, hazard contacts, replay divergence per-chunk. Per-chunk material checksums in snapshots. |
| `reaction` | `reaction.triggered`, `reaction.byproduct_emitted`, `reaction.skipped_priority`, `reaction.skipped_threshold`, `reaction.catalyzed`, `reaction.recipe_journal_logged` | Reaction table engine (DR-036); audit pair/triple reaction firing, priority ordering, temperature thresholds, catalysts, byproducts, recipe-journal entries for material lab. Cause-chain links upstream `material.*` and downstream `damage.*`/`affliction.*`. |
| `affliction` | `affliction.set`, `affliction.cleared`, `affliction.escalated`, `affliction.decayed`, `affliction.stack_added` | Per-actor affliction layer (M5.7 / DR-036 / DR-037): `wetness`, `burning`, `corroded`, `electrified`, `poisoned`, `asphyxiating`, `concussed`, `drowning`, `depressurizing`, `internal_shock`, `coolant_leaking`, `oil_leaking`, `overheating`, `low_battery`, `power_starved`, `weak`, `exhausted`, `hypoxia`, `downclocked`, `heat_exhaustion`. HUD-visible; cause-chained to upstream `material.*` / `reaction.*` / `atmospherics.*` events. Origin-gated per [[spec/origin-reaction-and-resource-model]]. |
| `atmospherics` | `atmospherics.kernel_tick`, `atmospherics.atmosphere_created`, `atmospherics.atmosphere_destroyed`, `atmospherics.atmosphere_merged`, `atmospherics.flow`, `atmospherics.aperture_created`, `atmospherics.aperture_changed`, `atmospherics.liquid_flow`, `atmospherics.liquid_jet_force_applied`, `atmospherics.partial_pressure_changed`, `atmospherics.temperature_changed`, `atmospherics.thermal_transfer`, `atmospherics.thermal_device_tick`, `atmospherics.phase_change`, `atmospherics.combustion_started`, `atmospherics.combustion_consumed`, `atmospherics.combustion_stopped`, `atmospherics.room_breach`, `atmospherics.structure_rupture`, `atmospherics.suit_breach`, `atmospherics.suit_filter_choked`, `atmospherics.breath_inhaled`, `atmospherics.breath_exhaled`, `atmospherics.hazardous_atmosphere_detected`, `atmospherics.wind_force_applied`, `atmospherics.gas_stratified` | Stationeers-grade-or-better atmospherics/thermal kernel (DR-037 / M5.9 + extended M7.5): real PV=nRT atmosphere units (room cells, pipe networks, suits, canisters, lungs, device internals); deterministic combustion stoichiometry; gradual phase change; pipe network topology; door state machine + airlock cycles; weapon-created apertures; liquid/gas pressure jets; material heat transfer; suit/helmet/lung life-support with breathing math; per-planet ambient; wind from ΔP impulse force on entities; gas stratification proportional to local g × ΔM. Cause-chained to upstream `material.*` / `collision.*` / `gravity.*` events. |
| `gravity` | `gravity.field_changed`, `gravity.override_activated`, `gravity.override_deactivated`, `gravity.entity_entered_region`, `gravity.entity_exited_region` | Universal gravity field (DR-038 / extended M5.5 + M5.6 + M5.9): one source of truth read by every system; per-planet ambient + per-cell / per-region overrides (gravity well, low-g lab, magnetic boots, damaged grav generator, reverse-g chamber). Server-authoritative with override deltas replicated. |
| `ballistics` | `ballistics.projectile_launched`, `ballistics.projectile_step` (sparse), `ballistics.projectile_terminated`, `ballistics.terminal_velocity_reached`, `ballistics.fall_damage_threshold_crossed` | Ballistic trajectory math (DR-038 / extended M5.5): `a = (F_gravity + F_drag + F_collision) / m`; drag reads atmospherics ρ_local. Sparse trajectory step events to keep run-bundle volume bounded. Cause-chained to upstream `weapon_fired` / `gravity.*` / `atmospherics.*` events. |
| `world` | `world.loaded`, `world.parent_chain_resolved` | World catalog (DR-039 / new M5.10 + M7.7): identity + classification + per-world ambient + ore deposits + weather table; one-shot at scenario load. |
| `astrography` | `astrography.tick` (sparse), `astrography.distance_changed`, `astrography.comms_latency_changed`, `astrography.eclipse` | Full astrography (DR-039): orbital position + per-body distance + comms light-lag; sparse cadence (per minute of game time) to keep bundle volume bounded. |
| `environment` | `environment.signal_changed`, `environment.hazard_detected`, `environment.hazard_cleared`, `environment.bundle_snapshot`, `environment.aggregator_perf` | EnvironmentSignal aggregation (DR-040 / new M5.10): bundle deltas (sparse threshold-gated); periodic full snapshot per scenario-second for debug scrub. Cause-chained to upstream signal-producing kernel events. |
| `weather` | `weather.event_started`, `weather.event_progressed`, `weather.event_ended`, `weather.intensity_ramp`, `weather.precipitation_started/ended` | Weather kernel (M7.7 + DR-039 / DR-040): per-world weather variation events; deterministic firing per scenario seed. |
| `mining` | `mining.sampled`, `mining.drilled`, `mining.extracted`, `mining.refined`, `mining.smelted`, `mining.cargo_overflow`, `mining.deposit_exhausted`, `mining.market_trade`, `mining.theft_detected` | Mining and extraction (DR-041 / new M8.6): full sample → drill → extract → refine → smelt → trade pipeline; server-authoritative ledger; anti-cheat. |
| `match` | `match.started`, `match.team_state_changed`, `match.objective_progressed`, `match.victory_condition_met`, `match.player_joined`, `match.player_left`, `match.ai_filled_slot`, `match.bunker_breach_event`, `match.command_core_state_changed`, `match.match_ended` | Match grammar (DR-042 / extended M7 + M11 + M12): full match lifecycle; Bunker Defence-specific events (breach, command core); AI-fill events. |
| `voice` | `voice.transmission_started`, `voice.transmission_received`, `voice.transmission_blocked`, `voice.shouted` | Voice acoustic propagation (DR-043 / new M9.5): per-receiver propagation events with reason labels (vacuum / sealed_helmet / hearing_damage / below_threshold). |
| `radio` | `radio.tuned`, `radio.transmission_started`, `radio.transmission_received`, `radio.transmission_blocked`, `radio.encryption_changed`, `radio.antenna_alignment_changed`, `radio.interference_event` | Radio propagation (DR-043 / new M9.5): ACRE2 multipath model; SNR + path_kind + interference; encryption + antenna events. |
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

## DR-002 v1 Lock Extensions (added 2026-05-05 in M0 pass)

The DR-002 hybrid event-log + snapshots lean was confirmed during M0 implementation. The additions below extend (do NOT replace) the canonical contract:

### `run_manifest.json` extensions

| Field | Type | Meaning |
|---|---|---|
| `checksum.algorithm` | string | Hash function used for `determinism.sim_checksum` events. Default: `"blake3"`. |
| `checksum.scope` | string | Named state scope hashed. M0 ships `"sim_state_v1"` covering `tick_counter || rng_state_bytes`. M2 appends terrain chunk material grid; M3 appends actor/inventory state; layout-breaking changes bump to `_v2` and register a migration. |
| `checksum.cadence_ticks` | u64 | Ticks between automatic checksum emits (M0: 60). |
| `tick_rate_hz` | u32 | Configured fixed-tick rate the run was executed at (M0 defaults to 60; 120 Hz is a first-class option). Run bundles MUST record the rate that was actually used; the engine MUST NOT hardcode a rate that disagrees with this field. |
| `settings` | object | Flat KV block of the six DR-012 accessibility flags active at run start: `ui_scale`, `high_contrast`, `captions`, `reduced_motion`, `reduced_shake`, `reduced_flash`. |

### `summary.json` extensions

| Field | Type | Meaning |
|---|---|---|
| `final_sim_checksum` | string \| null | Hex of the last `determinism.sim_checksum` event payload. M0.1 onward: bundles MUST emit at least one final `determinism.sim_checksum` on `run_finished` so this field is non-null on a valid run. Empty/0-tick runs MAY still report null. |
| `checksum_event_count` | u64 | Number of `determinism.sim_checksum` events recorded. M0.1 onward: ≥ 1 on every successful run. |
| `first_tick` | u64 \| null | Tick of the first event in `events.jsonl`. |
| `last_tick` | u64 \| null | Tick of the last event in `events.jsonl`. |
| `performance.tick_rate_hz` | u32 | Configured tick rate this run targeted (mirror of `run_manifest.json.tick_rate_hz`). Lets summary-only consumers compare wall-clock pacing without reopening the manifest. |
| `performance.p99_tick_ms` | f64 | 99th-percentile per-tick cost in milliseconds. M0 captures wall time spent inside `M0Engine::drive_tick`; future milestones append substep / kernel costs without changing the field name. |
| `performance.avg_tick_ms` | f64 | Mean per-tick cost in milliseconds, reported alongside `p99_tick_ms`. |
| `performance.wall_seconds` | f64 | Wall-clock seconds the run consumed. For a paced run, this should be ≈ `ticks_run / tick_rate_hz`. |

These extensions are append-only on the schema; the run-bundle checker does not require them and will not reject manifests/summaries that omit them. Future milestones (M3 closure of DR-002) MAY tighten the checker once snapshot/replay verification flows land.

## Native Milestone Acceptance Gates

| Milestone | Run Bundle Must Prove |
|---|---|
| M0 engine bootstrap | Native app starts/ends cleanly, seed/config/build metadata are captured, fixed-tick smoke evidence exists, `cfctl observe/run` evidence exists, the bundle validates with `prototype_run_check.py`, and the v1 checksum/settings extensions above are populated. |
| M1 actor controller | Input, movement, aim, weapon, reload, status, HUD, semantic control actions, actor/equipment observations, and recorder events are captured from the native controller scene. |
| M1.5 micro breach fun slice | Win/loss state, objective timer, reactive enemy behavior, temporary soft-breach surface edits, control-driven win/loss scripts, observation stream freshness, and HUD objective readability are captured. |
| M2 terrain/materials | Material probe, penetration, carve/fill, dirty-region refresh, path refresh hooks, and performance counters are captured from mutable terrain actions. |
| M3 replay/event recorder | Event cause chains, snapshots, dropped-event counters, deterministic replay checks, and viewer artifacts are present enough to debug a run without watching it live. |
| M4 HUD/comic-noir UI | HUD, overlays, death/material explanations, accessibility settings, caption evidence, and screenshots/captures show the player-facing state clearly. |
| M5 equipment/chassis | Item role labels, damage-stage state, armor/chassis effects, bot-usable fields, loadout validation, repair/salvage, and ejection/disable evidence are captured. |
| M5.5 full collision gauntlet | `collision.*` events captured for collision matrix coverage, limb/body/equipment/mech/base/projectile contacts, projectile-projectile deflection/fuze/detonation cases, CCD/tunneling fixtures, impulse-to-damage routing, collision-filter reasons, `cfctl observe --collisions`, perf counters, and headless replay checksums. |
| M5.6 material kernel | `material.*` and `reaction.*` events captured for chunked CA kernel, sleep/wake transitions, density layering, phase change, reaction table firing with priority/threshold/catalyst evidence, per-chunk material checksums in snapshots, `cfctl observe --materials/--reactions`, headless replay checksums match (DR-036 / T-MAT). |
| M5.7 hazard package | `material.*`, `reaction.*`, `affliction.*`, `damage.*` chained events for acid/electricity/debris/ingestion-stub damage routing through M5.5 impulse pathway and the affliction layer; HUD overlay screenshots; AI-H regression remains green (DR-036 / T-MAT). |
| M6 AI trust harness | Bot intent, perception facts, doctrine/personality labels, mistakes, recovery actions, blocked-path reasons, and explanation overlays are captured by AI-H scenarios. |
| M6.5 LLM mind lab | `mind.*` events captured for every task: prompt hash (raw text only when `debug` capability is on), response hash, validator result with reasons, applied patch ids, rejected proposals, memory writes; mock-provider runs are deterministic; live provider runs are flagged but never required for CI. |
| M6.6 AI material competence | AI-MAT-01..AI-MAT-08 acceptance suite passes; `ai_hazard_map_updated` events with fog-of-war audit; `tactic_chosen` and `tactic_scored` events carry affordance-tag reasons; `ai_path_avoided_hazard`, `ai_recovery_action`, `ai_friendly_fire_check`, `ai_hazard_exploit` events captured; AI-H regression remains green (DR-036 / DR-022). |
| M7 mission director | Manifest-driven objectives, director events, command-core/base-power state, debrief/retry state, and scenario completion/failure evidence are captured. |
| M7.5 base atmospherics | `atmospherics.*` events captured for hull/room state, gap/aperture topology, flooding, pressure equalization, breach decompression, liquid/gas jets, material heat transfer, fire propagation, smoke fill, toxic gas migration, pump/vent/thermal-device actions; mission director hull-state objectives evaluated; `cfctl observe --atmospheres`; server-authoritative replay checksums match (DR-036 / DR-037 / DR-005 / DR-034 / DR-035). |
| M9 dedicated server app | `server.*` events captured for boot, mode selection, config load, client admit/drop, persistence cadence, health/readiness probes, drain shutdown; M9 server-core subset passes against checked run bundles. |
| M10 LAN co-op | Per-client run bundles archived; replay-compare aligns tick-for-tick; mod hash sync events captured; `anti_cheat.profile_applied` logs profile `casual`. |
| M11 online co-op (self-hosted dedicated servers) | Per-client run bundles archived through NAT/relay; lobby_directory register/heartbeat/deregister captured; mod hash sync diff captured on mismatch; `anti_cheat.input_rate_kicked` logged when client misbehaves. |
| M12 PvP arena + persistent MMO shard | `mmo.*` events captured for shard lifecycle, player join/leave, contract director, faction memory, cross-shard handoff; MMO-001..MMO-012 acceptance suite passes; PvP arena per-match bundles align. |
| M8 editor/mod tools | Edited scenario/package data, validation diagnostics, content hashes, sample mod load evidence, and workbench screenshots are captured. |
| M8.5 material lab | `cf-tools-editor --mode material_lab` workbench evidence: brush/inspect/recipe-journal/stamp captures; designer authoring transcript ≤10 minutes; sample expansion material pack validates with `cf-mod validate --strict`; new material affordance verified by AI puppet test; modded run bundle archived (DR-036 / T-MAT). |
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
- [[decisions/dr-036-systemic-material-simulation-direction]]
- [[comparables/noita-grade-material-simulation-research]]
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
- `../../research_tools/prototype_run_check.py` (vault original) / `/Users/erol/projects/corefall/game/tools/prototype_run_check.py` (Corefall vendored copy)
