//! **M12B** § Per-source occlusion through walls.
//!
//! Per spec acceptance:
//!
//! ```text
//! Scenario: Sound through concrete wall attenuates and low-passes
//!   Given a source SFX at world (5, 0) and the listener at world (-5, 0)
//!   And a single concrete wall at x=0 lies between them
//!   When the SFX fires
//!   Then audio.occluded fires with occlusion_db ≈ -28
//!   And SpatialEnvelope.medium_filter applies an 800 Hz low-pass cutoff
//!   And the listener hears a muffled thump (not a sharp transient)
//!
//! Scenario: Sound through cloth curtain barely attenuates
//!   Given a source SFX at world (3, 0) and the listener at world (-3, 0)
//!   And a cloth curtain at x=0 lies between them
//!   When the SFX fires
//!   Then audio.occluded fires with occlusion_db ≈ -3
//!   And the low-pass cutoff is 4000 Hz (only slight high-frequency loss)
//! ```
//!
//! Pure math; no Bevy, no rodio. The wall registry is supplied by the
//! caller (cf-control::engine plumbs the M3B / M19G wall list).

use serde::{Deserialize, Serialize};

/// **M12B** § Per-wall acoustic descriptor consumed by [`resolve_occlusion`].
///
/// `transmission_loss_db` and `low_pass_cutoff_hz` come from
/// `cf-material::registry` per the M12B per-material acoustic registry
/// table (e.g. concrete = -28 dB, cloth = -3 dB).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WallAcoustics {
    /// Per-wall transmission loss in decibels (positive number; the
    /// occlusion model subtracts).
    pub transmission_loss_db: f32,
    /// Wall's low-pass cutoff in Hz. The minimum cutoff across all walls
    /// in the path wins (i.e. the heaviest masking wall caps the audio
    /// bandwidth).
    pub low_pass_cutoff_hz: f32,
}

/// **M12B** § Resolved per-source occlusion descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OcclusionEnvelope {
    /// Total transmission loss in dB across the path. Negative number
    /// (the spec scenarios use the convention `occlusion_db ≈ -28`).
    pub occlusion_db: f32,
    /// Effective low-pass cutoff in Hz — min of every wall's cutoff in
    /// the path. `20_000` when no walls intervene (no low-pass).
    pub low_pass_cutoff_hz: f32,
    /// Number of walls actually traversed (post-cap). Used by the replay
    /// event for debugging.
    pub wall_count: u32,
    /// `true` when [`MAX_WALLS_PER_RAY`] was hit and additional walls
    /// were discarded. Recorded for forensics.
    pub clipped: bool,
}

impl OcclusionEnvelope {
    /// No-occlusion descriptor — source has direct line of sight to the
    /// listener (open air, no walls).
    #[must_use]
    pub const fn direct() -> Self {
        Self {
            occlusion_db: 0.0,
            low_pass_cutoff_hz: 20_000.0,
            wall_count: 0,
            clipped: false,
        }
    }

    /// Multiplicative gain factor for the linear-mixer surface (`10^(db/20)`).
    #[must_use]
    pub fn gain_factor(self) -> f32 {
        // `occlusion_db` is the *negative* dB drop; `gain = 10^(db/20)`.
        10.0_f32.powf(self.occlusion_db / 20.0).clamp(0.0, 1.0)
    }
}

/// **M12B** § Cap walls per ray (per spec § "Wall-traversal segmentation":
/// "Cap at 8 walls per ray (rare; concrete-on-concrete fortifications get
/// capped)").
pub const MAX_WALLS_PER_RAY: usize = 8;

/// **M12B** § Resolve the cumulative occlusion across a list of walls
/// between source and listener. The wall list is produced by
/// [`walls_between`] (Bresenham-style raster of the line segment against
/// the wall registry).
///
/// Per spec § Notes:
/// > the cumulative transmission_loss_db across all walls is summed; the
/// > effective low-pass cutoff is the minimum of every wall's cutoff (the
/// > heaviest masking wall caps the audio bandwidth).
#[must_use]
pub fn resolve_occlusion(walls: &[WallAcoustics]) -> OcclusionEnvelope {
    let mut walls_iter = walls.iter().take(MAX_WALLS_PER_RAY);
    let mut total_loss_db = 0.0_f32;
    let mut min_cutoff = 20_000.0_f32;
    let mut count = 0u32;
    for w in &mut walls_iter {
        total_loss_db += w.transmission_loss_db.max(0.0);
        if w.low_pass_cutoff_hz > 0.0 && w.low_pass_cutoff_hz < min_cutoff {
            min_cutoff = w.low_pass_cutoff_hz;
        }
        count += 1;
    }
    let clipped = walls.len() > MAX_WALLS_PER_RAY;
    OcclusionEnvelope {
        // Spec scenarios use negative convention (-28 dB through concrete).
        occlusion_db: -total_loss_db,
        low_pass_cutoff_hz: min_cutoff,
        wall_count: count,
        clipped,
    }
}

/// **M12B** § Bresenham-style raster of a 2D line segment against a wall
/// registry. The caller supplies the wall registry as a closure that
/// returns the material id at a given world cell — this keeps cf-audio
/// independent of the M3B/M19G wall-registry shape.
///
/// The closure should return `Some(wall_acoustics)` for cells that
/// contain a wall, `None` otherwise. Successive identical wall cells are
/// **deduplicated**: a single 4 m thick concrete wall is one
/// transmission_loss event, not 8.
///
/// Determinism: rasters in `(x, y)` raster order (left → right, then top
/// → bottom). Cap respects [`MAX_WALLS_PER_RAY`].
#[must_use]
pub fn walls_between<F>(source: [f32; 2], listener: [f32; 2], mut sample: F) -> Vec<WallAcoustics>
where
    F: FnMut([i32; 2]) -> Option<WallAcoustics>,
{
    let x0 = source[0].round() as i32;
    let y0 = source[1].round() as i32;
    let x1 = listener[0].round() as i32;
    let y1 = listener[1].round() as i32;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;
    let mut out: Vec<WallAcoustics> = Vec::new();
    let mut last: Option<WallAcoustics> = None;
    loop {
        if let Some(wall) = sample([x, y]) {
            // Dedup: skip if this cell is the same wall as the previous
            // sampled cell (thick walls span many cells; we only want
            // one transmission_loss event per wall).
            let same_as_last = match last {
                Some(prev) => (prev.transmission_loss_db - wall.transmission_loss_db).abs() < 1e-4
                    && (prev.low_pass_cutoff_hz - wall.low_pass_cutoff_hz).abs() < 1e-4,
                None => false,
            };
            if !same_as_last && out.len() < MAX_WALLS_PER_RAY {
                out.push(wall);
            }
            last = Some(wall);
        } else {
            last = None;
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concrete() -> WallAcoustics {
        WallAcoustics {
            transmission_loss_db: 28.0,
            low_pass_cutoff_hz: 800.0,
        }
    }

    fn cloth() -> WallAcoustics {
        WallAcoustics {
            transmission_loss_db: 3.0,
            low_pass_cutoff_hz: 4000.0,
        }
    }

    #[test]
    fn direct_path_has_no_occlusion() {
        let env = resolve_occlusion(&[]);
        assert!(env.occlusion_db.abs() < 1e-6);
        assert!(env.low_pass_cutoff_hz >= 20_000.0);
        assert_eq!(env.wall_count, 0);
        assert!(!env.clipped);
    }

    #[test]
    fn concrete_wall_drops_by_28_db_and_caps_at_800_hz() {
        let env = resolve_occlusion(&[concrete()]);
        assert!((env.occlusion_db - -28.0).abs() < 1e-4);
        assert!((env.low_pass_cutoff_hz - 800.0).abs() < 1e-4);
        assert_eq!(env.wall_count, 1);
    }

    #[test]
    fn cloth_curtain_drops_by_3_db_and_caps_at_4000_hz() {
        let env = resolve_occlusion(&[cloth()]);
        assert!((env.occlusion_db - -3.0).abs() < 1e-4);
        assert!((env.low_pass_cutoff_hz - 4000.0).abs() < 1e-4);
    }

    #[test]
    fn multiple_walls_sum_transmission_loss_and_min_cutoff() {
        let env = resolve_occlusion(&[concrete(), cloth()]);
        assert!((env.occlusion_db - -31.0).abs() < 1e-4);
        // min of 800 and 4000 = 800.
        assert!((env.low_pass_cutoff_hz - 800.0).abs() < 1e-4);
        assert_eq!(env.wall_count, 2);
    }

    #[test]
    fn ray_caps_at_max_walls() {
        let walls: Vec<WallAcoustics> = std::iter::repeat(concrete()).take(20).collect();
        let env = resolve_occlusion(&walls);
        assert_eq!(env.wall_count as usize, MAX_WALLS_PER_RAY);
        assert!(env.clipped);
    }

    #[test]
    fn gain_factor_matches_db_conversion() {
        let env = OcclusionEnvelope {
            occlusion_db: -6.0,
            low_pass_cutoff_hz: 20_000.0,
            wall_count: 1,
            clipped: false,
        };
        // 10^(-6/20) ≈ 0.501.
        assert!((env.gain_factor() - 0.501).abs() < 0.01);
    }

    #[test]
    fn gain_factor_for_direct_is_unity() {
        assert!((OcclusionEnvelope::direct().gain_factor() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn walls_between_horizontal_ray_finds_wall_at_origin() {
        let env = walls_between([5.0, 0.0], [-5.0, 0.0], |p| {
            if p[0] == 0 && p[1] == 0 {
                Some(concrete())
            } else {
                None
            }
        });
        assert_eq!(env.len(), 1);
        assert!((env[0].transmission_loss_db - 28.0).abs() < 1e-4);
    }

    #[test]
    fn walls_between_deduplicates_thick_wall() {
        // Wall is 3 cells thick at x=0,1,2 — expect ONE event, not three.
        let env = walls_between([5.0, 0.0], [-5.0, 0.0], |p| {
            if p[1] == 0 && (p[0] == 0 || p[0] == 1 || p[0] == 2) {
                Some(concrete())
            } else {
                None
            }
        });
        assert_eq!(env.len(), 1);
    }

    #[test]
    fn walls_between_distinguishes_different_materials() {
        // Two distinct walls (concrete at x=0, cloth at x=-2).
        let env = walls_between([5.0, 0.0], [-5.0, 0.0], |p| {
            if p[1] == 0 && p[0] == 0 {
                Some(concrete())
            } else if p[1] == 0 && p[0] == -2 {
                Some(cloth())
            } else {
                None
            }
        });
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn walls_between_no_walls_returns_empty() {
        let env = walls_between([0.0, 0.0], [10.0, 0.0], |_| None);
        assert!(env.is_empty());
    }

    #[test]
    fn occlusion_envelope_round_trips_through_serde() {
        let env = resolve_occlusion(&[concrete()]);
        let s = serde_json::to_string(&env).unwrap();
        let back: OcclusionEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(env, back);
    }
}
