//! Compile-time embedded JSON schemas for per-event payload validation.
//!
//! Each `SCHEMA_*` constant is the raw JSON source for a single event-type
//! schema, included from `cf-replay/schemas/event/*.json` via
//! `include_str!`. The map between `(category, event_type)` and these
//! constants lives in `schemas_lookup.rs`; the validator that walks them
//! lives in `schemas_validate.rs`.

pub(crate) const SCHEMA_INPUT_INTENT_RECEIVED: &str = include_str!("../schemas/event/input_intent_received.json");
pub(crate) const SCHEMA_WEAPON_FIRED: &str = include_str!("../schemas/event/weapon_fired.json");
pub(crate) const SCHEMA_PROJECTILE_SPAWNED: &str = include_str!("../schemas/event/projectile_spawned.json");
pub(crate) const SCHEMA_WOUND_ADDED: &str = include_str!("../schemas/event/wound_added.json");
pub(crate) const SCHEMA_INVENTORY_DROPPED: &str = include_str!("../schemas/event/inventory_dropped.json");
pub(crate) const SCHEMA_ALARM_REGISTERED: &str = include_str!("../schemas/event/alarm_registered.json");
pub(crate) const SCHEMA_TERRAIN_CARVED: &str = include_str!("../schemas/event/terrain_carved.json");
pub(crate) const SCHEMA_TERRAIN_PENETRATION_THRESHOLD: &str =
    include_str!("../schemas/event/terrain_penetration_threshold.json");
pub(crate) const SCHEMA_TERRAIN_DIRTY_REGION_BATCH: &str =
    include_str!("../schemas/event/terrain_dirty_region_batch.json");
pub(crate) const SCHEMA_TERRAIN_CHUNK_MUTATED: &str = include_str!("../schemas/event/terrain_chunk_mutated.json");
pub(crate) const SCHEMA_TERRAIN_CHUNK_ACTIVE_REGION_CHANGED: &str =
    include_str!("../schemas/event/terrain_chunk_active_region_changed.json");
pub(crate) const SCHEMA_PERF_SAMPLE: &str = include_str!("../schemas/event/perf_sample.json");
pub(crate) const SCHEMA_NET_PROTOCOL_NEGOTIATED: &str = include_str!("../schemas/event/net_protocol_negotiated.json");
pub(crate) const SCHEMA_NET_ROLLBACK_WINDOW: &str = include_str!("../schemas/event/net_rollback_window.json");
pub(crate) const SCHEMA_NET_INPUT_RESENT_REDUNDANT: &str =
    include_str!("../schemas/event/net_input_resent_redundant.json");
pub(crate) const SCHEMA_NET_FEC_RECOVERED: &str = include_str!("../schemas/event/net_fec_recovered.json");
pub(crate) const SCHEMA_NET_NAT_TRAVERSAL_OUTCOME: &str =
    include_str!("../schemas/event/net_nat_traversal_outcome.json");
pub(crate) const SCHEMA_TERRAIN_PIXEL_DISLODGED: &str = include_str!("../schemas/event/terrain_pixel_dislodged.json");
pub(crate) const SCHEMA_HAZARD_CONTACT_OR_AVOIDANCE: &str =
    include_str!("../schemas/event/hazard_contact_or_avoidance.json");
pub(crate) const SCHEMA_ANCHOR_MATERIAL_RESULT: &str = include_str!("../schemas/event/anchor_material_result.json");
pub(crate) const SCHEMA_TERRAIN_MATERIAL_PROBE: &str = include_str!("../schemas/event/terrain_material_probe.json");
pub(crate) const SCHEMA_TERRAIN_FILL_OR_REPAIR: &str = include_str!("../schemas/event/terrain_fill_or_repair.json");
pub(crate) const SCHEMA_FORCED_REFRESH_REQUESTED: &str = include_str!("../schemas/event/forced_refresh_requested.json");
pub(crate) const SCHEMA_TERRAIN_DEBRIS_CAPPED: &str = include_str!("../schemas/event/debris_capped.json");
pub(crate) const SCHEMA_TERRAIN_TOOL_REFUSED: &str = include_str!("../schemas/event/tool_refused.json");
pub(crate) const SCHEMA_TERRAIN_TOOL_ACTION_STARTED: &str = include_str!("../schemas/event/tool_action_started.json");
pub(crate) const SCHEMA_EQUIPMENT_TOOL_ACTION_COMPLETED: &str =
    include_str!("../schemas/event/tool_action_completed.json");
pub(crate) const SCHEMA_TERRAIN_PATH_INVALIDATED: &str = include_str!("../schemas/event/path_invalidated.json");
pub(crate) const SCHEMA_MISSION_STARTED: &str = include_str!("../schemas/event/mission_started.json");
pub(crate) const SCHEMA_OBJECTIVE_STARTED: &str = include_str!("../schemas/event/objective_started.json");
pub(crate) const SCHEMA_OBJECTIVE_UPDATED: &str = include_str!("../schemas/event/objective_updated.json");
pub(crate) const SCHEMA_OBJECTIVE_COMPLETED: &str = include_str!("../schemas/event/objective_completed.json");
pub(crate) const SCHEMA_OBJECTIVE_FAILED: &str = include_str!("../schemas/event/objective_failed.json");
pub(crate) const SCHEMA_MISSION_RESOLVED: &str = include_str!("../schemas/event/mission_resolved.json");
pub(crate) const SCHEMA_AI_STATE_CHANGED: &str = include_str!("../schemas/event/ai_state_changed.json");
pub(crate) const SCHEMA_AI_PERCEPTION_SIGNAL: &str = include_str!("../schemas/event/ai_perception_signal.json");
pub(crate) const SCHEMA_AI_TACTIC_CHOSEN: &str = include_str!("../schemas/event/ai_tactic_chosen.json");
pub(crate) const SCHEMA_AI_MISSED_SHOT_REASON: &str = include_str!("../schemas/event/ai_missed_shot_reason.json");
pub(crate) const SCHEMA_AI_STUCK_STATE_CHANGED: &str = include_str!("../schemas/event/ai_stuck_state_changed.json");
pub(crate) const SCHEMA_AI_RECOVERY_ACTION: &str = include_str!("../schemas/event/ai_recovery_action.json");
pub(crate) const SCHEMA_SYSTEM_RUN_STARTED: &str = include_str!("../schemas/event/system_run_started.json");
pub(crate) const SCHEMA_SYSTEM_RUN_FINISHED: &str = include_str!("../schemas/event/system_run_finished.json");
pub(crate) const SCHEMA_SYSTEM_CATEGORY_BASELINE: &str = include_str!("../schemas/event/system_category_baseline.json");
pub(crate) const SCHEMA_DETERMINISM_SIM_CHECKSUM: &str = include_str!("../schemas/event/determinism_sim_checksum.json");
pub(crate) const SCHEMA_DETERMINISM_FIRST_DIVERGENCE: &str =
    include_str!("../schemas/event/determinism_first_divergence.json");
pub(crate) const SCHEMA_SNAPSHOT_BASELINE_EMITTED: &str =
    include_str!("../schemas/event/snapshot_baseline_emitted.json");
pub(crate) const SCHEMA_SNAPSHOT_DELTA_EMITTED: &str = include_str!("../schemas/event/snapshot_delta_emitted.json");
pub(crate) const SCHEMA_SAVE_COMPLETED: &str = include_str!("../schemas/event/save_completed.json");
pub(crate) const SCHEMA_SAVE_LOADED: &str = include_str!("../schemas/event/save_loaded.json");
pub(crate) const SCHEMA_SAVE_MIGRATED: &str = include_str!("../schemas/event/save_migrated.json");
pub(crate) const SCHEMA_LEDGER_CHAIN_VERIFIED: &str = include_str!("../schemas/event/ledger_chain_verified.json");
pub(crate) const SCHEMA_SNAPSHOT_ACTOR: &str = include_str!("../schemas/event/snapshot_actor.json");
pub(crate) const SCHEMA_SNAPSHOT_INVENTORY: &str = include_str!("../schemas/event/snapshot_inventory.json");
pub(crate) const SCHEMA_SNAPSHOT_TERRAIN_CHUNK: &str = include_str!("../schemas/event/snapshot_terrain_chunk.json");
pub(crate) const SCHEMA_SNAPSHOT_TERRAIN_SUMMARY: &str = include_str!("../schemas/event/snapshot_terrain_summary.json");
pub(crate) const SCHEMA_SNAPSHOT_CHASSIS: &str = include_str!("../schemas/event/snapshot_chassis.json");
pub(crate) const SCHEMA_SNAPSHOT_HAZARD_GRID: &str = include_str!("../schemas/event/snapshot_hazard_grid.json");
pub(crate) const SCHEMA_SNAPSHOT_AFFLICTION: &str = include_str!("../schemas/event/snapshot_affliction.json");
pub(crate) const SCHEMA_SNAPSHOT_ARMOR_LAYER: &str = include_str!("../schemas/event/snapshot_armor_layer.json");
pub(crate) const SCHEMA_SNAPSHOT_ATMOSPHERICS: &str = include_str!("../schemas/event/snapshot_atmospherics.json");
pub(crate) const SCHEMA_SNAPSHOT_ENVIRONMENT_SIGNAL: &str =
    include_str!("../schemas/event/snapshot_environment_signal.json");
pub(crate) const SCHEMA_SNAPSHOT_ARMOR: &str = include_str!("../schemas/event/snapshot_armor.json");
pub(crate) const SCHEMA_SNAPSHOT_INTERNAL: &str = include_str!("../schemas/event/snapshot_internal.json");
pub(crate) const SCHEMA_SNAPSHOT_CONCUSSION: &str = include_str!("../schemas/event/snapshot_concussion.json");
pub(crate) const SCHEMA_SNAPSHOT_FLUID: &str = include_str!("../schemas/event/snapshot_fluid.json");
pub(crate) const SCHEMA_SNAPSHOT_ORIGIN: &str = include_str!("../schemas/event/snapshot_origin.json");
pub(crate) const SCHEMA_SNAPSHOT_SHIELD: &str = include_str!("../schemas/event/snapshot_shield.json");
pub(crate) const SCHEMA_SNAPSHOT_THERMAL: &str = include_str!("../schemas/event/snapshot_thermal.json");
pub(crate) const SCHEMA_ARMOR_LAYER_HP_CHANGED: &str = include_str!("../schemas/event/armor_layer_hp_changed.json");
pub(crate) const SCHEMA_ARMOR_LAYER_CRITICAL: &str = include_str!("../schemas/event/armor_layer_critical.json");
pub(crate) const SCHEMA_ARMOR_LAYER_DESTROYED: &str = include_str!("../schemas/event/armor_layer_destroyed.json");
pub(crate) const SCHEMA_ARMOR_ALL_LAYERS_DESTROYED: &str =
    include_str!("../schemas/event/armor_all_layers_destroyed.json");
pub(crate) const SCHEMA_ARMOR_CHUNKED_OFF: &str = include_str!("../schemas/event/armor_chunked_off.json");
pub(crate) const SCHEMA_ARMOR_DEBRIS_SPAWNED: &str = include_str!("../schemas/event/armor_debris_spawned.json");
pub(crate) const SCHEMA_ARMOR_REPAIRED: &str = include_str!("../schemas/event/armor_repaired.json");
pub(crate) const SCHEMA_ARMOR_ANGLE_DEFLECTION_CALCULATED: &str =
    include_str!("../schemas/event/armor_angle_deflection_calculated.json");
pub(crate) const SCHEMA_ARMOR_RICOCHET: &str = include_str!("../schemas/event/armor_ricochet.json");
pub(crate) const SCHEMA_ARMOR_SPALLING: &str = include_str!("../schemas/event/armor_spalling.json");
pub(crate) const SCHEMA_ARMOR_PENETRATION_RAY_TRAVERSED: &str =
    include_str!("../schemas/event/armor_penetration_ray_traversed.json");
pub(crate) const SCHEMA_ARMOR_HE_OVERPRESSURE_WAVE: &str =
    include_str!("../schemas/event/armor_he_overpressure_wave.json");
pub(crate) const SCHEMA_ARMOR_HEAT_JET_PENETRATED: &str =
    include_str!("../schemas/event/armor_heat_jet_penetrated.json");
pub(crate) const SCHEMA_ARMOR_HEAT_JET_PRE_DETONATED_BY_ERA: &str =
    include_str!("../schemas/event/armor_heat_jet_pre_detonated_by_era.json");
pub(crate) const SCHEMA_ARMOR_APFSDS_PENETRATED: &str = include_str!("../schemas/event/armor_apfsds_penetrated.json");
pub(crate) const SCHEMA_ARMOR_ERA_PANEL_DETONATED: &str =
    include_str!("../schemas/event/armor_era_panel_detonated.json");
pub(crate) const SCHEMA_ARMOR_HEAT_JET_TRAVERSED: &str = include_str!("../schemas/event/armor_heat_jet_traversed.json");
pub(crate) const SCHEMA_ARMOR_APFSDS_LONG_ROD_THROUGH: &str =
    include_str!("../schemas/event/armor_apfsds_long_rod_through.json");
pub(crate) const SCHEMA_ARMOR_ERA_PRE_DETONATED: &str = include_str!("../schemas/event/armor_era_pre_detonated.json");
pub(crate) const SCHEMA_COLLISION_PROJECTILE_PAIR_CONTACT: &str =
    include_str!("../schemas/event/collision_projectile_pair_contact.json");
pub(crate) const SCHEMA_TERRAIN_STRUCTURAL_INTEGRITY_LOW: &str =
    include_str!("../schemas/event/terrain_structural_integrity_low.json");
pub(crate) const SCHEMA_TERRAIN_CAVE_IN_TRIGGERED: &str =
    include_str!("../schemas/event/terrain_cave_in_triggered.json");
pub(crate) const SCHEMA_TERRAIN_SUPPORT_BEAM_PLACED: &str =
    include_str!("../schemas/event/terrain_support_beam_placed.json");
pub(crate) const SCHEMA_TERRAIN_SUPPORT_BEAM_DESTROYED: &str =
    include_str!("../schemas/event/terrain_support_beam_destroyed.json");
pub(crate) const SCHEMA_TERRAIN_TERRAIN_CASCADE: &str = include_str!("../schemas/event/terrain_terrain_cascade.json");
pub(crate) const SCHEMA_TERRAIN_WALL_BULGING: &str = include_str!("../schemas/event/terrain_wall_bulging.json");
pub(crate) const SCHEMA_TERRAIN_WALL_CRACK_ADVANCED: &str =
    include_str!("../schemas/event/terrain_wall_crack_advanced.json");
pub(crate) const SCHEMA_TERRAIN_WALL_RUPTURE: &str = include_str!("../schemas/event/terrain_wall_rupture.json");
pub(crate) const SCHEMA_TERRAIN_BRACE_STRUT_PLACED: &str =
    include_str!("../schemas/event/terrain_brace_strut_placed.json");
pub(crate) const SCHEMA_WOUND_CREATED: &str = include_str!("../schemas/event/wound_created.json");
pub(crate) const SCHEMA_WOUND_ESCALATED: &str = include_str!("../schemas/event/wound_escalated.json");
pub(crate) const SCHEMA_WOUND_AGED: &str = include_str!("../schemas/event/wound_aged.json");
pub(crate) const SCHEMA_WOUND_SCABBED: &str = include_str!("../schemas/event/wound_scabbed.json");
pub(crate) const SCHEMA_WOUND_SCARRED: &str = include_str!("../schemas/event/wound_scarred.json");
pub(crate) const SCHEMA_TREATMENT_APPLIED: &str = include_str!("../schemas/event/treatment_applied.json");
pub(crate) const SCHEMA_TREATMENT_COMPLETED: &str = include_str!("../schemas/event/treatment_completed.json");
pub(crate) const SCHEMA_TREATMENT_FAILED: &str = include_str!("../schemas/event/treatment_failed.json");
pub(crate) const SCHEMA_TREATMENT_CANCELLED: &str = include_str!("../schemas/event/treatment_cancelled.json");
pub(crate) const SCHEMA_CARDIAC_ARRESTED: &str = include_str!("../schemas/event/cardiac_arrested.json");
pub(crate) const SCHEMA_CARDIAC_CPR_ROUND: &str = include_str!("../schemas/event/cardiac_cpr_round.json");
pub(crate) const SCHEMA_CARDIAC_DEFIB_ATTEMPTED: &str = include_str!("../schemas/event/cardiac_defib_attempted.json");
pub(crate) const SCHEMA_CARDIAC_RESTORED: &str = include_str!("../schemas/event/cardiac_restored.json");
pub(crate) const SCHEMA_CARDIAC_EXPIRED: &str = include_str!("../schemas/event/cardiac_expired.json");
pub(crate) const SCHEMA_SURGERY_PHASE_STARTED: &str = include_str!("../schemas/event/surgery_phase_started.json");
pub(crate) const SCHEMA_SURGERY_PHASE_COMPLETED: &str = include_str!("../schemas/event/surgery_phase_completed.json");
pub(crate) const SCHEMA_SURGERY_SKILL_CHECK: &str = include_str!("../schemas/event/surgery_skill_check.json");
pub(crate) const SCHEMA_SURGERY_COMPLETED: &str = include_str!("../schemas/event/surgery_completed.json");
pub(crate) const SCHEMA_SURGERY_FAILED: &str = include_str!("../schemas/event/surgery_failed.json");
pub(crate) const SCHEMA_SCAN_STARTED: &str = include_str!("../schemas/event/scan_started.json");
pub(crate) const SCHEMA_SCAN_COMPLETED: &str = include_str!("../schemas/event/scan_completed.json");
pub(crate) const SCHEMA_TRIAGE_QUEUE_CHANGED: &str = include_str!("../schemas/event/triage_queue_changed.json");
pub(crate) const SCHEMA_PATIENT_ASSESSED: &str = include_str!("../schemas/event/patient_assessed.json");
pub(crate) const SCHEMA_SCAR_ACQUIRED: &str = include_str!("../schemas/event/scar_acquired.json");
pub(crate) const SCHEMA_PHANTOM_LIMB_ACQUIRED: &str = include_str!("../schemas/event/phantom_limb_acquired.json");
pub(crate) const SCHEMA_PHANTOM_LIMB_PANIC_ATTACK: &str =
    include_str!("../schemas/event/phantom_limb_panic_attack.json");
pub(crate) const SCHEMA_MEMORY_LOSS_MINOR_ACQUIRED: &str =
    include_str!("../schemas/event/memory_loss_minor_acquired.json");
pub(crate) const SCHEMA_MEMORY_LOSS_MAJOR_ACQUIRED: &str =
    include_str!("../schemas/event/memory_loss_major_acquired.json");
pub(crate) const SCHEMA_AGE_YEAR_ADVANCED: &str = include_str!("../schemas/event/age_year_advanced.json");
pub(crate) const SCHEMA_AGE_RETIREMENT_OFFERED: &str = include_str!("../schemas/event/age_retirement_offered.json");
pub(crate) const SCHEMA_AGE_TERMINAL_ROLL: &str = include_str!("../schemas/event/age_terminal_roll.json");
pub(crate) const SCHEMA_PROSTHETIC_INSTALLED: &str = include_str!("../schemas/event/prosthetic_installed.json");
pub(crate) const SCHEMA_PROSTHETIC_MALFUNCTIONED: &str = include_str!("../schemas/event/prosthetic_malfunctioned.json");
pub(crate) const SCHEMA_PROSTHETIC_MAINTAINED: &str = include_str!("../schemas/event/prosthetic_maintained.json");
pub(crate) const SCHEMA_DISEASE_EXPOSED: &str = include_str!("../schemas/event/disease_exposed.json");
pub(crate) const SCHEMA_VETERAN_RETIRED: &str = include_str!("../schemas/event/veteran_retired.json");
pub(crate) const SCHEMA_M14J_ACTOR_VAULTED: &str = include_str!("../schemas/event/actor_vaulted.json");
pub(crate) const SCHEMA_M14J_ACTOR_WALL_JUMPED: &str = include_str!("../schemas/event/actor_wall_jumped.json");
pub(crate) const SCHEMA_M14J_GRAPPLE_FIRED: &str = include_str!("../schemas/event/grapple_fired.json");
pub(crate) const SCHEMA_M14J_GRAPPLE_EMBEDDED: &str = include_str!("../schemas/event/grapple_embedded.json");
pub(crate) const SCHEMA_M14J_ROPE_RELEASED: &str = include_str!("../schemas/event/rope_released.json");
pub(crate) const SCHEMA_M14J_ZIPLINE_DEPLOYED: &str = include_str!("../schemas/event/zipline_deployed.json");
pub(crate) const SCHEMA_M14J_ZIPLINE_CLIPPED: &str = include_str!("../schemas/event/zipline_clipped.json");
pub(crate) const SCHEMA_M14J_ACTOR_MOUNTED: &str = include_str!("../schemas/event/actor_mounted.json");
pub(crate) const SCHEMA_M14J_ACTOR_DISMOUNTED: &str = include_str!("../schemas/event/actor_dismounted.json");
pub(crate) const SCHEMA_M14J_SWIM_STROKE: &str = include_str!("../schemas/event/swim_stroke.json");
pub(crate) const SCHEMA_M14J_ACTOR_DROWNED: &str = include_str!("../schemas/event/actor_drowned.json");
pub(crate) const SCHEMA_ARMOR_SCHURZEN_PRE_DETONATED: &str =
    include_str!("../schemas/event/armor_schurzen_pre_detonated.json");
pub(crate) const SCHEMA_ARMOR_MULTI_HIT_DEGRADATION: &str =
    include_str!("../schemas/event/armor_multi_hit_degradation.json");
pub(crate) const SCHEMA_ARMOR_REACTIVE_ARMOR_CONSUMED: &str =
    include_str!("../schemas/event/armor_reactive_armor_consumed.json");
pub(crate) const SCHEMA_INTERNAL_ORGAN_DAMAGED: &str = include_str!("../schemas/event/internal_organ_damaged.json");
pub(crate) const SCHEMA_INTERNAL_ORGAN_DESTROYED: &str = include_str!("../schemas/event/internal_organ_destroyed.json");
pub(crate) const SCHEMA_INTERNAL_ORGAN_FAILURE_CASCADE: &str =
    include_str!("../schemas/event/internal_organ_failure_cascade.json");
pub(crate) const SCHEMA_INTERNAL_CIRCUIT_DAMAGED: &str = include_str!("../schemas/event/internal_circuit_damaged.json");
pub(crate) const SCHEMA_INTERNAL_CIRCUIT_DESTROYED: &str =
    include_str!("../schemas/event/internal_circuit_destroyed.json");
pub(crate) const SCHEMA_INTERNAL_CIRCUIT_FAILURE_CASCADE: &str =
    include_str!("../schemas/event/internal_circuit_failure_cascade.json");
pub(crate) const SCHEMA_CONCUSSION_DOSE_CHANGED: &str = include_str!("../schemas/event/concussion_dose_changed.json");
pub(crate) const SCHEMA_CONCUSSION_BAND_CHANGED: &str = include_str!("../schemas/event/concussion_band_changed.json");
pub(crate) const SCHEMA_CONCUSSION_KO_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/concussion_ko_threshold_crossed.json");
pub(crate) const SCHEMA_CONCUSSION_RECOVERED: &str = include_str!("../schemas/event/concussion_recovered.json");
pub(crate) const SCHEMA_INTERNAL_SHOCK_DOSE_CHANGED: &str =
    include_str!("../schemas/event/internal_shock_dose_changed.json");
pub(crate) const SCHEMA_INTERNAL_SHOCK_MODULE_DAMAGED: &str =
    include_str!("../schemas/event/internal_shock_module_damaged.json");
pub(crate) const SCHEMA_FLUID_LEAK_STARTED: &str = include_str!("../schemas/event/fluid_leak_started.json");
pub(crate) const SCHEMA_FLUID_LEAK_RATE_CHANGED: &str = include_str!("../schemas/event/fluid_leak_rate_changed.json");
pub(crate) const SCHEMA_FLUID_RESERVOIR_WARNING: &str = include_str!("../schemas/event/fluid_reservoir_warning.json");
pub(crate) const SCHEMA_FLUID_RESERVOIR_CRITICAL: &str = include_str!("../schemas/event/fluid_reservoir_critical.json");
pub(crate) const SCHEMA_FLUID_RESERVOIR_EMPTY: &str = include_str!("../schemas/event/fluid_reservoir_empty.json");
pub(crate) const SCHEMA_FLUID_IGNITION: &str = include_str!("../schemas/event/fluid_ignition.json");
pub(crate) const SCHEMA_FLUID_GROUND_SPLATTER_SPAWNED: &str =
    include_str!("../schemas/event/fluid_ground_splatter_spawned.json");
pub(crate) const SCHEMA_FLUID_LEAK_STOPPED: &str = include_str!("../schemas/event/fluid_leak_stopped.json");
pub(crate) const SCHEMA_FLUID_REFILLED: &str = include_str!("../schemas/event/fluid_refilled.json");
pub(crate) const SCHEMA_ORIGIN_SHOT_FORCE_FEEDBACK: &str =
    include_str!("../schemas/event/origin_shot_force_feedback.json");
pub(crate) const SCHEMA_ORIGIN_G_LOAD_DOSE_CHANGED: &str =
    include_str!("../schemas/event/origin_g_load_dose_changed.json");
pub(crate) const SCHEMA_ORIGIN_HELMET_BREACH: &str = include_str!("../schemas/event/origin_helmet_breach.json");
pub(crate) const SCHEMA_ORIGIN_OXYGEN_SUPPLY_CHANGED: &str =
    include_str!("../schemas/event/origin_oxygen_supply_changed.json");
pub(crate) const SCHEMA_HAZARD_SPAWNED: &str = include_str!("../schemas/event/hazard_spawned.json");
pub(crate) const SCHEMA_HAZARD_SPREAD: &str = include_str!("../schemas/event/hazard_spread.json");
pub(crate) const SCHEMA_HAZARD_ACTOR_CONTACT: &str = include_str!("../schemas/event/hazard_actor_contact.json");
pub(crate) const SCHEMA_HAZARD_TICK: &str = include_str!("../schemas/event/hazard_tick.json");
pub(crate) const SCHEMA_HAZARD_DISSIPATED: &str = include_str!("../schemas/event/hazard_dissipated.json");
pub(crate) const SCHEMA_AFFLICTION_APPLIED: &str = include_str!("../schemas/event/affliction_applied.json");
pub(crate) const SCHEMA_AFFLICTION_TICK: &str = include_str!("../schemas/event/affliction_tick.json");
pub(crate) const SCHEMA_AFFLICTION_CLEARED: &str = include_str!("../schemas/event/affliction_cleared.json");
pub(crate) const SCHEMA_AFFLICTION_ESCALATED: &str = include_str!("../schemas/event/affliction_escalated.json");
pub(crate) const SCHEMA_AFFLICTION_ENV_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/affliction_env_threshold_crossed.json");
pub(crate) const SCHEMA_AFFLICTION_ENV_CLEARED: &str =
    include_str!("../schemas/event/affliction_env_cleared.json");
pub(crate) const SCHEMA_AFFLICTION_ENV_SEVERITY_CHANGED: &str =
    include_str!("../schemas/event/affliction_env_severity_changed.json");
pub(crate) const SCHEMA_AFFLICTION_ENV_ORIGIN_IMMUNE: &str =
    include_str!("../schemas/event/affliction_env_origin_immune.json");
pub(crate) const SCHEMA_ANOMALY_ENTERED: &str = include_str!("../schemas/event/anomaly_entered.json");
pub(crate) const SCHEMA_ANOMALY_DAMAGE_APPLIED: &str = include_str!("../schemas/event/anomaly_damage_applied.json");
pub(crate) const SCHEMA_ARTIFACT_SPAWNED: &str = include_str!("../schemas/event/artifact_spawned.json");
pub(crate) const SCHEMA_ARTIFACT_PICKED_UP: &str = include_str!("../schemas/event/artifact_picked_up.json");
pub(crate) const SCHEMA_ARTIFACT_CARRIED_BONUS_APPLIED: &str =
    include_str!("../schemas/event/artifact_carried_bonus_applied.json");
pub(crate) const SCHEMA_ACTOR_DROWNING_STARTED: &str = include_str!("../schemas/event/actor_drowning_started.json");
pub(crate) const SCHEMA_ACTOR_DROWNING_LETHAL: &str = include_str!("../schemas/event/actor_drowning_lethal.json");
pub(crate) const SCHEMA_ACTOR_SWIM_STARTED: &str = include_str!("../schemas/event/actor_swim_started.json");
pub(crate) const SCHEMA_ACTOR_SWIM_ENDED: &str = include_str!("../schemas/event/actor_swim_ended.json");
pub(crate) const SCHEMA_ATMOS_PRESSURE_CHANGED: &str = include_str!("../schemas/event/atmos_pressure_changed.json");
pub(crate) const SCHEMA_ATMOS_TEMPERATURE_CHANGED: &str =
    include_str!("../schemas/event/atmos_temperature_changed.json");
pub(crate) const SCHEMA_ATMOS_GAS_RELEASED: &str = include_str!("../schemas/event/atmos_gas_released.json");
pub(crate) const SCHEMA_ATMOS_BREACH_DETECTED: &str = include_str!("../schemas/event/atmos_breach_detected.json");
pub(crate) const SCHEMA_ATMOS_COMBUSTION_IGNITION: &str =
    include_str!("../schemas/event/atmos_combustion_ignition.json");
pub(crate) const SCHEMA_ATMOS_PHASE_TRANSITION: &str = include_str!("../schemas/event/atmos_phase_transition.json");
pub(crate) const SCHEMA_ATMOS_PIPE_FLOW: &str = include_str!("../schemas/event/atmos_pipe_flow.json");
pub(crate) const SCHEMA_ATMOS_PIPE_FREEZE: &str = include_str!("../schemas/event/atmos_pipe_freeze.json");
pub(crate) const SCHEMA_ATMOS_PIPE_RUPTURE: &str = include_str!("../schemas/event/atmos_pipe_rupture.json");
pub(crate) const SCHEMA_ATMOS_ELECTROLYSIS_STARTED: &str =
    include_str!("../schemas/event/atmos_electrolysis_started.json");
pub(crate) const SCHEMA_ATMOS_WIND_FORCE_APPLIED: &str = include_str!("../schemas/event/atmos_wind_force_applied.json");
pub(crate) const SCHEMA_ATMOS_GAS_STRATIFIED: &str = include_str!("../schemas/event/atmos_gas_stratified.json");
pub(crate) const SCHEMA_GRAVITY_OVERRIDE_ACTIVATED: &str =
    include_str!("../schemas/event/gravity_override_activated.json");
pub(crate) const SCHEMA_GRAVITY_OVERRIDE_DEACTIVATED: &str =
    include_str!("../schemas/event/gravity_override_deactivated.json");
pub(crate) const SCHEMA_SHIELD_HIT: &str = include_str!("../schemas/event/shield_hit.json");
pub(crate) const SCHEMA_SHIELD_DEPLETED: &str = include_str!("../schemas/event/shield_depleted.json");
pub(crate) const SCHEMA_SHIELD_REGEN_STARTED: &str = include_str!("../schemas/event/shield_regen_started.json");
pub(crate) const SCHEMA_SHIELD_REGEN_COMPLETED: &str = include_str!("../schemas/event/shield_regen_completed.json");
pub(crate) const SCHEMA_SHIELD_DISRUPTED: &str = include_str!("../schemas/event/shield_disrupted.json");
pub(crate) const SCHEMA_ENVIRONMENT_SIGNAL_DELTA: &str = include_str!("../schemas/event/environment_signal_delta.json");
pub(crate) const SCHEMA_ENVIRONMENT_SIGNAL_AGGREGATED: &str =
    include_str!("../schemas/event/environment_signal_aggregated.json");
pub(crate) const SCHEMA_THERMAL_SIGNATURE_CHANGED: &str =
    include_str!("../schemas/event/thermal_signature_changed.json");
pub(crate) const SCHEMA_THERMAL_HEAT_EXCHANGED: &str = include_str!("../schemas/event/thermal_heat_exchanged.json");
pub(crate) const SCHEMA_THERMAL_MATERIAL_PHASE_CHANGE: &str =
    include_str!("../schemas/event/thermal_material_phase_change.json");
pub(crate) const SCHEMA_COMBAT_PROJECTILE_HIT_MO: &str = include_str!("../schemas/event/combat_projectile_hit_mo.json");
pub(crate) const SCHEMA_AUDIO_EVENT_REQUESTED: &str = include_str!("../schemas/event/audio_event_requested.json");
pub(crate) const SCHEMA_COMBAT_MELEE_HIT_MO: &str = include_str!("../schemas/event/combat_melee_hit_mo.json");
pub(crate) const SCHEMA_COMBAT_EXPLOSIVE_HIT_MO: &str = include_str!("../schemas/event/combat_explosive_hit_mo.json");
pub(crate) const SCHEMA_ACTOR_ACTION_REJECTED: &str = include_str!("../schemas/event/actor_action_rejected.json");
pub(crate) const SCHEMA_ACTOR_CLIMB_STARTED: &str = include_str!("../schemas/event/actor_climb_started.json");
pub(crate) const SCHEMA_ACTOR_DIVE_STARTED: &str = include_str!("../schemas/event/actor_dive_started.json");
pub(crate) const SCHEMA_ACTOR_FACING_CHANGED: &str = include_str!("../schemas/event/actor_facing_changed.json");
pub(crate) const SCHEMA_ACTOR_LEAN_CHANGED: &str = include_str!("../schemas/event/actor_lean_changed.json");
pub(crate) const SCHEMA_ACTOR_SLIDE_STARTED: &str = include_str!("../schemas/event/actor_slide_started.json");
pub(crate) const SCHEMA_ACTOR_STAMINA_CHANGED: &str = include_str!("../schemas/event/actor_stamina_changed.json");
pub(crate) const SCHEMA_ACTOR_STANCE_CHANGED: &str = include_str!("../schemas/event/actor_stance_changed.json");
pub(crate) const SCHEMA_ACTOR_VAULT_STARTED: &str = include_str!("../schemas/event/actor_vault_started.json");
pub(crate) const SCHEMA_COMBAT_KNIFE_THROW_LANDED: &str =
    include_str!("../schemas/event/combat_knife_throw_landed.json");
pub(crate) const SCHEMA_COMBAT_KNIFE_THROW_STARTED: &str =
    include_str!("../schemas/event/combat_knife_throw_started.json");
pub(crate) const SCHEMA_COMBAT_STEALTH_KILL_EXECUTED: &str =
    include_str!("../schemas/event/combat_stealth_kill_executed.json");
pub(crate) const SCHEMA_EQUIPMENT_BEACON_DROPPED: &str = include_str!("../schemas/event/equipment_beacon_dropped.json");
pub(crate) const SCHEMA_EQUIPMENT_BIPOD_DEPLOYED: &str = include_str!("../schemas/event/equipment_bipod_deployed.json");
pub(crate) const SCHEMA_EQUIPMENT_BIPOD_STOWED: &str = include_str!("../schemas/event/equipment_bipod_stowed.json");
pub(crate) const SCHEMA_EQUIPMENT_DRILL_OVERHEATED: &str =
    include_str!("../schemas/event/equipment_drill_overheated.json");
pub(crate) const SCHEMA_EQUIPMENT_FIRE_MODE_CYCLED: &str =
    include_str!("../schemas/event/equipment_fire_mode_cycled.json");
pub(crate) const SCHEMA_EQUIPMENT_GRENADE_COOKED: &str = include_str!("../schemas/event/equipment_grenade_cooked.json");
pub(crate) const SCHEMA_EQUIPMENT_GRENADE_DETONATED: &str =
    include_str!("../schemas/event/equipment_grenade_detonated.json");
pub(crate) const SCHEMA_EQUIPMENT_GRENADE_THROWN: &str = include_str!("../schemas/event/equipment_grenade_thrown.json");
pub(crate) const SCHEMA_EQUIPMENT_ITEM_DROPPED: &str = include_str!("../schemas/event/equipment_item_dropped.json");
pub(crate) const SCHEMA_EQUIPMENT_ITEM_PICKED_UP: &str = include_str!("../schemas/event/equipment_item_picked_up.json");
pub(crate) const SCHEMA_EQUIPMENT_MAGAZINE_CHANGED: &str =
    include_str!("../schemas/event/equipment_magazine_changed.json");
pub(crate) const SCHEMA_EQUIPMENT_MELEE_SWING: &str = include_str!("../schemas/event/equipment_melee_swing.json");
pub(crate) const SCHEMA_EQUIPMENT_MELEE_HIT_MO: &str = include_str!("../schemas/event/equipment_melee_hit_mo.json");
pub(crate) const SCHEMA_EQUIPMENT_SENSOR_PULSE_FIRED: &str =
    include_str!("../schemas/event/equipment_sensor_pulse_fired.json");
pub(crate) const SCHEMA_EQUIPMENT_SHELL_EJECTED: &str = include_str!("../schemas/event/equipment_shell_ejected.json");
pub(crate) const SCHEMA_EQUIPMENT_SUPPRESSOR_ATTACHED: &str =
    include_str!("../schemas/event/equipment_suppressor_attached.json");
pub(crate) const SCHEMA_EQUIPMENT_TOOL_BROKEN: &str = include_str!("../schemas/event/equipment_tool_broken.json");
pub(crate) const SCHEMA_EQUIPMENT_TOOL_REPAIRED: &str = include_str!("../schemas/event/equipment_tool_repaired.json");
pub(crate) const SCHEMA_EQUIPMENT_TOOL_USED: &str = include_str!("../schemas/event/equipment_tool_used.json");
pub(crate) const SCHEMA_EQUIPMENT_WEAPON_SWAP_COMPLETED: &str =
    include_str!("../schemas/event/equipment_weapon_swap_completed.json");
pub(crate) const SCHEMA_EQUIPMENT_WEAPON_SWAP_STARTED: &str =
    include_str!("../schemas/event/equipment_weapon_swap_started.json");
pub(crate) const SCHEMA_INVENTORY_TANK_SLOT_RESERVED: &str =
    include_str!("../schemas/event/inventory_tank_slot_reserved.json");
pub(crate) const SCHEMA_INVENTORY_WEIGHT_CHANGED: &str = include_str!("../schemas/event/inventory_weight_changed.json");
pub(crate) const SCHEMA_EQUIPMENT_ITEM_PICKED_UP_WITH_MASS: &str =
    include_str!("../schemas/event/item_picked_up_with_mass.json");
pub(crate) const SCHEMA_EQUIPMENT_ITEM_DROPPED_WITH_MASS: &str =
    include_str!("../schemas/event/item_dropped_with_mass.json");
pub(crate) const SCHEMA_INVENTORY_ENCUMBRANCE_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/encumbrance_threshold_crossed.json");
pub(crate) const SCHEMA_INVENTORY_CONTAINER_NESTED: &str = include_str!("../schemas/event/container_nested.json");
pub(crate) const SCHEMA_BODY_ARMOR_DEGRADED: &str = include_str!("../schemas/event/body_armor_degraded.json");
pub(crate) const SCHEMA_ATGM_LOCK_ACQUIRED: &str = include_str!("../schemas/event/atgm_lock_acquired.json");
pub(crate) const SCHEMA_ACTOR_REVIVED: &str = include_str!("../schemas/event/actor_revived.json");
pub(crate) const SCHEMA_MINE_DETONATED: &str = include_str!("../schemas/event/mine_detonated.json");
pub(crate) const SCHEMA_MORTAR_CREWED: &str = include_str!("../schemas/event/mortar_crewed.json");
pub(crate) const SCHEMA_PERCEPTION_ACTOR_SIGNAL: &str = include_str!("../schemas/event/perception_actor_signal.json");
pub(crate) const SCHEMA_PERCEPTION_FOOTSTEP_EMITTED: &str =
    include_str!("../schemas/event/perception_footstep_emitted.json");
pub(crate) const SCHEMA_PERCEPTION_OCCLUSION_APPLIED: &str =
    include_str!("../schemas/event/perception_occlusion_applied.json");
pub(crate) const SCHEMA_PERCEPTION_STEALTH_METER_CHANGED: &str =
    include_str!("../schemas/event/perception_stealth_meter_changed.json");
pub(crate) const SCHEMA_SQUAD_COMMAND_ISSUED: &str = include_str!("../schemas/event/squad_command_issued.json");
pub(crate) const SCHEMA_SQUAD_MEMBER_ADDED: &str = include_str!("../schemas/event/squad_member_added.json");
pub(crate) const SCHEMA_SQUAD_WAYPOINT_MARKED: &str = include_str!("../schemas/event/squad_waypoint_marked.json");
pub(crate) const SCHEMA_SQUAD_COMMAND_VETOED: &str = include_str!("../schemas/event/squad_command_vetoed.json");
pub(crate) const SCHEMA_SQUAD_FORMATION_SET: &str = include_str!("../schemas/event/squad_formation_set.json");
pub(crate) const SCHEMA_SQUAD_FORMATION_SLOT_ASSIGNED: &str =
    include_str!("../schemas/event/squad_formation_slot_assigned.json");
pub(crate) const SCHEMA_SQUAD_FORMATION_SLOT_BROKEN: &str =
    include_str!("../schemas/event/squad_formation_slot_broken.json");
pub(crate) const SCHEMA_SQUAD_FORMATION_COLLAPSED: &str =
    include_str!("../schemas/event/squad_formation_collapsed.json");
pub(crate) const SCHEMA_SQUAD_ROLE_ASSIGNED: &str = include_str!("../schemas/event/squad_role_assigned.json");
pub(crate) const SCHEMA_SQUAD_BREACH_CHAIN_STARTED: &str =
    include_str!("../schemas/event/squad_breach_chain_started.json");
pub(crate) const SCHEMA_SQUAD_BREACH_CHAIN_STEP: &str = include_str!("../schemas/event/squad_breach_chain_step.json");
pub(crate) const SCHEMA_SQUAD_BREACH_CHAIN_COMPLETE: &str =
    include_str!("../schemas/event/squad_breach_chain_complete.json");
pub(crate) const SCHEMA_SQUAD_BOUNDING_STEP: &str = include_str!("../schemas/event/squad_bounding_step.json");
pub(crate) const SCHEMA_SQUAD_BRAIN_HOP: &str = include_str!("../schemas/event/squad_brain_hop.json");
pub(crate) const SCHEMA_AI_REASON_LABEL_CHANGED: &str = include_str!("../schemas/event/ai_reason_label_changed.json");
pub(crate) const SCHEMA_AI_THINKING_LAYER_INVOKED: &str =
    include_str!("../schemas/event/ai_thinking_layer_invoked.json");
pub(crate) const SCHEMA_AI_ARCHETYPE_CHOSEN: &str = include_str!("../schemas/event/ai_archetype_chosen.json");
pub(crate) const SCHEMA_AI_AUTO_TRIAGE_INITIATED: &str = include_str!("../schemas/event/ai_auto_triage_initiated.json");
pub(crate) const SCHEMA_AI_AUTO_TRIAGE_APPLIED: &str = include_str!("../schemas/event/ai_auto_triage_applied.json");
pub(crate) const SCHEMA_AI_AUTO_REPAIR_INITIATED: &str = include_str!("../schemas/event/ai_auto_repair_initiated.json");
pub(crate) const SCHEMA_AI_AUTO_REPAIR_PROGRESSED: &str =
    include_str!("../schemas/event/ai_auto_repair_progressed.json");
pub(crate) const SCHEMA_AI_COVER_SEEKING_STARTED: &str = include_str!("../schemas/event/ai_cover_seeking_started.json");
pub(crate) const SCHEMA_AI_SUPPRESSION_STARTED: &str = include_str!("../schemas/event/ai_suppression_started.json");
pub(crate) const SCHEMA_AI_RETREAT_DECISION: &str = include_str!("../schemas/event/ai_retreat_decision.json");
pub(crate) const SCHEMA_AI_SQUAD_COMM_RELAYED: &str = include_str!("../schemas/event/ai_squad_comm_relayed.json");
pub(crate) const SCHEMA_AI_PATROL_WAYPOINT_REACHED: &str =
    include_str!("../schemas/event/ai_patrol_waypoint_reached.json");
pub(crate) const SCHEMA_AI_FRIENDLY_FIRE_AVOIDANCE: &str =
    include_str!("../schemas/event/ai_friendly_fire_avoidance.json");
pub(crate) const SCHEMA_AI_HIGH_GROUND_PREFERENCE_APPLIED: &str =
    include_str!("../schemas/event/ai_high_ground_preference_applied.json");
pub(crate) const SCHEMA_MISSION_PHASE_CHANGED: &str = include_str!("../schemas/event/mission_phase_changed.json");
pub(crate) const SCHEMA_MISSION_DIRECTOR_PHASE_CHANGE: &str =
    include_str!("../schemas/event/mission_director_phase_change.json");
pub(crate) const SCHEMA_MISSION_OBJECTIVE_BRANCHED: &str =
    include_str!("../schemas/event/mission_objective_branched.json");
pub(crate) const SCHEMA_MISSION_OPTIONAL_OFFERED: &str = include_str!("../schemas/event/mission_optional_offered.json");
pub(crate) const SCHEMA_MISSION_REINFORCEMENT_WAVE_SPAWNED: &str =
    include_str!("../schemas/event/mission_reinforcement_wave_spawned.json");
pub(crate) const SCHEMA_BOSS_PHASE_CHANGED: &str = include_str!("../schemas/event/boss_phase_changed.json");
pub(crate) const SCHEMA_BOSS_SPECIAL_ABILITY_TRIGGERED: &str =
    include_str!("../schemas/event/boss_special_ability_triggered.json");
pub(crate) const SCHEMA_AI_PRIORITY_TABLE_CHANGED: &str =
    include_str!("../schemas/event/ai_priority_table_changed.json");
pub(crate) const SCHEMA_AI_AUTONOMY_MODE_CHANGED: &str = include_str!("../schemas/event/ai_autonomy_mode_changed.json");
pub(crate) const SCHEMA_AI_ROLE_TEMPLATE_APPLIED: &str = include_str!("../schemas/event/ai_role_template_applied.json");
pub(crate) const SCHEMA_AI_QUICK_PRESET_APPLIED: &str = include_str!("../schemas/event/ai_quick_preset_applied.json");
pub(crate) const SCHEMA_AI_CHATTER_EMITTED: &str = include_str!("../schemas/event/ai_chatter_emitted.json");
pub(crate) const SCHEMA_AI_PERSONALITY_CHANGED: &str = include_str!("../schemas/event/ai_personality_changed.json");
pub(crate) const SCHEMA_AI_MOOD_CHANGED: &str = include_str!("../schemas/event/ai_mood_changed.json");
pub(crate) const SCHEMA_AI_STRESS_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/ai_stress_threshold_crossed.json");
pub(crate) const SCHEMA_AI_FACTION_ALLEGIANCE_CHANGED: &str =
    include_str!("../schemas/event/ai_faction_allegiance_changed.json");
pub(crate) const SCHEMA_CAMERA_HIT_STOP: &str = include_str!("../schemas/event/camera_hit_stop.json");
pub(crate) const SCHEMA_CAMERA_MODE_CHANGED: &str = include_str!("../schemas/event/camera_mode_changed.json");
pub(crate) const SCHEMA_PHOTO_MODE_ENTERED: &str = include_str!("../schemas/event/photo_mode_entered.json");
pub(crate) const SCHEMA_PHOTO_MODE_EXITED: &str = include_str!("../schemas/event/photo_mode_exited.json");
pub(crate) const SCHEMA_PHOTO_MODE_FILTER_CHANGED: &str =
    include_str!("../schemas/event/photo_mode_filter_changed.json");
pub(crate) const SCHEMA_PHOTO_MODE_SHOT_TAKEN: &str = include_str!("../schemas/event/photo_mode_shot_taken.json");
pub(crate) const SCHEMA_REPLAY_SCRUB_OFFSET_CHANGED: &str =
    include_str!("../schemas/event/replay_scrub_offset_changed.json");
pub(crate) const SCHEMA_REPLAY_BOOKMARK_ADDED: &str = include_str!("../schemas/event/replay_bookmark_added.json");
pub(crate) const SCHEMA_KILLCAM_PLAYED: &str = include_str!("../schemas/event/killcam_played.json");
pub(crate) const SCHEMA_KILLCAM_SKIPPED: &str = include_str!("../schemas/event/killcam_skipped.json");
pub(crate) const SCHEMA_SLOW_MO_KILL_CAM_TRIGGERED: &str =
    include_str!("../schemas/event/slow_mo_kill_cam_triggered.json");
pub(crate) const SCHEMA_UX_HUD_LAYOUT_CHANGED: &str = include_str!("../schemas/event/ux_hud_layout_changed.json");
pub(crate) const SCHEMA_UX_PRESET_SAVED: &str = include_str!("../schemas/event/ux_preset_saved.json");
pub(crate) const SCHEMA_UX_DEBUG_OVERLAY_TOGGLED: &str = include_str!("../schemas/event/ux_debug_overlay_toggled.json");
pub(crate) const SCHEMA_UX_TACTICAL_OVERLAY_TOGGLED: &str =
    include_str!("../schemas/event/ux_tactical_overlay_toggled.json");
pub(crate) const SCHEMA_UX_PIE_MENU_OPENED: &str = include_str!("../schemas/event/ux_pie_menu_opened.json");
pub(crate) const SCHEMA_UX_PIE_MENU_SLICE_CHOSEN: &str = include_str!("../schemas/event/ux_pie_menu_slice_chosen.json");
pub(crate) const SCHEMA_UX_PIE_MENU_CLOSED: &str = include_str!("../schemas/event/ux_pie_menu_closed.json");
pub(crate) const SCHEMA_UX_PIE_MENU_SLICE_REJECTED: &str =
    include_str!("../schemas/event/ux_pie_menu_slice_rejected.json");
pub(crate) const SCHEMA_UX_GAME_SPEED_ASSIST_CHANGED: &str =
    include_str!("../schemas/event/ux_game_speed_assist_changed.json");
pub(crate) const SCHEMA_UX_BANNER_RAISED: &str = include_str!("../schemas/event/banner_raised.json");
pub(crate) const SCHEMA_UX_BANNER_DISMISSED: &str = include_str!("../schemas/event/banner_dismissed.json");
pub(crate) const SCHEMA_UX_FOCUS_MOVED: &str = include_str!("../schemas/event/focus_moved.json");
pub(crate) const SCHEMA_UX_CAPTIONS_SHOWN: &str = include_str!("../schemas/event/captions_shown.json");
pub(crate) const SCHEMA_UX_TOOL_VALIDITY_CHANGED: &str = include_str!("../schemas/event/tool_validity_changed.json");
pub(crate) const SCHEMA_UX_MOUSE_CLICKED: &str = include_str!("../schemas/event/mouse_clicked.json");
pub(crate) const SCHEMA_UX_MOUSE_MOVED: &str = include_str!("../schemas/event/mouse_moved.json");
pub(crate) const SCHEMA_ACCESSIBILITY_SETTINGS_CHANGED: &str =
    include_str!("../schemas/event/settings_changed_accessibility.json");
pub(crate) const SCHEMA_ACCESSIBILITY_UI_SCALE_APPLIED: &str = include_str!("../schemas/event/ui_scale_applied.json");
pub(crate) const SCHEMA_AI_PLAN_COMPOSED: &str = include_str!("../schemas/event/ai_plan_composed.json");
pub(crate) const SCHEMA_AI_PLAN_EXECUTED: &str = include_str!("../schemas/event/ai_plan_executed.json");
pub(crate) const SCHEMA_AI_PLAN_ABORTED: &str = include_str!("../schemas/event/ai_plan_aborted.json");
pub(crate) const SCHEMA_AI_CONTEXT_WHEEL_OPENED: &str = include_str!("../schemas/event/ai_context_wheel_opened.json");
pub(crate) const SCHEMA_AI_CONTEXT_WHEEL_SELECTED: &str =
    include_str!("../schemas/event/ai_context_wheel_selected.json");
pub(crate) const SCHEMA_AI_PANIC_CALL_EMITTED: &str = include_str!("../schemas/event/ai_panic_call_emitted.json");
pub(crate) const SCHEMA_AI_TARGET_TAGGED: &str = include_str!("../schemas/event/ai_target_tagged.json");
pub(crate) const SCHEMA_AI_REASON_QUERY_RETURNED: &str = include_str!("../schemas/event/ai_reason_query_returned.json");
pub(crate) const SCHEMA_MISSION_REACTOR_HP_CHANGED: &str =
    include_str!("../schemas/event/mission_reactor_hp_changed.json");
pub(crate) const SCHEMA_MISSION_REACTOR_DESTROYED: &str =
    include_str!("../schemas/event/mission_reactor_destroyed.json");
pub(crate) const SCHEMA_MISSION_REACTOR_PRESSURE_STATE_CHANGED: &str =
    include_str!("../schemas/event/mission_reactor_pressure_state_changed.json");
pub(crate) const SCHEMA_MISSION_TIMER_WARNING_THRESHOLD: &str =
    include_str!("../schemas/event/mission_timer_warning_threshold.json");
pub(crate) const SCHEMA_TERRAIN_MATERIAL_STATE_CHANGED: &str =
    include_str!("../schemas/event/terrain_material_state_changed.json");
pub(crate) const SCHEMA_TERRAIN_PIXEL_REMOVED: &str = include_str!("../schemas/event/terrain_pixel_removed.json");
pub(crate) const SCHEMA_TERRAIN_CASCADE_TRIGGERED: &str =
    include_str!("../schemas/event/terrain_cascade_triggered.json");
pub(crate) const SCHEMA_TERRAIN_DEBRIS_SPAWNED: &str = include_str!("../schemas/event/terrain_debris_spawned.json");
pub(crate) const SCHEMA_AI_TARGET_SCORED: &str = include_str!("../schemas/event/ai_target_scored.json");
pub(crate) const SCHEMA_AI_PATH_INVALIDATED: &str = include_str!("../schemas/event/ai_path_invalidated.json");
pub(crate) const SCHEMA_UX_SLIDESHOW_STARTED: &str = include_str!("../schemas/event/ux_slideshow_started.json");
pub(crate) const SCHEMA_UX_SLIDESHOW_ENDED: &str = include_str!("../schemas/event/ux_slideshow_ended.json");
pub(crate) const SCHEMA_UX_JUICE_APPLIED: &str = include_str!("../schemas/event/ux_juice_applied.json");
pub(crate) const SCHEMA_AUDIO_EVENT_PLAYED: &str = include_str!("../schemas/event/audio_event_played.json");
pub(crate) const SCHEMA_AUDIO_SPATIAL_RESOLVED: &str = include_str!("../schemas/event/audio_spatial_resolved.json");
pub(crate) const SCHEMA_AUDIO_REVERB_APPLIED: &str = include_str!("../schemas/event/audio_reverb_applied.json");
pub(crate) const SCHEMA_AUDIO_OCCLUDED: &str = include_str!("../schemas/event/audio_occluded.json");
pub(crate) const SCHEMA_AUDIO_DOPPLER_SHIFTED: &str = include_str!("../schemas/event/audio_doppler_shifted.json");
pub(crate) const SCHEMA_CINEMATIC_STARTED: &str = include_str!("../schemas/event/cinematic_started.json");
pub(crate) const SCHEMA_CINEMATIC_CHAPTER_MARKER: &str = include_str!("../schemas/event/cinematic_chapter_marker.json");
pub(crate) const SCHEMA_CINEMATIC_SKIPPED: &str = include_str!("../schemas/event/cinematic_skipped.json");
pub(crate) const SCHEMA_CINEMATIC_PAUSED: &str = include_str!("../schemas/event/cinematic_paused.json");
pub(crate) const SCHEMA_CINEMATIC_RESUMED: &str = include_str!("../schemas/event/cinematic_resumed.json");
pub(crate) const SCHEMA_CINEMATIC_ENDED: &str = include_str!("../schemas/event/cinematic_ended.json");
pub(crate) const SCHEMA_CINEMATIC_NARRATION_WORD: &str = include_str!("../schemas/event/cinematic_narration_word.json");
pub(crate) const SCHEMA_COMBAT_SWEPT_COLLISION: &str = include_str!("../schemas/event/combat_swept_collision.json");
pub(crate) const SCHEMA_COMBAT_BULLET_SHARPNESS_DECAY: &str =
    include_str!("../schemas/event/combat_bullet_sharpness_decay.json");
pub(crate) const SCHEMA_COMBAT_EMBEDDED_IN_TERRAIN: &str =
    include_str!("../schemas/event/combat_embedded_in_terrain.json");
pub(crate) const SCHEMA_ATTACHABLE_DETACHED: &str = include_str!("../schemas/event/attachable_detached.json");
pub(crate) const SCHEMA_ATTACHABLE_GIB_THRESHOLD_CROSSED: &str =
    include_str!("../schemas/event/attachable_gib_threshold_crossed.json");
pub(crate) const SCHEMA_BODY_GIB_CREATED: &str = include_str!("../schemas/event/body_gib_created.json");
pub(crate) const SCHEMA_BODY_GIB_CASCADE_TRIGGERED: &str =
    include_str!("../schemas/event/body_gib_cascade_triggered.json");
pub(crate) const SCHEMA_PHYSICS_RAGDOLL_ACTIVATED: &str =
    include_str!("../schemas/event/physics_ragdoll_activated.json");
pub(crate) const SCHEMA_PHYSICS_IMPULSE_PROPAGATED: &str =
    include_str!("../schemas/event/physics_impulse_propagated.json");
pub(crate) const SCHEMA_ARMOR_SPALLING_FRAGMENT_SPAWNED: &str =
    include_str!("../schemas/event/armor_spalling_fragment_spawned.json");
pub(crate) const SCHEMA_ARMOR_SPALLING_FRAGMENT_HIT_MODULE: &str =
    include_str!("../schemas/event/armor_spalling_fragment_hit_module.json");
pub(crate) const SCHEMA_ARMOR_AMMO_RACK_DETONATED: &str =
    include_str!("../schemas/event/armor_ammo_rack_detonated.json");
pub(crate) const SCHEMA_OBJECTIVE_PAUSED: &str = include_str!("../schemas/event/mission_objective_paused.json");
pub(crate) const SCHEMA_OBJECTIVE_RESUMED: &str = include_str!("../schemas/event/mission_objective_resumed.json");
pub(crate) const SCHEMA_AI_SCOPE_SETTLE: &str = include_str!("../schemas/event/ai_scope_settle.json");
pub(crate) const SCHEMA_ACTOR_MOOD_CHANGED: &str = include_str!("../schemas/event/actor_mood_changed.json");
pub(crate) const SCHEMA_FACTION_RELATIONSHIP_CHANGED: &str =
    include_str!("../schemas/event/faction_relationship_changed.json");
pub(crate) const SCHEMA_UX_PLAN_COMPOSED: &str = include_str!("../schemas/event/ux_plan_composed.json");
pub(crate) const SCHEMA_UX_PLAN_EXECUTED: &str = include_str!("../schemas/event/ux_plan_executed.json");
pub(crate) const SCHEMA_UX_PLAN_ABORTED: &str = include_str!("../schemas/event/ux_plan_aborted.json");
pub(crate) const SCHEMA_MATERIAL_REACTION_TRIGGERED: &str =
    include_str!("../schemas/event/material_reaction_triggered.json");
pub(crate) const SCHEMA_MATERIAL_PHASE_TRANSITION: &str =
    include_str!("../schemas/event/material_phase_transition.json");
pub(crate) const SCHEMA_MATERIAL_CELLULAR_STEP: &str = include_str!("../schemas/event/material_cellular_step.json");
pub(crate) const SCHEMA_FLASK_THROWN: &str = include_str!("../schemas/event/flask_thrown.json");
pub(crate) const SCHEMA_FLASK_CONSUMED: &str = include_str!("../schemas/event/flask_consumed.json");
pub(crate) const SCHEMA_ALCHEMY_RECIPE_INVOKED: &str = include_str!("../schemas/event/alchemy_recipe_invoked.json");
pub(crate) const SCHEMA_ALCHEMY_RECIPE_COMPLETED: &str = include_str!("../schemas/event/alchemy_recipe_completed.json");
pub(crate) const SCHEMA_MATERIAL_PHASE_NUCLEATED: &str = include_str!("../schemas/event/material_phase_nucleated.json");
pub(crate) const SCHEMA_MATERIAL_PRECIPITATION_STARTED: &str =
    include_str!("../schemas/event/material_precipitation_started.json");
pub(crate) const SCHEMA_MATERIAL_GPU_CPU_DIVERGENCE_DETECTED: &str =
    include_str!("../schemas/event/material_gpu_cpu_divergence_detected.json");
pub(crate) const SCHEMA_MATERIAL_VIOLENT_BURST: &str = include_str!("../schemas/event/material_violent_burst.json");
pub(crate) const SCHEMA_MATERIAL_REGISTERED: &str = include_str!("../schemas/event/material_registered.json");
pub(crate) const SCHEMA_MATERIAL_REGISTRY_VALIDATION_FAILED: &str =
    include_str!("../schemas/event/material_registry_validation_failed.json");
