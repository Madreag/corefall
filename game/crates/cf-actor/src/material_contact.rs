//! **M14A** § "Per-stride material contact resolver".
//!
//! Per-stride material contact: friction → slip event, thermal contact,
//! hazard contact, audio cue, wound mass.

use serde::{Deserialize, Serialize};

/// One per-stride contact resolved against a material modulator.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MaterialContact {
    /// True when friction × lateral velocity > slip threshold.
    pub foot_slip: bool,
    /// True if this stride should emit a hazard.actor_contact event.
    pub hazard_emit: bool,
    /// Hazard kind label (fire / acid / electric / wet / ...).
    pub hazard_kind: String,
    /// Damage to apply to the planted-foot zone this tick (HP).
    pub foot_damage_hp: f32,
    /// Footstep audio cue id ("footstep_<material>"; default
    /// "footstep_generic").
    pub footstep_cue: String,
    /// Surface friction at planted foot (post-modulator).
    pub friction_at_contact: f32,
    /// Effective walk speed multiplier.
    pub walk_speed_mult: f32,
}

/// stride's worth of material contact effects.
///
/// `friction_mult` and `speed_mult` come from
/// `cf_terrain::material_walk_modulator()`.
pub fn resolve_material_contact(
    friction_mult: f32,
    speed_mult: f32,
    emit_hazard: bool,
    foot_damage_hp: f32,
    hazard_kind: &str,
    material_id: u8,
    lateral_velocity_px_per_s: f32,
    origin_id: &str,
) -> MaterialContact {
    let friction = friction_mult.clamp(0.0, 2.0);
    let slip_threshold = 30.0; // px/s lateral above which low-friction causes slip
    let foot_slip = friction < 0.5 && lateral_velocity_px_per_s.abs() > slip_threshold;

    let suffix = match origin_id {
        "robot" | "synth" => "_synthetic",
        "android" | "hybrid" => "_hybrid",
        _ => "_organic",
    };
    let mat_name = match material_id {
        2 => "concrete",
        3 => "metal",
        4 => "hazard",
        5 => "loose_fill",
        6 => "repair_fill",
        7 => "anchor",
        12 => "lava",
        13 => "acid",
        14 => "ice",
        15 => "snow",
        16 => "oil",
        17 => "mud",
        18 => "water",
        _ => "generic",
    };
    let footstep_cue = format!("footstep_{}{}", mat_name, suffix);

    MaterialContact {
        foot_slip,
        hazard_emit: emit_hazard,
        hazard_kind: hazard_kind.to_string(),
        foot_damage_hp,
        footstep_cue,
        friction_at_contact: friction,
        walk_speed_mult: speed_mult,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_causes_foot_slip_when_moving() {
        let c = resolve_material_contact(0.2, 0.7, false, 0.0, "", 14, 80.0, "human");
        assert!(c.foot_slip);
        assert_eq!(c.footstep_cue, "footstep_ice_organic");
    }

    #[test]
    fn lava_emits_hazard_with_burn_damage() {
        let c = resolve_material_contact(0.6, 0.5, true, 5.0, "fire", 12, 0.0, "human");
        assert!(c.hazard_emit);
        assert_eq!(c.hazard_kind, "fire");
        assert!((c.foot_damage_hp - 5.0).abs() < 1e-6);
    }

    #[test]
    fn synthetic_origin_uses_synthetic_cue_suffix() {
        let c = resolve_material_contact(1.0, 1.0, false, 0.0, "", 2, 0.0, "robot");
        assert_eq!(c.footstep_cue, "footstep_concrete_synthetic");
    }
}
