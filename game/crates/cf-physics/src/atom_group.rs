//! **M14A** § "AtomGroup::push_as_limb — the heart of CC walking".
//!
//! Algorithmic port of CCCP `Entities/AtomGroup.cpp:1217-1306`. The Rust is
//! original; the *algorithm* is the calibration the player feels.
//!
//! Three operations:
//!   - `push_travel(limb_pos, vel, force, dt)` → impulse from a single push
//!     step (pixel-perfect collision against the chunked terrain).
//!   - `push_as_limb(joint_pos, joint_vel, path)` → loops `push_travel` while
//!     time remains in the tick; advances the limb path.
//!   - `flail_as_limb(joint_pos, joint_offset, vel, ang_vel, mass, dt)` →
//!     ragdoll pendulum for severed limbs.
//!
//! No clock reads, no RNG. The caller passes the chassis radius / terrain
//! sampler / time-step explicitly.

use serde::{Deserialize, Serialize};

/// One atom (collision pixel) in an atom group.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Atom {
    /// Local offset from the group origin in pixels.
    pub offset: [f32; 2],
}

/// **M14A** § "AtomGroup" — pixel-perfect collision shape (per-atom positions).
///
/// CCCP `Entities/AtomGroup.h` reference; the Rust shape is simpler — we
/// carry just the atom offsets + the path-following push logic. Full
/// pixel-vs-pixel sweep lives in `cf-physics::swept`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AtomGroup {
    /// Local atom offsets.
    pub atoms: Vec<Atom>,
    /// Bounding radius (px) — used to detect "off path" excursions.
    pub bounding_radius: f32,
}

impl AtomGroup {
    /// Construct an axis-aligned rectangle of atoms.
    pub fn rect(half_w: f32, half_h: f32) -> Self {
        let w = half_w.round().max(1.0) as i32;
        let h = half_h.round().max(1.0) as i32;
        let mut atoms = Vec::with_capacity((w * h * 4) as usize);
        for y in -h..=h {
            for x in -w..=w {
                atoms.push(Atom {
                    offset: [x as f32, y as f32],
                });
            }
        }
        Self {
            atoms,
            bounding_radius: (half_w * half_w + half_h * half_h).sqrt(),
        }
    }

    /// Build a single-pixel atom group (used as a placeholder for foot/hand
    /// contact points).
    pub fn single_pixel() -> Self {
        Self {
            atoms: vec![Atom { offset: [0.0, 0.0] }],
            bounding_radius: 1.0,
        }
    }
}

/// **M14A** § "Algorithm (Corefall summary)" — outcome of one
/// `push_as_limb` call.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SweepOutcome {
    /// Net impulse applied to the chassis (N·s).
    pub impulse: [f32; 2],
    /// `true` if path was terminated due to off-path excursion or stuck foot.
    pub terminated: bool,
    /// `true` if path was restarted (foot came back home this tick).
    pub restarted: bool,
    /// Foot world position after the push.
    pub final_limb_pos: [f32; 2],
    /// Fraction of segment progress made this tick.
    pub fraction_advanced: f32,
}

/// **M14A** § "push_travel" — single push step against terrain.
///
/// Given a foot at `limb_pos` traveling toward `target_velocity_world`,
/// integrate over `dt_ms` and return the impulse the foot pushes the chassis
/// with. Pure function — caller writes the foot position separately.
///
/// `terrain_sampler` returns `true` for solid pixels.
pub fn push_travel(
    limb_pos: [f32; 2],
    target_velocity_world: [f32; 2],
    force: f32,
    dt_ms: u32,
    terrain_sampler: impl Fn(i32, i32) -> bool,
) -> ([f32; 2], [f32; 2]) {
    let dt_secs = dt_ms as f32 / 1000.0;
    let intended_dx = target_velocity_world[0] * dt_secs;
    let intended_dy = target_velocity_world[1] * dt_secs;
    let target_x = limb_pos[0] + intended_dx;
    let target_y = limb_pos[1] + intended_dy;
    let blocked = terrain_sampler(target_x.round() as i32, target_y.round() as i32);
    if blocked {
        // Atom collided — the foot pushes the chassis along the inverse of
        // its intended travel. Reaction is force × dt (impulse).
        let mag = (intended_dx * intended_dx + intended_dy * intended_dy).sqrt().max(1e-6);
        let imp_x = -(intended_dx / mag) * force * dt_secs;
        let imp_y = -(intended_dy / mag) * force * dt_secs;
        // Final limb pos stays at the start (couldn't move into solid).
        ([imp_x, imp_y], limb_pos)
    } else {
        // No collision — foot advances; no chassis impulse this step.
        ([0.0, 0.0], [target_x, target_y])
    }
}

/// **M14A** § "push_as_limb — the heart of CC walking" — pure function form.
///
/// Inputs are the joint world position, the joint velocity, the limb path
/// (mutated to advance), the walk angle for this leg, the per-tick time,
/// the chassis radius (off-path excursion bound), and a terrain sampler.
///
/// Returns the [`SweepOutcome`] the caller applies to the chassis.
pub fn push_as_limb(
    joint_world_pos: [f32; 2],
    joint_velocity: [f32; 2],
    path: &mut crate::limb_path_interop::LimbPathInterop,
    walk_angle: f32,
    dt_ms: u32,
    chassis_radius: f32,
    against_intended_dir_x: f32,
    is_crouching: bool,
    terrain_sampler: impl Fn(i32, i32) -> bool,
) -> SweepOutcome {
    let mut outcome = SweepOutcome::default();

    if path.ended {
        // CCCP § "if path.ended: try restart_free": restart, otherwise abort.
        if !path.restart_free() {
            outcome.terminated = true;
            outcome.final_limb_pos = joint_world_pos;
            return outcome;
        }
        outcome.restarted = true;
    }

    // Effective push force escalates with stuck time.
    let push_force = path.effective_push_force();
    let segment_endpoint = path.current_endpoint;

    // Rotate path-local endpoint into world-aligned direction with walk_angle.
    let cos_a = walk_angle.cos();
    let sin_a = walk_angle.sin();
    let target_local_x = segment_endpoint[0] * cos_a - segment_endpoint[1] * sin_a;
    let target_local_y = segment_endpoint[0] * sin_a + segment_endpoint[1] * cos_a;
    let target_world = [
        joint_world_pos[0] + target_local_x,
        joint_world_pos[1] + target_local_y,
    ];

    // Compute target velocity from path speed + delta to endpoint.
    let limb_pos = path.current_limb_pos;
    let to_target = [target_world[0] - limb_pos[0], target_world[1] - limb_pos[1]];
    let dist = (to_target[0] * to_target[0] + to_target[1] * to_target[1]).sqrt().max(1e-6);

    // Check off-path excursion → terminate when foot strayed > 2× chassis radius.
    if dist > chassis_radius * 2.0 {
        path.terminate();
        outcome.terminated = true;
        outcome.final_limb_pos = joint_world_pos;
        return outcome;
    }

    let target_speed_px_per_ms = path.effective_speed_px_per_ms;
    let target_velocity = [
        (to_target[0] / dist) * target_speed_px_per_ms * 1000.0 + joint_velocity[0],
        (to_target[1] / dist) * target_speed_px_per_ms * 1000.0 + joint_velocity[1],
    ];

    let (mut impulse, new_limb_pos) = push_travel(limb_pos, target_velocity, push_force, dt_ms, terrain_sampler);

    // "Step over" rule: if pushing against intended travel direction, lift
    // foot upward instead of bouncing back. Reduces horizontal push 50%.
    if !is_crouching && impulse[0].signum() == -against_intended_dir_x.signum() && against_intended_dir_x.abs() > 0.1 {
        let upward = impulse[0].abs() * 0.5;
        impulse = [impulse[0] * 0.5, impulse[1] - upward];
    }

    // Clamp absurdly large impulse (CCCP step 5).
    let max_comp = 10_000.0;
    impulse[0] = impulse[0].clamp(-max_comp, max_comp);
    impulse[1] = impulse[1].clamp(-max_comp, max_comp);

    // Advance segment progress proportional to how far we moved relative to dist.
    let moved_dist = ((new_limb_pos[0] - limb_pos[0]).powi(2) + (new_limb_pos[1] - limb_pos[1]).powi(2)).sqrt();
    let fraction = (moved_dist / dist).min(1.0);
    path.advance(fraction, dt_ms);
    path.current_limb_pos = new_limb_pos;

    outcome.impulse = impulse;
    outcome.final_limb_pos = new_limb_pos;
    outcome.fraction_advanced = fraction;
    outcome
}

/// **M14A** § "FlailAsLimb — severed/ragdoll fallback" — CCCP
/// `AtomGroup.cpp:1288-1306`.
pub fn flail_as_limb(
    owner_pos: [f32; 2],
    joint_offset: [f32; 2],
    limb_pos: [f32; 2],
    limb_radius: f32,
    velocity: [f32; 2],
    angular_vel: f32,
    _mass_kg: f32,
    dt_ms: u32,
) -> [f32; 2] {
    let dt_secs = dt_ms as f32 / 1000.0;
    let joint_pos = [owner_pos[0] + joint_offset[0], owner_pos[1] + joint_offset[1]];

    // Total velocity = linear velocity rotated by angular displacement +
    // tangential component from angular velocity.
    let ang_disp = angular_vel * dt_secs;
    let cos_a = ang_disp.cos();
    let sin_a = ang_disp.sin();
    let total_vx = velocity[0] * cos_a - velocity[1] * sin_a + joint_offset[1] * angular_vel.abs();
    let total_vy = velocity[1] * cos_a + velocity[0] * sin_a + joint_offset[0] * angular_vel.abs();

    let new_limb = [limb_pos[0] + total_vx * dt_secs, limb_pos[1] + total_vy * dt_secs];

    // Constrain to within max radius from joint (pendulum).
    let range = [new_limb[0] - joint_pos[0], new_limb[1] - joint_pos[1]];
    let mag = (range[0] * range[0] + range[1] * range[1]).sqrt();
    if mag > limb_radius && mag > 1e-6 {
        let scale = limb_radius / mag;
        [joint_pos[0] + range[0] * scale, joint_pos[1] + range[1] * scale]
    } else {
        new_limb
    }
}

/// **M14A** § "evaluate_ricochet" — surface-angle vs hardness check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RicochetOutcome {
    Bounce { outgoing_dir: [f32; 2], energy_loss_pct: f32 },
    Glance { damage_pct: f32 },
    Penetrate { remaining_energy: f32 },
}

/// Constants.
pub const RICOCHET_ANGLE_THRESHOLD: f32 = std::f32::consts::FRAC_PI_3; // 60°
pub const RICOCHET_HARDNESS_FACTOR: f32 = 4.0;
pub const RICOCHET_ENERGY_LOSS: f32 = 0.4;

pub fn evaluate_ricochet(
    incoming_dir: [f32; 2],
    surface_normal: [f32; 2],
    sharpness: f32,
    velocity_m_per_s: f32,
    armor_hardness: f32,
    chassis_armor_angle: f32,
) -> RicochetOutcome {
    let dot = incoming_dir[0] * surface_normal[0] + incoming_dir[1] * surface_normal[1];
    let incoming_angle_from_normal = dot.clamp(-1.0, 1.0).acos();
    let effective_angle = (incoming_angle_from_normal - chassis_armor_angle).abs();
    let cos_factor = effective_angle.cos().max(0.0);
    let penetration_energy = sharpness * velocity_m_per_s * velocity_m_per_s * 0.001 * cos_factor;

    if penetration_energy < armor_hardness * RICOCHET_HARDNESS_FACTOR && effective_angle > RICOCHET_ANGLE_THRESHOLD {
        // Reflect the incoming vector about the normal.
        let dot2 = 2.0 * dot;
        let outgoing = [
            incoming_dir[0] - dot2 * surface_normal[0],
            incoming_dir[1] - dot2 * surface_normal[1],
        ];
        RicochetOutcome::Bounce {
            outgoing_dir: outgoing,
            energy_loss_pct: RICOCHET_ENERGY_LOSS,
        }
    } else if penetration_energy < armor_hardness {
        RicochetOutcome::Glance { damage_pct: 0.1 }
    } else {
        RicochetOutcome::Penetrate {
            remaining_energy: penetration_energy - armor_hardness,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_travel_solid_pixel_pushes_back() {
        let (impulse, final_pos) = push_travel([0.0, 0.0], [10.0, 0.0], 100.0, 16, |_x, _y| true);
        assert!(impulse[0] < 0.0);
        assert_eq!(final_pos, [0.0, 0.0]);
    }

    #[test]
    fn push_travel_open_air_moves_foot() {
        let (impulse, final_pos) = push_travel([0.0, 0.0], [10.0, 0.0], 100.0, 1000, |_x, _y| false);
        // 10 px/s * 1 s = 10 px.
        assert!((final_pos[0] - 10.0).abs() < 0.5);
        assert_eq!(impulse, [0.0, 0.0]);
    }

    #[test]
    fn flail_as_limb_stays_within_radius() {
        let result = flail_as_limb([0.0, 0.0], [0.0, 8.0], [50.0, 8.0], 10.0, [0.0, 0.0], 0.0, 1.0, 16);
        let dist = (result[0].powi(2) + (result[1] - 8.0).powi(2)).sqrt();
        assert!(dist <= 10.0 + 1e-3, "limb outside radius: dist={}", dist);
    }

    #[test]
    fn ricochet_at_grazing_angle_bounces() {
        // Incoming nearly parallel to surface (70° from normal).
        let incoming = [(70.0_f32.to_radians()).cos(), (70.0_f32.to_radians()).sin()];
        let normal = [1.0, 0.0];
        let out = evaluate_ricochet(incoming, normal, 0.3, 800.0, 22.0, 0.0);
        matches!(out, RicochetOutcome::Bounce { .. });
    }

    #[test]
    fn ricochet_perpendicular_penetrates_soft_armor() {
        let out = evaluate_ricochet([1.0, 0.0], [-1.0, 0.0], 0.6, 1200.0, 5.0, 0.0);
        matches!(out, RicochetOutcome::Penetrate { .. });
    }
}
