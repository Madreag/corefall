//! **M14A** § "Constants — the CC magic numbers".
//!
//! Single canonical home for every constant the M14A spec table calls out.
//! Sub-modules re-export the focused subsets, but every constant lives here
//! so a worker can grep one file to find the spec-locked default.

// ============================================================================
// Walking + chassis attitude (CC AHuman.cpp)
// ============================================================================

pub const STAND_ROT_TARGET: f32 = 0.0;
pub const WALK_ROT_TARGET: f32 = 0.15;
pub const CROUCH_ROT_TARGET: f32 = 0.30;
pub const JUMP_ROT_TARGET: f32 = 0.45;
pub const SPRING_STRENGTH: f32 = 0.5;
pub const SPRING_DAMPING_BASE: f32 = 0.98;
pub const SPRING_DAMPING_HEALTH_COEF: f32 = 0.06;
pub const UNSTABLE_SPRING_K: f32 = 0.05;
pub const DYING_SPRING_K_SCALAR: f32 = 0.5;
pub const DYING_DURATION_MS: u32 = 125;
pub const STABLE_RECOVER_MS: u32 = 1500;
pub const PRONE_TRANSITION_MS: u32 = 333;
pub const PRONE_GOSPRING_K: f32 = 0.4;
pub const PRONE_HOLD_SPRING_K: f32 = 0.65;
pub const PRONE_DAMP_FACTOR: f32 = 0.85;
pub const FG_ARM_FLAIL_SCALAR: f32 = 0.0;
pub const BG_ARM_FLAIL_SCALAR: f32 = 0.7;
pub const ARM_SWING_RATE: f32 = 1.0;
pub const DEVICE_ARM_SWAY_RATE: f32 = 0.5;
pub const LOOK_TO_AIM_RATIO: f32 = 0.7;
pub const HEAD_SMOOTHING: f32 = 0.15;
pub const THROW_PREP_MS: u32 = 1000;
pub const PUSH_FORCE_ESCALATION_MS: u32 = 500;
pub const WALK_ANGLE_CLAMP_DEG: f32 = 40.0;
pub const WALK_ANGLE_RAY_LENGTH: f32 = 15.0;
pub const WALK_ANGLE_RAY_OFFSET: f32 = 10.0;
pub const WALK_ANGLE_SMOOTHING_PER_SEC: f32 = 4.0;
pub const CROUCH_SPEED_MULT: f32 = 0.5;
pub const MAX_WALKPATH_CROUCH_SHIFT: f32 = 6.0;
pub const MAX_CROUCH_ROTATION: f32 = 0.45;

// ============================================================================
// Jetpack physics
// ============================================================================

pub const MIN_TIME_TO_BEGIN_THRUSTING_MS: u32 = 250;
pub const JET_DEFAULT_ANGLE_RANGE: f32 = 0.6;
pub const JET_AGAINST_TRAVEL_MULT: f32 = 0.5;
pub const JET_PRESSURE_EFFICIENCY_VACUUM: f32 = 1.5;
pub const JET_PRESSURE_EFFICIENCY_EARTH: f32 = 1.0;
pub const JET_PRESSURE_EFFICIENCY_VENUS: f32 = 0.5;
pub const AIR_DRAG_VACUUM_MULTIPLIER: f32 = 0.0;

// ============================================================================
// Mass aggregation
// ============================================================================

pub const MASS_FACTOR_MIN_CLAMP: f32 = 0.25;
pub const MASS_FACTOR_MAX_CLAMP: f32 = 1.2;
pub const BASELINE_MASS_KG: f32 = 80.0;
pub const WOUND_PIXEL_MASS_KG: f32 = 0.01;

// ============================================================================
// Heavy armor + knockdown
// ============================================================================

pub const STAGGER_THRESHOLD_FACTOR: f32 = 5.0;
pub const RICOCHET_ANGLE_THRESHOLD: f32 = std::f32::consts::FRAC_PI_3;
pub const RICOCHET_HARDNESS_FACTOR: f32 = 4.0;
pub const RICOCHET_ENERGY_LOSS: f32 = 0.4;
pub const HEAVY_TROOPER_MASS_KG: f32 = 380.0;
pub const HEAVY_DAMAGE_MULTIPLIER_TORSO: f32 = 0.6;
pub const HEAVY_DAMAGE_MULTIPLIER_LIMB: f32 = 0.75;
pub const HEAVY_STAGGER_FACTOR: f32 = 0.2;
pub const HEAVY_GIB_IMPULSE_TORSO: f32 = 3200.0;

// ============================================================================
// Quick Action UX
// ============================================================================

pub const QUICK_ACTION_OPEN_MS: u32 = 80;
pub const QUICK_ACTION_TIME_SLOW: f32 = 0.25;
pub const QUICK_ACTION_TIME_SLOW_REDUCE_MOTION: f32 = 0.50;
pub const QUICK_ACTION_TAP_MAX_MS: u32 = 120;
pub const QUICK_ACTION_DEADZONE_PX: f32 = 12.0;

// ============================================================================
// Simulation overlay — material/walk modulators
// ============================================================================

pub const WALK_FRICTION_DRY: f32 = 1.0;
pub const WALK_FRICTION_WET: f32 = 0.4;
pub const WALK_FRICTION_OIL: f32 = 0.2;
pub const WALK_FRICTION_ICE: f32 = 0.2;
pub const WALK_FRICTION_SAND: f32 = 0.7;
pub const WALK_FRICTION_MUD: f32 = 0.5;
pub const WALK_SPEED_SNOW_MULT: f32 = 0.6;
pub const WALK_SPEED_MUD_MULT: f32 = 0.4;
pub const LAVA_FOOT_DAMAGE_HP_PER_TICK: f32 = 5.0;
pub const ACID_ARMOR_DECAL_RATE_PER_TICK: f32 = 0.1;
pub const ELECTRIC_TILE_SHOCK_THRESHOLD_J: f32 = 100.0;
pub const RADIATION_DOSE_PER_STRIDE_REM: f32 = 0.5;
pub const STRIDE_HAZARD_CONTACT_DEBOUNCE_MS: u32 = 100;
pub const HEAT_TRANSFER_FOOT_CONTACT_AREA_M2: f32 = 0.04;

// ============================================================================
// Atmosphere walk-speed modulators
// ============================================================================

pub const WALK_SPEED_HYPOXIA_MULT: f32 = 0.85;
pub const WALK_SPEED_HYPERTHERMIC_MULT: f32 = 0.9;
pub const WALK_SPEED_HYPOTHERMIC_MULT: f32 = 0.75;
pub const WALK_SPEED_TOXIC_STAMINA_MULT: f32 = 2.0;

// ============================================================================
// Universal gravity field
// ============================================================================

pub const LOW_G_THRESHOLD_M_PER_S2: f32 = 4.9;
pub const LOW_G_JUMP_ARC_MULTIPLIER: f32 = 2.0;

// ============================================================================
// Affliction-on-stride
// ============================================================================

pub const BLEEDING_TO_UNSTABLE_HP_RATIO: f32 = 0.3;
pub const FROZEN_GAIT_MULT: f32 = 0.75;
pub const FROZEN_TRANSITION_MULT: f32 = 2.0;
pub const SHOCKED_STRIDE_FREEZE_MS: u32 = 200;

// ============================================================================
// EM disruption — QAB slots disabled (electronic ability slots 6/7/8 = 5/6/7 in 0-indexed)
// ============================================================================

pub const EM_DISRUPTION_QAB_SLOTS_DISABLED: &[u8] = &[5, 6, 7];

// ============================================================================
// Suit life-support (Stationeers comfort band — survivable is in cf-atmos)
// ============================================================================

pub const SUIT_PRESSURE_COMFORT_MIN_KPA: f32 = 50.0;
pub const SUIT_PRESSURE_COMFORT_MAX_KPA: f32 = 100.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_locked_constants_match_table() {
        assert_eq!(WALK_ROT_TARGET, 0.15);
        assert_eq!(SPRING_DAMPING_BASE, 0.98);
        assert_eq!(PRONE_TRANSITION_MS, 333);
        assert_eq!(BG_ARM_FLAIL_SCALAR, 0.7);
        assert_eq!(STAGGER_THRESHOLD_FACTOR, 5.0);
        assert_eq!(HEAVY_TROOPER_MASS_KG, 380.0);
        assert_eq!(QUICK_ACTION_OPEN_MS, 80);
        assert_eq!(QUICK_ACTION_TIME_SLOW, 0.25);
        assert_eq!(LAVA_FOOT_DAMAGE_HP_PER_TICK, 5.0);
        assert_eq!(MIN_TIME_TO_BEGIN_THRUSTING_MS, 250);
        assert_eq!(LOW_G_THRESHOLD_M_PER_S2, 4.9);
        assert_eq!(EM_DISRUPTION_QAB_SLOTS_DISABLED, &[5, 6, 7]);
    }

    #[test]
    fn ricochet_angle_threshold_is_pi_over_three() {
        assert!((RICOCHET_ANGLE_THRESHOLD - std::f32::consts::FRAC_PI_3).abs() < 1e-6);
    }
}
