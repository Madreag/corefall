//! M1+M5: weapon presets, per-actor weapon state, and the M5 role-record schema.
//!
//! - M1 owns the [`RifleSpec`] preset + [`RifleState`] state machine. The engine
//!   ticks one rifle per actor each fixed step; the state machine emits structured
//!   outcomes (`fired`, `reloaded`, `dry_fire`) that the caller turns into
//!   `weapon.*` events.
//! - **M5** lands the full **role-record** schema ([`RoleRecord`]) + the [`Loadout`]
//!   registry + AI policy hints + jam-chance / origin-compatibility metadata.
//!   The M1 rifle still works as before; under the hood every rifle preset is now
//!   ALSO exposed through [`role_record`]/[`loadouts()`] so chassis sockets, AI
//!   doctrine, and modding tools all see the same role-record contract.
//!
//! The M5 contract is the **minimum bar** per AGENTS.md: a `RoleRecord` carries
//! every field the chassis grammar (cf-chassis), AI (cf-ai), and HUD/inspect
//! (cfctl) need to reason about a piece of equipment without a screenshot.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::float_cmp,
    clippy::manual_is_multiple_of,
    clippy::similar_names,
    clippy::too_many_lines
)]

// M1 / M2 / M3 spec "## Files" wiring: the helpers live in dedicated
// submodules so consumers that import per the spec paths
// (`cf_equipment::projectile::*`, `cf_equipment::digger::*`) resolve cleanly.
pub mod digger;
pub mod projectile;
pub use digger::DiggerTool;
pub use projectile::ProjectileSpawnParams;

// M6 modules — see spec § Files for the canonical list.
// M6C adds: heavy, medical, survival, sensor, ppe.
pub mod ammo_spec;
pub mod bipod;
pub mod cram;
pub mod deployables;
pub mod durability;
pub mod fire_modes;
pub mod grapple_gun;
pub mod grenade;
pub mod heavy;
pub mod inventory;
pub mod item_spec;
pub mod jetpack;
pub mod jetpack_atmos;
pub mod knife_throw;
pub mod cures;
pub mod magazine;
pub mod medical;
pub mod medical_scanner;
pub mod melee;
pub mod psych_meds;
pub mod stims;
pub mod vaccines;
pub mod ppe;
pub mod sensor;
pub mod shell;
pub mod stealth_kill;
pub mod suppressor;
pub mod survival;
pub mod tool;
pub mod weapon;
pub mod weapon_swap;
pub mod weapons_m14c;
pub mod zip_kit;

// M1 / M5 core surfaces extracted from the original monolithic lib.rs.
pub mod fire_mode;
pub mod loadout;
pub mod presets;
pub mod rifle_runtime;
pub mod rifle_spec;
pub mod role;

pub use bipod::{Bipod, BipodState, BIPOD_BLOOM_FACTOR, BIPOD_RECOIL_FACTOR};
pub use grapple_gun::{
    fire_grapple, GrappleFireOutcome, GRAPPLE_GUN_T2_ID, GRAPPLE_LONG_DISTANCE_M, GRAPPLE_MAX_RANGE_M,
    ROPE_CLIMB_SPEED_M_PER_S, ROPE_RAPPEL_SPEED_M_PER_S, SADDLE_UNIVERSAL_ID,
};
pub use zip_kit::{
    deploy_zip_kit, zipline_step_speed, ZipKitDeployOutcome, ZIPLINE_BRAKE_DECEL_M_PER_S2,
    ZIPLINE_MAX_SPAN_M, ZIPLINE_MAX_SPEED_M_PER_S, ZIPLINE_MIN_HEIGHT_DELTA_M, ZIP_KIT_T2_ID,
};
pub use cram::{Cram, DEFAULT_CRAM_COOLDOWN_TICKS};
pub use durability::{Durability, DURABILITY_MAX};
pub use fire_modes::{
    charge_damage_multiplier, charge_fraction, AdvancedFireMode, FireModeSet, BURST3_INTER_SHOT_SECONDS,
    BURST3_ROUND_COUNT, SNIPER_CHARGE_MAX_SECONDS, SNIPER_MISFIRE_BELOW,
};
pub use grenade::{
    cook_grenade, m6_grenade_presets, m6c_throwable_presets,
    mines::{
        evaluate_mine_trigger as evaluate_mine_trigger_for_candidates, CandidateActor as MineCandidateActor,
        MineDescriptor, MineTriggerKind, MineTriggerOutcome, Relation as MineRelation,
        PROXIMITY_TRIGGER_RADIUS_TILES,
    },
    GrenadeKind, GrenadePreset,
};
pub use heavy::{
    atgm::{AtgmLockOutcome, AtgmLockState, AtgmLockTracker, ATGM_LOCK_ACQUISITION_SECONDS},
    flamethrower::{
        FireSpawnTick, FlamethrowerState, FlamethrowerTickOutcome, FIRE_TILES_PER_SECOND,
        FIRE_TILE_INTENSITY, FUEL_PER_SECOND_LITERS, FUEL_PRESSURE_CUTOFF, FUEL_PRESSURE_FULL,
    },
    m6c_heavy_presets,
    mortar::{evaluate_crew_fire, CrewFireOutcome, CREW_REQUIRED_REASON},
    HeavyWeaponKind, HeavyWeaponPreset,
};
pub use inventory::{
    ExtendedInventory, ExtendedSlot, SlotKind, SlotState, ACTIVE_SLOT_COUNT, TANK_SLOT_COUNT, TANK_SLOT_LOCKED_REASON,
    TOTAL_SLOT_COUNT, WEIGHT_FORCE_CRAWL_KG, WEIGHT_FORCE_WALK_KG,
};
pub use item_spec::{
    bulk_volume_l_for_id, carry_capacity_modifier, encumbrance_band, liquid_fill_mass, m6c_entries,
    m6c_entries_by_category, mass_kg_for_id, max_carry_kg_for_origin, max_carry_volume_l_for_origin,
    quick_slot_eligible_ids, registered_ids as item_registered_ids, spec_for_id, try_nest_depth,
    walk_speed_multiplier, BackpackTier, ContainerCapacity, EncumbranceBand, GridDim, ItemCategory, ItemId, ItemSpec,
    MaterialId, OriginId, RecipeId, HUMAN_BASELINE_MAX_CARRY_KG, HUMAN_BASELINE_MAX_CARRY_VOLUME_L,
    MAX_CONTAINER_NEST_DEPTH, MAX_DEPTH_EXCEEDED, WALK_SPEED_AT_EMPTY_CARRY, WALK_SPEED_AT_FULL_CARRY,
};
pub use jetpack::{
    jet_pressure_efficiency, jetpack_tick, Jetpack, JetpackEvent, JetpackTickOutcome, JetpackType,
};
pub use jetpack_atmos::{muzzle_flash_combusts, pressure_modulated_thrust};
pub use knife_throw::{KnifeProjectile, KnifeThrowState, KNIFE_THROW_DAMAGE_FACTOR, KNIFE_THROW_MAX_FLIGHT_SECONDS};
pub use ammo_spec::{
    apfsds_round_v1, heat_round_v1, parse_ammo_spec, resolve_ammo_spec, AmmoSpec, AmmoSpecLoadError,
    ApfsdsRoundSpec, HeatRoundSpec, APFSDS_ROUND_V1_ID, APFSDS_ROUND_V1_RON, HEAT_ROUND_V1_ID, HEAT_ROUND_V1_RON,
};
pub use magazine::{Magazine, PoppedRound, RoundKind};
pub use weapons_m14c::{
    parse_m14c_weapon, resolve_m14c_weapon, M14cWeaponSpec, M14cWeaponSpecLoadError, RPG_LAUNCHER_V1_ID,
    RPG_LAUNCHER_V1_RON, TANK_AUTOCANNON_T3_ID, TANK_AUTOCANNON_T3_RON,
};
pub use medical::{
    defibrillator::{
        apply_defibrillator, DefibOutcome, DefibTarget, DEFIB_REVIVE_HP_FRACTION,
        DEFIB_REVIVE_WINDOW_SECONDS,
    },
    m6c_medical_presets, MedicalEffectKind, MedicalPreset,
};
pub use medical_scanner::{
    load_scanner_dir, load_scanner_spec, MedicalScannerSpec, ScanInProgress, ScannerLoadError,
    SCAN_CONFIDENCE_DEFAULT, SCAN_DURATION_SECONDS_DEFAULT,
};
pub use cures::{
    cure_item_for, default_cure_catalog, load_cure_dir, CureItemSpec, CureLoadError,
};
pub use vaccines::{
    default_vaccine_catalog, load_vaccine_dir, vaccine_catalog, VaccineItemSpec, VaccineLoadError,
};
pub use melee::{
    m6_melee_presets, m6c_melee_presets, MeleeKind, MeleePreset, BATON_M6_DEFAULT_ID, HATCHET_M6_DEFAULT_ID,
    KNIFE_M6_DEFAULT_ID, RIFLE_BASH_M6_DEFAULT_ID,
};
pub use ppe::{
    armor_calc::{apply_kinetic_hit, DamageReductionResult, ARMOR_FAILURE_FRACTION, DEGRADED_THRESHOLD_FRACTION},
    eva::{tick_vacuum, VacuumTickInputs, VacuumTickResult, DECOMPRESSION_DAMAGE_PER_SECOND, SEALED_O2_DRAIN_PER_SECOND_L},
    m6c_ppe_presets, ppe_preset, PpeKind, PpePreset,
};
pub use sensor::{m6c_sensor_presets, SensorKind, SensorPreset};
pub use survival::{m6c_survival_presets, SurvivalEffectKind, SurvivalPreset};
pub use shell::{ShellEjection, ShellKind};
pub use stealth_kill::{
    evaluate_attempt, StealthKillAttempt, StealthKillRejection, STEALTH_KILL_ANIMATION_SECONDS, STEALTH_KILL_METER_MAX,
    STEALTH_KILL_REACH,
};
pub use suppressor::{Suppressor, SUPPRESSOR_LOUDNESS_FACTOR};
pub use tool::{
    brace_strut::{
        brace_strut_catalog, brace_strut_for_tier, brace_strut_ron_for_tier, brace_strut_t1_default,
        brace_strut_t2_default, brace_strut_t3_default, find_brace_strut, parse_brace_strut_ron,
        BraceStrutSpec, BraceStrutSpecLoadError, BraceStrutTier, BRACE_STRUT_T1_ID,
        BRACE_STRUT_T1_RON, BRACE_STRUT_T2_ID, BRACE_STRUT_T2_RON, BRACE_STRUT_T3_ID,
        BRACE_STRUT_T3_RON,
    },
    drill::{DRILL_HEAT_DECAY_PER_S, DRILL_HEAT_PER_USE, DRILL_HEAT_RATE_PER_S, DRILL_JAM_HEAT_THRESHOLD},
    engineering_tool::{
        engineering_tool_m9c_default, EngineeringToolSpec, ENGINEERING_TOOL_AT_DITCH_SECONDS,
        ENGINEERING_TOOL_ID,
    },
    entrenching::{entrenching_tool_m9b_default, EntrenchingToolSpec, ENTRENCHING_TOOL_ID},
    find_entrenching_tool, find_support_beam_placer, is_m9c_tool, m14e_support_beam_placer, m6_tool_presets,
    m9b_entrenching_tools, m9c_tool_catalog,
    minesweeper::{
        minesweeper_m9c_default, MinesweeperToolSpec, MINESWEEPER_ID, MINESWEEPER_PING_SECONDS,
        MINESWEEPER_RADIUS_PRESSURE_IED, MINESWEEPER_RADIUS_PROXIMITY_TRIPWIRE,
    },
    sensor_pulse::{SENSOR_PULSE_REVEAL_RADIUS, SENSOR_PULSE_REVEAL_SECONDS},
    support_beam_placer::{
        support_beam_placer_m14e_default, SupportBeamPlacerSpec, SUPPORT_BEAM_PLACER_ID,
        SUPPORT_BEAM_PLACER_TIER,
    },
    wire_cutters::{wire_cutters_m9c_default, WireCuttersSpec, WIRE_CUTTERS_ID},
    M9cToolCatalog, ToolKind, ToolPreset,
};
pub use deployables::{
    bomb_disposal_robot_spec, BombDisposalRobotSpec, BombDisposalRobotState,
    BombDisposalRobotStatus, BOMB_DISPOSAL_ROBOT_CHASSIS_ID,
    BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND, BOMB_DISPOSAL_ROBOT_HP,
    BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS,
    BOMB_DISPOSAL_ROBOT_REACTIVE_ARMOR_REDUCTION_PERCENT,
};
pub use weapon::{m6_weapon_presets, m6c_firearm_presets, WeaponClass, WeaponPreset};
pub use weapon_swap::{swap_duration_for_target, WeaponSwap, PISTOL_SWAP_SECONDS, WEAPON_SWAP_SECONDS};

// M1 / M5 re-exports from the extracted modules — preserves the canonical
// `cf_equipment::*` surface for cf-control + cf-actor + cf-mod + every other
// consumer.
pub use fire_mode::FireMode;
pub use loadout::{
    load_loadout_from_json, load_loadouts_from_dir, Loadout, LoadoutFile, LoadoutLoadError,
    LOADOUT_SCHEMA_VERSION,
};
pub use presets::{
    available_fire_modes_for, loadout, loadouts, rifle_preset, rifle_presets, role_record,
    role_records, RIFLE_M1_TRACER_ID, RPG_LAUNCHER_V1_RIFLE_ID, SHOTGUN_M1_DEFAULT_ID,
    TANK_AUTOCANNON_T3_RIFLE_ID,
};
pub use rifle_runtime::{tick_rifle, RifleState, RifleTickInputs, TickOutcomes};
pub use rifle_spec::{
    RifleSpec, CARBINE_M5_POWERED_ID, RIFLE_M1_DEFAULT_ID, RIFLE_M5_MECH_HEAVY_ID,
};
pub use role::{AiPolicyHint, FiringProfile, OriginCompatibility, RoleKind, RoleRecord};

#[cfg(test)]
mod tests;
