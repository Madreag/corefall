//! M8A § GPU fragment shader — 5-mode material overlay.

use serde::{Deserialize, Serialize};

pub const MATERIAL_OVERLAY_WGSL: &str = include_str!("../shaders/material_overlay.wgsl");

/// M3-shipped 5 overlay modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    #[default]
    Integrity,
    Pathability,
    Mobility,
    Hazard,
    BuildRepair,
}

impl OverlayMode {
    pub fn as_u32(self) -> u32 {
        match self {
            OverlayMode::Integrity => 0,
            OverlayMode::Pathability => 1,
            OverlayMode::Mobility => 2,
            OverlayMode::Hazard => 3,
            OverlayMode::BuildRepair => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpuOverlaySystem {
    pub mode: OverlayMode,
    pub shader_source: &'static str,
}

impl Default for GpuOverlaySystem {
    fn default() -> Self {
        Self {
            mode: OverlayMode::default(),
            shader_source: MATERIAL_OVERLAY_WGSL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_source_is_loaded() {
        assert!(MATERIAL_OVERLAY_WGSL.contains("fragment_overlay"));
    }

    #[test]
    fn overlay_modes_have_distinct_u32() {
        let modes = [
            OverlayMode::Integrity,
            OverlayMode::Pathability,
            OverlayMode::Mobility,
            OverlayMode::Hazard,
            OverlayMode::BuildRepair,
        ];
        for (i, m) in modes.iter().enumerate() {
            assert_eq!(m.as_u32() as usize, i);
        }
    }
}
