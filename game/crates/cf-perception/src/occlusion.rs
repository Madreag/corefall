//! M6: occlusion model — walls partially block sound.
//!
//! Each material along the ray contributes an attenuation factor; the
//! aggregate is multiplied into the perceived loudness. We bake the
//! per-material attenuation table here so cf-control + cf-ai don't depend
//! on cf-material (perception stays a thin pure crate).

use serde::{Deserialize, Serialize};

/// Per-material attenuation factor. `1.0` = sound passes unchanged;
/// `0.0` = sound fully blocked.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcclusionMaterial {
    Air = 0,
    LooseFill = 1,
    Concrete = 2,
    Metal = 3,
    Glass = 4,
    Wood = 5,
    Bone = 6,
}

impl OcclusionMaterial {
    /// Attenuation factor when the ray crosses one tile of this material.
    pub fn attenuation_per_tile(self) -> f32 {
        match self {
            OcclusionMaterial::Air => 1.0,
            OcclusionMaterial::Glass => 0.85,
            OcclusionMaterial::Wood => 0.7,
            OcclusionMaterial::Bone => 0.65,
            OcclusionMaterial::LooseFill => 0.55,
            OcclusionMaterial::Concrete => 0.4,
            OcclusionMaterial::Metal => 0.25,
        }
    }
}

/// Result of an occlusion query — aggregate attenuation factor plus per-material
/// trace counts so debug overlays can explain "why is this quiet?".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcclusionResult {
    /// Combined attenuation factor (0..1).
    pub factor: f32,
    /// Number of tiles crossed of each occluder material.
    pub tiles_crossed_concrete: u32,
    pub tiles_crossed_metal: u32,
    pub tiles_crossed_glass: u32,
    pub tiles_crossed_wood: u32,
    pub tiles_crossed_loose_fill: u32,
    pub tiles_crossed_bone: u32,
}

impl OcclusionResult {
    pub fn passthrough() -> Self {
        Self {
            factor: 1.0,
            tiles_crossed_concrete: 0,
            tiles_crossed_metal: 0,
            tiles_crossed_glass: 0,
            tiles_crossed_wood: 0,
            tiles_crossed_loose_fill: 0,
            tiles_crossed_bone: 0,
        }
    }
}

/// Apply a single tile of `material` to an existing occlusion result.
#[must_use]
pub fn apply_occlusion(prior: OcclusionResult, material: OcclusionMaterial) -> OcclusionResult {
    let mut out = prior;
    out.factor = (out.factor * material.attenuation_per_tile()).clamp(0.0, 1.0);
    match material {
        OcclusionMaterial::Concrete => out.tiles_crossed_concrete += 1,
        OcclusionMaterial::Metal => out.tiles_crossed_metal += 1,
        OcclusionMaterial::Glass => out.tiles_crossed_glass += 1,
        OcclusionMaterial::Wood => out.tiles_crossed_wood += 1,
        OcclusionMaterial::LooseFill => out.tiles_crossed_loose_fill += 1,
        OcclusionMaterial::Bone => out.tiles_crossed_bone += 1,
        OcclusionMaterial::Air => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metal_blocks_more_than_wood() {
        let m = apply_occlusion(OcclusionResult::passthrough(), OcclusionMaterial::Metal);
        let w = apply_occlusion(OcclusionResult::passthrough(), OcclusionMaterial::Wood);
        assert!(m.factor < w.factor);
    }

    #[test]
    fn air_passes_through() {
        let r = apply_occlusion(OcclusionResult::passthrough(), OcclusionMaterial::Air);
        assert_eq!(r.factor, 1.0);
    }

    #[test]
    fn multiple_tiles_compound() {
        let r = apply_occlusion(OcclusionResult::passthrough(), OcclusionMaterial::Concrete);
        let factor_before = r.factor;
        let r2 = apply_occlusion(r, OcclusionMaterial::Concrete);
        assert!(r2.factor < factor_before);
        assert_eq!(r2.tiles_crossed_concrete, 2);
    }
}
