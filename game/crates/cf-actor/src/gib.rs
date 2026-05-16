//! **M14**: gib spawn registry + cascade rules per CCCP
//! `MOSRotating::CreateGibsWhenGibbing` + `RemoveAttachablesWhenGibbing`.
//!
//! A [`GibSpawn`] is the authored data for one gib batch (which particle
//! to clone, how many copies, angular spread, velocity range, etc.). When
//! a parent attachable gibs, each authored gib batch fires + each child
//! attachable cascades (gibs independently per its own spawn data).
//!
//! Determinism: deterministic spread modes are pure; the random spread
//! mode takes a seeded `[0, 1)` RNG roll per particle.

use serde::{Deserialize, Serialize};

/// Spread mode per CCCP `SpreadMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SpreadMode {
    /// Particles spawn at random angles within `m_Spread`. Requires per-
    /// particle RNG rolls (deterministic given seeded RNG).
    SpreadRandom,
    /// Particles spawn at evenly-distributed angles within `m_Spread`. No RNG.
    SpreadEven,
    /// Particles spawn along an outward spiral. No RNG.
    SpreadSpiral,
}

/// Origin kind for gib visual treatment per spec § "Per-origin: red blood
/// (human), oil (robot), synth-blood (android), bio-fluid (biomech)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GibOriginKind {
    Human,
    Robot,
    Android,
    Biomech,
    Other,
}

impl GibOriginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GibOriginKind::Human => "human",
            GibOriginKind::Robot => "robot",
            GibOriginKind::Android => "android",
            GibOriginKind::Biomech => "biomech",
            GibOriginKind::Other => "other",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "human" => GibOriginKind::Human,
            "robot" => GibOriginKind::Robot,
            "android" => GibOriginKind::Android,
            "biomech" => GibOriginKind::Biomech,
            _ => GibOriginKind::Other,
        }
    }
}

/// Authored spawn data for one gib batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GibSpawn {
    pub particle: String,
    pub count: u32,
    pub spread_radians: f32,
    pub min_velocity: f32,
    pub max_velocity: f32,
    pub life_variation: f32,
    pub inherits_velocity: bool,
    pub inherits_angular_velocity: bool,
    pub ignores_team_hits: bool,
    pub spread_mode: SpreadMode,
}

impl GibSpawn {
    /// Default authored gib for an organic limb (red blood, ~6 droplets,
    /// ~30 degree spread).
    pub fn default_organic() -> Self {
        Self {
            particle: "blood_pixel".to_string(),
            count: 6,
            spread_radians: std::f32::consts::FRAC_PI_6,
            min_velocity: 50.0,
            max_velocity: 200.0,
            life_variation: 0.25,
            inherits_velocity: true,
            inherits_angular_velocity: false,
            ignores_team_hits: true,
            spread_mode: SpreadMode::SpreadRandom,
        }
    }

    /// Default authored gib for a robotic limb (oil, ~4 droplets, even spread).
    pub fn default_robotic() -> Self {
        Self {
            particle: "oil_pixel".to_string(),
            count: 4,
            spread_radians: std::f32::consts::FRAC_PI_4,
            min_velocity: 30.0,
            max_velocity: 150.0,
            life_variation: 0.10,
            inherits_velocity: true,
            inherits_angular_velocity: false,
            ignores_team_hits: true,
            spread_mode: SpreadMode::SpreadEven,
        }
    }

    /// Default authored gib for an android limb (synth-blood, spiral spread).
    pub fn default_android() -> Self {
        Self {
            particle: "synth_blood_pixel".to_string(),
            count: 5,
            spread_radians: std::f32::consts::FRAC_PI_4,
            min_velocity: 40.0,
            max_velocity: 180.0,
            life_variation: 0.20,
            inherits_velocity: true,
            inherits_angular_velocity: false,
            ignores_team_hits: true,
            spread_mode: SpreadMode::SpreadSpiral,
        }
    }

    /// Pick the default gib for an origin kind.
    pub fn default_for_origin(origin: GibOriginKind) -> Self {
        match origin {
            GibOriginKind::Robot => Self::default_robotic(),
            GibOriginKind::Android => Self::default_android(),
            GibOriginKind::Biomech => {
                let mut g = Self::default_organic();
                g.particle = "biofluid_pixel".to_string();
                g
            }
            GibOriginKind::Human | GibOriginKind::Other => Self::default_organic(),
        }
    }
}

/// **M14**: cascade chain from a parent zone to every child attachable
/// that gibs when the parent gibs (per CCCP `RemoveAttachablesWhenGibbing`).
///
/// The map is `parent_zone → list_of_child_zones`. For the default
/// humanoid graph: torso → all limbs; arm → forearm + hand; leg → shin + foot.
#[must_use]
pub fn default_cascade_chain(parent_zone: &str) -> &'static [&'static str] {
    match parent_zone {
        "torso" => &[
            "head",
            "arm_left",
            "arm_right",
            "leg_left",
            "leg_right",
            "backpack",
        ],
        "arm_left" => &["forearm_left", "hand_left"],
        "arm_right" => &["forearm_right", "hand_right"],
        "leg_left" => &["shin_left", "foot_left"],
        "leg_right" => &["shin_right", "foot_right"],
        "forearm_left" => &["hand_left"],
        "forearm_right" => &["hand_right"],
        "shin_left" => &["foot_left"],
        "shin_right" => &["foot_right"],
        _ => &[],
    }
}

/// **M14**: deterministic per-particle angle computation for [`SpreadMode`].
/// `index` is the 0-based particle within the batch; `count` is the total;
/// `spread` is the angular spread in radians; `rng_roll` is a [0, 1) draw
/// (used only by [`SpreadMode::SpreadRandom`]).
#[must_use]
pub fn spread_angle(mode: SpreadMode, index: u32, count: u32, spread: f32, rng_roll: f32) -> f32 {
    let count = count.max(1);
    let half = spread.abs() / 2.0;
    match mode {
        SpreadMode::SpreadRandom => {
            // Uniform in [-half, +half].
            (rng_roll.clamp(0.0, 1.0) * 2.0 - 1.0) * half
        }
        SpreadMode::SpreadEven => {
            if count == 1 {
                0.0
            } else {
                let step = spread / (count - 1) as f32;
                -half + index as f32 * step
            }
        }
        SpreadMode::SpreadSpiral => {
            // Phyllotaxis-style spiral; tightens visually at larger counts.
            let golden = 2.39996_f32; // golden angle ~ pi * (3 - sqrt 5)
            (index as f32 * golden).sin() * half
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_organic_has_red_blood() {
        let g = GibSpawn::default_organic();
        assert_eq!(g.particle, "blood_pixel");
        assert!(g.count >= 1);
        assert_eq!(g.spread_mode, SpreadMode::SpreadRandom);
    }

    #[test]
    fn default_robotic_has_oil() {
        let g = GibSpawn::default_robotic();
        assert_eq!(g.particle, "oil_pixel");
        assert_eq!(g.spread_mode, SpreadMode::SpreadEven);
    }

    #[test]
    fn default_android_has_synth_blood() {
        let g = GibSpawn::default_android();
        assert_eq!(g.particle, "synth_blood_pixel");
        assert_eq!(g.spread_mode, SpreadMode::SpreadSpiral);
    }

    #[test]
    fn cascade_chain_torso_lists_all_limbs() {
        let children = default_cascade_chain("torso");
        assert!(children.contains(&"head"));
        assert!(children.contains(&"arm_left"));
        assert!(children.contains(&"leg_right"));
    }

    #[test]
    fn cascade_chain_unknown_is_empty() {
        let children = default_cascade_chain("turret_left");
        assert!(children.is_empty());
    }

    #[test]
    fn spread_even_two_particles_brackets_zero() {
        let a = spread_angle(SpreadMode::SpreadEven, 0, 2, 1.0, 0.0);
        let b = spread_angle(SpreadMode::SpreadEven, 1, 2, 1.0, 0.0);
        assert!((a - -0.5).abs() < 1e-3);
        assert!((b - 0.5).abs() < 1e-3);
    }

    #[test]
    fn spread_random_in_range() {
        let half = 0.5;
        for roll in [0.0_f32, 0.25, 0.5, 0.75, 0.999] {
            let a = spread_angle(SpreadMode::SpreadRandom, 0, 6, 1.0, roll);
            assert!(a >= -half - 1e-3 && a <= half + 1e-3);
        }
    }

    #[test]
    fn from_str_lossy_round_trips() {
        for k in [
            GibOriginKind::Human,
            GibOriginKind::Robot,
            GibOriginKind::Android,
            GibOriginKind::Biomech,
        ] {
            assert_eq!(GibOriginKind::from_str_lossy(k.as_str()), k);
        }
    }
}
