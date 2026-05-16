//! M6: suppressor attachment.
//!
//! Spec § "Suppressor attachment reduces loudness by 60%" → loudness × 0.4.

use serde::{Deserialize, Serialize};

/// M6 § suppressor loudness factor. Mirrors
/// [`cf_perception::SUPPRESSOR_LOUDNESS_FACTOR`] but is owned here as the
/// canonical equipment-side constant.
pub const SUPPRESSOR_LOUDNESS_FACTOR: f32 = 0.4;

/// One suppressor attachment + its mounted state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Suppressor {
    pub attached: bool,
    /// Per-use degradation 0..1; suppressor breaks at 0. Configurable so
    /// scenarios can ship indestructible suppressors for tutorials.
    pub integrity: f32,
}

impl Default for Suppressor {
    fn default() -> Self {
        Self {
            attached: false,
            integrity: 1.0,
        }
    }
}

impl Suppressor {
    pub fn attached_default() -> Self {
        Self {
            attached: true,
            integrity: 1.0,
        }
    }

    /// Returns the effective loudness multiplier for the next shot. When the
    /// suppressor is attached and intact, returns the spec factor; otherwise 1.0.
    pub fn loudness_factor(self) -> f32 {
        if self.attached && self.integrity > 0.0 {
            SUPPRESSOR_LOUDNESS_FACTOR
        } else {
            1.0
        }
    }

    /// Apply per-shot wear; returns true if the suppressor just broke.
    pub fn apply_wear(&mut self, wear: f32) -> bool {
        if !self.attached {
            return false;
        }
        if !wear.is_finite() {
            return false;
        }
        let was_alive = self.integrity > 0.0;
        self.integrity = (self.integrity - wear).clamp(0.0, 1.0);
        was_alive && self.integrity <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_suppressor_reduces_loudness() {
        let s = Suppressor::attached_default();
        assert!((s.loudness_factor() - 0.4).abs() < 1e-3);
    }

    #[test]
    fn detached_full_loudness() {
        let s = Suppressor::default();
        assert!((s.loudness_factor() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn wear_breaks_suppressor() {
        let mut s = Suppressor::attached_default();
        assert!(!s.apply_wear(0.5));
        let broke = s.apply_wear(1.0);
        assert!(broke);
        assert_eq!(s.integrity, 0.0);
        assert!((s.loudness_factor() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn nan_wear_rejected() {
        let mut s = Suppressor::attached_default();
        assert!(!s.apply_wear(f32::NAN));
        assert_eq!(s.integrity, 1.0);
    }
}
