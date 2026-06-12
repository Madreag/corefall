//! M16B — per-actor multi-disease state (replaces the M19H 3-disease stub).
//!
//! The disease data model + lifecycle FSM live in `cf-disease`; this module
//! is the actor-facing surface: it re-exports the per-actor state, advances
//! it each tick, rolls up HP drain, and exposes carrier / immunity / TTD
//! queries the actor sim + Medic AI consume.

use cf_disease::{
    lifecycle::{self, DiseaseStage},
    DiseaseKind, DiseaseRegistry,
};

pub use cf_disease::{
    lifecycle::{
        ActorDisease, ActorDiseases, DiseaseDiagnosedEvent, DiseaseDiedEvent, DiseaseExposedEvent,
        DiseaseQuarantineEnteredEvent, DiseaseRecoveredEvent, DiseaseRelapsedEvent,
        DiseaseStageChangedEvent, DiseaseVaccinatedEvent, ImmunityRecord, LifecycleOutput,
        RelapseReason, TreatmentProgress,
    },
    DiseaseSpec, IsolationClass, OriginId, TransmissionVector,
};

/// HP drained per second by one symptomatic disease, scaled by severity and
/// untreated lethality. Manifest is the symptomatic burn; Dying is terminal.
pub fn disease_hp_drain_per_second(spec: &DiseaseSpec, stage: DiseaseStage, severity: f32) -> f32 {
    match stage {
        DiseaseStage::Manifest => 0.02 * severity * (0.5 + spec.lethality_untreated),
        DiseaseStage::Dying => 0.2,
        _ => 0.0,
    }
}

/// One tick of disease progression for an actor. Returns the lifecycle
/// events plus the total HP to drain this tick (the actor sim applies it).
pub fn tick_actor_diseases(
    state: &mut ActorDiseases,
    actor_id: u64,
    registry: &DiseaseRegistry,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
) -> (LifecycleOutput, f32) {
    let dt = 1.0 / tick_rate_hz.max(1) as f32;
    let mut hp_drain = 0.0;
    for disease in &state.active {
        if let Some(spec) = registry.get(disease.kind) {
            hp_drain += disease_hp_drain_per_second(spec, disease.stage, disease.severity) * dt;
        }
    }
    let out = lifecycle::tick_actor(state, actor_id, registry, tick, tick_rate_hz, seed);
    (out, hp_drain)
}

/// Disease kinds the actor is currently a carrier of (asymptomatic spreader).
pub fn carrier_kinds(state: &ActorDiseases) -> Vec<DiseaseKind> {
    state
        .active
        .iter()
        .filter(|d| d.stage == DiseaseStage::Carrier)
        .map(|d| d.kind)
        .collect()
}

/// Estimated seconds-to-death from the most lethal active disease (feeds the
/// compound TTD contract). `f32::INFINITY` when nothing is life-threatening.
pub fn disease_ttd_seconds(
    state: &ActorDiseases,
    registry: &DiseaseRegistry,
    tick: u64,
    tick_rate_hz: u32,
) -> f32 {
    let mut best = f32::INFINITY;
    for disease in &state.active {
        let Some(spec) = registry.get(disease.kind) else {
            continue;
        };
        if spec.lethality_untreated <= 0.0 {
            continue;
        }
        let elapsed = (tick.saturating_sub(disease.stage_entered_tick)) as f32 / tick_rate_hz.max(1) as f32;
        let ttd = match disease.stage {
            DiseaseStage::Dying => ((spec.manifest_seconds * 0.1).max(3600.0) - elapsed).max(0.0),
            DiseaseStage::Manifest => (spec.manifest_seconds - elapsed).max(0.0),
            DiseaseStage::Prodromal => spec.prodromal_seconds + spec.manifest_seconds,
            DiseaseStage::Incubating => spec.incubation_seconds + spec.manifest_seconds,
            _ => f32::INFINITY,
        };
        if ttd < best {
            best = ttd;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_disease::lifecycle::expose;

    #[test]
    fn manifest_disease_drains_hp() {
        let reg = DiseaseRegistry::default_registry();
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 1, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        if let Some(d) = state.find_mut(DiseaseKind::Pneumonia) {
            d.stage = DiseaseStage::Manifest;
            d.severity = 1.0;
        }
        let (_out, hp) = tick_actor_diseases(&mut state, 1, &reg, 1, 60, 7);
        assert!(hp > 0.0, "manifest pneumonia must drain HP");
    }

    #[test]
    fn carriers_are_reported() {
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 1, DiseaseKind::Typhoid, TransmissionVector::Waterborne, None, 0);
        state.find_mut(DiseaseKind::Typhoid).unwrap().stage = DiseaseStage::Carrier;
        assert_eq!(carrier_kinds(&state), vec![DiseaseKind::Typhoid]);
    }

    #[test]
    fn ttd_reflects_lethal_manifest() {
        let reg = DiseaseRegistry::default_registry();
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 1, DiseaseKind::Sepsis, TransmissionVector::WoundInfection, None, 0);
        state.find_mut(DiseaseKind::Sepsis).unwrap().stage = DiseaseStage::Manifest;
        let ttd = disease_ttd_seconds(&state, &reg, 0, 60);
        assert!(ttd.is_finite() && ttd <= 6.0 * 3600.0, "sepsis TTD should be <= ~6h, got {ttd}");
    }
}
