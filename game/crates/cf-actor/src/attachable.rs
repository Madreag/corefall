//! **M14**: attachable state + damage propagation per CCCP `Attachable::Update`.
//!
//! An [`Attachable`] is a child object bound to a parent body zone via a
//! joint (`cf_physics::Joint`). Each attachable has its own HP pool +
//! `damage_multiplier` weighting that propagates damage upward through the
//! joint to the parent body's HP when the joint is intact, OR detaches the
//! attachable as a physical debris object when the joint snaps.
//!
//! Determinism: every public mutator is pure; no clocks, no `thread_rng`.
//! The engine seeds any RNG it needs (severance roll) and feeds it in.

use serde::{Deserialize, Serialize};

/// One attachable bound to a parent body zone. Identified by a stable
/// string id (e.g. `"left_arm"`, `"backpack"`, `"weapon_mount"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachable {
    /// Stable string id (used for replay correlation + event payloads).
    pub id: String,
    /// Parent body zone this attachable is bound to. When the parent
    /// zone is destroyed, this attachable cascades (per CCCP
    /// `RemoveAttachablesWhenGibbing`).
    pub parent_zone: String,
    /// HP pool for the attachable itself (e.g. how much damage it can soak
    /// before it gibs).
    pub hp: f32,
    pub max_hp: f32,
    /// Damage-multiplier weighting propagated up to the parent body
    /// (per CCCP `m_DamageMultiplier`). 1.0 = passthrough; 2.0 = double damage.
    pub damage_multiplier: f32,
    /// True when the attachable has detached as a physical debris object
    /// (intact, not gibbed). Once true, the attachable no longer routes
    /// damage to the parent — it's purely a kinematic body.
    pub detached: bool,
    /// True when the attachable has gibbed (shattered into authored
    /// particles). Mutually exclusive with `detached`.
    pub gibbed: bool,
    /// Tick the detach / gib transition fired (engine-supplied).
    pub state_change_tick: u64,
    /// Whether the attachable can be detached at all. False for
    /// `mission_critical` actors' arms / head per spec § "Visible
    /// consequences per CCCP body damage".
    pub can_detach: bool,
}

impl Attachable {
    /// Construct a fresh attachable.
    pub fn new(id: impl Into<String>, parent_zone: impl Into<String>, max_hp: f32, damage_multiplier: f32) -> Self {
        Self {
            id: id.into(),
            parent_zone: parent_zone.into(),
            hp: max_hp.max(0.0),
            max_hp: max_hp.max(0.0),
            damage_multiplier: damage_multiplier.max(0.0),
            detached: false,
            gibbed: false,
            state_change_tick: 0,
            can_detach: true,
        }
    }

    /// Mark the attachable as cleanly detached. Returns `true` if the
    /// transition fired (it was intact and `can_detach`).
    pub fn detach(&mut self, tick: u64) -> bool {
        if self.detached || self.gibbed || !self.can_detach {
            return false;
        }
        self.detached = true;
        self.state_change_tick = tick;
        self.hp = 0.0;
        true
    }

    /// Mark the attachable as gibbed (overrides detach). Returns `true`
    /// if the transition fired.
    pub fn gib(&mut self, tick: u64) -> bool {
        if self.gibbed {
            return false;
        }
        self.gibbed = true;
        self.detached = false;
        self.state_change_tick = tick;
        self.hp = 0.0;
        true
    }

    pub fn is_intact(&self) -> bool {
        !self.detached && !self.gibbed && self.hp > 0.0
    }
}

/// upward through its joint to the parent body's HP pool.
///
/// Damage routing:
///   - The attachable absorbs damage up to its remaining HP.
///   - Any overflow + `damage * damage_multiplier` propagates to the parent.
///   - When `damage_multiplier > 1.0` and `damage` is fractional, the
///     parent receives a multiplied scalar even when the attachable
///     survives (per CCCP `m_DamageMultiplier`).
#[must_use]
pub fn apply_damage(attachable: &mut Attachable, damage: f32) -> f32 {
    if attachable.detached || attachable.gibbed {
        return 0.0;
    }
    let dmg = damage.max(0.0);
    let absorbed = dmg.min(attachable.hp);
    attachable.hp = (attachable.hp - absorbed).max(0.0);
    let overflow = dmg - absorbed;
    (overflow + dmg * attachable.damage_multiplier).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_attachable_intact() {
        let a = Attachable::new("left_arm", "torso", 50.0, 1.0);
        assert!(a.is_intact());
        assert!(!a.detached);
        assert!(!a.gibbed);
    }

    #[test]
    fn detach_marks_state_changed() {
        let mut a = Attachable::new("left_arm", "torso", 50.0, 1.0);
        assert!(a.detach(42));
        assert!(a.detached);
        assert_eq!(a.state_change_tick, 42);
        assert!(!a.is_intact());
    }

    #[test]
    fn detach_rejects_when_already_detached() {
        let mut a = Attachable::new("left_arm", "torso", 50.0, 1.0);
        a.detach(10);
        assert!(!a.detach(20));
    }

    #[test]
    fn detach_rejects_when_cant_detach() {
        let mut a = Attachable::new("left_arm", "torso", 50.0, 1.0);
        a.can_detach = false;
        assert!(!a.detach(1));
        assert!(!a.detached);
    }

    #[test]
    fn gib_marks_state_changed_and_overrides_detach() {
        let mut a = Attachable::new("left_arm", "torso", 50.0, 1.0);
        a.detach(10);
        assert!(a.gib(20));
        assert!(a.gibbed);
        assert!(!a.detached);
        assert_eq!(a.state_change_tick, 20);
    }

    #[test]
    fn apply_damage_propagates_via_multiplier() {
        let mut a = Attachable::new("left_arm", "torso", 100.0, 2.0);
        let propagated = apply_damage(&mut a, 10.0);
        // multiplier 2.0 → 20 propagated to parent.
        assert!((propagated - 20.0).abs() < 1e-3);
        assert!((a.hp - 90.0).abs() < 1e-3);
    }

    #[test]
    fn apply_damage_zero_when_detached() {
        let mut a = Attachable::new("left_arm", "torso", 100.0, 2.0);
        a.detach(5);
        let propagated = apply_damage(&mut a, 50.0);
        assert!((propagated).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_damage_overflow_propagates() {
        let mut a = Attachable::new("left_arm", "torso", 10.0, 1.0);
        let propagated = apply_damage(&mut a, 50.0);
        // 10 absorbed, 40 overflow + 50 * 1.0 = 90 propagated.
        assert!((propagated - 90.0).abs() < 1e-3);
        assert!((a.hp).abs() < f32::EPSILON);
    }
}
