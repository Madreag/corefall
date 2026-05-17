//! **M6B**: per-actor mass aggregator.
//!
//! Spec-mandated `cf-actor::mass_aggregator` module — feeds the
//! inventory-grid total mass into the M14A "total actor mass" pipeline.
//! M14A extends this aggregator with chassis + held + jetpack + wound
//! mass; M6B locks the inventory contribution + the public surface so
//! M14A can layer on top without API drift.
//!
//! Spec literal (from M14A):
//! > `total_mass = chassis_mass + limb_mass + held_devices_mass + inventory_mass + jetpack_fuel_mass`
//!
//! At M6B every term except `inventory_mass` returns the actor's
//! `ActorState::mass_kg` baseline (chassis or infantry default); M14A
//! replaces those with the live chassis aggregation.

use serde::{Deserialize, Serialize};

use crate::ActorState;

/// **M6B**: per-source breakdown of [`total_mass`]. Surfaced through
/// `observe.actor.mass_breakdown` so HUD + replay tools can render
/// the per-source contribution.
///
/// Every field carries the `_kg` suffix on purpose so callers can grep
/// for "anything-mass-in-kg" across the codebase; that pedantic warning
/// is suppressed here intentionally.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MassBreakdown {
    /// Base actor / chassis mass in kg (infantry default 80, chassis
    /// overrides via `attach_chassis`).
    pub chassis_kg: f32,
    /// Per-limb mass contribution. M6B reserves the slot at 0.0 — M14A
    /// fills with live limb-loss + lodged-pixel mass.
    pub limb_kg: f32,
    /// Held-device mass (currently equipped weapon / tool). M6B
    /// reserves the slot at 0.0 — M14A fills with the held-rifle
    /// summand.
    pub held_kg: f32,
    /// Inventory mass — the M6B contribution. Aggregated from
    /// [`ActorState::inventory_grid`]. Falls back to
    /// [`ActorState::inventory_weight_kg`] when no grid is attached so
    /// legacy actors that still use the M1/M6 slot model report a
    /// non-zero inventory contribution.
    pub inventory_kg: f32,
    /// Jetpack fuel mass. M6B reserves the slot at 0.0 — M14A fills.
    pub jetpack_fuel_kg: f32,
    /// Wound mass (lodged pixels). M6B reserves the slot at 0.0 — M14A
    /// fills.
    pub wound_kg: f32,
}

impl MassBreakdown {
    /// Sum every contribution to a single total in kg.
    pub fn total(&self) -> f32 {
        self.chassis_kg + self.limb_kg + self.held_kg + self.inventory_kg + self.jetpack_fuel_kg + self.wound_kg
    }
}

/// **M6B**: compute the per-source mass breakdown for `actor`. The
/// inventory contribution comes from
/// [`ActorState::inventory_grid_total_mass_kg`], with a fallback to the
/// legacy `inventory_weight_kg` when no grid is attached.
pub fn breakdown(actor: &ActorState) -> MassBreakdown {
    let inventory_kg = actor.inventory_grid_total_mass_kg();
    MassBreakdown {
        chassis_kg: actor.mass_kg.max(0.0),
        limb_kg: 0.0,
        held_kg: 0.0,
        inventory_kg,
        jetpack_fuel_kg: 0.0,
        wound_kg: 0.0,
    }
}

/// **M6B**: aggregated total actor mass in kg. Equivalent to
/// `breakdown(actor).total()`. M14A overrides this function (or
/// extends [`breakdown`]) to fill the currently-reserved slots.
pub fn total_mass(actor: &ActorState) -> f32 {
    breakdown(actor).total()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{ActorId, ActorState, Inventory, Vec2};

    fn make_actor() -> ActorState {
        let inv = Inventory::with_rifle("rifle_m1_default");
        ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv)
    }

    #[test]
    fn empty_actor_mass_is_chassis_baseline() {
        let actor = make_actor();
        let bd = breakdown(&actor);
        // Default infantry = 80 kg chassis baseline; no inventory yet.
        assert!((bd.chassis_kg - 80.0).abs() < 1e-6);
        assert!((bd.inventory_kg - 0.0).abs() < 1e-6);
        assert!((bd.total() - 80.0).abs() < 1e-6);
    }

    #[test]
    fn inventory_grid_contributes_to_total() {
        let mut actor = make_actor();
        actor.inventory_grid_attach();
        let grid = actor.inventory_grid_mut().expect("grid attached");
        grid.add_top_level("rifle_m1", 1, 0.0);
        let bd = breakdown(&actor);
        assert!((bd.inventory_kg - 3.5).abs() < 1e-6);
        assert!((bd.total() - 83.5).abs() < 1e-6);
    }

    #[test]
    fn legacy_inventory_weight_falls_back_when_grid_absent() {
        let mut actor = make_actor();
        actor.inventory_grid = None;
        actor.inventory_weight_kg = 12.0;
        let bd = breakdown(&actor);
        assert!((bd.inventory_kg - 12.0).abs() < 1e-6);
        assert!((bd.total() - 92.0).abs() < 1e-6);
    }
}
