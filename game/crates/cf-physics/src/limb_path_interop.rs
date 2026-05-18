//! **M14A** § "AtomGroup::push_as_limb" — narrow shim type so
//! `cf_physics::atom_group::push_as_limb` can advance a limb path without
//! depending on `cf-actor`. The full LimbPath lives in `cf-actor::limb_path`;
//! callers project the fields they need into this shim before invoking.

use serde::{Deserialize, Serialize};

/// Narrow view of a limb-path for `push_as_limb`. Mutated in place; caller
/// copies the updated fields back to its owning `cf_actor::limb_path::LimbPath`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LimbPathInterop {
    /// Foot world position from the *previous* tick. Updated by `push_as_limb`.
    pub current_limb_pos: [f32; 2],
    /// Current segment endpoint in path-local coords.
    pub current_endpoint: [f32; 2],
    /// Base push force (N).
    pub push_force_base: f32,
    /// Push force escalation timer (ms).
    pub seg_timer_ms: u32,
    /// Effective speed in px/ms (calculated from per-tier × multiplier).
    pub effective_speed_px_per_ms: f32,
    /// `true` after the last segment completed.
    pub ended: bool,
    /// `true` at start of stride.
    pub at_start: bool,
}

impl LimbPathInterop {
    /// Effective push force matches `LimbPath::effective_push_force()`.
    pub fn effective_push_force(&self) -> f32 {
        self.push_force_base * (1.0 + self.seg_timer_ms as f32 / 500.0)
    }

    /// Restart at start of stride.
    pub fn restart_free(&mut self) -> bool {
        self.ended = false;
        self.at_start = true;
        self.seg_timer_ms = 0;
        true
    }

    /// Mark the path terminated.
    pub fn terminate(&mut self) {
        self.ended = true;
        self.seg_timer_ms = 0;
    }

    /// Bump segment progress by `fraction` (will not advance segment here —
    /// that's owned by the canonical LimbPath). Bumps the seg_timer for
    /// push-force escalation.
    pub fn advance(&mut self, _fraction: f32, dt_ms: u32) {
        self.at_start = false;
        self.seg_timer_ms = self.seg_timer_ms.saturating_add(dt_ms);
    }
}
