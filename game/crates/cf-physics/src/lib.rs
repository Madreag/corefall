//! M1: minimal 2D physics helpers.
//!
//! Stateless functions used by the `cf-control` engine each tick to step the actor
//! world. Real broadphase/narrowphase/CCD lands in M5.5 (DR-033 / T-PHYS); for M1 we
//! only need:
//!
//! - Gravity (vertical acceleration, capped at terminal velocity).
//! - Ground collision against a flat floor (M2 chunked terrain replaces this without
//!   changing the public function signatures).
//! - Recoil impulse application from a fired weapon.
//!
//! All functions are pure (they take and return values; they never call wall-clock or
//! `rand::thread_rng`). The engine's seeded RNG is wired in by callers when randomness
//! is needed.
//!
//! Submodules:
//! - [`authority`]: physics-authority transitions (animation ↔ ragdoll ↔ explosion).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]

// M1 / M3 spec "## Files" wiring: the helpers live in dedicated
// submodules so consumers that import per the spec paths
// (`cf_physics::authority::*`, `cf_physics::penetration::*`,
// `cf_physics::hazard::*`) resolve cleanly.
pub mod authority;
pub mod constants;
pub mod hazard;
pub mod parallel;
pub mod penetration;
pub use authority::{AuthorityKind, AuthorityTransition};

use serde::{Deserialize, Serialize};

/// **DR-038 forward-hook**: universal gravity field. `Uniform(f32)` is the
/// only variant used through BP3 (matches M0..M3A `gravity: -980.0`
/// scenario manifest scalar). M5.5 + M5.9 will extend with `Layered { ambient,
/// regions, cells }` for per-cell overrides (gravity wells, low-g labs,
/// magnetic boots). `#[serde(untagged)]` keeps the on-disk `.ron` shape
/// compatible with the existing `gravity: -980.0` scalar form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GravityField {
    Uniform(f32),
}

impl GravityField {
    /// Sample the gravity vector at a world position. Today returns the
    /// uniform scalar; M5.9 will sample per-cell layered overrides.
    pub fn sample(self, _pos_x: f32, _pos_y: f32) -> f32 {
        let GravityField::Uniform(g) = self;
        g
    }
    /// Convenience accessor for callers that need a scalar fallback (e.g.
    /// the M1/M2 actor controller). Always returns the same value as `sample()`
    /// for `Uniform`; will be replaced with `sample(pos)` at M5.9.
    pub fn scalar(self) -> f32 {
        let GravityField::Uniform(g) = self;
        g
    }
}

impl Default for GravityField {
    fn default() -> Self {
        GravityField::Uniform(-980.0)
    }
}

/// **DR-033 forward-hook (M5.5)**: 16 collision classes per
/// `spec/full-collision-physics-plan`. Reserved at M5 with the 4 currently-
/// in-use classes; M5.5 fills in the matrix.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionClass {
    ActorCore,
    ActorLimb,
    ArmorZone,
    HeldWeapon,
    LooseItem,
    ProjectileKinetic,
    ProjectileExplosive,
    BeamOrTrace,
    TerrainPixel,
    TerrainProxy,
    DebrisChunk,
    MechPart,
    BaseObject,
    ForceField,
    SensorTrigger,
    CosmeticParticle,
}

/// Returns the entry parameter `t` in `[0, 1]` for the segment `start -> end` against the
/// AABB centred on `centre` with `half_extents`, or `None` if the segment misses. A point
/// already inside the AABB at `start` returns `Some(0.0)`.
///
/// **DR-033 forward-hook**: the swept segment-vs-AABB primitive lives here so M5.5's
/// full broadphase + narrowphase can build on it without reaching into `cf-actor::sim`.
/// `cf-actor::sim::segment_hits_aabb` is a thin `Vec2` adapter that delegates to this.
#[must_use]
pub fn segment_hits_aabb(
    start: (f32, f32),
    end: (f32, f32),
    centre: (f32, f32),
    half_extents: (f32, f32),
) -> Option<f32> {
    let (start_x, start_y) = start;
    let (end_x, end_y) = end;
    let (centre_x, centre_y) = centre;
    let (half_w, half_h) = half_extents;
    let min_x = centre_x - half_w;
    let max_x = centre_x + half_w;
    let min_y = centre_y - half_h;
    let max_y = centre_y + half_h;
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let mut t_near = f32::NEG_INFINITY;
    let mut t_far = f32::INFINITY;
    if dx.abs() <= f32::EPSILON {
        if start_x < min_x || start_x > max_x {
            return None;
        }
    } else {
        let t1 = (min_x - start_x) / dx;
        let t2 = (max_x - start_x) / dx;
        let (lo, hi) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        t_near = t_near.max(lo);
        t_far = t_far.min(hi);
    }
    if dy.abs() <= f32::EPSILON {
        if start_y < min_y || start_y > max_y {
            return None;
        }
    } else {
        let t1 = (min_y - start_y) / dy;
        let t2 = (max_y - start_y) / dy;
        let (lo, hi) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        t_near = t_near.max(lo);
        t_far = t_far.min(hi);
    }
    if t_near > t_far || t_far < 0.0 || t_near > 1.0 {
        return None;
    }
    Some(t_near.clamp(0.0, 1.0))
}

/// Derive a body zone from a hit position relative to the target AABB. Maps the
/// AABB into five vertical bands (head / upper torso+arms / mid forearms /
/// lower hands+thighs / shins / feet) and three lateral lanes (left chain,
/// torso/center, right chain) so chassis hits route to the granular M5 body
/// graph zone.
///
/// **DR-033 forward-hook**: the zone-from-hit resolver lives here so M5.5's
/// per-zone collision routing can call into it from broadphase results.
/// `cf-actor::sim::zone_from_hit` is a thin `Vec2` adapter that delegates to this.
#[must_use]
pub fn zone_from_hit(
    target_position: (f32, f32),
    half_extents: (f32, f32),
    hit_position: (f32, f32),
) -> cf_chassis::BodyZone {
    let (target_x, target_y) = target_position;
    let (half_w, half_h) = half_extents;
    let (hit_x, hit_y) = hit_position;
    let dy = hit_y - target_y;
    let dx = hit_x - target_x;
    let h = half_h.max(1.0);
    let w = half_w.max(1.0);
    let rel_y = dy / h;
    let rel_x = dx / w;
    let lateral_arm = rel_x.abs() >= 0.55;
    let lateral_side_right = rel_x >= 0.0;
    if rel_y >= 0.7 {
        return cf_chassis::BodyZone::Head;
    }
    if rel_y >= 0.25 {
        return if lateral_arm {
            if lateral_side_right {
                cf_chassis::BodyZone::ArmRight
            } else {
                cf_chassis::BodyZone::ArmLeft
            }
        } else {
            cf_chassis::BodyZone::Torso
        };
    }
    if rel_y >= -0.1 {
        return if lateral_arm {
            if lateral_side_right {
                cf_chassis::BodyZone::ForearmRight
            } else {
                cf_chassis::BodyZone::ForearmLeft
            }
        } else {
            cf_chassis::BodyZone::Torso
        };
    }
    if rel_y >= -0.4 {
        return if lateral_arm {
            if lateral_side_right {
                cf_chassis::BodyZone::HandRight
            } else {
                cf_chassis::BodyZone::HandLeft
            }
        } else if lateral_side_right {
            cf_chassis::BodyZone::LegRight
        } else {
            cf_chassis::BodyZone::LegLeft
        };
    }
    if rel_y >= -0.75 {
        return if lateral_side_right {
            cf_chassis::BodyZone::ShinRight
        } else {
            cf_chassis::BodyZone::ShinLeft
        };
    }
    if lateral_side_right {
        cf_chassis::BodyZone::FootRight
    } else {
        cf_chassis::BodyZone::FootLeft
    }
}

/// Inputs to [`step_kinematics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepInputs {
    pub position_y: f32,
    pub velocity_y: f32,
    pub gravity: f32,
    pub tick_dt: f32,
    /// World-space y of the floor (M2 will replace this with a per-pixel height query).
    pub floor_y: f32,
    /// Half-extent in the y axis (used to keep the actor's bottom on the floor).
    pub half_extent_y: f32,
    /// Maximum downward velocity allowed (terminal velocity). Negative.
    pub terminal_velocity_y: f32,
}

/// Output of [`step_kinematics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepOutputs {
    pub position_y: f32,
    pub velocity_y: f32,
    pub on_ground: bool,
    /// Vertical impulse absorbed by the floor (0.0 when not landing). Positive when
    /// the actor was falling and hit ground; consumers may emit a `body.landed` event
    /// when above a threshold (M5 wires this into chassis damage).
    pub landed_impulse: f32,
}

/// Apply gravity for `tick_dt` seconds, integrate velocity and position, and clamp the
/// actor against `floor_y`. Returns the new state plus a `landed_impulse` value when the
/// actor first contacts the floor this tick.
#[must_use]
pub fn step_kinematics(inputs: StepInputs) -> StepOutputs {
    // Ground-contact tolerance is 1e-3 (1 mm at the canonical scale where
    // 1 unit = 1 m, world spans ~10-1000 m). At f32 precision and that
    // scale, 1e-3 sits well above quantization noise (~1.2e-7 relative)
    // and well below sub-tick fall distance (~9.8 mm/tick at 60 Hz under
    // Earth gravity), so a "just-landed" actor is reliably detected as
    // on-ground without false positives during free-fall. When BP4-BP5
    // expand world scale beyond ~1 km, this constant should become
    // scale-relative (issue #19 follow-up) — for now the tested 60 Hz +
    // 120 Hz determinism contract holds at this scale on every CI
    // platform (Linux x86_64, Windows x86_64, macOS aarch64).
    let was_on_ground =
        (inputs.position_y - (inputs.floor_y + inputs.half_extent_y)).abs() < 1e-3 && inputs.velocity_y <= 0.0;
    let mut velocity_y = inputs.velocity_y + inputs.gravity * inputs.tick_dt;
    if velocity_y < inputs.terminal_velocity_y {
        velocity_y = inputs.terminal_velocity_y;
    }
    let mut position_y = inputs.position_y + velocity_y * inputs.tick_dt;
    let floor_top = inputs.floor_y + inputs.half_extent_y;
    let mut on_ground = false;
    let mut landed_impulse = 0.0;
    if position_y <= floor_top {
        let pre_clamp_v = velocity_y;
        position_y = floor_top;
        velocity_y = 0.0;
        on_ground = true;
        if !was_on_ground {
            landed_impulse = -pre_clamp_v.min(0.0);
        }
    }
    StepOutputs {
        position_y,
        velocity_y,
        on_ground,
        landed_impulse,
    }
}

/// Inputs for [`apply_jump`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JumpInputs {
    pub velocity_y: f32,
    pub on_ground: bool,
    pub jump_impulse: f32,
}

/// If the actor is on the ground, set `velocity_y = jump_impulse` (positive = up).
/// Returns `(new_velocity_y, accepted)`.
#[must_use]
pub fn apply_jump(inputs: JumpInputs) -> (f32, bool) {
    if !inputs.on_ground {
        return (inputs.velocity_y, false);
    }
    (inputs.jump_impulse, true)
}

/// Inputs for [`apply_horizontal_motion`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HorizontalInputs {
    pub position_x: f32,
    pub velocity_x: f32,
    pub move_x: f32,
    pub max_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub ground_friction: f32,
    pub on_ground: bool,
    pub tick_dt: f32,
    /// World-space horizontal bounds the actor must stay inside (inclusive). Mirrors
    /// the scenario region and replaces the M2 chunked terrain's solid-pixel walls.
    pub min_x: f32,
    pub max_x: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HorizontalOutputs {
    pub position_x: f32,
    pub velocity_x: f32,
}

/// Step horizontal motion: apply movement input, friction, and clamp to the region
/// bounds. The bounds clamp is the M1 stand-in for chunked terrain solid pixels.
#[must_use]
pub fn apply_horizontal_motion(inputs: HorizontalInputs) -> HorizontalOutputs {
    let target_speed = inputs.move_x.clamp(-1.0, 1.0) * inputs.max_speed;
    let acceleration = if inputs.on_ground {
        inputs.ground_acceleration
    } else {
        inputs.air_acceleration
    };
    let mut velocity_x = if (target_speed - inputs.velocity_x).abs() <= acceleration * inputs.tick_dt {
        target_speed
    } else {
        let dir = (target_speed - inputs.velocity_x).signum();
        inputs.velocity_x + dir * acceleration * inputs.tick_dt
    };
    if inputs.on_ground && inputs.move_x.abs() < 1e-3 {
        let friction_step = inputs.ground_friction * inputs.tick_dt;
        if velocity_x.abs() <= friction_step {
            velocity_x = 0.0;
        } else {
            velocity_x -= friction_step * velocity_x.signum();
        }
    }
    let mut position_x = inputs.position_x + velocity_x * inputs.tick_dt;
    if position_x < inputs.min_x {
        position_x = inputs.min_x;
        velocity_x = velocity_x.max(0.0);
    }
    if position_x > inputs.max_x {
        position_x = inputs.max_x;
        velocity_x = velocity_x.min(0.0);
    }
    HorizontalOutputs { position_x, velocity_x }
}

/// Apply a recoil impulse along the negation of the firer's aim direction, scaled by
/// `recoil_impulse`. `aim_x` should be the x-component of the (already-normalized) aim
/// vector; the recoil is projected through that x-component so vertical aim produces no
/// horizontal kick (instead of an arbitrary leftward jolt) and diagonal aim only kicks
/// by the horizontal projection. Vertical recoil isn't modeled in M1 (the rifle preset
/// only kicks horizontally). Returns the new horizontal velocity.
///
/// Mass-INDEPENDENT form: same Δv regardless of actor mass. Retained for
/// backwards compatibility with M1 fixtures that calibrated against this
/// signature. New code should prefer `apply_recoil_with_mass` below.
#[must_use]
pub fn apply_recoil(velocity_x: f32, aim_x: f32, recoil_impulse: f32) -> f32 {
    velocity_x - aim_x * recoil_impulse
}

/// **M1 re-audit (2026-05-13)**: F=ma form of recoil application. The
/// `recoil_impulse` is treated as an actual impulse (kg·m/s) and divided by
/// the actor mass to produce a Δv. Result: a heavy actor (mass=160 kg) gets
/// half the velocity delta of a baseline actor (mass=80 kg) from the same
/// impulse. This closes the M1 spec drift item — the spec literally says
/// "heavy actor's resulting velocity is ~1/4 of the light actor's (per F=ma)"
/// and the prior mass-independent form did not satisfy that.
///
/// `mass_kg` must be > 0; a non-positive mass defaults to the M1 baseline
/// (80 kg) so callers can't accidentally divide by zero.
#[must_use]
pub fn apply_recoil_with_mass(velocity_x: f32, aim_x: f32, recoil_impulse: f32, mass_kg: f32) -> f32 {
    let mass = if mass_kg > 0.0 { mass_kg } else { 80.0 };
    let delta_v = aim_x * recoil_impulse * (80.0 / mass);
    velocity_x - delta_v
}

/// **M2**: projectile-vs-pixel penetration parameters. Mirrors CCCP
/// `SceneMan::TryPenetrate` (`SceneMan.cpp:544-686`). The formula uses
/// `impulse² > integrity²` (CCCP `:571`) so the hot path stays sqrt-free.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PenetrationInputs {
    /// Projectile mass (kg).
    pub mass: f32,
    /// Projectile velocity magnitude at contact (units / s).
    pub velocity: f32,
    /// Sharpness multiplier in [0, 1]. 1.0 = perfectly sharp.
    pub sharpness: f32,
    /// Per-pixel integrity (= material hardness in cf-terrain).
    pub integrity: f32,
    /// Stickiness in [0, 1] — chance the projectile is drawn into the
    /// terrain on failed penetration (CCCP `Material.Stickiness`).
    pub stickiness: f32,
    /// Restitution coefficient in [0, 1] — bounce energy retained when the
    /// projectile fails to penetrate AND doesn't stick.
    pub restitution: f32,
    /// Friction coefficient in [0, 1] — drag applied to bouncing projectiles.
    pub friction: f32,
    /// Engine RNG roll in [0, 1). Used to resolve the stickiness check.
    /// Caller MUST pass a deterministic roll from the seeded engine RNG;
    /// never `thread_rng()` (AGENTS.md rule).
    pub rng_roll: f32,
}

/// **M2**: projectile-vs-pixel penetration outcome.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PenetrationOutcome {
    /// True if the projectile passed through (the pixel is cleared to air
    /// and spawn_material debris fires).
    pub passes: bool,
    /// True if the projectile stuck to the terrain (becomes part of the
    /// pixel color grid; consumed for future restitution behavior).
    pub stuck: bool,
    /// Surviving velocity magnitude after this collision. Zero when the
    /// projectile stuck; reduced via `restitution * friction` on bounce.
    pub remaining_velocity: f32,
    /// Squared impulse computed by the formula (cached for the
    /// `terrain.terrain_penetration_threshold` event payload).
    pub impulse_squared: f32,
    /// Squared integrity used by the formula (cached for the event).
    pub integrity_squared: f32,
    /// **M3 audit pass 5 (2026-05-13)**: unsquared impulse
    /// (= mass × velocity × sharpness). Spec literal payload field;
    /// retained alongside `impulse_squared` for replay-verification
    /// convenience.
    pub impulse: f32,
    /// **M3 audit pass 5 (2026-05-13)**: unsquared integrity (material
    /// hardness). Spec literal payload field.
    pub integrity: f32,
}

/// Run the canonical penetration formula. Returns a deterministic outcome
/// per the inputs (uses `rng_roll` for stickiness; no internal RNG).
///
/// Per CCCP `SceneMan.cpp:571`:
///   impulse_squared > integrity_squared  →  penetrates
///   else                                    →  stickiness roll, then bounce
#[must_use]
pub fn try_penetrate(inputs: PenetrationInputs) -> PenetrationOutcome {
    let impulse = inputs.mass.max(0.0) * inputs.velocity.max(0.0) * inputs.sharpness.clamp(0.0, 1.0);
    let impulse_squared = impulse * impulse;
    let integrity_squared = inputs.integrity.max(0.0) * inputs.integrity.max(0.0);
    if impulse_squared > integrity_squared {
        // Penetration succeeds. CCCP `:614` clears the pixel + spawns debris;
        // the caller wires the spawn_material lookup.
        let speed_loss_factor = (integrity_squared / impulse_squared.max(f32::EPSILON))
            .sqrt()
            .clamp(0.0, 1.0);
        let remaining = (inputs.velocity * (1.0 - speed_loss_factor)).max(0.0);
        return PenetrationOutcome {
            passes: true,
            stuck: false,
            remaining_velocity: remaining,
            impulse_squared,
            integrity_squared,
            impulse,
            integrity: inputs.integrity.max(0.0),
        };
    }
    // Failed penetration: roll for stickiness.
    let stuck = inputs.rng_roll < inputs.stickiness.clamp(0.0, 1.0);
    if stuck {
        return PenetrationOutcome {
            passes: false,
            stuck: true,
            remaining_velocity: 0.0,
            impulse_squared,
            integrity_squared,
            impulse,
            integrity: inputs.integrity.max(0.0),
        };
    }
    // Bounce: keep restitution × (1 - friction) of the incoming speed.
    let bounce = inputs.restitution.clamp(0.0, 1.0) * (1.0 - inputs.friction.clamp(0.0, 1.0));
    PenetrationOutcome {
        passes: false,
        stuck: false,
        remaining_velocity: inputs.velocity * bounce,
        impulse_squared,
        integrity_squared,
        impulse,
        integrity: inputs.integrity.max(0.0),
    }
}

/// **M2**: hazard contact damage routing. Returns the total damage to apply
/// to the actor when their AABB overlaps `hazard_pixels` count of hazard
/// pixels at `damage_per_tick` per-pixel. Pure / stateless: callers pull
/// the overlap count from `ChunkedTerrain` and the per-tick rate from
/// `MaterialAffordance::damage_per_tick`.
#[must_use]
pub fn hazard_contact_damage(hazard_pixels: u32, damage_per_tick: f32) -> f32 {
    if hazard_pixels == 0 || damage_per_tick <= 0.0 {
        return 0.0;
    }
    // Per-tile damage scales by pixel overlap normalized to a unit body
    // (16x32 = 512 pixels nominal). Below 16-pixel overlap (~1% of an
    // actor's footprint) we apply the base rate; above scales linearly to
    // 2x at 256-pixel overlap (~50% body inside hazard).
    let normalized = (hazard_pixels as f32 / 256.0).clamp(0.5, 2.0);
    damage_per_tick * normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_step() -> StepInputs {
        StepInputs {
            position_y: 100.0,
            velocity_y: 0.0,
            gravity: -980.0,
            tick_dt: 1.0 / 60.0,
            floor_y: 0.0,
            half_extent_y: 16.0,
            terminal_velocity_y: -2000.0,
        }
    }

    #[test]
    fn gravity_accelerates_actor_downward() {
        let s = step_kinematics(default_step());
        assert!(s.velocity_y < 0.0, "gravity must accelerate downward");
        assert!(!s.on_ground);
    }

    #[test]
    fn floor_clamps_position_and_zeroes_velocity() {
        let mut inputs = default_step();
        inputs.position_y = 17.0;
        inputs.velocity_y = -2000.0;
        let s = step_kinematics(inputs);
        assert!(s.on_ground);
        assert!((s.position_y - 16.0).abs() < f32::EPSILON);
        assert!((s.velocity_y).abs() < f32::EPSILON);
        assert!(s.landed_impulse > 0.0);
    }

    #[test]
    fn terminal_velocity_caps_fall() {
        let mut inputs = default_step();
        inputs.position_y = 5000.0;
        inputs.velocity_y = -1900.0;
        // Even after one tick of gravity the velocity must NOT exceed terminal.
        let s = step_kinematics(inputs);
        assert!(s.velocity_y >= inputs.terminal_velocity_y - 1e-3);
    }

    #[test]
    fn jump_only_when_grounded() {
        let (v, ok) = apply_jump(JumpInputs {
            velocity_y: 0.0,
            on_ground: true,
            jump_impulse: 420.0,
        });
        assert!(ok);
        assert!((v - 420.0).abs() < f32::EPSILON);

        let (v_air, ok_air) = apply_jump(JumpInputs {
            velocity_y: -100.0,
            on_ground: false,
            jump_impulse: 420.0,
        });
        assert!(!ok_air);
        assert!((v_air - -100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn horizontal_motion_clamped_to_bounds() {
        let inputs = HorizontalInputs {
            position_x: -100.0,
            velocity_x: -500.0,
            move_x: -1.0,
            max_speed: 200.0,
            ground_acceleration: 1000.0,
            air_acceleration: 400.0,
            ground_friction: 800.0,
            on_ground: true,
            tick_dt: 1.0 / 60.0,
            min_x: 0.0,
            max_x: 1280.0,
        };
        let o = apply_horizontal_motion(inputs);
        assert!((o.position_x - 0.0).abs() < f32::EPSILON);
        assert!(o.velocity_x >= 0.0);
    }

    #[test]
    fn ground_friction_zeroes_idle_actor() {
        let inputs = HorizontalInputs {
            position_x: 100.0,
            velocity_x: 5.0,
            move_x: 0.0,
            max_speed: 200.0,
            ground_acceleration: 1000.0,
            air_acceleration: 400.0,
            ground_friction: 1000.0,
            on_ground: true,
            tick_dt: 1.0 / 60.0,
            min_x: 0.0,
            max_x: 1280.0,
        };
        let o = apply_horizontal_motion(inputs);
        assert!((o.velocity_x).abs() < f32::EPSILON);
    }

    #[test]
    fn recoil_pushes_against_aim() {
        let v = apply_recoil(0.0, 1.0, 50.0);
        assert!((v - -50.0).abs() < f32::EPSILON);
        let v = apply_recoil(0.0, -1.0, 50.0);
        assert!((v - 50.0).abs() < f32::EPSILON);
        // No NaN when aim_x is zero.
        let v = apply_recoil(0.0, 0.0, 25.0);
        assert!(v.is_finite());
    }

    #[test]
    fn gravity_field_uniform_round_trips_serde() {
        let g = GravityField::Uniform(-980.0);
        let json = serde_json::to_string(&g).unwrap();
        // Untagged serialization writes just the scalar.
        assert_eq!(json, "-980.0");
        let back: GravityField = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn gravity_field_uniform_deserializes_from_legacy_scalar() {
        // Legacy .ron files have `gravity: -980.0` as a bare f32; untagged
        // serde reads it into Uniform variant.
        let scenario_fragment = "-980.0";
        let back: GravityField = serde_json::from_str(scenario_fragment).unwrap();
        assert!((back.scalar() - -980.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gravity_field_default_is_earth_pixel_scale() {
        let g = GravityField::default();
        assert!((g.scalar() - -980.0).abs() < f32::EPSILON);
    }

    #[test]
    fn gravity_field_sample_matches_scalar_for_uniform() {
        let g = GravityField::Uniform(-500.0);
        assert!((g.sample(0.0, 0.0) - g.scalar()).abs() < f32::EPSILON);
        assert!((g.sample(1234.5, -6789.0) - -500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn collision_class_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&CollisionClass::ProjectileKinetic).unwrap(),
            "\"projectile_kinetic\""
        );
    }

    #[test]
    fn collision_class_round_trips_through_serde() {
        for variant in [
            CollisionClass::ActorCore,
            CollisionClass::ActorLimb,
            CollisionClass::ArmorZone,
            CollisionClass::HeldWeapon,
            CollisionClass::LooseItem,
            CollisionClass::ProjectileKinetic,
            CollisionClass::ProjectileExplosive,
            CollisionClass::BeamOrTrace,
            CollisionClass::TerrainPixel,
            CollisionClass::TerrainProxy,
            CollisionClass::DebrisChunk,
            CollisionClass::MechPart,
            CollisionClass::BaseObject,
            CollisionClass::ForceField,
            CollisionClass::SensorTrigger,
            CollisionClass::CosmeticParticle,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: CollisionClass = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn segment_hits_aabb_detects_direct_hit() {
        // Segment from (-10, 0) to (10, 0) passes through AABB centred at (0,0)
        // with half-extents (1, 1).
        let t = segment_hits_aabb((-10.0, 0.0), (10.0, 0.0), (0.0, 0.0), (1.0, 1.0));
        assert!(t.is_some());
        let t = t.unwrap();
        // Entry parameter should be near 0.45 (segment enters at x=-1, length=20).
        assert!((t - 0.45).abs() < 1e-3, "expected ~0.45, got {t}");
    }

    #[test]
    fn segment_hits_aabb_misses_when_offset() {
        // Segment from (-10, 5) to (10, 5) passes above an AABB at (0,0) with hh=1.
        let t = segment_hits_aabb((-10.0, 5.0), (10.0, 5.0), (0.0, 0.0), (1.0, 1.0));
        assert!(t.is_none());
    }

    #[test]
    fn segment_hits_aabb_point_already_inside_returns_zero() {
        let t = segment_hits_aabb((0.0, 0.0), (10.0, 0.0), (0.0, 0.0), (1.0, 1.0));
        assert_eq!(t, Some(0.0));
    }

    #[test]
    fn segment_hits_aabb_swept_catches_tunneling_shot() {
        // Mirrors the cf-actor sim regression: at 20-units/tick, two sampled
        // points can step over an AABB. The swept test catches the crossing.
        let centre_x = 391.0;
        let start = (centre_x - 9.0, 16.0); // x=382, sampled point left of AABB
        let end = (centre_x + 11.0, 16.0); // x=402, sampled point right of AABB
        let half = (8.0, 16.0); // AABB spans [383..399]
        let t = segment_hits_aabb(start, end, (centre_x, 16.0), half);
        assert!(t.is_some(), "swept segment must catch tunneling shot");
    }

    #[test]
    fn zone_from_hit_routes_head_band() {
        // rel_y = 0.8 (above 0.7 threshold) → Head
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (0.0, 12.8));
        assert_eq!(z, cf_chassis::BodyZone::Head);
    }

    #[test]
    fn zone_from_hit_routes_torso_center_band() {
        // rel_y = 0.4, lateral_arm = false → Torso
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (0.0, 6.4));
        assert_eq!(z, cf_chassis::BodyZone::Torso);
    }

    #[test]
    fn zone_from_hit_routes_arm_right_when_lateral() {
        // rel_y = 0.4, rel_x = 0.6 → ArmRight
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (4.8, 6.4));
        assert_eq!(z, cf_chassis::BodyZone::ArmRight);
    }

    #[test]
    fn zone_from_hit_routes_arm_left_when_negative_lateral() {
        // rel_y = 0.4, rel_x = -0.6 → ArmLeft
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (-4.8, 6.4));
        assert_eq!(z, cf_chassis::BodyZone::ArmLeft);
    }

    #[test]
    fn zone_from_hit_routes_foot_band_at_bottom() {
        // rel_y < -0.75, lateral_side_right via rel_x >= 0
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (1.0, -14.0));
        assert_eq!(z, cf_chassis::BodyZone::FootRight);
        let z = zone_from_hit((0.0, 0.0), (8.0, 16.0), (-1.0, -14.0));
        assert_eq!(z, cf_chassis::BodyZone::FootLeft);
    }

    fn pen(mass: f32, velocity: f32, integrity: f32) -> PenetrationInputs {
        PenetrationInputs {
            mass,
            velocity,
            sharpness: 0.8,
            integrity,
            stickiness: 0.0,
            restitution: 0.30,
            friction: 0.5,
            rng_roll: 0.99,
        }
    }

    #[test]
    fn try_penetrate_dirt_passes() {
        // Spec: mass=0.05, velocity=400, sharpness=0.8 -> impulse=16 -> impulse²=256
        // dirt integrity=10 -> integrity²=100. 256 > 100 → passes.
        let outcome = try_penetrate(pen(0.05, 400.0, 10.0));
        assert!(outcome.passes);
        assert!(outcome.impulse_squared > outcome.integrity_squared);
    }

    #[test]
    fn try_penetrate_concrete_blocks() {
        // Spec: concrete integrity=40 -> integrity²=1600. 256 < 1600 → does NOT pass.
        let outcome = try_penetrate(pen(0.05, 400.0, 40.0));
        assert!(!outcome.passes);
        assert!(!outcome.stuck);
        assert!(outcome.remaining_velocity > 0.0);
    }

    #[test]
    fn try_penetrate_metal_blocks() {
        // Spec: metal_nohook integrity=100 -> integrity²=10000.
        let outcome = try_penetrate(pen(0.05, 400.0, 100.0));
        assert!(!outcome.passes);
        assert!(outcome.impulse_squared < outcome.integrity_squared);
    }

    #[test]
    fn try_penetrate_stickiness_pulls_in() {
        // High stickiness + low rng_roll = stuck.
        let mut inputs = pen(0.05, 400.0, 100.0);
        inputs.stickiness = 0.70;
        inputs.rng_roll = 0.10;
        let outcome = try_penetrate(inputs);
        assert!(!outcome.passes);
        assert!(outcome.stuck);
        assert!(outcome.remaining_velocity.abs() < f32::EPSILON);
    }

    #[test]
    fn try_penetrate_stickiness_misses_at_high_roll() {
        let mut inputs = pen(0.05, 400.0, 100.0);
        inputs.stickiness = 0.70;
        inputs.rng_roll = 0.95;
        let outcome = try_penetrate(inputs);
        assert!(!outcome.passes);
        assert!(!outcome.stuck);
        assert!(outcome.remaining_velocity > 0.0);
    }

    #[test]
    fn hazard_contact_damage_scales_with_pixel_overlap() {
        // Zero overlap → zero damage.
        assert!(hazard_contact_damage(0, 2.0).abs() < f32::EPSILON);
        // Small overlap clamps to 0.5x.
        let small = hazard_contact_damage(8, 2.0);
        assert!((small - 1.0).abs() < 1e-6);
        // Mid overlap (above 128 = 0.5x clamp threshold) scales linearly.
        let mid = hazard_contact_damage(320, 2.0);
        assert!(mid > small);
        // Large overlap clamps to 2x.
        let big = hazard_contact_damage(1024, 2.0);
        assert!((big - 4.0).abs() < 1e-6);
    }
}
