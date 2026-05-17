//! **M14**: joint state + impulse propagation per CCCP `Attachable::Update`.
//!
//! Each [`Joint`] connects a child attachable to a parent body zone. The
//! `joint_strength` is the impulse magnitude that snaps the joint cleanly
//! (limb detaches as an intact debris object); `gib_impulse_limit` is the
//! higher impulse that shatters the limb into authored gib particles
//! instead of detaching cleanly. `damage_multiplier` weights damage
//! propagated upward through the joint to the parent body's HP pool
//! (CCCP `m_DamageMultiplier`).
//!
//! All functions are pure (state in → state out); deterministic. No clock,
//! no `thread_rng`. The engine's seeded RNG is wired in by callers when
//! randomness is needed (e.g. melee severance roll in [`severance_roll`]).

use serde::{Deserialize, Serialize};

/// One joint in a body graph. The joint is the connection between a
/// child attachable (limb, weapon, sensor) and its parent zone (torso,
/// arm-stump, backpack mount).
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    /// Impulse magnitude (N) at which the joint cleanly snaps and the
    /// attachable detaches as a single physical-debris object.
    pub joint_strength: f32,
    /// Impulse magnitude (N) at which the joint shatters and the attachable
    /// gibs (per CCCP `Attachable::GibThis`) rather than detaching cleanly.
    /// Must be >= `joint_strength`; M14 enforces this on construction.
    pub gib_impulse_limit: f32,
    /// Damage-multiplier weighting for damage propagated upward through
    /// this joint to the parent body's HP pool (per CCCP `m_DamageMultiplier`).
    /// 1.0 = passthrough; 2.0 = double damage to parent.
    pub damage_multiplier: f32,
    /// Absorption coefficient in [0, 1]. Fraction of incoming impulse the
    /// joint absorbs before propagation continues upward. Default 0.30
    /// matches the falling-damage chain (foot absorbs 30% before forwarding
    /// to shin / leg).
    pub absorption: f32,
}

impl Joint {
    /// Default joint for a typical humanoid attachment. M14 chassis specs
    /// override these per zone.
    pub fn default_for_zone(zone: &str) -> Self {
        match zone {
            "foot_left" | "foot_right" => Self {
                joint_strength: 500.0,
                gib_impulse_limit: 1500.0,
                damage_multiplier: 1.0,
                absorption: 0.30,
            },
            "shin_left" | "shin_right" => Self {
                joint_strength: 1500.0,
                gib_impulse_limit: 4000.0,
                damage_multiplier: 1.0,
                absorption: 0.25,
            },
            "leg_left" | "leg_right" => Self {
                joint_strength: 2500.0,
                gib_impulse_limit: 6000.0,
                damage_multiplier: 1.2,
                absorption: 0.20,
            },
            "arm_left" | "arm_right" => Self {
                joint_strength: 80.0,
                gib_impulse_limit: 150.0,
                damage_multiplier: 1.0,
                absorption: 0.30,
            },
            "forearm_left" | "forearm_right" | "hand_left" | "hand_right" => Self {
                joint_strength: 60.0,
                gib_impulse_limit: 110.0,
                damage_multiplier: 1.0,
                absorption: 0.30,
            },
            "head" => Self {
                joint_strength: 200.0,
                gib_impulse_limit: 350.0,
                damage_multiplier: 2.5,
                absorption: 0.10,
            },
            "torso" => Self {
                joint_strength: 12000.0,
                gib_impulse_limit: 25000.0,
                damage_multiplier: 1.5,
                absorption: 0.10,
            },
            "backpack" => Self {
                joint_strength: 300.0,
                gib_impulse_limit: 600.0,
                damage_multiplier: 0.5,
                absorption: 0.25,
            },
            _ => Self {
                joint_strength: 100.0,
                gib_impulse_limit: 250.0,
                damage_multiplier: 1.0,
                absorption: 0.30,
            },
        }
    }

    /// Construct a joint, normalizing `gib_impulse_limit >= joint_strength`
    /// and clamping absorption / damage_multiplier so caller mistakes don't
    /// silently produce nonsense state.
    pub fn new(joint_strength: f32, gib_impulse_limit: f32, damage_multiplier: f32, absorption: f32) -> Self {
        let js = joint_strength.max(0.0);
        let gil = gib_impulse_limit.max(js);
        Self {
            joint_strength: js,
            gib_impulse_limit: gil,
            damage_multiplier: damage_multiplier.max(0.0),
            absorption: absorption.clamp(0.0, 1.0),
        }
    }
}

/// Result of evaluating one joint against an incoming impulse.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JointEval {
    /// Incoming impulse magnitude (N).
    pub impulse_in: f32,
    /// Impulse the joint absorbed in the contact.
    pub impulse_absorbed: f32,
    /// Impulse forwarded upward through the body graph (parent-side).
    pub impulse_out: f32,
    /// True when the joint cleanly detaches (impulse > joint_strength AND
    /// impulse <= gib_impulse_limit).
    pub detach: bool,
    /// True when the joint gibs (impulse > gib_impulse_limit).
    pub gib: bool,
    /// Damage propagated upward (impulse_out * joint.damage_multiplier).
    pub propagated_damage: f32,
}

/// Evaluate one joint against an incoming impulse. Returns the absorbed +
/// forwarded magnitudes plus the detach / gib verdicts. Deterministic.
#[must_use]
pub fn evaluate_joint(joint: Joint, impulse_in: f32) -> JointEval {
    let imp = impulse_in.max(0.0);
    let absorbed = imp * joint.absorption.clamp(0.0, 1.0);
    let out = (imp - absorbed).max(0.0);
    let gib = imp > joint.gib_impulse_limit;
    let detach = !gib && imp > joint.joint_strength;
    JointEval {
        impulse_in: imp,
        impulse_absorbed: absorbed,
        impulse_out: out,
        detach,
        gib,
        propagated_damage: out * joint.damage_multiplier.max(0.0),
    }
}

/// **M14**: falling-damage impulse chain. Given a landing velocity (m/s)
/// plus actor mass (kg) plus a chain of joints (in body-graph order,
/// foot to leg to torso), return per-joint evaluations. The first joint
/// that detaches or gibs is the "severance point"; later joints in the
/// chain still see reduced impulse forwarded through the absorbed cascade.
///
/// Per CCCP `AHuman` foot landing — impulse splits across both feet when
/// standing; M14 callers pass `mass_kg / 2.0` when the actor has both feet
/// on the ground, else full mass.
#[must_use]
pub fn fall_impulse_chain(landing_velocity: f32, mass_kg: f32, joints: &[(String, Joint)]) -> Vec<(String, JointEval)> {
    let mut out: Vec<(String, JointEval)> = Vec::with_capacity(joints.len());
    let mut impulse = (landing_velocity.abs() * mass_kg.max(0.0)).max(0.0);
    for (zone, joint) in joints {
        let eval = evaluate_joint(*joint, impulse);
        impulse = eval.impulse_out;
        out.push((zone.clone(), eval));
    }
    out
}

/// **M14**: heavy-melee severance probability. Returns a [0, 1] probability
/// that a melee weapon strike severs the limb at this joint. Roll an engine
/// RNG sample uniform in [0, 1] and compare: if `roll < severance_probability`,
/// severance fires + a `attachable.detached` event emits with cause `"melee_severance"`.
///
/// `severance_chance` is the weapon's authored coefficient (chainsaw=0.4,
/// katana=0.3, hatchet=0.15, knife/baton=0.0). The joint's normalized strength
/// suppresses severance for tougher attachment points.
#[must_use]
pub fn severance_probability(severance_chance: f32, joint_strength: f32, reference_strength: f32) -> f32 {
    let chance = severance_chance.clamp(0.0, 1.0);
    let ref_s = reference_strength.max(1.0);
    let normalized = (joint_strength / ref_s).clamp(0.0, 1.0);
    (chance * (1.0 - normalized)).clamp(0.0, 1.0)
}

/// **M14**: deterministic severance roll. Takes a [0, 1) RNG draw from the
/// engine's seeded RNG and a probability from [`severance_probability`].
/// Returns `true` when `roll < probability` (severance fires).
#[must_use]
pub fn severance_roll(rng_roll: f32, probability: f32) -> bool {
    rng_roll < probability.clamp(0.0, 1.0)
}

/// **M14**: explosion proximity severance impulse. Impulse on actor falls
/// off as `base_impulse / (1 + distance²)` (inverse-square, regularized so
/// callers don't divide by zero at `distance == 0`). Used to evaluate every
/// joint against the same impulse-at-actor; the nearest zone severs first.
#[must_use]
pub fn explosion_impulse(base_impulse: f32, distance: f32) -> f32 {
    let d = distance.max(0.0);
    (base_impulse.max(0.0) / (1.0 + d * d)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joint_new_normalizes_gib_limit_above_strength() {
        let j = Joint::new(100.0, 50.0, 1.0, 0.3);
        assert!(j.gib_impulse_limit >= j.joint_strength);
        assert!((j.gib_impulse_limit - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn joint_new_clamps_absorption() {
        let j = Joint::new(100.0, 200.0, 1.0, 2.5);
        assert!((j.absorption - 1.0).abs() < f32::EPSILON);
        let j = Joint::new(100.0, 200.0, 1.0, -0.5);
        assert!((j.absorption - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn detach_at_strength_threshold() {
        let j = Joint::new(80.0, 150.0, 1.0, 0.0);
        let eval = evaluate_joint(j, 100.0);
        assert!(eval.detach);
        assert!(!eval.gib);
    }

    #[test]
    fn gib_at_gib_impulse_limit() {
        let j = Joint::new(80.0, 150.0, 1.0, 0.0);
        let eval = evaluate_joint(j, 200.0);
        assert!(eval.gib);
        assert!(!eval.detach);
    }

    #[test]
    fn no_detach_when_under_threshold() {
        let j = Joint::new(80.0, 150.0, 1.0, 0.0);
        let eval = evaluate_joint(j, 50.0);
        assert!(!eval.detach);
        assert!(!eval.gib);
    }

    #[test]
    fn damage_multiplier_doubles_propagated() {
        let j = Joint::new(80.0, 150.0, 2.0, 0.0);
        let eval = evaluate_joint(j, 10.0);
        assert!((eval.propagated_damage - 20.0).abs() < 1e-3);
    }

    #[test]
    fn absorbed_plus_out_equals_in() {
        let j = Joint::new(80.0, 150.0, 1.0, 0.3);
        let eval = evaluate_joint(j, 100.0);
        assert!((eval.impulse_absorbed + eval.impulse_out - 100.0).abs() < 1e-3);
    }

    #[test]
    fn fall_chain_severs_foot_first() {
        // 5 m fall → v = sqrt(2 * 9.8 * 5) ≈ 9.9 m/s
        // Mass 80 kg → impulse ≈ 792 N — overshoots foot threshold (500 N)
        // but undershoots gib (1500 N) → foot detaches.
        let joints = vec![
            ("foot_left".to_string(), Joint::default_for_zone("foot_left")),
            ("shin_left".to_string(), Joint::default_for_zone("shin_left")),
            ("leg_left".to_string(), Joint::default_for_zone("leg_left")),
        ];
        let chain = fall_impulse_chain(9.9, 80.0, &joints);
        assert_eq!(chain.len(), 3);
        assert!(chain[0].1.detach || chain[0].1.gib);
        // After foot absorbs / detaches, shin sees reduced impulse → survives.
        assert!(!chain[1].1.detach);
        assert!(!chain[1].1.gib);
    }

    #[test]
    fn severance_probability_zero_when_chance_zero() {
        let p = severance_probability(0.0, 80.0, 100.0);
        assert!((p).abs() < f32::EPSILON);
    }

    #[test]
    fn severance_probability_higher_when_joint_weaker() {
        let strong = severance_probability(0.4, 200.0, 100.0); // normalized 1.0 → 0
        let weak = severance_probability(0.4, 20.0, 100.0); // normalized 0.2 → 0.32
        assert!(weak > strong);
    }

    #[test]
    fn severance_roll_fires_when_under() {
        let p = 0.4;
        assert!(severance_roll(0.1, p));
        assert!(!severance_roll(0.5, p));
    }

    #[test]
    fn explosion_impulse_falls_off_with_distance() {
        let near = explosion_impulse(1000.0, 0.0);
        let far = explosion_impulse(1000.0, 5.0);
        assert!(near > far);
        assert!((near - 1000.0).abs() < 1e-3);
    }

    #[test]
    fn explosion_impulse_clamps_negative_distance() {
        let v = explosion_impulse(500.0, -10.0);
        assert!((v - 500.0).abs() < 1e-3);
    }
}
