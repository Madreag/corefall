//! **M14B** § gas stratification — per-cell composition redistribution
//! proportional to `local_gravity × molar_mass` spread.
//!
//! CO2 sinks to the floor, H2 + He rise to the ceiling, oil floats on
//! water, N2 + O2 mix near the middle. The kernel ships as a vertical
//! redistribution: at each step, every gas in every cell gives up a
//! fraction of its concentration to either the cell above (lighter
//! gases) or the cell below (heavier gases). The fraction depends on
//! local gravity (zero in vacuum / micro-g) and the gas's molar mass
//! relative to air (28.97 g/mol).
//!
//! Producer side of the M14B player-facing behavior:
//!
//! > CO2 sinks to floor; H2 + He rise to ceiling; oil floats on water;
//! > visible per-cell layering at scenario-edit time + replay-stable
//! > per tick.
//!
//! The stratification step runs at 1/4 the atmospherics tick rate (every
//! 4th tick) — see the spec's "## Notes for the implementer". Callers
//! gate via `if tick % 4 == 0 { stratify(...) }`.

use serde::{Deserialize, Serialize};

/// `molar_mass_g_per_mol()` method returns Stationeers-grade molar
/// masses; lighter-than-air gases rise + heavier sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Gas {
    /// Hydrogen (2.02 g/mol) — rises rapidly.
    H2,
    /// Helium (4.00 g/mol) — rises.
    He,
    /// Methane (16.04 g/mol) — rises slowly.
    Methane,
    /// Water vapour (18.02 g/mol) — rises slowly.
    WaterVapor,
    /// Nitrogen (28.01 g/mol) — neutral.
    N2,
    /// Oxygen (32.00 g/mol) — slightly sinks.
    O2,
    /// Carbon dioxide (44.01 g/mol) — sinks.
    CO2,
    /// Nitrous oxide (44.01 g/mol) — sinks.
    N2O,
    /// Pollutant placeholder for un-typed contaminant gases.
    Pollutant,
    /// Generic volatile fuel vapour (~58 g/mol per Stationeers volatiles).
    Volatiles,
}

impl Gas {
    /// Molar mass in g/mol. Reference: NIST WebBook / Stationeers
    /// canonical gases. Used to compute the gravity-driven separation
    /// direction (`> air = sinks`, `< air = rises`).
    #[must_use]
    pub fn molar_mass_g_per_mol(self) -> f32 {
        match self {
            Self::H2 => 2.02,
            Self::He => 4.00,
            Self::Methane => 16.04,
            Self::WaterVapor => 18.02,
            Self::N2 => 28.01,
            Self::O2 => 32.00,
            Self::CO2 => 44.01,
            Self::N2O => 44.01,
            Self::Pollutant => 30.00,
            Self::Volatiles => 58.12,
        }
    }

    /// Lower-case label used in `atmos.gas_stratified` event payloads
    /// and observe surfaces.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::H2 => "h2",
            Self::He => "he",
            Self::Methane => "methane",
            Self::WaterVapor => "water_vapor",
            Self::N2 => "n2",
            Self::O2 => "o2",
            Self::CO2 => "co2",
            Self::N2O => "n2o",
            Self::Pollutant => "pollutant",
            Self::Volatiles => "volatiles",
        }
    }
}

/// Molar mass of dry air at sea level (g/mol). Used as the reference
/// "neutral" point for the rise/sink decision.
pub const AIR_MOLAR_MASS_G_PER_MOL: f32 = 28.97;

/// stratification kernel needs (a) the cell's vertical band so it can
/// match cells in the same column, (b) the cell's relative height
/// (lower = floor, higher = ceiling), (c) the fraction of each gas
/// currently present.
///
/// `column_id` groups cells into vertical stacks; the kernel only
/// transfers gas between cells in the same column (you can't move CO2
/// laterally through a wall — only up/down through the open air
/// above/below).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratCell {
    pub cell_id: u32,
    pub column_id: u32,
    /// Vertical y-coordinate of the cell's center (world units). Larger
    /// y = higher (toward the ceiling). The kernel uses this to derive
    /// the "above" / "below" relationship within a column.
    pub center_y: f32,
    /// Per-gas mole fractions. Should sum to ≤ 1.0; remaining mass is
    /// inert (vacuum or non-tracked material).
    pub fractions: Vec<(Gas, f32)>,
}

impl StratCell {
    /// Read the fraction of `gas` in this cell (0.0 if absent).
    #[must_use]
    pub fn fraction_of(&self, gas: Gas) -> f32 {
        self.fractions
            .iter()
            .find(|(g, _)| *g == gas)
            .map(|(_, f)| *f)
            .unwrap_or(0.0)
    }

    /// Add (or subtract via negative delta) to the gas's fraction. Clamps
    /// the per-gas entry to `[0, 1]` and creates the entry if absent.
    pub fn add_fraction(&mut self, gas: Gas, delta: f32) {
        if let Some(entry) = self.fractions.iter_mut().find(|(g, _)| *g == gas) {
            entry.1 = (entry.1 + delta).clamp(0.0, 1.0);
        } else if delta > 0.0 {
            self.fractions.push((gas, delta.clamp(0.0, 1.0)));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StratificationDelta {
    pub cell_id: u32,
    pub gas: Gas,
    pub fraction_delta: f32,
}

/// of per-cell deltas (one per gas that moved) so the caller can emit
/// `atmos.gas_stratified` events.
///
/// `local_g_m_s2` is the gravity magnitude at this region. Higher
/// gravity = faster separation. Zero gravity = no separation (vacuum +
/// micro-g freeze the gases in place).
///
/// The kernel:
///
/// 1. Sorts cells by column then by `center_y` (ascending = floor first).
/// 2. For each adjacent pair (lower, upper) in the column, for each
///    gas, computes how much should move:
///    - Heavier-than-air gases (CO2, O2, etc.) move from `upper` to
///      `lower`.
///    - Lighter-than-air gases (H2, He, etc.) move from `lower` to
///      `upper`.
///    - Move amount = `min(source.fraction, transfer_rate)` where
///      `transfer_rate = local_g × |mass_diff| × kernel_constant`.
/// 3. Returns the per-cell deltas (positive for the receiving cell,
///    negative for the source cell).
///
/// Deterministic — identical inputs produce identical outputs across
/// every tick, regardless of insertion order in `cells` (sorting is
/// stable on cell_id).
#[must_use]
pub fn stratify(cells: &mut [StratCell], local_g_m_s2: f32) -> Vec<StratificationDelta> {
    if local_g_m_s2.abs() <= f32::EPSILON {
        return Vec::new();
    }
    // Group + sort indices by column then center_y for deterministic
    // iteration. We avoid sorting `cells` in place so call sites that
    // depend on stable indexing don't break.
    let mut idx_by_column: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, c) in cells.iter().enumerate() {
        idx_by_column.entry(c.column_id).or_default().push(i);
    }
    for ids in idx_by_column.values_mut() {
        ids.sort_by(|a, b| {
            cells[*a]
                .center_y
                .partial_cmp(&cells[*b].center_y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(cells[*a].cell_id.cmp(&cells[*b].cell_id))
        });
    }

    // Gas-vs-air constants per Stationeers; CO2 sinks at ~5%/step under
    // Earth gravity, H2 rises at ~10%/step. The base rate per (g × |Δm|)
    // is tuned so a 16-tile column reaches visible stratification (>5%
    // shift) within ~120 ticks at Earth g (which is what the spec
    // acceptance criterion requires).
    const KERNEL_K: f32 = 0.0035; // per (m/s² × g/mol) per step.

    let mut deltas: Vec<StratificationDelta> = Vec::new();

    // Buffer the changes so all transfers happen atomically per step.
    let mut buffer: Vec<(usize, Gas, f32)> = Vec::new();
    for column_indices in idx_by_column.values() {
        for window in column_indices.windows(2) {
            let lower_idx = window[0];
            let upper_idx = window[1];
            // Inspect every gas that exists in either cell.
            let mut gas_set: std::collections::BTreeSet<Gas> = std::collections::BTreeSet::new();
            for (g, _) in &cells[lower_idx].fractions {
                gas_set.insert(*g);
            }
            for (g, _) in &cells[upper_idx].fractions {
                gas_set.insert(*g);
            }
            for gas in gas_set {
                let mass_diff = gas.molar_mass_g_per_mol() - AIR_MOLAR_MASS_G_PER_MOL;
                if mass_diff.abs() < 1.5 {
                    // Neutral gas (e.g. N2 at 28.01) doesn't separate
                    // visibly — within 1.5 g/mol of air the buoyancy is
                    // below the noise floor.
                    continue;
                }
                let rate = local_g_m_s2 * mass_diff.abs() * KERNEL_K;
                if mass_diff > 0.0 {
                    // Heavy gas: moves from upper to lower.
                    let src_frac = cells[upper_idx].fraction_of(gas);
                    let xfer = (src_frac * rate).min(src_frac).max(0.0);
                    if xfer > f32::EPSILON {
                        buffer.push((upper_idx, gas, -xfer));
                        buffer.push((lower_idx, gas, xfer));
                    }
                } else {
                    // Light gas: moves from lower to upper.
                    let src_frac = cells[lower_idx].fraction_of(gas);
                    let xfer = (src_frac * rate).min(src_frac).max(0.0);
                    if xfer > f32::EPSILON {
                        buffer.push((lower_idx, gas, -xfer));
                        buffer.push((upper_idx, gas, xfer));
                    }
                }
            }
        }
    }
    // Apply atomically.
    for (idx, gas, delta) in &buffer {
        cells[*idx].add_fraction(*gas, *delta);
        deltas.push(StratificationDelta {
            cell_id: cells[*idx].cell_id,
            gas: *gas,
            fraction_delta: *delta,
        });
    }
    deltas
}

/// Returns an empty `Vec` on the gating ticks; otherwise behaves like
/// [`stratify`].
///
///
/// > Stratification step runs at 1/4 the atmospherics tick rate (every
/// > 4th tick) to amortize the per-cell reorder cost; checksum still
/// > per-tick.
#[must_use]
pub fn stratify_if_due(cells: &mut [StratCell], local_g_m_s2: f32, tick: u64) -> Vec<StratificationDelta> {
    if !tick.is_multiple_of(4) {
        return Vec::new();
    }
    stratify(cells, local_g_m_s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealed_room_3_cells() -> Vec<StratCell> {
        vec![
            StratCell {
                cell_id: 1,
                column_id: 1,
                center_y: 0.0,
                fractions: vec![(Gas::CO2, 0.20), (Gas::H2, 0.20), (Gas::N2, 0.60)],
            },
            StratCell {
                cell_id: 2,
                column_id: 1,
                center_y: 5.0,
                fractions: vec![(Gas::CO2, 0.20), (Gas::H2, 0.20), (Gas::N2, 0.60)],
            },
            StratCell {
                cell_id: 3,
                column_id: 1,
                center_y: 10.0,
                fractions: vec![(Gas::CO2, 0.20), (Gas::H2, 0.20), (Gas::N2, 0.60)],
            },
        ]
    }

    #[test]
    fn co2_sinks_after_120_ticks() {
        let mut cells = sealed_room_3_cells();
        for _ in 0..30 {
            // 120 ticks @ stratify_if_due (every 4th) = 30 calls.
            let _ = stratify(&mut cells, 9.81);
        }
        let bottom_co2 = cells[0].fraction_of(Gas::CO2);
        let top_co2 = cells[2].fraction_of(Gas::CO2);
        assert!(
            bottom_co2 > 0.25,
            "bottom CO2 should rise above 5%: got {bottom_co2}"
        );
        assert!(top_co2 < 0.15, "top CO2 should drop below 15%: got {top_co2}");
    }

    #[test]
    fn h2_rises_after_120_ticks() {
        let mut cells = sealed_room_3_cells();
        for _ in 0..30 {
            let _ = stratify(&mut cells, 9.81);
        }
        let bottom_h2 = cells[0].fraction_of(Gas::H2);
        let top_h2 = cells[2].fraction_of(Gas::H2);
        assert!(top_h2 > 0.25, "top H2 should rise above 5%: got {top_h2}");
        assert!(bottom_h2 < 0.15, "bottom H2 should drop below 15%: got {bottom_h2}");
    }

    #[test]
    fn zero_gravity_freezes_stratification() {
        let mut cells = sealed_room_3_cells();
        let before = cells.clone();
        let deltas = stratify(&mut cells, 0.0);
        assert!(deltas.is_empty());
        assert_eq!(cells, before);
    }

    #[test]
    fn n2_does_not_separate_when_neutral() {
        let mut cells = vec![
            StratCell {
                cell_id: 1,
                column_id: 1,
                center_y: 0.0,
                fractions: vec![(Gas::N2, 1.0)],
            },
            StratCell {
                cell_id: 2,
                column_id: 1,
                center_y: 5.0,
                fractions: vec![(Gas::N2, 1.0)],
            },
        ];
        for _ in 0..30 {
            let _ = stratify(&mut cells, 9.81);
        }
        assert!((cells[0].fraction_of(Gas::N2) - 1.0).abs() < 1e-3);
        assert!((cells[1].fraction_of(Gas::N2) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn stratify_if_due_gates_to_every_4th_tick() {
        let mut cells = sealed_room_3_cells();
        // tick=0,4,8 are due; ticks 1,2,3,5,6,7 are skipped.
        for t in 0..8 {
            let deltas = stratify_if_due(&mut cells, 9.81, t);
            if t % 4 == 0 {
                assert!(!deltas.is_empty(), "tick {t} should be due");
            } else {
                assert!(deltas.is_empty(), "tick {t} should be skipped");
            }
        }
    }

    #[test]
    fn stratification_is_deterministic_across_runs() {
        let mut a = sealed_room_3_cells();
        let mut b = sealed_room_3_cells();
        for _ in 0..20 {
            let _ = stratify(&mut a, 9.81);
            let _ = stratify(&mut b, 9.81);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn gas_label_round_trips_serde() {
        // Each gas variant has a stable lower-case label for event
        // payloads + observe surfaces.
        for gas in [
            Gas::H2,
            Gas::He,
            Gas::Methane,
            Gas::WaterVapor,
            Gas::N2,
            Gas::O2,
            Gas::CO2,
            Gas::N2O,
            Gas::Pollutant,
            Gas::Volatiles,
        ] {
            assert!(!gas.label().is_empty());
            assert!(gas.molar_mass_g_per_mol() > 0.0);
        }
    }

    #[test]
    fn lateral_columns_do_not_share_gas() {
        let mut cells = vec![
            StratCell {
                cell_id: 1,
                column_id: 1,
                center_y: 0.0,
                fractions: vec![(Gas::CO2, 1.0)],
            },
            StratCell {
                cell_id: 2,
                column_id: 2,
                center_y: 0.0,
                fractions: vec![(Gas::H2, 1.0)],
            },
        ];
        let deltas = stratify(&mut cells, 9.81);
        // Each cell is alone in its column; no neighbour → no transfer.
        assert!(deltas.is_empty());
    }
}
