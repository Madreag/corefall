//! M1 actor simulation step: per-tick actor + rifle + projectile pipeline.
//!
//! This module owns the deterministic per-tick state transition for the M1 actor world:
//!
//! 1. Snapshot the inbound [`ControlIntent`]s.
//! 2. Apply movement/aim/jump/reload/fire intent.
//! 3. Step kinematics through `cf-physics`.
//! 4. Step the rifle state machine (`cf-equipment`).
//! 5. Spawn projectiles when fire fired this tick.
//! 6. Step projectiles + check actor hits.
//! 7. Resolve status changes from accumulated damage.
//!
//! Output is a [`StepReport`] of structured outcomes the engine turns into recorder
//! events. The sim itself never touches the recorder so we keep `cf-actor` decoupled
//! from `cf-replay`.
//!
//! Types + helpers live in `sim_state`, `sim_deps`, `sim_outcomes` siblings;
//! the per-actor / per-projectile / per-loose-item step functions live in
//! `sim_step_actor`, `sim_step_projectile`, `sim_step_loose`. All public items
//! are re-exported here so `cf_actor::sim::X` paths remain stable.

use std::collections::BTreeMap;

use crate::{ActorId, ControlIntent, IntentSource};

pub use crate::sim_deps::{ActorTuning, StepDeps};
pub use crate::sim_outcomes::{
    zone_from_hit, ActorTickOutcome, ExpiredProjectile, HitOutcome, SettledLooseItem, SpawnedProjectile, StepReport,
};
pub use crate::sim_state::{ActorSimState, LooseItem, Projectile, RifleStates};

use crate::sim_step_actor::step_one_actor;
use crate::sim_step_loose::step_loose_items;
use crate::sim_step_projectile::step_projectiles;

/// Run one fixed-tick step for every actor in `state.world`.
///
/// `intents` maps actor → their [`ControlIntent`] for this tick. Actors without an
/// entry are stationary (idle move, no fire/jump/reload). Actors whose `status` does
/// not [`crate::Status::accepts_input`] ignore movement/fire/reload intents but still
/// take physics steps (so a downed actor falls).
///
/// `rng` is the engine's seeded source consumed for multi-particle spread cones.
/// Pass any `FnMut() -> u64`; in production it forwards to
/// `cf_sim_core::Rng::next_u64`. Single-particle weapons never call it.
pub fn step<R: FnMut() -> u64>(
    state: &mut ActorSimState,
    intents: &mut BTreeMap<ActorId, ControlIntent>,
    deps: StepDeps,
    rng: &mut R,
) -> StepReport {
    let mut report = StepReport::default();

    let actor_ids: Vec<ActorId> = state.world.actors.keys().copied().collect();
    for actor_id in actor_ids {
        let intent = intents
            .remove(&actor_id)
            .unwrap_or_else(|| ControlIntent::new(actor_id, IntentSource::Cfctl));
        let outcome = step_one_actor(state, actor_id, intent, deps, &mut report, rng);
        report.actor_outcomes.push(outcome);
    }

    step_projectiles(state, deps, &mut report);
    step_loose_items(state, deps, &mut report);

    report
}

/// Convenience wrapper for tests / callers that don't need per-tick RNG (no
/// multi-particle weapons in play). Internally seeds a zero state; safe for
/// deterministic single-particle scenarios.
pub fn step_no_rng(
    state: &mut ActorSimState,
    intents: &mut BTreeMap<ActorId, ControlIntent>,
    deps: StepDeps,
) -> StepReport {
    let mut counter: u64 = 0x6b67_c98a_7f3d_1ad9_u64;
    step(state, intents, deps, &mut || {
        counter = counter
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        counter
    })
}
