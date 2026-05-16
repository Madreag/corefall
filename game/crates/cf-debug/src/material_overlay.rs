//! F4 — Material affordance overlay. Cursor tooltip: material name +
//! integrity_band + 9 affordance flags per spec § Debug overlays.

use serde::{Deserialize, Serialize};

/// 9 affordance flags surfaced in the cursor tooltip per spec § F4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffordanceFlags {
    /// Surface can be dug.
    pub diggable: bool,
    /// Surface can be welded / repaired.
    pub repairable: bool,
    /// Surface conducts heat.
    pub conducts_heat: bool,
    /// Surface conducts electricity.
    pub conducts_power: bool,
    /// Surface can be vaulted over.
    pub vaultable: bool,
    /// Surface can be climbed.
    pub climbable: bool,
    /// Surface absorbs sound.
    pub absorbs_sound: bool,
    /// Surface produces sparks on impact.
    pub sparks_on_impact: bool,
    /// Surface is anchorable (per M3 cf-control `anchor_material_result`).
    pub anchorable: bool,
}

impl AffordanceFlags {
    /// Number of flags set true (used by the renderer for compact display).
    pub fn count_set(self) -> u8 {
        [
            self.diggable,
            self.repairable,
            self.conducts_heat,
            self.conducts_power,
            self.vaultable,
            self.climbable,
            self.absorbs_sound,
            self.sparks_on_impact,
            self.anchorable,
        ]
        .into_iter()
        .filter(|b| *b)
        .count() as u8
    }
}

/// Cursor tooltip render data for the material overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialOverlayTooltip {
    /// Cursor world position the tooltip describes.
    pub cursor: (f32, f32),
    /// Material name (e.g. `Concrete`, `SteelPlate`).
    pub material_name: String,
    /// Integrity band (e.g. `Pristine`, `Damaged`, `Critical`).
    pub integrity_band: String,
    /// 9 affordance flags.
    pub affordances: AffordanceFlags,
}

/// Aggregated overlay payload (single tooltip per cursor).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialOverlayData {
    /// Optional cursor tooltip; `None` when the cursor is off-world.
    pub tooltip: Option<MaterialOverlayTooltip>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affordance_count_set_zero_by_default() {
        let a = AffordanceFlags::default();
        assert_eq!(a.count_set(), 0);
    }

    #[test]
    fn affordance_count_set_works() {
        let a = AffordanceFlags {
            diggable: true,
            climbable: true,
            anchorable: true,
            ..AffordanceFlags::default()
        };
        assert_eq!(a.count_set(), 3);
    }

    #[test]
    fn tooltip_round_trips() {
        let t = MaterialOverlayTooltip {
            cursor: (1.0, 2.0),
            material_name: "Steel".into(),
            integrity_band: "Pristine".into(),
            affordances: AffordanceFlags {
                diggable: true,
                ..AffordanceFlags::default()
            },
        };
        let s = serde_json::to_string(&t).unwrap();
        let back: MaterialOverlayTooltip = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }
}
