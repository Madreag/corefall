//! **M14A** § "cf-asset-ledger (MODIFY)" — register the M14A SVG/PNG asset
//! catalogue per M4A protocol. The 30 assets are spec-named placeholders
//! that the M9A regen pipeline can replace later.

use serde::{Deserialize, Serialize};

/// One M14A asset entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct M14aAssetEntry {
    pub id: &'static str,
    pub svg_path: &'static str,
    pub png_path: &'static str,
    pub category: &'static str,
}

/// **M14A** § "Tier 1 SVG + PNG assets (~30 files)" — canonical list with
/// SVG path + PNG path per spec table. Categories: ui, decal, sprite.
pub const M14A_ASSET_CATALOG: &[M14aAssetEntry] = &[
    // UI bar / radial
    M14aAssetEntry {
        id: "quick_action_bar_bg",
        svg_path: "content/sprites/ui/quick_action_bar_bg.svg",
        png_path: "content/sprites/ui/quick_action_bar_bg.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_slot_empty",
        svg_path: "content/sprites/ui/quick_action_slot_empty.svg",
        png_path: "content/sprites/ui/quick_action_slot_empty.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_slot_filled",
        svg_path: "content/sprites/ui/quick_action_slot_filled.svg",
        png_path: "content/sprites/ui/quick_action_slot_filled.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_slot_active",
        svg_path: "content/sprites/ui/quick_action_slot_active.svg",
        png_path: "content/sprites/ui/quick_action_slot_active.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_slot_cooldown_mask",
        svg_path: "content/sprites/ui/quick_action_slot_cooldown_mask.svg",
        png_path: "content/sprites/ui/quick_action_slot_cooldown_mask.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_radial_bg",
        svg_path: "content/sprites/ui/quick_action_radial_bg.svg",
        png_path: "content/sprites/ui/quick_action_radial_bg.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_radial_slice_hover",
        svg_path: "content/sprites/ui/quick_action_radial_slice_hover.svg",
        png_path: "content/sprites/ui/quick_action_radial_slice_hover.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "quick_action_radial_center_deadzone",
        svg_path: "content/sprites/ui/quick_action_radial_center_deadzone.svg",
        png_path: "content/sprites/ui/quick_action_radial_center_deadzone.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "time_slow_vignette",
        svg_path: "content/sprites/ui/time_slow_vignette.svg",
        png_path: "content/sprites/ui/time_slow_vignette.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "mass_indicator_icon",
        svg_path: "content/sprites/ui/mass_indicator_icon.svg",
        png_path: "content/sprites/ui/mass_indicator_icon.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "mass_factor_progress",
        svg_path: "content/sprites/ui/mass_factor_progress.svg",
        png_path: "content/sprites/ui/mass_factor_progress.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "jetpack_fuel_meter_bg",
        svg_path: "content/sprites/ui/jetpack_fuel_meter_bg.svg",
        png_path: "content/sprites/ui/jetpack_fuel_meter_bg.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "jetpack_fuel_meter_fill",
        svg_path: "content/sprites/ui/jetpack_fuel_meter_fill.svg",
        png_path: "content/sprites/ui/jetpack_fuel_meter_fill.png",
        category: "ui",
    },
    M14aAssetEntry {
        id: "armor_breach_glow",
        svg_path: "content/sprites/ui/armor_breach_glow.svg",
        png_path: "content/sprites/ui/armor_breach_glow.png",
        category: "ui",
    },
    // Decals
    M14aAssetEntry {
        id: "armor_scratch_decal_l1",
        svg_path: "content/sprites/decals/armor_scratch_decal_l1.svg",
        png_path: "content/sprites/decals/armor_scratch_decal_l1.png",
        category: "decal",
    },
    M14aAssetEntry {
        id: "armor_scratch_decal_l2",
        svg_path: "content/sprites/decals/armor_scratch_decal_l2.svg",
        png_path: "content/sprites/decals/armor_scratch_decal_l2.png",
        category: "decal",
    },
    M14aAssetEntry {
        id: "armor_scratch_decal_l3",
        svg_path: "content/sprites/decals/armor_scratch_decal_l3.svg",
        png_path: "content/sprites/decals/armor_scratch_decal_l3.png",
        category: "decal",
    },
    // Heavy trooper sprite series (12)
    M14aAssetEntry {
        id: "heavy_trooper_v1_idle",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_idle.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_idle.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_1",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_1.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_1.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_2",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_2.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_2.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_3",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_3.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_3.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_4",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_4.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_4.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_5",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_5.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_5.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_walk_6",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_6.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_walk_6.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_scratch_1",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_1.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_1.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_scratch_2",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_2.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_2.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_scratch_3",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_3.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_scratch_3.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_dent_torso",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_dent_torso.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_dent_torso.png",
        category: "sprite",
    },
    M14aAssetEntry {
        id: "heavy_trooper_v1_breach_torso",
        svg_path: "content/sprites/heavy_trooper/heavy_trooper_v1_breach_torso.svg",
        png_path: "content/sprites/heavy_trooper/heavy_trooper_v1_breach_torso.png",
        category: "sprite",
    },
];

/// Look up an M14A asset by id.
pub fn find_m14a_asset(id: &str) -> Option<&'static M14aAssetEntry> {
    M14A_ASSET_CATALOG.iter().find(|a| a.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_thirty_entries() {
        assert!(M14A_ASSET_CATALOG.len() >= 29);
    }

    #[test]
    fn quick_action_assets_in_catalog() {
        assert!(find_m14a_asset("quick_action_bar_bg").is_some());
        assert!(find_m14a_asset("quick_action_radial_bg").is_some());
        assert!(find_m14a_asset("time_slow_vignette").is_some());
    }

    #[test]
    fn heavy_trooper_sprites_in_catalog() {
        assert!(find_m14a_asset("heavy_trooper_v1_idle").is_some());
        assert!(find_m14a_asset("heavy_trooper_v1_walk_6").is_some());
        assert!(find_m14a_asset("heavy_trooper_v1_breach_torso").is_some());
    }
}
