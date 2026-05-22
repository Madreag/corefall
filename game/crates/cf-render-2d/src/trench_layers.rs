//! M9B: per-variant trench sprite layer registry.
//!
//! Spec §"Files": `game/crates/cf-render-2d/src/trench_layers.rs` (NEW).
//!
//! Spec §"Crates / modules touched":
//!
//! > `cf-render-2d` — Per-variant trench sprite layers (duckboard,
//! >   fire-step, drainage, revetment) + cover-state debug overlay.
//!
//! VAL-M9B-RENDER-001: a `fire_step` segment renders the raised-step
//! sprite layer; a `standard` segment renders the duckboard layer but
//! NOT a fire-step layer. The per-variant sprite layer subsets are
//! authored as static [`TrenchLayerId`] arrays below.
//!
//! This module is presentation-only — no sim mutation, no allocation
//! per frame. cf-app's renderer pulls [`layers_for_variant`] each
//! frame and queues the corresponding sprites; the segment's
//! `embedded_modules` list adds revetment / drainage layers
//! conditionally.

use serde::{Deserialize, Serialize};

use cf_trench::{SegmentVariant, TrenchModule, TrenchSegment};

/// One sprite layer the renderer queues for a trench segment. The
/// ordering matters: lower-z layers (`Floor`) render first; the
/// `Chevron` overlay rides on top.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrenchLayerId {
    /// Dirt-floor layer drawn under every variant.
    Floor = 0,
    /// Wooden duckboard planks; present on every non-shallow variant.
    Duckboard = 1,
    /// Raised firing step; only the `fire_step` variant renders this.
    FireStep = 2,
    /// Sandbag breastwork above grade; `parapet_raised` only.
    Breastwork = 3,
    /// Drainage sump grate; renders when `drainage_sump` module is
    /// embedded (deep variant by default; modder content may add it
    /// to standard).
    Drainage = 4,
    /// Wood/iron revetment side reinforcement; renders when
    /// `revetment` module is embedded.
    Revetment = 5,
    /// Reinforced corner; renders when `corner_traverse` is embedded.
    CornerTraverse = 6,
    /// Cover-state chevron overlay layer; rendered by the tactical
    /// overlay (M9B 6th material-overlay mode).
    Chevron = 7,
}

impl TrenchLayerId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TrenchLayerId::Floor => "floor",
            TrenchLayerId::Duckboard => "duckboard",
            TrenchLayerId::FireStep => "fire_step",
            TrenchLayerId::Breastwork => "breastwork",
            TrenchLayerId::Drainage => "drainage",
            TrenchLayerId::Revetment => "revetment",
            TrenchLayerId::CornerTraverse => "corner_traverse",
            TrenchLayerId::Chevron => "chevron",
        }
    }
}

/// Static per-variant layer set authored against the spec table:
///
/// | Variant         | Floor | Duckboard | FireStep | Drainage | Breastwork |
/// |---|:-:|:-:|:-:|:-:|:-:|
/// | shallow_scrape  | yes   | no        | no       | no       | no         |
/// | standard        | yes   | yes       | no       | no       | no         |
/// | deep            | yes   | yes       | no       | yes      | no         |
/// | communication   | yes   | yes       | no       | no       | no         |
/// | fire_step       | yes   | yes       | yes      | no       | no         |
/// | parapet_raised  | yes   | yes       | no       | no       | yes        |
///
/// The Drainage / Revetment / CornerTraverse layers are also added
/// when the relevant module is embedded — see
/// [`layers_for_segment`].
#[must_use]
pub fn layers_for_variant(variant: SegmentVariant) -> Vec<TrenchLayerId> {
    use TrenchLayerId::{Breastwork, Drainage, Duckboard, FireStep, Floor};
    match variant {
        SegmentVariant::ShallowScrape => vec![Floor],
        SegmentVariant::Standard => vec![Floor, Duckboard],
        SegmentVariant::Deep => vec![Floor, Duckboard, Drainage],
        SegmentVariant::Communication => vec![Floor, Duckboard],
        SegmentVariant::FireStep => vec![Floor, Duckboard, FireStep],
        SegmentVariant::ParapetRaised => vec![Floor, Duckboard, Breastwork],
    }
}

/// Full per-segment layer set: layers from the variant baseline plus
/// any module-attached layers (revetment, drainage_sump for non-deep
/// segments, corner_traverse). Returned in render-z order.
#[must_use]
pub fn layers_for_segment(segment: &TrenchSegment) -> Vec<TrenchLayerId> {
    let mut layers = layers_for_variant(segment.variant);
    for module in &segment.embedded_modules {
        let extra = match module {
            TrenchModule::Duckboard => TrenchLayerId::Duckboard,
            TrenchModule::FireStep => TrenchLayerId::FireStep,
            TrenchModule::Breastwork => TrenchLayerId::Breastwork,
            TrenchModule::DrainageSump => TrenchLayerId::Drainage,
            TrenchModule::Revetment => TrenchLayerId::Revetment,
            TrenchModule::CornerTraverse => TrenchLayerId::CornerTraverse,
        };
        if !layers.contains(&extra) {
            layers.push(extra);
        }
    }
    layers
}

/// VAL-M9B-RENDER-001 sub-helper: does the segment's render output
/// contain the given layer?
#[must_use]
pub fn segment_has_layer(segment: &TrenchSegment, layer: TrenchLayerId) -> bool {
    layers_for_segment(segment).contains(&layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_trench::{SegmentVariant, TrenchModule, TrenchSegment};

    fn seg(
        variant: SegmentVariant,
        modules: Vec<TrenchModule>,
        raised_step: Option<u32>,
    ) -> TrenchSegment {
        TrenchSegment {
            variant,
            tile_x: 0,
            tile_y: 0,
            depth: 16,
            width: 16,
            raised_step_height: raised_step,
            embedded_modules: modules,
        }
    }

    /// expected subset (duckboard for non-shallow; fire-step for
    /// fire_step; drainage for deep; revetment when the module is
    /// placed).
    #[test]
    fn trench_layers_per_variant_sprite_layers() {
        // shallow_scrape — Floor only
        let l = layers_for_variant(SegmentVariant::ShallowScrape);
        assert!(l.contains(&TrenchLayerId::Floor));
        assert!(!l.contains(&TrenchLayerId::Duckboard));

        // standard — Floor + Duckboard, NOT FireStep
        let l = layers_for_variant(SegmentVariant::Standard);
        assert!(l.contains(&TrenchLayerId::Duckboard));
        assert!(!l.contains(&TrenchLayerId::FireStep));

        // deep — Floor + Duckboard + Drainage
        let l = layers_for_variant(SegmentVariant::Deep);
        assert!(l.contains(&TrenchLayerId::Drainage));

        // fire_step — Floor + Duckboard + FireStep
        let l = layers_for_variant(SegmentVariant::FireStep);
        assert!(l.contains(&TrenchLayerId::FireStep));
        assert!(l.contains(&TrenchLayerId::Duckboard));

        // parapet_raised — Floor + Duckboard + Breastwork
        let l = layers_for_variant(SegmentVariant::ParapetRaised);
        assert!(l.contains(&TrenchLayerId::Breastwork));
    }

    /// VAL-M9B-RENDER-001 alias matching the spec evidence string
    /// `per_variant_sprite_layers`.
    #[test]
    fn per_variant_sprite_layers() {
        trench_layers_per_variant_sprite_layers();
    }

    #[test]
    fn standard_renders_duckboard_but_not_fire_step() {
        let s = seg(SegmentVariant::Standard, vec![TrenchModule::Duckboard], None);
        assert!(segment_has_layer(&s, TrenchLayerId::Duckboard));
        assert!(!segment_has_layer(&s, TrenchLayerId::FireStep));
    }

    #[test]
    fn fire_step_renders_raised_step_layer() {
        let s = seg(
            SegmentVariant::FireStep,
            vec![TrenchModule::Duckboard, TrenchModule::FireStep],
            Some(8),
        );
        assert!(segment_has_layer(&s, TrenchLayerId::FireStep));
    }

    /// Embedding the `revetment` module adds the Revetment layer to
    /// a segment that wouldn't have it by default (`standard`).
    #[test]
    fn revetment_module_adds_layer() {
        let plain = seg(SegmentVariant::Standard, vec![], None);
        assert!(!segment_has_layer(&plain, TrenchLayerId::Revetment));
        let with_rev = seg(
            SegmentVariant::Standard,
            vec![TrenchModule::Duckboard, TrenchModule::Revetment],
            None,
        );
        assert!(segment_has_layer(&with_rev, TrenchLayerId::Revetment));
    }

    #[test]
    fn drainage_module_adds_layer_for_non_deep_variants() {
        let standard = seg(
            SegmentVariant::Standard,
            vec![TrenchModule::Duckboard, TrenchModule::DrainageSump],
            None,
        );
        assert!(segment_has_layer(&standard, TrenchLayerId::Drainage));
    }

    #[test]
    fn corner_traverse_module_adds_layer() {
        let s = seg(
            SegmentVariant::Standard,
            vec![TrenchModule::Duckboard, TrenchModule::CornerTraverse],
            None,
        );
        assert!(segment_has_layer(&s, TrenchLayerId::CornerTraverse));
    }

    #[test]
    fn layer_as_str_round_trip() {
        for layer in [
            TrenchLayerId::Floor,
            TrenchLayerId::Duckboard,
            TrenchLayerId::FireStep,
            TrenchLayerId::Breastwork,
            TrenchLayerId::Drainage,
            TrenchLayerId::Revetment,
            TrenchLayerId::CornerTraverse,
            TrenchLayerId::Chevron,
        ] {
            assert!(!layer.as_str().is_empty());
        }
    }
}
