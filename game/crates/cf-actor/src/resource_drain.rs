//! **M14A** § "Per-origin resource overlay" — per-stride caloric / power /
//! oil / blood depletion. Wires into M17's origin reaction matrix; M14A
//! adds the per-stride drain rate + warning thresholds.

use crate::{ActorState, ResourceAccumulators};

/// Per-stride drain rate by origin.
pub fn drain_per_stride(origin_id: &str) -> ResourceAccumulators {
    let mut acc = ResourceAccumulators::default();
    match origin_id {
        "human" | "human_marine" => {
            acc.caloric_energy = -0.05;
            acc.oxygen_supply = -0.001;
        }
        "robot" | "synth" => {
            acc.power = -0.02;
            acc.battery_charge = -0.02;
            acc.heat = 0.005;
        }
        "android" | "hybrid" => {
            acc.caloric_energy = -0.025;
            acc.power = -0.01;
            acc.battery_charge = -0.01;
        }
        _ => {
            acc.caloric_energy = -0.05;
        }
    }
    acc
}

/// Apply a stride's worth of drain. Returns whether any resource crossed
/// a warning threshold this stride.
pub fn apply_stride_drain(actor: &mut ActorState) -> bool {
    let drain = drain_per_stride(&actor.origin_id);
    let prev = actor.resources;
    actor.resources.caloric_energy += drain.caloric_energy;
    actor.resources.power += drain.power;
    actor.resources.battery_charge += drain.battery_charge;
    actor.resources.heat += drain.heat;
    actor.resources.oxygen_supply += drain.oxygen_supply;

    // Threshold checks: caloric < 30 = warning band; battery < 20 = warning.
    let crossed_caloric = prev.caloric_energy >= 30.0 && actor.resources.caloric_energy < 30.0;
    let crossed_battery = prev.battery_charge >= 20.0 && actor.resources.battery_charge < 20.0;
    crossed_caloric || crossed_battery
}

/// Walk-speed modifier from depleted resources (caloric < 30, battery < 20).
pub fn resource_speed_mult(actor: &ActorState) -> f32 {
    let mut mult = 1.0;
    if matches!(actor.origin_id.as_str(), "human" | "human_marine" | "android" | "hybrid")
        && actor.resources.caloric_energy < 30.0
    {
        mult *= 0.85;
    }
    if matches!(actor.origin_id.as_str(), "robot" | "synth" | "android" | "hybrid")
        && actor.resources.battery_charge < 20.0
    {
        mult *= 0.7;
    }
    mult
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{ActorId, Inventory, Vec2};

    #[test]
    fn human_drains_caloric_only() {
        let d = drain_per_stride("human");
        assert!(d.caloric_energy < 0.0);
        assert!(d.power >= 0.0);
    }

    #[test]
    fn robot_drains_power_only() {
        let d = drain_per_stride("robot");
        assert_eq!(d.caloric_energy, 0.0);
        assert!(d.power < 0.0);
    }

    #[test]
    fn android_drains_both() {
        let d = drain_per_stride("android");
        assert!(d.caloric_energy < 0.0);
        assert!(d.power < 0.0);
    }

    #[test]
    fn stride_drain_modifies_resources() {
        let mut a =
            ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, Inventory::with_rifle("rifle_m1_default"));
        a.resources.caloric_energy = 100.0;
        let crossed = apply_stride_drain(&mut a);
        assert!((a.resources.caloric_energy - 99.95).abs() < 1e-3);
        assert!(!crossed); // 100 → 99.95 doesn't cross 30
    }
}
