//! **M14A** § "cf-render-2d::armor_scratch_overlay" — per-zone scratch decal
//! progression at L1/L2/L3 thresholds + armor-breach glow hint.

use serde::{Deserialize, Serialize};

/// Per-zone decal data the renderer overlays on top of the heavy chassis sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScratchOverlay {
    /// Level 0..3 (0 = no decals; 1..3 = L1/L2/L3).
    pub level: u8,
    /// `true` when the External layer has been breached → red breach glow on.
    pub breached: bool,
}

impl ScratchOverlay {
    /// external HP integrity (1.0 = full; 0.0 = breached).
    pub fn for_external_integrity(integrity: f32) -> Self {
        let level = if integrity >= 0.7 {
            1
        } else if integrity >= 0.4 {
            2
        } else if integrity > 0.0 {
            3
        } else {
            0
        };
        Self {
            level,
            breached: integrity <= 0.0,
        }
    }

    pub fn decal_asset_id(self) -> Option<&'static str> {
        match self.level {
            1 => Some("armor_scratch_decal_l1"),
            2 => Some("armor_scratch_decal_l2"),
            3 => Some("armor_scratch_decal_l3"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_advances_with_damage() {
        assert_eq!(ScratchOverlay::for_external_integrity(0.9).level, 1);
        assert_eq!(ScratchOverlay::for_external_integrity(0.5).level, 2);
        assert_eq!(ScratchOverlay::for_external_integrity(0.2).level, 3);
    }

    #[test]
    fn breach_flag_set_when_external_dead() {
        let o = ScratchOverlay::for_external_integrity(0.0);
        assert!(o.breached);
    }
}
