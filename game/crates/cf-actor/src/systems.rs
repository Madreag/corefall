//! M8A § Files / cf-actor — ECS systems scaffold.
//!
//! M8A's parallel-determinism refactor decomposes the monolithic actor
//! tick into four sub-systems that the engine scheduler (M9+) runs
//! sequentially per tick. Within each sub-system, individual actor work
//! runs over the snapshot-read pattern: read previous-tick state (frozen),
//! compute into per-actor buffers, single-threaded merge at the
//! sub-system boundary.
//!
//! The sub-systems are:
//!
//! 1. `apply_intent` — consume `ControlIntent` → update `Stance` / `Aim` /
//!    `Vel`. Pre-rolls RNG for any randomized rolls (e.g., recoil drift,
//!    stamina ticks).
//! 2. `step_kinematics` — integrate `Pos = Pos + Vel * dt`; read terrain
//!    snapshot.
//! 3. `derive_status` — recompute `Hp` / `Stability` / `Stamina` from
//!    previous-tick state.
//! 4. `latch_outcomes` — write back `ActorTickOutcome` to recorder +
//!    observer surfaces.
//!
//! Each sub-system has signature `fn(&[ActorBundle], &mut [ActorBundle],
//! &PreRolledRng) -> ()` so a future Bevy ECS query system can wrap it.
//! M8A ships the scaffold; M9+ wires it through Bevy's parallel
//! scheduler.

use crate::components::ActorBundle;

///
/// Per `docs/plan/spec/determinism-island-contract.md` rule 2: RNG calls
/// must NOT happen inside `par_iter` closures. Pre-roll into a `Vec<u64>`
/// of length N BEFORE entering the parallel block; workers index by
/// stable entity id.
#[derive(Debug, Clone, Default)]
pub struct PreRolledRng {
    pub values: Vec<u64>,
}

impl PreRolledRng {
    pub fn new(n: usize) -> Self {
        Self {
            values: Vec::with_capacity(n),
        }
    }

    pub fn fill(&mut self, seed: u64, n: usize) {
        self.values.clear();
        self.values.reserve(n);
        let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        for _ in 0..n {
            state = state
                .wrapping_mul(0xBF58_476D_1CE4_E5B9)
                .wrapping_add(0xD2B7_4407_B1CE_6E93);
            self.values.push(state);
        }
    }

    pub fn get(&self, actor_id: usize) -> u64 {
        self.values.get(actor_id).copied().unwrap_or(0)
    }
}

///
/// Each actor's work is per-entity isolated (no cross-actor reads). Safe
/// to par_iter over `actors_in.iter().zip(actors_out.iter_mut())`.
pub fn apply_intent(actors_in: &[ActorBundle], actors_out: &mut [ActorBundle], _rng: &PreRolledRng) {
    debug_assert_eq!(actors_in.len(), actors_out.len());
    for (i, prev) in actors_in.iter().enumerate() {
        actors_out[i] = prev.clone();
    }
}

pub fn step_kinematics(actors: &mut [ActorBundle], dt_seconds: f32) {
    for actor in actors {
        actor.pos.x += actor.vel.x * dt_seconds;
        actor.pos.y += actor.vel.y * dt_seconds;
    }
}

pub fn derive_status(actors: &mut [ActorBundle]) {
    for actor in actors {
        actor.stamina.value = (actor.stamina.value + actor.stamina.regen_per_tick).min(100.0);
    }
}

///
/// The actual outcome serialization happens in cf-replay; this stage is
/// the merge point where per-actor outcomes converge for the recorder.
pub fn latch_outcomes(_actors: &[ActorBundle]) {
    // Outcome latching is hooked from cf-control's drive_tick in M9+.
}

/// from u64 RNG uses the 53-bit-mantissa trick, with f32 output (no f64
/// passes the boundary into sim state).
///
/// Pre-M8A the cf-actor::sim path used `(rng() as f64) / (u64::MAX as
/// f64)` which traverses f64 unnecessarily. The 53-bit trick below
/// produces the canonical uniform f32 distribution used by cf-ai.
#[inline]
pub fn rng_to_uniform_f32(raw: u64) -> f32 {
    let bits = (raw >> 11) as f64 / ((1u64 << 53) as f64);
    bits as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_rolled_rng_is_deterministic() {
        let mut a = PreRolledRng::new(10);
        let mut b = PreRolledRng::new(10);
        a.fill(42, 10);
        b.fill(42, 10);
        assert_eq!(a.values, b.values);
    }

    #[test]
    fn step_kinematics_integrates_velocity() {
        let mut actors = vec![ActorBundle::default(); 3];
        for actor in &mut actors {
            actor.vel.x = 1.0;
        }
        step_kinematics(&mut actors, 0.5);
        for actor in &actors {
            assert!((actor.pos.x - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn rng_to_uniform_f32_is_in_unit_interval() {
        for raw in [0u64, 1, 1234, u64::MAX] {
            let v = rng_to_uniform_f32(raw);
            assert!((0.0..=1.0).contains(&v), "raw={raw} v={v}");
        }
    }
}
