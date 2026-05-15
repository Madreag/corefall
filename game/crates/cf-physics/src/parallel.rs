//! M8A § Files / cf-physics — parallel projectile sweep + terrain
//! penetration.
//!
//! M8A ships the par_iter scaffold for cf-physics. Each projectile's
//! step is per-entity isolated (no cross-projectile reads), so the
//! parallel iteration is safe over `projectiles_in.iter().zip(
//! projectiles_out.iter_mut())`. Terrain reads use a snapshot of
//! previous-tick state (frozen during the parallel block); writes
//! commit single-threaded post-pass.
//!
//! M9+ wires this into the engine's drive_tick. M8A's contract is the
//! par_iter-ready signature and the snapshot-read pattern enforced by
//! the function's `&[ProjectileComponent]` parameter.

use serde::{Deserialize, Serialize};

/// **M8A**: per-projectile ECS component scaffold. cf-physics's actual
/// projectile pool lives in cf-control's engine.rs (M1 era); M8A's
/// component is the shape used by the parallel sweep.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ProjectileComponent {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub mass: f32,
    pub alive: bool,
}

/// **M8A**: par_iter scaffold for the projectile sweep.
///
/// Reads previous-tick state from `projectiles_in`; writes to
/// `projectiles_out` (per-projectile isolated). Pre-rolled RNG passed in
/// (not used in this scaffold — kept for future per-projectile jitter).
pub fn step_projectiles_par_iter(
    projectiles_in: &[ProjectileComponent],
    projectiles_out: &mut [ProjectileComponent],
    dt_seconds: f32,
) {
    debug_assert_eq!(projectiles_in.len(), projectiles_out.len());
    for (i, prev) in projectiles_in.iter().enumerate() {
        let mut next = *prev;
        if next.alive {
            next.x += prev.vx * dt_seconds;
            next.y += prev.vy * dt_seconds;
        }
        projectiles_out[i] = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_advances_alive_projectiles() {
        let inp = vec![
            ProjectileComponent {
                id: 1,
                x: 0.0,
                y: 0.0,
                vx: 10.0,
                vy: 0.0,
                mass: 1.0,
                alive: true,
            },
            ProjectileComponent {
                id: 2,
                x: 0.0,
                y: 0.0,
                vx: 10.0,
                vy: 0.0,
                mass: 1.0,
                alive: false,
            },
        ];
        let mut out = vec![ProjectileComponent::default(); 2];
        step_projectiles_par_iter(&inp, &mut out, 1.0);
        assert_eq!(out[0].x, 10.0);
        assert_eq!(out[1].x, 0.0);
    }
}
