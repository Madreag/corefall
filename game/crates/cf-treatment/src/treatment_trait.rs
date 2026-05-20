//! **M14H** § Treatment trait + registry.
//!
//! Per the spec § "Notes for the implementer": "Treatment producers all
//! share the same `Treatment` trait; per-item RON file declares
//! applicability + effect".
//!
//! The trait gives downstream consumers (M14H apply state machine, AI
//! medic doctrine, replay viewer) a single abstraction over every
//! producer's apply window, tool requirement, skill requirement, risk
//! surface, and outcome effect.

use std::collections::BTreeMap;

use crate::effects::{effect_for, TreatmentEffect};
use crate::producers::{
    treatment_spec, RiskKind, SkillRequirement, ToolRequirement, TreatmentKind, TreatmentSpec,
};

/// **M14H** § canonical Treatment trait. Every producer implements this
/// uniformly; the catalog ([`TreatmentRegistry`]) exposes them as a
/// single map.
pub trait Treatment: Send + Sync + std::fmt::Debug {
    /// The producer's [`TreatmentKind`].
    fn kind(&self) -> TreatmentKind;

    /// Display label (player-facing).
    fn display_name(&self) -> &str;

    /// Apply-time window (`min`, `max`) in sim seconds.
    fn apply_window_seconds(&self) -> (f32, f32);

    /// Tool requirement gate.
    fn tool(&self) -> ToolRequirement;

    /// Skill requirement gate.
    fn skill(&self) -> SkillRequirement;

    /// Risk surface (per spec table).
    fn risk(&self) -> RiskKind;

    /// True when this producer targets cardiac arrest (CPR / defib).
    fn revives_downed(&self) -> bool;

    /// True when this producer is origin-aware (rejects on robot).
    fn origin_aware(&self) -> bool;

    /// Resolve the [`TreatmentEffect`] for this producer.
    fn effect(&self, zone_hint: Option<String>) -> TreatmentEffect {
        effect_for(self.kind(), zone_hint)
    }
}

/// **M14H** § default trait impl backed by [`TreatmentSpec`].
#[derive(Debug, Clone)]
pub struct SpecTreatment {
    spec: TreatmentSpec,
}

impl SpecTreatment {
    pub fn new(spec: TreatmentSpec) -> Self {
        Self { spec }
    }
}

impl Treatment for SpecTreatment {
    fn kind(&self) -> TreatmentKind {
        self.spec.kind
    }

    fn display_name(&self) -> &str {
        &self.spec.display_name
    }

    fn apply_window_seconds(&self) -> (f32, f32) {
        (self.spec.apply_seconds_min, self.spec.apply_seconds_max)
    }

    fn tool(&self) -> ToolRequirement {
        self.spec.tool
    }

    fn skill(&self) -> SkillRequirement {
        self.spec.skill
    }

    fn risk(&self) -> RiskKind {
        self.spec.risk
    }

    fn revives_downed(&self) -> bool {
        self.spec.revives_downed
    }

    fn origin_aware(&self) -> bool {
        self.spec.origin_aware
    }
}

/// **M14H** § registry of `Treatment` trait objects, keyed by
/// [`TreatmentKind`]. Equivalent to [`crate::TreatmentSpecRegistry`] but
/// exposes the trait surface instead of the raw spec record.
pub struct TreatmentRegistry {
    treatments: BTreeMap<TreatmentKind, Box<dyn Treatment>>,
}

impl Default for TreatmentRegistry {
    fn default() -> Self {
        Self::baked_default()
    }
}

impl TreatmentRegistry {
    pub fn new() -> Self {
        Self {
            treatments: BTreeMap::new(),
        }
    }

    /// Build the registry with the 22 baked-default producers.
    pub fn baked_default() -> Self {
        let mut registry = TreatmentRegistry::new();
        for kind in TreatmentKind::ALL.iter() {
            let spec = treatment_spec(*kind);
            registry
                .treatments
                .insert(*kind, Box::new(SpecTreatment::new(spec)));
        }
        registry
    }

    pub fn get(&self, kind: TreatmentKind) -> Option<&dyn Treatment> {
        self.treatments.get(&kind).map(|b| b.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (TreatmentKind, &dyn Treatment)> {
        self.treatments.iter().map(|(k, v)| (*k, v.as_ref()))
    }

    pub fn len(&self) -> usize {
        self.treatments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.treatments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_22_producers() {
        let r = TreatmentRegistry::baked_default();
        assert_eq!(r.len(), TreatmentKind::COUNT);
        for kind in TreatmentKind::ALL.iter() {
            assert!(r.get(*kind).is_some(), "missing producer {:?}", kind);
        }
    }

    #[test]
    fn trait_surfaces_match_spec() {
        let r = TreatmentRegistry::baked_default();
        let defib = r.get(TreatmentKind::DefibrillatorV1).expect("defib");
        assert!(defib.revives_downed());
        assert!(defib.origin_aware());
        let window = defib.apply_window_seconds();
        assert!(window.0 > 0.0);
    }

    #[test]
    fn trait_resolves_treatment_effect() {
        let r = TreatmentRegistry::baked_default();
        let cauterize = r.get(TreatmentKind::CauterizeV1).expect("cauterize");
        let effect = cauterize.effect(Some("arm_left".to_string()));
        match effect {
            TreatmentEffect::Cauterize { zone } => assert_eq!(zone, "arm_left"),
            _ => panic!("expected Cauterize effect"),
        }
    }
}
