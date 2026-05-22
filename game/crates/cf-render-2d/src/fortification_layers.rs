//! M9C: per-kind fortification sprite layer registry.
//!
//! Spec §"Files": `game/crates/cf-render-2d/src/fortification_layers.rs`
//! (NEW).
//!
//! Spec §"Crates / modules touched":
//!
//! > `cf-render-2d` — Per-fortification sprite layers, spotlight cone
//! > shader, wire visuals (barbed/razor/electric/concertina), camo
//! > netting overlay.
//!
//! VAL-M9C-051: cf-render-2d ships `fortification_layers`,
//! `spotlight_cone`, and `wire_visuals` modules + camo overlay; per-
//! kind sprite-layer subsets are authored as static [`FortLayerId`]
//! arrays below. The module mirrors the M9B `trench_layers` pattern
//! so cf-app's renderer can pull `layers_for_kind(kind)` each frame
//! and queue the corresponding sprites.

use serde::{Deserialize, Serialize};

use cf_fortification::{
    sandbag::SandbagTier, FortificationKind, SandbagWall,
};

/// One sprite layer the renderer queues for a fortification. Layer
/// ordering matters: lower-z layers (`Base`) render first; the
/// `DamageOverlay` rides on top.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FortLayerId {
    /// Base sprite layer (always rendered for any placed fortification).
    Base = 0,
    /// Per-tier sandbag fill (12/8/4 px tall). Drawn from the bottom
    /// up; eroded rows are masked off.
    SandbagFill = 1,
    /// MG nest barrel + tripod silhouette layer.
    MgBarrel = 2,
    /// Watchtower platform + railings layer.
    TowerPlatform = 3,
    /// Spotlight housing layer (the cone itself is owned by
    /// [`crate::spotlight_cone`]).
    SpotlightHousing = 4,
    /// Wire visual layer (kind-specific texture; barbed / razor /
    /// concertina / electrified).
    WireSprite = 5,
    /// Anti-tank obstacle layer (dragon's teeth / tank trap / bollard
    /// / ditch outline).
    AntiTankSilhouette = 6,
    /// Camo netting overlay (transparent 4×4 fabric texture).
    CamoOverlay = 7,
    /// Mine surface marker (visible only to the sweeping faction; the
    /// cf-app renderer masks the layer per-faction).
    MineMarker = 8,
    /// Per-fortification HP damage overlay (cracks / smoke / partial
    /// destruction). Topmost layer.
    DamageOverlay = 9,
}

impl FortLayerId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FortLayerId::Base => "base",
            FortLayerId::SandbagFill => "sandbag_fill",
            FortLayerId::MgBarrel => "mg_barrel",
            FortLayerId::TowerPlatform => "tower_platform",
            FortLayerId::SpotlightHousing => "spotlight_housing",
            FortLayerId::WireSprite => "wire_sprite",
            FortLayerId::AntiTankSilhouette => "anti_tank_silhouette",
            FortLayerId::CamoOverlay => "camo_overlay",
            FortLayerId::MineMarker => "mine_marker",
            FortLayerId::DamageOverlay => "damage_overlay",
        }
    }
}

/// Static per-kind layer set authored against the spec table. Every
/// fortification renders the `Base` layer; kind-specific layers are
/// added per the spec's gameplay role.
#[must_use]
pub fn layers_for_kind(kind: FortificationKind) -> Vec<FortLayerId> {
    use FortLayerId::*;
    match kind {
        // MG nest module + ammo box + tripod variant + spotter scope +
        // bunker firing slit.
        FortificationKind::MgNestStatic => vec![Base, MgBarrel],
        FortificationKind::AmmoBoxMg => vec![Base],
        FortificationKind::MgTripodPortable => vec![Base, MgBarrel],
        FortificationKind::SpotterScope => vec![Base],
        FortificationKind::BunkerFiringSlit => vec![Base, MgBarrel],

        // Sandbag walls (per-tier fill height authored on the wall).
        FortificationKind::SandbagLow
        | FortificationKind::SandbagMid
        | FortificationKind::SandbagHigh => vec![Base, SandbagFill],

        // Watchtower tier ladder + spotlight + observation post +
        // radio repeater.
        FortificationKind::WatchtowerT1
        | FortificationKind::WatchtowerT2
        | FortificationKind::WatchtowerT3 => vec![Base, TowerPlatform],
        FortificationKind::Spotlight => vec![Base, SpotlightHousing],
        FortificationKind::ObservationPost => vec![Base, TowerPlatform],
        FortificationKind::RadioRepeater => vec![Base],

        // Wire family.
        FortificationKind::BarbedWire
        | FortificationKind::RazorWire
        | FortificationKind::ElectrifiedFence
        | FortificationKind::ConcertinaRoll => vec![Base, WireSprite],

        // Anti-tank family.
        FortificationKind::AntiTankDitch
        | FortificationKind::DragonsTeeth
        | FortificationKind::TankTrapX
        | FortificationKind::BollardConcrete => vec![Base, AntiTankSilhouette],

        // Camo netting (overlay layer is its only render-state).
        FortificationKind::CamoNetting => vec![Base, CamoOverlay],
    }
}

/// Per-tier sandbag fill height that the renderer uses to mask the
/// `SandbagFill` layer. The eroded rows (top-first per VAL-M9C-017)
/// are masked off. The function returns the number of intact rows.
#[must_use]
pub fn sandbag_fill_intact_rows(wall: &SandbagWall) -> u32 {
    wall.pixel_mask
        .rows
        .iter()
        .filter(|row| row.iter().any(|p| *p))
        .count() as u32
}

/// Per-tier sandbag fill row count. Convenience accessor for static
/// rendering (a freshly-built wall has every row intact).
#[must_use]
pub const fn sandbag_full_row_count(tier: SandbagTier) -> u32 {
    tier.height_px()
}

/// Does the fortification's render output include the given layer?
#[must_use]
pub fn kind_has_layer(kind: FortificationKind, layer: FortLayerId) -> bool {
    layers_for_kind(kind).contains(&layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_fortification::FortificationId;

    /// layer; sandbag walls additionally have the fill layer; wire
    /// kinds have the wire sprite layer.
    #[test]
    fn fortification_layers_per_kind() {
        for kind in FortificationKind::ALL {
            let layers = layers_for_kind(kind);
            assert!(!layers.is_empty(), "{kind:?} must render at least 1 layer");
            assert_eq!(
                layers[0],
                FortLayerId::Base,
                "{kind:?} must render the base layer first"
            );
        }

        for sandbag in [
            FortificationKind::SandbagLow,
            FortificationKind::SandbagMid,
            FortificationKind::SandbagHigh,
        ] {
            assert!(
                kind_has_layer(sandbag, FortLayerId::SandbagFill),
                "{sandbag:?} must render sandbag_fill"
            );
        }

        for wire in [
            FortificationKind::BarbedWire,
            FortificationKind::RazorWire,
            FortificationKind::ElectrifiedFence,
            FortificationKind::ConcertinaRoll,
        ] {
            assert!(
                kind_has_layer(wire, FortLayerId::WireSprite),
                "{wire:?} must render wire_sprite"
            );
        }

        for at in [
            FortificationKind::AntiTankDitch,
            FortificationKind::DragonsTeeth,
            FortificationKind::TankTrapX,
            FortificationKind::BollardConcrete,
        ] {
            assert!(
                kind_has_layer(at, FortLayerId::AntiTankSilhouette),
                "{at:?} must render anti_tank_silhouette"
            );
        }

        assert!(kind_has_layer(
            FortificationKind::CamoNetting,
            FortLayerId::CamoOverlay
        ));
        assert!(kind_has_layer(
            FortificationKind::Spotlight,
            FortLayerId::SpotlightHousing
        ));
        assert!(kind_has_layer(
            FortificationKind::MgNestStatic,
            FortLayerId::MgBarrel
        ));
    }

    /// Erosion (per VAL-M9C-017) shrinks the intact-row count from
    /// the top first; the renderer's `sandbag_fill_intact_rows`
    /// reflects that.
    #[test]
    fn sandbag_fill_intact_rows_shrinks_from_top() {
        let mut wall = SandbagWall::new_full(FortificationId(1), SandbagTier::High, 50);
        let baseline = sandbag_fill_intact_rows(&wall);
        assert_eq!(baseline, SandbagTier::High.height_px());

        // Wipe the top row entirely.
        let top_width = wall.pixel_mask.rows[0].len() as u32;
        wall.pixel_mask.erode_from_top(top_width);
        assert_eq!(
            sandbag_fill_intact_rows(&wall),
            baseline - 1,
            "after top row eroded, intact_rows == baseline - 1"
        );
    }

    #[test]
    fn fort_layer_id_as_str_round_trip() {
        for layer in [
            FortLayerId::Base,
            FortLayerId::SandbagFill,
            FortLayerId::MgBarrel,
            FortLayerId::TowerPlatform,
            FortLayerId::SpotlightHousing,
            FortLayerId::WireSprite,
            FortLayerId::AntiTankSilhouette,
            FortLayerId::CamoOverlay,
            FortLayerId::MineMarker,
            FortLayerId::DamageOverlay,
        ] {
            assert!(!layer.as_str().is_empty());
        }
    }

    #[test]
    fn sandbag_full_row_count_matches_height_px() {
        assert_eq!(sandbag_full_row_count(SandbagTier::Low), 4);
        assert_eq!(sandbag_full_row_count(SandbagTier::Mid), 8);
        assert_eq!(sandbag_full_row_count(SandbagTier::High), 12);
    }
}
