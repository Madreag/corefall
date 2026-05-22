//! M9C: per-kind wire visual layer registry.
//!
//! Spec §"Crates / modules touched":
//!
//! > `cf-render-2d` — wire visuals (barbed / razor / electric /
//! > concertina) + camo netting overlay.
//!
//! Per kind:
//!
//! | Kind                | Texture id                  | Tint (RGB)       |
//! |---------------------|-----------------------------|------------------|
//! | barbed_wire         | wire_barbed_strand          | (96, 64, 48)     |
//! | razor_wire          | wire_razor_strand           | (192, 192, 192)  |
//! | electrified_fence   | wire_electric_strand        | (220, 200, 96)   |
//! | concertina_roll     | wire_concertina_coil        | (160, 120, 80)   |
//!
//! VAL-M9C-051 lands here.

use serde::{Deserialize, Serialize};

use cf_fortification::WireKind;

/// One render-layer descriptor for a wire instance: texture id +
/// tint + powered-pulse flag. cf-app's renderer drives the per-frame
/// shimmer animation off the `powered_pulse` field; the M9C kernel
/// only owns the static metadata.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WireVisual {
    pub texture_id: &'static str,
    pub tint_rgb: [u8; 3],
    /// True for the `electrified_fence` kind: the renderer should
    /// pulse the wire's brightness based on the M29 power-grid state.
    pub powered_pulse: bool,
}

impl WireVisual {
    /// Wire-kind metadata table (the spec's per-kind sprite-layer
    /// table above).
    #[must_use]
    pub const fn for_kind(kind: WireKind) -> Self {
        match kind {
            WireKind::BarbedWire => WireVisual {
                texture_id: "wire_barbed_strand",
                tint_rgb: [96, 64, 48],
                powered_pulse: false,
            },
            WireKind::RazorWire => WireVisual {
                texture_id: "wire_razor_strand",
                tint_rgb: [192, 192, 192],
                powered_pulse: false,
            },
            WireKind::ElectrifiedFence => WireVisual {
                texture_id: "wire_electric_strand",
                tint_rgb: [220, 200, 96],
                powered_pulse: true,
            },
            WireKind::ConcertinaRoll => WireVisual {
                texture_id: "wire_concertina_coil",
                tint_rgb: [160, 120, 80],
                powered_pulse: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_visuals_are_pairwise_distinct() {
        let mut seen = vec![];
        for kind in WireKind::ALL {
            let v = WireVisual::for_kind(kind);
            assert!(
                !v.texture_id.is_empty(),
                "{kind:?} must have non-empty texture id"
            );
            for prev in &seen {
                let (id, tint, _): (&str, [u8; 3], bool) = *prev;
                assert!(
                    id != v.texture_id,
                    "{kind:?} texture id collides with earlier kind"
                );
                assert!(
                    tint != v.tint_rgb,
                    "{kind:?} tint collides with earlier kind"
                );
            }
            seen.push((v.texture_id, v.tint_rgb, v.powered_pulse));
        }
    }

    /// Only the electrified_fence powered_pulses; barbed/razor/
    /// concertina are static.
    #[test]
    fn wire_visuals_powered_pulse_only_for_electrified() {
        assert!(!WireVisual::for_kind(WireKind::BarbedWire).powered_pulse);
        assert!(!WireVisual::for_kind(WireKind::RazorWire).powered_pulse);
        assert!(WireVisual::for_kind(WireKind::ElectrifiedFence).powered_pulse);
        assert!(!WireVisual::for_kind(WireKind::ConcertinaRoll).powered_pulse);
    }
}
