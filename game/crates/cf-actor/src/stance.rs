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

use crate::{Stance, Status, Vec2};

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
    if inputs.dying_ticks_remaining > 0 {
        return Stance::Dying;
    }
    match inputs.status {
        Status::Dead => return Stance::Dead,
        Status::Downed => return Stance::Downed,
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
    )
}

/// Returns true when the stance is one of the M6 cinematic transition
/// states (animation-bound; can't be interrupted by ordinary movement).
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
    )
}

/// Per-stance bloom multiplier (lower = tighter cone). Mirrors the rule
/// table in M6 spec § "Crouch reduces bloom + improves aim": crouch=0.6
/// (40% reduction), prone=0.4, running=1.0 (baseline), airborne=3.0, etc.
#[must_use]
pub fn stance_bloom_factor(stance: Stance) -> f32 {
    match stance {
        Stance::Crouching => 0.6,
        Stance::CrouchWalk => 0.75,
        Stance::Prone => 0.4,
        Stance::ProneWalk => 0.55,
        Stance::Lean => 0.8,
        Stance::Running => 1.4,
        Stance::Sprint | Stance::Climbing | Stance::RopeClimb | Stance::LadderClimb | Stance::PipeClimb => 2.0,
        Stance::Airborne | Stance::Jetting => 3.0,
        Stance::Slide => 0.9,
        Stance::Dive | Stance::Vault => 2.5,
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
