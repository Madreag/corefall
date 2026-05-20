//! **M15** § Per-cell heat field + thermal exchange.
//!
//! Per the M15 spec § "Air / heat / gravity fields (per Powder Toy)":
//! - Per-cell air pressure
//! - **Per-cell heat field** (this module)
//! - Per-cell gravity field
//! - Fields drive material movement + reactions
//!
//! The heat field stores absolute temperature in Kelvin per cell. The
//! CA reaction evaluator reads this when checking `min_temperature_k`
//! gates; phase transitions read it to decide whether a tile crossed
//! its melt/boil threshold.

use serde::{Deserialize, Serialize};

/// Edge length of the per-cell heat grid in cells. 256×256 matches
/// the [`crate::air::AIR_GRID_SIZE`] for canonical alignment.
pub const HEAT_GRID_SIZE: u32 = 256;

/// Total cell count.
pub const HEAT_GRID_CELLS: usize = (HEAT_GRID_SIZE as usize) * (HEAT_GRID_SIZE as usize);

/// Earth-equivalent ambient temperature (293.15 K = 20°C). Mods can
/// override per scenario.
pub const AMBIENT_TEMPERATURE_K_DEFAULT: f32 = 293.15;

/// Resolve the ambient temperature for a scenario. Stand-alone helper
/// so `cfctl inspect.scenario.ambient` can read it. M19 atmospherics
/// will extend with per-room overrides.
#[must_use]
pub fn ambient_temperature_k(scenario_kind: Option<&str>) -> f32 {
    match scenario_kind {
        Some("arctic") => 243.15,
        Some("desert") => 313.15,
        Some("vulcan") => 343.15,
        Some("vacuum") => 2.7,
        _ => AMBIENT_TEMPERATURE_K_DEFAULT,
    }
}

/// Per-cell heat field. Layout matches [`crate::air::AirField`] —
/// row-major `temperature_k[y * HEAT_GRID_SIZE + x]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatField {
    pub ambient_k: f32,
    pub temperature_k: Vec<f32>,
    pub anchor: [f32; 2],
    pub cell_size_px: f32,
}

impl Default for HeatField {
    fn default() -> Self {
        Self::new(AMBIENT_TEMPERATURE_K_DEFAULT, [0.0, 0.0], 16.0)
    }
}

impl HeatField {
    pub fn new(ambient_k: f32, anchor: [f32; 2], cell_size_px: f32) -> Self {
        Self {
            ambient_k,
            temperature_k: vec![ambient_k; HEAT_GRID_CELLS],
            anchor,
            cell_size_px: cell_size_px.max(1.0),
        }
    }

    pub fn cell_index(cx: u32, cy: u32) -> usize {
        debug_assert!(cx < HEAT_GRID_SIZE && cy < HEAT_GRID_SIZE);
        (cy as usize) * (HEAT_GRID_SIZE as usize) + (cx as usize)
    }

    pub fn world_to_cell(&self, world_x: f32, world_y: f32) -> Option<(u32, u32)> {
        let lx = ((world_x - self.anchor[0]) / self.cell_size_px).floor();
        let ly = ((world_y - self.anchor[1]) / self.cell_size_px).floor();
        if lx < 0.0 || ly < 0.0 {
            return None;
        }
        let cx = lx as u32;
        let cy = ly as u32;
        if cx >= HEAT_GRID_SIZE || cy >= HEAT_GRID_SIZE {
            return None;
        }
        Some((cx, cy))
    }

    pub fn temperature_at_cell(&self, cx: u32, cy: u32) -> f32 {
        if cx >= HEAT_GRID_SIZE || cy >= HEAT_GRID_SIZE {
            return self.ambient_k;
        }
        self.temperature_k[Self::cell_index(cx, cy)]
    }

    pub fn temperature_at_world(&self, world_x: f32, world_y: f32) -> f32 {
        match self.world_to_cell(world_x, world_y) {
            Some((cx, cy)) => self.temperature_at_cell(cx, cy),
            None => self.ambient_k,
        }
    }

    /// Set the cell temperature directly. Clamps to `[2.7 K, 1e5 K]` to
    /// avoid Inf/NaN drift through reactions.
    pub fn set_temperature(&mut self, cx: u32, cy: u32, t_k: f32) {
        if cx >= HEAT_GRID_SIZE || cy >= HEAT_GRID_SIZE {
            return;
        }
        let next = t_k.clamp(2.7, 1.0e5);
        self.temperature_k[Self::cell_index(cx, cy)] = next;
    }

    pub fn add_heat_at_cell(&mut self, cx: u32, cy: u32, delta_k: f32) {
        if cx >= HEAT_GRID_SIZE || cy >= HEAT_GRID_SIZE {
            return;
        }
        let idx = Self::cell_index(cx, cy);
        let next = (self.temperature_k[idx] + delta_k).clamp(2.7, 1.0e5);
        self.temperature_k[idx] = next;
    }

    pub fn add_heat_at_world(&mut self, world_x: f32, world_y: f32, delta_k: f32) {
        if let Some((cx, cy)) = self.world_to_cell(world_x, world_y) {
            self.add_heat_at_cell(cx, cy, delta_k);
        }
    }

    /// Diffuse heat toward neighbors. One pass = self × (1 - mix) +
    /// 4-neighbor mean × mix. Per spec § "Per-cell heat field +
    /// thermal exchange". `mix_ratio` 0.05 is the tuned default at
    /// 60 Hz.
    pub fn diffuse(&mut self, mix_ratio: f32) {
        let mix = mix_ratio.clamp(0.0, 1.0);
        if mix == 0.0 {
            return;
        }
        let mut next = self.temperature_k.clone();
        for cy in 0..HEAT_GRID_SIZE {
            for cx in 0..HEAT_GRID_SIZE {
                let idx = Self::cell_index(cx, cy);
                let mut sum = 0.0;
                let mut count = 0;
                if cx > 0 {
                    sum += self.temperature_k[Self::cell_index(cx - 1, cy)];
                    count += 1;
                }
                if cx + 1 < HEAT_GRID_SIZE {
                    sum += self.temperature_k[Self::cell_index(cx + 1, cy)];
                    count += 1;
                }
                if cy > 0 {
                    sum += self.temperature_k[Self::cell_index(cx, cy - 1)];
                    count += 1;
                }
                if cy + 1 < HEAT_GRID_SIZE {
                    sum += self.temperature_k[Self::cell_index(cx, cy + 1)];
                    count += 1;
                }
                let mean = if count > 0 { sum / (count as f32) } else { self.ambient_k };
                next[idx] = self.temperature_k[idx] * (1.0 - mix) + mean * mix;
            }
        }
        self.temperature_k = next;
    }

    /// Reset every cell to ambient.
    pub fn clear(&mut self) {
        for t in &mut self.temperature_k {
            *t = self.ambient_k;
        }
    }

    /// True if every cell sits within `epsilon_k` of ambient.
    pub fn is_at_ambient(&self, epsilon_k: f32) -> bool {
        self.temperature_k.iter().all(|t| (t - self.ambient_k).abs() <= epsilon_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15-heat-001: default field is ambient.
    #[test]
    fn default_heat_field_is_ambient() {
        let f = HeatField::default();
        assert!(f.is_at_ambient(1e-3));
    }

    /// VAL-M15-heat-002: ambient lookup by scenario kind.
    #[test]
    fn ambient_for_scenario_kinds() {
        assert!((ambient_temperature_k(Some("arctic")) - 243.15).abs() < 1e-3);
        assert!((ambient_temperature_k(Some("desert")) - 313.15).abs() < 1e-3);
        assert!((ambient_temperature_k(None) - AMBIENT_TEMPERATURE_K_DEFAULT).abs() < 1e-3);
    }

    /// VAL-M15-heat-003: heat injection raises the local cell.
    #[test]
    fn heat_injection_raises_local_cell() {
        let mut f = HeatField::default();
        f.add_heat_at_cell(10, 10, 200.0);
        assert!(f.temperature_at_cell(10, 10) > AMBIENT_TEMPERATURE_K_DEFAULT + 100.0);
    }

    /// VAL-M15-heat-004: diffusion spreads heat outward over time.
    #[test]
    fn diffusion_spreads_heat() {
        let mut f = HeatField::default();
        f.add_heat_at_cell(10, 10, 500.0);
        let before_neighbor = f.temperature_at_cell(11, 10);
        for _ in 0..40 {
            f.diffuse(0.10);
        }
        let after_neighbor = f.temperature_at_cell(11, 10);
        assert!(after_neighbor > before_neighbor);
    }

    /// VAL-M15-heat-005: lookup by world coords routes through anchor.
    #[test]
    fn world_lookup_routes_through_anchor() {
        let f = HeatField::new(293.15, [64.0, 64.0], 16.0);
        let t = f.temperature_at_world(64.0, 64.0);
        assert!((t - 293.15).abs() < 1e-3);
    }

    /// VAL-M15-heat-006: temperature clamps to safe range.
    #[test]
    fn temperature_clamps_to_safe_range() {
        let mut f = HeatField::default();
        f.set_temperature(0, 0, 1.0e10);
        assert!(f.temperature_at_cell(0, 0) <= 1.0e5);
        f.set_temperature(0, 0, -1000.0);
        assert!(f.temperature_at_cell(0, 0) >= 2.7);
    }

    /// VAL-M15-heat-007: serde round-trip.
    #[test]
    fn field_serializes() {
        let mut f = HeatField::default();
        f.add_heat_at_cell(5, 5, 100.0);
        let json = serde_json::to_string(&f).expect("ser");
        let back: HeatField = serde_json::from_str(&json).expect("de");
        assert!((back.temperature_at_cell(5, 5) - f.temperature_at_cell(5, 5)).abs() < 1e-3);
    }
}
