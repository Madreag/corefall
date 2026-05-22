//! **M1**: reticle bloom from movement / recoil / sharp-aim.
//!
//! The reticle widget's render-side logic lives in `cf-render-2d/src/lib.rs`
//! (in the `update_reticle_*` systems). This module exposes the pure
//! bloom-scaling helpers so consumers + tests can compute bloom values
//! without depending on Bevy.

/// Bloom 1.0 = baseline reticle; bloom 7.0 = max-bloom running/jumping shot.
/// The formula mirrors OpenSoldat's `Sprites.pas:4870` — pixel radius scales
/// with the cube root of the bloom factor so the perceived size grows
/// sub-linearly with bloom magnitude.
pub fn reticle_pixel_radius(bloom_factor: f32, base_radius_px: f32) -> f32 {
    let bloom = bloom_factor.max(0.4);
    base_radius_px * bloom.powf(1.0 / 3.0)
}

/// on `Some(true)` or `None`. Matches the `update_reticle_color` system.
pub fn reticle_color_for_validity(tool_valid: Option<bool>) -> [f32; 3] {
    match tool_valid {
        Some(false) => [1.0, 0.25, 0.25],
        _ => [1.0, 1.0, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_min_clamps_below_partial_sharp_aim() {
        // Sharp aim can push bloom multiplier as low as 0.4 (sharp aim
        // tighten with full progress). Reticle pixel radius must not
        // collapse below sqrt(0.4) * base.
        let r = reticle_pixel_radius(0.1, 100.0);
        let expected = 100.0_f32 * 0.4_f32.powf(1.0 / 3.0);
        assert!((r - expected).abs() < 1.0);
    }

    #[test]
    fn bloom_baseline_returns_base() {
        assert!((reticle_pixel_radius(1.0, 100.0) - 100.0).abs() < 0.001);
    }

    #[test]
    fn invalid_target_red_tint() {
        assert_eq!(reticle_color_for_validity(Some(false)), [1.0, 0.25, 0.25]);
    }

    #[test]
    fn valid_target_white() {
        assert_eq!(reticle_color_for_validity(Some(true)), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn no_target_white() {
        assert_eq!(reticle_color_for_validity(None), [1.0, 1.0, 1.0]);
    }
}
