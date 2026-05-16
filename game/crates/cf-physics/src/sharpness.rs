//! **M14**: bullet sharpness decay over distance per CCCP `MOPixel::Update`.
//!
//! A projectile loses *effective* damage as it travels beyond its weapon's
//! effective range. Within the effective range the projectile carries full
//! damage; from `effective_range` to `max_range` damage decays linearly to
//! zero; past `max_range` the projectile despawns. Sharpness is the
//! same coefficient applied to penetration (per `cf_physics::try_penetrate`),
//! so sharpness also decays — a projectile that has traveled past its
//! effective range loses both lethality AND material-penetration power.
//!
//! All functions are pure; deterministic; no clocks; no `thread_rng`.

use serde::{Deserialize, Serialize};

/// Decay band per CCCP `MOPixel::Update` lethality range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecayBand {
    /// Distance < effective_range. Damage is full; sharpness unaffected.
    InRange,
    /// effective_range <= distance < max_range. Damage decays linearly.
    Decaying,
    /// Distance >= max_range. Projectile expires.
    Expired,
}

/// Inputs for [`decay_damage`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SharpnessInputs {
    /// Distance the projectile has traveled (world units, e.g. pixels).
    pub distance_traveled: f32,
    /// Effective range of the weapon (units). Below this, damage is full.
    pub effective_range: f32,
    /// Maximum range (units). Past this, projectile expires.
    pub max_range: f32,
    /// Base damage (HP).
    pub base_damage: f32,
    /// Base sharpness in [0, 1]. Decays alongside damage past effective_range.
    pub base_sharpness: f32,
}

/// Result of [`decay_damage`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SharpnessOutcome {
    pub band: DecayBand,
    pub decayed_damage: f32,
    pub decayed_sharpness: f32,
    pub expired: bool,
}

/// Apply sharpness decay. Linear falloff between `effective_range` and
/// `max_range`; past `max_range` damage = 0 and `expired = true`.
///
/// `effective_range <= 0` collapses to a perpetually-full-damage round (for
/// projectiles like beam weapons that don't decay). `max_range <=
/// effective_range` collapses to a no-decay round.
#[must_use]
pub fn decay_damage(inputs: SharpnessInputs) -> SharpnessOutcome {
    let dist = inputs.distance_traveled.max(0.0);
    let eff = inputs.effective_range.max(0.0);
    let max = inputs.max_range.max(eff);
    let base = inputs.base_damage.max(0.0);
    let sharp = inputs.base_sharpness.clamp(0.0, 1.0);
    if eff <= f32::EPSILON || max <= f32::EPSILON {
        return SharpnessOutcome {
            band: DecayBand::InRange,
            decayed_damage: base,
            decayed_sharpness: sharp,
            expired: false,
        };
    }
    if dist >= max {
        return SharpnessOutcome {
            band: DecayBand::Expired,
            decayed_damage: 0.0,
            decayed_sharpness: 0.0,
            expired: true,
        };
    }
    if dist < eff {
        return SharpnessOutcome {
            band: DecayBand::InRange,
            decayed_damage: base,
            decayed_sharpness: sharp,
            expired: false,
        };
    }
    // Linear falloff in the decay band.
    let span = (max - eff).max(f32::EPSILON);
    let t = ((dist - eff) / span).clamp(0.0, 1.0);
    let damage = (base * (1.0 - t)).max(0.0);
    let sharpness = (sharp * (1.0 - t)).max(0.0);
    SharpnessOutcome {
        band: DecayBand::Decaying,
        decayed_damage: damage,
        decayed_sharpness: sharpness,
        expired: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rifle() -> SharpnessInputs {
        SharpnessInputs {
            distance_traveled: 0.0,
            effective_range: 100.0,
            max_range: 500.0,
            base_damage: 25.0,
            base_sharpness: 0.8,
        }
    }

    #[test]
    fn in_range_full_damage() {
        let mut inputs = rifle();
        inputs.distance_traveled = 50.0;
        let out = decay_damage(inputs);
        assert!(matches!(out.band, DecayBand::InRange));
        assert!((out.decayed_damage - 25.0).abs() < 1e-3);
        assert!((out.decayed_sharpness - 0.8).abs() < 1e-3);
        assert!(!out.expired);
    }

    #[test]
    fn at_effective_range_full_damage() {
        let mut inputs = rifle();
        inputs.distance_traveled = 100.0;
        let out = decay_damage(inputs);
        // At exactly effective_range, decay band starts (t = 0 → full damage).
        assert!(matches!(out.band, DecayBand::Decaying));
        assert!((out.decayed_damage - 25.0).abs() < 1e-3);
    }

    #[test]
    fn midway_decay_band_half_damage() {
        let mut inputs = rifle();
        inputs.distance_traveled = 300.0; // halfway between 100 and 500
        let out = decay_damage(inputs);
        assert!(matches!(out.band, DecayBand::Decaying));
        assert!((out.decayed_damage - 12.5).abs() < 1e-2);
    }

    #[test]
    fn past_max_range_expires() {
        let mut inputs = rifle();
        inputs.distance_traveled = 600.0;
        let out = decay_damage(inputs);
        assert!(matches!(out.band, DecayBand::Expired));
        assert!(out.decayed_damage.abs() < 1e-3);
        assert!(out.expired);
    }

    #[test]
    fn effective_range_zero_collapses_to_full() {
        let mut inputs = rifle();
        inputs.effective_range = 0.0;
        inputs.distance_traveled = 9999.0;
        let out = decay_damage(inputs);
        assert!((out.decayed_damage - 25.0).abs() < 1e-3);
    }

    #[test]
    fn sharpness_decays_with_damage() {
        let mut inputs = rifle();
        inputs.distance_traveled = 300.0;
        let out = decay_damage(inputs);
        assert!(out.decayed_sharpness < inputs.base_sharpness);
        assert!(out.decayed_sharpness > 0.0);
    }
}
