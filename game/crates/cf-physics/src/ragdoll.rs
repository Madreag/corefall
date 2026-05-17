//! **M14**: ragdoll-on-death state machine per CCCP physics-authority transition.
//!
//! When an actor enters DYING the body becomes a physical debris object:
//! gravity applies, terrain collisions resolve, and the renderer can swap
//! to a ragdoll sprite. When DEAD, the body remains physical but loses any
//! residual animation control.
//!
//! Settings.reduced_motion = true ⇒ renderer skips the ragdoll animation but
//! state still transitions (the sim is deterministic; cosmetic-only changes
//! never affect replay). The [`Ragdoll::reduced_motion_skip`] flag rides on
//! the activation event so consumers (renderer, replay viewer) can decide
//! whether to play the animation.

use serde::{Deserialize, Serialize};

/// Ragdoll lifecycle state. Mirrors the actor's high-level status but
/// tracks the *physics-authority* axis separately so the renderer can read
/// it without recomputing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RagdollState {
    /// Animation-driven. No ragdoll authority.
    #[default]
    Animated,
    /// Physics-driven; the body is a debris object but the actor is still
    /// DYING (alive-but-collapsing). Inventory has been tossed.
    Activating,
    /// Fully physics-driven. Actor is DEAD; body is a debris object only.
    Active,
    /// Renderer is told to skip the ragdoll animation (reduced motion); the
    /// sim still steps the physics state but the visual swap is skipped.
    StaticCollapse,
}



/// One ragdoll instance. Created when an actor enters DYING; the body
/// integrates gravity per [`cf_physics::step_kinematics`] each tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ragdoll {
    pub state: RagdollState,
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub mass_kg: f32,
    /// True when the activation event recorded reduced_motion=true so
    /// the renderer skipped the ragdoll animation.
    pub reduced_motion_skip: bool,
    /// Tick the activation transitioned (engine-supplied; not used in the
    /// pure helpers but reserved for replay correlation).
    pub activated_at_tick: u64,
}

impl Ragdoll {
    /// Activate a ragdoll for an actor entering DYING. `reduced_motion`
    /// controls whether the renderer skips the animation (sim state still
    /// transitions either way).
    #[must_use]
    pub fn activate(
        position: (f32, f32),
        velocity: (f32, f32),
        mass_kg: f32,
        reduced_motion: bool,
        tick: u64,
    ) -> Self {
        Self {
            state: if reduced_motion {
                RagdollState::StaticCollapse
            } else {
                RagdollState::Activating
            },
            position,
            velocity,
            mass_kg: mass_kg.max(0.0),
            reduced_motion_skip: reduced_motion,
            activated_at_tick: tick,
        }
    }

    /// Promote Activating → Active when the actor transitions DYING → DEAD.
    pub fn promote_to_active(&mut self) {
        if matches!(self.state, RagdollState::Activating | RagdollState::StaticCollapse) {
            self.state = RagdollState::Active;
        }
    }

    pub fn is_active(self) -> bool {
        !matches!(self.state, RagdollState::Animated)
    }
}

/// **M14**: ragdoll integration step. Applies gravity + clamps to the
/// floor in the same way `cf_physics::step_kinematics` does for the active
/// actor, but without the user-controlled jump/move impulses. Pure helper;
/// returns the new state.
#[must_use]
pub fn step_ragdoll(
    ragdoll: Ragdoll,
    gravity: f32,
    tick_dt: f32,
    floor_y: f32,
    half_extent_y: f32,
    terminal_velocity_y: f32,
) -> Ragdoll {
    let (x, y) = ragdoll.position;
    let (mut vx, mut vy) = ragdoll.velocity;
    // Apply gravity.
    vy += gravity * tick_dt;
    if vy < terminal_velocity_y {
        vy = terminal_velocity_y;
    }
    // Decay horizontal velocity with a constant ground-drag analogue (the
    // body slides across terrain). Friction in air = 0; once on ground we
    // damp toward zero.
    let mut new_y = y + vy * tick_dt;
    let floor_top = floor_y + half_extent_y;
    if new_y <= floor_top {
        new_y = floor_top;
        vy = 0.0;
        let friction_step = 600.0 * tick_dt;
        if vx.abs() <= friction_step {
            vx = 0.0;
        } else {
            vx -= friction_step * vx.signum();
        }
    }
    let new_x = x + vx * tick_dt;
    Ragdoll {
        state: ragdoll.state,
        position: (new_x, new_y),
        velocity: (vx, vy),
        mass_kg: ragdoll.mass_kg,
        reduced_motion_skip: ragdoll.reduced_motion_skip,
        activated_at_tick: ragdoll.activated_at_tick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_default_uses_activating() {
        let r = Ragdoll::activate((0.0, 100.0), (10.0, 0.0), 80.0, false, 1);
        assert!(matches!(r.state, RagdollState::Activating));
        assert!(!r.reduced_motion_skip);
    }

    #[test]
    fn activate_reduced_motion_uses_static_collapse() {
        let r = Ragdoll::activate((0.0, 100.0), (10.0, 0.0), 80.0, true, 1);
        assert!(matches!(r.state, RagdollState::StaticCollapse));
        assert!(r.reduced_motion_skip);
    }

    #[test]
    fn promote_moves_to_active() {
        let mut r = Ragdoll::activate((0.0, 100.0), (0.0, 0.0), 80.0, false, 1);
        r.promote_to_active();
        assert!(matches!(r.state, RagdollState::Active));
    }

    #[test]
    fn promote_from_static_collapse_moves_to_active() {
        let mut r = Ragdoll::activate((0.0, 100.0), (0.0, 0.0), 80.0, true, 1);
        r.promote_to_active();
        assert!(matches!(r.state, RagdollState::Active));
    }

    #[test]
    fn step_applies_gravity() {
        let r = Ragdoll::activate((0.0, 200.0), (0.0, 0.0), 80.0, false, 0);
        let r2 = step_ragdoll(r, -980.0, 1.0 / 60.0, 0.0, 16.0, -2000.0);
        assert!(r2.velocity.1 < 0.0);
    }

    #[test]
    fn step_clamps_to_floor() {
        let r = Ragdoll::activate((0.0, 17.0), (50.0, -1000.0), 80.0, false, 0);
        let r2 = step_ragdoll(r, -980.0, 1.0 / 60.0, 0.0, 16.0, -2000.0);
        assert!((r2.position.1 - 16.0).abs() < 1e-3);
        assert!((r2.velocity.1).abs() < f32::EPSILON);
    }
}
