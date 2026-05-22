//! **M14E** § Tunnel-collapse render primitives.
//!
//! Owns three things:
//! - [`CrackDecal`] — L1/L2/L3 ceiling crack tile anchored to a chunk
//!   bbox. Levels match the `terrain.structural_integrity_low` payload's
//!   `level` field (`l1` / `l2` / `l3`).
//! - [`FallingDebrisCone`] — gravity-aligned debris primitive emitted
//!   once per `terrain.cave_in_triggered`. Origin = ceiling bbox top
//!   edge; direction = +Y down; span = full chunk width.
//! - [`TunnelCollapseQueue::enqueue_cave_in`] — pushes the cone + the
//!   final L3 decal in one call.
//!
//! Bevy-free pure data. Consumed by the live + offline renderers; the
//! queue itself is `Clone + Debug + Default` so tests can snapshot it.

use serde::{Deserialize, Serialize};

/// Discriminator for the crack-decal level. Levels match the
/// `terrain.structural_integrity_low.level` payload field verbatim.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrackLevel {
    L1,
    L2,
    L3,
}

impl CrackLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            CrackLevel::L1 => "l1",
            CrackLevel::L2 => "l2",
            CrackLevel::L3 => "l3",
        }
    }

    /// Parse a crack-level discriminator from its canonical wire name.
    /// Named distinctly from `std::str::FromStr::from_str` so clippy
    /// doesn't flag the method-confusion lint.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "l1" => Some(CrackLevel::L1),
            "l2" => Some(CrackLevel::L2),
            "l3" => Some(CrackLevel::L3),
            _ => None,
        }
    }
}

/// One ceiling crack decal primitive. Anchored to a chunk bbox.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CrackDecal {
    pub chunk_id: (i32, i32),
    pub level: CrackLevel,
    /// Pixel-space anchor (centre of the top edge of the bbox).
    pub anchor: (f32, f32),
    /// Pixel-space half-width of the decal (= half the bbox width).
    pub half_width_px: f32,
}

/// One falling-debris cone primitive. Spawned per
/// `terrain.cave_in_triggered`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FallingDebrisCone {
    pub chunk_id: (i32, i32),
    /// Pixel-space origin — top edge of the ceiling bbox.
    pub origin: (f32, f32),
    /// Gravity-aligned direction unit vector. Always (0, +1) for cave-ins.
    pub direction: (f32, f32),
    /// Pixel-space span (width). Set to the chunk bbox width.
    pub span_px: f32,
    /// Number of debris pixels to emit; mirrors
    /// `terrain.cave_in_triggered.falling_debris_count`.
    pub debris_count: u32,
}

/// Render queue for the M14E primitives. Lives on the renderer side and
/// is drained per-frame.
#[derive(Debug, Clone, Default)]
pub struct TunnelCollapseQueue {
    decals: Vec<CrackDecal>,
    cones: Vec<FallingDebrisCone>,
}

impl TunnelCollapseQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a crack decal at `chunk_id` for the given `level`. The
    /// anchor + half-width are derived from the supplied bbox.
    pub fn enqueue_crack_decal(
        &mut self,
        chunk_id: (i32, i32),
        level: CrackLevel,
        bbox_min: (f32, f32),
        bbox_max: (f32, f32),
    ) {
        let half_width = ((bbox_max.0 - bbox_min.0) * 0.5).max(0.0);
        let anchor_x = (bbox_min.0 + bbox_max.0) * 0.5;
        let anchor_y = bbox_min.1;
        self.decals.push(CrackDecal {
            chunk_id,
            level,
            anchor: (anchor_x, anchor_y),
            half_width_px: half_width,
        });
    }

    /// Push the full M14E cave-in render bundle: an L3 crack decal +
    /// a gravity-aligned falling-debris cone. Per VAL-M14E-025.
    pub fn enqueue_cave_in(
        &mut self,
        chunk_id: (i32, i32),
        bbox_min: (f32, f32),
        bbox_max: (f32, f32),
        debris_count: u32,
    ) {
        self.enqueue_crack_decal(chunk_id, CrackLevel::L3, bbox_min, bbox_max);
        let span_px = (bbox_max.0 - bbox_min.0).max(0.0);
        self.cones.push(FallingDebrisCone {
            chunk_id,
            origin: ((bbox_min.0 + bbox_max.0) * 0.5, bbox_min.1),
            direction: (0.0, 1.0),
            span_px,
            debris_count,
        });
    }

    /// Drain every queued decal. Returns ownership so the renderer can
    /// upload + recycle.
    pub fn drain_decals(&mut self) -> Vec<CrackDecal> {
        std::mem::take(&mut self.decals)
    }

    /// Drain every queued cone. Returns ownership so the renderer can
    /// upload + recycle.
    pub fn drain_cones(&mut self) -> Vec<FallingDebrisCone> {
        std::mem::take(&mut self.cones)
    }

    /// Read-only access to the decal buffer.
    pub fn decals(&self) -> &[CrackDecal] {
        &self.decals
    }

    /// Read-only access to the falling-cone buffer.
    pub fn cones(&self) -> &[FallingDebrisCone] {
        &self.cones
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crack_level_round_trips_through_str() {
        for level in [CrackLevel::L1, CrackLevel::L2, CrackLevel::L3] {
            assert_eq!(CrackLevel::parse_str(level.as_str()), Some(level));
        }
        assert_eq!(CrackLevel::parse_str("garbage"), None);
    }

    #[test]
    fn enqueue_crack_decal_progresses_in_authored_order() {
        let mut q = TunnelCollapseQueue::new();
        q.enqueue_crack_decal((0, 0), CrackLevel::L1, (64.0, 60.0), (96.0, 76.0));
        q.enqueue_crack_decal((0, 0), CrackLevel::L2, (64.0, 60.0), (96.0, 76.0));
        q.enqueue_crack_decal((0, 0), CrackLevel::L3, (64.0, 60.0), (96.0, 76.0));
        let decals = q.drain_decals();
        assert_eq!(decals.len(), 3);
        assert_eq!(decals[0].level, CrackLevel::L1);
        assert_eq!(decals[1].level, CrackLevel::L2);
        assert_eq!(decals[2].level, CrackLevel::L3);
    }

    /// is at the ceiling bbox top edge + whose direction is +Y down.
    #[test]
    fn enqueue_cave_in_emits_l3_decal_and_falling_cone_aligned_down() {
        let mut q = TunnelCollapseQueue::new();
        q.enqueue_cave_in((0, 0), (64.0, 60.0), (96.0, 76.0), 96);
        let decals = q.decals();
        let cones = q.cones();
        assert_eq!(decals.len(), 1);
        assert_eq!(decals[0].level, CrackLevel::L3);
        assert_eq!(cones.len(), 1);
        assert_eq!(cones[0].chunk_id, (0, 0));
        assert_eq!(cones[0].origin, (80.0, 60.0));
        assert_eq!(cones[0].direction, (0.0, 1.0));
        assert!((cones[0].span_px - 32.0).abs() < 1e-3);
        assert_eq!(cones[0].debris_count, 96);
    }
}
