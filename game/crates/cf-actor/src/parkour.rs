//! **M14J** § "parkour — vault + wall-jump".
//!
//! Sweept-volume helpers that detect chest-high obstacles in the actor's
//! facing direction (vault candidate) and vertical surfaces at the bounding-
//! box mid-line perpendicular to current motion (wall-jump candidate).
//!
//! The helpers are pure: input is the actor's kinematic state + a single
//! "is solid at world tile?" closure provided by the caller (the M14
//! collision layer / chunked terrain). Outputs are cached on the actor's
//! [`ParkourSignal`] so the limb-path dispatch can consult them without
//! re-querying every tick.

use serde::{Deserialize, Serialize};

use crate::FacingDirection;

/// 70% of the canonical jump impulse.
pub const WALL_JUMP_PERPENDICULAR_FRACTION: f32 = 0.70;

pub const MAX_CHAINED_WALL_JUMPS: u32 = 3;

pub const WALL_CONTACT_GRACE_MS: u32 = 250;

/// for the implementer: "Tune the swept-volume forward distance to 0.4-0.5
/// m so the actor commits only when collision is inevitable." We pick 0.5 m.
pub const VAULT_FORWARD_SWEEP_M: f32 = 0.5;

/// obstacles (≤1.2 m)".
pub const VAULT_MAX_OBSTACLE_HEIGHT_M: f32 = 1.2;

pub const VAULT_DURATION_MS: u32 = 200;

/// each step".
pub const WALL_JUMP_DURATION_MS: u32 = 200;

/// [`detect_vault`] and [`detect_wall_jump`]; consumed by `walk_sim_tick`
/// when it sees the auto-vault / wall-jump trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ParkourSignal {
    /// Detected chest-high obstacle in the swept volume this tick.
    pub vault_candidate: Option<VaultCandidate>,
    /// Detected vertical surface within `WALL_CONTACT_GRACE_MS` of contact.
    pub wall_candidate: Option<WallCandidate>,
    /// Number of wall-jumps the actor has chained since last touching
    /// ground. Resets to 0 on ground contact.
    pub chained_wall_jumps_since_ground: u32,
    /// Ms remaining in the active vault cinematic. 0 when not vaulting.
    pub vault_ticks_remaining_ms: u32,
    /// Ms remaining in the active wall-jump cinematic.
    pub wall_jump_ticks_remaining_ms: u32,
    /// Ms remaining in the wall-contact grace window. While > 0 the actor
    /// can still trigger a wall-jump. Decrements every tick when airborne.
    pub wall_contact_grace_remaining_ms: u32,
}

impl ParkourSignal {
    /// Reset the wall-jump chain on ground contact. Should be called every
    /// tick the actor's `on_ground` flag is true.
    pub fn note_ground_contact(&mut self) {
        self.chained_wall_jumps_since_ground = 0;
        self.wall_contact_grace_remaining_ms = 0;
    }

    /// Advance per-tick timers by `dt_ms`.
    pub fn tick(&mut self, dt_ms: u32) {
        self.vault_ticks_remaining_ms = self.vault_ticks_remaining_ms.saturating_sub(dt_ms);
        self.wall_jump_ticks_remaining_ms = self.wall_jump_ticks_remaining_ms.saturating_sub(dt_ms);
        self.wall_contact_grace_remaining_ms = self
            .wall_contact_grace_remaining_ms
            .saturating_sub(dt_ms);
    }

    /// True when the player can issue another wall-jump (chain not yet
    /// exhausted AND grace window still open AND wall surface still in
    /// contact).
    pub fn wall_jump_available(&self) -> bool {
        self.chained_wall_jumps_since_ground < MAX_CHAINED_WALL_JUMPS
            && self.wall_contact_grace_remaining_ms > 0
            && self.wall_candidate.is_some()
    }
}

/// Detected vault candidate from one swept-volume query.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VaultCandidate {
    /// World-space x at the obstacle's near face (where the actor enters).
    pub near_x: f32,
    /// World-space top of the obstacle (Y).
    pub top_y: f32,
    /// Obstacle height in meters (≤ [`VAULT_MAX_OBSTACLE_HEIGHT_M`]).
    pub height_m: f32,
}

/// Detected wall candidate from one perpendicular swept-volume query.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WallCandidate {
    /// World-space x of the wall surface the actor is in contact with.
    pub surface_x: f32,
    /// Sign of the wall's normal in world x (`+1` = wall to the actor's
    /// right; `-1` = wall to the actor's left).
    pub normal_sign: f32,
}

/// chest height in the actor's facing direction. Returns `Some(...)` when
/// a solid chest-high obstacle is within `VAULT_FORWARD_SWEEP_M` in front
/// of the actor.
///
/// `is_solid(x, y)` is the caller's "world solid at this position?"
/// predicate (chunked-terrain query). `actor_position` is the actor's
/// world-space center; `half_extents` is its AABB; `facing` is the
/// direction the actor is looking; `velocity_x` is the horizontal speed
/// (so a near-stationary actor doesn't false-trigger vaults).
#[must_use]
pub fn detect_vault<F>(
    actor_position: [f32; 2],
    half_extents: [f32; 2],
    facing: FacingDirection,
    velocity_x: f32,
    is_solid: F,
) -> Option<VaultCandidate>
where
    F: Fn(f32, f32) -> bool,
{
    // Only trigger when the actor is actually moving forward.
    if velocity_x.abs() < 0.5 {
        return None;
    }
    let dir_x = facing.sign();
    let chest_y = actor_position[1] + half_extents[1] * 0.5;
    let foot_y = actor_position[1] - half_extents[1] * 0.5;
    let near_x = actor_position[0] + dir_x * half_extents[0];
    let far_x = near_x + dir_x * VAULT_FORWARD_SWEEP_M;
    // Sample at the obstacle face (foot height, chest height).
    let foot_solid = is_solid(far_x, foot_y + 0.05);
    let chest_solid = is_solid(far_x, chest_y);
    if !foot_solid {
        return None;
    }
    // If the chest is solid too, the obstacle is too tall to vault.
    if chest_solid {
        return None;
    }
    // Probe upward to find the top of the obstacle.
    let mut top_y = foot_y;
    let step = (chest_y - foot_y).max(0.1) / 8.0;
    for i in 1..=8 {
        let probe_y = foot_y + step * i as f32;
        if !is_solid(far_x, probe_y) {
            top_y = probe_y - step * 0.5;
            break;
        }
        top_y = probe_y;
    }
    let height_m = (top_y - foot_y).max(0.0);
    if height_m > VAULT_MAX_OBSTACLE_HEIGHT_M {
        return None;
    }
    Some(VaultCandidate {
        near_x,
        top_y,
        height_m,
    })
}

/// query at the actor's bounding-box mid-line. Returns `Some(...)` when a
/// vertical surface is in contact within 0.2 m on either side.
#[must_use]
pub fn detect_wall<F>(actor_position: [f32; 2], half_extents: [f32; 2], is_solid: F) -> Option<WallCandidate>
where
    F: Fn(f32, f32) -> bool,
{
    let mid_y = actor_position[1];
    let probe_offset = half_extents[0] + 0.2;
    let left_x = actor_position[0] - probe_offset;
    let right_x = actor_position[0] + probe_offset;
    if is_solid(right_x, mid_y) {
        return Some(WallCandidate {
            surface_x: right_x,
            normal_sign: 1.0,
        });
    }
    if is_solid(left_x, mid_y) {
        return Some(WallCandidate {
            surface_x: left_x,
            normal_sign: -1.0,
        });
    }
    None
}

/// velocity delta from a wall-jump trigger. Returns `(vx, vy)` to ADD to
/// the actor's current velocity:
///  - vx flips sign at full magnitude (reflects horizontal velocity)
///  - vy gains +70% of `jump_impulse` perpendicular to the wall (always up)
///
/// `wall_normal_sign` is the sign of the wall's normal vector in world x
/// (returned by [`detect_wall`]). Spec § Acceptance criteria literally
/// pins this formula.
#[must_use]
pub fn wall_jump_velocity_delta(current_velocity: [f32; 2], wall_normal_sign: f32, jump_impulse: f32) -> [f32; 2] {
    let vx = current_velocity[0];
    // Reflect: new vx = -vx (full magnitude). Delta = -2 * vx.
    let dvx = -2.0 * vx;
    // Vertical perpendicular kick: +70% jump_impulse upward.
    let dvy = WALL_JUMP_PERPENDICULAR_FRACTION * jump_impulse;
    // Push slightly away from the wall (the perpendicular impulse also
    // imparts a small horizontal kick away from the wall to disengage).
    let dvx = dvx - wall_normal_sign * WALL_JUMP_PERPENDICULAR_FRACTION * jump_impulse * 0.25;
    [dvx, dvy]
}

/// vault outcome to the actor's state. Returns the new world-space position
/// + velocity once the vault completes; horizontal velocity is preserved.
#[must_use]
pub fn apply_vault(actor_position: [f32; 2], candidate: &VaultCandidate, facing_sign: f32) -> [f32; 2] {
    // Translate the actor through the obstacle by the actor's bounding-box
    // width + a small safety margin so it lands on the far side rather than
    // intersecting the crate's far face.
    let dx = facing_sign * (VAULT_FORWARD_SWEEP_M + 0.4);
    let dy = (candidate.top_y - actor_position[1]).max(0.0);
    [actor_position[0] + dx, actor_position[1] + dy]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FacingDirection;

    #[test]
    fn vault_detects_chest_high_obstacle() {
        // Solid wall from x=2 to x=3, height 0 to 0.8m.
        let is_solid = |x: f32, y: f32| x >= 2.0 && x <= 3.0 && y >= 0.0 && y <= 0.8;
        let c = detect_vault(
            [1.5, 0.5],
            [0.4, 1.0],
            FacingDirection::Right,
            5.0,
            is_solid,
        );
        assert!(c.is_some(), "expected vault candidate");
        let c = c.unwrap();
        assert!(c.height_m > 0.0);
        assert!(c.height_m <= VAULT_MAX_OBSTACLE_HEIGHT_M);
    }

    #[test]
    fn vault_skipped_when_obstacle_too_tall() {
        // Solid wall from y=0 to y=3 — too tall to vault.
        let is_solid = |x: f32, y: f32| x >= 2.0 && x <= 3.0 && y >= 0.0 && y <= 3.0;
        let c = detect_vault(
            [1.5, 0.5],
            [0.4, 1.0],
            FacingDirection::Right,
            5.0,
            is_solid,
        );
        assert!(c.is_none(), "tall obstacle must not produce vault candidate");
    }

    #[test]
    fn vault_skipped_when_stationary() {
        let is_solid = |x: f32, y: f32| x >= 2.0 && x <= 3.0 && y >= 0.0 && y <= 0.8;
        let c = detect_vault(
            [1.5, 0.5],
            [0.4, 1.0],
            FacingDirection::Right,
            0.0,
            is_solid,
        );
        assert!(c.is_none());
    }

    #[test]
    fn wall_detect_right() {
        let is_solid = |x: f32, _y: f32| x >= 1.5;
        let c = detect_wall([1.2, 0.5], [0.4, 1.0], is_solid);
        assert!(c.is_some());
        let c = c.unwrap();
        assert!((c.normal_sign - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wall_detect_left() {
        let is_solid = |x: f32, _y: f32| x <= 0.8;
        let c = detect_wall([1.2, 0.5], [0.4, 1.0], is_solid);
        assert!(c.is_some());
        let c = c.unwrap();
        assert!((c.normal_sign - -1.0).abs() < 1e-6);
    }

    #[test]
    fn wall_jump_chain_resets_on_ground() {
        let mut sig = ParkourSignal::default();
        sig.chained_wall_jumps_since_ground = 2;
        sig.wall_contact_grace_remaining_ms = 100;
        sig.note_ground_contact();
        assert_eq!(sig.chained_wall_jumps_since_ground, 0);
        assert_eq!(sig.wall_contact_grace_remaining_ms, 0);
    }

    #[test]
    fn wall_jump_chain_exhausted_blocks_further_jumps() {
        let mut sig = ParkourSignal::default();
        sig.wall_contact_grace_remaining_ms = WALL_CONTACT_GRACE_MS;
        sig.wall_candidate = Some(WallCandidate {
            surface_x: 1.0,
            normal_sign: 1.0,
        });
        sig.chained_wall_jumps_since_ground = 3;
        assert!(!sig.wall_jump_available());
    }

    #[test]
    fn wall_jump_velocity_reflects_horizontal() {
        // Moving rightward at +5 hits a wall on the right. New vx should be
        // -5 (full reflection) and vy gains 70% of jump impulse.
        let dv = wall_jump_velocity_delta([5.0, 0.0], 1.0, 100.0);
        // delta_vx = -10 (reflect from +5 to -5) MINUS perpendicular push
        // (-0.25 * 0.7 * 100 * 1 = -17.5). Total dvx = -27.5.
        assert!(dv[0] < -10.0);
        // delta_vy = +70 (perpendicular kick = 0.7 * 100)
        assert!((dv[1] - 70.0).abs() < 1e-3);
    }
}
