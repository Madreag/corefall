//! **M14B** § wind force producer — per-cell ΔP × aperture-area kernel that
//! emits `atmos.wind_force_applied` events for actors standing in the
//! cross-flow lane between two cells.
//!
//! The producer side of M14A PARITY-86 (wind pushes walking actor). Today
//! cf-actor's `wind_force_for_actor` reads a single `wind: [f32; 2]` field
//! on [`crate::AtmosphereSample`]; M14B's wind kernel fills that field
//! per actor from authored [`AtmosCell`] pressures + the [`WindSource`]
//! aperture index. M19's full pipe-network kernel will replace the
//! authored cells with live PV=nRT state; the actor-facing surface stays
//! unchanged.
//!
//! The kernel is pure / deterministic. Identical inputs (cells +
//! wind_sources + actor positions) → identical outputs across every
//! tick.

use serde::{Deserialize, Serialize};

use crate::EARTH_AMBIENT_KPA;

/// **M14B** § per-cell atmospheric state used by the wind producer.
/// Authored by the scenario manifest (and by M19's pipe-network kernel
/// at runtime). The wind kernel only needs pressure + a bounding rect to
/// find which cell an actor stands in; gas composition belongs to
/// [`crate::stratification`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosCell {
    pub id: u32,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub pressure_kpa: f32,
    pub temp_k: f32,
}

impl AtmosCell {
    /// True when `world_pos` lies inside the cell's bounding rectangle.
    #[must_use]
    pub fn contains(&self, world_pos: [f32; 2]) -> bool {
        world_pos[0] >= self.min[0]
            && world_pos[0] <= self.max[0]
            && world_pos[1] >= self.min[1]
            && world_pos[1] <= self.max[1]
    }

    /// Earth-ambient default — 101 kPa, 293 K, full unit rect at origin.
    #[must_use]
    pub fn earth_default(id: u32, min: [f32; 2], max: [f32; 2]) -> Self {
        Self {
            id,
            min,
            max,
            pressure_kpa: EARTH_AMBIENT_KPA,
            temp_k: 293.15,
        }
    }
}

/// **M14B** § wind source — an aperture (open door, breach, vent, pipe
/// rupture) that couples two cells. The aperture's `area_m2` × the
/// pressure differential (cell_high.pressure - cell_low.pressure)
/// produces the wind force applied to actors standing in the jet lane.
///
/// `axis` is the unit vector pointing from the high-pressure cell toward
/// the low-pressure cell; the actor receives an impulse along this axis.
/// `jet_length` is the world-space extent (px) of the jet flow lane —
/// actors outside `[origin .. origin + axis*jet_length]` are unaffected.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindSource {
    pub id: u32,
    pub origin: [f32; 2],
    pub axis: [f32; 2],
    pub aperture_area_m2: f32,
    pub cell_high_id: u32,
    pub cell_low_id: u32,
    pub jet_length: f32,
    pub jet_half_width: f32,
}

/// **M14B** § per-actor wind force outcome surfaced by [`wind_force_at`].
///
/// `force_n` is the net force vector in newtons (chassis-mass-aware
/// caller scales by mass for Δv integration). `source_aperture_id` is
/// the dominating aperture (max contribution) used for the
/// `atmos.wind_force_applied` event payload's `source_aperture_id`
/// field.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WindForceOutcome {
    pub force_n: [f32; 2],
    pub source_aperture_id: Option<u32>,
    /// Squared magnitude — surfaced for the stagger-threshold check
    /// without forcing the caller through a sqrt.
    pub magnitude_sq: f32,
}

impl WindForceOutcome {
    /// True when the wind force magnitude exceeds the stagger threshold
    /// for a light actor (80 kg baseline). Per Stationeers parity with
    /// the CCCP drag model + DR-037 §4: 60 N is "knocks a baseline
    /// soldier off balance".
    #[must_use]
    pub fn staggers_light_actor(&self) -> bool {
        self.magnitude_sq > 60.0 * 60.0
    }
}

/// **M14B** § Stationeers wind formula `F = ΔP × aperture_area × k`.
///
/// The constant `k = 1000` Pa/kPa converts kPa-differential to Pa for
/// proper N output (`F_N = ΔP_Pa × A_m² × 0.001` per Stationeers; the
/// inner constant 0.001 expresses unit reconciliation between the
/// authored kPa and the produced N). The result is signed: positive in
/// the `axis` direction when `cell_high.pressure > cell_low.pressure`,
/// reversed when the differential inverts so a closed door reopening
/// pushes both ways consistently.
#[must_use]
pub fn wind_force_from_aperture(
    cell_high_pressure_kpa: f32,
    cell_low_pressure_kpa: f32,
    aperture_area_m2: f32,
) -> f32 {
    let delta_kpa = cell_high_pressure_kpa - cell_low_pressure_kpa;
    // 1 kPa = 1000 Pa; F = ΔP × A.
    delta_kpa * 1000.0 * aperture_area_m2.max(0.0) * crate::WIND_FORCE_PER_KPA_DIFFERENTIAL / 1000.0
}

/// **M14B** § chimney effect / buoyancy lift in N for an actor at
/// `actor_pos`. Implements the spec implementer note:
///
/// > Wind impulse direction must include gravity bias (vertical wind
/// > from chimney-effect = real). Combine `wind_horizontal` from ΔP +
/// > `wind_vertical` from temperature + buoyancy.
///
/// Algorithm: find the cell containing `actor_pos`; find the cell
/// directly above (smallest center_y > actor_y in the same column id,
/// or the cell whose min.y == this cell's max.y). Compute the
/// temperature differential and apply Stationeers buoyancy
/// `F_lift = (T_below - T_above) / T_above × ρ × V × g_local`. The
/// `ρ × V` factor is folded into [`BUOYANCY_FORCE_PER_K_DELTA`] so the
/// kernel returns N directly.
///
/// Returns 0 when there's no temperature gradient or no neighbor above.
#[must_use]
pub fn buoyancy_lift_at(actor_pos: [f32; 2], cells: &[AtmosCell], local_g_m_s2: f32) -> f32 {
    let here = cells.iter().find(|c| c.contains(actor_pos));
    let Some(here) = here else {
        return 0.0;
    };
    // Find the cell directly above (lowest min.y greater than here.max.y,
    // sharing the lateral range). Used as the chimney's "cold" reference.
    let above = cells
        .iter()
        .filter(|c| c.id != here.id)
        .filter(|c| (c.min[0] - here.min[0]).abs() < 1e-3 && (c.max[0] - here.max[0]).abs() < 1e-3)
        .filter(|c| c.min[1] >= here.max[1] - 1e-3)
        .min_by(|a, b| a.min[1].partial_cmp(&b.min[1]).unwrap_or(std::cmp::Ordering::Equal));
    let Some(above) = above else {
        return 0.0;
    };
    let t_below = here.temp_k.max(1.0);
    let t_above = above.temp_k.max(1.0);
    let delta = t_below - t_above;
    if delta.abs() < 0.5 {
        return 0.0;
    }
    let g = local_g_m_s2.abs();
    if g < 1e-3 {
        // Vacuum / micro-g — no buoyancy.
        return 0.0;
    }
    // Stationeers chimney: lift in N per kelvin delta per actor proxy.
    // BUOYANCY_FORCE_PER_K_DELTA = 0.5 N/K at 1 m/s²; scales linearly
    // with local g. A 30 K differential in Earth-g (9.81) produces
    // ~15 × 9.81 ≈ 147 N — enough to feel.
    delta * BUOYANCY_FORCE_PER_K_DELTA * g
}

/// Buoyancy lift constant: N per kelvin delta per 1 m/s² of local
/// gravity. Tuned so a sealed cell 30 K warmer than its ceiling under
/// Earth gravity produces ~150 N of lift (enough to push a light actor).
pub const BUOYANCY_FORCE_PER_K_DELTA: f32 = 0.5;

/// **M14B** § sample the wind force vector at `actor_pos` from the cell
/// + aperture index. Walks every [`WindSource`] in `wind_sources`,
///   projects `actor_pos` onto the jet lane (origin + axis × jet_length,
///   half_width perpendicular), and sums the contributions.
///
/// Adds the chimney/buoyancy lift from [`buoyancy_lift_at`] to the
/// vertical force component when `local_g_m_s2 > 0` (spec implementer
/// note: "vertical wind from chimney-effect = real").
///
/// Returns the dominating aperture id (largest scalar contribution)
/// alongside the summed force; the dominating id is what the per-tick
/// `atmos.wind_force_applied` event payload reports.
#[must_use]
pub fn wind_force_at(
    actor_pos: [f32; 2],
    cells: &[AtmosCell],
    wind_sources: &[WindSource],
) -> WindForceOutcome {
    let mut fx = 0.0_f32;
    let mut fy = 0.0_f32;
    let mut best_id: Option<u32> = None;
    let mut best_contribution: f32 = 0.0;
    for ws in wind_sources {
        let high = cells.iter().find(|c| c.id == ws.cell_high_id);
        let low = cells.iter().find(|c| c.id == ws.cell_low_id);
        let (Some(high), Some(low)) = (high, low) else {
            continue;
        };
        // Normalise axis defensively.
        let ax = ws.axis[0];
        let ay = ws.axis[1];
        let len_sq = ax * ax + ay * ay;
        if len_sq <= f32::EPSILON {
            continue;
        }
        let inv_len = 1.0 / len_sq.sqrt();
        let nx = ax * inv_len;
        let ny = ay * inv_len;
        // Project actor_pos onto the lane (origin + axis * t).
        let rx = actor_pos[0] - ws.origin[0];
        let ry = actor_pos[1] - ws.origin[1];
        let along = rx * nx + ry * ny;
        let perp = (rx * ny - ry * nx).abs();
        if along < 0.0 || along > ws.jet_length.max(0.0) || perp > ws.jet_half_width.max(0.0) {
            continue;
        }
        let scalar = wind_force_from_aperture(high.pressure_kpa, low.pressure_kpa, ws.aperture_area_m2);
        if scalar.abs() <= f32::EPSILON {
            continue;
        }
        // Distance attenuation along the lane: full strength at origin,
        // 50% at jet_length.
        let attenuation = 1.0 - 0.5 * (along / ws.jet_length.max(f32::EPSILON)).clamp(0.0, 1.0);
        let force = scalar * attenuation;
        fx += nx * force;
        fy += ny * force;
        if force.abs() > best_contribution.abs() {
            best_contribution = force;
            best_id = Some(ws.id);
        }
    }
    WindForceOutcome {
        force_n: [fx, fy],
        source_aperture_id: best_id,
        magnitude_sq: fx * fx + fy * fy,
    }
}

/// **M14B** § wind force WITH chimney/buoyancy lift folded into the
/// vertical component. Combines [`wind_force_at`] (horizontal ΔP from
/// apertures + vertical from axis projection) with [`buoyancy_lift_at`]
/// (vertical lift from temperature differential between stacked cells).
///
/// Per spec implementer note: "Combine wind_horizontal from ΔP +
/// wind_vertical from temperature + buoyancy."
#[must_use]
pub fn wind_force_with_buoyancy_at(
    actor_pos: [f32; 2],
    cells: &[AtmosCell],
    wind_sources: &[WindSource],
    local_g_m_s2: f32,
) -> WindForceOutcome {
    let mut outcome = wind_force_at(actor_pos, cells, wind_sources);
    let lift = buoyancy_lift_at(actor_pos, cells, local_g_m_s2);
    if lift.abs() > 0.01 {
        outcome.force_n[1] += lift;
        outcome.magnitude_sq = outcome.force_n[0] * outcome.force_n[0] + outcome.force_n[1] * outcome.force_n[1];
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wind_force_from_aperture_zero_differential_is_zero() {
        let f = wind_force_from_aperture(101.0, 101.0, 0.5);
        assert!(f.abs() < f32::EPSILON);
    }

    #[test]
    fn wind_force_from_aperture_scales_with_pressure_delta() {
        let small = wind_force_from_aperture(102.0, 101.0, 0.5);
        let large = wind_force_from_aperture(120.0, 101.0, 0.5);
        assert!(large.abs() > small.abs());
        assert!(large > 0.0);
        // 5 kPa × 0.5 m² × 2.0 (WIND_FORCE_PER_KPA_DIFFERENTIAL constant)
        // = 5 N exactly (per the per-kPa drag constant the actor side
        // already uses).
        let exact = wind_force_from_aperture(106.0, 101.0, 0.5);
        assert!((exact - 5.0).abs() < 1e-3, "got {exact}");
    }

    #[test]
    fn wind_force_at_returns_zero_outside_jet_lane() {
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 10.0],
                pressure_kpa: 110.0,
                temp_k: 293.15,
            },
            AtmosCell {
                id: 2,
                min: [10.0, 0.0],
                max: [20.0, 10.0],
                pressure_kpa: 100.0,
                temp_k: 293.15,
            },
        ];
        let sources = vec![WindSource {
            id: 1,
            origin: [10.0, 5.0],
            axis: [1.0, 0.0],
            aperture_area_m2: 0.5,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 10.0,
            jet_half_width: 1.0,
        }];
        let out = wind_force_at([50.0, 50.0], &cells, &sources);
        assert_eq!(out.force_n, [0.0, 0.0]);
        assert!(out.source_aperture_id.is_none());
    }

    #[test]
    fn wind_force_at_pushes_actor_in_axis_direction() {
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 10.0],
                pressure_kpa: 110.0,
                temp_k: 293.15,
            },
            AtmosCell {
                id: 2,
                min: [10.0, 0.0],
                max: [20.0, 10.0],
                pressure_kpa: 100.0,
                temp_k: 293.15,
            },
        ];
        let sources = vec![WindSource {
            id: 7,
            origin: [10.0, 5.0],
            axis: [1.0, 0.0],
            aperture_area_m2: 0.5,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 10.0,
            jet_half_width: 1.0,
        }];
        let out = wind_force_at([12.0, 5.0], &cells, &sources);
        assert!(out.force_n[0] > 0.0, "force_n = {:?}", out.force_n);
        assert!(out.force_n[1].abs() < 1e-4);
        assert_eq!(out.source_aperture_id, Some(7));
    }

    #[test]
    fn wind_force_at_is_deterministic_across_ticks() {
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 10.0],
                pressure_kpa: 110.0,
                temp_k: 293.15,
            },
            AtmosCell {
                id: 2,
                min: [10.0, 0.0],
                max: [20.0, 10.0],
                pressure_kpa: 100.0,
                temp_k: 293.15,
            },
        ];
        let sources = vec![WindSource {
            id: 7,
            origin: [10.0, 5.0],
            axis: [1.0, 0.0],
            aperture_area_m2: 0.5,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 10.0,
            jet_half_width: 1.0,
        }];
        let a = wind_force_at([12.0, 5.0], &cells, &sources);
        for _ in 0..500 {
            let b = wind_force_at([12.0, 5.0], &cells, &sources);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn buoyancy_lift_pushes_up_when_lower_cell_is_warmer() {
        // Hot cell below (323 K = 50 °C), cool cell above (293 K = 20 °C).
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 5.0],
                pressure_kpa: 101.0,
                temp_k: 323.15,
            },
            AtmosCell {
                id: 2,
                min: [0.0, 5.0],
                max: [10.0, 10.0],
                pressure_kpa: 101.0,
                temp_k: 293.15,
            },
        ];
        let lift = buoyancy_lift_at([5.0, 2.0], &cells, 9.81);
        assert!(lift > 0.0, "expected upward lift, got {lift}");
        // Sanity: 30 K × 0.5 N/K × 9.81 m/s² ≈ 147 N
        assert!((lift - 147.15).abs() < 5.0, "lift {lift} not in expected range");
    }

    #[test]
    fn buoyancy_lift_zero_in_vacuum() {
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 5.0],
                pressure_kpa: 101.0,
                temp_k: 323.15,
            },
            AtmosCell {
                id: 2,
                min: [0.0, 5.0],
                max: [10.0, 10.0],
                pressure_kpa: 101.0,
                temp_k: 293.15,
            },
        ];
        // Zero g (vacuum / orbit) → no buoyancy.
        let lift = buoyancy_lift_at([5.0, 2.0], &cells, 0.0);
        assert!(lift.abs() < 1e-3);
    }

    #[test]
    fn wind_force_with_buoyancy_adds_chimney_lift() {
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [10.0, 5.0],
                pressure_kpa: 110.0,
                temp_k: 323.15,
            },
            AtmosCell {
                id: 2,
                min: [10.0, 0.0],
                max: [20.0, 5.0],
                pressure_kpa: 100.0,
                temp_k: 293.15,
            },
            AtmosCell {
                id: 3,
                min: [0.0, 5.0],
                max: [10.0, 10.0],
                pressure_kpa: 101.0,
                temp_k: 293.15,
            },
        ];
        let sources = vec![WindSource {
            id: 1,
            origin: [10.0, 2.0],
            axis: [1.0, 0.0],
            aperture_area_m2: 0.5,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 10.0,
            jet_half_width: 3.0,
        }];
        let without = wind_force_at([5.0, 2.0], &cells, &sources);
        let with = wind_force_with_buoyancy_at([5.0, 2.0], &cells, &sources, 9.81);
        // Same horizontal force; bigger vertical (buoyancy lifts upward).
        assert!((with.force_n[0] - without.force_n[0]).abs() < 1e-3);
        assert!(with.force_n[1] > without.force_n[1]);
    }

    #[test]
    fn pipe_rupture_high_pressure_jet_staggers_light_actor() {
        // 70 MPa pipe rupture into 100 kPa room. ΔP = 70 000 kPa - 100 kPa
        // ≈ 69 900 kPa. Even with a 0.01 m² aperture the resulting force
        // dwarfs the 60 N stagger threshold.
        let cells = vec![
            AtmosCell {
                id: 1,
                min: [0.0, 0.0],
                max: [4.0, 4.0],
                pressure_kpa: 70_000.0,
                temp_k: 293.15,
            },
            AtmosCell {
                id: 2,
                min: [4.0, 0.0],
                max: [16.0, 4.0],
                pressure_kpa: 100.0,
                temp_k: 293.15,
            },
        ];
        let sources = vec![WindSource {
            id: 2,
            origin: [4.0, 2.0],
            axis: [1.0, 0.0],
            aperture_area_m2: 0.01,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 12.0,
            jet_half_width: 1.0,
        }];
        let out = wind_force_at([8.0, 2.0], &cells, &sources);
        assert!(out.staggers_light_actor(), "force={:?}", out);
    }
}
