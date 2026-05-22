//! **M14**: per-organ + per-circuit internal damage routing per the
//! M9 contract.
//!
//! When a projectile's passthrough damage exceeds the `heavy_damage_threshold`
//! (5 HP per spec § "Per-organ internal damage routing"), this crate
//! deterministically selects an organ (humans / androids) or a circuit
//! (robots) weighted by the hit zone's proximity to the organ / circuit's
//! mount point, then applies `passthrough * internal_damage_modifier` to
//! the selected organ.
//!
//! Determinism: all routing is pure (state in → state out). The engine
//! supplies a `[0, 1)` RNG draw from its seeded RNG for the weighted
//! selection. No clock; no `thread_rng`.

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
    clippy::cast_lossless,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use serde::{Deserialize, Serialize};

pub mod heat_transfer;
pub use heat_transfer::{body_temp_delta_per_tick, ThermalContactEffect, FOOT_CONTACT_AREA_M2};

/// Heavy-damage threshold per spec § "Per-organ internal damage routing"
/// — internal damage rolls only when passthrough_damage > 5.
pub const HEAVY_DAMAGE_THRESHOLD: f32 = 5.0;

/// Default internal-damage modifier per spec § "Per-organ internal damage
/// routing". `passthrough_damage * 0.6` is applied to the selected organ
/// when an internal-shock roll succeeds.
pub const DEFAULT_INTERNAL_DAMAGE_MODIFIER: f32 = 0.6;

/// routing path (organ graph vs circuit graph).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalGraphKind {
    /// 15-organ humanoid graph (humans + androids).
    Humanoid,
    /// 12-circuit robot graph.
    Robot,
}

/// `internal_organ_damaged.json` schema enum.
///
/// Names mirror the schema literal so producer + validator agree.
pub const HUMANOID_ORGANS: &[&str] = &[
    "brain",
    "eyes_left",
    "eyes_right",
    "ears_left",
    "ears_right",
    "heart",
    "lungs_left",
    "lungs_right",
    "liver",
    "kidneys_left",
    "kidneys_right",
    "spine",
    "stomach",
    "intestines",
    "pancreas",
];

/// `internal_circuit_damaged.json` schema enum.
pub const ROBOT_CIRCUITS: &[&str] = &[
    "power_core",
    "cpu",
    "sensor_array",
    "motor_controller_left_arm",
    "motor_controller_right_arm",
    "motor_controller_left_leg",
    "motor_controller_right_leg",
    "hydraulic_pump",
    "coolant_pump",
    "oil_reservoir",
    "fuel_tank",
    "comm_relay",
];

pub fn organ_kind(organ_id: &str) -> &'static str {
    match organ_id {
        "brain" | "spine" => "central_nervous",
        "eyes_left" | "eyes_right" | "ears_left" | "ears_right" => "sensory",
        "heart" => "circulatory",
        "lungs_left" | "lungs_right" => "respiratory",
        "liver" | "stomach" | "intestines" | "pancreas" => "digestive",
        "kidneys_left" | "kidneys_right" => "renal",
        _ => "vital",
    }
}

pub fn circuit_kind(circuit_id: &str) -> &'static str {
    match circuit_id {
        "power_core" | "fuel_tank" | "oil_reservoir" => "power",
        "motor_controller_left_arm" | "motor_controller_right_arm" => "actuator_arm",
        "motor_controller_left_leg" | "motor_controller_right_leg" => "actuator_leg",
        "hydraulic_pump" | "coolant_pump" => "support",
        _ => "control",
    }
}

/// to be selected. The engine builds the candidate list from the hit zone
/// + the graph, then [`select_weighted`] picks one deterministically.
#[derive(Debug, Clone, Copy)]
pub struct WeightedCandidate {
    pub id: &'static str,
    pub weight: f32,
}

/// stance-specific zone AABB table" — the hit zone determines which
/// organs are statistically near the impact point.
#[must_use]
pub fn humanoid_organ_weights(hit_zone: &str) -> Vec<WeightedCandidate> {
    let mut out: Vec<WeightedCandidate> = Vec::with_capacity(HUMANOID_ORGANS.len());
    let push = |out: &mut Vec<WeightedCandidate>, id: &'static str, w: f32| {
        if w > 0.0 {
            out.push(WeightedCandidate { id, weight: w });
        }
    };
    match hit_zone {
        "head" => {
            push(&mut out, "brain", 5.0);
            push(&mut out, "eyes_left", 2.0);
            push(&mut out, "eyes_right", 2.0);
            push(&mut out, "ears_left", 1.5);
            push(&mut out, "ears_right", 1.5);
            push(&mut out, "spine", 0.8);
        }
        "torso" | "chest" => {
            push(&mut out, "heart", 4.0);
            push(&mut out, "lungs_left", 3.0);
            push(&mut out, "lungs_right", 3.0);
            push(&mut out, "liver", 2.0);
            push(&mut out, "spine", 1.5);
            push(&mut out, "stomach", 1.5);
        }
        "abdomen" => {
            push(&mut out, "liver", 3.0);
            push(&mut out, "stomach", 3.0);
            push(&mut out, "intestines", 3.0);
            push(&mut out, "pancreas", 2.0);
            push(&mut out, "kidneys_left", 2.0);
            push(&mut out, "kidneys_right", 2.0);
            push(&mut out, "spine", 1.5);
        }
        "arm_left" | "forearm_left" | "hand_left" => {
            push(&mut out, "lungs_left", 0.7);
            push(&mut out, "heart", 0.4);
        }
        "arm_right" | "forearm_right" | "hand_right" => {
            push(&mut out, "lungs_right", 0.7);
            push(&mut out, "heart", 0.4);
        }
        "leg_left" | "shin_left" | "foot_left" => {
            push(&mut out, "kidneys_left", 0.6);
            push(&mut out, "spine", 0.2);
        }
        "leg_right" | "shin_right" | "foot_right" => {
            push(&mut out, "kidneys_right", 0.6);
            push(&mut out, "spine", 0.2);
        }
        _ => {
            push(&mut out, "heart", 1.0);
            push(&mut out, "lungs_left", 1.0);
            push(&mut out, "lungs_right", 1.0);
            push(&mut out, "liver", 1.0);
        }
    }
    out
}

#[must_use]
pub fn robot_circuit_weights(hit_zone: &str) -> Vec<WeightedCandidate> {
    let mut out: Vec<WeightedCandidate> = Vec::with_capacity(ROBOT_CIRCUITS.len());
    let push = |out: &mut Vec<WeightedCandidate>, id: &'static str, w: f32| {
        if w > 0.0 {
            out.push(WeightedCandidate { id, weight: w });
        }
    };
    match hit_zone {
        "head" => {
            push(&mut out, "cpu", 5.0);
            push(&mut out, "sensor_array", 3.0);
            push(&mut out, "comm_relay", 1.5);
        }
        "torso" | "chest" => {
            push(&mut out, "power_core", 4.0);
            push(&mut out, "cpu", 2.0);
            push(&mut out, "hydraulic_pump", 2.0);
            push(&mut out, "coolant_pump", 1.5);
            push(&mut out, "comm_relay", 1.0);
        }
        "abdomen" => {
            push(&mut out, "oil_reservoir", 3.0);
            push(&mut out, "fuel_tank", 3.0);
            push(&mut out, "hydraulic_pump", 2.0);
            push(&mut out, "coolant_pump", 2.0);
        }
        "arm_left" | "forearm_left" | "hand_left" => {
            push(&mut out, "motor_controller_left_arm", 4.0);
            push(&mut out, "hydraulic_pump", 0.7);
        }
        "arm_right" | "forearm_right" | "hand_right" => {
            push(&mut out, "motor_controller_right_arm", 4.0);
            push(&mut out, "hydraulic_pump", 0.7);
        }
        "leg_left" | "shin_left" | "foot_left" => {
            push(&mut out, "motor_controller_left_leg", 4.0);
            push(&mut out, "hydraulic_pump", 0.6);
        }
        "leg_right" | "shin_right" | "foot_right" => {
            push(&mut out, "motor_controller_right_leg", 4.0);
            push(&mut out, "hydraulic_pump", 0.6);
        }
        _ => {
            push(&mut out, "power_core", 1.0);
            push(&mut out, "cpu", 1.0);
        }
    }
    out
}

/// draw from the engine's seeded RNG. Returns `None` only when the
/// candidate list is empty.
///
/// Algorithm: linear scan + cumulative weight prefix sum (O(n)). For
/// the sub-15 organ graph this is fastest and exactly deterministic.
#[must_use]
pub fn select_weighted(candidates: &[WeightedCandidate], rng_roll: f32) -> Option<&'static str> {
    if candidates.is_empty() {
        return None;
    }
    let total: f32 = candidates.iter().map(|c| c.weight.max(0.0)).sum();
    if total <= f32::EPSILON {
        return Some(candidates[0].id);
    }
    let r = rng_roll.clamp(0.0, 0.999_999) * total;
    let mut accum = 0.0_f32;
    for c in candidates {
        accum += c.weight.max(0.0);
        if r < accum {
            return Some(c.id);
        }
    }
    Some(candidates[candidates.len() - 1].id)
}

/// id + applied damage when the heavy-damage threshold was crossed, else
/// `None`.
///
///   - When passthrough_damage > heavy_damage_threshold (5 HP):
///     - Roll internal_shock_probability based on (passthrough, impulse, ap_factor)
///     - Select random organ/circuit weighted by hit_zone proximity to organ's mount_point
///     - Apply (passthrough * internal_damage_modifier) to selected organ
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InternalDamageDecision {
    pub target_id: &'static str,
    pub applied_damage: f32,
    pub graph_kind: InternalGraphKind,
}

#[must_use]
pub fn route_internal_damage(
    graph_kind: InternalGraphKind,
    hit_zone: &str,
    passthrough_damage: f32,
    rng_roll: f32,
) -> Option<InternalDamageDecision> {
    if passthrough_damage <= HEAVY_DAMAGE_THRESHOLD {
        return None;
    }
    let candidates = match graph_kind {
        InternalGraphKind::Humanoid => humanoid_organ_weights(hit_zone),
        InternalGraphKind::Robot => robot_circuit_weights(hit_zone),
    };
    let target = select_weighted(&candidates, rng_roll)?;
    Some(InternalDamageDecision {
        target_id: target,
        applied_damage: passthrough_damage * DEFAULT_INTERNAL_DAMAGE_MODIFIER,
        graph_kind,
    })
}

/// organs / circuits in the affected radius. Returns the selected ids in
/// the order they were drawn. Uses `rng_rolls` as a slice of independent
/// `[0, 1)` draws from the engine RNG; consumes up to 3 rolls.
#[must_use]
pub fn route_explosion_internal_damage(
    graph_kind: InternalGraphKind,
    hit_zone: &str,
    passthrough_damage: f32,
    rng_rolls: &[f32],
) -> Vec<InternalDamageDecision> {
    if passthrough_damage <= HEAVY_DAMAGE_THRESHOLD {
        return Vec::new();
    }
    let mut candidates = match graph_kind {
        InternalGraphKind::Humanoid => humanoid_organ_weights(hit_zone),
        InternalGraphKind::Robot => robot_circuit_weights(hit_zone),
    };
    let mut decisions: Vec<InternalDamageDecision> = Vec::with_capacity(3);
    for &roll in rng_rolls.iter().take(3) {
        if candidates.is_empty() {
            break;
        }
        if let Some(target) = select_weighted(&candidates, roll) {
            decisions.push(InternalDamageDecision {
                target_id: target,
                applied_damage: passthrough_damage * DEFAULT_INTERNAL_DAMAGE_MODIFIER,
                graph_kind,
            });
            candidates.retain(|c| c.id != target);
        }
    }
    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organ_weights_head_favors_brain() {
        let weights = humanoid_organ_weights("head");
        let brain = weights.iter().find(|c| c.id == "brain").unwrap();
        let eye = weights.iter().find(|c| c.id == "eyes_left").unwrap();
        assert!(brain.weight > eye.weight);
    }

    #[test]
    fn organ_weights_torso_favors_heart() {
        let weights = humanoid_organ_weights("torso");
        let heart = weights.iter().find(|c| c.id == "heart").unwrap();
        assert!(heart.weight > 0.0);
        // brain is not in torso weights (different bucket).
        assert!(weights.iter().all(|c| c.id != "brain"));
    }

    #[test]
    fn organ_weights_arm_propagates_to_chest() {
        let weights = humanoid_organ_weights("arm_left");
        // arm hits should route to lungs (chest-adjacent) at low weight.
        assert!(weights.iter().any(|c| c.id == "lungs_left"));
    }

    #[test]
    fn select_weighted_returns_some_when_nonempty() {
        let candidates = humanoid_organ_weights("head");
        let v = select_weighted(&candidates, 0.5);
        assert!(v.is_some());
    }

    #[test]
    fn select_weighted_zero_roll_picks_first() {
        let candidates = humanoid_organ_weights("head");
        let v = select_weighted(&candidates, 0.0).unwrap();
        assert_eq!(v, "brain");
    }

    #[test]
    fn select_weighted_returns_none_on_empty() {
        let v = select_weighted(&[], 0.5);
        assert!(v.is_none());
    }

    #[test]
    fn route_below_threshold_returns_none() {
        let d = route_internal_damage(InternalGraphKind::Humanoid, "torso", 3.0, 0.5);
        assert!(d.is_none());
    }

    #[test]
    fn route_above_threshold_returns_some() {
        let d = route_internal_damage(InternalGraphKind::Humanoid, "torso", 15.0, 0.5).unwrap();
        assert!(d.applied_damage > 0.0);
        // 15.0 * 0.6 = 9.0
        assert!((d.applied_damage - 9.0).abs() < 1e-3);
        assert_eq!(d.graph_kind, InternalGraphKind::Humanoid);
    }

    #[test]
    fn route_robot_picks_circuit() {
        let d = route_internal_damage(InternalGraphKind::Robot, "torso", 20.0, 0.1).unwrap();
        // Robot circuit catalog — power_core / cpu / hydraulic_pump etc.
        assert!(ROBOT_CIRCUITS.contains(&d.target_id));
    }

    #[test]
    fn explosion_routing_picks_up_to_three() {
        let rolls = vec![0.0, 0.3, 0.6, 0.9];
        let decisions = route_explosion_internal_damage(InternalGraphKind::Humanoid, "torso", 20.0, &rolls);
        assert_eq!(decisions.len(), 3);
        // No duplicates (we retain candidates.retain to remove picked).
        let ids: Vec<&str> = decisions.iter().map(|d| d.target_id).collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j]);
            }
        }
    }

    #[test]
    fn explosion_routing_empty_below_threshold() {
        let rolls = vec![0.0, 0.5];
        let decisions = route_explosion_internal_damage(InternalGraphKind::Humanoid, "torso", 1.0, &rolls);
        assert!(decisions.is_empty());
    }

    #[test]
    fn organ_kind_returns_classification() {
        assert_eq!(organ_kind("brain"), "central_nervous");
        assert_eq!(organ_kind("heart"), "circulatory");
        assert_eq!(organ_kind("liver"), "digestive");
        assert_eq!(organ_kind("kidneys_left"), "renal");
    }

    #[test]
    fn circuit_kind_returns_classification() {
        assert_eq!(circuit_kind("power_core"), "power");
        assert_eq!(circuit_kind("cpu"), "control");
        assert_eq!(circuit_kind("motor_controller_left_arm"), "actuator_arm");
    }
}
