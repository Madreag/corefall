//! **M2**: 5-mode material overlay (off / integrity / pathability / mobility /
//! hazard / build_repair).
//!
//! **M9B** adds a 6th overlay mode `tactical` per spec
//! § "Per-segment cover state field — HUD readability":
//!
//! > per-segment chevron icon when material overlay `tactical` mode is on
//! > (M9B adds this 6th overlay mode).
//!
//! VAL-M9B-HUD-002: `OverlayMode::variants().len() == 6` after M9B.
//! VAL-M9B-HUD-TACTICAL-001: when tactical mode is active, every
//! trench segment renders a chevron labelled with its cover state.
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
use cf_trench::{cover_state, CoverState, SegmentVariant, TrenchSegment, TrenchStance};

/// Six canonical overlay modes (5 M2 modes + the M9B `tactical` mode).
/// Plus `off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    Off,
    Integrity,
    Pathability,
    Mobility,
    Hazard,
    BuildRepair,
    /// screen renders a chevron sprite labelled with its cover state.
    Tactical,
}

impl OverlayMode {
    /// Parse the canonical engine mode string. Distinct from `FromStr` so
    /// it can be `#[must_use]` and infallible (unknown names map to Off).
    #[must_use]
    pub fn parse_mode(s: &str) -> Self {
        match s {
            "integrity" => Self::Integrity,
            "pathability" => Self::Pathability,
            "mobility" => Self::Mobility,
            "hazard" => Self::Hazard,
            "build_repair" => Self::BuildRepair,
            "tactical" => Self::Tactical,
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
            Self::Tactical => "tactical",
        }
    }

    /// M9B the length is 6 (`Integrity..=Tactical`), excluding `Off`.
    #[must_use]
    pub const fn variants() -> [OverlayMode; 6] {
        [
            OverlayMode::Integrity,
            OverlayMode::Pathability,
            OverlayMode::Mobility,
            OverlayMode::Hazard,
            OverlayMode::BuildRepair,
            OverlayMode::Tactical,
        ]
    }
}

/// VAL-M9B-HUD-TACTICAL-001 sub-helper: one chevron sprite drawn by
/// the tactical overlay for a single trench segment.
#[derive(Debug, Clone, PartialEq)]
pub struct TacticalChevronSprite {
    /// World-space anchor for the chevron sprite (tile coordinates).
    pub world_pos: (i32, i32),
    /// The cover state the chevron displays. Drives glyph + tint.
    pub cover_state: CoverState,
    /// Variant of the source segment. Useful for cf-app to choose a
    /// per-variant tint accent.
    pub variant: SegmentVariant,
}

impl TacticalChevronSprite {
    /// String label cf-ui's HUD draws under the chevron (`Exposed |
    /// Partial | Full`).
    #[must_use]
    pub fn label(&self) -> &'static str {
        self.cover_state.as_str()
    }
}

/// trench segment, labelled with its derived cover state for the
/// `stance` parameter (defaults to Standing).
///
/// Used by cf-app's overlay renderer when [`OverlayMode::Tactical`] is
/// active.
#[must_use]
pub fn tactical_overlay_chevrons(
    segments: &[TrenchSegment],
    stance: TrenchStance,
) -> Vec<TacticalChevronSprite> {
    segments
        .iter()
        .map(|seg| {
            let chevron_pos = (
                seg.tile_x + (seg.width as i32) / 2,
                seg.tile_y + (seg.depth as i32) / 2,
            );
            TacticalChevronSprite {
                world_pos: chevron_pos,
                cover_state: cover_state(stance, seg.variant),
                variant: seg.variant,
            }
        })
        .collect()
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
            // from refusal_reason string (renamed to material_not_diggable)
            // to direct material-id comparison.
            if aff.id == cf_terrain::MATERIAL_AIR {
                [0, 0, 0, 0]
            } else if aff.id == cf_terrain::MATERIAL_METAL_NOHOOK {
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
            // the stable `material_not_diggable` refusal reason. The
            // anchor/metal_nohook visual cue now relies on the material id
            // directly rather than the refusal_reason vocabulary.
            if aff.id == cf_terrain::MATERIAL_AIR {
                [80, 200, 80, 0x70]
            } else if aff.hazard || aff.id == cf_terrain::MATERIAL_METAL_NOHOOK || aff.id == cf_terrain::MATERIAL_ANCHOR
            {
                [100, 100, 110, 0x70]
            } else {
                [100, 200, 100, 0x90]
            }
        }
        OverlayMode::Tactical => {
            // neutral desaturated mask — the per-segment chevron sprites
            // carry the cover-state semantics. Air stays transparent.
            if aff.id == cf_terrain::MATERIAL_AIR {
                [0, 0, 0, 0]
            } else {
                [120, 130, 140, 0x80]
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
        MATERIAL_AIR, MATERIAL_ANCHOR, MATERIAL_CONCRETE, MATERIAL_DIRT, MATERIAL_HAZARD, MATERIAL_METAL_NOHOOK,
        MATERIAL_REPAIR_FILL,
    };
    use cf_trench::{SegmentVariant, TrenchModule, TrenchSegment, TrenchStance};

    #[test]
    fn overlay_mode_round_trips_via_string() {
        for mode in [
            OverlayMode::Off,
            OverlayMode::Integrity,
            OverlayMode::Pathability,
            OverlayMode::Mobility,
            OverlayMode::Hazard,
            OverlayMode::BuildRepair,
            OverlayMode::Tactical,
        ] {
            assert_eq!(OverlayMode::parse_mode(mode.as_str()), mode);
        }
        // Unknown defaults to Off.
        assert_eq!(OverlayMode::parse_mode("garbage"), OverlayMode::Off);
    }

    #[test]
    fn material_overlay_tactical_mode_is_registered() {
        let variants = OverlayMode::variants();
        assert_eq!(variants.len(), 6, "M9B adds Tactical as the 6th overlay");
        assert!(variants.contains(&OverlayMode::Tactical));
    }

    /// Alias matching the spec evidence string
    /// `tactical_mode_registered`.
    #[test]
    fn material_overlay_tactical_mode_registered() {
        material_overlay_tactical_mode_is_registered();
    }

    /// VAL-M9B-HUD-TACTICAL-001 (a): tactical mode produces one
    /// chevron per visible trench segment.
    #[test]
    fn material_overlay_tactical_renders_chevron_per_segment() {
        let segments = vec![
            TrenchSegment {
                variant: SegmentVariant::Standard,
                tile_x: 10,
                tile_y: 0,
                depth: 16,
                width: 16,
                raised_step_height: None,
                embedded_modules: vec![TrenchModule::Duckboard],
            },
            TrenchSegment {
                variant: SegmentVariant::Deep,
                tile_x: 30,
                tile_y: 0,
                depth: 24,
                width: 16,
                raised_step_height: None,
                embedded_modules: vec![
                    TrenchModule::Duckboard,
                    TrenchModule::DrainageSump,
                ],
            },
            TrenchSegment {
                variant: SegmentVariant::FireStep,
                tile_x: 50,
                tile_y: 0,
                depth: 16,
                width: 20,
                raised_step_height: Some(8),
                embedded_modules: vec![TrenchModule::Duckboard, TrenchModule::FireStep],
            },
        ];
        let chevrons = tactical_overlay_chevrons(&segments, TrenchStance::Standing);
        assert_eq!(
            chevrons.len(),
            segments.len(),
            "tactical mode draws exactly one chevron per segment"
        );
        let labels: Vec<&str> = chevrons.iter().map(|c| c.label()).collect();
        assert!(labels.contains(&"Partial"), "standard expects Partial");
        assert!(labels.contains(&"Full"), "deep expects Full");
        assert!(
            labels.contains(&"Exposed"),
            "fire_step on-step expects Exposed"
        );
    }

    /// VAL-M9B-HUD-TACTICAL-001 alias matching the spec evidence
    /// string `tactical_mode_renders_chevron_per_segment`.
    #[test]
    fn material_overlay_tactical_mode_renders_chevron_per_segment() {
        material_overlay_tactical_renders_chevron_per_segment();
    }

    #[test]
    fn tactical_overlay_chevron_labels_round_trip_via_cover_state() {
        let segments = vec![TrenchSegment {
            variant: SegmentVariant::Standard,
            tile_x: 0,
            tile_y: 0,
            depth: 16,
            width: 16,
            raised_step_height: None,
            embedded_modules: vec![],
        }];
        let chevrons = tactical_overlay_chevrons(&segments, TrenchStance::Crouched);
        assert_eq!(chevrons[0].label(), "Full", "crouched in standard = Full");
    }

    #[test]
    fn tactical_overlay_chevrons_empty_when_no_segments() {
        let chevrons = tactical_overlay_chevrons(&[], TrenchStance::Standing);
        assert!(chevrons.is_empty());
    }

    #[test]
    fn tactical_overlay_air_transparent_dirt_neutral() {
        assert_eq!(material_tint(OverlayMode::Tactical, MATERIAL_AIR), [0, 0, 0, 0]);
        let dirt = material_tint(OverlayMode::Tactical, MATERIAL_DIRT);
        assert!(dirt[3] > 0, "tactical mask must be visible on dirt");
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
