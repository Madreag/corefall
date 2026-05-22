//! **M14A** § "cf-render-2d::quick_action_render" — QAB + radial sprite
//! data for the renderer. Pure data; the actual draw lives in cf-app.

use serde::{Deserialize, Serialize};

/// Per-slot render data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SlotRenderData {
    pub slot_index: u8,
    pub item_id: String,
    pub icon_asset_id: String,
    pub cooldown_fill: f32,
    pub ammo_text: String,
    pub disabled: bool,
    pub highlighted: bool,
}

impl SlotRenderData {
    pub fn cooldown_mask_fraction(&self) -> f32 {
        1.0 - self.cooldown_fill.clamp(0.0, 1.0)
    }
}

/// Radial render snapshot.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RadialRenderData {
    pub is_open: bool,
    pub sim_time_multiplier: f32,
    pub selected_slice: u8,
    pub deadzone_radius_px: f32,
    pub slot_render: Vec<SlotRenderData>,
}

/// derived from the radial sim time multiplier. Returns 0..1 alpha.
pub fn time_slow_vignette_alpha(sim_time_multiplier: f32, reduce_motion: bool) -> f32 {
    if reduce_motion {
        return 0.0;
    }
    let span = 1.0 - 0.25; // span from 1.0 to 0.25
    ((1.0 - sim_time_multiplier) / span).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_fades_in_as_sim_slows() {
        assert!((time_slow_vignette_alpha(1.0, false) - 0.0).abs() < 1e-6);
        assert!((time_slow_vignette_alpha(0.25, false) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vignette_disabled_in_reduce_motion() {
        assert_eq!(time_slow_vignette_alpha(0.25, true), 0.0);
    }

    #[test]
    fn cooldown_mask_inverts_fill() {
        let s = SlotRenderData {
            cooldown_fill: 0.3,
            ..Default::default()
        };
        assert!((s.cooldown_mask_fraction() - 0.7).abs() < 1e-6);
    }
}
