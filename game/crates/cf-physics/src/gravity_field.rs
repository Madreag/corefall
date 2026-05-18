//! **M14B** § gravity field — universal gravity with per-cell + per-region
//! overrides + per-actor magnetic boot anchors.
//!
//! The producer side of DR-038 universal gravity. Every consumer that
//! previously read a single `gravity: f32` scalar can now sample a
//! [`GravityVec`] (magnitude + direction) at a world position; M14A
//! PARITY-121 (low-g jump arc), PARITY-86 (wind force) and PARITY-97
//! (low-g cell) close the loop against this module's outputs.
//!
//! The module is deterministic + stateless: callers own the override
//! vector + pass it into [`apply_overrides`] each tick. Last-writer-wins
//! stacking discipline is honoured by walking the override slice in
//! declaration order; the actor-scope `MagneticBoots` override fires
//! last and resets to baseline so the spec's "scenario-base → per-region
//! → per-cell → per-actor" order is preserved.

use serde::{Deserialize, Serialize};

/// **M14B** § gravity vector — magnitude in m/s² (or pixel units per the
/// scenario's authored scale) + a unit-length direction vector.
///
/// `Default` returns Earth-down 9.81 m/s² for callers that want a baseline
/// before applying scenario overrides.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GravityVec {
    pub magnitude: f32,
    pub direction: [f32; 2],
}

impl GravityVec {
    /// Construct a gravity vector, normalising the direction. A zero
    /// direction defaults to `[0, -1]` (Earth-down).
    #[must_use]
    pub fn new(magnitude: f32, direction: [f32; 2]) -> Self {
        let len_sq = direction[0] * direction[0] + direction[1] * direction[1];
        if len_sq <= f32::EPSILON {
            Self {
                magnitude: magnitude.max(0.0),
                direction: [0.0, -1.0],
            }
        } else {
            let inv_len = 1.0 / len_sq.sqrt();
            Self {
                magnitude: magnitude.max(0.0),
                direction: [direction[0] * inv_len, direction[1] * inv_len],
            }
        }
    }

    /// Earth-default (9.81 m/s² down).
    #[must_use]
    pub fn earth_default() -> Self {
        Self {
            magnitude: 9.81,
            direction: [0.0, -1.0],
        }
    }

    /// True when the magnitude is below `1e-4` (effectively zero-g).
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.magnitude < 1e-4
    }

    /// Acceleration vector (`magnitude × direction`) in the caller's
    /// authored units.
    #[must_use]
    pub fn acceleration(&self) -> [f32; 2] {
        [self.direction[0] * self.magnitude, self.direction[1] * self.magnitude]
    }
}

impl Default for GravityVec {
    fn default() -> Self {
        Self::earth_default()
    }
}

/// **M14B** § gravity override kinds.
///
/// Stacking order honoured by [`apply_overrides`] (last writer wins per
/// the spec's "scenario-base → per-region → per-cell → per-actor" rule):
///
/// 1. Region overrides (`RegionLowG`, `ReverseG`).
/// 2. Per-cell well overrides (`UniformWell`, `DamagedGrav`).
/// 3. Per-actor anchor (`MagneticBoots`) — resets the result to baseline.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GravityOverride {
    /// Per-cell well that bends gravity toward `center` with the given
    /// magnitude. Only applies inside `radius` (in world units).
    UniformWell {
        id: u32,
        center: [f32; 2],
        radius: f32,
        magnitude: f32,
    },
    /// Per-region rectangle that swaps the magnitude for `local_g`
    /// (direction is preserved).
    RegionLowG {
        id: u32,
        min: [f32; 2],
        max: [f32; 2],
        local_g: f32,
    },
    /// Per-actor magnetic-boot anchor; sampling for `actor_id` cancels
    /// all overrides and returns to baseline.
    MagneticBoots { id: u32, actor_id: u64 },
    /// Per-region rectangle that flips the direction sign (and optionally
    /// scales the magnitude).
    ReverseG {
        id: u32,
        min: [f32; 2],
        max: [f32; 2],
    },
    /// Per-cell damaged-generator override — magnitude decays from
    /// `magnitude_factor` at `center` toward the wave-front boundary.
    /// `wave_front_radius` is the progressive-collapse radius (grows over
    /// time; producer holds the state, the override sees the current
    /// radius).
    DamagedGrav {
        id: u32,
        center: [f32; 2],
        radius: f32,
        magnitude_factor: f32,
        wave_front_radius: f32,
    },
}

impl GravityOverride {
    /// Stable id of the override (used by event payloads to identify the
    /// activated/deactivated entry).
    #[must_use]
    pub fn id(&self) -> u32 {
        match self {
            Self::UniformWell { id, .. }
            | Self::RegionLowG { id, .. }
            | Self::MagneticBoots { id, .. }
            | Self::ReverseG { id, .. }
            | Self::DamagedGrav { id, .. } => *id,
        }
    }

    /// One of `"uniform_well" | "region_low_g" | "magnetic_boots" |
    /// "reverse_g" | "damaged_grav"`. Mirrors the serde `kind` tag for
    /// event payloads + observe surfaces.
    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::UniformWell { .. } => "uniform_well",
            Self::RegionLowG { .. } => "region_low_g",
            Self::MagneticBoots { .. } => "magnetic_boots",
            Self::ReverseG { .. } => "reverse_g",
            Self::DamagedGrav { .. } => "damaged_grav",
        }
    }

    /// True when this override's spatial extent contains `world_pos`.
    /// `MagneticBoots` is per-actor (not spatial) so it always returns
    /// `false` here — callers use [`actor_anchored`] for that.
    #[must_use]
    pub fn contains(&self, world_pos: [f32; 2]) -> bool {
        match self {
            Self::UniformWell { center, radius, .. } | Self::DamagedGrav { center, radius, .. } => {
                let dx = world_pos[0] - center[0];
                let dy = world_pos[1] - center[1];
                dx * dx + dy * dy <= radius * radius
            }
            Self::RegionLowG { min, max, .. } | Self::ReverseG { min, max, .. } => {
                world_pos[0] >= min[0] && world_pos[0] <= max[0] && world_pos[1] >= min[1] && world_pos[1] <= max[1]
            }
            Self::MagneticBoots { .. } => false,
        }
    }

    /// True when the override is a per-actor magnetic-boot anchor for
    /// the given actor id.
    #[must_use]
    pub fn actor_anchored(&self, actor_id: u64) -> bool {
        matches!(self, Self::MagneticBoots { actor_id: a, .. } if *a == actor_id)
    }
}

/// **M14B** § Result of [`apply_overrides`] — the final gravity vector
/// plus the ids of overrides that were active at the queried position.
#[derive(Debug, Clone, PartialEq)]
pub struct OverrideResult {
    pub gravity: GravityVec,
    /// Ids of every override that contributed (in stacking order).
    pub active_ids: Vec<u32>,
}

/// **M14B** § Apply the override list to `base` at `world_pos` for an
/// optional `actor_id`.
///
/// Stacking discipline (spec):
///
/// > Gravity overrides MUST stack deterministically. Order: scenario-base
/// > → per-region → per-cell → per-actor (magnetic boots). Last writer
/// > wins.
///
/// This function walks overrides in **declaration order** so callers can
/// author their `overrides[]` slice with the stacking order baked in
/// (per-region first, per-cell second, magnetic boots last). The
/// magnetic-boot pass always resets the result to `base` when the actor
/// is anchored.
#[must_use]
pub fn apply_overrides(
    base: GravityVec,
    world_pos: [f32; 2],
    actor_id: Option<u64>,
    overrides: &[GravityOverride],
) -> OverrideResult {
    let mut current = base;
    let mut active_ids: Vec<u32> = Vec::new();
    let mut magnetic_boot_id: Option<u32> = None;

    for ovr in overrides {
        match ovr {
            GravityOverride::UniformWell {
                id,
                center,
                radius,
                magnitude,
            } => {
                if !ovr.contains(world_pos) {
                    continue;
                }
                let dx = center[0] - world_pos[0];
                let dy = center[1] - world_pos[1];
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= f32::EPSILON {
                    current.magnitude = *magnitude;
                    active_ids.push(*id);
                    continue;
                }
                let dist = dist_sq.sqrt();
                let inv_dist = 1.0 / dist;
                let dir = [dx * inv_dist, dy * inv_dist];
                let falloff = (1.0 - dist / radius.max(f32::EPSILON)).clamp(0.0, 1.0);
                let extra_mag = *magnitude * falloff;
                let ax = current.direction[0] * current.magnitude + dir[0] * extra_mag;
                let ay = current.direction[1] * current.magnitude + dir[1] * extra_mag;
                let new_mag = (ax * ax + ay * ay).sqrt();
                let new_dir = if new_mag <= f32::EPSILON {
                    current.direction
                } else {
                    [ax / new_mag, ay / new_mag]
                };
                current = GravityVec {
                    magnitude: new_mag,
                    direction: new_dir,
                };
                active_ids.push(*id);
            }
            GravityOverride::RegionLowG { id, local_g, .. } => {
                if ovr.contains(world_pos) {
                    current = GravityVec {
                        magnitude: local_g.max(0.0),
                        direction: current.direction,
                    };
                    active_ids.push(*id);
                }
            }
            GravityOverride::ReverseG { id, .. } => {
                if ovr.contains(world_pos) {
                    current = GravityVec {
                        magnitude: current.magnitude,
                        direction: [-current.direction[0], -current.direction[1]],
                    };
                    active_ids.push(*id);
                }
            }
            GravityOverride::DamagedGrav {
                id,
                center,
                radius,
                magnitude_factor,
                wave_front_radius,
            } => {
                if !ovr.contains(world_pos) {
                    continue;
                }
                let dx = world_pos[0] - center[0];
                let dy = world_pos[1] - center[1];
                let dist = (dx * dx + dy * dy).sqrt();
                // Wave front grows from the centre outward; cells past
                // the front are unaffected, cells inside the front get
                // the magnitude scaled by `magnitude_factor`.
                if dist <= *wave_front_radius && *wave_front_radius <= *radius {
                    let factor = magnitude_factor.clamp(0.0, 4.0);
                    current = GravityVec {
                        magnitude: current.magnitude * factor,
                        direction: current.direction,
                    };
                    active_ids.push(*id);
                }
            }
            GravityOverride::MagneticBoots { id, actor_id: anchor_actor } => {
                if let Some(a) = actor_id {
                    if a == *anchor_actor {
                        magnetic_boot_id = Some(*id);
                    }
                }
            }
        }
    }
    if let Some(id) = magnetic_boot_id {
        active_ids.push(id);
        current = base;
    }
    OverrideResult {
        gravity: current,
        active_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_vec_new_normalises_direction() {
        let g = GravityVec::new(10.0, [0.0, -2.0]);
        assert!((g.direction[0]).abs() < 1e-6);
        assert!((g.direction[1] - -1.0).abs() < 1e-6);
        assert!((g.magnitude - 10.0).abs() < 1e-6);
    }

    #[test]
    fn gravity_vec_zero_direction_defaults_to_earth_down() {
        let g = GravityVec::new(5.0, [0.0, 0.0]);
        assert!(g.direction[0].abs() < 1e-6);
        assert!((g.direction[1] - -1.0).abs() < 1e-6);
        assert!((g.magnitude - 5.0).abs() < 1e-6);
    }

    #[test]
    fn earth_default_is_9_81_down() {
        let g = GravityVec::earth_default();
        assert!((g.magnitude - 9.81).abs() < 1e-6);
        assert!(g.direction[0].abs() < 1e-6);
        assert!((g.direction[1] - -1.0).abs() < 1e-6);
    }

    #[test]
    fn apply_overrides_passes_through_when_empty() {
        let base = GravityVec::earth_default();
        let out = apply_overrides(base, [100.0, 50.0], Some(1), &[]);
        assert_eq!(out.gravity, base);
        assert!(out.active_ids.is_empty());
    }

    #[test]
    fn region_low_g_swaps_magnitude_inside_box() {
        let base = GravityVec::earth_default();
        let overrides = vec![GravityOverride::RegionLowG {
            id: 7,
            min: [0.0, 0.0],
            max: [100.0, 100.0],
            local_g: 4.9,
        }];
        let inside = apply_overrides(base, [50.0, 50.0], Some(1), &overrides);
        assert!((inside.gravity.magnitude - 4.9).abs() < 1e-6);
        assert!(inside.active_ids.contains(&7));
        let outside = apply_overrides(base, [200.0, 50.0], Some(1), &overrides);
        assert!((outside.gravity.magnitude - 9.81).abs() < 1e-6);
        assert!(outside.active_ids.is_empty());
    }

    #[test]
    fn uniform_well_bends_direction_toward_center() {
        let base = GravityVec::earth_default();
        let overrides = vec![GravityOverride::UniformWell {
            id: 1,
            center: [200.0, 100.0],
            radius: 50.0,
            magnitude: 25.0,
        }];
        // Position 10 px to the right of the well center; gravity should
        // gain a leftward (negative x) component.
        let out = apply_overrides(base, [210.0, 100.0], Some(1), &overrides);
        assert!(out.gravity.direction[0] < 0.0, "expected leftward bend: {:?}", out.gravity);
        assert!(out.active_ids.contains(&1));
    }

    #[test]
    fn magnetic_boots_cancel_overrides_for_anchored_actor() {
        let base = GravityVec::new(9.81, [0.0, -1.0]);
        let overrides = vec![
            GravityOverride::RegionLowG {
                id: 1,
                min: [0.0, 0.0],
                max: [100.0, 100.0],
                local_g: 4.9,
            },
            GravityOverride::MagneticBoots { id: 2, actor_id: 99 },
        ];
        // Anchored actor: back to baseline.
        let anchored = apply_overrides(base, [50.0, 50.0], Some(99), &overrides);
        assert!((anchored.gravity.magnitude - 9.81).abs() < 1e-6);
        assert!(anchored.active_ids.contains(&2));
        // Unanchored actor: still gets low-g.
        let unanchored = apply_overrides(base, [50.0, 50.0], Some(1), &overrides);
        assert!((unanchored.gravity.magnitude - 4.9).abs() < 1e-6);
        assert!(!unanchored.active_ids.contains(&2));
    }

    #[test]
    fn reverse_g_flips_direction_inside_region() {
        let base = GravityVec::earth_default();
        let overrides = vec![GravityOverride::ReverseG {
            id: 5,
            min: [0.0, 0.0],
            max: [100.0, 100.0],
        }];
        let inside = apply_overrides(base, [50.0, 50.0], Some(1), &overrides);
        assert!(inside.gravity.direction[0].abs() < 1e-6);
        assert!((inside.gravity.direction[1] - 1.0).abs() < 1e-6);
        assert!(inside.active_ids.contains(&5));
    }

    #[test]
    fn damaged_grav_only_inside_wave_front() {
        let base = GravityVec::earth_default();
        let overrides = vec![GravityOverride::DamagedGrav {
            id: 9,
            center: [0.0, 0.0],
            radius: 100.0,
            magnitude_factor: 0.5,
            wave_front_radius: 20.0,
        }];
        // Inside wave front — halved magnitude.
        let inside = apply_overrides(base, [10.0, 0.0], Some(1), &overrides);
        assert!((inside.gravity.magnitude - 9.81 * 0.5).abs() < 1e-3);
        // Outside wave front but inside radius — no change yet.
        let outside_front = apply_overrides(base, [50.0, 0.0], Some(1), &overrides);
        assert!((outside_front.gravity.magnitude - 9.81).abs() < 1e-3);
    }

    #[test]
    fn override_stacking_is_deterministic() {
        let base = GravityVec::earth_default();
        let overrides = vec![
            GravityOverride::RegionLowG {
                id: 1,
                min: [0.0, 0.0],
                max: [100.0, 100.0],
                local_g: 4.9,
            },
            GravityOverride::UniformWell {
                id: 2,
                center: [50.0, 50.0],
                radius: 30.0,
                magnitude: 5.0,
            },
        ];
        let a = apply_overrides(base, [50.0, 60.0], Some(1), &overrides);
        let b = apply_overrides(base, [50.0, 60.0], Some(1), &overrides);
        assert_eq!(a, b);
    }
}
