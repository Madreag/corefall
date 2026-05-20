//! **M15** § Per-cell air-pressure field.
//!
//! Per the M15 spec § "Air / heat / gravity fields (per Powder Toy)":
//! - Per-cell air pressure (256×256 grid; updated per tick)
//! - Per-cell heat field
//! - Per-cell gravity field (allows gravity anomalies M16+)
//! - Fields drive material movement + reactions
//!
//! This module owns the **air-pressure** half. Heat lives in
//! [`crate::heat`]; gravity anomalies live downstream (M16 hazards).
//!
//! ## Grid layout
//!
//! The pressure grid is `AIR_GRID_SIZE × AIR_GRID_SIZE` cells (256×256
//! per spec). One cell maps to a `cell_size_px` square in world space;
//! the engine sets `cell_size_px = 16` so a 256×256 air grid covers
//! 4096×4096 world pixels — enough for the BP4 scenarios. Scenarios
//! with larger maps subdivide the field per chunk.
//!
//! ## Pressure semantics
//!
//! `ambient` is the equilibrium pressure (101.325 kPa at sea-level
//! Earth analog). Per-cell pressure is stored as a delta above ambient:
//! `pressure_kpa[cell] = ambient + delta`. Explosions / breaches drive
//! the delta up/down; the diffusion step ([`AirField::equalize`])
//! smooths neighbors back toward ambient.

use serde::{Deserialize, Serialize};

/// Edge length of the per-cell pressure grid in cells. Locked at 256
/// per the M15 spec § "Per-cell air pressure (256×256 grid)".
pub const AIR_GRID_SIZE: u32 = 256;

/// Total cell count.
pub const AIR_GRID_CELLS: usize = (AIR_GRID_SIZE as usize) * (AIR_GRID_SIZE as usize);

/// Earth-equivalent ambient pressure in kPa. Per ONI / Stationeers
/// reference. Mods can override per scenario.
pub const AMBIENT_PRESSURE_KPA: f32 = 101.325;

/// Per-cell air pressure field. The grid is row-major: `pressure[y *
/// AIR_GRID_SIZE + x]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirField {
    pub ambient_kpa: f32,
    /// Pressure delta above ambient per cell. Zero = ambient.
    pub delta_kpa: Vec<f32>,
    /// World-space anchor (px) of cell `(0, 0)`. The cell at world
    /// `(world_x, world_y)` is `((world_x - anchor[0]) / cell_size_px,
    /// (world_y - anchor[1]) / cell_size_px)`.
    pub anchor: [f32; 2],
    pub cell_size_px: f32,
}

impl Default for AirField {
    fn default() -> Self {
        Self::new(AMBIENT_PRESSURE_KPA, [0.0, 0.0], 16.0)
    }
}

impl AirField {
    pub fn new(ambient_kpa: f32, anchor: [f32; 2], cell_size_px: f32) -> Self {
        Self {
            ambient_kpa,
            delta_kpa: vec![0.0; AIR_GRID_CELLS],
            anchor,
            cell_size_px: cell_size_px.max(1.0),
        }
    }

    pub fn cell_index(cx: u32, cy: u32) -> usize {
        debug_assert!(cx < AIR_GRID_SIZE && cy < AIR_GRID_SIZE);
        (cy as usize) * (AIR_GRID_SIZE as usize) + (cx as usize)
    }

    /// Resolve world-space `(x, y)` to a `(cx, cy)` grid cell. Returns
    /// `None` for points outside the grid.
    pub fn world_to_cell(&self, world_x: f32, world_y: f32) -> Option<(u32, u32)> {
        let lx = ((world_x - self.anchor[0]) / self.cell_size_px).floor();
        let ly = ((world_y - self.anchor[1]) / self.cell_size_px).floor();
        if lx < 0.0 || ly < 0.0 {
            return None;
        }
        let cx = lx as u32;
        let cy = ly as u32;
        if cx >= AIR_GRID_SIZE || cy >= AIR_GRID_SIZE {
            return None;
        }
        Some((cx, cy))
    }

    /// Read the pressure at cell `(cx, cy)` in kPa. Returns ambient for
    /// out-of-bounds.
    pub fn pressure_at_cell(&self, cx: u32, cy: u32) -> f32 {
        if cx >= AIR_GRID_SIZE || cy >= AIR_GRID_SIZE {
            return self.ambient_kpa;
        }
        self.ambient_kpa + self.delta_kpa[Self::cell_index(cx, cy)]
    }

    /// Read the pressure at world-space `(x, y)`. Returns ambient outside
    /// the grid.
    pub fn pressure_at_world(&self, world_x: f32, world_y: f32) -> f32 {
        match self.world_to_cell(world_x, world_y) {
            Some((cx, cy)) => self.pressure_at_cell(cx, cy),
            None => self.ambient_kpa,
        }
    }

    /// Add `delta` kPa to the cell at `(cx, cy)`. Used by explosion +
    /// breach drivers. Clamps to ±1000 kPa to keep the grid finite.
    pub fn add_pressure(&mut self, cx: u32, cy: u32, delta: f32) {
        if cx >= AIR_GRID_SIZE || cy >= AIR_GRID_SIZE {
            return;
        }
        let idx = Self::cell_index(cx, cy);
        let next = (self.delta_kpa[idx] + delta).clamp(-1000.0, 1000.0);
        self.delta_kpa[idx] = next;
    }

    /// Add `delta` kPa to the cell containing world-space `(x, y)`.
    pub fn add_pressure_at_world(&mut self, world_x: f32, world_y: f32, delta: f32) {
        if let Some((cx, cy)) = self.world_to_cell(world_x, world_y) {
            self.add_pressure(cx, cy, delta);
        }
    }

    /// Apply pressure to every cell whose world-space center is within
    /// `radius_px` of `(world_x, world_y)`. Used by explosions + flask
    /// pressure spikes.
    pub fn add_pressure_radial(&mut self, world_x: f32, world_y: f32, radius_px: f32, peak_delta_kpa: f32) {
        let r = radius_px.max(self.cell_size_px);
        let r2 = r * r;
        let cx_min = (((world_x - r - self.anchor[0]) / self.cell_size_px).floor()).max(0.0) as i64;
        let cy_min = (((world_y - r - self.anchor[1]) / self.cell_size_px).floor()).max(0.0) as i64;
        let cx_max = (((world_x + r - self.anchor[0]) / self.cell_size_px).ceil()).max(0.0) as i64;
        let cy_max = (((world_y + r - self.anchor[1]) / self.cell_size_px).ceil()).max(0.0) as i64;
        for cy in cy_min..=cy_max {
            for cx in cx_min..=cx_max {
                if cx < 0 || cy < 0 || cx >= (AIR_GRID_SIZE as i64) || cy >= (AIR_GRID_SIZE as i64) {
                    continue;
                }
                let cell_world_x = self.anchor[0] + (cx as f32 + 0.5) * self.cell_size_px;
                let cell_world_y = self.anchor[1] + (cy as f32 + 0.5) * self.cell_size_px;
                let dx = cell_world_x - world_x;
                let dy = cell_world_y - world_y;
                let dist2 = dx * dx + dy * dy;
                if dist2 > r2 {
                    continue;
                }
                let fall = 1.0 - (dist2 / r2);
                self.add_pressure(cx as u32, cy as u32, peak_delta_kpa * fall);
            }
        }
    }

    /// Diffuse pressure toward ambient. One pass = neighbor 4-cell
    /// average × `mix_ratio` + (1 - `mix_ratio`) × self. Cells with no
    /// neighbors (corners) just decay by `mix_ratio` toward zero delta.
    ///
    /// Per spec § "When breach opens to vacuum: Then pressure equalizes
    /// via aperture flow" — the diffusion is the canonical aperture
    /// flow path. `mix_ratio = 0.10` is the tuned default for one CA
    /// tick at 60 Hz; mods can pass smaller values for slower decay.
    pub fn equalize(&mut self, mix_ratio: f32) {
        let mix = mix_ratio.clamp(0.0, 1.0);
        if mix == 0.0 {
            return;
        }
        let mut next = self.delta_kpa.clone();
        for cy in 0..AIR_GRID_SIZE {
            for cx in 0..AIR_GRID_SIZE {
                let idx = Self::cell_index(cx, cy);
                let mut sum = 0.0;
                let mut count = 0;
                if cx > 0 {
                    sum += self.delta_kpa[Self::cell_index(cx - 1, cy)];
                    count += 1;
                }
                if cx + 1 < AIR_GRID_SIZE {
                    sum += self.delta_kpa[Self::cell_index(cx + 1, cy)];
                    count += 1;
                }
                if cy > 0 {
                    sum += self.delta_kpa[Self::cell_index(cx, cy - 1)];
                    count += 1;
                }
                if cy + 1 < AIR_GRID_SIZE {
                    sum += self.delta_kpa[Self::cell_index(cx, cy + 1)];
                    count += 1;
                }
                let mean = if count > 0 { sum / (count as f32) } else { 0.0 };
                next[idx] = self.delta_kpa[idx] * (1.0 - mix) + mean * mix;
            }
        }
        self.delta_kpa = next;
    }

    /// Reset every cell to ambient.
    pub fn clear(&mut self) {
        for d in &mut self.delta_kpa {
            *d = 0.0;
        }
    }

    /// True if every cell sits at ambient (within `epsilon_kpa`).
    pub fn is_at_ambient(&self, epsilon_kpa: f32) -> bool {
        self.delta_kpa.iter().all(|d| d.abs() <= epsilon_kpa)
    }

    /// Determinism feed: sum of absolute deltas. Used as a checksum
    /// proxy — when the field changes, this number changes.
    pub fn total_abs_delta(&self) -> f32 {
        self.delta_kpa.iter().map(|d| d.abs()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15-air-001: default field is ambient everywhere.
    #[test]
    fn default_field_is_ambient() {
        let f = AirField::default();
        assert!((f.pressure_at_cell(0, 0) - AMBIENT_PRESSURE_KPA).abs() < 1e-3);
        assert!(f.is_at_ambient(1e-3));
    }

    /// VAL-M15-air-002: adding pressure raises the affected cell.
    #[test]
    fn add_pressure_raises_cell() {
        let mut f = AirField::default();
        f.add_pressure(10, 10, 25.0);
        assert!((f.pressure_at_cell(10, 10) - (AMBIENT_PRESSURE_KPA + 25.0)).abs() < 1e-3);
        assert!((f.pressure_at_cell(11, 10) - AMBIENT_PRESSURE_KPA).abs() < 1e-3);
    }

    /// VAL-M15-air-003: radial pressure add covers a circle.
    #[test]
    fn radial_pressure_lights_neighboring_cells() {
        let mut f = AirField::default();
        // cell_size=16; world (256, 256) is cell (16, 16).
        f.add_pressure_radial(256.0, 256.0, 32.0, 50.0);
        assert!(f.pressure_at_cell(16, 16) > AMBIENT_PRESSURE_KPA + 5.0);
        // Edge cell (17, 16) sits ~16 px away → still inside r=32.
        assert!(f.pressure_at_cell(17, 16) > AMBIENT_PRESSURE_KPA);
    }

    /// VAL-M15-air-004: diffusion pushes pressure toward ambient.
    #[test]
    fn diffusion_decays_local_spike() {
        let mut f = AirField::default();
        f.add_pressure(10, 10, 100.0);
        let before = f.pressure_at_cell(10, 10);
        for _ in 0..50 {
            f.equalize(0.10);
        }
        let after = f.pressure_at_cell(10, 10);
        assert!(after < before, "diffusion lowered the peak");
        // Adjacent cell got some of the pressure.
        assert!(f.pressure_at_cell(11, 10) > AMBIENT_PRESSURE_KPA);
    }

    /// VAL-M15-air-005: per spec scenario "sealed room with explosion
    /// event Then air pressure builds in the room". We emulate that
    /// here: emit explosion pressure into a region; pressure rises.
    #[test]
    fn explosion_raises_room_pressure() {
        let mut f = AirField::default();
        f.add_pressure_radial(100.0, 100.0, 48.0, 80.0);
        let inside = f.pressure_at_world(100.0, 100.0);
        let outside = f.pressure_at_world(1000.0, 1000.0);
        assert!(inside > outside, "room pressure > ambient outside");
    }

    /// VAL-M15-air-006: per spec scenario "When breach opens to vacuum
    /// Then pressure equalizes via aperture flow" — repeated diffusion
    /// reduces a sealed spike toward ambient.
    #[test]
    fn breach_equalization_returns_toward_ambient() {
        let mut f = AirField::default();
        f.add_pressure(64, 64, 200.0);
        let before_total = f.total_abs_delta();
        for _ in 0..200 {
            f.equalize(0.10);
        }
        let after_total = f.total_abs_delta();
        // Diffusion conserves mass; the total delta stays > 0 but the
        // peak smooths down. We check that the *peak* cell has decayed.
        assert!(after_total <= before_total);
        assert!(f.pressure_at_cell(64, 64) < AMBIENT_PRESSURE_KPA + 200.0);
    }

    /// VAL-M15-air-007: world_to_cell handles the anchor offset.
    #[test]
    fn world_to_cell_respects_anchor() {
        let f = AirField::new(101.325, [128.0, 128.0], 16.0);
        let (cx, cy) = f.world_to_cell(128.0, 128.0).expect("inside");
        assert_eq!((cx, cy), (0, 0));
        let (cx, cy) = f.world_to_cell(144.0, 144.0).expect("inside");
        assert_eq!((cx, cy), (1, 1));
        assert!(f.world_to_cell(127.0, 127.0).is_none());
    }

    /// VAL-M15-air-008: round-trip via serde.
    #[test]
    fn field_serializes() {
        let mut f = AirField::default();
        f.add_pressure(5, 5, 10.0);
        let json = serde_json::to_string(&f).expect("ser");
        let back: AirField = serde_json::from_str(&json).expect("de");
        assert!((back.pressure_at_cell(5, 5) - f.pressure_at_cell(5, 5)).abs() < 1e-3);
    }
}
