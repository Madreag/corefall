//! **M14J** § "zip-line kit".
//!
//! T2 equipment that deploys + anchors a zip-line cable between two
//! embedded grapple-hook anchors. The player clips on at the high end and
//! slides toward the low end at gravity-driven speed capped at 12 m/s.
//! Pressing `act.player.zipline_brake { engaged: true }` applies a
//! `-3 m/s²` deceleration.
//!
//! Pure / deterministic.

use serde::{Deserialize, Serialize};

pub const ZIP_KIT_T2_ID: &str = "zip_kit_t2";

/// at 12 m/s". Spec literal.
pub const ZIPLINE_MAX_SPEED_M_PER_S: f32 = 12.0;

/// deceleration". Spec literal.
pub const ZIPLINE_BRAKE_DECEL_M_PER_S2: f32 = 3.0;

/// cable (no slide).
pub const ZIPLINE_MIN_HEIGHT_DELTA_M: f32 = 0.5;

/// hard. Soft cap at 60 m.
pub const ZIPLINE_MAX_SPAN_M: f32 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZipKitDeployOutcome {
    Deployed {
        high_end: [f32; 2],
        low_end: [f32; 2],
        span_m: f32,
        height_delta_m: f32,
    },
    Rejected {
        reason: &'static str,
    },
}

/// apart at 5 m height delta". Deploys a zip line between `anchor_a` and
/// `anchor_b`. Returns the canonical (high_end, low_end) orientation so
/// the slide direction is unambiguous.
#[must_use]
pub fn deploy_zip_kit(anchor_a: [f32; 2], anchor_b: [f32; 2]) -> ZipKitDeployOutcome {
    if !anchor_a[0].is_finite() || !anchor_a[1].is_finite() || !anchor_b[0].is_finite() || !anchor_b[1].is_finite() {
        return ZipKitDeployOutcome::Rejected {
            reason: "non_finite_anchor",
        };
    }
    let dx = anchor_b[0] - anchor_a[0];
    let dy = anchor_b[1] - anchor_a[1];
    let span = (dx * dx + dy * dy).sqrt();
    if span < 1.0 {
        return ZipKitDeployOutcome::Rejected {
            reason: "anchors_too_close",
        };
    }
    if span > ZIPLINE_MAX_SPAN_M {
        return ZipKitDeployOutcome::Rejected {
            reason: "anchors_too_far_apart",
        };
    }
    let height_delta = (anchor_a[1] - anchor_b[1]).abs();
    if height_delta < ZIPLINE_MIN_HEIGHT_DELTA_M {
        return ZipKitDeployOutcome::Rejected {
            reason: "insufficient_height_delta",
        };
    }
    let (high, low) = if anchor_a[1] > anchor_b[1] {
        (anchor_a, anchor_b)
    } else {
        (anchor_b, anchor_a)
    };
    ZipKitDeployOutcome::Deployed {
        high_end: high,
        low_end: low,
        span_m: span,
        height_delta_m: height_delta,
    }
}

/// at 12 m/s" — advance the zip-line slide speed one tick.
///
/// `speed_m_s` is the current speed along the cable (positive = toward
/// low end). `slope_pct` is the cable slope (height delta / span). `dt_s`
/// is the sim tick dt. `brake_engaged` applies `ZIPLINE_BRAKE_DECEL_M_PER_S2`
/// per the spec. `gravity_m_s2` is the world gravity magnitude.
#[must_use]
pub fn zipline_step_speed(speed_m_s: f32, slope_pct: f32, dt_s: f32, brake_engaged: bool, gravity_m_s2: f32) -> f32 {
    let g = gravity_m_s2.abs();
    let accel_along_cable = g * slope_pct.abs();
    let brake_decel = if brake_engaged {
        ZIPLINE_BRAKE_DECEL_M_PER_S2
    } else {
        0.0
    };
    let net_accel = accel_along_cable - brake_decel;
    let new_speed = speed_m_s + net_accel * dt_s;
    new_speed.max(0.0).min(ZIPLINE_MAX_SPEED_M_PER_S)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_kit_deploys_canonical_orientation() {
        // anchor_a higher than anchor_b
        let outcome = deploy_zip_kit([0.0, 10.0], [25.0, 5.0]);
        match outcome {
            ZipKitDeployOutcome::Deployed {
                high_end,
                low_end,
                span_m,
                height_delta_m,
            } => {
                assert_eq!(high_end[1], 10.0);
                assert_eq!(low_end[1], 5.0);
                assert!((height_delta_m - 5.0).abs() < 1e-3);
                assert!(span_m > 25.0);
            }
            _ => panic!("expected Deployed"),
        }
    }

    #[test]
    fn zip_kit_rejects_flat_anchors() {
        let outcome = deploy_zip_kit([0.0, 5.0], [25.0, 5.0]);
        assert!(matches!(
            outcome,
            ZipKitDeployOutcome::Rejected { reason } if reason == "insufficient_height_delta"
        ));
    }

    #[test]
    fn zipline_speed_caps_at_12() {
        // Steep slope 50% → big accel, but cap at 12.
        let s = zipline_step_speed(15.0, 0.5, 1.0 / 60.0, false, 9.81);
        assert!(s <= ZIPLINE_MAX_SPEED_M_PER_S + 1e-3);
    }

    #[test]
    fn zipline_brake_reduces_speed() {
        // Mild slope, brake on → speed should drop or stay flat.
        let before = 8.0;
        let after = zipline_step_speed(before, 0.05, 1.0 / 60.0, true, 9.81);
        // Brake decel (-3) > slope accel (9.81 * 0.05 = ~0.49) → speed
        // decreases.
        assert!(after < before);
    }

    #[test]
    fn zipline_speed_never_negative() {
        let s = zipline_step_speed(0.5, 0.0, 1.0, true, 9.81);
        assert!(s >= 0.0);
    }
}
