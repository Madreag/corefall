//! M6: extended stance state machine.
//!
//! M1 + M5 ship a 12-variant `Stance` (in `lib.rs`); M6 adds the modern
//! tactical surface: Sprint, Slide, Vault, Dive, Lean, Prone, ProneWalk,
//! CrouchWalk, Dying, StealthAttack, KnifeThrow, RopeClimb, LadderClimb,
//! PipeClimb, Swim. The base enum is extended in-place in `lib.rs`; this
//! module owns the derivation + transition tables.
//!
//! The state machine is pure: input is the kinematic + intent + actor flags;
//! output is the new stance. No clock reads.

use serde::{Deserialize, Serialize};

use cf_trench::{
    cover_state as derive_trench_cover_state, cover_state_fire_step, CoverState as TrenchCoverState,
    SegmentVariant as TrenchSegmentVariant, TrenchSegmentLookup, TrenchStance,
};

use crate::{ActorState, Stance, Status, Vec2};

/// One-stop record of all M6 stance inputs. Engine builds this each tick
/// from the actor's [`crate::ActorState`] + edge/sticky intents from
/// [`crate::ControlIntent`], then calls [`derive_stance`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StanceInputs {
    pub velocity: Vec2,
    pub on_ground: bool,
    pub status: Status,
    pub crouch_active: bool,
    pub prone_active: bool,
    pub climb_active: bool,
    pub jet_active: bool,
    pub ejecting: bool,
    pub sprint_active: bool,
    pub slide_active: bool,
    pub vault_active: bool,
    pub dive_active: bool,
    pub lean_active: bool,
    pub stealth_attack_active: bool,
    pub knife_throw_active: bool,
    pub knockdown_ticks_remaining: u32,
    pub dying_ticks_remaining: u32,
    pub panic_freeze_ticks_remaining: u32,
}

impl Default for StanceInputs {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            on_ground: true,
            status: Status::Stable,
            crouch_active: false,
            prone_active: false,
            climb_active: false,
            jet_active: false,
            ejecting: false,
            sprint_active: false,
            slide_active: false,
            vault_active: false,
            dive_active: false,
            lean_active: false,
            stealth_attack_active: false,
            knife_throw_active: false,
            knockdown_ticks_remaining: 0,
            dying_ticks_remaining: 0,
            panic_freeze_ticks_remaining: 0,
        }
    }
}

/// Derive the full M6 stance from the input bag. Priority order is fixed:
/// status overrides everything; then ejecting; then active animations
/// (slide / vault / dive / climb / stealth attack / knife throw); then
/// posture flags (sprint, prone, crouch); then kinematic stance.
#[must_use]
pub fn derive_stance(inputs: StanceInputs) -> Stance {
    if inputs.knockdown_ticks_remaining > 0 {
        return Stance::KnockedDown;
    }
    if inputs.panic_freeze_ticks_remaining > 0 {
        return Stance::PanickedFreeze;
    }
    if inputs.dying_ticks_remaining > 0 {
        return Stance::Dying;
    }
    match inputs.status {
        Status::Dead => return Stance::Dead,
        Status::Downed | Status::Inert => return Stance::Downed,
        Status::Inactive => return Stance::Idle,
        Status::Dying => return Stance::Dying,
        _ => {}
    }
    if inputs.ejecting {
        return Stance::Ejecting;
    }
    if inputs.stealth_attack_active {
        return Stance::StealthAttack;
    }
    if inputs.knife_throw_active {
        return Stance::KnifeThrow;
    }
    if inputs.vault_active {
        return Stance::Vault;
    }
    if inputs.slide_active {
        return Stance::Slide;
    }
    if inputs.dive_active {
        return Stance::Dive;
    }
    if inputs.jet_active {
        return Stance::Jetting;
    }
    if inputs.climb_active {
        return Stance::Climbing;
    }
    if !inputs.on_ground {
        return Stance::Airborne;
    }
    let speed = inputs.velocity.x.abs();
    if inputs.prone_active {
        return if speed >= Stance::WALK_THRESHOLD {
            Stance::ProneWalk
        } else {
            Stance::Prone
        };
    }
    if inputs.crouch_active {
        return if speed >= Stance::WALK_THRESHOLD {
            Stance::CrouchWalk
        } else {
            Stance::Crouching
        };
    }
    if inputs.sprint_active && speed >= Stance::RUN_THRESHOLD {
        return Stance::Sprint;
    }
    if speed >= Stance::RUN_THRESHOLD {
        Stance::Running
    } else if speed >= Stance::WALK_THRESHOLD {
        Stance::Walking
    } else {
        Stance::Stand
    }
}

/// Returns true when the actor in this stance is allowed to fire ranged
/// weapons. Cinematic stances (Slide/Vault/Climb/Dive/StealthAttack/
/// KnifeThrow) lock the weapon trigger.
///
/// + zip-lining + rope-hanging + swim-surface allow firing (the rider /
/// rope-bob / swimmer can still aim their free arm). Wall-jump locks fire
/// during the 200 ms cinematic.
#[must_use]
pub fn fire_allowed_in_stance(stance: Stance) -> bool {
    matches!(
        stance,
        Stance::Stand
            | Stance::Idle
            | Stance::Walking
            | Stance::Running
            | Stance::Sprint
            | Stance::Crouching
            | Stance::CrouchWalk
            | Stance::Prone
            | Stance::ProneWalk
            | Stance::Lean
            | Stance::Airborne
            | Stance::Climbing
            | Stance::Jetting
            | Stance::Mounted
            | Stance::Ziplining
            | Stance::RopeHanging
            | Stance::SwimSurface
    )
}

/// Returns true when the stance is one of the M6 cinematic transition
/// states (animation-bound; can't be interrupted by ordinary movement).
///
/// windows that cannot be interrupted by ordinary input.
#[must_use]
pub fn is_cinematic(stance: Stance) -> bool {
    matches!(
        stance,
        Stance::Slide
            | Stance::Vault
            | Stance::Dive
            | Stance::RopeClimb
            | Stance::LadderClimb
            | Stance::PipeClimb
            | Stance::StealthAttack
            | Stance::KnifeThrow
            | Stance::WallJump
    )
}

/// Per-stance bloom multiplier (lower = tighter cone). Implements the
/// literal table from M1 spec § "Movement accuracy bloom" + M6 spec
/// § "Crouch reduces bloom + improves aim":
/// - Standing/Walking = 1.0× (baseline)
/// - Crouching = 0.6× / Prone = 0.4× (M6 crouch/prone bonuses)
/// - Running/Jumping = 7.0× (per OpenSoldat `Sprites.pas:4870` + spec line 244)
/// - Jetting = 7.0× (per spec line 244)
/// - Airborne / Prone-transition = 3.0×
/// - Slide/Vault/Dive = cinematic transition penalties
///
/// Jetting from 3.0× to 7.0× to match the literal spec table; previously
/// the implementation was internally consistent but visibly drifted from
/// the spec's OpenSoldat-baseline values.
#[must_use]
pub fn stance_bloom_factor(stance: Stance) -> f32 {
    match stance {
        Stance::Crouching => 0.6,
        Stance::CrouchWalk => 0.75,
        Stance::Prone => 0.4,
        Stance::ProneWalk => 0.55,
        Stance::Lean => 0.8,
        // Running / Jumping share the M1 spec line 244 literal (7.0×).
        Stance::Running => 7.0,
        // Sprint sits between Running and the cinematic stances. Spec line
        // 444 lumps "running/jumping/jetting=7×"; Sprint as a separate
        // stance carries the same 7.0× penalty.
        Stance::Sprint => 7.0,
        Stance::Climbing | Stance::RopeClimb | Stance::LadderClimb | Stance::PipeClimb => 2.0,
        // Jetting matches spec literal 7.0×.
        Stance::Jetting => 7.0,
        // Airborne + prone-transition = 3.0× per spec line 444.
        Stance::Airborne => 3.0,
        Stance::Slide => 0.9,
        Stance::Dive | Stance::Vault => 2.5,
        // less stable (still moving on a pendulum); zip-line trades tighter
        // grouping for forced glide; mount = 2.0× (a galloping critter is
        // less stable than standing); swim_surface = 2.0×; submerged dive
        // floats heavily.
        Stance::RopeHanging => 2.0,
        Stance::RopeSwinging => 4.0,
        Stance::Ziplining => 2.5,
        Stance::Mounted => 2.0,
        Stance::WallJump => 5.0,
        Stance::SwimSurface => 2.0,
        Stance::SwimSubmerged => 4.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inputs() -> StanceInputs {
        StanceInputs {
            on_ground: true,
            status: Status::Stable,
            ..StanceInputs::default()
        }
    }

    #[test]
    fn dead_overrides_all() {
        let mut i = base_inputs();
        i.status = Status::Dead;
        i.sprint_active = true;
        assert_eq!(derive_stance(i), Stance::Dead);
    }

    #[test]
    fn knockdown_overrides_dying() {
        let mut i = base_inputs();
        i.knockdown_ticks_remaining = 5;
        i.dying_ticks_remaining = 10;
        assert_eq!(derive_stance(i), Stance::KnockedDown);
    }

    #[test]
    fn panic_freeze_locks_movement_above_dying() {
        let mut i = base_inputs();
        i.panic_freeze_ticks_remaining = 90;
        i.dying_ticks_remaining = 10;
        assert_eq!(derive_stance(i), Stance::PanickedFreeze);
        assert!(Stance::PanickedFreeze.locks_fire());
    }

    #[test]
    fn sprint_requires_run_speed() {
        let mut i = base_inputs();
        i.sprint_active = true;
        i.velocity = Vec2::new(70.0, 0.0);
        assert_eq!(derive_stance(i), Stance::Sprint);
        i.velocity = Vec2::new(40.0, 0.0);
        assert_eq!(derive_stance(i), Stance::Walking);
    }

    #[test]
    fn prone_walk_when_moving() {
        let mut i = base_inputs();
        i.prone_active = true;
        i.velocity = Vec2::new(15.0, 0.0);
        assert_eq!(derive_stance(i), Stance::ProneWalk);
    }

    #[test]
    fn crouch_when_stationary() {
        let mut i = base_inputs();
        i.crouch_active = true;
        assert_eq!(derive_stance(i), Stance::Crouching);
    }

    #[test]
    fn slide_overrides_sprint() {
        let mut i = base_inputs();
        i.sprint_active = true;
        i.slide_active = true;
        i.velocity = Vec2::new(100.0, 0.0);
        assert_eq!(derive_stance(i), Stance::Slide);
    }

    #[test]
    fn vault_overrides_kinematic() {
        let mut i = base_inputs();
        i.on_ground = false;
        i.vault_active = true;
        assert_eq!(derive_stance(i), Stance::Vault);
    }

    #[test]
    fn fire_locked_in_cinematic() {
        assert!(!fire_allowed_in_stance(Stance::Slide));
        assert!(!fire_allowed_in_stance(Stance::Vault));
        assert!(!fire_allowed_in_stance(Stance::Dive));
        assert!(!fire_allowed_in_stance(Stance::StealthAttack));
        assert!(!fire_allowed_in_stance(Stance::KnifeThrow));
        assert!(fire_allowed_in_stance(Stance::Crouching));
        assert!(fire_allowed_in_stance(Stance::Prone));
        assert!(fire_allowed_in_stance(Stance::Sprint));
    }

    #[test]
    fn crouch_bloom_under_baseline() {
        assert!(stance_bloom_factor(Stance::Crouching) < stance_bloom_factor(Stance::Stand));
        assert!(stance_bloom_factor(Stance::Prone) < stance_bloom_factor(Stance::Crouching));
        assert!(stance_bloom_factor(Stance::Airborne) > stance_bloom_factor(Stance::Stand));
    }
}

/// three-state [`TrenchStance`] axis used by the cf-trench cover-state
/// derivation. Crouch-variants → [`TrenchStance::Crouched`]; prone-variants
/// → [`TrenchStance::Prone`]; everything else (Stand, Walking, Running,
/// Sprint, Airborne, Idle, Slide, Vault, Dive, Climb, Jet, Eject, etc.) →
/// [`TrenchStance::Standing`] because the actor's torso is at full standing
/// silhouette for cover-routing purposes.
#[must_use]
pub fn trench_stance_for(stance: Stance) -> TrenchStance {
    match stance {
        Stance::Crouching | Stance::CrouchWalk | Stance::Slide => TrenchStance::Crouched,
        Stance::Prone | Stance::ProneWalk => TrenchStance::Prone,
        _ => TrenchStance::Standing,
    }
}

/// to a single [`TrenchStance`] for cover-state derivation. Honours
/// `prone_active` and `crouch_active` directly so cover updates the
/// instant the player toggles the intent flag — even when `ActorState::stance()`
/// (which routes via `Stance::from_chassis`) has not yet promoted those
/// intents into the chassis-derived Stance value.
#[must_use]
pub fn trench_stance_for_actor(actor: &ActorState) -> TrenchStance {
    if actor.prone_active {
        return TrenchStance::Prone;
    }
    if actor.crouch_active {
        return TrenchStance::Crouched;
    }
    trench_stance_for(actor.stance())
}

impl ActorState {
    /// `world` lookup. The result equals
    /// `cf_trench::cover_state(trench_stance_for_actor(self),
    /// segment.variant)` when the actor stands inside a segment, and
    /// [`TrenchCoverState::Exposed`] on open ground. Per spec §"Notes"
    /// the value is **derived, not stored** — mutate stance and the next
    /// call observes the new cover.
    ///
    /// `{ fortification_id }` the cover state is unconditionally
    /// [`TrenchCoverState::Full`], regardless of the underlying trench
    /// segment (the spec § "Crewing semantics" promises Full cover when
    /// the actor is bound to a static fortification — MG nest / tripod /
    /// bunker firing slit).
    ///
    /// `world` is any implementor of [`cf_trench::TrenchSegmentLookup`].
    /// m9b-2's procgen + m9b-3's cfctl handlers each provide their own
    /// implementation against the chunked terrain index.
    #[must_use]
    pub fn cover_state<W: TrenchSegmentLookup + ?Sized>(&self, world: &W) -> TrenchCoverState {
        if self.is_crewing() {
            return TrenchCoverState::Full;
        }
        let trench_stance = trench_stance_for_actor(self);
        let tile_x = self.position.x as i32;
        let tile_y = self.position.y as i32;
        match world.segment_at(tile_x, tile_y) {
            Some(segment) => {
                // fire_step has an on/off-step sub-axis. The actor stands
                // on-step when its y-position sits within the
                // raised_step_height band at the top of the segment's
                // depth range. Off-step (the default) routes through the
                // canonical (stance × variant) table.
                if let Some(step_height) = segment.raised_step_height {
                    if matches!(segment.variant, TrenchSegmentVariant::FireStep) {
                        let on_step = tile_y
                            >= segment.tile_y + segment.depth as i32 - step_height as i32;
                        return cover_state_fire_step(trench_stance, on_step);
                    }
                }
                derive_trench_cover_state(trench_stance, segment.variant)
            }
            None => TrenchCoverState::Exposed,
        }
    }

    /// `Stance::Crewing { fortification_id }` spec-shape (M9C § "Crewing
    /// semantics"). The bound fortification id lives on
    /// `crewing_fortification_id`; this returns true exactly when the
    /// id is `Some(_)`.
    #[must_use]
    pub fn is_crewing(&self) -> bool {
        self.crewing_fortification_id.is_some()
    }

    /// `Crewing { fortification_id }` payload as a plain `u32`.
    #[must_use]
    pub fn crewed_fortification_id(&self) -> Option<u32> {
        self.crewing_fortification_id
    }

    /// binding (M9C § "Crewing semantics"). The binding is 1:1
    /// actor→fortification; movement inputs are suspended at the cf-control
    /// dispatch boundary, primary fire is rebound to the fortification's
    /// mounted weapon at the cf-fortification layer, and `cover_state`
    /// becomes Full per the spec's cover-routing table.
    pub fn crew_fortification(&mut self, fortification_id: u32) {
        self.crewing_fortification_id = Some(fortification_id);
        // Suspend movement intents per spec: "Movement inputs are
        // suspended; firing inputs are rebound to the fortification's
        // mounted weapon."
        self.crouch_active = false;
        self.prone_active = false;
        self.sprint_active = false;
        self.climb_active = false;
        self.jet_active = false;
    }

    /// binding. Caller emits the corresponding `mg_nest_uncrewed`
    /// replay event with the appropriate cause.
    pub fn uncrew_fortification(&mut self) -> Option<u32> {
        self.crewing_fortification_id.take()
    }
}

#[cfg(test)]
mod cover_state_api_tests {
    use super::*;
    use crate::{ActorId, Inventory};
    use cf_trench::segment::{InMemorySegments, TrenchSegment};
    use cf_trench::{CoverState as TrenchCoverState, SegmentVariant, TrenchModule};

    fn standing_player(pos: Vec2) -> ActorState {
        let mut a = ActorState::player(ActorId(1), "blue", pos, 100.0, Inventory::default());
        a.on_ground = true;
        a.crouch_active = false;
        a.prone_active = false;
        a
    }

    fn seg(variant: SegmentVariant, tile_x: i32, tile_y: i32) -> TrenchSegment {
        let (depth, width, step, modules): (u32, u32, Option<u32>, Vec<TrenchModule>) = match variant {
            SegmentVariant::ShallowScrape => (6, 12, None, vec![]),
            SegmentVariant::Standard => (16, 16, None, vec![TrenchModule::Duckboard]),
            SegmentVariant::Deep => (24, 16, None, vec![TrenchModule::Duckboard, TrenchModule::DrainageSump]),
            SegmentVariant::Communication => (16, 8, None, vec![TrenchModule::Duckboard]),
            SegmentVariant::FireStep => (16, 20, Some(8), vec![TrenchModule::Duckboard, TrenchModule::FireStep]),
            SegmentVariant::ParapetRaised => (16, 24, Some(8), vec![TrenchModule::Duckboard, TrenchModule::Breastwork]),
        };
        TrenchSegment {
            variant,
            tile_x,
            tile_y,
            depth,
            width,
            raised_step_height: step,
            embedded_modules: modules,
        }
    }

    /// VAL-M9B feature `actor.cover_state(&world)`: open ground returns
    /// `Exposed`; standing inside a `standard` trench returns `Partial`;
    /// crouching inside `standard` upgrades to `Full`; mutating stance
    /// mid-tick observes the new value.
    #[test]
    fn cover_state_api() {
        let world = InMemorySegments::with_segments(vec![seg(
            SegmentVariant::Standard,
            10,
            0,
        )]);
        // open ground far to the left
        let actor_open = standing_player(Vec2::new(0.0, 5.0));
        assert_eq!(actor_open.cover_state(&world), TrenchCoverState::Exposed);

        // inside the standard segment — standing
        let mut actor_in = standing_player(Vec2::new(15.0, 8.0));
        assert_eq!(actor_in.cover_state(&world), TrenchCoverState::Partial);

        // mutate stance to crouched → expect Full
        actor_in.crouch_active = true;
        assert_eq!(actor_in.cover_state(&world), TrenchCoverState::Full);

        // toggle to prone → still Full (standard is Full when prone)
        actor_in.crouch_active = false;
        actor_in.prone_active = true;
        assert_eq!(actor_in.cover_state(&world), TrenchCoverState::Full);
    }

    /// Standing inside `deep` → Full (head below grade). VAL-M9B-SEGMENT-003.
    #[test]
    fn cover_state_api_deep_standing_is_full() {
        let world = InMemorySegments::with_segments(vec![seg(SegmentVariant::Deep, 0, 0)]);
        let a = standing_player(Vec2::new(8.0, 4.0));
        assert_eq!(a.cover_state(&world), TrenchCoverState::Full);
    }

    /// `fire_step` on-step Standing → Exposed; off-step Standing → Partial.
    /// VAL-M9B-SEGMENT-004.
    #[test]
    fn cover_state_api_fire_step_on_off_step() {
        let world = InMemorySegments::with_segments(vec![seg(SegmentVariant::FireStep, 0, 0)]);
        // fire_step: depth=16, raised_step_height=Some(8). On-step band
        // sits at y >= 16-8 = 8. tile_y=10 → on-step.
        let on_step_player = standing_player(Vec2::new(5.0, 10.0));
        assert_eq!(on_step_player.cover_state(&world), TrenchCoverState::Exposed);

        // tile_y=2 → off-step (lower than step band).
        let off_step_player = standing_player(Vec2::new(5.0, 2.0));
        assert_eq!(off_step_player.cover_state(&world), TrenchCoverState::Partial);
    }

    /// Stance changes mid-tick are visible immediately (derivation is
    /// not cached). VAL-M9B-COVER-001 mirror on the actor surface.
    #[test]
    fn cover_state_api_not_cached_across_stance_change() {
        let world = InMemorySegments::with_segments(vec![seg(SegmentVariant::Standard, 0, 0)]);
        let mut a = standing_player(Vec2::new(8.0, 8.0));
        let first = a.cover_state(&world);
        a.crouch_active = true;
        let second = a.cover_state(&world);
        a.crouch_active = false;
        let third = a.cover_state(&world);
        assert_eq!(first, TrenchCoverState::Partial);
        assert_eq!(second, TrenchCoverState::Full);
        assert_eq!(third, TrenchCoverState::Partial);
    }

    #[test]
    fn trench_stance_for_collapses_full_axis() {
        assert_eq!(trench_stance_for(Stance::Stand), TrenchStance::Standing);
        assert_eq!(trench_stance_for(Stance::Walking), TrenchStance::Standing);
        assert_eq!(trench_stance_for(Stance::Running), TrenchStance::Standing);
        assert_eq!(trench_stance_for(Stance::Sprint), TrenchStance::Standing);
        assert_eq!(trench_stance_for(Stance::Airborne), TrenchStance::Standing);
        assert_eq!(trench_stance_for(Stance::Crouching), TrenchStance::Crouched);
        assert_eq!(trench_stance_for(Stance::CrouchWalk), TrenchStance::Crouched);
        assert_eq!(trench_stance_for(Stance::Slide), TrenchStance::Crouched);
        assert_eq!(trench_stance_for(Stance::Prone), TrenchStance::Prone);
        assert_eq!(trench_stance_for(Stance::ProneWalk), TrenchStance::Prone);
    }

    /// **VAL-M9C-011**: `Stance::Crewing { fortification_id }` grants
    /// Full cover irrespective of trench segment. The spec § "Crewing
    /// semantics" promises:
    ///
    /// > a crewed fortification has a 1:1 actor→fortification binding.
    /// > The actor's stance becomes `Crewing { fortification_id }`.
    /// > Movement inputs are suspended; firing inputs are rebound to
    /// > the fortification's mounted weapon.
    ///
    /// The cover routing overrides any underlying segment-derived
    /// cover (Exposed / Partial / Full all promote to Full once the
    /// actor crews a fortification).
    #[test]
    fn stance_crewing_full_cover() {
        // Open ground (no trench): un-crewed actor → Exposed; crewed
        // actor → Full.
        let world = InMemorySegments::with_segments(vec![]);
        let mut a = standing_player(Vec2::new(0.0, 0.0));
        assert_eq!(a.cover_state(&world), TrenchCoverState::Exposed);
        a.crew_fortification(42);
        assert!(a.is_crewing());
        assert_eq!(a.crewed_fortification_id(), Some(42));
        assert_eq!(a.cover_state(&world), TrenchCoverState::Full);

        // Even inside a fire_step segment standing on-step (which would
        // otherwise be Exposed), crewing forces Full.
        let world_fire_step =
            InMemorySegments::with_segments(vec![seg(SegmentVariant::FireStep, 0, 0)]);
        // tile_y=10 is on-step (depth=16, step_height=8 → step band at
        // y>=8); standing → Exposed pre-crew.
        let mut b = standing_player(Vec2::new(5.0, 10.0));
        assert_eq!(
            b.cover_state(&world_fire_step),
            TrenchCoverState::Exposed,
            "on-step baseline must be Exposed before crewing"
        );
        b.crew_fortification(7);
        assert_eq!(b.cover_state(&world_fire_step), TrenchCoverState::Full);

        // Movement intent flags are suspended on crew.
        assert!(!b.crouch_active);
        assert!(!b.prone_active);
        assert!(!b.sprint_active);

        // Uncrew releases the binding + restores normal cover routing.
        // Off-step position (5, 2) → standing → Partial after uncrew.
        let released = b.uncrew_fortification();
        assert_eq!(released, Some(7));
        assert!(!b.is_crewing());
        assert_eq!(b.crewed_fortification_id(), None);
        b.position = Vec2::new(5.0, 2.0);
        assert_eq!(b.cover_state(&world_fire_step), TrenchCoverState::Partial);
    }
}
