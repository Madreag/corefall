//! **M1**: physics-authority transitions (animation ↔ ragdoll ↔ explosion).
//!
//! The spec's "## Files" section lists this module separately from the
//! kinematics path. The actual authority-change events are emitted by
//! `cf-control::engine` when the actor's stance crosses into KnockedDown
//! / Downed / Dying / Dead and back out — see `cf-control/src/engine.rs`
//! around the `physics.authority_changed` recorder calls. This file holds
//! the public surface for that contract so consumers can `use cf_physics::authority::*`.

/// Stable replay vocabulary for the `physics.authority_changed` event's
/// `to`/`from` fields. The engine emits the snake_case names verbatim so
/// downstream consumers can match on a known set.
///
/// **M1**: animation = controlled by the actor controller (intent-driven).
/// ragdoll = post-knockdown / death physical state.
/// explosion = M5.5+ overrides (gibbing, large-impulse displacement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityKind {
    Animation,
    Ragdoll,
    Explosion,
}

impl AuthorityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityKind::Animation => "animation",
            AuthorityKind::Ragdoll => "ragdoll",
            AuthorityKind::Explosion => "explosion",
        }
    }
}

/// One authority transition. Returned by the engine's per-tick body-status
/// classifier so the recorder can emit a `physics.authority_changed` event
/// with `{ actor, from, to, cause_event_id }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityTransition {
    pub from: AuthorityKind,
    pub to: AuthorityKind,
}

impl AuthorityTransition {
    pub fn new(from: AuthorityKind, to: AuthorityKind) -> Self {
        Self { from, to }
    }

    pub fn is_change(self) -> bool {
        self.from != self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_round_trip() {
        assert_eq!(AuthorityKind::Animation.as_str(), "animation");
        assert_eq!(AuthorityKind::Ragdoll.as_str(), "ragdoll");
        assert_eq!(AuthorityKind::Explosion.as_str(), "explosion");
    }

    #[test]
    fn is_change_distinguishes_self_vs_other() {
        assert!(!AuthorityTransition::new(AuthorityKind::Animation, AuthorityKind::Animation).is_change());
        assert!(AuthorityTransition::new(AuthorityKind::Animation, AuthorityKind::Ragdoll).is_change());
    }
}
