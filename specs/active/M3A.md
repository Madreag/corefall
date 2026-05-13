# M3A — Event Recorder Core

## Status

`active`

## Intent

The event taxonomy, snapshot writer, and headless replay verifier are complete enough that any prior milestone's run can be replayed off-line and produce identical state checksums per tick. Determinism is real, drift is detected, and the run bundle envelope is the source of truth for everything an offline reviewer needs.

## Player-facing behavior

(M3A is infrastructure, not directly player-facing — but it underpins every visible feature: death recap, debug, AI trust, grading, future networking.)

- **Every player action emits a typed event** into a deterministic stream. The event taxonomy (**38 categories** — 27 from baseline + 9 added at M2.5: `hazard`, `shield`, `thermal`, `environment`, `armor`, `internal`, `concussion`, `fluid`, `origin` + 1 added at M5: `module` + 1 added at M5.8: `resource`) is the single source of truth for what the sim did.
- **`cf-headless replay <bundle>`** replays a run off-line and prints `result=ok` (matching) or `result=divergence` with structured `first_divergence` (tick, recorded, live checksums). Non-zero exit on divergence.
- **`cfctl observe --once`** returns a snapshot of current sim state — actor, inventory, mission, terrain summary, hazards, afflictions, armor layers, atmospherics.
- The **run bundle directory** under `prototype_runs/native/<run-id>/` is the offline-review contract: `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`, `screenshots/`, `captures/` (per-tick keyframes + 8×8 grids + `summary_grid.png`), `expected_outcome` enforced.
- **Replay reproducibility**: the same manifest + seed + inputs reproduces the same final state byte-for-byte. Per-tick `blake3` checksum (`determinism.sim_checksum`) anchors this contract.
- **Death recap, AI debug, mission grading** all read from the run bundle — never from live state.
- **Backpressure is visible**: if the recorder drops events under combat density, the `dropped_count` is exported and the canonical checker flags it.

### M2.5 firehose surface — what M3A MUST handle without renaming

M2.5 introduces a high-density firehose of damage/destructible-terrain/hazard/affliction/atmospheric/shield/thermal/environment events. M3A LOCKS the schemas for every M2.5 event family so M5+M5.5+M5.7+M5.9+M5.10 ladder up additively. The locked surfaces are:

| Family | Active producers at M2.5 | Registered producers (later milestones) |
|---|---|---|
| `terrain.*` | terrain_carved, material_state_changed (5-tier), pixel_removed, debris_spawned, cascade_triggered, terrain_dirty_region_batch | terrain_penetration_threshold (M5.5), terrain_repaired (M5+) |
| `body.*` / `combat.*` | actor_status_changed, weapon_fired, projectile_spawned, projectile_hit_mo (full expanded payload — ap_factor / armor_effective_hardness / armor_absorbed_dmg / passthrough_dmg / surface_kind / layer_struck / organ_damaged_id / circuit_damaged_id), projectile_hit_terrain, wound_added (scalar M2.5), reactor.armor_layer_hp_changed, reactor.armor_layer_cracked, reactor.armor_layer_destroyed | wound_added (per-zone M5), attachable_detached (M5), gib_created (M5), gib_cascade (M5) |
| `hazard.*` | hazard.spawned, hazard.spread, hazard.tick (cosmetic batched), hazard.dissipated, hazard.actor_contact | (M2.5 producer-active; M5.7+ ladders advanced rules) |
| `affliction.*` | affliction.applied, affliction.tick (cosmetic batched), affliction.cleared, affliction.escalated | (M2.5 producer-active for cfctl-injected; M5.7+ ladders environmental sources) |
| `atmos.*` | atmos.pressure_changed, atmos.gas_released, atmos.breach_detected, atmos.temperature_changed (placeholder values) | (M5.9 fills real kernel) |
| `shield.*` | (schemas only; no producer at M2.5) | shield.hit, shield.depleted, shield.regen_started, shield.regen_completed, shield.disrupted (M5+) |
| `thermal.*` | thermal.signature_changed | thermal.conduction_event (M5.7+), thermal.phase_change (M5.9+) |
| `environment.*` | environment.signal_delta (M5.10 surface; placeholder payload at M2.5) | (M5.10 fills aggregator) |
| `armor.*` (NEW @ M2.5) | armor.layer_hp_changed, armor.layer_critical, armor.layer_destroyed, armor.all_layers_destroyed, armor.chunked_off, armor.debris_spawned, armor.repaired | (M5+ chassis armor producer fills with per-zone × per-layer state) |
| `internal.*` (NEW @ M2.5) | internal.organ_damaged, internal.organ_destroyed, internal.organ_failure_cascade, internal.circuit_damaged, internal.circuit_destroyed, internal.circuit_failure_cascade (schemas locked; cfctl can inject for testing) | (M5+ chassis/origin model fills producers) |
| `concussion.*` (NEW @ M2.5) | concussion.dose_changed, concussion.band_changed, concussion.ko_threshold_crossed, concussion.recovered, internal_shock.dose_changed (robot equivalent), internal_shock.module_damaged | (M5.8 origin reaction model fills producer at scale) |
| `fluid.*` (NEW @ M2.5) | fluid.leak_started, fluid.leak_rate_changed, fluid.reservoir_warning, fluid.reservoir_critical, fluid.reservoir_empty, fluid.ignition, fluid.ground_splatter_spawned (cosmetic), fluid.leak_stopped, fluid.refilled | (M5+ chassis fluid producer fills with per-reservoir state) |
| `origin.*` (NEW @ M2.5) | origin.shot_force_feedback (g_load_delta + concussion_dose_delta + internal_shock_module_id + leak_channel + screen_kick_intensity), origin.g_load_dose_changed, origin.helmet_breach, origin.oxygen_supply_changed | (M5.8 origin reaction model fills full producer; M2.5 schemas locked) |
| `ability.*` | (registered, no producer at M2.5) | (M5+ chassis abilities) |
| `chassis.*` | (registered) | armor_layer_hp_changed, armor_layer_cracked, armor_layer_destroyed, module_state_changed, fuel_state, heat_state, eject_window_open, pilot_bailed_too_late, salvage_finished, etc. (M5+) |

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-replay` | MODIFY | **Event envelope (locked at v0.1)**: `schema_version`, `run_id`, `tick`, `sim_time_ms`, `event_id`, `parent_event_id` (Optional), `category`, `event_type`, `actor_id` (Optional), `source_id` (Optional), `team` (Optional), `pos` (Optional), `bbox` (Optional), `payload` (JSON), `dropped_count` (Optional), `cosmetic: bool` (DR-052 forward-compat — excluded from determinism island). **27-category taxonomy** (see Acceptance Criteria § Event taxonomy for full list + per-category typical event types + producer status). **Recorder API**: `Recorder::with_capacity(N)` (ring buffer); `event_count() -> u64`; `dropped_count() -> u64`; `priority_threshold` (cosmetic events drop first under pressure); non-blocking append (no sim-thread stall); reentrancy guard (recorder hooks emit inert data only — no subscriber mutation per CCCP `Atom.cpp:96`). **Stable record_id layer**: `RecordId(u64)` registry for actors/items/projectiles/chunks — NEVER use raw pointers or pooled MOIDs (per CCCP `MovableMan.cpp:126-143` stale-pointer warning). |
| `cf-control` | MODIFY | `M0EngineConfig.checksum_cadence_ticks` (default 60; configurable per scenario). `ConfigInputs.checksum_cadence_ticks: Option<u64>`. `system.run_started` event payload carries `protocol_version` (cf-control SCHEMA_VERSION). `system.category_baseline` event fires once at run start listing all 27 categories with status `active` (has producer) or `registered` (producer ladders up at later milestone). `system.run_finished` event carries `outcome: "clean" \| "panic" \| "abort"`. **Snapshot writer** fires: `snapshot_actor` + `snapshot_inventory` + `snapshot_terrain_chunk` (per dirty chunk) + `snapshot_terrain_summary` (material distribution) at: scene start AND every objective transition (`mission.objective_{started,updated,completed,failed,paused,resumed}`). `runbundle.write` rejects path-traversal in `id_override` (rejects `../`, `/`, `\`). |
| `cf-headless` | MODIFY (already partly landed) | `cargo run -p cf-headless -- replay <bundle> [--no-verify-checksums] [--scenario-path <path>] [--measure throughput] [--max-no-advance-retries N]`. Walks `events.jsonl`, parses every `control.command_accepted` back to `ControlCommand`, dispatches against fresh `M0Engine` from manifest, verifies per-tick `determinism.sim_checksum`. Outputs structured JSON on stdout: success `{result: "ok", replayed_ticks, checksums_verified, commands_replayed, final_run_id}`, divergence `{result: "divergence", first_divergence: {tick, recorded, live}, all_divergences: [...], total_divergences}`. Hard-codes `write_run_bundle: false` on every replayed RunForTicks (verifier never writes side-effect bundles). MAX_NO_ADVANCE_RETRIES iteration guard prevents infinite-loop on stalled engine. |
| `cf-app` | MODIFY | `--checksum-cadence-ticks <N>` CLI flag → `ConfigInputs.checksum_cadence_ticks`. `--tick-rate-hz <N>` already exists; bundles MUST record `tick_rate_hz` in `run_manifest.json`. `--write-run-bundle` flag with `--expected-outcome <clean\|panic\|abort>` for the caller's declared outcome. `--headless-smoke` for no-window runs (CI safety). |
| `cf-mod` | MODIFY | `cargo run -p cf-mod -- validate content/` validates `prototype-recorder-event` schema files + scenario manifests against the locked v0.1 envelope. |
| Run-bundle checker (Python) | MODIFY | `game/tools/prototype_run_check.py` validates: required files exist (run_manifest.json + events.jsonl + summary.json + notes.md), JSON parses + `schema_version` constants match, `summary.json.manifest_run_id == run_manifest.json.run_id`, every event's `run_id` matches manifest, event_ids unique within run, ticks monotonic, `parent_event_id` references resolve, `summary.json.event_counts.total == JSONL line count`, `event_counts.by_category` + `by_type` match actual events, test evidence ids exist in events.jsonl, `dropped_total >= sum(dropped_count per event)`, notes.md has 6 required headings (Assumptions Tested / Good / Bad / Meh / Evidence Links / Next Actions), `expected_outcome` matches `system.run_finished.outcome` else REJECT with structured error. |
| Documentation | NEW | `docs/plan/spec/determinism-island-contract.md` — explicit list of: **deterministic subsystems** (cf-sim-core fixed-tick loop, terrain mutation, AI decisions, equipment RNG, mission state machine, projectile RNG, stability/recoil math), **non-deterministic subsystems** (cf-audio cosmetic, cf-render-2d particle cosmetic — both flagged `cosmetic: true`), **the contract** (replay verifier ignores `cosmetic: true` events; cross-platform float-determinism rules per DR-052: `f32` only in sim islands, `RUSTFLAGS=-C target-feature=+sse2,+sse4.2`, LLVM `-ffast-math` disabled, no `f64` in sim crates). |

## Files

Source:
- `game/crates/cf-replay/src/lib.rs` (MODIFY: envelope v0.1 + 27 categories + RecordId + Recorder API)
- `game/crates/cf-replay/src/recorder.rs` (MODIFY: ring buffer + backpressure + priority threshold + reentrancy guard)
- `game/crates/cf-replay/src/event.rs` (MODIFY: 27 categories + cosmetic flag)
- `game/crates/cf-replay/src/record_id.rs` (NEW: stable id layer; lifecycle events for actor/item/chunk lifetimes)
- `game/crates/cf-replay/src/manifest.rs` (NEW or MODIFY: RunManifest struct with all DR-002 v1 extensions)
- `game/crates/cf-replay/src/summary.rs` (NEW or MODIFY: RunSummary struct + writers)
- `game/crates/cf-replay/src/checksum.rs` (NEW or MODIFY: blake3 over sim_state_v1 scope; per-tick cadence)
- `game/crates/cf-control/src/engine.rs` (MODIFY: snapshot writer cadence, system.run_started.protocol_version, system.category_baseline, system.run_finished.outcome, checksum_cadence_ticks plumbing)
- `game/crates/cf-control/src/server.rs` (MODIFY: runbundle.write path-traversal guard + expected_outcome param)
- `game/crates/cf-control/src/runtime.rs` (MODIFY: ConfigInputs.checksum_cadence_ticks)
- `game/crates/cf-headless/src/main.rs` (MODIFY: replay subcommand + structured divergence output + --no-verify-checksums + --scenario-path + --measure throughput)
- `game/crates/cf-headless/src/parse_command.rs` (MODIFY: covers every M0..M5 control method round-trip)
- `game/crates/cf-app/src/main.rs` (MODIFY: --checksum-cadence-ticks + --expected-outcome + --headless-smoke)
- `game/crates/cf-capture/src/lib.rs` (MODIFY or NEW if stub: per-tick frame readback, integrated with run-bundle captures/)

Tooling:
- `game/tools/prototype_run_check.py` (MODIFY: expected_outcome validation + 12 cross-file consistency rules + 6 required notes.md headings)
- `game/tools/replay_throughput_bench.py` (NEW: measures ticks/sec replay throughput across bundles for perf regression)

Schemas:
- `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json` (LOCKED — additive-only extensions)
- `game/crates/cf-replay/schemas/v1/run_manifest.schema.json` (MODIFY: all DR-002 v1 extensions)
- `game/crates/cf-replay/schemas/v1/run_summary.schema.json` (MODIFY: final_sim_checksum, checksum_event_count, first_tick, last_tick, performance counters)
- `game/crates/cf-replay/schemas/event/system_run_started.json` (NEW)
- `game/crates/cf-replay/schemas/event/system_run_finished.json` (NEW)
- `game/crates/cf-replay/schemas/event/system_category_baseline.json` (NEW)
- `game/crates/cf-replay/schemas/event/determinism_sim_checksum.json` (NEW)
- `game/crates/cf-replay/schemas/event/determinism_first_divergence.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_actor.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_inventory.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_terrain_chunk.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_terrain_summary.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_chassis.json` (NEW — M5 forward-compat)

Documentation:
- `docs/plan/spec/determinism-island-contract.md` (NEW — names deterministic vs non-deterministic subsystems)
- `docs/plan/references/prototype-run-bundle-schema.md` (EXISTS — the canonical envelope reference; M3A spec aligns to it)

Scripts:
- `game/scripts/cfctl/m3a_replay_compare.cfctl.json` (EXISTS — m2.5 win bundle replay match)
- `game/scripts/cfctl/m3a_5min_endurance_m1.cfctl.json` (NEW — literal 18000-tick run-bundle for the 5-minute claim)
- `game/scripts/cfctl/m3a_panic_outcome.cfctl.json` (NEW — induces panic + verifies checker rejects expected_outcome=clean)
- `game/scripts/cfctl/m3a_abort_outcome.cfctl.json` (NEW — act.player.abort → expected_outcome=abort)
- `game/scripts/cfctl/m3a_first_divergence_injection.cfctl.json` (NEW — runs scenario, then a test mutates one event byte, runs replay, expects first_divergence)
- `game/scripts/cfctl/m3a_backpressure_burst.cfctl.json` (NEW — generates >capacity events/tick to verify dropped_count surface)
- `game/scripts/cfctl/m3a_120hz_replay_determinism.cfctl.json` (NEW — same script @ 60Hz and 120Hz; bundles replay individually)

## Acceptance criteria

### Event taxonomy (27 categories)

```gherkin
Scenario: 36-category baseline declared at run start with producer status
  Given any cf-app run
  When the run starts
  Then system.category_baseline event fires once at tick 0 with payload:
    {
      "schema_version": 1,
      "categories": [
        { "name": "input",         "status": "active",     "first_event_type": "input.intent_received" },
        { "name": "control",       "status": "active",     "first_event_type": "control.command_received" },
        { "name": "mind",          "status": "registered", "ladder_at": "M6.5" },
        { "name": "collision",     "status": "registered", "ladder_at": "M5.5" },
        { "name": "server",        "status": "registered", "ladder_at": "M9" },
        { "name": "anti_cheat",    "status": "registered", "ladder_at": "M9" },
        { "name": "mmo",           "status": "registered", "ladder_at": "M12" },
        { "name": "material",      "status": "registered", "ladder_at": "M5.6" },
        { "name": "reaction",      "status": "registered", "ladder_at": "M5.6" },
        { "name": "atmospherics",  "status": "active",     "first_event_type": "atmos.pressure_changed" },
        { "name": "affliction",    "status": "active",     "first_event_type": "affliction.applied" },
        { "name": "hazard",        "status": "active",     "first_event_type": "hazard.spawned" },
        { "name": "shield",        "status": "registered", "ladder_at": "M5+" },
        { "name": "thermal",       "status": "active",     "first_event_type": "thermal.signature_changed" },
        { "name": "environment",   "status": "active",     "first_event_type": "environment.signal_delta" },
        { "name": "armor",         "status": "active",     "first_event_type": "armor.layer_hp_changed" },
        { "name": "module",        "status": "registered", "ladder_at": "M5+" },
        { "name": "resource",      "status": "registered", "ladder_at": "M5.8" },
        { "name": "internal",      "status": "active",     "first_event_type": "internal.organ_damaged" },
        { "name": "concussion",    "status": "active",     "first_event_type": "concussion.dose_changed" },
        { "name": "fluid",         "status": "active",     "first_event_type": "fluid.leak_started" },
        { "name": "origin",        "status": "active",     "first_event_type": "origin.shot_force_feedback" },
        { "name": "combat",        "status": "active",     "first_event_type": "combat.weapon_fired" },
        { "name": "body",          "status": "active",     "first_event_type": "actor.actor_status_changed" },
        { "name": "terrain",       "status": "active",     "first_event_type": "terrain.terrain_carved" },
        { "name": "ai",            "status": "active",     "first_event_type": "ai.state_changed" },
        { "name": "logistics",     "status": "registered", "ladder_at": "M7" },
        { "name": "mission",       "status": "active",     "first_event_type": "mission.mission_started" },
        { "name": "system",        "status": "active",     "first_event_type": "system.run_started" },
        { "name": "snapshot",      "status": "active",     "first_event_type": "snapshot.snapshot_actor" },
        { "name": "determinism",   "status": "active",     "first_event_type": "determinism.sim_checksum" },
        { "name": "ux",            "status": "active",     "first_event_type": "ux.banner_raised" },
        { "name": "accessibility", "status": "active",     "first_event_type": "accessibility.settings_changed" },
        { "name": "performance",   "status": "active",     "first_event_type": "performance.tick_cost_sample" },
        { "name": "equipment",     "status": "active",     "first_event_type": "equipment.weapon_fired" },
        { "name": "chassis",       "status": "registered", "ladder_at": "M5" },
        { "name": "actor",         "status": "active",     "first_event_type": "actor.snapshot" },
        { "name": "ability",       "status": "registered", "ladder_at": "M5+" }
      ]
    }
  And the payload exactly matches docs/plan/references/prototype-run-bundle-schema.md § Event Category Baseline
  (Categories `hazard`, `shield`, `thermal`, `environment`, `armor`, `internal`, `concussion`, `fluid`, `origin` are NEW at M2.5; `atmospherics` and `affliction` upgrade from `registered` to `active` because M2.5 fires placeholder events. Total: 36 categories = 27 baseline + 9 M2.5 deep-damage additions.)

Scenario: Locked event-type taxonomy per category (M2.5 active surfaces)
  Given the system.category_baseline event fires
  And the per-category event-type schema list is loaded
  Then the `terrain.*` category accepts these event types (M2.5 locked):
    terrain_carved, terrain_dirty_region_batch, material_state_changed, pixel_removed, debris_spawned, debris_capped, cascade_triggered, tool_refused, terrain_penetration_threshold, terrain_pixel_dislodged, anchor_material_result, terrain_fill_or_repair, terrain_material_probe, hazard_contact_or_avoidance, path_invalidated
  And the `combat.*` category accepts (M2.5 locked):
    weapon_fired, projectile_spawned, projectile_hit_mo, projectile_hit_terrain, wound_added, kill, reactor.armor_layer_hp_changed, reactor.armor_layer_cracked, reactor.armor_layer_destroyed
  And the `hazard.*` category accepts (M2.5 locked):
    spawned, spread, tick (cosmetic), dissipated, actor_contact
  And the `affliction.*` category accepts (M2.5 locked):
    applied, tick (cosmetic), cleared, escalated
  And the `atmos.*` (atmospherics) category accepts (M2.5 locked placeholder, M5.9 fills):
    pressure_changed, gas_released, breach_detected, temperature_changed, phase_change (M5.9), combustion_ignition (M5.9), pipe_flow (M5.9+)
  And the `shield.*` category accepts (M2.5 locked, M5+ fills):
    hit, depleted, regen_started, regen_completed, disrupted
  And the `thermal.*` category accepts (M2.5 locked):
    signature_changed, conduction_event (M5.7+), phase_change (M5.9+)
  And the `environment.*` category accepts (M2.5 locked):
    signal_delta (M5.10 fills payload), hazard_band_changed (M5.7+)
  And the `ai.*` category accepts (M2.5 locked):
    state_changed, target_acquired, target_lost, target_scored, perception_signal, tactic_chosen, path_invalidated, recovery_action
  And the `mission.*` category accepts:
    mission_started, objective_started, objective_updated, objective_completed, objective_failed, objective_paused, objective_resumed, mission_resolved, reactor_hp_changed, reactor_destroyed, reactor_pressure_state_changed, timer_warning_threshold, objective_progress_updated, director_phase_change
  And the `armor.*` category accepts (M2.5 locked):
    layer_hp_changed, layer_critical, layer_destroyed, all_layers_destroyed, chunked_off, debris_spawned, repaired, angle_deflection_calculated, ricochet, spalling, spalling_fragment_spawned, spalling_fragment_hit_module, penetration_ray_traversed, he_overpressure_wave, heat_jet_penetrated, heat_jet_pre_detonated_by_era, apfsds_penetrated, era_panel_detonated, schurzen_pre_detonated, multi_hit_degradation (ceramic), reactive_armor_consumed
  And the `module.*` category accepts (M5+ chassis):
    module_hp_changed, module_state_changed, module_penetrated_by_ray, module_spalling_damage, module_destroyed, ammo_rack_cooking, ammo_rack_detonated, engine_fire_started, engine_destroyed, optics_damaged, optics_blind, transmission_damaged, transmission_immobile, crew_knockout, crew_revived, cockpit_breach, reactor_pressure_advanced, motor_controller_damaged
  And the `resource.*` category accepts (M5.8 origin reaction model — "no HP bar" survival):
    changed (kind: blood|oil|power|caloric|bio_fluid|oxygen_supply + from + to + cause), critical (threshold 30%/10%/0%), depleted (at 0%), restored (medikit/repair_tool/food/recharge/transfusion), drain_rate_changed (from rate + to rate + source_affliction_id), cascade_offline (kind + organ_id + reason)
  And the `internal.*` category accepts (M2.5 locked):
    organ_damaged, organ_destroyed, organ_failure_cascade, circuit_damaged, circuit_destroyed, circuit_failure_cascade
  And the `concussion.*` category accepts (M2.5 locked):
    dose_changed, band_changed, ko_threshold_crossed, recovered, internal_shock.dose_changed (robot equivalent under same category), internal_shock.module_damaged
  And the `fluid.*` category accepts (M2.5 locked):
    leak_started, leak_rate_changed, reservoir_warning, reservoir_critical, reservoir_empty, ignition, ground_splatter_spawned (cosmetic), leak_stopped, refilled
  And the `origin.*` category accepts (M2.5 locked):
    shot_force_feedback, g_load_dose_changed, helmet_breach, oxygen_supply_changed
  And cf-mod validates the registry against the locked v0.1 schema
  And M3B's plain-language renderer ships templates for every type listed

Scenario: Event envelope schema v0.1 is locked
  Given any event written to events.jsonl
  Then the line is a JSON object containing: schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload
  And optionally: parent_event_id, actor_id, source_id, team, pos, bbox, dropped_count, cosmetic
  And the schema_version field is "0.1" for M3A
  And no field outside this set is present (additive extensions require a schema bump)

Scenario: Cosmetic events excluded from determinism island
  Given an event with cosmetic=true (e.g. cf-render-2d particle effect)
  When cf-headless replay walks the bundle
  Then the cosmetic event is skipped for checksum verification
  And only non-cosmetic events contribute to sim_state checksum
  (DR-052 cosmetic vs gameplay split)
```

### Run bundle directory structure + manifest contract

```gherkin
Scenario: Bundle directory has the full file set
  Given cf-app --write-run-bundle micro_breach
  When the run completes
  Then prototype_runs/native/<run-id>/ contains:
    - run_manifest.json
    - events.jsonl
    - summary.json
    - notes.md (with 6 required headings: Assumptions Tested / Good / Bad / Meh / Evidence Links / Next Actions)
    - screenshots/ (optional)
    - captures/ (if --capture-grid: frame_<tick>.png + grid_<NNN>.png + summary_grid.png + grid.json)
  And the run-id follows `<milestone>_<UTC-ISO-8601-hyphenated>_<short_hash>` (e.g. m1_2026-05-11T18-30-00Z_a1b2c3d4)

Scenario: run_manifest.json contract
  Given a fresh run bundle
  Then run_manifest.json includes:
    - run_id (must equal the directory name)
    - schema_version
    - build_id (cf-app git commit short hash)
    - scenario_id (e.g. "micro_breach")
    - seed (u64)
    - tick_rate_hz (60 or 120)
    - material_schema_version (from cf-material)
    - config_hash (blake3 of effective settings)
    - assumptions_tested[]
    - expected_tests[] (acceptance test ids the run intends to evidence)
    - checksum.algorithm ("blake3")
    - checksum.scope ("sim_state_v1" or higher)
    - checksum.cadence_ticks (default 60)
    - expected_outcome ("clean" | "panic" | "abort")
    - settings { ui_scale, captions, reduced_motion, reduced_shake, reduced_flash, hold_to_confirm, hold_threshold_ms, key_remap_enabled, key_bindings }

Scenario: summary.json contract
  Given a completed run bundle
  Then summary.json includes:
    - manifest_run_id (must equal run_manifest.json.run_id)
    - final_sim_checksum (hex of the last determinism.sim_checksum event payload; must be non-null on a valid M0.1+ run)
    - checksum_event_count (must be ≥ 1)
    - first_tick (u64), last_tick (u64)
    - event_counts.total (must equal JSONL line count)
    - event_counts.by_category (must match actual event distribution)
    - event_counts.by_type
    - event_counts.dropped_total (must be ≥ sum of per-event dropped_count fields)
    - performance.tick_rate_hz (mirror of manifest)
    - performance.p99_tick_ms
    - performance.avg_tick_ms
    - performance.wall_seconds
    - artifacts[] (paths to screenshots/captures with type tags)
    - blockers[]
    - next_actions[]

Scenario: Cross-file consistency rules enforced
  Given a bundle that fails any of these:
    - summary.json.manifest_run_id ≠ run_manifest.json.run_id
    - Some event.run_id ≠ manifest.run_id
    - Duplicate event_id within a run
    - Event ticks not monotonic
    - parent_event_id references an event not in the bundle (without explicit unknown_cause marker)
    - summary.event_counts.total ≠ JSONL line count
    - summary.event_counts.by_category ≠ actual category counts
    - dropped_total < sum of per-event dropped_count
    - Test evidence event_ids missing from events.jsonl
    - notes.md missing any of the 6 required headings
  When prototype_run_check.py runs
  Then exit code is non-zero with structured error pointing at the rule violated
```

### Replay determinism + headless verifier

```gherkin
Scenario: 5-minute literal run replays headlessly with matching checksums
  Given a 5-minute m1_actor_range run bundle (literal 18000 ticks at 60Hz; produced by m3a_5min_endurance_m1.cfctl.json)
  When `cargo run -p cf-headless -- replay <bundle>` runs
  Then stdout JSON includes: { "result": "ok", "replayed_ticks": 18000, "checksums_verified": ≥300, "commands_replayed": N, "final_run_id": "..." }
  And every recorded determinism.sim_checksum matches the live re-run
  And no first_divergence event fires
  And exit code is 0

Scenario: First-divergence reporting on tampered bundle
  Given a run bundle with one event payload mutated (e.g. velocity field changed)
  When cf-headless replay runs
  Then stdout JSON includes: { "result": "divergence", "first_divergence": { "tick": N, "recorded": "<hex>", "live": "<hex>" }, "all_divergences": [...], "total_divergences": M }
  And exit code is non-zero
  And tracing::error log carries "determinism.first_divergence" target

Scenario: --no-verify-checksums flag for diagnostic replay
  Given a bundle that has divergent checksums
  When cf-headless replay --no-verify-checksums runs
  Then the verifier still replays every command but skips checksum verification
  And result=ok even though checksums diverge
  And output indicates "checksum verification skipped"

Scenario: --scenario-path override
  Given a bundle stored in a non-default directory
  When cf-headless replay --scenario-path <path> runs
  Then the verifier resolves the scenario from the override path
  And relative-path fallback finds standard Corefall layouts

Scenario: Replay verifier safety — MAX_NO_ADVANCE_RETRIES guard
  Given a corrupt bundle where RunForTicks is ineffective (engine doesn't advance)
  When cf-headless replay runs
  Then after MAX_NO_ADVANCE_RETRIES (default 3) the verifier exits with a clear error
  And does NOT infinite-loop

Scenario: Replay verifier never writes side-effect bundles
  Given cf-headless replay running against any bundle
  When the replayed engine encounters RunForTicks with write_run_bundle=true
  Then the verifier overrides write_run_bundle=false (so no side-effect bundle is produced)
  (per CCCP / cf-headless AGENTS.md replay-safety contract)

Scenario: Replay throughput benchmark
  Given `cf-headless replay <bundle> --measure throughput`
  Then output includes: throughput_ticks_per_sec, wall_time_ms, peak_memory_mb
  And the benchmark runs without verifying checksums (orthogonal mode)
```

### Checksum cadence + determinism

```gherkin
Scenario: Per-scenario checksum cadence
  Given cf-app --checksum-cadence-ticks 30 --scenario micro_breach
  Then determinism.sim_checksum events fire every 30 ticks (instead of default 60)
  And run_manifest.json.checksum.cadence_ticks=30
  And cf-headless replay verifies at the same cadence

Scenario: Checksum scope sim_state_v1 (M3A canonical)
  Given the default checksum scope at M3A
  Then the scope name "sim_state_v1" is canonical
  And the bytes hashed are (in fixed order, big-endian where applicable):
    1. tick_counter (u64)
    2. rng_state_bytes (engine seeded RNG state)
    3. actor_state_quantized (per actor: id, pos_q16, vel_q16, hp_i16, status_u8, stance_u8, stability_q16, sharp_aim_q16, mass_q16, origin_id_u8)
    4. inventory_state (per actor: slots with item_id + state-quantized)
    5. terrain_chunk_grid (per dirty chunk: material_grid bytes)
    6. terrain_integrity_grid (per dirty chunk: integrity grid f32-quantized to u8) (M2.5)
    7. hazard_grid (per (chunk_id, local_pos) cell: kind_u8 + intensity_q16) (M2.5)
    8. affliction_state (per actor: sorted Vec<(kind_u8, severity_q16, expected_clear_tick_u64)>) (M2.5)
    9. armor_layer_state — per actor: per-zone armor_item_id + per-layer (kind_u8, hp_i16, hardness_q16, status_u8); per-zone occupant ("equipped armor" or "none") sorted (M2.5 NEW)
    10. internal_organ_state — per actor (humans/androids): sorted Vec<(organ_id_u8, hp_i16, condition_u8)>; per actor (robots): sorted Vec<(circuit_id_u8, hp_i16, condition_u8)> (M2.5 NEW)
    11. concussion_dose_state — per actor: (concussion_dose_q16, band_u8, internal_shock_dose_q16) where applicable per origin (M2.5 NEW)
    12. fluid_reservoir_state — per actor: sorted Vec<(fluid_kind_u8, current_q16, leak_rate_q16, leak_active_u8)> (M2.5 NEW)
    13. origin_state — per actor: (origin_id_u8, g_load_dose_q16, oxygen_supply_s_q16 [if applicable]) (M2.5 NEW)
    14. reactor_state (hp_i16, max_hp_i16, pressure_state_u8, heat_signature_k_q16) (M2.5)
    15. atmospherics_state (per atm_id: moles_per_gas_q16 + temperature_k_q16 + total_pressure_q16) (M2.5 placeholder; M5.9 fills)
    16. environment_signal_state (per actor: per-slice band enums) (M5.10 placeholder)
    17. mission_state (current_phase, timer_remaining_ticks, objective_states[])
    18. chassis_state (None at M3A; M5 fills with stance + module states + bound zones)
  And bumping the scope to "sim_state_v2" requires a migration shim
  (M2 adds terrain chunk; M3A adds actor + inventory + M2.5 deep damage firehose; M5 fills chassis_state)

Scenario: Cosmetic event types excluded from determinism scope
  Given the cosmetic-event list (per the determinism-island-contract)
  Then events with `cosmetic=true` are excluded from sim_state_v1 hashing
  And these event types are flagged cosmetic by default:
    - terrain.debris_spawned (visual particle)
    - hazard.tick (batched per-tick visualization)
    - affliction.tick (batched per-tick damage application)
    - ux.banner_raised / ux.banner_dismissed (UI visual)
    - shield.hit ripple (M5+ visual cosmetic; the shield.hit GAMEPLAY event is non-cosmetic)
    - render-2d particle, spark, dust events
  And the underlying STATE changes (terrain integrity, affliction severity, hazard intensity) ARE included in the checksum
  (rule: cosmetic events DESCRIBE the change; the state itself is hashed)

Scenario: Checksum differs across scenarios with same/different seed
  Given two runs of m1_actor_range with seed=1234
  When both complete
  Then their final_sim_checksum values match (determinism)
  Given two runs with seed=1234 and seed=5678
  Then their final_sim_checksum values differ (RNG advances change actor positions)

Scenario: 60Hz vs 120Hz determinism (per-rate)
  Given the same cfctl script
  When run at --tick-rate-hz 60 and 120
  Then both replays match their own bundle's checksums
  And the cross-rate match is NOT required at M3A (it lands as DR-052 CI gate at M9+)

Scenario: Determinism island contract document exists and is complete
  Given the project tree
  Then docs/plan/spec/determinism-island-contract.md exists
  And lists deterministic subsystems:
    - cf-sim-core (fixed-tick loop)
    - cf-actor (status, stability, sharp aim, control intent)
    - cf-physics (gravity, ground collision, recoil impulse, fall impact)
    - cf-equipment (rifle fire RNG, magazine pop, reload timer)
    - cf-terrain (carve, blast, dirty regions, material grid)
    - cf-mission (state machine, timer, objective transitions)
    - cf-ai (perception, miss_chance, recovery — uses engine seeded RNG)
  And lists non-deterministic (cosmetic-only) subsystems:
    - cf-render-2d (particle effects — flagged cosmetic=true)
    - cf-audio (cosmetic sound playback)
    - cf-app camera shake / hit-stop (visual juice; emit request events but rendering is cosmetic)
  And documents the cross-platform float rules per DR-052: f32 only in sim, RUSTFLAGS=-C target-feature=+sse2,+sse4.2, LLVM -ffast-math disabled
```

### Snapshot writer

```gherkin
Scenario: Snapshot cadence at scene start and every objective transition
  Given a scenario with 3 objectives
  When the player completes objective 1, fails objective 2, starts objective 3
  Then snapshot_actor + snapshot_inventory + snapshot_terrain_summary fire at:
    - Scene start (tick 0)
    - mission.objective_completed for #1
    - mission.objective_failed for #2
    - mission.objective_started for #3
  And every snapshot has parent_event_id pointing at the triggering event

Scenario: snapshot_actor payload contract
  Given a fired snapshot_actor event
  Then the payload contains: actor_id, tick, pos, velocity, aim, status, stance, hp, max_hp, stability, sharp_aim_progress, selected_slot, inventory_summary, mass, body_silhouette (placeholder=true at M3A; M5 fills with chassis-backed data)

Scenario: snapshot_inventory payload contract
  Given a fired snapshot_inventory event
  Then the payload contains: actor_id, tick, slots[] where each slot has { kind, weapon_id (if any), rifle_state { ammo_in_mag, mag_capacity, reloading } (if rifle) }

Scenario: snapshot_terrain_chunk payload contract
  Given a fired snapshot_terrain_chunk event (one per dirty chunk)
  Then the payload contains: chunk_id (x,y), version (last_modified_tick), bbox, checksum (blake3 of chunk material grid), compact_payload (RLE or hex of grid) OR diff_id (pointer to prior snapshot if smaller)

Scenario: snapshot_terrain_summary payload contract
  Given a fired snapshot_terrain_summary event
  Then the payload contains: tick, total_chunks, dirty_chunk_count, material_counts (BTreeMap<material_id, pixel_count>), total_carve_events, total_debris_spawned, integrity_distribution (BTreeMap<integrity_band, pixel_count> where band ∈ {Pristine, Scratched, Cracked, Critical, Destroyed}), hazard_tile_count, average_integrity

Scenario: snapshot_hazard_grid payload contract (M2.5)
  Given a fired snapshot_hazard_grid event
  Then the payload contains: tick, dirty_hazard_cell_count, hazard_cells: Vec<{ chunk_id, local_pos, kind, intensity, dissipation_rate, spawned_at_tick }>, summary_per_kind (BTreeMap<kind, count>)
  And the snapshot fires at scenario start and on every objective transition (same cadence as snapshot_terrain_*)

Scenario: snapshot_affliction payload contract (M2.5)
  Given a fired snapshot_affliction event
  Then the payload contains: tick, total_active_afflictions, by_actor: Vec<{ actor_id, afflictions: Vec<{ kind, severity, applied_at_tick, expected_clear_tick }> }>, by_kind (BTreeMap<kind, count>)

Scenario: snapshot_armor_layer payload contract (M2.5)
  Given a fired snapshot_armor_layer event
  Then the payload contains: tick, actors_with_layers: Vec<{ actor_id, layers: Vec<{ kind: External|Internal|Core, hp, max_hp, hardness, status: Pristine|Scratched|Cracked|Critical|Destroyed }> }>

Scenario: snapshot_atmospherics payload contract (M2.5 placeholder; M5.9 fills)
  Given a fired snapshot_atmospherics event
  Then the payload contains: tick, atm_ids: Vec<{ atm_id, kind: RoomCell|PipeNetwork|Suit|Canister|Lung|DeviceInternal, volume_l, moles_per_gas, temperature_k, total_pressure_pa, flags }>
  (At M2.5: empty for default scenarios; M5.9 fills with real atm states)

Scenario: snapshot_environment_signal payload contract (M5.10 placeholder)
  Given a fired snapshot_environment_signal event
  Then the payload contains: tick, by_actor: Vec<{ actor_id, slice_bands: BTreeMap<slice_name, band_enum> }>
  (M2.5: empty; M5.10 fills with real aggregator data)

Scenario: snapshot_armor payload contract (M2.5 deep damage)
  Given a fired snapshot_armor event
  Then the payload contains: tick, actors_with_armor: Vec<{
    actor_id,
    per_zone: Vec<{
      zone,                              // head | torso | arm_left | arm_right | forearm_left | forearm_right | hand_left | hand_right | leg_left | leg_right | shin_left | shin_right | foot_left | foot_right | backpack
      armor_item_id: Option<ItemId>,     // None = un-armored
      material_id,
      mass_kg,
      coverage_zones: Vec<BodyZone>,
      damage_resist: BTreeMap<DamageKind, f32>,
      absorption_factor,
      ap_resistance,
      chunkable,
      layers: Vec<{ kind: External|Internal|Core, hp, max_hp, hardness, condition: Pristine|Scratched|Cracked|Critical|Destroyed|ChunkedOff }>
    }>
  }>

Scenario: snapshot_internal payload contract (M2.5 deep damage)
  Given a fired snapshot_internal event
  Then the payload contains: tick, actors_with_internal: Vec<{
    actor_id,
    origin_id,                            // discriminator: humans/androids get organ list; robots get circuit list
    organs: Vec<{ organ_id, organ_kind, hp, max_hp, located_in_zone, condition, applied_afflictions }> (humans/androids only),
    circuits: Vec<{ circuit_id, circuit_kind, hp, max_hp, located_in_zone, condition, applied_afflictions }> (robots only)
  }>
  And the snapshot fires at scenario start + every objective transition (same cadence)

Scenario: snapshot_concussion payload contract (M2.5 deep damage)
  Given a fired snapshot_concussion event
  Then the payload contains: tick, by_actor: Vec<{
    actor_id, origin_id,
    concussion_dose, band: Clear|Mild|Moderate|Severe|KO_Imminent|KO, recovery_rate_per_s,
    internal_shock_dose (robots only),
    g_load_dose (humans + androids only)
  }>

Scenario: snapshot_fluid payload contract (M2.5 deep damage)
  Given a fired snapshot_fluid event
  Then the payload contains: tick, actors_with_fluids: Vec<{
    actor_id,
    reservoirs: Vec<{ fluid_kind: oil|coolant|fuel|electrolyte, current_l, capacity_l, leak_rate_per_s, leak_position, leak_active, ignition_risk_0_1 }>
  }>

Scenario: snapshot_origin payload contract (M2.5 deep damage)
  Given a fired snapshot_origin event
  Then the payload contains: tick, by_actor: Vec<{
    actor_id, origin_id: Human|Android|Robot|PoweredOrganic|Construct|HeavyBioMech,
    g_load_dose, oxygen_supply_s (humans + androids), helmet_seal_intact (humans + androids)
  }>

Scenario: Stable record_id layer (no raw pointers) — M2.5 firehose entities
  Given any event with an id field
  Then the id is a stable RecordId(u64) from the cf-replay registry
  And the registry emits lifecycle events when an entity is created / destroyed / pooled
  And raw MOID or pointer values are NEVER serialized
  (per CCCP MovableMan.cpp:126-143 warning about stale pointers after pooled allocation)
  And the registry tracks lifecycles for these entity kinds (M3A-locked taxonomy):
    - actor_id (Actor in MovableMan)
    - item_id (Item / weapon / equipment in inventory; INCLUDES armor items)
    - projectile_id (in-flight projectile)
    - chunk_id (terrain chunk)
    - hazard_cell_id (M2.5: hazard tile instance)
    - affliction_instance_id (M2.5: per (actor_id, kind, applied_at_tick) instance)
    - shield_instance_id (M5+: per actor shield)
    - armor_layer_id (M2.5: per (actor_id, zone, layer_kind) instance)
    - armor_item_id (M2.5: per (actor_id, zone) armor item slot)
    - armor_debris_record_id (M2.5: per chunked-off armor piece on the ground)
    - organ_id (M2.5: per (actor_id, organ_kind) organ instance for humans/androids)
    - circuit_id (M2.5: per (actor_id, circuit_kind) circuit instance for robots)
    - fluid_reservoir_id (M2.5: per (actor_id, fluid_kind) reservoir instance)
    - fluid_leak_id (M2.5: per active leak instance)
    - atm_id (M2.5 placeholder; M5.9 fills atm units)
    - environment_signal_id (M5.10 placeholder)
    - module_id (M5+: chassis module instance)
  And every entity_id has a `<kind>.entity_created` and `<kind>.entity_destroyed` event with parent_event_id linking to the cause

Scenario: High-density firehose backpressure handling
  Given a scenario with 50+ actors + dense terrain carving + 20+ hazard tiles + multiple afflictions per actor
  When per-tick event rate exceeds the recorder ring buffer capacity
  Then dropped_count surfaces are populated on the dropped event
  And summary.json.event_counts.dropped_total >= sum of all dropped_count fields
  And priority threshold ensures these are NEVER dropped (gameplay-critical):
    combat.*, mission.*, reactor.armor_layer_*, terrain.material_state_changed (band crossing), terrain.pixel_removed, hazard.spawned / .dissipated, affliction.applied / .cleared / .escalated, shield.depleted, atmos.breach_detected, determinism.sim_checksum, system.*, snapshot.*
  And these MAY be dropped under pressure (cosmetic):
    terrain.debris_spawned, hazard.tick, affliction.tick, ux.banner_raised (info), shield ripple cosmetic
  And the canonical checker (prototype_run_check.py) verifies the priority discipline (CRITICAL kinds never appear in dropped_count > 0 bundle without a `system.critical_drop` event explaining why)
```

### Recorder backpressure + reentrancy guard

```gherkin
Scenario: Recorder backpressure does not drop silently
  Given Recorder::with_capacity(100) (max 100 events per tick)
  When sim emits 150 events in one tick (combat burst)
  Then 50 events are dropped
  And recorder.dropped_count() returns 50
  And summary.json.event_counts.dropped_total ≥ 50
  And the per-event payload that triggered the overflow includes dropped_count=50 in the next emitted event
  And summary.json.recorder.peak_buffer_depth records the max queue depth reached

Scenario: Cosmetic events drop first under pressure
  Given a priority_threshold setting (cosmetic events have priority=0; gameplay events have priority=1)
  When the recorder is over capacity
  Then cosmetic events drop before gameplay events
  And no gameplay event is dropped while any cosmetic event remains in queue

Scenario: Reentrancy guard — recorder hooks emit inert data only
  Given a sim hook on collision (e.g. terrain_penetration)
  When the hook emits a `terrain.terrain_carved` event
  Then the event is appended as plain data (struct, no callbacks)
  And no subscriber is invoked synchronously from the recorder path
  And no sim state is mutated by the recorder
  (per CCCP Atom.cpp:96 collision/script reentrancy caveat)

Scenario: Recorder cannot block sim thread
  Given the recorder ring buffer is at 100% capacity
  When sim emits another event
  Then the sim thread does NOT block waiting for queue drain
  And the event is dropped (and counted) instead
  And tick wall-time is unaffected by recorder state
```

### Expected outcome + system events

```gherkin
Scenario: system.run_started carries protocol_version + manifest hash
  Given any cf-app run
  When the engine starts
  Then system.run_started fires at tick 0 with payload:
    { "protocol_version": <cf-control SCHEMA_VERSION>, "manifest_hash": "<blake3>", "build_id": "<git short>", "scenario_id": "...", "seed": N, "tick_rate_hz": 60 }

Scenario: system.run_finished carries outcome
  Given a run that completes cleanly
  Then system.run_finished fires once at the last tick with payload:
    { "outcome": "clean", "ticks_run": N, "wall_seconds": S, "final_sim_checksum": "<hex>" }
  Given a run that panics mid-sim
  Then system.run_finished fires with outcome="panic" before unwinding (best-effort)
  Given a run aborted via act.player.abort
  Then system.run_finished fires with outcome="abort"

Scenario: Canonical checker enforces expected_outcome matches actual outcome
  Given a bundle with run_manifest.expected_outcome=clean but system.run_finished.outcome=panic
  When prototype_run_check.py runs
  Then exit code is non-zero with structured error: { rule: "expected_outcome_mismatch", expected: "clean", actual: "panic" }
  Given a bundle with expected_outcome=panic and actual outcome=panic
  Then exit code is 0 (negative test bundles are valid)
  Given a bundle with expected_outcome=abort and actual outcome=abort
  Then exit code is 0

Scenario: runbundle.write rejects path traversal
  Given an active engine session
  When cfctl invokes runbundle.write { id_override: "../../../etc/passwd" }
  Then the engine rejects with reason="path_traversal_rejected"
  When cfctl invokes runbundle.write { id_override: "/absolute/path" }
  Then the engine rejects with reason="absolute_path_rejected"
  When cfctl invokes runbundle.write { id_override: "..\\windows\\system32" }
  Then the engine rejects with reason="path_traversal_rejected"
  And no file is written outside prototype_runs/native/

Scenario: protocol_version bump policy
  Given an additive cfctl method addition (e.g. M5 adds act.chassis.repair)
  Then protocol_version does NOT bump (additive surface is forward-compatible)
  Given a method's params shape changes (e.g. act.player.move adds a required field)
  Then protocol_version MUST bump
  And run_manifest captures the bump
```

### Parent-event-id cause chains (M3B forward-compat)

```gherkin
Scenario: Every causal event sets parent_event_id
  Given a 30-second run with mixed combat + terrain + AI events
  When the bundle is scanned
  Then every event in {weapon_fired, projectile_spawned, projectile_hit_mo, wound_added, actor_status_changed, inventory_dropped, terrain_carved, terrain_penetration_threshold, terrain_pixel_dislodged, ai.state_changed, ai.tactic_chosen, mission.objective_completed, mission.mission_resolved, snapshot_*} has parent_event_id
  And the parent_event_id references an earlier event in the same bundle
  And the cause chain for any leaf event walks back to an `input.intent_received` or `system.run_started` root within MAX_CHAIN_DEPTH (default 50)

Scenario: M3B viewer can walk the chain
  Given any bundle with parent_event_id chains
  When cf-tools-replay-viewer cause-chain --event-type actor_died runs (M3B integration)
  Then the chain resolves and prints the full causal sequence
  And handles terminations: RootReached / ParentMissingFromBundle / MaxDepthReached / CycleDetected
  (M3A emits the chain surface; M3B consumes it)

Scenario: unknown_cause marker for genuinely uncaused events
  Given an event where no causal predecessor exists (e.g. external interrupt)
  Then parent_event_id is None
  And payload includes "cause_origin": "unknown_cause" with a reason
  And the M3B viewer reports the chain terminated cleanly (not a missing-parent bug)
```

### Tooling + cross-platform forward-compat

```gherkin
Scenario: prototype_run_check.py 12 cross-file rules
  Given a malformed bundle (any one rule violation)
  When the checker runs
  Then exit code is non-zero with a structured error JSON
  And the error names the failed rule (one of: missing_file, schema_version_mismatch, run_id_mismatch, duplicate_event_id, non_monotonic_ticks, parent_event_missing, event_count_mismatch, category_count_mismatch, dropped_total_underflow, evidence_event_missing, missing_notes_heading, expected_outcome_mismatch)

Scenario: Notes.md required headings
  Given any bundle with notes.md
  Then notes.md contains all 6 required headings (## Assumptions Tested / ## Good / ## Bad / ## Meh / ## Evidence Links / ## Next Actions)
  When any is missing
  Then prototype_run_check.py exits non-zero with rule=missing_notes_heading + which_heading

Scenario: --headless-smoke produces a valid bundle (no window)
  Given cf-app --headless-smoke --scenario m1_actor_range --write-run-bundle
  When the run completes
  Then no Bevy window is opened
  And the run bundle validates with prototype_run_check.py (errors=0)
  And the bundle is suitable for CI smoke tests

Scenario: Cross-platform CI checksum match (DR-052 forward-compat)
  Given the same scenario + seed run on Linux x86_64, Windows x86_64, and macOS aarch64
  When all three bundles are produced
  Then their final_sim_checksum values match (cross-platform bit-determinism)
  (M3A produces the surface — float-determinism rules in determinism-island-contract.md; M9+ adds the CI matrix that verifies)
  (At M3A, this scenario is documented but not blocking — CI infra is M9 work)
```

## Out of scope

- Replay viewer GUI / scrubbing / cause-chain walker / debrief markdown — M3B (M3A ships the surface; M3B consumes)
- Replay branching (multiple paths from same checkpoint) — DR-002 future / BP4+
- Replay editing tools (replay-as-data) — DR-002 future / BP6+ (modding / sharing flow)
- GGPO-style rollback netcode — DR-052 / M12 PvP arena (M3A ships deterministic surface; rollback is a network adapter on top)
- Client prediction + reconciliation for online co-op — DR-052 / M11
- Lockstep input traces for LAN co-op — DR-052 / M10
- Per-platform CI checksum matrix on Linux/Windows/macOS aarch64 + x86 — DR-052 / M9+ CI infra (M3A documents the float-determinism rules + produces the surface; the CI matrix is a workflow file added at M9)
- `cfctl test sync-drift` / `cfctl test latency-injection` / `cfctl test rollback-burst` — DR-052 / M10+
- Atmospherics / material kernel / collision / mind / mmo / chassis / affliction event PRODUCERS — those land at their owning milestones (M5.5, M5.6, M5.9, M6.5, M5, M5.7, M12). M3A REGISTERS the categories in system.category_baseline so the schema is locked, but does not implement the producers.
- Network event replication (event broadcast across clients) — DR-005 / M9+
- `mind.*` event family (LLM agent layer) — M6.5
- Replay sharing UI / community browser — BP6+ post-launch
- Save-game model (cf-save mid-mission resume) — DR-029 / M5+ (separate from replay; replay is past-only, save is resume-point)
- T-CAPTURE pipeline implementation (cf-capture frame readback) — M3A ships the contract for `captures/` in run bundles; cf-capture itself lands at BP2 (already shipped before M3A)
- Replay format compression (zstd/lz4) — BP6+ optimization (M3A ships plain JSONL for inspectability; compression layers on later)
- Replay schema migration tooling — BP6+ (M3A locks v0.1; migrations land when v0.2 ships)
- Tournament-grade rollback patent licensing audit — DR-052 / M12 PvP (M3A is pre-network)
- Real-player playtest of "is the recap useful?" — OPTIONAL per AGENTS.md (AI Self-Test = headless replay of every prior milestone's bundles = primary gate)

## Dependencies

- **M0 engine bootstrap (closed)**: fixed-tick sim, run-bundle writer baseline, cfctl JSON-RPC, blake3 checksum infrastructure.
- **M1 actor controller (must be done OR closed)**: emits `input.intent_received`, `equipment.weapon_fired`, `actor.actor_status_changed`, `actor.inventory_dropped` events that M3A's verifier replays.
- **M1.5 micro breach (must be done)**: emits `mission.*` + `ai.*` events; provides the canonical multi-objective scenario for M3A's snapshot cadence test.
- **M2 chunked terrain (must be done)**: emits `terrain.terrain_carved` + `terrain.terrain_dirty_region_batch` + per-chunk checksum surface that M3A's verifier hashes into `sim_state_v1`.
- **M2.5 micro reactor defense (must be done)**: provides the multi-tick combat run-bundle used as the canonical M3A replay-determinism fixture (`m3a_replay_compare.cfctl.json` validates the M2.5 win bundle replays headlessly).

## Notes for the implementer

### Architecture rules

- **Recorder is on the sim thread; export is on a worker.** The sim thread MUST NOT block on disk I/O. Ring buffer + worker-thread export per CCCP demo recorder lesson (Soldat `Demo.pas` records to disk but the sim doesn't wait).
- **Recorder hooks emit inert data only.** No subscriber callbacks, no sim mutation, no script invocation from the recorder path. Per CCCP `Atom.cpp:96-99`: `OnCollideWithTerrain` Lua can run while AtomGroup is travelling — recorder hooks MUST stay non-reentrant.
- **Stable record_id, not raw pointers.** Per CCCP `MovableMan.cpp:126-143`: `GetMOFromID` can return stale pointers because pooled memory re-allocates at old addresses. cf-replay's `RecordId(u64)` registry is the canonical id layer. Lifecycle events (`actor.id_assigned`, `actor.id_retired`) fire when entities are created/destroyed.
- **Cosmetic flag is the determinism-island opt-out.** Per DR-052: events with `cosmetic: true` are excluded from `determinism.sim_checksum`. Use it for particle effects, audio cues, camera shake. Do NOT use it for any event a player/AI/replay-grader can read.
- **`system.category_baseline` is a declaration, not a producer claim.** Categories with no events yet still appear with `status: "registered"` and `ladder_at: "<milestone>"`. The schema is locked; producers ladder up.

### Event taxonomy at M3A (status snapshot)

| Status | Categories |
|---|---|
| **active** (M0..M2.5 produce events) | input, control, combat, body, terrain, ai, mission, system, snapshot, determinism, ux, accessibility, performance, equipment, actor |
| **registered** (later milestones produce; M3A locks the schema) | mind (M6.5), collision (M5.5), server (M9), anti_cheat (M9), mmo (M12), material (M5.6), reaction (M5.6), atmospherics (M5.9), affliction (M5.7), logistics (M7), chassis (M5), ability (M5+) |

Don't skip the registered categories from the baseline event — they're how the schema stays additive-only. Adding a new producer at M5.6 doesn't require a baseline-schema bump; it just flips status from registered → active.

### Checksum scope evolution

`sim_state_v1` (M0 ships, M3A formalizes):

| Milestone | Bytes added to checksum |
|---|---|
| M0 | `tick_counter || rng_state_bytes` |
| M1 | `+ actor_state_quantized (HP, position, velocity, aim, status, stability)` |
| M2 | `+ chunk_grid_checksums (per dirty chunk: blake3 of material_grid)` |
| M3A | `+ inventory_state (slot id + ammo_in_mag + reloading flag) + mission_state (status + current_objective + timer)` |
| M5 | `+ chassis_state (per-zone HP, module states, pilot state, eject ticks)` — bumps scope to `sim_state_v2` with migration shim |

Each scope name is canonical; bumping requires a migration. `_v1` accepts the M0..M3A layered additions because they're additive (extending the byte stream doesn't change the algorithm, just the input).

### Snapshot cadence (per replay-recorder-slice-a § Snapshot Cadence)

| Snapshot | Cadence |
|---|---|
| `snapshot_actor` | Scene start + every 250ms (~15 ticks at 60Hz) + on status/death transitions + on every objective transition |
| `snapshot_inventory` | Scene start + on actor.inventory_dropped + on every objective transition |
| `snapshot_terrain_chunk` | On chunk dirty-rect coalesce (at most every 500ms per chunk) + on every objective transition |
| `snapshot_terrain_summary` | Scene start + every 1 second + on every objective transition |
| `snapshot_chassis` | Scene start + on chassis.stage_changed + on every objective transition (M5+ when chassis is attached) |

### Replay envelope contract (v0.1 LOCKED at M0; M3A enforces)

The envelope CANNOT change shape at M3A. Additive extensions to PAYLOAD are fine; envelope field additions require a v0.2 bump + migration. M5.5+ collision / M5.6 material kernels will ship payload-extensions but the envelope stays v0.1.

### cf-headless replay safety contract

Per cf-headless (already-implemented) and CCCP/cf-headless AGENTS.md:

- Verifier dispatches `ControlCommand` through the SAME `M0Engine::dispatch` path the live engine uses (no parallel "replay engine"). This is the "no parallel production paths" rule.
- Verifier hard-codes `write_run_bundle: false` on replayed RunForTicks (so it never writes side-effect bundles).
- `MAX_NO_ADVANCE_RETRIES=3` guards against permanently-stalled engine.
- Settings patches replay as `SettingsPatch::default()` (no-op) because the recorded command_accepted payload deliberately doesn't carry the patch contents (avoids leaking accessibility flags into audit log). Settings don't affect checksum so this is safe.
- `verify_checksums: bool` exposed as `--no-verify-checksums` (default false = verify) per clap v4 idiomatic negation.
- Final-kind checksums (emitted by `record_run_finished` / `write_run_bundle`) are intentionally skipped — they fire outside the replay loop.
- Path resolution: relative-path fallback resolves common Corefall layouts.

### Comparable references

- **OpenSoldat `Demo.pas`** (vault: `comparables/opensoldat-local-audit.md`): records message records during sim; replay reads back. Lesson: replay infrastructure belongs near sim, not patched on later. We follow this by integrating cf-replay with cf-control/cf-app from M0.
- **OpenLieroX NewNet** (`comparables/openlierox-local-audit.md:NewNetEngine.cpp:47`): had save/restore/checksum/rollback intentions but RestoreState was marked outdated. **Don't ship aspirational rollback at M3A** — DR-052 directs it to M12 PvP.
- **Noita** falling-sand replay: deterministic per-tick input replay (no networking; single-player). Per `comparables/noita-grade-material-simulation-research.md`. Our M3A is equivalent for solo runs.
- **Teardown** networked destruction: deterministic destruction commands + state sync (not raw voxel dumps). Per the Noita research § Teardown. Our replay events are semantic commands (`terrain.terrain_carved` with mask_id), not raw pixel dumps — matches this pattern.
- **CCCP** `Demo.cpp` / `ActivityMan::Save`: save/replay is tightly coupled to activity state. Lesson: snapshot writer must include mission state, not just actor state.

### Decision-record alignment

- **DR-002 (Replay/Event / closed at M3B)**: M3A IS the closure milestone for hybrid event-log + snapshots. Lock the envelope at v0.1. M3B builds the viewer.
- **DR-005 (Multiplayer Posture / OPEN)**: M3A produces the deterministic surface that DR-005's lockstep + prediction + reconciliation will consume at M10+. Replay envelope size feeds the bandwidth estimate.
- **DR-018 (Death Meaning / closed)**: M3A emits the cause-chain (status → wound → projectile → fire → input). M3B walks it for death recap.
- **DR-024 (Native Engine Stack / closed)**: cf-replay is a custom crate per the "custom for hot paths" rule.
- **DR-052 (Network Sync + CLI-Testable Determinism / closed)**: M3A ships the surface for `cfctl test replay-determinism` + `cfctl test replay-bit-identical` + cross-platform float rules. CI matrix lands at M9+.

### Existing W1.2 work to credit during audit

The parallel agent's W1.2 commits already landed many M3A items. Audit must mark these `PASS (already in)`:

- `Recorder::with_capacity(N)` + `dropped_count()` + `event_count()` (item #94, #1989-1992)
- `system.category_baseline` event emitting all 27 categories (item #91, #896)
- `snapshot_terrain_chunk` + `snapshot_terrain_summary` with material_counts BTreeMap (item #92, #768)
- `snapshot_inventory` carries `rifle_state` (ammo_in_mag, mag_capacity, reloading) (item #767)
- `emit_initial_snapshots` re-fires on objective state changes (item #770, #92)
- `M0EngineConfig.checksum_cadence_ticks` + `ConfigInputs.checksum_cadence_ticks` (item #97)
- `system.run_started.protocol_version` (matches cf-control SCHEMA_VERSION) (item #91)
- `runbundle.write` rejects path traversal (`../`, `/`, `\`) (item #787 from M1.2)
- `cf-headless replay <bundle>` structured `first_divergence` + `all_divergences` array (item #95, #96)
- `cf-headless` MAX_NO_ADVANCE_RETRIES safety guard (existing)
- `--no-verify-checksums` + `--scenario-path` flags (existing)
- `prototype_run_check.py` validates `expected_outcome` against `system.run_finished.outcome` (item #769)
- `docs/plan/spec/determinism-island-contract.md` exists (item #93)
- `summary.json.event_counts.by_category` populated (item #896)
- `--checksum-cadence-ticks <N>` CLI flag wired (item #895)

The audit should mark these as STILL FAILING:

- Cross-platform CI checksum matrix (item #98 — needs M9+ CI infra; M3A documents the rules)
- `expected_outcome=panic` / `=abort` test bundles that prove checker REJECTS mismatches (item #1386, #1951, #942 — adversarial proof needed)
- 5-minute LITERAL run bundle (not 60s) + headless replay match (item #1290 / #1291 from M1)
- `cf-replay::EventRecorder` ring buffer size configurable per scenario (item #1991)
- Event-priority field for cosmetic-first drop ordering (item #1992)
- `snapshot_chassis` for M5 forward-compat (placeholder OK at M3A)

### Pitfalls / things that have bitten us before

- **Recorder blocks sim thread**: NEVER. Ring buffer overflow drops events; sim continues. Per AGENTS.md performance contract.
- **Raw pointer / MOID as event id**: CCCP bug. Always use `RecordId(u64)` from cf-replay registry.
- **Subscriber callbacks from collision/script hooks**: reentrancy bug. Recorder hooks emit inert data only.
- **Floating-point determinism breaks across platforms**: f64 in sim (use f32), -ffast-math enabled (disable in sim crates), x87 fpu mode (force SSE2/SSE4.2). Per DR-052.
- **Cosmetic events leak into checksum**: replay diverges visually but reports match. Use `cosmetic: true` flag and exclude from sim_state_v1.
- **expected_outcome mismatch not caught**: prototype_run_check.py must REJECT bundles where manifest.expected_outcome ≠ system.run_finished.outcome. Adversarial test bundles required.
- **5-minute claim using 60s loops**: literal 18000-tick run required. The run-bundle checker's math is real.
- **Replay verifier writes side-effect bundles**: cf-headless MUST hard-code write_run_bundle=false on replayed RunForTicks. Else CI is full of phantom bundles.
- **schema_version bump without migration shim**: replay breaks. Lock v0.1 at M3A; v0.2 requires migration tooling that's BP6+ scope.
- **No emoji in any HUD / replay text / events** (project rule).
