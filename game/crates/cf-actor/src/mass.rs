//! **M14A** § "Files — `cf-actor::mass.rs` (NEW)" — spec-canonical mass
//! aggregation module. Re-exports + extends `mass_aggregator` with the
//! `mass_factor`, dirty-flag helpers, and the live-recalc event hooks the
//! spec names directly.

pub use crate::m14a_constants::{BASELINE_MASS_KG, MASS_FACTOR_MAX_CLAMP, MASS_FACTOR_MIN_CLAMP};
pub use crate::mass_aggregator::{breakdown, mass_factor, total_mass, MassBreakdown};

use crate::ActorState;

/// cached mass dirty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassDirtyReason {
    ArmorZoneDestroyed,
    Repaired,
    ItemPickedUp,
    ItemDropped,
    WeaponSwapCompleted,
    Salvaged,
    JetpackFuelChanged,
    WoundAdded,
    GearDroppedByLimbLoss,
}

impl MassDirtyReason {
    pub fn as_str(self) -> &'static str {
        match self {
            MassDirtyReason::ArmorZoneDestroyed => "chassis.armor_zone_destroyed",
            MassDirtyReason::Repaired => "chassis.repaired",
            MassDirtyReason::ItemPickedUp => "equipment.item_picked_up",
            MassDirtyReason::ItemDropped => "equipment.item_dropped",
            MassDirtyReason::WeaponSwapCompleted => "equipment.weapon_swap_completed",
            MassDirtyReason::Salvaged => "chassis.salvaged",
            MassDirtyReason::JetpackFuelChanged => "actor.jetpack_fuel_changed",
            MassDirtyReason::WoundAdded => "actor.wound_added",
            MassDirtyReason::GearDroppedByLimbLoss => "actor.gear_dropped_by_limb_loss",
        }
    }
}

/// surface the reason for replay events.
pub fn invalidate_mass(actor: &mut ActorState, _reason: MassDirtyReason) {
    actor.mark_mass_dirty();
}

/// mass to the actor's wound mass pool. Marks mass dirty.
pub fn add_wound_pixel(actor: &mut ActorState) {
    actor.wound_mass_kg += crate::m14a_constants::WOUND_PIXEL_MASS_KG;
    invalidate_mass(actor, MassDirtyReason::WoundAdded);
}

pub fn jump_velocity_from_impulse(jump_impulse_n_s: f32, total_mass_kg: f32) -> f32 {
    jump_impulse_n_s / total_mass_kg.max(1e-3)
}

pub fn fall_damage(total_mass_kg: f32, impact_velocity_m_per_s: f32) -> f32 {
    0.5 * total_mass_kg.max(0.0) * impact_velocity_m_per_s * impact_velocity_m_per_s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorId, Inventory, Vec2};

    #[test]
    fn add_wound_pixel_increments_mass() {
        let mut a = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, Inventory::with_rifle("rifle"));
        let before = a.wound_mass_kg;
        add_wound_pixel(&mut a);
        assert!(a.wound_mass_kg > before);
        assert!(a.total_mass_dirty);
    }

    #[test]
    fn fall_damage_scales_with_mass_squared_velocity() {
        let light = fall_damage(80.0, 20.0);
        let heavy = fall_damage(380.0, 20.0);
        assert!(heavy > light * 4.0);
    }

    #[test]
    fn jump_velocity_inversely_scales_with_mass() {
        let v_light = jump_velocity_from_impulse(800.0, 80.0);
        let v_heavy = jump_velocity_from_impulse(800.0, 200.0);
        assert!(v_light > v_heavy * 2.0);
    }
}
