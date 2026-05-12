//! **M2**: 5-mode material overlay (off / integrity / pathability / mobility /
//! hazard / build_repair).
//!
//! Mirrors the engine's `material_overlay_mode` (a string carried on
//! `ObserveFrame::terrain.current_overlay_mode`). cf-app's bridge writes
//! the active mode each frame into the [`OverlayModeState`] resource;
//! this module exposes:
//!
//! - The [`OverlayMode`] enum.
//! - A [`material_tint`] helper that resolves the per-material display
//!   color under each overlay mode. The chunked-terrain renderer
//!   (terrain.rs) consults this to recolor textures when a non-`off` mode
//!   is active.
//! - A small Bevy plugin ([`OverlayModePlugin`]) that registers the
//!   resource so consumers can read it safely.
//!
//! The overlay never mutates the canonical chunk texture data; the
//! recoloring path is a per-frame palette swap so flipping back to
//! `off` restores the registry colors instantly.

use bevy::prelude::*;

use cf_terrain::{material_affordance, MaterialId};

/// Five canonical overlay modes plus `off`. Mirrors the engine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    Off,
    Integrity,
    Pathability,
    Mobility,
    Hazard,
    BuildRepair,
}

impl OverlayMode {
    /// Parse a string mode (matches the canonical names the engine emits).
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "integrity" => Self::Integrity,
            "pathability" => Self::Pathability,
            "mobility" => Self::Mobility,
            "hazard" => Self::Hazard,
            "build_repair" => Self::BuildRepair,
            _ => Self::Off,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Integrity => "integrity",
            Self::Pathability => "pathability",
            Self::Mobility => "mobility",
            Self::Hazard => "hazard",
            Self::BuildRepair => "build_repair",
        }
    }
}

/// Bevy resource carrying the current overlay mode. Written by cf-app's
/// bridge from `ObserveFrame::terrain.current_overlay_mode`; read by the
/// chunked-terrain renderer and the cf-ui legend.
#[derive(Resource, Debug, Clone, Default)]
pub struct OverlayModeState {
    pub mode: OverlayMode,
}

/// Plugin that registers [`OverlayModeState`] so consumers can read it
/// without manually inserting the resource.
pub struct OverlayModePlugin;

impl Plugin for OverlayModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlayModeState>();
    }
}

/// Resolve the per-material RGBA tint for the requested overlay mode.
///
/// - `Off` → the registry overlay color (no tint).
/// - `Integrity` → hardness-mapped gradient: WHITE for soft materials,
///   dark gray for hard materials. Air is transparent.
/// - `Pathability` → green for actor_passable, red for blocked, semi-
///   transparent for hazard.
/// - `Mobility` → green for anchorable, red for metal_nohook, gray neutral.
/// - `Hazard` → red tint on damage_on_touch tiles; neutral overlay alpha
///   elsewhere.
/// - `BuildRepair` → green where `repair_fill` may be placed (air +
///   non-hazard non-anchor surfaces), gray elsewhere.
#[must_use]
pub fn material_tint(mode: OverlayMode, id: MaterialId) -> [u8; 4] {
    let Some(aff) = material_affordance(id) else {
        return [0, 0, 0, 0];
    };
    match mode {
        OverlayMode::Off => aff.overlay_rgba,
        OverlayMode::Integrity => integrity_tint(aff.hardness, aff.overlay_rgba[3]),
        OverlayMode::Pathability => {
            if aff.id == cf_terrain::MATERIAL_AIR {
                [0, 0, 0, 0]
            } else if aff.actor_passable {
                [90, 200, 90, 0xC0]
            } else if aff.hazard {
                [220, 120, 60, 0xD0]
            } else {
                [220, 80, 80, 0xC0]
            }
        }
        OverlayMode::Mobility => {
            if aff.id == cf_terrain::MATERIAL_AIR {
                [0, 0, 0, 0]
            } else if matches!(aff.refusal_reason, Some("material_metal_nohook")) {
                [220, 80, 80, 0xD0]
            } else if aff.anchorable {
                [90, 140, 220, 0xC8]
            } else {
                [140, 140, 150, 0xB0]
            }
        }
        OverlayMode::Hazard => {
            if aff.hazard {
                [230, 50, 50, 0xE0]
            } else if aff.id == cf_terrain::MATERIAL_AIR {
                [0, 0, 0, 0]
            } else {
                [80, 80, 90, 0x60]
            }
        }
        OverlayMode::BuildRepair => {
            if aff.id == cf_terrain::MATERIAL_AIR {
                [80, 200, 80, 0x70]
            } else if aff.hazard
                || matches!(
                    aff.refusal_reason,
                    Some("material_metal_nohook") | Some("material_anchor")
                )
            {
                [100, 100, 110, 0x70]
            } else {
                [100, 200, 100, 0x90]
            }
        }
    }
}

fn integrity_tint(hardness: f32, base_alpha: u8) -> [u8; 4] {
    if hardness <= 0.5 {
        return [0, 0, 0, 0];
    }
    let max_hardness = 100.0_f32;
    let norm = (hardness / max_hardness).clamp(0.0, 1.0);
    let brightness = ((1.0 - norm) * 230.0 + 25.0) as u8;
    [brightness, brightness, brightness, base_alpha]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_terrain::{
        MATERIAL_AIR, MATERIAL_ANCHOR, MATERIAL_CONCRETE, MATERIAL_DIRT, MATERIAL_HAZARD,
        MATERIAL_METAL_NOHOOK, MATERIAL_REPAIR_FILL,
    };

    #[test]
    fn overlay_mode_round_trips_via_string() {
        for mode in [
            OverlayMode::Off,
            OverlayMode::Integrity,
            OverlayMode::Pathability,
            OverlayMode::Mobility,
            OverlayMode::Hazard,
            OverlayMode::BuildRepair,
        ] {
            assert_eq!(OverlayMode::from_str(mode.as_str()), mode);
        }
        // Unknown defaults to Off.
        assert_eq!(OverlayMode::from_str("garbage"), OverlayMode::Off);
    }

    #[test]
    fn overlay_off_returns_registry_color_for_dirt() {
        let dirt = material_tint(OverlayMode::Off, MATERIAL_DIRT);
        let registry = material_affordance(MATERIAL_DIRT).unwrap().overlay_rgba;
        assert_eq!(dirt, registry);
    }

    #[test]
    fn integrity_overlay_brightness_inversely_proportional_to_hardness() {
        let soft = material_tint(OverlayMode::Integrity, MATERIAL_DIRT)[0];
        let hard = material_tint(OverlayMode::Integrity, MATERIAL_METAL_NOHOOK)[0];
        assert!(soft > hard, "softer material must render brighter");
        // Air should still be transparent.
        assert_eq!(material_tint(OverlayMode::Integrity, MATERIAL_AIR)[3], 0);
    }

    #[test]
    fn pathability_routes_metal_to_red_and_air_to_transparent() {
        let metal = material_tint(OverlayMode::Pathability, MATERIAL_METAL_NOHOOK);
        assert!(metal[0] > 180 && metal[1] < 120 && metal[2] < 120);
        assert_eq!(material_tint(OverlayMode::Pathability, MATERIAL_AIR), [0, 0, 0, 0]);
    }

    #[test]
    fn mobility_marks_anchor_blue_and_metal_red() {
        let anchor = material_tint(OverlayMode::Mobility, MATERIAL_ANCHOR);
        assert!(anchor[2] > 180, "anchor must show blue dominant");
        let metal = material_tint(OverlayMode::Mobility, MATERIAL_METAL_NOHOOK);
        assert!(metal[0] > 180 && metal[1] < 120, "metal must show red");
    }

    #[test]
    fn hazard_overlay_highlights_only_hazard() {
        let hazard = material_tint(OverlayMode::Hazard, MATERIAL_HAZARD);
        assert!(hazard[0] > 200 && hazard[3] >= 0xC0);
        let neutral = material_tint(OverlayMode::Hazard, MATERIAL_CONCRETE);
        assert!(neutral[3] < 0xC0, "non-hazard surfaces must read muted");
    }

    #[test]
    fn build_repair_overlay_marks_repair_fill_and_air_green() {
        let air = material_tint(OverlayMode::BuildRepair, MATERIAL_AIR);
        assert!(air[1] >= 180, "air shows where repair_fill may go");
        let repair = material_tint(OverlayMode::BuildRepair, MATERIAL_REPAIR_FILL);
        assert!(repair[1] >= 180, "repair_fill surface marked green");
        let metal = material_tint(OverlayMode::BuildRepair, MATERIAL_METAL_NOHOOK);
        assert!(metal[1] < 180, "metal_nohook can't accept repair_fill");
    }
}
