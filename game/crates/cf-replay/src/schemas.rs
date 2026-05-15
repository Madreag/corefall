//! **M1 Gap H**: per-event JSON schema validation for the prototype-recorder
//! event payloads.
//!
//! The full schemas live under `cf-replay/schemas/event/` (one JSON file per
//! `(category, event_type)` pair). The validator here is intentionally a
//! minimal "required field present + type matches" check rather than a
//! full draft-2020-12 implementation: pulling in a JSON Schema crate just
//! to assert payload shapes would balloon the dependency surface for a
//! benefit M1 doesn't need. The validator handles:
//!
//! - `required` array (every listed field MUST exist in the payload).
//! - per-field `type` (`object`, `array`, `string`, `number`, `integer`,
//!   `boolean`; arrays of types are interpreted as a union).
//! - `minItems` + `maxItems` on arrays.
//! - `minimum` on numeric values.
//! - `enum` on strings.
//!
//! `additionalProperties: true` is implicit — payloads may carry extra
//! fields beyond the schema without rejection (the recorder envelope is
//! intentionally extensible).
//!
//! `cf-mod validate-bundle` calls `validate_event_payload` on every event
//! in a run bundle; the workspace test under `cf-replay/tests` walks a
//! freshly-recorded smoke bundle to prove the schemas accept real events.

use serde::Deserialize;
use serde_json::Value;

const SCHEMA_INPUT_INTENT_RECEIVED: &str = include_str!("../schemas/event/input_intent_received.json");
const SCHEMA_WEAPON_FIRED: &str = include_str!("../schemas/event/weapon_fired.json");
const SCHEMA_PROJECTILE_SPAWNED: &str = include_str!("../schemas/event/projectile_spawned.json");
const SCHEMA_WOUND_ADDED: &str = include_str!("../schemas/event/wound_added.json");
const SCHEMA_INVENTORY_DROPPED: &str = include_str!("../schemas/event/inventory_dropped.json");
const SCHEMA_ALARM_REGISTERED: &str = include_str!("../schemas/event/alarm_registered.json");
const SCHEMA_TERRAIN_CARVED: &str = include_str!("../schemas/event/terrain_carved.json");
const SCHEMA_TERRAIN_PENETRATION_THRESHOLD: &str = include_str!("../schemas/event/terrain_penetration_threshold.json");
const SCHEMA_TERRAIN_DIRTY_REGION_BATCH: &str = include_str!("../schemas/event/terrain_dirty_region_batch.json");
// M8A: semantic terrain event protocol (chunk_mutated + chunk_active_region_changed).
const SCHEMA_TERRAIN_CHUNK_MUTATED: &str = include_str!("../schemas/event/terrain_chunk_mutated.json");
const SCHEMA_TERRAIN_CHUNK_ACTIVE_REGION_CHANGED: &str =
    include_str!("../schemas/event/terrain_chunk_active_region_changed.json");
// M8A: perf_sample (cosmetic) — per-cadence per-subsystem latency samples.
const SCHEMA_PERF_SAMPLE: &str = include_str!("../schemas/event/perf_sample.json");
const SCHEMA_TERRAIN_PIXEL_DISLODGED: &str = include_str!("../schemas/event/terrain_pixel_dislodged.json");
const SCHEMA_HAZARD_CONTACT_OR_AVOIDANCE: &str = include_str!("../schemas/event/hazard_contact_or_avoidance.json");
const SCHEMA_ANCHOR_MATERIAL_RESULT: &str = include_str!("../schemas/event/anchor_material_result.json");
const SCHEMA_TERRAIN_MATERIAL_PROBE: &str = include_str!("../schemas/event/terrain_material_probe.json");
const SCHEMA_TERRAIN_FILL_OR_REPAIR: &str = include_str!("../schemas/event/terrain_fill_or_repair.json");
const SCHEMA_FORCED_REFRESH_REQUESTED: &str = include_str!("../schemas/event/forced_refresh_requested.json");
// M3 audit pass 7 (2026-05-13): schemas for terrain.* events that were
// previously recorded without a schema.
const SCHEMA_TERRAIN_DEBRIS_CAPPED: &str = include_str!("../schemas/event/debris_capped.json");
const SCHEMA_TERRAIN_TOOL_REFUSED: &str = include_str!("../schemas/event/tool_refused.json");
const SCHEMA_TERRAIN_TOOL_ACTION_STARTED: &str = include_str!("../schemas/event/tool_action_started.json");
const SCHEMA_EQUIPMENT_TOOL_ACTION_COMPLETED: &str = include_str!("../schemas/event/tool_action_completed.json");
const SCHEMA_TERRAIN_PATH_INVALIDATED: &str = include_str!("../schemas/event/path_invalidated.json");
// M2 re-audit (2026-05-13): mission + AI event schemas the spec lists in
// "## Files" / "## Crates / modules touched" but were never created.
const SCHEMA_MISSION_STARTED: &str = include_str!("../schemas/event/mission_started.json");
const SCHEMA_OBJECTIVE_STARTED: &str = include_str!("../schemas/event/objective_started.json");
const SCHEMA_OBJECTIVE_UPDATED: &str = include_str!("../schemas/event/objective_updated.json");
const SCHEMA_OBJECTIVE_COMPLETED: &str = include_str!("../schemas/event/objective_completed.json");
const SCHEMA_OBJECTIVE_FAILED: &str = include_str!("../schemas/event/objective_failed.json");
const SCHEMA_MISSION_RESOLVED: &str = include_str!("../schemas/event/mission_resolved.json");
const SCHEMA_AI_STATE_CHANGED: &str = include_str!("../schemas/event/ai_state_changed.json");
const SCHEMA_AI_PERCEPTION_SIGNAL: &str = include_str!("../schemas/event/ai_perception_signal.json");
const SCHEMA_AI_TACTIC_CHOSEN: &str = include_str!("../schemas/event/ai_tactic_chosen.json");
const SCHEMA_AI_MISSED_SHOT_REASON: &str = include_str!("../schemas/event/ai_missed_shot_reason.json");
const SCHEMA_AI_STUCK_STATE_CHANGED: &str = include_str!("../schemas/event/ai_stuck_state_changed.json");
const SCHEMA_AI_RECOVERY_ACTION: &str = include_str!("../schemas/event/ai_recovery_action.json");
// M4 (2026-05-13): system/determinism/snapshot event schemas locked at v0.1.
const SCHEMA_SYSTEM_RUN_STARTED: &str = include_str!("../schemas/event/system_run_started.json");
const SCHEMA_SYSTEM_RUN_FINISHED: &str = include_str!("../schemas/event/system_run_finished.json");
const SCHEMA_SYSTEM_CATEGORY_BASELINE: &str = include_str!("../schemas/event/system_category_baseline.json");
const SCHEMA_DETERMINISM_SIM_CHECKSUM: &str = include_str!("../schemas/event/determinism_sim_checksum.json");
const SCHEMA_DETERMINISM_FIRST_DIVERGENCE: &str = include_str!("../schemas/event/determinism_first_divergence.json");
const SCHEMA_SNAPSHOT_ACTOR: &str = include_str!("../schemas/event/snapshot_actor.json");
const SCHEMA_SNAPSHOT_INVENTORY: &str = include_str!("../schemas/event/snapshot_inventory.json");
const SCHEMA_SNAPSHOT_TERRAIN_CHUNK: &str = include_str!("../schemas/event/snapshot_terrain_chunk.json");
const SCHEMA_SNAPSHOT_TERRAIN_SUMMARY: &str = include_str!("../schemas/event/snapshot_terrain_summary.json");
const SCHEMA_SNAPSHOT_CHASSIS: &str = include_str!("../schemas/event/snapshot_chassis.json");
// M4 § M9 firehose surface — placeholder snapshot schemas locked at v0.1 so
// M9/M17/M19/M20 producers can ladder up without renaming. M4 emits
// placeholder payloads (`placeholder: true`); later milestones fill the
// optional arrays with real data and clear the flag.
const SCHEMA_SNAPSHOT_HAZARD_GRID: &str = include_str!("../schemas/event/snapshot_hazard_grid.json");
const SCHEMA_SNAPSHOT_AFFLICTION: &str = include_str!("../schemas/event/snapshot_affliction.json");
const SCHEMA_SNAPSHOT_ARMOR_LAYER: &str = include_str!("../schemas/event/snapshot_armor_layer.json");
const SCHEMA_SNAPSHOT_ATMOSPHERICS: &str = include_str!("../schemas/event/snapshot_atmospherics.json");
const SCHEMA_SNAPSHOT_ENVIRONMENT_SIGNAL: &str = include_str!("../schemas/event/snapshot_environment_signal.json");
const SCHEMA_SNAPSHOT_ARMOR: &str = include_str!("../schemas/event/snapshot_armor.json");
const SCHEMA_SNAPSHOT_INTERNAL: &str = include_str!("../schemas/event/snapshot_internal.json");
const SCHEMA_SNAPSHOT_CONCUSSION: &str = include_str!("../schemas/event/snapshot_concussion.json");
const SCHEMA_SNAPSHOT_FLUID: &str = include_str!("../schemas/event/snapshot_fluid.json");
const SCHEMA_SNAPSHOT_ORIGIN: &str = include_str!("../schemas/event/snapshot_origin.json");
// M5-A1 (2026-05-13): snapshot.shield mirrors the ShieldState struct for the
// M9 firehose. Parallels snapshot_atmospherics + snapshot_environment_signal.
const SCHEMA_SNAPSHOT_SHIELD: &str = include_str!("../schemas/event/snapshot_shield.json");
// M5-A2 (2026-05-13): snapshot.thermal mirrors the M16/M19 thermal kernel
// state for the M9 firehose. Parallels snapshot_atmospherics +
// snapshot_environment_signal + snapshot_shield.
const SCHEMA_SNAPSHOT_THERMAL: &str = include_str!("../schemas/event/snapshot_thermal.json");
// M5 (2026-05-13): deep-damage event-surface lock. Each schema declares the
// M4 v0.1 envelope shape with `schema_version: "0.1"` const, `category` const,
// `event_type` const, and the payload nested under properties.payload.
// Producers ladder up at M13/M14/M15/M16/M17/M19/M20.
// armor.* (M13 + M14).
const SCHEMA_ARMOR_LAYER_HP_CHANGED: &str = include_str!("../schemas/event/armor_layer_hp_changed.json");
const SCHEMA_ARMOR_LAYER_CRITICAL: &str = include_str!("../schemas/event/armor_layer_critical.json");
const SCHEMA_ARMOR_LAYER_DESTROYED: &str = include_str!("../schemas/event/armor_layer_destroyed.json");
const SCHEMA_ARMOR_ALL_LAYERS_DESTROYED: &str = include_str!("../schemas/event/armor_all_layers_destroyed.json");
const SCHEMA_ARMOR_CHUNKED_OFF: &str = include_str!("../schemas/event/armor_chunked_off.json");
const SCHEMA_ARMOR_DEBRIS_SPAWNED: &str = include_str!("../schemas/event/armor_debris_spawned.json");
const SCHEMA_ARMOR_REPAIRED: &str = include_str!("../schemas/event/armor_repaired.json");
const SCHEMA_ARMOR_ANGLE_DEFLECTION_CALCULATED: &str =
    include_str!("../schemas/event/armor_angle_deflection_calculated.json");
const SCHEMA_ARMOR_RICOCHET: &str = include_str!("../schemas/event/armor_ricochet.json");
const SCHEMA_ARMOR_SPALLING: &str = include_str!("../schemas/event/armor_spalling.json");
const SCHEMA_ARMOR_PENETRATION_RAY_TRAVERSED: &str =
    include_str!("../schemas/event/armor_penetration_ray_traversed.json");
const SCHEMA_ARMOR_HE_OVERPRESSURE_WAVE: &str = include_str!("../schemas/event/armor_he_overpressure_wave.json");
const SCHEMA_ARMOR_HEAT_JET_PENETRATED: &str = include_str!("../schemas/event/armor_heat_jet_penetrated.json");
const SCHEMA_ARMOR_HEAT_JET_PRE_DETONATED_BY_ERA: &str =
    include_str!("../schemas/event/armor_heat_jet_pre_detonated_by_era.json");
const SCHEMA_ARMOR_APFSDS_PENETRATED: &str = include_str!("../schemas/event/armor_apfsds_penetrated.json");
const SCHEMA_ARMOR_ERA_PANEL_DETONATED: &str = include_str!("../schemas/event/armor_era_panel_detonated.json");
const SCHEMA_ARMOR_SCHURZEN_PRE_DETONATED: &str = include_str!("../schemas/event/armor_schurzen_pre_detonated.json");
const SCHEMA_ARMOR_MULTI_HIT_DEGRADATION: &str = include_str!("../schemas/event/armor_multi_hit_degradation.json");
const SCHEMA_ARMOR_REACTIVE_ARMOR_CONSUMED: &str = include_str!("../schemas/event/armor_reactive_armor_consumed.json");
// internal.* (M14 + M17).
const SCHEMA_INTERNAL_ORGAN_DAMAGED: &str = include_str!("../schemas/event/internal_organ_damaged.json");
const SCHEMA_INTERNAL_ORGAN_DESTROYED: &str = include_str!("../schemas/event/internal_organ_destroyed.json");
const SCHEMA_INTERNAL_ORGAN_FAILURE_CASCADE: &str =
    include_str!("../schemas/event/internal_organ_failure_cascade.json");
const SCHEMA_INTERNAL_CIRCUIT_DAMAGED: &str = include_str!("../schemas/event/internal_circuit_damaged.json");
const SCHEMA_INTERNAL_CIRCUIT_DESTROYED: &str = include_str!("../schemas/event/internal_circuit_destroyed.json");
const SCHEMA_INTERNAL_CIRCUIT_FAILURE_CASCADE: &str =
    include_str!("../schemas/event/internal_circuit_failure_cascade.json");
// concussion.* + internal_shock.* (M17).
const SCHEMA_CONCUSSION_DOSE_CHANGED: &str = include_str!("../schemas/event/concussion_dose_changed.json");
const SCHEMA_CONCUSSION_BAND_CHANGED: &str = include_str!("../schemas/event/concussion_band_changed.json");
const SCHEMA_CONCUSSION_KO_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/concussion_ko_threshold_crossed.json");
const SCHEMA_CONCUSSION_RECOVERED: &str = include_str!("../schemas/event/concussion_recovered.json");
const SCHEMA_INTERNAL_SHOCK_DOSE_CHANGED: &str = include_str!("../schemas/event/internal_shock_dose_changed.json");
const SCHEMA_INTERNAL_SHOCK_MODULE_DAMAGED: &str = include_str!("../schemas/event/internal_shock_module_damaged.json");
// fluid.* (M13 + M14).
const SCHEMA_FLUID_LEAK_STARTED: &str = include_str!("../schemas/event/fluid_leak_started.json");
const SCHEMA_FLUID_LEAK_RATE_CHANGED: &str = include_str!("../schemas/event/fluid_leak_rate_changed.json");
const SCHEMA_FLUID_RESERVOIR_WARNING: &str = include_str!("../schemas/event/fluid_reservoir_warning.json");
const SCHEMA_FLUID_RESERVOIR_CRITICAL: &str = include_str!("../schemas/event/fluid_reservoir_critical.json");
const SCHEMA_FLUID_RESERVOIR_EMPTY: &str = include_str!("../schemas/event/fluid_reservoir_empty.json");
const SCHEMA_FLUID_IGNITION: &str = include_str!("../schemas/event/fluid_ignition.json");
const SCHEMA_FLUID_GROUND_SPLATTER_SPAWNED: &str = include_str!("../schemas/event/fluid_ground_splatter_spawned.json");
const SCHEMA_FLUID_LEAK_STOPPED: &str = include_str!("../schemas/event/fluid_leak_stopped.json");
const SCHEMA_FLUID_REFILLED: &str = include_str!("../schemas/event/fluid_refilled.json");
// origin.* (M17).
const SCHEMA_ORIGIN_SHOT_FORCE_FEEDBACK: &str = include_str!("../schemas/event/origin_shot_force_feedback.json");
const SCHEMA_ORIGIN_G_LOAD_DOSE_CHANGED: &str = include_str!("../schemas/event/origin_g_load_dose_changed.json");
const SCHEMA_ORIGIN_HELMET_BREACH: &str = include_str!("../schemas/event/origin_helmet_breach.json");
const SCHEMA_ORIGIN_OXYGEN_SUPPLY_CHANGED: &str = include_str!("../schemas/event/origin_oxygen_supply_changed.json");
// hazard.* (M16).
const SCHEMA_HAZARD_SPAWNED: &str = include_str!("../schemas/event/hazard_spawned.json");
const SCHEMA_HAZARD_SPREAD: &str = include_str!("../schemas/event/hazard_spread.json");
const SCHEMA_HAZARD_ACTOR_CONTACT: &str = include_str!("../schemas/event/hazard_actor_contact.json");
const SCHEMA_HAZARD_TICK: &str = include_str!("../schemas/event/hazard_tick.json");
const SCHEMA_HAZARD_DISSIPATED: &str = include_str!("../schemas/event/hazard_dissipated.json");
// affliction.* (M16).
const SCHEMA_AFFLICTION_APPLIED: &str = include_str!("../schemas/event/affliction_applied.json");
const SCHEMA_AFFLICTION_TICK: &str = include_str!("../schemas/event/affliction_tick.json");
const SCHEMA_AFFLICTION_CLEARED: &str = include_str!("../schemas/event/affliction_cleared.json");
const SCHEMA_AFFLICTION_ESCALATED: &str = include_str!("../schemas/event/affliction_escalated.json");
// atmos.* (M19).
const SCHEMA_ATMOS_PRESSURE_CHANGED: &str = include_str!("../schemas/event/atmos_pressure_changed.json");
const SCHEMA_ATMOS_TEMPERATURE_CHANGED: &str = include_str!("../schemas/event/atmos_temperature_changed.json");
const SCHEMA_ATMOS_GAS_RELEASED: &str = include_str!("../schemas/event/atmos_gas_released.json");
const SCHEMA_ATMOS_BREACH_DETECTED: &str = include_str!("../schemas/event/atmos_breach_detected.json");
const SCHEMA_ATMOS_COMBUSTION_IGNITION: &str = include_str!("../schemas/event/atmos_combustion_ignition.json");
const SCHEMA_ATMOS_PHASE_TRANSITION: &str = include_str!("../schemas/event/atmos_phase_transition.json");
const SCHEMA_ATMOS_PIPE_FLOW: &str = include_str!("../schemas/event/atmos_pipe_flow.json");
const SCHEMA_ATMOS_PIPE_FREEZE: &str = include_str!("../schemas/event/atmos_pipe_freeze.json");
const SCHEMA_ATMOS_PIPE_RUPTURE: &str = include_str!("../schemas/event/atmos_pipe_rupture.json");
const SCHEMA_ATMOS_ELECTROLYSIS_STARTED: &str = include_str!("../schemas/event/atmos_electrolysis_started.json");
// shield.* (M13+ + M25+).
const SCHEMA_SHIELD_HIT: &str = include_str!("../schemas/event/shield_hit.json");
const SCHEMA_SHIELD_DEPLETED: &str = include_str!("../schemas/event/shield_depleted.json");
const SCHEMA_SHIELD_REGEN_STARTED: &str = include_str!("../schemas/event/shield_regen_started.json");
const SCHEMA_SHIELD_REGEN_COMPLETED: &str = include_str!("../schemas/event/shield_regen_completed.json");
const SCHEMA_SHIELD_DISRUPTED: &str = include_str!("../schemas/event/shield_disrupted.json");
// environment.* (M20).
const SCHEMA_ENVIRONMENT_SIGNAL_DELTA: &str = include_str!("../schemas/event/environment_signal_delta.json");
const SCHEMA_ENVIRONMENT_SIGNAL_AGGREGATED: &str = include_str!("../schemas/event/environment_signal_aggregated.json");
// thermal.* (M16 + M19).
const SCHEMA_THERMAL_SIGNATURE_CHANGED: &str = include_str!("../schemas/event/thermal_signature_changed.json");
const SCHEMA_THERMAL_HEAT_EXCHANGED: &str = include_str!("../schemas/event/thermal_heat_exchanged.json");
const SCHEMA_THERMAL_MATERIAL_PHASE_CHANGE: &str = include_str!("../schemas/event/thermal_material_phase_change.json");
// combat.projectile_hit_mo expanded payload (M13 + M14).
const SCHEMA_COMBAT_PROJECTILE_HIT_MO: &str = include_str!("../schemas/event/combat_projectile_hit_mo.json");
// audio.event_requested (M5 mandate — M13.x cf-audio consumes).
const SCHEMA_AUDIO_EVENT_REQUESTED: &str = include_str!("../schemas/event/audio_event_requested.json");
// M5-A2: combat.melee_hit_mo + combat.explosive_hit_mo siblings of
// projectile_hit_mo for M6 melee + grenade hits.
const SCHEMA_COMBAT_MELEE_HIT_MO: &str = include_str!("../schemas/event/combat_melee_hit_mo.json");
const SCHEMA_COMBAT_EXPLOSIVE_HIT_MO: &str = include_str!("../schemas/event/combat_explosive_hit_mo.json");
// M6 actor / combat / equipment / inventory / perception / squad event
// schema registrations. The 41 schema files were authored alongside the M6
// dispatch surface but never wired into the validator registry — without
// these include_str! lines + match arms the M5-locked envelope validator
// silently bypasses every M6 event payload.
const SCHEMA_ACTOR_ACTION_REJECTED: &str = include_str!("../schemas/event/actor_action_rejected.json");
const SCHEMA_ACTOR_CLIMB_STARTED: &str = include_str!("../schemas/event/actor_climb_started.json");
const SCHEMA_ACTOR_DIVE_STARTED: &str = include_str!("../schemas/event/actor_dive_started.json");
const SCHEMA_ACTOR_FACING_CHANGED: &str = include_str!("../schemas/event/actor_facing_changed.json");
const SCHEMA_ACTOR_LEAN_CHANGED: &str = include_str!("../schemas/event/actor_lean_changed.json");
const SCHEMA_ACTOR_SLIDE_STARTED: &str = include_str!("../schemas/event/actor_slide_started.json");
const SCHEMA_ACTOR_STAMINA_CHANGED: &str = include_str!("../schemas/event/actor_stamina_changed.json");
const SCHEMA_ACTOR_STANCE_CHANGED: &str = include_str!("../schemas/event/actor_stance_changed.json");
const SCHEMA_ACTOR_VAULT_STARTED: &str = include_str!("../schemas/event/actor_vault_started.json");
const SCHEMA_COMBAT_KNIFE_THROW_LANDED: &str = include_str!("../schemas/event/combat_knife_throw_landed.json");
const SCHEMA_COMBAT_KNIFE_THROW_STARTED: &str = include_str!("../schemas/event/combat_knife_throw_started.json");
const SCHEMA_COMBAT_STEALTH_KILL_EXECUTED: &str = include_str!("../schemas/event/combat_stealth_kill_executed.json");
const SCHEMA_EQUIPMENT_BEACON_DROPPED: &str = include_str!("../schemas/event/equipment_beacon_dropped.json");
const SCHEMA_EQUIPMENT_BIPOD_DEPLOYED: &str = include_str!("../schemas/event/equipment_bipod_deployed.json");
const SCHEMA_EQUIPMENT_BIPOD_STOWED: &str = include_str!("../schemas/event/equipment_bipod_stowed.json");
const SCHEMA_EQUIPMENT_DRILL_OVERHEATED: &str = include_str!("../schemas/event/equipment_drill_overheated.json");
const SCHEMA_EQUIPMENT_FIRE_MODE_CYCLED: &str = include_str!("../schemas/event/equipment_fire_mode_cycled.json");
const SCHEMA_EQUIPMENT_GRENADE_COOKED: &str = include_str!("../schemas/event/equipment_grenade_cooked.json");
const SCHEMA_EQUIPMENT_GRENADE_DETONATED: &str = include_str!("../schemas/event/equipment_grenade_detonated.json");
const SCHEMA_EQUIPMENT_GRENADE_THROWN: &str = include_str!("../schemas/event/equipment_grenade_thrown.json");
const SCHEMA_EQUIPMENT_ITEM_DROPPED: &str = include_str!("../schemas/event/equipment_item_dropped.json");
const SCHEMA_EQUIPMENT_ITEM_PICKED_UP: &str = include_str!("../schemas/event/equipment_item_picked_up.json");
const SCHEMA_EQUIPMENT_MAGAZINE_CHANGED: &str = include_str!("../schemas/event/equipment_magazine_changed.json");
const SCHEMA_EQUIPMENT_MELEE_SWING: &str = include_str!("../schemas/event/equipment_melee_swing.json");
const SCHEMA_EQUIPMENT_MELEE_HIT_MO: &str = include_str!("../schemas/event/equipment_melee_hit_mo.json");
const SCHEMA_EQUIPMENT_SENSOR_PULSE_FIRED: &str = include_str!("../schemas/event/equipment_sensor_pulse_fired.json");
const SCHEMA_EQUIPMENT_SHELL_EJECTED: &str = include_str!("../schemas/event/equipment_shell_ejected.json");
const SCHEMA_EQUIPMENT_SUPPRESSOR_ATTACHED: &str = include_str!("../schemas/event/equipment_suppressor_attached.json");
const SCHEMA_EQUIPMENT_TOOL_BROKEN: &str = include_str!("../schemas/event/equipment_tool_broken.json");
const SCHEMA_EQUIPMENT_TOOL_REPAIRED: &str = include_str!("../schemas/event/equipment_tool_repaired.json");
const SCHEMA_EQUIPMENT_TOOL_USED: &str = include_str!("../schemas/event/equipment_tool_used.json");
const SCHEMA_EQUIPMENT_WEAPON_SWAP_COMPLETED: &str =
    include_str!("../schemas/event/equipment_weapon_swap_completed.json");
const SCHEMA_EQUIPMENT_WEAPON_SWAP_STARTED: &str = include_str!("../schemas/event/equipment_weapon_swap_started.json");
const SCHEMA_INVENTORY_TANK_SLOT_RESERVED: &str = include_str!("../schemas/event/inventory_tank_slot_reserved.json");
const SCHEMA_INVENTORY_WEIGHT_CHANGED: &str = include_str!("../schemas/event/inventory_weight_changed.json");
const SCHEMA_PERCEPTION_ACTOR_SIGNAL: &str = include_str!("../schemas/event/perception_actor_signal.json");
const SCHEMA_PERCEPTION_FOOTSTEP_EMITTED: &str = include_str!("../schemas/event/perception_footstep_emitted.json");
const SCHEMA_PERCEPTION_OCCLUSION_APPLIED: &str = include_str!("../schemas/event/perception_occlusion_applied.json");
const SCHEMA_PERCEPTION_STEALTH_METER_CHANGED: &str =
    include_str!("../schemas/event/perception_stealth_meter_changed.json");
const SCHEMA_SQUAD_COMMAND_ISSUED: &str = include_str!("../schemas/event/squad_command_issued.json");
const SCHEMA_SQUAD_MEMBER_ADDED: &str = include_str!("../schemas/event/squad_member_added.json");
const SCHEMA_SQUAD_WAYPOINT_MARKED: &str = include_str!("../schemas/event/squad_waypoint_marked.json");

// **M7-A**: smart commandable AI surface — reason labels, layer invocations,
// archetype assignment, auto-triage / auto-repair contracts, cover seeking,
// suppression, retreat, squad-comm relays, patrol, friendly-fire avoidance,
// high-ground preference. Mission director v0.5 adds phase changes,
// branching, optional offerings, reinforcement waves, mini-boss phases.
const SCHEMA_AI_REASON_LABEL_CHANGED: &str = include_str!("../schemas/event/ai_reason_label_changed.json");
const SCHEMA_AI_THINKING_LAYER_INVOKED: &str = include_str!("../schemas/event/ai_thinking_layer_invoked.json");
const SCHEMA_AI_ARCHETYPE_CHOSEN: &str = include_str!("../schemas/event/ai_archetype_chosen.json");
const SCHEMA_AI_AUTO_TRIAGE_INITIATED: &str = include_str!("../schemas/event/ai_auto_triage_initiated.json");
const SCHEMA_AI_AUTO_TRIAGE_APPLIED: &str = include_str!("../schemas/event/ai_auto_triage_applied.json");
const SCHEMA_AI_AUTO_REPAIR_INITIATED: &str = include_str!("../schemas/event/ai_auto_repair_initiated.json");
const SCHEMA_AI_AUTO_REPAIR_PROGRESSED: &str = include_str!("../schemas/event/ai_auto_repair_progressed.json");
const SCHEMA_AI_COVER_SEEKING_STARTED: &str = include_str!("../schemas/event/ai_cover_seeking_started.json");
const SCHEMA_AI_SUPPRESSION_STARTED: &str = include_str!("../schemas/event/ai_suppression_started.json");
const SCHEMA_AI_RETREAT_DECISION: &str = include_str!("../schemas/event/ai_retreat_decision.json");
const SCHEMA_AI_SQUAD_COMM_RELAYED: &str = include_str!("../schemas/event/ai_squad_comm_relayed.json");
const SCHEMA_AI_PATROL_WAYPOINT_REACHED: &str = include_str!("../schemas/event/ai_patrol_waypoint_reached.json");
const SCHEMA_AI_FRIENDLY_FIRE_AVOIDANCE: &str = include_str!("../schemas/event/ai_friendly_fire_avoidance.json");
const SCHEMA_AI_HIGH_GROUND_PREFERENCE_APPLIED: &str =
    include_str!("../schemas/event/ai_high_ground_preference_applied.json");
const SCHEMA_MISSION_PHASE_CHANGED: &str = include_str!("../schemas/event/mission_phase_changed.json");
const SCHEMA_MISSION_DIRECTOR_PHASE_CHANGE: &str = include_str!("../schemas/event/mission_director_phase_change.json");
const SCHEMA_MISSION_OBJECTIVE_BRANCHED: &str = include_str!("../schemas/event/mission_objective_branched.json");
const SCHEMA_MISSION_OPTIONAL_OFFERED: &str = include_str!("../schemas/event/mission_optional_offered.json");
const SCHEMA_MISSION_REINFORCEMENT_WAVE_SPAWNED: &str =
    include_str!("../schemas/event/mission_reinforcement_wave_spawned.json");
const SCHEMA_BOSS_PHASE_CHANGED: &str = include_str!("../schemas/event/boss_phase_changed.json");
const SCHEMA_BOSS_SPECIAL_ABILITY_TRIGGERED: &str =
    include_str!("../schemas/event/boss_special_ability_triggered.json");

// **M7-B**: commandability + chatter + personality + faction event surface.
// Spec § Smart commandable AI — per-task override + autonomy mode + role
// templates + quick presets + chatter scaffold + personality + mood/stress
// + faction matrix.
const SCHEMA_AI_PRIORITY_TABLE_CHANGED: &str = include_str!("../schemas/event/ai_priority_table_changed.json");
const SCHEMA_AI_AUTONOMY_MODE_CHANGED: &str = include_str!("../schemas/event/ai_autonomy_mode_changed.json");
const SCHEMA_AI_ROLE_TEMPLATE_APPLIED: &str = include_str!("../schemas/event/ai_role_template_applied.json");
const SCHEMA_AI_QUICK_PRESET_APPLIED: &str = include_str!("../schemas/event/ai_quick_preset_applied.json");
const SCHEMA_AI_CHATTER_EMITTED: &str = include_str!("../schemas/event/ai_chatter_emitted.json");
const SCHEMA_AI_PERSONALITY_CHANGED: &str = include_str!("../schemas/event/ai_personality_changed.json");
const SCHEMA_AI_MOOD_CHANGED: &str = include_str!("../schemas/event/ai_mood_changed.json");
const SCHEMA_AI_STRESS_THRESHOLD_CROSSED: &str = include_str!("../schemas/event/ai_stress_threshold_crossed.json");
const SCHEMA_AI_FACTION_ALLEGIANCE_CHANGED: &str = include_str!("../schemas/event/ai_faction_allegiance_changed.json");

// **M8**: camera + photo mode + replay scrubber + killcam + UX widgets +
// smart commandable AI player UX surfaces. Spec §§ Camera + game feel,
// Photo mode, Replay scrubber, Killcam, Settings menu tree, Smart
// commandable AI player UX (Tab tactical overlay + plan composer +
// context wheel + panic surfaces + MMB tag + Why? key).
const SCHEMA_CAMERA_HIT_STOP: &str = include_str!("../schemas/event/camera_hit_stop.json");
const SCHEMA_CAMERA_MODE_CHANGED: &str = include_str!("../schemas/event/camera_mode_changed.json");
const SCHEMA_PHOTO_MODE_ENTERED: &str = include_str!("../schemas/event/photo_mode_entered.json");
const SCHEMA_PHOTO_MODE_EXITED: &str = include_str!("../schemas/event/photo_mode_exited.json");
const SCHEMA_PHOTO_MODE_FILTER_CHANGED: &str = include_str!("../schemas/event/photo_mode_filter_changed.json");
const SCHEMA_PHOTO_MODE_SHOT_TAKEN: &str = include_str!("../schemas/event/photo_mode_shot_taken.json");
const SCHEMA_REPLAY_SCRUB_OFFSET_CHANGED: &str = include_str!("../schemas/event/replay_scrub_offset_changed.json");
const SCHEMA_REPLAY_BOOKMARK_ADDED: &str = include_str!("../schemas/event/replay_bookmark_added.json");
const SCHEMA_KILLCAM_PLAYED: &str = include_str!("../schemas/event/killcam_played.json");
const SCHEMA_KILLCAM_SKIPPED: &str = include_str!("../schemas/event/killcam_skipped.json");
const SCHEMA_SLOW_MO_KILL_CAM_TRIGGERED: &str = include_str!("../schemas/event/slow_mo_kill_cam_triggered.json");
const SCHEMA_UX_HUD_LAYOUT_CHANGED: &str = include_str!("../schemas/event/ux_hud_layout_changed.json");
const SCHEMA_UX_PRESET_SAVED: &str = include_str!("../schemas/event/ux_preset_saved.json");
const SCHEMA_UX_DEBUG_OVERLAY_TOGGLED: &str = include_str!("../schemas/event/ux_debug_overlay_toggled.json");
const SCHEMA_UX_TACTICAL_OVERLAY_TOGGLED: &str = include_str!("../schemas/event/ux_tactical_overlay_toggled.json");
const SCHEMA_UX_PIE_MENU_OPENED: &str = include_str!("../schemas/event/ux_pie_menu_opened.json");
const SCHEMA_UX_PIE_MENU_SLICE_CHOSEN: &str = include_str!("../schemas/event/ux_pie_menu_slice_chosen.json");
const SCHEMA_UX_PIE_MENU_CLOSED: &str = include_str!("../schemas/event/ux_pie_menu_closed.json");
const SCHEMA_UX_PIE_MENU_SLICE_REJECTED: &str = include_str!("../schemas/event/ux_pie_menu_slice_rejected.json");
const SCHEMA_UX_GAME_SPEED_ASSIST_CHANGED: &str = include_str!("../schemas/event/ux_game_speed_assist_changed.json");
const SCHEMA_AI_PLAN_COMPOSED: &str = include_str!("../schemas/event/ai_plan_composed.json");
const SCHEMA_AI_PLAN_EXECUTED: &str = include_str!("../schemas/event/ai_plan_executed.json");
const SCHEMA_AI_PLAN_ABORTED: &str = include_str!("../schemas/event/ai_plan_aborted.json");
const SCHEMA_AI_CONTEXT_WHEEL_OPENED: &str = include_str!("../schemas/event/ai_context_wheel_opened.json");
const SCHEMA_AI_CONTEXT_WHEEL_SELECTED: &str = include_str!("../schemas/event/ai_context_wheel_selected.json");
const SCHEMA_AI_PANIC_CALL_EMITTED: &str = include_str!("../schemas/event/ai_panic_call_emitted.json");
const SCHEMA_AI_TARGET_TAGGED: &str = include_str!("../schemas/event/ai_target_tagged.json");
const SCHEMA_AI_REASON_QUERY_RETURNED: &str = include_str!("../schemas/event/ai_reason_query_returned.json");

// **M9**: reactor defense + 5-tier integrity machine event surface lock.
// Spec § Schemas (scenario-specific) + Acceptance criteria. Producers fire
// at runtime from cf-control/src/engine.rs reactor hit + mission timer +
// per-pixel terrain integrity paths. Schemas pair with M5's deep-damage
// families (armor.* / internal.* / concussion.*) that M9 also wires.
const SCHEMA_MISSION_REACTOR_HP_CHANGED: &str = include_str!("../schemas/event/mission_reactor_hp_changed.json");
const SCHEMA_MISSION_REACTOR_DESTROYED: &str = include_str!("../schemas/event/mission_reactor_destroyed.json");
const SCHEMA_MISSION_REACTOR_PRESSURE_STATE_CHANGED: &str =
    include_str!("../schemas/event/mission_reactor_pressure_state_changed.json");
const SCHEMA_MISSION_TIMER_WARNING_THRESHOLD: &str =
    include_str!("../schemas/event/mission_timer_warning_threshold.json");
const SCHEMA_TERRAIN_MATERIAL_STATE_CHANGED: &str =
    include_str!("../schemas/event/terrain_material_state_changed.json");
const SCHEMA_TERRAIN_PIXEL_REMOVED: &str = include_str!("../schemas/event/terrain_pixel_removed.json");
const SCHEMA_TERRAIN_CASCADE_TRIGGERED: &str = include_str!("../schemas/event/terrain_cascade_triggered.json");
const SCHEMA_TERRAIN_DEBRIS_SPAWNED: &str = include_str!("../schemas/event/terrain_debris_spawned.json");
// **M9** (audit round-3 fix gaps 1+2) § Reactive guard targeting + path
// reaction: utility-scored target selection + per-guard path invalidation.
// Producers in `cf-control/src/engine.rs` emit `ai.target_scored` on every
// `ai.target_acquired` (utility scorer ranks player + non-destroyed reactors)
// and `ai.path_invalidated` per affected guard when `terrain.terrain_dirty_region_batch`
// intersects the planned pursuit line. Distinct from terrain.path_invalidated
// (which is the generic dirty-region flush) — these AI-category schemas carry
// the per-guard breakdown M10 viewer + death-recap consume.
const SCHEMA_AI_TARGET_SCORED: &str = include_str!("../schemas/event/ai_target_scored.json");
const SCHEMA_AI_PATH_INVALIDATED: &str = include_str!("../schemas/event/ai_path_invalidated.json");

/// Look up the schema source by `(category, event_type)`. Returns `None` if
/// no schema exists for this pair (callers treat as "no validation
/// constraint"; the recorder envelope itself is checked by the bundle
/// checker separately).
pub fn event_schema_for(category: &str, event_type: &str) -> Option<&'static str> {
    match (category, event_type) {
        ("input", "intent_received") => Some(SCHEMA_INPUT_INTENT_RECEIVED),
        ("equipment", "weapon_fired") => Some(SCHEMA_WEAPON_FIRED),
        ("combat", "projectile_spawned") => Some(SCHEMA_PROJECTILE_SPAWNED),
        ("combat", "wound_added") => Some(SCHEMA_WOUND_ADDED),
        ("actor", "inventory_dropped") => Some(SCHEMA_INVENTORY_DROPPED),
        ("equipment", "alarm_registered") => Some(SCHEMA_ALARM_REGISTERED),
        ("terrain", "terrain_carved") => Some(SCHEMA_TERRAIN_CARVED),
        ("terrain", "terrain_penetration_threshold") => Some(SCHEMA_TERRAIN_PENETRATION_THRESHOLD),
        ("terrain", "terrain_dirty_region_batch") => Some(SCHEMA_TERRAIN_DIRTY_REGION_BATCH),
        // M8A: semantic terrain event protocol (PR-7).
        ("terrain", "chunk_mutated") => Some(SCHEMA_TERRAIN_CHUNK_MUTATED),
        ("terrain", "chunk_active_region_changed") => Some(SCHEMA_TERRAIN_CHUNK_ACTIVE_REGION_CHANGED),
        // M8A: perf_sample cosmetic per-cadence latency event (PR-8).
        ("perf", "sample") => Some(SCHEMA_PERF_SAMPLE),
        ("terrain", "terrain_pixel_dislodged") => Some(SCHEMA_TERRAIN_PIXEL_DISLODGED),
        ("terrain", "hazard_contact_or_avoidance") => Some(SCHEMA_HAZARD_CONTACT_OR_AVOIDANCE),
        ("terrain", "anchor_material_result") => Some(SCHEMA_ANCHOR_MATERIAL_RESULT),
        ("terrain", "terrain_material_probe") => Some(SCHEMA_TERRAIN_MATERIAL_PROBE),
        ("terrain", "terrain_fill_or_repair") => Some(SCHEMA_TERRAIN_FILL_OR_REPAIR),
        ("terrain", "forced_refresh_requested") => Some(SCHEMA_FORCED_REFRESH_REQUESTED),
        // M3 audit pass 7 (2026-05-13): newly-registered schemas.
        ("terrain", "debris_capped") => Some(SCHEMA_TERRAIN_DEBRIS_CAPPED),
        ("terrain", "tool_refused") => Some(SCHEMA_TERRAIN_TOOL_REFUSED),
        ("terrain", "tool_action_started") => Some(SCHEMA_TERRAIN_TOOL_ACTION_STARTED),
        ("equipment", "tool_action_completed") => Some(SCHEMA_EQUIPMENT_TOOL_ACTION_COMPLETED),
        ("terrain", "path_invalidated") => Some(SCHEMA_TERRAIN_PATH_INVALIDATED),
        // M2 re-audit (2026-05-13): mission + AI event schemas.
        ("mission", "mission_started") => Some(SCHEMA_MISSION_STARTED),
        ("mission", "objective_started") => Some(SCHEMA_OBJECTIVE_STARTED),
        ("mission", "objective_updated") => Some(SCHEMA_OBJECTIVE_UPDATED),
        ("mission", "objective_completed") => Some(SCHEMA_OBJECTIVE_COMPLETED),
        ("mission", "objective_failed") => Some(SCHEMA_OBJECTIVE_FAILED),
        ("mission", "mission_resolved") => Some(SCHEMA_MISSION_RESOLVED),
        ("ai", "state_changed") => Some(SCHEMA_AI_STATE_CHANGED),
        ("ai", "perception_signal") => Some(SCHEMA_AI_PERCEPTION_SIGNAL),
        ("ai", "tactic_chosen") => Some(SCHEMA_AI_TACTIC_CHOSEN),
        ("ai", "missed_shot_reason") => Some(SCHEMA_AI_MISSED_SHOT_REASON),
        ("ai", "stuck_state_changed") => Some(SCHEMA_AI_STUCK_STATE_CHANGED),
        ("ai", "recovery_action") => Some(SCHEMA_AI_RECOVERY_ACTION),
        // M4: system/determinism/snapshot event schemas locked at v0.1.
        ("system", "run_started") => Some(SCHEMA_SYSTEM_RUN_STARTED),
        ("system", "run_finished") => Some(SCHEMA_SYSTEM_RUN_FINISHED),
        ("system", "category_baseline") => Some(SCHEMA_SYSTEM_CATEGORY_BASELINE),
        ("determinism", "sim_checksum") => Some(SCHEMA_DETERMINISM_SIM_CHECKSUM),
        ("determinism", "first_divergence") => Some(SCHEMA_DETERMINISM_FIRST_DIVERGENCE),
        ("snapshot", "snapshot_actor") => Some(SCHEMA_SNAPSHOT_ACTOR),
        ("snapshot", "snapshot_inventory") => Some(SCHEMA_SNAPSHOT_INVENTORY),
        ("snapshot", "snapshot_terrain_chunk") => Some(SCHEMA_SNAPSHOT_TERRAIN_CHUNK),
        ("snapshot", "snapshot_terrain_summary") => Some(SCHEMA_SNAPSHOT_TERRAIN_SUMMARY),
        ("snapshot", "snapshot_chassis") => Some(SCHEMA_SNAPSHOT_CHASSIS),
        // M4 § M9 firehose surface placeholders.
        ("snapshot", "snapshot_hazard_grid") => Some(SCHEMA_SNAPSHOT_HAZARD_GRID),
        ("snapshot", "snapshot_affliction") => Some(SCHEMA_SNAPSHOT_AFFLICTION),
        ("snapshot", "snapshot_armor_layer") => Some(SCHEMA_SNAPSHOT_ARMOR_LAYER),
        ("snapshot", "snapshot_atmospherics") => Some(SCHEMA_SNAPSHOT_ATMOSPHERICS),
        ("snapshot", "snapshot_environment_signal") => Some(SCHEMA_SNAPSHOT_ENVIRONMENT_SIGNAL),
        ("snapshot", "snapshot_armor") => Some(SCHEMA_SNAPSHOT_ARMOR),
        ("snapshot", "snapshot_internal") => Some(SCHEMA_SNAPSHOT_INTERNAL),
        ("snapshot", "snapshot_concussion") => Some(SCHEMA_SNAPSHOT_CONCUSSION),
        ("snapshot", "snapshot_fluid") => Some(SCHEMA_SNAPSHOT_FLUID),
        ("snapshot", "snapshot_origin") => Some(SCHEMA_SNAPSHOT_ORIGIN),
        ("snapshot", "snapshot_shield") => Some(SCHEMA_SNAPSHOT_SHIELD),
        ("snapshot", "snapshot_thermal") => Some(SCHEMA_SNAPSHOT_THERMAL),
        // M5 deep-damage event-surface lock (2026-05-13). Each schema is the
        // M4 v0.1 envelope shape with `schema_version: "0.1"` const and the
        // payload nested under properties.payload. Producers ladder up at
        // M13/M14/M15/M16/M17/M19/M20.
        // armor.* (M13 + M14).
        ("armor", "layer_hp_changed") => Some(SCHEMA_ARMOR_LAYER_HP_CHANGED),
        ("armor", "layer_critical") => Some(SCHEMA_ARMOR_LAYER_CRITICAL),
        ("armor", "layer_destroyed") => Some(SCHEMA_ARMOR_LAYER_DESTROYED),
        ("armor", "all_layers_destroyed") => Some(SCHEMA_ARMOR_ALL_LAYERS_DESTROYED),
        ("armor", "chunked_off") => Some(SCHEMA_ARMOR_CHUNKED_OFF),
        ("armor", "debris_spawned") => Some(SCHEMA_ARMOR_DEBRIS_SPAWNED),
        ("armor", "repaired") => Some(SCHEMA_ARMOR_REPAIRED),
        ("armor", "angle_deflection_calculated") => Some(SCHEMA_ARMOR_ANGLE_DEFLECTION_CALCULATED),
        ("armor", "ricochet") => Some(SCHEMA_ARMOR_RICOCHET),
        ("armor", "spalling") => Some(SCHEMA_ARMOR_SPALLING),
        ("armor", "penetration_ray_traversed") => Some(SCHEMA_ARMOR_PENETRATION_RAY_TRAVERSED),
        ("armor", "he_overpressure_wave") => Some(SCHEMA_ARMOR_HE_OVERPRESSURE_WAVE),
        ("armor", "heat_jet_penetrated") => Some(SCHEMA_ARMOR_HEAT_JET_PENETRATED),
        ("armor", "heat_jet_pre_detonated_by_era") => Some(SCHEMA_ARMOR_HEAT_JET_PRE_DETONATED_BY_ERA),
        ("armor", "apfsds_penetrated") => Some(SCHEMA_ARMOR_APFSDS_PENETRATED),
        ("armor", "era_panel_detonated") => Some(SCHEMA_ARMOR_ERA_PANEL_DETONATED),
        ("armor", "schurzen_pre_detonated") => Some(SCHEMA_ARMOR_SCHURZEN_PRE_DETONATED),
        ("armor", "multi_hit_degradation") => Some(SCHEMA_ARMOR_MULTI_HIT_DEGRADATION),
        ("armor", "reactive_armor_consumed") => Some(SCHEMA_ARMOR_REACTIVE_ARMOR_CONSUMED),
        // internal.* (M14 + M17).
        ("internal", "organ_damaged") => Some(SCHEMA_INTERNAL_ORGAN_DAMAGED),
        ("internal", "organ_destroyed") => Some(SCHEMA_INTERNAL_ORGAN_DESTROYED),
        ("internal", "organ_failure_cascade") => Some(SCHEMA_INTERNAL_ORGAN_FAILURE_CASCADE),
        ("internal", "circuit_damaged") => Some(SCHEMA_INTERNAL_CIRCUIT_DAMAGED),
        ("internal", "circuit_destroyed") => Some(SCHEMA_INTERNAL_CIRCUIT_DESTROYED),
        ("internal", "circuit_failure_cascade") => Some(SCHEMA_INTERNAL_CIRCUIT_FAILURE_CASCADE),
        // concussion.* + internal_shock.* (M17).
        ("concussion", "dose_changed") => Some(SCHEMA_CONCUSSION_DOSE_CHANGED),
        ("concussion", "band_changed") => Some(SCHEMA_CONCUSSION_BAND_CHANGED),
        ("concussion", "ko_threshold_crossed") => Some(SCHEMA_CONCUSSION_KO_THRESHOLD_CROSSED),
        ("concussion", "recovered") => Some(SCHEMA_CONCUSSION_RECOVERED),
        ("internal_shock", "dose_changed") => Some(SCHEMA_INTERNAL_SHOCK_DOSE_CHANGED),
        ("internal_shock", "module_damaged") => Some(SCHEMA_INTERNAL_SHOCK_MODULE_DAMAGED),
        // fluid.* (M13 + M14).
        ("fluid", "leak_started") => Some(SCHEMA_FLUID_LEAK_STARTED),
        ("fluid", "leak_rate_changed") => Some(SCHEMA_FLUID_LEAK_RATE_CHANGED),
        ("fluid", "reservoir_warning") => Some(SCHEMA_FLUID_RESERVOIR_WARNING),
        ("fluid", "reservoir_critical") => Some(SCHEMA_FLUID_RESERVOIR_CRITICAL),
        ("fluid", "reservoir_empty") => Some(SCHEMA_FLUID_RESERVOIR_EMPTY),
        ("fluid", "ignition") => Some(SCHEMA_FLUID_IGNITION),
        ("fluid", "ground_splatter_spawned") => Some(SCHEMA_FLUID_GROUND_SPLATTER_SPAWNED),
        ("fluid", "leak_stopped") => Some(SCHEMA_FLUID_LEAK_STOPPED),
        ("fluid", "refilled") => Some(SCHEMA_FLUID_REFILLED),
        // origin.* (M17).
        ("origin", "shot_force_feedback") => Some(SCHEMA_ORIGIN_SHOT_FORCE_FEEDBACK),
        ("origin", "g_load_dose_changed") => Some(SCHEMA_ORIGIN_G_LOAD_DOSE_CHANGED),
        ("origin", "helmet_breach") => Some(SCHEMA_ORIGIN_HELMET_BREACH),
        ("origin", "oxygen_supply_changed") => Some(SCHEMA_ORIGIN_OXYGEN_SUPPLY_CHANGED),
        // hazard.* (M16).
        ("hazard", "spawned") => Some(SCHEMA_HAZARD_SPAWNED),
        ("hazard", "spread") => Some(SCHEMA_HAZARD_SPREAD),
        ("hazard", "actor_contact") => Some(SCHEMA_HAZARD_ACTOR_CONTACT),
        ("hazard", "tick") => Some(SCHEMA_HAZARD_TICK),
        ("hazard", "dissipated") => Some(SCHEMA_HAZARD_DISSIPATED),
        // affliction.* (M16).
        ("affliction", "applied") => Some(SCHEMA_AFFLICTION_APPLIED),
        ("affliction", "tick") => Some(SCHEMA_AFFLICTION_TICK),
        ("affliction", "cleared") => Some(SCHEMA_AFFLICTION_CLEARED),
        ("affliction", "escalated") => Some(SCHEMA_AFFLICTION_ESCALATED),
        // atmos.* (M19).
        ("atmos", "pressure_changed") => Some(SCHEMA_ATMOS_PRESSURE_CHANGED),
        ("atmos", "temperature_changed") => Some(SCHEMA_ATMOS_TEMPERATURE_CHANGED),
        ("atmos", "gas_released") => Some(SCHEMA_ATMOS_GAS_RELEASED),
        ("atmos", "breach_detected") => Some(SCHEMA_ATMOS_BREACH_DETECTED),
        ("atmos", "combustion_ignition") => Some(SCHEMA_ATMOS_COMBUSTION_IGNITION),
        ("atmos", "phase_transition") => Some(SCHEMA_ATMOS_PHASE_TRANSITION),
        ("atmos", "pipe_flow") => Some(SCHEMA_ATMOS_PIPE_FLOW),
        ("atmos", "pipe_freeze") => Some(SCHEMA_ATMOS_PIPE_FREEZE),
        ("atmos", "pipe_rupture") => Some(SCHEMA_ATMOS_PIPE_RUPTURE),
        ("atmos", "electrolysis_started") => Some(SCHEMA_ATMOS_ELECTROLYSIS_STARTED),
        // shield.* (M13+ + M25+).
        ("shield", "hit") => Some(SCHEMA_SHIELD_HIT),
        ("shield", "depleted") => Some(SCHEMA_SHIELD_DEPLETED),
        ("shield", "regen_started") => Some(SCHEMA_SHIELD_REGEN_STARTED),
        ("shield", "regen_completed") => Some(SCHEMA_SHIELD_REGEN_COMPLETED),
        ("shield", "disrupted") => Some(SCHEMA_SHIELD_DISRUPTED),
        // environment.* (M20).
        ("environment", "signal_delta") => Some(SCHEMA_ENVIRONMENT_SIGNAL_DELTA),
        ("environment", "signal_aggregated") => Some(SCHEMA_ENVIRONMENT_SIGNAL_AGGREGATED),
        // thermal.* (M16 + M19).
        ("thermal", "signature_changed") => Some(SCHEMA_THERMAL_SIGNATURE_CHANGED),
        ("thermal", "heat_exchanged") => Some(SCHEMA_THERMAL_HEAT_EXCHANGED),
        ("thermal", "material_phase_change") => Some(SCHEMA_THERMAL_MATERIAL_PHASE_CHANGE),
        // combat.projectile_hit_mo expanded payload (M13 + M14).
        ("combat", "projectile_hit_mo") => Some(SCHEMA_COMBAT_PROJECTILE_HIT_MO),
        // audio.event_requested (M5 spec mandate — M13.x cf-audio consumes).
        ("audio", "event_requested") => Some(SCHEMA_AUDIO_EVENT_REQUESTED),
        // M5-A2: combat.melee_hit_mo + combat.explosive_hit_mo for M6 melee
        // + grenade hits + M13 HE rounds.
        ("combat", "melee_hit_mo") => Some(SCHEMA_COMBAT_MELEE_HIT_MO),
        ("combat", "explosive_hit_mo") => Some(SCHEMA_COMBAT_EXPLOSIVE_HIT_MO),
        // M6 actor / combat / equipment / inventory / perception / squad
        // event payload schemas. Event types match the literals the cf-control
        // engine records via `recorder.record(tick, sim_time_ms, category,
        // event_type, payload, parent)` for each M6 dispatch path.
        ("actor", "action_rejected") => Some(SCHEMA_ACTOR_ACTION_REJECTED),
        ("actor", "climb_started") => Some(SCHEMA_ACTOR_CLIMB_STARTED),
        ("actor", "dive_started") => Some(SCHEMA_ACTOR_DIVE_STARTED),
        ("actor", "facing_changed") => Some(SCHEMA_ACTOR_FACING_CHANGED),
        ("actor", "lean_changed") => Some(SCHEMA_ACTOR_LEAN_CHANGED),
        ("actor", "slide_started") => Some(SCHEMA_ACTOR_SLIDE_STARTED),
        ("actor", "stamina_changed") => Some(SCHEMA_ACTOR_STAMINA_CHANGED),
        ("actor", "stance_changed") => Some(SCHEMA_ACTOR_STANCE_CHANGED),
        ("actor", "vault_started") => Some(SCHEMA_ACTOR_VAULT_STARTED),
        ("combat", "knife_throw_landed") => Some(SCHEMA_COMBAT_KNIFE_THROW_LANDED),
        ("combat", "knife_throw_started") => Some(SCHEMA_COMBAT_KNIFE_THROW_STARTED),
        ("combat", "stealth_kill_executed") => Some(SCHEMA_COMBAT_STEALTH_KILL_EXECUTED),
        ("equipment", "beacon_dropped") => Some(SCHEMA_EQUIPMENT_BEACON_DROPPED),
        ("equipment", "bipod_deployed") => Some(SCHEMA_EQUIPMENT_BIPOD_DEPLOYED),
        ("equipment", "bipod_stowed") => Some(SCHEMA_EQUIPMENT_BIPOD_STOWED),
        ("equipment", "drill_overheated") => Some(SCHEMA_EQUIPMENT_DRILL_OVERHEATED),
        ("equipment", "fire_mode_cycled") => Some(SCHEMA_EQUIPMENT_FIRE_MODE_CYCLED),
        ("equipment", "grenade_cooked") => Some(SCHEMA_EQUIPMENT_GRENADE_COOKED),
        ("equipment", "grenade_detonated") => Some(SCHEMA_EQUIPMENT_GRENADE_DETONATED),
        ("equipment", "grenade_thrown") => Some(SCHEMA_EQUIPMENT_GRENADE_THROWN),
        ("equipment", "item_dropped") => Some(SCHEMA_EQUIPMENT_ITEM_DROPPED),
        ("equipment", "item_picked_up") => Some(SCHEMA_EQUIPMENT_ITEM_PICKED_UP),
        ("equipment", "magazine_changed") => Some(SCHEMA_EQUIPMENT_MAGAZINE_CHANGED),
        ("equipment", "melee_swing") => Some(SCHEMA_EQUIPMENT_MELEE_SWING),
        ("equipment", "melee_hit_mo") => Some(SCHEMA_EQUIPMENT_MELEE_HIT_MO),
        ("equipment", "sensor_pulse_fired") => Some(SCHEMA_EQUIPMENT_SENSOR_PULSE_FIRED),
        ("equipment", "shell_ejected") => Some(SCHEMA_EQUIPMENT_SHELL_EJECTED),
        ("equipment", "suppressor_attached") => Some(SCHEMA_EQUIPMENT_SUPPRESSOR_ATTACHED),
        ("equipment", "tool_broken") => Some(SCHEMA_EQUIPMENT_TOOL_BROKEN),
        ("equipment", "tool_repaired") => Some(SCHEMA_EQUIPMENT_TOOL_REPAIRED),
        ("equipment", "tool_used") => Some(SCHEMA_EQUIPMENT_TOOL_USED),
        ("equipment", "weapon_swap_completed") => Some(SCHEMA_EQUIPMENT_WEAPON_SWAP_COMPLETED),
        ("equipment", "weapon_swap_started") => Some(SCHEMA_EQUIPMENT_WEAPON_SWAP_STARTED),
        ("inventory", "tank_slot_reserved") => Some(SCHEMA_INVENTORY_TANK_SLOT_RESERVED),
        ("inventory", "weight_changed") => Some(SCHEMA_INVENTORY_WEIGHT_CHANGED),
        ("perception", "actor_signal") => Some(SCHEMA_PERCEPTION_ACTOR_SIGNAL),
        ("perception", "footstep_emitted") => Some(SCHEMA_PERCEPTION_FOOTSTEP_EMITTED),
        ("perception", "occlusion_applied") => Some(SCHEMA_PERCEPTION_OCCLUSION_APPLIED),
        ("perception", "stealth_meter_changed") => Some(SCHEMA_PERCEPTION_STEALTH_METER_CHANGED),
        ("squad", "command_issued") => Some(SCHEMA_SQUAD_COMMAND_ISSUED),
        ("squad", "member_added") => Some(SCHEMA_SQUAD_MEMBER_ADDED),
        ("squad", "waypoint_marked") => Some(SCHEMA_SQUAD_WAYPOINT_MARKED),
        // **M7-A**: smart commandable AI event surface.
        ("ai", "reason_label_changed") => Some(SCHEMA_AI_REASON_LABEL_CHANGED),
        ("ai", "thinking_layer_invoked") => Some(SCHEMA_AI_THINKING_LAYER_INVOKED),
        ("ai", "archetype_chosen") => Some(SCHEMA_AI_ARCHETYPE_CHOSEN),
        ("ai", "auto_triage_initiated") => Some(SCHEMA_AI_AUTO_TRIAGE_INITIATED),
        ("ai", "auto_triage_applied") => Some(SCHEMA_AI_AUTO_TRIAGE_APPLIED),
        ("ai", "auto_repair_initiated") => Some(SCHEMA_AI_AUTO_REPAIR_INITIATED),
        ("ai", "auto_repair_progressed") => Some(SCHEMA_AI_AUTO_REPAIR_PROGRESSED),
        ("ai", "cover_seeking_started") => Some(SCHEMA_AI_COVER_SEEKING_STARTED),
        ("ai", "suppression_started") => Some(SCHEMA_AI_SUPPRESSION_STARTED),
        ("ai", "retreat_decision") => Some(SCHEMA_AI_RETREAT_DECISION),
        ("ai", "squad_comm_relayed") => Some(SCHEMA_AI_SQUAD_COMM_RELAYED),
        ("ai", "patrol_waypoint_reached") => Some(SCHEMA_AI_PATROL_WAYPOINT_REACHED),
        ("ai", "friendly_fire_avoidance") => Some(SCHEMA_AI_FRIENDLY_FIRE_AVOIDANCE),
        ("ai", "high_ground_preference_applied") => Some(SCHEMA_AI_HIGH_GROUND_PREFERENCE_APPLIED),
        // **M7**: mission director v0.5 event surface.
        ("mission", "phase_changed") => Some(SCHEMA_MISSION_PHASE_CHANGED),
        // **M9**: 7-phase reactor-defense pacer surfaces a distinct event so
        // M10 viewer + M11 HUD can render dwell-aware phase strips without
        // mixing the M7 4-phase stream.
        ("mission", "director_phase_change") => Some(SCHEMA_MISSION_DIRECTOR_PHASE_CHANGE),
        ("mission", "objective_branched") => Some(SCHEMA_MISSION_OBJECTIVE_BRANCHED),
        ("mission", "optional_offered") => Some(SCHEMA_MISSION_OPTIONAL_OFFERED),
        ("mission", "reinforcement_wave_spawned") => Some(SCHEMA_MISSION_REINFORCEMENT_WAVE_SPAWNED),
        ("boss", "phase_changed") => Some(SCHEMA_BOSS_PHASE_CHANGED),
        ("boss", "special_ability_triggered") => Some(SCHEMA_BOSS_SPECIAL_ABILITY_TRIGGERED),
        // **M7-B**: commandability + chatter + personality + faction event surface.
        ("ai", "priority_table_changed") => Some(SCHEMA_AI_PRIORITY_TABLE_CHANGED),
        ("ai", "autonomy_mode_changed") => Some(SCHEMA_AI_AUTONOMY_MODE_CHANGED),
        ("ai", "role_template_applied") => Some(SCHEMA_AI_ROLE_TEMPLATE_APPLIED),
        ("ai", "quick_preset_applied") => Some(SCHEMA_AI_QUICK_PRESET_APPLIED),
        ("ai", "chatter_emitted") => Some(SCHEMA_AI_CHATTER_EMITTED),
        ("ai", "personality_changed") => Some(SCHEMA_AI_PERSONALITY_CHANGED),
        ("ai", "mood_changed") => Some(SCHEMA_AI_MOOD_CHANGED),
        ("ai", "stress_threshold_crossed") => Some(SCHEMA_AI_STRESS_THRESHOLD_CROSSED),
        ("ai", "faction_allegiance_changed") => Some(SCHEMA_AI_FACTION_ALLEGIANCE_CHANGED),
        // **M8**: camera + photo + replay scrub + killcam + UX + smart-AI
        // player UX surfaces.
        ("camera", "hit_stop") => Some(SCHEMA_CAMERA_HIT_STOP),
        ("camera", "mode_changed") => Some(SCHEMA_CAMERA_MODE_CHANGED),
        ("photo_mode", "entered") => Some(SCHEMA_PHOTO_MODE_ENTERED),
        ("photo_mode", "exited") => Some(SCHEMA_PHOTO_MODE_EXITED),
        ("photo_mode", "filter_changed") => Some(SCHEMA_PHOTO_MODE_FILTER_CHANGED),
        ("photo_mode", "shot_taken") => Some(SCHEMA_PHOTO_MODE_SHOT_TAKEN),
        ("replay", "scrub_offset_changed") => Some(SCHEMA_REPLAY_SCRUB_OFFSET_CHANGED),
        ("replay", "bookmark_added") => Some(SCHEMA_REPLAY_BOOKMARK_ADDED),
        ("killcam", "played") => Some(SCHEMA_KILLCAM_PLAYED),
        ("killcam", "skipped") => Some(SCHEMA_KILLCAM_SKIPPED),
        ("slow_mo", "kill_cam_triggered") => Some(SCHEMA_SLOW_MO_KILL_CAM_TRIGGERED),
        ("ux", "hud_layout_changed") => Some(SCHEMA_UX_HUD_LAYOUT_CHANGED),
        ("ux", "preset_saved") => Some(SCHEMA_UX_PRESET_SAVED),
        ("ux", "debug_overlay_toggled") => Some(SCHEMA_UX_DEBUG_OVERLAY_TOGGLED),
        ("ux", "tactical_overlay_toggled") => Some(SCHEMA_UX_TACTICAL_OVERLAY_TOGGLED),
        ("ux", "pie_menu_opened") => Some(SCHEMA_UX_PIE_MENU_OPENED),
        ("ux", "pie_menu_slice_chosen") => Some(SCHEMA_UX_PIE_MENU_SLICE_CHOSEN),
        ("ux", "pie_menu_closed") => Some(SCHEMA_UX_PIE_MENU_CLOSED),
        ("ux", "pie_menu_slice_rejected") => Some(SCHEMA_UX_PIE_MENU_SLICE_REJECTED),
        ("ux", "game_speed_assist_changed") => Some(SCHEMA_UX_GAME_SPEED_ASSIST_CHANGED),
        ("ai", "plan_composed") => Some(SCHEMA_AI_PLAN_COMPOSED),
        ("ai", "plan_executed") => Some(SCHEMA_AI_PLAN_EXECUTED),
        ("ai", "plan_aborted") => Some(SCHEMA_AI_PLAN_ABORTED),
        ("ai", "context_wheel_opened") => Some(SCHEMA_AI_CONTEXT_WHEEL_OPENED),
        ("ai", "context_wheel_selected") => Some(SCHEMA_AI_CONTEXT_WHEEL_SELECTED),
        ("ai", "panic_call_emitted") => Some(SCHEMA_AI_PANIC_CALL_EMITTED),
        ("ai", "target_tagged") => Some(SCHEMA_AI_TARGET_TAGGED),
        ("ai", "reason_query_returned") => Some(SCHEMA_AI_REASON_QUERY_RETURNED),
        // **M9**: reactor defense + 5-tier integrity event surface.
        ("mission", "reactor_hp_changed") => Some(SCHEMA_MISSION_REACTOR_HP_CHANGED),
        ("mission", "reactor_destroyed") => Some(SCHEMA_MISSION_REACTOR_DESTROYED),
        ("mission", "reactor_pressure_state_changed") => Some(SCHEMA_MISSION_REACTOR_PRESSURE_STATE_CHANGED),
        ("mission", "timer_warning_threshold") => Some(SCHEMA_MISSION_TIMER_WARNING_THRESHOLD),
        ("terrain", "material_state_changed") => Some(SCHEMA_TERRAIN_MATERIAL_STATE_CHANGED),
        ("terrain", "pixel_removed") => Some(SCHEMA_TERRAIN_PIXEL_REMOVED),
        ("terrain", "cascade_triggered") => Some(SCHEMA_TERRAIN_CASCADE_TRIGGERED),
        ("terrain", "debris_spawned") => Some(SCHEMA_TERRAIN_DEBRIS_SPAWNED),
        // **M9** (audit round-3 fix gaps 1+2) — ai.target_scored +
        // ai.path_invalidated for the utility-scored target selection +
        // per-guard path reaction surface.
        ("ai", "target_scored") => Some(SCHEMA_AI_TARGET_SCORED),
        ("ai", "path_invalidated") => Some(SCHEMA_AI_PATH_INVALIDATED),
        _ => None,
    }
}

/// Result of a schema validation.
pub type ValidationResult = Result<(), String>;

#[derive(Deserialize)]
struct RawSchema {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    properties: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct PropConstraint {
    #[serde(default, rename = "type")]
    ty: Option<Value>,
    #[serde(default, rename = "enum")]
    enum_values: Option<Vec<Value>>,
    #[serde(default, rename = "minItems")]
    min_items: Option<usize>,
    #[serde(default, rename = "maxItems")]
    max_items: Option<usize>,
    #[serde(default)]
    minimum: Option<f64>,
    #[serde(default)]
    maximum: Option<f64>,
    /// **M5-A1**: `oneOf` lets a property accept one of several alternative
    /// type/enum branches (e.g. `origin_id` accepts either an integer OR a
    /// canonical Origin-enum string). The minimal validator walks each
    /// branch and passes if ANY branch accepts the value.
    #[serde(default, rename = "oneOf")]
    one_of: Option<Vec<Value>>,
    /// **M5-A2**: `items` lets an array property constrain its items'
    /// type + enum (e.g. `applied_afflictions: array<affliction-kind-string>`).
    /// Only the simple form `items: { type, enum }` is honored — tuple form
    /// `items: [...]` is left unenforced (the projectile-spawned schemas
    /// use that form for fixed-arity tuples; their arity is already gated
    /// by `minItems`/`maxItems`).
    #[serde(default)]
    items: Option<Value>,
    /// **M5-A2**: `properties` + `required` on a nested object property
    /// (e.g. `signal: { properties: { schema_version, active_hazards }, required: [...] }`).
    /// The validator recurses into the nested object and enforces both.
    #[serde(default)]
    properties: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    required: Option<Vec<String>>,
}

/// Validate that `payload` matches the schema registered for
/// `(category, event_type)`. Returns `Ok(())` when there is no registered
/// schema or the payload satisfies the schema's required-field + type +
/// range constraints.
///
/// Supports two schema shapes:
/// 1. **Legacy payload-only** (M2/M3/M4 schemas in `schemas/event/`): the
///    schema describes the payload object directly; `required` /
///    `properties` apply to the payload value.
/// 2. **M5 envelope-shaped**: the schema describes the full event envelope
///    with `properties.schema_version.const = "prototype-recorder-event.v0.1"`,
///    `category` const, `event_type` const, and `payload` nested under
///    `properties.payload`. For these schemas the validator extracts the
///    `payload` sub-schema and validates the supplied `payload` argument
///    against it. The canonical literal matches `EVENT_SCHEMA_VERSION` in
///    `lib.rs` so producer events emitted by the recorder will satisfy
///    strict JSON Schema validators reading these schemas externally.
pub fn validate_event_payload(category: &str, event_type: &str, payload: &Value) -> ValidationResult {
    let Some(raw) = event_schema_for(category, event_type) else {
        return Ok(());
    };
    let full_value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("schema parse error for {category}.{event_type}: {e}"))?;
    // Detect M5 envelope-shape: schema_version is a const-checked property at
    // the top level pinning the canonical M4 envelope literal. If so, walk
    // into properties.payload to extract the actual payload sub-schema. We
    // accept either the canonical envelope literal or the legacy short form
    // ("0.1") as the marker so the marker check is tolerant during migration.
    let payload_schema_source: Value = if let Some(props) = full_value.get("properties").and_then(|v| v.as_object()) {
        let sv = props
            .get("schema_version")
            .and_then(|v| v.get("const"))
            .and_then(|v| v.as_str());
        if matches!(sv, Some("prototype-recorder-event.v0.1") | Some("0.1")) {
            props
                .get("payload")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object"}))
        } else {
            full_value.clone()
        }
    } else {
        full_value.clone()
    };
    let schema: RawSchema = serde_json::from_value(payload_schema_source)
        .map_err(|e| format!("payload schema parse error for {category}.{event_type}: {e}"))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| format!("payload for {category}.{event_type} must be an object"))?;
    for req in &schema.required {
        if !obj.contains_key(req) {
            return Err(format!("{category}.{event_type}: required field `{req}` missing"));
        }
    }
    for (key, raw_constraint) in &schema.properties {
        let Some(value) = obj.get(key) else {
            continue;
        };
        let constraint: PropConstraint = serde_json::from_value(raw_constraint.clone())
            .map_err(|e| format!("{category}.{event_type}::{key} constraint parse error: {e}"))?;
        if let Some(ty) = &constraint.ty {
            check_type(category, event_type, key, ty, value)?;
        }
        if let Some(enum_values) = &constraint.enum_values {
            if !enum_values.contains(value) {
                return Err(format!(
                    "{category}.{event_type}::{key} value {value} not in enum {enum_values:?}"
                ));
            }
        }
        if let Some(min_items) = constraint.min_items {
            if let Some(arr) = value.as_array() {
                if arr.len() < min_items {
                    return Err(format!(
                        "{category}.{event_type}::{key} array length {} < minItems {}",
                        arr.len(),
                        min_items
                    ));
                }
            }
        }
        if let Some(max_items) = constraint.max_items {
            if let Some(arr) = value.as_array() {
                if arr.len() > max_items {
                    return Err(format!(
                        "{category}.{event_type}::{key} array length {} > maxItems {}",
                        arr.len(),
                        max_items
                    ));
                }
            }
        }
        if let Some(min) = constraint.minimum {
            if let Some(n) = value.as_f64() {
                if n < min {
                    return Err(format!("{category}.{event_type}::{key} value {n} < minimum {min}"));
                }
            }
        }
        if let Some(max) = constraint.maximum {
            if let Some(n) = value.as_f64() {
                if n > max {
                    return Err(format!("{category}.{event_type}::{key} value {n} > maximum {max}"));
                }
            }
        }
        if let Some(branches) = &constraint.one_of {
            let mut any_match = false;
            let mut branch_errors: Vec<String> = Vec::new();
            for (i, branch) in branches.iter().enumerate() {
                match check_one_of_branch(category, event_type, key, branch, value) {
                    Ok(()) => {
                        any_match = true;
                        break;
                    }
                    Err(e) => branch_errors.push(format!("branch[{i}]: {e}")),
                }
            }
            if !any_match {
                return Err(format!(
                    "{category}.{event_type}::{key} value {value} did not satisfy any oneOf branch — {}",
                    branch_errors.join("; ")
                ));
            }
        }
        // **M5-A2**: array-item type + enum enforcement.
        if let Some(items_schema) = &constraint.items {
            if items_schema.is_object() {
                if let Some(arr) = value.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        check_array_item(category, event_type, key, i, items_schema, item)?;
                    }
                }
            }
        }
        // **M5-A2**: nested-object properties + required enforcement.
        // Triggered when the parent property is type=object AND declares
        // its own properties/required (e.g. environment.signal_aggregated.signal).
        if let Some(nested_props) = &constraint.properties {
            if let Some(obj) = value.as_object() {
                if let Some(nested_required) = &constraint.required {
                    for req in nested_required {
                        if !obj.contains_key(req) {
                            return Err(format!("{category}.{event_type}::{key}.{req} required field missing"));
                        }
                    }
                }
                for (nested_key, nested_raw) in nested_props {
                    let Some(nested_value) = obj.get(nested_key) else {
                        continue;
                    };
                    check_nested_property(category, event_type, key, nested_key, nested_raw, nested_value)?;
                }
            }
        }
    }
    Ok(())
}

/// **M5-A2**: validate one element of an array against the schema's
/// `items` constraint. Supports `type` (string or array union) and `enum`.
fn check_array_item(
    category: &str,
    event_type: &str,
    key: &str,
    index: usize,
    items_schema: &Value,
    item: &Value,
) -> ValidationResult {
    let items_obj = items_schema
        .as_object()
        .ok_or_else(|| format!("{category}.{event_type}::{key}.items must be a JSON object schema"))?;
    if let Some(ty) = items_obj.get("type") {
        let key_with_index = format!("{key}[{index}]");
        check_type(category, event_type, &key_with_index, ty, item)?;
    }
    if let Some(enum_values) = items_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(item) {
            return Err(format!(
                "{category}.{event_type}::{key}[{index}] value {item} not in enum {enum_values:?}"
            ));
        }
    }
    Ok(())
}

/// **M5-A2**: validate a single nested-object property as a mini-schema
/// (type + enum + minimum + maximum). Used for sub-structs like
/// environment.signal_aggregated.signal.{schema_version,active_hazards}.
/// Supports recursion one level deep.
fn check_nested_property(
    category: &str,
    event_type: &str,
    parent_key: &str,
    nested_key: &str,
    nested_schema: &Value,
    value: &Value,
) -> ValidationResult {
    let nested_obj = nested_schema
        .as_object()
        .ok_or_else(|| format!("{category}.{event_type}::{parent_key}.{nested_key} schema must be an object"))?;
    let qualified_key = format!("{parent_key}.{nested_key}");
    if let Some(ty) = nested_obj.get("type") {
        check_type(category, event_type, &qualified_key, ty, value)?;
    }
    if let Some(enum_values) = nested_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(value) {
            return Err(format!(
                "{category}.{event_type}::{qualified_key} value {value} not in enum {enum_values:?}"
            ));
        }
    }
    if let Some(min) = nested_obj.get("minimum").and_then(|v| v.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n < min {
                return Err(format!(
                    "{category}.{event_type}::{qualified_key} value {n} < minimum {min}"
                ));
            }
        }
    }
    if let Some(max) = nested_obj.get("maximum").and_then(|v| v.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n > max {
                return Err(format!(
                    "{category}.{event_type}::{qualified_key} value {n} > maximum {max}"
                ));
            }
        }
    }
    if let Some(items_schema) = nested_obj.get("items") {
        if items_schema.is_object() {
            if let Some(arr) = value.as_array() {
                for (i, item) in arr.iter().enumerate() {
                    check_array_item(category, event_type, &qualified_key, i, items_schema, item)?;
                }
            }
        }
    }
    Ok(())
}

/// **M5-A1**: validate a single `oneOf` branch as a mini-schema (type +
/// enum). Returns `Ok` if the value satisfies the branch. The minimal
/// validator only supports `type` and `enum` constraints inside `oneOf`
/// branches; richer JSON-Schema features inside `oneOf` are not needed by
/// any M5 schema today.
fn check_one_of_branch(category: &str, event_type: &str, key: &str, branch: &Value, value: &Value) -> ValidationResult {
    let branch_obj = branch
        .as_object()
        .ok_or_else(|| format!("oneOf branch for {category}.{event_type}::{key} must be an object"))?;
    if let Some(ty) = branch_obj.get("type") {
        check_type(category, event_type, key, ty, value)?;
    }
    if let Some(enum_values) = branch_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(value) {
            return Err(format!(
                "{category}.{event_type}::{key} value {value} not in enum {enum_values:?}"
            ));
        }
    }
    Ok(())
}

fn check_type(category: &str, event_type: &str, key: &str, ty: &Value, value: &Value) -> ValidationResult {
    let types: Vec<&str> = match ty {
        Value::String(s) => vec![s.as_str()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };
    let matches = types.iter().any(|t| match *t {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_f64() || value.is_i64() || value.is_u64(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    });
    if !matches {
        return Err(format!(
            "{category}.{event_type}::{key} expected type {types:?}, got {value}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schemas_load_for_every_registered_event_type() {
        for (cat, ty) in [
            ("input", "intent_received"),
            ("equipment", "weapon_fired"),
            ("combat", "projectile_spawned"),
            ("combat", "wound_added"),
            ("actor", "inventory_dropped"),
            ("equipment", "alarm_registered"),
            ("terrain", "terrain_carved"),
            ("terrain", "terrain_penetration_threshold"),
            ("terrain", "terrain_dirty_region_batch"),
            ("terrain", "terrain_pixel_dislodged"),
            ("terrain", "hazard_contact_or_avoidance"),
            ("terrain", "anchor_material_result"),
            ("terrain", "terrain_material_probe"),
            ("terrain", "terrain_fill_or_repair"),
            // M4 schemas
            ("system", "run_started"),
            ("system", "run_finished"),
            ("system", "category_baseline"),
            ("determinism", "sim_checksum"),
            ("determinism", "first_divergence"),
            ("snapshot", "snapshot_actor"),
            ("snapshot", "snapshot_inventory"),
            ("snapshot", "snapshot_terrain_chunk"),
            ("snapshot", "snapshot_terrain_summary"),
            ("snapshot", "snapshot_chassis"),
            // M4 § M9 firehose surface placeholders.
            ("snapshot", "snapshot_hazard_grid"),
            ("snapshot", "snapshot_affliction"),
            ("snapshot", "snapshot_armor_layer"),
            ("snapshot", "snapshot_atmospherics"),
            ("snapshot", "snapshot_environment_signal"),
            ("snapshot", "snapshot_armor"),
            ("snapshot", "snapshot_internal"),
            ("snapshot", "snapshot_concussion"),
            ("snapshot", "snapshot_fluid"),
            ("snapshot", "snapshot_origin"),
            ("snapshot", "snapshot_shield"),
            ("snapshot", "snapshot_thermal"),
            // M5 deep-damage event-surface lock — armor.* family.
            ("armor", "layer_hp_changed"),
            ("armor", "layer_critical"),
            ("armor", "layer_destroyed"),
            ("armor", "all_layers_destroyed"),
            ("armor", "chunked_off"),
            ("armor", "debris_spawned"),
            ("armor", "repaired"),
            ("armor", "angle_deflection_calculated"),
            ("armor", "ricochet"),
            ("armor", "spalling"),
            ("armor", "penetration_ray_traversed"),
            ("armor", "he_overpressure_wave"),
            ("armor", "heat_jet_penetrated"),
            ("armor", "heat_jet_pre_detonated_by_era"),
            ("armor", "apfsds_penetrated"),
            ("armor", "era_panel_detonated"),
            ("armor", "schurzen_pre_detonated"),
            ("armor", "multi_hit_degradation"),
            ("armor", "reactive_armor_consumed"),
            // M5 internal.* family.
            ("internal", "organ_damaged"),
            ("internal", "organ_destroyed"),
            ("internal", "organ_failure_cascade"),
            ("internal", "circuit_damaged"),
            ("internal", "circuit_destroyed"),
            ("internal", "circuit_failure_cascade"),
            // M5 concussion.* + internal_shock.*.
            ("concussion", "dose_changed"),
            ("concussion", "band_changed"),
            ("concussion", "ko_threshold_crossed"),
            ("concussion", "recovered"),
            ("internal_shock", "dose_changed"),
            ("internal_shock", "module_damaged"),
            // M5 fluid.*.
            ("fluid", "leak_started"),
            ("fluid", "leak_rate_changed"),
            ("fluid", "reservoir_warning"),
            ("fluid", "reservoir_critical"),
            ("fluid", "reservoir_empty"),
            ("fluid", "ignition"),
            ("fluid", "ground_splatter_spawned"),
            ("fluid", "leak_stopped"),
            ("fluid", "refilled"),
            // M5 origin.*.
            ("origin", "shot_force_feedback"),
            ("origin", "g_load_dose_changed"),
            ("origin", "helmet_breach"),
            ("origin", "oxygen_supply_changed"),
            // M5 hazard.*.
            ("hazard", "spawned"),
            ("hazard", "spread"),
            ("hazard", "actor_contact"),
            ("hazard", "tick"),
            ("hazard", "dissipated"),
            // M5 affliction.*.
            ("affliction", "applied"),
            ("affliction", "tick"),
            ("affliction", "cleared"),
            ("affliction", "escalated"),
            // M5 atmos.*.
            ("atmos", "pressure_changed"),
            ("atmos", "temperature_changed"),
            ("atmos", "gas_released"),
            ("atmos", "breach_detected"),
            ("atmos", "combustion_ignition"),
            ("atmos", "phase_transition"),
            ("atmos", "pipe_flow"),
            ("atmos", "pipe_freeze"),
            ("atmos", "pipe_rupture"),
            ("atmos", "electrolysis_started"),
            // M5 shield.*.
            ("shield", "hit"),
            ("shield", "depleted"),
            ("shield", "regen_started"),
            ("shield", "regen_completed"),
            ("shield", "disrupted"),
            // M5 environment.*.
            ("environment", "signal_delta"),
            ("environment", "signal_aggregated"),
            // M5 thermal.*.
            ("thermal", "signature_changed"),
            ("thermal", "heat_exchanged"),
            ("thermal", "material_phase_change"),
            // M5 combat.projectile_hit_mo expanded payload.
            ("combat", "projectile_hit_mo"),
            // M5 audio.event_requested (M5 mandate, M5-A1).
            ("audio", "event_requested"),
            // M5-A2: combat melee + explosive hit_mo siblings.
            ("combat", "melee_hit_mo"),
            ("combat", "explosive_hit_mo"),
            // M6 actor / combat / equipment / inventory / perception / squad.
            ("actor", "action_rejected"),
            ("actor", "climb_started"),
            ("actor", "dive_started"),
            ("actor", "facing_changed"),
            ("actor", "lean_changed"),
            ("actor", "slide_started"),
            ("actor", "stamina_changed"),
            ("actor", "stance_changed"),
            ("actor", "vault_started"),
            ("combat", "knife_throw_landed"),
            ("combat", "knife_throw_started"),
            ("combat", "stealth_kill_executed"),
            ("equipment", "beacon_dropped"),
            ("equipment", "bipod_deployed"),
            ("equipment", "bipod_stowed"),
            ("equipment", "drill_overheated"),
            ("equipment", "fire_mode_cycled"),
            ("equipment", "grenade_cooked"),
            ("equipment", "grenade_detonated"),
            ("equipment", "grenade_thrown"),
            ("equipment", "item_dropped"),
            ("equipment", "item_picked_up"),
            ("equipment", "magazine_changed"),
            ("equipment", "melee_swing"),
            ("equipment", "sensor_pulse_fired"),
            ("equipment", "shell_ejected"),
            ("equipment", "suppressor_attached"),
            ("equipment", "tool_broken"),
            ("equipment", "tool_repaired"),
            ("equipment", "tool_used"),
            ("equipment", "weapon_swap_completed"),
            ("equipment", "weapon_swap_started"),
            ("inventory", "tank_slot_reserved"),
            ("inventory", "weight_changed"),
            ("perception", "actor_signal"),
            ("perception", "footstep_emitted"),
            ("perception", "occlusion_applied"),
            ("perception", "stealth_meter_changed"),
            ("squad", "command_issued"),
            ("squad", "member_added"),
            ("squad", "waypoint_marked"),
            // **M9** reactor defense + 5-tier integrity event surface.
            ("mission", "reactor_hp_changed"),
            ("mission", "reactor_destroyed"),
            ("mission", "reactor_pressure_state_changed"),
            ("mission", "timer_warning_threshold"),
            ("terrain", "material_state_changed"),
            ("terrain", "pixel_removed"),
            ("terrain", "cascade_triggered"),
            ("terrain", "debris_spawned"),
            // M9 audit round-3 fix gaps 1+2 — utility-scored target selection
            // and per-guard path invalidation.
            ("ai", "target_scored"),
            ("ai", "path_invalidated"),
        ] {
            let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
            let _parsed_value: serde_json::Value =
                serde_json::from_str(raw).unwrap_or_else(|e| panic!("schema json parse error for {cat}.{ty}: {e}"));
        }
    }

    #[test]
    fn terrain_carved_event_validates_minimum_payload() {
        let payload = json!({
            "bbox": { "min": [0, 0], "max": [10, 10] },
            "count": 12u32,
            "removed_count": 12u32,
            "debris_count": 12u32,
            "material_ids": [1u32],
        });
        validate_event_payload("terrain", "terrain_carved", &payload).expect("valid payload");
    }

    #[test]
    fn terrain_penetration_threshold_event_validates() {
        let payload = json!({
            "projectile_id": 7u32,
            "material_id": 1u32,
            "passed": true,
            "impulse_squared": 256.0,
            "integrity_squared": 100.0,
        });
        validate_event_payload("terrain", "terrain_penetration_threshold", &payload).expect("valid");
    }

    #[test]
    fn unknown_event_type_is_ok_by_default() {
        let payload = json!({});
        assert!(validate_event_payload("not", "registered", &payload).is_ok());
    }

    #[test]
    fn validates_input_intent_received_required_fields() {
        let mut payload = json!({
            "actor": 1,
            "source": "cfctl",
            "move_x": 0.0,
            "aim_x": 1.0,
            "aim_y": 0.0,
            "jump": false,
            "fire": false,
            "reload": false,
        });
        assert!(validate_event_payload("input", "intent_received", &payload).is_ok());
        payload.as_object_mut().unwrap().remove("actor");
        let err = validate_event_payload("input", "intent_received", &payload).unwrap_err();
        assert!(err.contains("`actor` missing"), "got: {err}");
    }

    #[test]
    fn validates_projectile_spawned_array_arity() {
        let bad = json!({
            "id": 1,
            "owner": 2,
            "origin": [0.0],
            "velocity": [1.0, 0.0],
            "damage": 12.0,
        });
        let err = validate_event_payload("combat", "projectile_spawned", &bad).unwrap_err();
        assert!(err.contains("origin"), "got: {err}");
    }

    /// **M5**: per-spec sample armor.layer_destroyed payload validates against
    /// the envelope-shaped schema (the validator must walk into
    /// `properties.payload` to find required/properties).
    #[test]
    fn m5_armor_layer_destroyed_payload_validates() {
        let payload = json!({
            "item_id": 12,
            "zone": "torso",
            "layer": "External",
            "breach_kind": "punctured",
        });
        validate_event_payload("armor", "layer_destroyed", &payload).expect("valid payload");
    }

    /// **M5**: an additive extra field (`bound_zone`) is accepted (the schema
    /// declares `additionalProperties: true` at the payload level so producer
    /// extensions don't bump the envelope).
    #[test]
    fn m5_armor_layer_destroyed_accepts_additive_payload_extension() {
        let payload = json!({
            "item_id": 12,
            "zone": "torso",
            "layer": "External",
            "breach_kind": "punctured",
            "bound_zone": "torso_front",
        });
        validate_event_payload("armor", "layer_destroyed", &payload).expect("additive ok");
    }

    /// **M5**: a missing required payload field fails validation.
    #[test]
    fn m5_armor_layer_destroyed_rejects_missing_breach_kind() {
        let payload = json!({
            "item_id": 12,
            "zone": "torso",
            "layer": "External",
        });
        let err = validate_event_payload("armor", "layer_destroyed", &payload).unwrap_err();
        assert!(err.contains("breach_kind"), "got: {err}");
    }

    /// **M5-A1**: per-family happy-path round trip — one event per family
    /// validates with a representative payload. Closes the test-coverage gap
    /// flagged by the validator audit.
    #[test]
    fn m5_per_family_happy_path() {
        validate_event_payload(
            "armor",
            "layer_hp_changed",
            &json!({
                "actor_id": 1,
                "item_id": 10,
                "zone": "torso",
                "layer": "External",
                "from": 50.0,
                "to": 30.0,
                "cause": "kinetic_round",
                "ap_factor": 0.7,
            }),
        )
        .expect("armor.layer_hp_changed valid");
        validate_event_payload(
            "internal",
            "organ_damaged",
            &json!({
                "actor_id": 1,
                "organ_id": "heart",
                "organ_kind": "vital",
                "from_hp": 100.0,
                "to_hp": 60.0,
                "cause": "kinetic_pierce",
                "source_hit_event_id": "run:42:7",
            }),
        )
        .expect("internal.organ_damaged valid");
        validate_event_payload(
            "concussion",
            "band_changed",
            &json!({
                "actor_id": 1,
                "from_band": "Mild",
                "to_band": "Moderate",
                "dose": 45.0,
            }),
        )
        .expect("concussion.band_changed valid");
        validate_event_payload(
            "internal_shock",
            "dose_changed",
            &json!({
                "actor_id": 2,
                "from_dose": 10.0,
                "to_dose": 40.0,
                "source_event_id": "run:42:9",
            }),
        )
        .expect("internal_shock.dose_changed valid");
        validate_event_payload(
            "fluid",
            "leak_started",
            &json!({
                "actor_id": 1,
                "fluid_kind": "oil",
                "source_module_id": "oil_reservoir",
                "leak_rate": 0.5,
                "position": [10.0, 20.0],
            }),
        )
        .expect("fluid.leak_started valid");
        validate_event_payload(
            "origin",
            "g_load_dose_changed",
            &json!({
                "actor_id": 1,
                "from_dose": 0.0,
                "to_dose": 4.5,
                "source": "fall",
            }),
        )
        .expect("origin.g_load_dose_changed valid");
        validate_event_payload(
            "hazard",
            "spawned",
            &json!({
                "hazard_id": "h-1",
                "kind": "fire",
                "position": [5.0, 5.0],
                "intensity": 0.8,
                "source_event_id": "run:42:11",
            }),
        )
        .expect("hazard.spawned valid");
        validate_event_payload(
            "affliction",
            "applied",
            &json!({
                "actor_id": 1,
                "kind": "blinded",
                "source_event_id": "run:42:13",
                "expected_duration_ticks": 90,
                "severity_0_1": 0.6,
            }),
        )
        .expect("affliction.applied (with new `blinded` kind) valid");
        validate_event_payload(
            "atmos",
            "gas_released",
            &json!({
                "atm_id": "atm-1",
                "gas": "H2",
                "moles": 3.5,
                "source": "electrolysis",
                "ignition_risk": 0.4,
            }),
        )
        .expect("atmos.gas_released valid");
        validate_event_payload(
            "shield",
            "hit",
            &json!({
                "actor_id": 1,
                "hp_before": 100.0,
                "hp_after": 75.0,
                "cause": "kinetic_round",
            }),
        )
        .expect("shield.hit valid");
        validate_event_payload(
            "environment",
            "signal_delta",
            &json!({
                "actor_id": 1,
                "slice": "thermal",
                "from": 295.0,
                "to": 310.0,
                "tick": 100,
            }),
        )
        .expect("environment.signal_delta valid");
        validate_event_payload(
            "thermal",
            "material_phase_change",
            &json!({
                "material_id": 7,
                "from_phase": "solid",
                "to_phase": "liquid",
                "position": [1.0, 2.0],
                "latent_heat_consumed_j": 12345.0,
            }),
        )
        .expect("thermal.material_phase_change valid");
        validate_event_payload(
            "combat",
            "projectile_hit_mo",
            &json!({
                "shooter_id": 1,
                "weapon_id": 5,
                "projectile_id": 99,
                "target_id": 2,
                "hit_zone": "torso",
                "impact_point": [10.0, 20.0],
                "impact_normal": [0.0, 1.0],
                "impact_impulse": 50.0,
                "impact_energy_j": 1200.0,
                "ap_factor": 0.5,
                "ap_round_tier": "standard",
                "material_at_impact": 1,
                "surface_kind": "armor_external",
                "armor_effective_hardness": 0.8,
                "armor_absorbed_dmg": 30.0,
                "passthrough_dmg": 20.0,
                "damage_kind": "kinetic",
                "hp_before": 100.0,
                "hp_after": 80.0,
                "damage_amount": 20.0,
                "pierced_armor": false,
                "parent_hit_event_id": "run:42:5",
            }),
        )
        .expect("combat.projectile_hit_mo valid");
        validate_event_payload(
            "audio",
            "event_requested",
            &json!({
                "kind": "material_state",
                "material": "metal",
                "impact_state": "pristine_hit",
                "surface_kind": "armor_external",
                "damage_kind": "kinetic",
                "source_event_id": "run:42:5",
            }),
        )
        .expect("audio.event_requested valid");
        validate_event_payload(
            "combat",
            "melee_hit_mo",
            &json!({
                "attacker_id": 1,
                "melee_weapon_id": 8,
                "melee_kind": "knife_stab",
                "target_id": 2,
                "hit_zone": "torso",
                "impact_point": [3.0, 4.0],
                "impact_normal": [0.0, 1.0],
                "impact_impulse": 5.0,
                "impact_energy_j": 80.0,
                "material_at_impact": 1,
                "surface_kind": "flesh",
                "armor_effective_hardness": 0.0,
                "armor_absorbed_dmg": 0.0,
                "passthrough_dmg": 25.0,
                "damage_kind": "kinetic",
                "hp_before": 100.0,
                "hp_after": 75.0,
                "damage_amount": 25.0,
                "pierced_armor": false,
                "parent_hit_event_id": "run:42:20",
            }),
        )
        .expect("combat.melee_hit_mo valid");
        validate_event_payload(
            "combat",
            "explosive_hit_mo",
            &json!({
                "attacker_id": 1,
                "weapon_id": 12,
                "explosive_kind": "frag",
                "target_id": 2,
                "hit_zone": "torso",
                "epicenter": [10.0, 10.0],
                "range_to_epicenter_m": 1.5,
                "blast_radius_m": 4.0,
                "impact_impulse": 200.0,
                "impact_energy_j": 5000.0,
                "overpressure_pa": 150000.0,
                "material_at_impact": 1,
                "surface_kind": "armor_external",
                "armor_effective_hardness": 0.6,
                "armor_absorbed_dmg": 20.0,
                "passthrough_dmg": 60.0,
                "damage_kind": "thermal",
                "hp_before": 100.0,
                "hp_after": 40.0,
                "damage_amount": 60.0,
                "pierced_armor": false,
                "parent_hit_event_id": "run:42:30",
            }),
        )
        .expect("combat.explosive_hit_mo valid");
    }

    /// **M5-A2**: `applied_afflictions` array items now enforce the
    /// 23-affliction enum.
    #[test]
    fn m5_internal_failure_cascade_rejects_unknown_affliction() {
        let payload = json!({
            "actor_id": 1,
            "organ_id": "heart",
            "applied_afflictions": ["bleeding", "not_a_real_affliction"],
            "hp_drain_per_s": 1.0,
        });
        let err = validate_event_payload("internal", "organ_failure_cascade", &payload).unwrap_err();
        assert!(
            err.contains("applied_afflictions"),
            "expected error about applied_afflictions, got: {err}"
        );
    }

    /// **M5-A2**: `applied_afflictions` happy path with a real affliction.
    #[test]
    fn m5_internal_failure_cascade_accepts_known_affliction() {
        let payload = json!({
            "actor_id": 1,
            "organ_id": "lungs_left",
            "applied_afflictions": ["bleeding", "hypoxic", "concussed"],
            "hp_drain_per_s": 1.5,
        });
        validate_event_payload("internal", "organ_failure_cascade", &payload).expect("valid applied_afflictions array");
    }

    /// **M5-A2**: nested-object recursion validates
    /// environment.signal_aggregated.signal.{schema_version, active_hazards}.
    /// A missing required nested field is rejected.
    #[test]
    fn m5_environment_signal_aggregated_rejects_missing_signal_field() {
        let payload = json!({
            "actor_id": 1,
            "tick": 100,
            "signal": {
                "active_hazards": ["Hypoxic"]
            }
        });
        let err = validate_event_payload("environment", "signal_aggregated", &payload).unwrap_err();
        assert!(err.contains("schema_version"), "got: {err}");
    }

    /// **M5-A2**: nested-object recursion rejects unknown HazardClass enum
    /// values in active_hazards array.
    #[test]
    fn m5_environment_signal_aggregated_rejects_bad_hazard_class() {
        let payload = json!({
            "actor_id": 1,
            "tick": 100,
            "signal": {
                "schema_version": 1,
                "active_hazards": ["Hypoxic", "NotARealHazard"]
            }
        });
        let err = validate_event_payload("environment", "signal_aggregated", &payload).unwrap_err();
        assert!(
            err.contains("active_hazards") || err.contains("NotARealHazard"),
            "got: {err}"
        );
    }

    /// **M5-A2**: nested-object recursion accepts valid signal sub-struct.
    #[test]
    fn m5_environment_signal_aggregated_accepts_valid_signal() {
        let payload = json!({
            "actor_id": 1,
            "tick": 100,
            "signal": {
                "schema_version": 1,
                "active_hazards": ["Hypoxic", "Hyperthermic", "Radiation"]
            }
        });
        validate_event_payload("environment", "signal_aggregated", &payload).expect("valid signal");
    }

    /// **M5-A2**: concussion.dose_changed `to_dose: 100.0` is exactly at the
    /// new locked maximum and should pass.
    #[test]
    fn m5_concussion_dose_changed_accepts_max_dose() {
        let payload = json!({
            "actor_id": 1,
            "from_dose": 99.0,
            "to_dose": 100.0,
            "source_event_id": "run:42:7",
            "origin_id": "Human",
        });
        validate_event_payload("concussion", "dose_changed", &payload).expect("dose=100 accepted");
    }

    /// **M5-A2**: concussion.dose_changed `to_dose: 100.1` exceeds locked
    /// maximum and is rejected.
    #[test]
    fn m5_concussion_dose_changed_rejects_over_max_dose() {
        let payload = json!({
            "actor_id": 1,
            "from_dose": 50.0,
            "to_dose": 100.1,
            "source_event_id": "run:42:7",
            "origin_id": "Human",
        });
        let err = validate_event_payload("concussion", "dose_changed", &payload).unwrap_err();
        assert!(err.contains("100"), "expected error about maximum 100, got: {err}");
    }

    /// **M5-A2**: armor.ricochet `ricochet_probability: 1.5` exceeds locked
    /// maximum 1.0 and is rejected.
    #[test]
    fn m5_armor_ricochet_rejects_probability_over_one() {
        let payload = json!({
            "impact_angle": 0.5,
            "ricochet_probability": 1.5,
            "was_ricocheted": false,
            "deflection_vector": [0.0, 1.0],
        });
        let err = validate_event_payload("armor", "ricochet", &payload).unwrap_err();
        assert!(err.contains("ricochet_probability"), "got: {err}");
    }

    /// **M5-A2**: armor.spalling.fragment_count is locked to 1..3 per spec.
    #[test]
    fn m5_armor_spalling_rejects_fragment_count_out_of_range() {
        let payload_high = json!({
            "item_id": 1,
            "zone": "torso",
            "layer": "External",
            "fragment_count": 99,
            "damage_per_fragment": 10.0,
            "cause_event_id": "run:42:1",
        });
        let err = validate_event_payload("armor", "spalling", &payload_high).unwrap_err();
        assert!(err.contains("fragment_count"), "got: {err}");
        let payload_low = json!({
            "item_id": 1,
            "zone": "torso",
            "layer": "External",
            "fragment_count": 0,
            "damage_per_fragment": 10.0,
            "cause_event_id": "run:42:1",
        });
        let err = validate_event_payload("armor", "spalling", &payload_low).unwrap_err();
        assert!(err.contains("fragment_count"), "got: {err}");
    }

    /// **M5-A2**: hazard.spread now requires hazard_id for M10 cause-chain
    /// integrity.
    #[test]
    fn m5_hazard_spread_requires_hazard_id() {
        let payload = json!({
            "from_pos": [0.0, 0.0],
            "to_pos": [1.0, 0.0],
            "kind": "fire",
            "intensity": 0.5,
            "rate": 0.1,
        });
        let err = validate_event_payload("hazard", "spread", &payload).unwrap_err();
        assert!(err.contains("hazard_id"), "got: {err}");
    }

    /// **M5-A2**: concussion.ko_threshold_crossed.ko_duration_s is now
    /// locked to 5..10 per spec ("full blackout 5-10s").
    #[test]
    fn m5_concussion_ko_threshold_crossed_rejects_out_of_range_duration() {
        let payload_short = json!({
            "actor_id": 1,
            "ko_duration_s": 2.0,
        });
        let err = validate_event_payload("concussion", "ko_threshold_crossed", &payload_short).unwrap_err();
        assert!(err.contains("ko_duration_s"), "got: {err}");
        let payload_long = json!({
            "actor_id": 1,
            "ko_duration_s": 30.0,
        });
        let err = validate_event_payload("concussion", "ko_threshold_crossed", &payload_long).unwrap_err();
        assert!(err.contains("ko_duration_s"), "got: {err}");
    }

    /// **M5-A1**: combat.projectile_hit_mo payload now requires
    /// `parent_hit_event_id` instead of the envelope-colliding
    /// `parent_event_id`.
    #[test]
    fn m5_combat_projectile_hit_mo_rejects_envelope_named_parent() {
        let payload = json!({
            "shooter_id": 1,
            "weapon_id": 5,
            "projectile_id": 99,
            "target_id": 2,
            "hit_zone": "torso",
            "impact_point": [10.0, 20.0],
            "impact_normal": [0.0, 1.0],
            "impact_impulse": 50.0,
            "impact_energy_j": 1200.0,
            "ap_factor": 0.5,
            "ap_round_tier": "standard",
            "material_at_impact": 1,
            "surface_kind": "armor_external",
            "armor_effective_hardness": 0.8,
            "armor_absorbed_dmg": 30.0,
            "passthrough_dmg": 20.0,
            "damage_kind": "kinetic",
            "hp_before": 100.0,
            "hp_after": 80.0,
            "damage_amount": 20.0,
            "pierced_armor": false,
            "parent_event_id": "run:42:5",
        });
        let err = validate_event_payload("combat", "projectile_hit_mo", &payload).unwrap_err();
        assert!(
            err.contains("parent_hit_event_id"),
            "expected error about parent_hit_event_id, got: {err}"
        );
    }

    /// **M5-A1**: Origin enum is locked — a non-canonical Origin string is
    /// rejected on concussion.dose_changed.
    #[test]
    fn m5_concussion_dose_changed_rejects_bad_origin() {
        let payload = json!({
            "actor_id": 1,
            "from_dose": 10.0,
            "to_dose": 20.0,
            "source_event_id": "run:42:7",
            "origin_id": "Construct",
        });
        let result = validate_event_payload("concussion", "dose_changed", &payload);
        assert!(result.is_err(), "expected rejection of non-canonical Origin");
    }

    /// **M5-A1**: payload enum mismatch on `zone` is rejected.
    #[test]
    fn m5_armor_layer_destroyed_rejects_bad_zone_enum() {
        let payload = json!({
            "item_id": 12,
            "zone": "not_a_zone",
            "layer": "External",
            "breach_kind": "punctured",
        });
        let err = validate_event_payload("armor", "layer_destroyed", &payload).unwrap_err();
        assert!(err.contains("zone"), "got: {err}");
    }

    /// **M9** (audit round-3 fix gap 1) — ai.target_scored validates a
    /// representative utility-scored target payload (one player candidate +
    /// one reactor candidate, player chosen, full weights breakdown).
    #[test]
    fn m9_ai_target_scored_validates_happy_path() {
        let payload = json!({
            "actor": 11,
            "target_actor": 7,
            "chosen_id": "7",
            "score": 1.42,
            "candidates": [
                {
                    "id": "7",
                    "kind": "player",
                    "actor_id": 7,
                    "distance": 30.0,
                    "has_los": true,
                    "is_player": true,
                    "is_high_value_static": false,
                    "score": 1.42,
                    "reason": "player_aggressive_los",
                },
                {
                    "id": "reactor_alpha",
                    "kind": "reactor",
                    "reactor_id": "reactor_alpha",
                    "distance": 80.0,
                    "has_los": true,
                    "is_player": false,
                    "is_high_value_static": true,
                    "score": 1.10,
                    "reason": "defensive_value",
                },
            ],
            "rationale": "player_aggressive: player_aggressive_los",
            "weights": {
                "proximity": 1.0,
                "los": 1.0,
                "threat": 0.7,
                "value": 0.5,
            },
        });
        validate_event_payload("ai", "target_scored", &payload).expect("ai.target_scored valid");
    }

    /// **M9** (audit round-3 fix gap 1) — missing required field is rejected.
    #[test]
    fn m9_ai_target_scored_requires_chosen_id() {
        let payload = json!({
            "actor": 11,
            "target_actor": 7,
            "candidates": [],
            "rationale": "no_candidates",
        });
        let err = validate_event_payload("ai", "target_scored", &payload).unwrap_err();
        assert!(err.contains("chosen_id"), "got: {err}");
    }

    /// **M9** (audit round-3 fix gap 2) — ai.path_invalidated validates a
    /// representative payload (guard's planned pursuit line crosses a
    /// freshly-carved bbox).
    #[test]
    fn m9_ai_path_invalidated_validates_happy_path() {
        let payload = json!({
            "actor": 9,
            "actor_id": 9,
            "bbox": { "min": [10.0, 20.0], "max": [40.0, 50.0] },
            "old_path": [[5.0, 5.0], [60.0, 60.0]],
            "reason": "terrain_dirty",
            "fraction_of_path_dirty": 0.375,
        });
        validate_event_payload("ai", "path_invalidated", &payload).expect("ai.path_invalidated valid");
    }

    /// **M9** (audit round-3 fix gap 2) — missing required `bbox` is rejected.
    #[test]
    fn m9_ai_path_invalidated_requires_bbox() {
        let payload = json!({
            "actor": 9,
            "actor_id": 9,
            "old_path": [[5.0, 5.0]],
            "reason": "terrain_dirty",
            "fraction_of_path_dirty": 0.1,
        });
        let err = validate_event_payload("ai", "path_invalidated", &payload).unwrap_err();
        assert!(err.contains("bbox"), "got: {err}");
    }

    /// **M5**: every registered M5 schema declares `schema_version: "0.1"` as
    /// a const property — proves the M5 conformance contract per the spec's
    /// "each schema declares schema_version=\"0.1\" matching the M4 locked
    /// envelope" scenario.
    #[test]
    fn m5_schemas_declare_schema_version_v0_1() {
        let pairs: &[(&str, &str)] = &[
            ("armor", "layer_hp_changed"),
            ("armor", "layer_critical"),
            ("armor", "layer_destroyed"),
            ("armor", "all_layers_destroyed"),
            ("armor", "chunked_off"),
            ("armor", "debris_spawned"),
            ("armor", "repaired"),
            ("armor", "angle_deflection_calculated"),
            ("armor", "ricochet"),
            ("armor", "spalling"),
            ("armor", "penetration_ray_traversed"),
            ("armor", "he_overpressure_wave"),
            ("armor", "heat_jet_penetrated"),
            ("armor", "heat_jet_pre_detonated_by_era"),
            ("armor", "apfsds_penetrated"),
            ("armor", "era_panel_detonated"),
            ("armor", "schurzen_pre_detonated"),
            ("armor", "multi_hit_degradation"),
            ("armor", "reactive_armor_consumed"),
            ("internal", "organ_damaged"),
            ("internal", "organ_destroyed"),
            ("internal", "organ_failure_cascade"),
            ("internal", "circuit_damaged"),
            ("internal", "circuit_destroyed"),
            ("internal", "circuit_failure_cascade"),
            ("concussion", "dose_changed"),
            ("concussion", "band_changed"),
            ("concussion", "ko_threshold_crossed"),
            ("concussion", "recovered"),
            ("internal_shock", "dose_changed"),
            ("internal_shock", "module_damaged"),
            ("fluid", "leak_started"),
            ("fluid", "leak_rate_changed"),
            ("fluid", "reservoir_warning"),
            ("fluid", "reservoir_critical"),
            ("fluid", "reservoir_empty"),
            ("fluid", "ignition"),
            ("fluid", "ground_splatter_spawned"),
            ("fluid", "leak_stopped"),
            ("fluid", "refilled"),
            ("origin", "shot_force_feedback"),
            ("origin", "g_load_dose_changed"),
            ("origin", "helmet_breach"),
            ("origin", "oxygen_supply_changed"),
            ("hazard", "spawned"),
            ("hazard", "spread"),
            ("hazard", "actor_contact"),
            ("hazard", "tick"),
            ("hazard", "dissipated"),
            ("affliction", "applied"),
            ("affliction", "tick"),
            ("affliction", "cleared"),
            ("affliction", "escalated"),
            ("atmos", "pressure_changed"),
            ("atmos", "temperature_changed"),
            ("atmos", "gas_released"),
            ("atmos", "breach_detected"),
            ("atmos", "combustion_ignition"),
            ("atmos", "phase_transition"),
            ("atmos", "pipe_flow"),
            ("atmos", "pipe_freeze"),
            ("atmos", "pipe_rupture"),
            ("atmos", "electrolysis_started"),
            ("shield", "hit"),
            ("shield", "depleted"),
            ("shield", "regen_started"),
            ("shield", "regen_completed"),
            ("shield", "disrupted"),
            ("environment", "signal_delta"),
            ("environment", "signal_aggregated"),
            ("thermal", "signature_changed"),
            ("thermal", "heat_exchanged"),
            ("thermal", "material_phase_change"),
            ("combat", "projectile_hit_mo"),
            ("audio", "event_requested"),
            ("combat", "melee_hit_mo"),
            ("combat", "explosive_hit_mo"),
            // M9 audit round-3 fix gaps 1+2 — both new AI schemas use the
            // canonical M4 envelope shape per the M5-locked contract.
            ("ai", "target_scored"),
            ("ai", "path_invalidated"),
        ];
        for (cat, ty) in pairs {
            let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
            let v: serde_json::Value = serde_json::from_str(raw).expect("schema is json");
            let sv = v
                .pointer("/properties/schema_version/const")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.schema_version.const"));
            assert_eq!(
                sv, "prototype-recorder-event.v0.1",
                "{cat}.{ty} schema_version must be canonical M4 envelope literal (got {sv})"
            );
            let cat_const = v
                .pointer("/properties/category/const")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.category.const"));
            assert_eq!(cat_const, *cat, "{cat}.{ty} category const mismatch");
            let ty_const = v
                .pointer("/properties/event_type/const")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.event_type.const"));
            assert_eq!(ty_const, *ty, "{cat}.{ty} event_type const mismatch");
        }
    }
}
