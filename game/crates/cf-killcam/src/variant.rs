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
/// Stable id of the M14D projectile-pair-contact killcam variant.
/// Excluded from the killcam queue by default — surfaced only when the
/// per-player `replay_intercepts` setting is opted in.
pub const PROJECTILE_PAIR_CONTACT_VARIANT_ID: KillcamVariantId = KillcamVariantId(3);

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

/// **M14D** § projectile-pair contact killcam payload (VAL-M14D-019).
/// Surfaced only when the per-player `replay_intercepts` setting is
/// opted in; the default-off behaviour means the killcam queue is
/// empty for these events on default settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProjectilePairContactPayload {
    /// Canonical lower-ordered projectile id.
    pub a_id: u64,
    /// Canonical higher-ordered projectile id.
    pub b_id: u64,
    /// Outcome discriminator copied from
    /// `collision.projectile_pair_contact.outcome`.
    pub outcome: String,
    /// World coordinates of the swept-vs-swept intercept point.
    pub intercept_point: [f32; 2],
}

/// Killcam variant trigger discriminator — what event produced this cam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillcamVariantTrigger {
    /// `armor.heat_jet_traversed` fired.
    HeatJetTraversed,
    /// `armor.apfsds_long_rod_through` fired.
    ApfsdsLongRodThrough,
    /// **M14D** § `collision.projectile_pair_contact` fired. Killcam
    /// excludes this trigger from the candidate queue by default; the
    /// per-player `replay_intercepts` setting opts inclusion.
    ProjectilePairContact,
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
    /// **M14D** § projectile-pair contact cam. Only added to the
    /// killcam queue when the per-player `replay_intercepts` setting is
    /// `true` (default is `false` per VAL-M14D-019).
    ProjectilePairContact(ProjectilePairContactPayload),
}

impl KillcamVariant {
    /// Stable id.
    pub fn id(&self) -> KillcamVariantId {
        match self {
            KillcamVariant::Default => KillcamVariantId(0),
            KillcamVariant::HeatPenetration(_) => HEAT_PENETRATION_VARIANT_ID,
            KillcamVariant::ApfsdsThroughModule(_) => APFSDS_THROUGH_MODULE_VARIANT_ID,
            KillcamVariant::ProjectilePairContact(_) => PROJECTILE_PAIR_CONTACT_VARIANT_ID,
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
        // **M14D** trigger handled by `dispatch_pair_contact_variant`
        // which threads the per-player `replay_intercepts` opt-in. The
        // default surface here returns the fallback so the M14C surface
        // doesn't accidentally surface projectile-pair cams when called
        // without the gate.
        KillcamVariantTrigger::ProjectilePairContact => KillcamVariant::Default,
        KillcamVariantTrigger::Default => KillcamVariant::Default,
    }
}

/// **M14D** § VAL-M14D-019 / VAL-CROSS-004: dispatch a killcam variant
/// for a `collision.projectile_pair_contact` event. Returns the
/// non-default `ProjectilePairContact` variant ONLY when the player's
/// `replay_intercepts` setting is `true`; otherwise returns the default
/// fallback so the killcam queue stays empty for these events.
#[must_use]
pub fn dispatch_pair_contact_variant(
    payload: ProjectilePairContactPayload,
    replay_intercepts: bool,
) -> KillcamVariant {
    if replay_intercepts {
        KillcamVariant::ProjectilePairContact(payload)
    } else {
        KillcamVariant::Default
    }
}

/// **M14D** § VAL-M14D-019 default value: per-player `replay_intercepts`
/// defaults to `false` (killcam excludes projectile-pair contacts by
/// default; player can opt in via settings).
pub const DEFAULT_REPLAY_INTERCEPTS: bool = false;

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

    /// **VAL-M14D-019**: default `replay_intercepts` is false. With
    /// the default off, dispatching a pair contact returns the default
    /// fallback variant (queue stays empty for these events).
    #[test]
    fn pair_contact_dispatch_defaults_to_empty_on_default_setting() {
        assert!(!DEFAULT_REPLAY_INTERCEPTS);
        let payload = ProjectilePairContactPayload {
            a_id: 1,
            b_id: 2,
            outcome: "fuze_triggered".to_string(),
            intercept_point: [10.0, 0.0],
        };
        let v = dispatch_pair_contact_variant(payload, DEFAULT_REPLAY_INTERCEPTS);
        assert!(v.is_default(), "default setting must produce default variant");
    }

    /// **VAL-M14D-019**: flipping `replay_intercepts` to true surfaces
    /// the non-default `ProjectilePairContact` variant with the payload.
    #[test]
    fn pair_contact_dispatch_surfaces_variant_when_opted_in() {
        let payload = ProjectilePairContactPayload {
            a_id: 3,
            b_id: 4,
            outcome: "kinetic_deflect".to_string(),
            intercept_point: [25.0, 100.0],
        };
        let v = dispatch_pair_contact_variant(payload, true);
        assert!(!v.is_default());
        assert_eq!(v.id(), PROJECTILE_PAIR_CONTACT_VARIANT_ID);
        match v {
            KillcamVariant::ProjectilePairContact(p) => {
                assert_eq!(p.a_id, 3);
                assert_eq!(p.b_id, 4);
                assert_eq!(p.outcome, "kinetic_deflect");
            }
            _ => panic!("expected ProjectilePairContact variant"),
        }
    }

    /// **VAL-CROSS-004**: a stream of only `ProjectilePairContact` triggers
    /// with `replay_intercepts=false` produces an empty queued-variant set.
    #[test]
    fn pair_contact_stream_with_default_setting_produces_empty_queue() {
        let payloads = [
            ProjectilePairContactPayload {
                a_id: 7,
                b_id: 11,
                outcome: "aps_intercept".to_string(),
                intercept_point: [80.0, 0.0],
            },
            ProjectilePairContactPayload {
                a_id: 13,
                b_id: 17,
                outcome: "fuze_triggered".to_string(),
                intercept_point: [120.0, 30.0],
            },
        ];
        let queued: Vec<KillcamVariant> = payloads
            .iter()
            .map(|p| dispatch_pair_contact_variant(p.clone(), false))
            .filter(|v| !v.is_default())
            .collect();
        assert!(queued.is_empty(), "default setting must produce empty queue");
    }

    /// VAL-M14D-019 / VAL-CROSS-004: variant id is pairwise distinct
    /// from M14C variants (HEAT / APFSDS) so future replay viewers can
    /// filter by id.
    #[test]
    fn pair_contact_variant_id_is_distinct() {
        assert_ne!(PROJECTILE_PAIR_CONTACT_VARIANT_ID, HEAT_PENETRATION_VARIANT_ID);
        assert_ne!(PROJECTILE_PAIR_CONTACT_VARIANT_ID, APFSDS_THROUGH_MODULE_VARIANT_ID);
        assert_ne!(PROJECTILE_PAIR_CONTACT_VARIANT_ID.0, 0);
    }
}
