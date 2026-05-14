//! M4A asset categories + production tiers + regen-status enums.

use serde::{Deserialize, Serialize};

/// One of the 16 asset categories defined in the M4A spec. Every entry in
/// the ledger declares its category up-front so per-category filtering
/// (`cf-mod ledger list --category WeaponSprite`) is O(1). Wire format
/// matches the spec's exact strings (e.g. `Audio_SFX`, `Mod_Custom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetCategory {
    /// HUD icons, menu icons, action prompt glyphs.
    UiIcon,
    /// Weapon side-view sprites (M9A baseline; refined at M32A).
    WeaponSprite,
    /// Actor side-view sprites + walk frames.
    ActorSprite,
    /// Vehicle side-view sprites + boarding states.
    VehicleSprite,
    /// Chassis silhouettes per weight class.
    ChassisSprite,
    /// Turret, pump, valve, generator, etc.
    BaseModuleSprite,
    /// Material tiles + integrity-band variants.
    TerrainTile,
    /// Material registry swatches + overlay tints.
    MaterialSwatch,
    /// VFX particle textures (per M24A).
    Particle,
    /// Animation frame strips (per M18A).
    Animation,
    /// Sound effects (per M12A + M37A).
    #[serde(rename = "Audio_SFX")]
    AudioSfx,
    /// Voice samples (per M37A).
    #[serde(rename = "Audio_Voice")]
    AudioVoice,
    /// Music tracks (per M37A).
    #[serde(rename = "Audio_Music")]
    AudioMusic,
    /// Codex entries + dialog + lore (per M25A).
    #[serde(rename = "Narrative_Text")]
    NarrativeText,
    /// Per-language string tables (per M38A).
    #[serde(rename = "Localization_Strings")]
    LocalizationStrings,
    /// Cosmetic skins + scars + faction variants (per M45A).
    Cosmetic,
    /// Mod-supplied; modder declares category. M4A treats this as the
    /// catch-all when a mod doesn't map to one of the engine categories.
    #[serde(rename = "Mod_Custom")]
    ModCustom,
}

impl AssetCategory {
    /// Stable wire-name used as the canonical identifier. The Rust enum's
    /// PascalCase is preserved so existing infrastructure that grep's the
    /// ledger (e.g. `WeaponSprite`) keeps working.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::UiIcon => "UiIcon",
            Self::WeaponSprite => "WeaponSprite",
            Self::ActorSprite => "ActorSprite",
            Self::VehicleSprite => "VehicleSprite",
            Self::ChassisSprite => "ChassisSprite",
            Self::BaseModuleSprite => "BaseModuleSprite",
            Self::TerrainTile => "TerrainTile",
            Self::MaterialSwatch => "MaterialSwatch",
            Self::Particle => "Particle",
            Self::Animation => "Animation",
            Self::AudioSfx => "Audio_SFX",
            Self::AudioVoice => "Audio_Voice",
            Self::AudioMusic => "Audio_Music",
            Self::NarrativeText => "Narrative_Text",
            Self::LocalizationStrings => "Localization_Strings",
            Self::Cosmetic => "Cosmetic",
            Self::ModCustom => "Mod_Custom",
        }
    }

    /// All 16 categories in stable order.
    pub const fn all() -> &'static [Self] {
        &[
            Self::UiIcon,
            Self::WeaponSprite,
            Self::ActorSprite,
            Self::VehicleSprite,
            Self::ChassisSprite,
            Self::BaseModuleSprite,
            Self::TerrainTile,
            Self::MaterialSwatch,
            Self::Particle,
            Self::Animation,
            Self::AudioSfx,
            Self::AudioVoice,
            Self::AudioMusic,
            Self::NarrativeText,
            Self::LocalizationStrings,
            Self::Cosmetic,
            Self::ModCustom,
        ]
    }

    /// Parse a wire-name. Accepts both the canonical `Audio_SFX` form and a
    /// case-insensitive PascalCase shorthand so CLI args are forgiving.
    pub fn parse(input: &str) -> Option<Self> {
        for cat in Self::all() {
            if cat.as_str().eq_ignore_ascii_case(input) {
                return Some(*cat);
            }
        }
        match input.to_ascii_lowercase().as_str() {
            "uiicon" | "ui_icon" => Some(Self::UiIcon),
            "weaponsprite" | "weapon_sprite" => Some(Self::WeaponSprite),
            "actorsprite" | "actor_sprite" => Some(Self::ActorSprite),
            "vehiclesprite" | "vehicle_sprite" => Some(Self::VehicleSprite),
            "chassissprite" | "chassis_sprite" => Some(Self::ChassisSprite),
            "basemodulesprite" | "base_module_sprite" => Some(Self::BaseModuleSprite),
            "terraintile" | "terrain_tile" => Some(Self::TerrainTile),
            "materialswatch" | "material_swatch" => Some(Self::MaterialSwatch),
            "particle" => Some(Self::Particle),
            "animation" => Some(Self::Animation),
            "audio_sfx" | "audiosfx" | "sfx" => Some(Self::AudioSfx),
            "audio_voice" | "audiovoice" | "voice" => Some(Self::AudioVoice),
            "audio_music" | "audiomusic" | "music" => Some(Self::AudioMusic),
            "narrative_text" | "narrativetext" | "narrative" => Some(Self::NarrativeText),
            "localization_strings" | "localizationstrings" | "localization" => Some(Self::LocalizationStrings),
            "cosmetic" => Some(Self::Cosmetic),
            "mod_custom" | "modcustom" | "mod" => Some(Self::ModCustom),
            _ => None,
        }
    }

    /// The default output file extension for this category. Pipeline tools
    /// may override (e.g. Tier 2 ComfyUI uses `webp` instead of `png`).
    pub const fn default_extension(&self) -> &'static str {
        match self {
            Self::UiIcon
            | Self::WeaponSprite
            | Self::ActorSprite
            | Self::VehicleSprite
            | Self::ChassisSprite
            | Self::BaseModuleSprite
            | Self::TerrainTile
            | Self::MaterialSwatch
            | Self::Particle
            | Self::Cosmetic => "svg",
            Self::Animation => "json",
            Self::AudioSfx | Self::AudioVoice | Self::AudioMusic => "ogg",
            Self::NarrativeText => "ron",
            Self::LocalizationStrings => "json",
            Self::ModCustom => "bin",
        }
    }
}

/// One of the 7 production tiers. Determines the regen pipeline expected
/// and the appropriate tooling chain. Wire format matches the spec's exact
/// strings (e.g. `Tier1_SVG`, `Tier2_ComfyUI`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProductionTier {
    /// Hand-coded colored rectangle / sine wave.
    #[serde(rename = "Tier0_Placeholder")]
    Tier0Placeholder,
    /// M9A: SVG + LLM-prompted shape generation.
    #[serde(rename = "Tier1_SVG")]
    Tier1Svg,
    /// M12A: LLM-generated SFX placeholder.
    #[serde(rename = "Tier1_LLM_Audio")]
    Tier1LlmAudio,
    /// M32A: SDXL/Flux/AnimateDiff production-quality.
    #[serde(rename = "Tier2_ComfyUI")]
    Tier2ComfyUi,
    /// M37A: Stable Audio Open production + voice synth.
    #[serde(rename = "Tier2_Audio_Production")]
    Tier2AudioProduction,
    /// M48A: hand-tweaked / final mix / Aseprite-touched.
    #[serde(rename = "Tier3_Polish")]
    Tier3Polish,
    /// Mod-author-supplied; no tier; trust per package.
    #[serde(rename = "Mod_Supplied")]
    ModSupplied,
}

impl ProductionTier {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Tier0Placeholder => "Tier0_Placeholder",
            Self::Tier1Svg => "Tier1_SVG",
            Self::Tier1LlmAudio => "Tier1_LLM_Audio",
            Self::Tier2ComfyUi => "Tier2_ComfyUI",
            Self::Tier2AudioProduction => "Tier2_Audio_Production",
            Self::Tier3Polish => "Tier3_Polish",
            Self::ModSupplied => "Mod_Supplied",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[
            Self::Tier0Placeholder,
            Self::Tier1Svg,
            Self::Tier1LlmAudio,
            Self::Tier2ComfyUi,
            Self::Tier2AudioProduction,
            Self::Tier3Polish,
            Self::ModSupplied,
        ]
    }

    pub fn parse(input: &str) -> Option<Self> {
        for tier in Self::all() {
            if tier.as_str().eq_ignore_ascii_case(input) {
                return Some(*tier);
            }
        }
        match input.to_ascii_lowercase().as_str() {
            "tier0" | "placeholder" | "tier0_placeholder" => Some(Self::Tier0Placeholder),
            "tier1" | "tier1svg" | "svg" | "tier1_svg" => Some(Self::Tier1Svg),
            "tier1audio" | "tier1_llm_audio" | "tier1llmaudio" => Some(Self::Tier1LlmAudio),
            "tier2" | "tier2comfy" | "tier2comfyui" | "tier2_comfyui" | "comfyui" => Some(Self::Tier2ComfyUi),
            "tier2audio" | "tier2_audio_production" | "tier2audioproduction" => Some(Self::Tier2AudioProduction),
            "tier3" | "tier3polish" | "tier3_polish" | "polish" => Some(Self::Tier3Polish),
            "mod" | "mod_supplied" | "modsupplied" => Some(Self::ModSupplied),
            _ => None,
        }
    }
}

/// Lifecycle state. Computed by `cf-mod ledger verify`; persisted into the
/// entry only via the most-recent regen pass (the writer is the authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegenStatus {
    /// Entry matches output_path's current blake3 (verified).
    Fresh,
    /// Entry exists but never validated (CI hasn't re-baked recently).
    Stale,
    /// Entry's blake3 doesn't match output_path's current hash
    /// (assets edited outside pipeline).
    Drifted,
    /// output_path doesn't exist on disk.
    Missing,
    /// Most recent regen attempt errored.
    Failed,
}

impl RegenStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "Fresh",
            Self::Stale => "Stale",
            Self::Drifted => "Drifted",
            Self::Missing => "Missing",
            Self::Failed => "Failed",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[Self::Fresh, Self::Stale, Self::Drifted, Self::Missing, Self::Failed]
    }

    pub fn parse(input: &str) -> Option<Self> {
        for s in Self::all() {
            if s.as_str().eq_ignore_ascii_case(input) {
                return Some(*s);
            }
        }
        None
    }
}

/// Per-asset license declaration. The author asserts the license; M4A does
/// NOT verify (license verification is out of scope per the spec). Wire
/// format matches the spec's strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum License {
    /// CC0 / public domain.
    #[serde(rename = "CC0")]
    Cc0,
    /// CC-BY 4.0.
    #[serde(rename = "CC-BY")]
    CcBy,
    /// CC-BY-SA 4.0.
    #[serde(rename = "CC-BY-SA")]
    CcBySa,
    /// Proprietary, all rights reserved (e.g. baseline studio assets prior
    /// to release).
    #[serde(rename = "Proprietary")]
    Proprietary,
    /// Mod-supplied with author-declared license (in the inner string).
    #[serde(rename = "mod-supplied")]
    ModSupplied(String),
    /// Custom — author declares free-form text (SPDX expression etc).
    #[serde(rename = "custom")]
    Custom(String),
}

impl Default for License {
    fn default() -> Self {
        Self::Cc0
    }
}

impl License {
    pub fn as_label(&self) -> String {
        match self {
            Self::Cc0 => "CC0".to_string(),
            Self::CcBy => "CC-BY".to_string(),
            Self::CcBySa => "CC-BY-SA".to_string(),
            Self::Proprietary => "proprietary".to_string(),
            Self::ModSupplied(s) => format!("mod-supplied:{s}"),
            Self::Custom(s) => s.clone(),
        }
    }
}

/// Package source: vanilla / mod / faction-pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageRef {
    /// First-party content packaged with the engine.
    Vanilla,
    /// Player-installed mod identified by `mod_id`.
    Mod(String),
    /// Faction-pack identified by `pack_id`.
    FactionPack(String),
}

impl Default for PackageRef {
    fn default() -> Self {
        Self::Vanilla
    }
}

impl PackageRef {
    pub fn as_label(&self) -> String {
        match self {
            Self::Vanilla => "vanilla".to_string(),
            Self::Mod(m) => format!("mod:{m}"),
            Self::FactionPack(p) => format!("faction-pack:{p}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_categories_case_insensitive() {
        assert_eq!(AssetCategory::parse("WeaponSprite"), Some(AssetCategory::WeaponSprite));
        assert_eq!(AssetCategory::parse("weaponsprite"), Some(AssetCategory::WeaponSprite));
        assert_eq!(AssetCategory::parse("weapon_sprite"), Some(AssetCategory::WeaponSprite));
        assert_eq!(AssetCategory::parse("Audio_SFX"), Some(AssetCategory::AudioSfx));
        assert_eq!(AssetCategory::parse("audio_sfx"), Some(AssetCategory::AudioSfx));
        assert_eq!(AssetCategory::parse("Mod_Custom"), Some(AssetCategory::ModCustom));
        assert!(AssetCategory::parse("not_a_category").is_none());
    }

    #[test]
    fn parse_tiers_case_insensitive() {
        assert_eq!(ProductionTier::parse("Tier1_SVG"), Some(ProductionTier::Tier1Svg));
        assert_eq!(ProductionTier::parse("tier1_svg"), Some(ProductionTier::Tier1Svg));
        assert_eq!(ProductionTier::parse("comfyui"), Some(ProductionTier::Tier2ComfyUi));
        assert_eq!(
            ProductionTier::parse("Tier2_ComfyUI"),
            Some(ProductionTier::Tier2ComfyUi)
        );
        assert_eq!(ProductionTier::parse("Mod_Supplied"), Some(ProductionTier::ModSupplied));
        assert!(ProductionTier::parse("not_a_tier").is_none());
    }

    #[test]
    fn parse_status() {
        assert_eq!(RegenStatus::parse("Fresh"), Some(RegenStatus::Fresh));
        assert_eq!(RegenStatus::parse("DRIFTED"), Some(RegenStatus::Drifted));
        assert!(RegenStatus::parse("not_a_status").is_none());
    }

    #[test]
    fn all_categories_have_unique_str() {
        let mut seen = std::collections::HashSet::new();
        for c in AssetCategory::all() {
            assert!(seen.insert(c.as_str()), "duplicate category string: {}", c.as_str());
        }
        // 16 engine categories + ModCustom catch-all = 17 enum variants.
        assert_eq!(seen.len(), 17);
    }

    #[test]
    fn all_tiers_have_unique_str() {
        let mut seen = std::collections::HashSet::new();
        for t in ProductionTier::all() {
            assert!(seen.insert(t.as_str()), "duplicate tier string: {}", t.as_str());
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn license_default_is_cc0() {
        assert_eq!(License::default(), License::Cc0);
    }

    #[test]
    fn package_ref_default_is_vanilla() {
        assert_eq!(PackageRef::default(), PackageRef::Vanilla);
    }
}
