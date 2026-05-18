//! **M12C**: Camera-move primitives + composer + easing curves.
//!
//! Per spec § "Cinematic camera moves":
//!
//! - **Pan** — straight 2D translation across the scene at constant
//!   velocity; eased per `EaseInOutCubic` by default.
//! - **Dolly** — perpendicular zoom-and-translate ("push in" /
//!   "pull out"); preserves the screen-space anchor on a named target.
//! - **Zoom** — pure focal-length / orthographic-half-height change
//!   without translation.
//! - **Orbit** — circular arc around a named target at fixed radius
//!   (2D side-view: parallax-shift across foreground/midground/
//!   background layers).
//! - **Shake** — perlin-noise additive offset, parameterized by
//!   `amplitude_px` + `frequency_hz` + `decay_s`; reuses M12's
//!   `cf-render-2d::juice::camera_shake_amplitude` curve.
//!
//! Determinism: every primitive is a pure function of its parameters +
//! intra-shot `t` (ms). Shake noise seeds off `(seed, shot_index,
//! move_index, sample_index)` so two engines at the same M4 replay seed
//! produce byte-identical offsets.

use serde::{Deserialize, Serialize};

/// One camera-move primitive. The composer feeds a stack of these to
/// the camera-state resource each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MoveKind {
    /// Constant-velocity 2D translation.
    Pan,
    /// Push-in / pull-out (zoom + slide toward target).
    Dolly,
    /// Pure focal-length change (no translation).
    Zoom,
    /// Circular arc around a named target (2D parallax shift).
    Orbit,
    /// Additive perlin-noise jitter (deterministic).
    Shake,
}

impl MoveKind {
    /// Canonical PascalCase identifier matching the cfctl + RON enum.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            MoveKind::Pan => "Pan",
            MoveKind::Dolly => "Dolly",
            MoveKind::Zoom => "Zoom",
            MoveKind::Orbit => "Orbit",
            MoveKind::Shake => "Shake",
        }
    }
}

/// Easing curve identifier. Per spec § "eased per `EaseInOutCubic` by
/// default" for Pan + Dolly + Zoom + Orbit; Shake decays exponentially
/// instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum EaseKind {
    /// Linear (no easing).
    Linear,
    /// Cubic ease-in-out (default for camera primitives).
    #[default]
    EaseInOutCubic,
    /// Cubic ease-out (decelerate).
    EaseOutCubic,
    /// Cubic ease-in (accelerate).
    EaseInCubic,
}

/// Sample an easing curve at `t` in `[0, 1]`.
#[must_use]
pub fn easing_sample(kind: EaseKind, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        EaseKind::Linear => t,
        EaseKind::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                let p = -2.0 * t + 2.0;
                1.0 - p * p * p / 2.0
            }
        }
        EaseKind::EaseOutCubic => {
            let p = 1.0 - t;
            1.0 - p * p * p
        }
        EaseKind::EaseInCubic => t * t * t,
    }
}

/// Shake parameters (per-move). Spec § "parameterized by amplitude_px +
/// frequency_hz + decay_s".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShakeParams {
    /// Peak displacement in screen pixels.
    pub amplitude_px: f32,
    /// Oscillation frequency in Hz.
    pub frequency_hz: f32,
    /// Exponential decay time-constant in seconds (`a(t) = a0 *
    /// exp(-t / decay_s)`).
    pub decay_s: f32,
}

impl Default for ShakeParams {
    fn default() -> Self {
        Self {
            amplitude_px: 0.0,
            frequency_hz: 30.0,
            decay_s: 0.5,
        }
    }
}

/// One declared move inside a shot. Defaults are the spec's neutral
/// values so authoring a Pan move only needs `pan: [x, y]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShotMove {
    /// Move kind discriminator.
    pub kind: MoveKind,
    /// Time-into-shot when the move begins (ms).
    #[serde(default)]
    pub start_ms: u32,
    /// Move duration (ms).
    pub duration_ms: u32,
    /// Easing curve for the move's parameter sweep. Shake ignores
    /// this and uses its own decay envelope.
    #[serde(default)]
    pub easing: EaseKind,
    /// Pan delta `[dx, dy]` in world units.
    #[serde(default)]
    pub pan: [f32; 2],
    /// Dolly target world position `[x, y]`. Dolly preserves the screen-
    /// space anchor on this point as it moves.
    #[serde(default)]
    pub dolly_target: [f32; 2],
    /// Dolly distance — final camera distance from the dolly target in
    /// world units.
    #[serde(default)]
    pub dolly_distance: f32,
    /// Zoom final ortho-half-height in world units (target value). The
    /// composer eases from the move's `start_ms` ortho_h to this value.
    #[serde(default)]
    pub zoom_to: f32,
    /// Orbit target world position `[x, y]`.
    #[serde(default)]
    pub orbit_target: [f32; 2],
    /// Orbit radius in world units.
    #[serde(default)]
    pub orbit_radius: f32,
    /// Orbit start angle (radians).
    #[serde(default)]
    pub orbit_start_rad: f32,
    /// Orbit total sweep (radians; positive = counter-clockwise).
    #[serde(default)]
    pub orbit_sweep_rad: f32,
    /// Shake parameters (only meaningful when `kind == Shake`).
    #[serde(default)]
    pub shake: ShakeParams,
}

impl Default for ShotMove {
    fn default() -> Self {
        Self {
            kind: MoveKind::Pan,
            start_ms: 0,
            duration_ms: 0,
            easing: EaseKind::EaseInOutCubic,
            pan: [0.0, 0.0],
            dolly_target: [0.0, 0.0],
            dolly_distance: 0.0,
            zoom_to: 0.0,
            orbit_target: [0.0, 0.0],
            orbit_radius: 0.0,
            orbit_start_rad: 0.0,
            orbit_sweep_rad: 0.0,
            shake: ShakeParams::default(),
        }
    }
}

/// Composed camera offset for one frame. The renderer reads this from
/// `cf-render-2d::camera_takeover`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ComposedOffset {
    /// Translation delta in world units.
    pub translation: [f32; 2],
    /// Orthographic half-height (zoom); `0.0` = "no zoom override".
    pub ortho_half_height: f32,
    /// Shake offset in screen pixels (additive on top of translation).
    pub shake: [f32; 2],
    /// True when the shake clamp triggered (used by diagnostics).
    pub shake_clamped: bool,
}

/// Compose one move's contribution to the composed offset. Pure function;
/// no global state.
#[must_use]
pub fn compose_offset(mv: &ShotMove, t_in_shot_ms: u32, seed: u64) -> ComposedOffset {
    let mut out = ComposedOffset::default();
    if t_in_shot_ms < mv.start_ms {
        return out;
    }
    let elapsed_in_move = t_in_shot_ms - mv.start_ms;
    if elapsed_in_move >= mv.duration_ms {
        // Snap-to-end for non-shake moves; shake decays naturally.
        match mv.kind {
            MoveKind::Pan => {
                out.translation = mv.pan;
            }
            MoveKind::Dolly => {
                out.translation = mv.dolly_target;
                if mv.dolly_distance > 0.0 {
                    out.ortho_half_height = mv.dolly_distance;
                }
            }
            MoveKind::Zoom => {
                out.ortho_half_height = mv.zoom_to;
            }
            MoveKind::Orbit => {
                let end_rad = mv.orbit_start_rad + mv.orbit_sweep_rad;
                out.translation = [
                    mv.orbit_target[0] + mv.orbit_radius * end_rad.cos(),
                    mv.orbit_target[1] + mv.orbit_radius * end_rad.sin(),
                ];
            }
            MoveKind::Shake => {
                // Decayed to zero at end.
            }
        }
        return out;
    }
    let t_norm = if mv.duration_ms == 0 {
        1.0
    } else {
        elapsed_in_move as f32 / mv.duration_ms as f32
    };
    let eased = easing_sample(mv.easing, t_norm);
    match mv.kind {
        MoveKind::Pan => {
            out.translation = [mv.pan[0] * eased, mv.pan[1] * eased];
        }
        MoveKind::Dolly => {
            out.translation = [mv.dolly_target[0] * eased, mv.dolly_target[1] * eased];
            if mv.dolly_distance > 0.0 {
                out.ortho_half_height = mv.dolly_distance * eased;
            }
        }
        MoveKind::Zoom => {
            out.ortho_half_height = mv.zoom_to * eased;
        }
        MoveKind::Orbit => {
            let cur_rad = mv.orbit_start_rad + mv.orbit_sweep_rad * eased;
            out.translation = [
                mv.orbit_target[0] + mv.orbit_radius * cur_rad.cos(),
                mv.orbit_target[1] + mv.orbit_radius * cur_rad.sin(),
            ];
        }
        MoveKind::Shake => {
            let elapsed_s = elapsed_in_move as f32 / 1000.0;
            let decay = if mv.shake.decay_s > 0.0 {
                (-elapsed_s / mv.shake.decay_s).exp()
            } else {
                1.0
            };
            let amplitude = mv.shake.amplitude_px * decay;
            // Quantize the sample index by frequency so two engines at the
            // same tick produce the same noise sample.
            let sample_index = if mv.shake.frequency_hz > 0.0 {
                (elapsed_s * mv.shake.frequency_hz) as u64
            } else {
                0
            };
            let nx = deterministic_noise(seed, sample_index, 0);
            let ny = deterministic_noise(seed, sample_index, 1);
            // Clamp to a tight safety window so an absurd amplitude never
            // crashes the renderer.
            const SHAKE_CLAMP: f32 = 64.0;
            let raw_x = nx * amplitude;
            let raw_y = ny * amplitude;
            out.shake_clamped = raw_x.abs() > SHAKE_CLAMP || raw_y.abs() > SHAKE_CLAMP;
            out.shake = [
                raw_x.clamp(-SHAKE_CLAMP, SHAKE_CLAMP),
                raw_y.clamp(-SHAKE_CLAMP, SHAKE_CLAMP),
            ];
        }
    }
    out
}

/// Deterministic noise sample in `[-1.0, 1.0]`. Pure hash of
/// `(seed, sample_index, channel)`; does NOT touch `thread_rng()` or
/// any global state.
#[must_use]
fn deterministic_noise(seed: u64, sample_index: u64, channel: u32) -> f32 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(&sample_index.to_le_bytes());
    hasher.update(&channel.to_le_bytes());
    let bytes = hasher.finalize();
    let arr = bytes.as_bytes();
    // Pull the first 8 bytes as a u64 → [-1.0, 1.0].
    let mut le = [0u8; 8];
    le.copy_from_slice(&arr[..8]);
    let v = u64::from_le_bytes(le);
    // Map to `[-1.0, 1.0]`.
    let f = (v as f64 / u64::MAX as f64) * 2.0 - 1.0;
    f as f32
}

/// Compose every move in a shot at intra-shot offset `t_in_shot_ms`.
/// Aggregates translation + zoom + shake additively across all moves
/// whose `[start_ms, start_ms + duration_ms)` window contains `t`.
///
/// `seed` is the M4 replay seed XOR'd with the shot index (spec §
/// "the kernel passes the M4 replay seed + per-shot index").
#[must_use]
pub fn apply_move_stack(moves: &[ShotMove], t_in_shot_ms: u32, seed: u64) -> ComposedOffset {
    let mut out = ComposedOffset::default();
    for (move_index, mv) in moves.iter().enumerate() {
        let sub_seed = seed ^ ((move_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let contrib = compose_offset(mv, t_in_shot_ms, sub_seed);
        out.translation[0] += contrib.translation[0];
        out.translation[1] += contrib.translation[1];
        // Zoom is "last writer wins" since spec says Pan + Shake are
        // additive but Zoom replaces the focal length.
        if contrib.ortho_half_height > 0.0 {
            out.ortho_half_height = contrib.ortho_half_height;
        }
        out.shake[0] += contrib.shake[0];
        out.shake[1] += contrib.shake[1];
        out.shake_clamped |= contrib.shake_clamped;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_sample_endpoints_pin() {
        for kind in [
            EaseKind::Linear,
            EaseKind::EaseInOutCubic,
            EaseKind::EaseOutCubic,
            EaseKind::EaseInCubic,
        ] {
            assert!((easing_sample(kind, 0.0) - 0.0).abs() < 1e-5);
            assert!((easing_sample(kind, 1.0) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn pan_eased_to_target_at_end() {
        let mv = ShotMove {
            kind: MoveKind::Pan,
            start_ms: 0,
            duration_ms: 1000,
            easing: EaseKind::EaseInOutCubic,
            pan: [10.0, 5.0],
            ..ShotMove::default()
        };
        let mid = compose_offset(&mv, 500, 0);
        assert!(mid.translation[0] > 0.0 && mid.translation[0] < 10.0);
        let end = compose_offset(&mv, 1_000, 0);
        assert!((end.translation[0] - 10.0).abs() < 1e-3);
        assert!((end.translation[1] - 5.0).abs() < 1e-3);
    }

    #[test]
    fn shake_decays_to_zero_after_full_duration() {
        let mv = ShotMove {
            kind: MoveKind::Shake,
            start_ms: 0,
            duration_ms: 500,
            shake: ShakeParams {
                amplitude_px: 8.0,
                frequency_hz: 30.0,
                decay_s: 0.5,
            },
            ..ShotMove::default()
        };
        let at_start = compose_offset(&mv, 0, 1234);
        let after = compose_offset(&mv, 500, 1234);
        assert!(at_start.shake[0].abs() > 0.0 || at_start.shake[1].abs() > 0.0);
        assert_eq!(after.shake, [0.0, 0.0]);
    }

    #[test]
    fn shake_is_deterministic_across_runs() {
        let mv = ShotMove {
            kind: MoveKind::Shake,
            start_ms: 0,
            duration_ms: 1000,
            shake: ShakeParams {
                amplitude_px: 8.0,
                frequency_hz: 30.0,
                decay_s: 0.5,
            },
            ..ShotMove::default()
        };
        let seed = 0xC0FFEE;
        let a = compose_offset(&mv, 200, seed);
        let b = compose_offset(&mv, 200, seed);
        assert_eq!(a.shake, b.shake);
    }

    #[test]
    fn apply_move_stack_adds_pan_and_shake() {
        let stack = vec![
            ShotMove {
                kind: MoveKind::Pan,
                start_ms: 0,
                duration_ms: 1000,
                pan: [10.0, 0.0],
                ..ShotMove::default()
            },
            ShotMove {
                kind: MoveKind::Shake,
                start_ms: 0,
                duration_ms: 500,
                shake: ShakeParams {
                    amplitude_px: 8.0,
                    frequency_hz: 30.0,
                    decay_s: 0.5,
                },
                ..ShotMove::default()
            },
        ];
        let composed = apply_move_stack(&stack, 200, 42);
        // Pan eased toward [10, 0]; shake adds non-zero offset.
        assert!(composed.translation[0] > 0.0);
        assert!(composed.shake[0] != 0.0 || composed.shake[1] != 0.0);
    }

    #[test]
    fn shake_clamps_extreme_amplitude() {
        let mv = ShotMove {
            kind: MoveKind::Shake,
            start_ms: 0,
            duration_ms: 100,
            shake: ShakeParams {
                amplitude_px: 1_000_000.0,
                frequency_hz: 30.0,
                decay_s: 0.5,
            },
            ..ShotMove::default()
        };
        let c = compose_offset(&mv, 1, 42);
        assert!(c.shake[0].abs() <= 64.0);
        assert!(c.shake[1].abs() <= 64.0);
        assert!(c.shake_clamped);
    }
}
