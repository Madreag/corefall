//! **M14C** — Killcam variants registry.
//!
//! Spec § Crates / modules touched:
//!   cf-killcam MODIFY: New death-cam variants: `heat_penetration` +
//!   `apfsds_through_module`.
//!
//! M14C ships two new variants beyond the default kill cam:
//!   - `heat_penetration` — shaped-charge jet trail + spalling debris cone
//!     (Gherkin-1 § "kill cam plays heat_penetration variant").
//!   - `apfsds_through_module` — glowing rod path + module-by-module
//!     energy decay (Gherkin-3 § "kill cam plays apfsds_through_module
//!     variant").

use serde::{Deserialize, Serialize};

/// Stable id of the M14C HEAT-penetration killcam variant.
pub const HEAT_PENETRATION_VARIANT_ID: KillcamVariantId = KillcamVariantId(1);
/// Stable id of the M14C APFSDS-through-module killcam variant.
pub const APFSDS_THROUGH_MODULE_VARIANT_ID: KillcamVariantId = KillcamVariantId(2);

/// Stable killcam-variant discriminator. The default cam variant uses
/// `KillcamVariantId(0)`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct KillcamVariantId(pub u16);

impl KillcamVariantId {
    /// True if this is the default fallback variant (no specific cam).
    pub fn is_default(self) -> bool {
        self.0 == 0
    }
}

/// `heat_penetration` payload (Gherkin-1 / VAL-M14C-013).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HeatPenetrationPayload {
    /// Ordered module path the jet traversed.
    pub modules: Vec<String>,
    /// Standoff distance at which the warhead detonated (m).
    pub standoff_m: f32,
    /// Effective damage delivered (post-ERA, post-standoff curve).
    pub effective_damage: f32,
}

/// `apfsds_through_module` payload (Gherkin-3 / VAL-M14C-013).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ApfsdsThroughModulePayload {
    /// Per-module energy decay path.
    pub modules: Vec<String>,
    /// Initial kinetic energy at impact (J).
    pub initial_energy_j: f32,
    /// Residual energy after the final module (J).
    pub final_energy_j: f32,
}

/// Killcam variant trigger discriminator — what event produced this cam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillcamVariantTrigger {
    /// `armor.heat_jet_traversed` fired.
    HeatJetTraversed,
    /// `armor.apfsds_long_rod_through` fired.
    ApfsdsLongRodThrough,
    /// Any other death-event (default fallback cam).
    Default,
}

/// Resolved killcam variant payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KillcamVariant {
    /// Default fallback variant (no specific cam payload).
    Default,
    /// M14C HEAT-penetration cam (shaped-charge jet trail).
    HeatPenetration(HeatPenetrationPayload),
    /// M14C APFSDS-through-module cam (glowing long-rod path).
    ApfsdsThroughModule(ApfsdsThroughModulePayload),
}

impl KillcamVariant {
    /// Stable id.
    pub fn id(&self) -> KillcamVariantId {
        match self {
            KillcamVariant::Default => KillcamVariantId(0),
            KillcamVariant::HeatPenetration(_) => HEAT_PENETRATION_VARIANT_ID,
            KillcamVariant::ApfsdsThroughModule(_) => APFSDS_THROUGH_MODULE_VARIANT_ID,
        }
    }

    /// True when this variant is the default fallback (no specific cam).
    pub fn is_default(&self) -> bool {
        matches!(self, KillcamVariant::Default)
    }
}

/// **M14C** § VAL-M14C-013: dispatch a killcam variant based on the
/// trigger event. Returns a non-default variant payload for HEAT /
/// APFSDS impacts; returns the default fallback for everything else.
#[must_use]
pub fn dispatch_variant(
    trigger: KillcamVariantTrigger,
    heat_payload: Option<HeatPenetrationPayload>,
    apfsds_payload: Option<ApfsdsThroughModulePayload>,
) -> KillcamVariant {
    match trigger {
        KillcamVariantTrigger::HeatJetTraversed => {
            KillcamVariant::HeatPenetration(heat_payload.unwrap_or_default())
        }
        KillcamVariantTrigger::ApfsdsLongRodThrough => {
            KillcamVariant::ApfsdsThroughModule(apfsds_payload.unwrap_or_default())
        }
        KillcamVariantTrigger::Default => KillcamVariant::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **VAL-M14C-013**: HEAT trigger returns the heat_penetration variant.
    #[test]
    fn heat_trigger_dispatches_heat_variant() {
        let payload = HeatPenetrationPayload {
            modules: vec!["torso_external".into(), "torso_internal".into()],
            standoff_m: 0.6,
            effective_damage: 1500.0,
        };
        let v = dispatch_variant(KillcamVariantTrigger::HeatJetTraversed, Some(payload.clone()), None);
        assert!(!v.is_default());
        assert_eq!(v.id(), HEAT_PENETRATION_VARIANT_ID);
        assert!(matches!(v, KillcamVariant::HeatPenetration(_)));
    }

    /// **VAL-M14C-013**: APFSDS trigger returns the apfsds_through_module variant.
    #[test]
    fn apfsds_trigger_dispatches_apfsds_variant() {
        let payload = ApfsdsThroughModulePayload {
            modules: vec!["front_plate".into(), "engine".into(), "fuel_tank".into()],
            initial_energy_j: 9_000_000.0,
            final_energy_j: 3_000_000.0,
        };
        let v = dispatch_variant(KillcamVariantTrigger::ApfsdsLongRodThrough, None, Some(payload));
        assert!(!v.is_default());
        assert_eq!(v.id(), APFSDS_THROUGH_MODULE_VARIANT_ID);
        assert!(matches!(v, KillcamVariant::ApfsdsThroughModule(_)));
    }

    /// Default trigger returns the default fallback variant.
    #[test]
    fn default_trigger_returns_default_variant() {
        let v = dispatch_variant(KillcamVariantTrigger::Default, None, None);
        assert!(v.is_default());
        assert!(v.id().is_default());
    }

    /// **VAL-M14C-013**: variant ids are pairwise distinct + non-default
    /// for HEAT / APFSDS.
    #[test]
    fn variant_ids_pairwise_distinct() {
        assert_ne!(HEAT_PENETRATION_VARIANT_ID.0, 0);
        assert_ne!(APFSDS_THROUGH_MODULE_VARIANT_ID.0, 0);
        assert_ne!(HEAT_PENETRATION_VARIANT_ID, APFSDS_THROUGH_MODULE_VARIANT_ID);
    }

    /// VAL-M14C-013 follow-on: dispatching does not panic when payload is missing.
    #[test]
    fn dispatch_does_not_panic_on_missing_payload() {
        let _ = dispatch_variant(KillcamVariantTrigger::HeatJetTraversed, None, None);
        let _ = dispatch_variant(KillcamVariantTrigger::ApfsdsLongRodThrough, None, None);
    }
}
