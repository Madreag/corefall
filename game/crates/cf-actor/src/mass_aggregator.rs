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
///
/// **M14A** fills the held / jetpack-fuel / wound slots so `total()`
/// matches the per-source breakdown for the HUD MASS line.
pub fn breakdown(actor: &ActorState) -> MassBreakdown {
    let inventory_kg = actor.inventory_grid_total_mass_kg();

    // **M14A** § "held_devices_mass": currently-equipped rifle / pistol.
    // M6B's inventory grid already accounts for stored items; here we add
    // the *equipped* weapon if it's not double-counted in the grid.
    let held_kg = if actor.inventory_grid.is_some() {
        0.0 // grid already includes equipped + stored
    } else {
        // Legacy actors: rifle ≈ 3.5 kg if a rifle slot is held.
        if actor.inventory.rifle_slot().is_some() {
            3.5
        } else {
            0.0
        }
    };

    // **M14A** § "Jetpack fuel mass decreases as fuel burns".
    let jetpack_fuel_kg = actor.jetpack.as_ref().map_or(0.0, |j| j.fuel_mass_kg());
    let jetpack_dry_kg = actor.jetpack.as_ref().map_or(0.0, |j| j.dry_mass_kg);

    // **M14A** § "Wound mass from lodged pixels".
    let wound_kg = actor.wound_mass_kg.max(0.0);

    MassBreakdown {
        chassis_kg: actor.mass_kg.max(0.0) + jetpack_dry_kg,
        limb_kg: 0.0,
        held_kg,
        inventory_kg,
        jetpack_fuel_kg,
        wound_kg,
    }
}

/// **M14A** § "Mass aggregation system" — aggregated total actor mass in kg.
pub fn total_mass(actor: &ActorState) -> f32 {
    breakdown(actor).total()
}

/// **M14A** § "Mass factor" — walk speed multiplier from total mass.
///
/// `(BASELINE_MASS_KG / total_mass).clamp(MASS_FACTOR_MIN, MASS_FACTOR_MAX)`.
pub fn mass_factor(actor: &ActorState) -> f32 {
    const BASELINE_MASS_KG: f32 = 80.0;
    const MASS_FACTOR_MIN: f32 = 0.25;
    const MASS_FACTOR_MAX: f32 = 1.2;
    let total = total_mass(actor).max(1.0);
    (BASELINE_MASS_KG / total).clamp(MASS_FACTOR_MIN, MASS_FACTOR_MAX)
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
        // Default infantry = 80 kg chassis baseline; legacy rifle adds 3.5 kg held.
        assert!((bd.chassis_kg - 80.0).abs() < 1e-6);
        assert!((bd.inventory_kg - 0.0).abs() < 1e-6);
        // **M14A** legacy path: held rifle (3.5 kg) counts as held device.
        assert!((bd.held_kg - 3.5).abs() < 1e-6);
        assert!((bd.total() - 83.5).abs() < 1e-6);
    }

    #[test]
    fn inventory_grid_contributes_to_total() {
        let mut actor = make_actor();
        actor.inventory_grid_attach();
        let grid = actor.inventory_grid_mut().expect("grid attached");
        grid.add_top_level("rifle_m1", 1, 0.0);
        let bd = breakdown(&actor);
        // Grid mode: held_kg = 0 (grid covers all); only inventory_kg counts.
        assert!((bd.inventory_kg - 3.5).abs() < 1e-6);
        assert!((bd.held_kg - 0.0).abs() < 1e-6);
        assert!((bd.total() - 83.5).abs() < 1e-6);
    }

    #[test]
    fn legacy_inventory_weight_falls_back_when_grid_absent() {
        let mut actor = make_actor();
        actor.inventory_grid = None;
        actor.inventory_weight_kg = 12.0;
        let bd = breakdown(&actor);
        assert!((bd.inventory_kg - 12.0).abs() < 1e-6);
        // **M14A** held rifle: 3.5 kg added separately.
        assert!((bd.held_kg - 3.5).abs() < 1e-6);
        assert!((bd.total() - 95.5).abs() < 1e-6);
    }

    #[test]
    fn jetpack_fuel_contributes_to_total() {
        let mut actor = make_actor();
        actor.jetpack = Some(cf_equipment::Jetpack::standard_powered_armor());
        let bd = breakdown(&actor);
        // Jetpack dry mass (5 kg) folds into chassis; fuel mass surfaces separately.
        assert!(bd.jetpack_fuel_kg > 10.0);
        // chassis_kg = 80 + dry_mass 5 = 85
        assert!((bd.chassis_kg - 85.0).abs() < 1e-6);
    }

    #[test]
    fn mass_factor_clamped_at_quarter_for_heavy_actor() {
        let mut actor = make_actor();
        actor.mass_kg = 380.0;
        let mf = mass_factor(&actor);
        assert!((mf - 0.25).abs() < 1e-3, "heavy actor should clamp to 0.25, got {mf}");
    }
}
