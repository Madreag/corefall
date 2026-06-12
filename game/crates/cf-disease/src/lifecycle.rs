//! Per-disease lifecycle FSM + per-actor multi-disease state.
//!
//! FSM: `Exposed → Incubating → Prodromal → Manifest →
//! (Recovering → Recovered | Chronic | Dying → Dead | Carrier)`.
//!
//! All transitions are deterministic and tick-driven. Stochastic outcomes
//! (death, partial-course resistance) use the seeded `deterministic_roll`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    deterministic_roll,
    registry::{DiseaseRegistry, DiseaseSpec, VaccineSpec},
    DiseaseKind, IsolationClass, ItemId, TransmissionVector,
};

const HOUR: f32 = 3_600.0;

// Salts keep the per-(actor,disease) deterministic rolls independent.
const SALT_LETHALITY: u64 = 0x01;
const SALT_CARRIER: u64 = 0x02;
const SALT_RESISTANCE: u64 = 0x03;
const SALT_TREATMENT: u64 = 0x04;

/// Lifecycle stage. `as_str` mirrors the `from`/`to` strings on
/// `disease.stage_changed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiseaseStage {
    Exposed,
    Incubating,
    Prodromal,
    Manifest,
    Recovering,
    Chronic,
    Dying,
    Carrier,
    Recovered,
    Dead,
}

impl DiseaseStage {
    pub fn as_str(self) -> &'static str {
        match self {
            DiseaseStage::Exposed => "exposed",
            DiseaseStage::Incubating => "incubating",
            DiseaseStage::Prodromal => "prodromal",
            DiseaseStage::Manifest => "manifest",
            DiseaseStage::Recovering => "recovering",
            DiseaseStage::Chronic => "chronic",
            DiseaseStage::Dying => "dying",
            DiseaseStage::Carrier => "carrier",
            DiseaseStage::Recovered => "recovered",
            DiseaseStage::Dead => "dead",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, DiseaseStage::Recovered | DiseaseStage::Dead)
    }

    /// Stages where the actor is symptomatic + can transmit (counts toward
    /// the infected ratio + R0 spread).
    pub fn is_infectious(self) -> bool {
        matches!(
            self,
            DiseaseStage::Prodromal | DiseaseStage::Manifest | DiseaseStage::Carrier
        )
    }
}

/// In-progress treatment course on one infection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreatmentProgress {
    pub cure_item: Option<ItemId>,
    pub doses_taken: u8,
    pub doses_required: u8,
    pub last_dose_tick: u64,
    pub completed: bool,
    pub abandoned: bool,
}

impl TreatmentProgress {
    pub fn new(cure_item: Option<ItemId>, doses_required: u8, tick: u64) -> Self {
        Self {
            cure_item,
            doses_taken: 0,
            doses_required,
            last_dose_tick: tick,
            completed: false,
            abandoned: false,
        }
    }

    pub fn fraction(&self) -> f32 {
        if self.doses_required == 0 {
            return 1.0;
        }
        (self.doses_taken as f32 / self.doses_required as f32).min(1.0)
    }
}

/// One active infection on an actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorDisease {
    pub kind: DiseaseKind,
    pub stage: DiseaseStage,
    pub exposed_at_tick: u64,
    pub stage_entered_tick: u64,
    pub vector: TransmissionVector,
    pub source_item_id: Option<ItemId>,
    pub treatment: Option<TreatmentProgress>,
    /// True when this strain resists first-line treatment (forward-compat
    /// hook; full resistant-strain model lands at M16B+1).
    pub resistant_strain: bool,
    pub severity: f32,
    /// Exposure dose multiplier. 1.0 = baseline; > 1.0 scales the lethality
    /// roll for `lethality_scales_with_dose` diseases (radiation sickness).
    #[serde(default = "one_f32")]
    pub dose_factor: f32,
    /// True once a treatment course drove recovery (vs natural resolution),
    /// so `disease.recovered.cured` is accurate.
    #[serde(default)]
    pub cured_by_treatment: bool,
}

fn one_f32() -> f32 {
    1.0
}

impl ActorDisease {
    fn new(kind: DiseaseKind, vector: TransmissionVector, source: Option<ItemId>, tick: u64, resistant: bool) -> Self {
        Self {
            kind,
            stage: DiseaseStage::Exposed,
            exposed_at_tick: tick,
            stage_entered_tick: tick,
            vector,
            source_item_id: source,
            treatment: None,
            resistant_strain: resistant,
            severity: 0.0,
            dose_factor: 1.0,
            cured_by_treatment: false,
        }
    }

    fn enter(&mut self, stage: DiseaseStage, tick: u64) {
        self.stage = stage;
        self.stage_entered_tick = tick;
    }
}

/// Vaccine / natural immunity record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImmunityRecord {
    pub kind: DiseaseKind,
    pub vaccine_id: Option<ItemId>,
    pub doses_taken: u8,
    pub doses_required: u8,
    /// Tick the immunity expires. `u64::MAX` = permanent (natural immunity).
    pub expires_at_tick: u64,
}

impl ImmunityRecord {
    pub fn active(&self, tick: u64) -> bool {
        // Full course (or natural) required for protection.
        self.doses_taken >= self.doses_required && tick < self.expires_at_tick
    }

    pub fn remaining_ticks(&self, tick: u64) -> u64 {
        self.expires_at_tick.saturating_sub(tick)
    }
}

/// Per-actor disease state — multiple concurrent infections, carriers, and
/// the immunity record. Replaces the M19H 3-disease stub.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorDiseases {
    pub origin: crate::OriginId,
    pub active: Vec<ActorDisease>,
    pub immunity: BTreeMap<String, ImmunityRecord>,
    /// Disease kinds whose NEXT infection should spawn a resistant strain
    /// (set when a partial antibiotic course drives resistance).
    pub pending_resistant_strains: BTreeSet<DiseaseKind>,
    /// True once this actor is quarantined.
    pub quarantined: bool,
}

impl ActorDiseases {
    pub fn with_origin(origin: crate::OriginId) -> Self {
        Self {
            origin,
            ..Default::default()
        }
    }

    pub fn find(&self, kind: DiseaseKind) -> Option<&ActorDisease> {
        self.active.iter().find(|d| d.kind == kind)
    }

    pub fn find_mut(&mut self, kind: DiseaseKind) -> Option<&mut ActorDisease> {
        self.active.iter_mut().find(|d| d.kind == kind)
    }

    pub fn has_active(&self, kind: DiseaseKind) -> bool {
        self.active.iter().any(|d| d.kind == kind && !d.stage.is_terminal())
    }

    /// True when the actor has protective immunity to `kind` at `tick`.
    pub fn is_immune(&self, kind: DiseaseKind, tick: u64) -> bool {
        self.immunity
            .get(kind.as_str())
            .map(|r| r.active(tick))
            .unwrap_or(false)
    }

    pub fn immunity_record(&self, kind: DiseaseKind) -> Option<&ImmunityRecord> {
        self.immunity.get(kind.as_str())
    }

    /// Count of currently-infectious diseases (for the pandemic ratio).
    pub fn infectious_count(&self) -> usize {
        self.active.iter().filter(|d| d.stage.is_infectious()).count()
    }

    pub fn is_infectious(&self) -> bool {
        self.active.iter().any(|d| d.stage.is_infectious())
    }
}

// ----- Events -----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseExposedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub vector: TransmissionVector,
    pub source_item_id: Option<ItemId>,
    pub resistant_strain: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseStageChangedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub from: DiseaseStage,
    pub to: DiseaseStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelapseReason {
    PartialCourse,
    TreatmentFailed,
}

impl RelapseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RelapseReason::PartialCourse => "partial_course",
            RelapseReason::TreatmentFailed => "treatment_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseRelapsedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub reason: RelapseReason,
    pub drove_resistance: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseDiagnosedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub stage: DiseaseStage,
    pub confidence: f32,
    /// Cumulative dose (radiation) or accumulation, when the scanner reads it.
    pub dose: f32,
    /// Actor age in seconds when scanned (scanner reads age).
    pub age_seconds: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseRecoveredEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub cured: bool,
    pub granted_immunity: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseDiedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseQuarantineEnteredEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub room_class: IsolationClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiseaseVaccinatedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub pathogen: DiseaseKind,
    pub vaccine_id: ItemId,
    pub doses_taken: u8,
    pub doses_required: u8,
    pub immune: bool,
    pub remaining_duration_seconds: f32,
}

/// Aggregated output of one lifecycle tick for an actor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LifecycleOutput {
    pub stage_changed: Vec<DiseaseStageChangedEvent>,
    pub relapsed: Vec<DiseaseRelapsedEvent>,
    pub recovered: Vec<DiseaseRecoveredEvent>,
    pub died: Vec<DiseaseDiedEvent>,
}

fn dying_seconds(spec: &DiseaseSpec) -> f32 {
    (spec.manifest_seconds * 0.1).clamp(HOUR, 7.0 * 24.0 * HOUR)
}

fn recovery_seconds(spec: &DiseaseSpec) -> f32 {
    (spec.manifest_seconds * 0.25).clamp(HOUR, 3.0 * 24.0 * HOUR)
}

fn ticks_to_seconds(ticks: u64, tick_rate_hz: u32) -> f32 {
    ticks as f32 / tick_rate_hz.max(1) as f32
}

/// Expose an actor to a disease. Creates the infection at `Exposed` (or
/// reuses an existing non-terminal one) and returns the exposure event.
/// Returns `None` when the actor already has an active infection of this
/// kind (no double exposure).
pub fn expose(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    vector: TransmissionVector,
    source_item_id: Option<ItemId>,
    tick: u64,
) -> Option<DiseaseExposedEvent> {
    expose_with_dose(state, actor_id, kind, vector, source_item_id, tick, 1.0)
}

/// Expose an actor with a non-baseline dose magnitude (radiation / toxin
/// exposures). `dose_factor` > 1.0 scales the lethality roll for diseases
/// flagged `lethality_scales_with_dose`.
pub fn expose_with_dose(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    vector: TransmissionVector,
    source_item_id: Option<ItemId>,
    tick: u64,
    dose_factor: f32,
) -> Option<DiseaseExposedEvent> {
    if state.has_active(kind) {
        return None;
    }
    // Drop any terminal record of the same kind before re-infecting.
    state.active.retain(|d| d.kind != kind);
    let resistant = state.pending_resistant_strains.remove(&kind);
    let mut disease = ActorDisease::new(kind, vector, source_item_id.clone(), tick, resistant);
    disease.dose_factor = dose_factor.max(0.0);
    state.active.push(disease);
    Some(DiseaseExposedEvent {
        actor_id,
        tick,
        pathogen: kind,
        vector,
        source_item_id,
        resistant_strain: resistant,
    })
}

/// Begin a treatment course on an active infection.
pub fn start_treatment(state: &mut ActorDiseases, kind: DiseaseKind, spec: &DiseaseSpec, tick: u64) -> bool {
    let Some(disease) = state.find_mut(kind) else {
        return false;
    };
    if disease.stage.is_terminal() {
        return false;
    }
    disease.treatment = Some(TreatmentProgress::new(
        spec.cure.item_required.clone(),
        spec.cure.dose_count,
        tick,
    ));
    true
}

/// Administer one dose of the active treatment course.
pub fn administer_dose(state: &mut ActorDiseases, kind: DiseaseKind, tick: u64) -> bool {
    let Some(disease) = state.find_mut(kind) else {
        return false;
    };
    let Some(t) = disease.treatment.as_mut() else {
        return false;
    };
    if t.completed || t.abandoned {
        return false;
    }
    t.doses_taken = t.doses_taken.saturating_add(1);
    t.last_dose_tick = tick;
    if t.doses_taken >= t.doses_required {
        t.completed = true;
    }
    true
}

/// Player stops a treatment course early. If the completed fraction is below
/// the completion threshold and the disease relapses on partial course, emit
/// `disease.relapsed` and (deterministically) roll whether resistance is
/// driven for the next infection.
pub fn abandon_treatment(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    spec: &DiseaseSpec,
    tick: u64,
    seed: u64,
) -> Option<DiseaseRelapsedEvent> {
    let frac;
    {
        let disease = state.find_mut(kind)?;
        let t = disease.treatment.as_mut()?;
        if t.completed {
            return None;
        }
        t.abandoned = true;
        frac = t.fraction();
        // Relapse returns the infection to Manifest.
        disease.enter(DiseaseStage::Manifest, tick);
    }
    let partial = &spec.cure.partial_course_consequence;
    if frac >= crate::ANTIBIOTIC_COURSE_COMPLETION_THRESHOLD || !partial.relapses {
        return None;
    }
    let mut drove = false;
    if partial.drives_resistance_chance > 0.0 {
        let roll = deterministic_roll(seed, actor_id, kind, SALT_RESISTANCE);
        if roll < partial.drives_resistance_chance {
            state.pending_resistant_strains.insert(kind);
            drove = true;
        }
    }
    Some(DiseaseRelapsedEvent {
        actor_id,
        tick,
        pathogen: kind,
        reason: RelapseReason::PartialCourse,
        drove_resistance: drove,
    })
}

/// Administer (or progress) a vaccine. Returns the vaccinated event with the
/// updated immunity record. Granting immunity requires the full dose course.
pub fn vaccinate(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    vaccine: &VaccineSpec,
    tick: u64,
    tick_rate_hz: u32,
) -> DiseaseVaccinatedEvent {
    let key = kind.as_str().to_string();
    let doses_required = vaccine.doses_required.max(1);
    let entry = state.immunity.entry(key).or_insert(ImmunityRecord {
        kind,
        vaccine_id: Some(vaccine.vaccine_id.clone()),
        doses_taken: 0,
        doses_required,
        expires_at_tick: tick,
    });
    entry.vaccine_id = Some(vaccine.vaccine_id.clone());
    entry.doses_required = doses_required;
    entry.doses_taken = entry.doses_taken.saturating_add(1);
    let duration_ticks = (vaccine.immunity_duration_seconds * tick_rate_hz.max(1) as f32) as u64;
    entry.expires_at_tick = tick.saturating_add(duration_ticks);
    let immune = entry.active(tick);
    DiseaseVaccinatedEvent {
        actor_id,
        tick,
        pathogen: kind,
        vaccine_id: vaccine.vaccine_id.clone(),
        doses_taken: entry.doses_taken,
        doses_required,
        immune,
        remaining_duration_seconds: if immune {
            ticks_to_seconds(entry.remaining_ticks(tick), tick_rate_hz)
        } else {
            0.0
        },
    }
}

/// Decrement a vaccine immunity record's remaining duration view. Returns
/// the remaining seconds (clamped at 0). Used by the per-tick exposure
/// pipeline (scenario: "the actor's vaccine record decrements
/// remaining_duration").
pub fn vaccine_remaining_seconds(state: &ActorDiseases, kind: DiseaseKind, tick: u64, tick_rate_hz: u32) -> f32 {
    state
        .immunity
        .get(kind.as_str())
        .map(|r| ticks_to_seconds(r.remaining_ticks(tick), tick_rate_hz))
        .unwrap_or(0.0)
}

/// Build a diagnosis event for the active infection of `kind` (Medical
/// Scanner consumer). Returns `None` when the actor has no such infection.
pub fn diagnose(
    state: &ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    confidence: f32,
    dose: f32,
    age_seconds: f32,
    tick: u64,
) -> Option<DiseaseDiagnosedEvent> {
    let disease = state.find(kind)?;
    Some(DiseaseDiagnosedEvent {
        actor_id,
        tick,
        pathogen: kind,
        stage: disease.stage,
        confidence: confidence.clamp(0.0, 1.0),
        dose,
        age_seconds,
    })
}

/// Advance every active infection on one actor by one sim tick. Emits
/// stage transitions + outcomes (recovered / died) deterministically.
pub fn tick_actor(
    state: &mut ActorDiseases,
    actor_id: u64,
    registry: &DiseaseRegistry,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
) -> LifecycleOutput {
    let mut out = LifecycleOutput::default();
    let kinds: Vec<DiseaseKind> = state.active.iter().map(|d| d.kind).collect();
    for kind in kinds {
        let Some(spec) = registry.get(kind).cloned() else {
            continue;
        };
        advance_one(state, actor_id, kind, &spec, tick, tick_rate_hz, seed, &mut out);
    }
    // Reap terminal infections (keep one Dead/Recovered marker out of the
    // active list to avoid re-processing; immunity already recorded).
    state.active.retain(|d| !matches!(d.stage, DiseaseStage::Dead));
    out
}

fn advance_one(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    spec: &DiseaseSpec,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
    out: &mut LifecycleOutput,
) {
    // Treatment resolution can fire from any pre-recovery stage.
    if resolve_treatment(state, actor_id, kind, spec, tick, seed, out) {
        return;
    }
    let Some(disease) = state.find_mut(kind) else {
        return;
    };
    let elapsed = ticks_to_seconds(tick.saturating_sub(disease.stage_entered_tick), tick_rate_hz);
    let from = disease.stage;
    match disease.stage {
        DiseaseStage::Exposed => {
            disease.enter(DiseaseStage::Incubating, tick);
            push_stage(out, actor_id, tick, kind, from, DiseaseStage::Incubating);
        }
        DiseaseStage::Incubating => {
            if elapsed >= spec.incubation_seconds {
                let next = if spec.prodromal_seconds > 0.0 {
                    DiseaseStage::Prodromal
                } else {
                    DiseaseStage::Manifest
                };
                disease.enter(next, tick);
                push_stage(out, actor_id, tick, kind, from, next);
            }
        }
        DiseaseStage::Prodromal => {
            if elapsed >= spec.prodromal_seconds {
                disease.enter(DiseaseStage::Manifest, tick);
                push_stage(out, actor_id, tick, kind, from, DiseaseStage::Manifest);
            }
        }
        DiseaseStage::Manifest => {
            disease.severity = (disease.severity + 0.05).min(1.0);
            if spec.manifest_seconds.is_finite() && elapsed >= spec.manifest_seconds {
                decide_outcome(state, actor_id, kind, spec, tick, seed, out);
            }
        }
        DiseaseStage::Dying => {
            if elapsed >= dying_seconds(spec) {
                disease.enter(DiseaseStage::Dead, tick);
                push_stage(out, actor_id, tick, kind, from, DiseaseStage::Dead);
                out.died.push(DiseaseDiedEvent {
                    actor_id,
                    tick,
                    pathogen: kind,
                });
            }
        }
        DiseaseStage::Recovering => {
            if elapsed >= recovery_seconds(spec) {
                let cured = disease.cured_by_treatment;
                disease.enter(DiseaseStage::Recovered, tick);
                push_stage(out, actor_id, tick, kind, from, DiseaseStage::Recovered);
                let granted = grant_natural_immunity(state, kind, tick);
                out.recovered.push(DiseaseRecoveredEvent {
                    actor_id,
                    tick,
                    pathogen: kind,
                    cured,
                    granted_immunity: granted,
                });
            }
        }
        DiseaseStage::Chronic | DiseaseStage::Carrier => {
            // Stays until cured; carriers remain infectious.
        }
        DiseaseStage::Recovered | DiseaseStage::Dead => {}
    }
}

/// Returns true when a treatment course resolved this tick (advancing the
/// infection to Recovering), so the normal stage clock is skipped.
fn resolve_treatment(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    spec: &DiseaseSpec,
    tick: u64,
    seed: u64,
    out: &mut LifecycleOutput,
) -> bool {
    let completed;
    let pre_manifest_ok;
    {
        let Some(disease) = state.find_mut(kind) else {
            return false;
        };
        if matches!(
            disease.stage,
            DiseaseStage::Recovering | DiseaseStage::Recovered | DiseaseStage::Dead | DiseaseStage::Dying
        ) {
            return false;
        }
        let Some(t) = disease.treatment.as_ref() else {
            return false;
        };
        completed = t.completed && !t.abandoned;
        pre_manifest_ok = !spec.cure_only_pre_manifest
            || matches!(
                disease.stage,
                DiseaseStage::Exposed | DiseaseStage::Incubating | DiseaseStage::Prodromal
            );
    }
    if !completed {
        return false;
    }
    let success_roll = deterministic_roll(seed, actor_id, kind, SALT_TREATMENT);
    let success = pre_manifest_ok && success_roll < spec.cure.success_chance;
    let Some(disease) = state.find_mut(kind) else {
        return false;
    };
    let from = disease.stage;
    if success {
        disease.cured_by_treatment = true;
        disease.enter(DiseaseStage::Recovering, tick);
        push_stage(out, actor_id, tick, kind, from, DiseaseStage::Recovering);
        // A cured course grants the same immunity as natural recovery.
        true
    } else {
        // Failed course: relapse to Manifest.
        disease.treatment = None;
        disease.enter(DiseaseStage::Manifest, tick);
        out.relapsed.push(DiseaseRelapsedEvent {
            actor_id,
            tick,
            pathogen: kind,
            reason: RelapseReason::TreatmentFailed,
            drove_resistance: false,
        });
        false
    }
}

fn decide_outcome(
    state: &mut ActorDiseases,
    actor_id: u64,
    kind: DiseaseKind,
    spec: &DiseaseSpec,
    tick: u64,
    seed: u64,
    out: &mut LifecycleOutput,
) {
    let lethal_roll = deterministic_roll(seed, actor_id, kind, SALT_LETHALITY);
    let Some(disease) = state.find_mut(kind) else {
        return;
    };
    let from = disease.stage;
    let effective_lethality = if spec.lethality_scales_with_dose {
        (spec.lethality_untreated * disease.dose_factor).clamp(0.0, 1.0)
    } else {
        spec.lethality_untreated
    };
    if lethal_roll < effective_lethality {
        disease.enter(DiseaseStage::Dying, tick);
        push_stage(out, actor_id, tick, kind, from, DiseaseStage::Dying);
        return;
    }
    if spec.becomes_chronic {
        disease.enter(DiseaseStage::Chronic, tick);
        push_stage(out, actor_id, tick, kind, from, DiseaseStage::Chronic);
        return;
    }
    if spec.can_become_carrier {
        let carrier_roll = deterministic_roll(seed, actor_id, kind, SALT_CARRIER);
        if carrier_roll < 0.25 {
            disease.enter(DiseaseStage::Carrier, tick);
            push_stage(out, actor_id, tick, kind, from, DiseaseStage::Carrier);
            return;
        }
    }
    disease.enter(DiseaseStage::Recovering, tick);
    push_stage(out, actor_id, tick, kind, from, DiseaseStage::Recovering);
}

fn grant_natural_immunity(state: &mut ActorDiseases, kind: DiseaseKind, tick: u64) -> bool {
    // Permanent natural immunity, unless a vaccine record already grants it.
    let record = ImmunityRecord {
        kind,
        vaccine_id: None,
        doses_taken: 1,
        doses_required: 1,
        expires_at_tick: u64::MAX,
    };
    state.immunity.insert(kind.as_str().to_string(), record);
    let _ = tick;
    true
}

fn push_stage(
    out: &mut LifecycleOutput,
    actor_id: u64,
    tick: u64,
    kind: DiseaseKind,
    from: DiseaseStage,
    to: DiseaseStage,
) {
    out.stage_changed.push(DiseaseStageChangedEvent {
        actor_id,
        tick,
        pathogen: kind,
        from,
        to,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OriginId, TransmissionVector};

    fn reg() -> DiseaseRegistry {
        DiseaseRegistry::default_registry()
    }

    #[test]
    fn exposure_creates_infection_with_fields() {
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        let ev = expose(
            &mut s,
            7,
            DiseaseKind::FoodPoisoning,
            TransmissionVector::Foodborne,
            Some("mystery_stew".to_string()),
            100,
        )
        .unwrap();
        assert_eq!(ev.pathogen, DiseaseKind::FoodPoisoning);
        assert_eq!(ev.vector, TransmissionVector::Foodborne);
        assert_eq!(ev.source_item_id.as_deref(), Some("mystery_stew"));
        assert_eq!(s.find(DiseaseKind::FoodPoisoning).unwrap().stage, DiseaseStage::Exposed);
    }

    #[test]
    fn double_exposure_is_ignored() {
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        assert!(expose(&mut s, 7, DiseaseKind::Flu, TransmissionVector::Airborne, None, 0).is_some());
        assert!(expose(&mut s, 7, DiseaseKind::Flu, TransmissionVector::Airborne, None, 1).is_none());
    }

    #[test]
    fn foodborne_progresses_exposed_incubating_manifest() {
        let reg = reg();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::FoodPoisoning, TransmissionVector::Foodborne, Some("stew".into()), 0);
        // First tick: Exposed -> Incubating.
        let out = tick_actor(&mut s, 7, &reg, 1, 60, 1);
        assert!(out
            .stage_changed
            .iter()
            .any(|e| e.from == DiseaseStage::Exposed && e.to == DiseaseStage::Incubating));
        // Advance past incubation (1h). prodromal=0 → Incubating -> Manifest.
        let incub_ticks = (3600.0 * 60.0) as u64 + 2;
        let mut hit_manifest = false;
        for tick in 2..=incub_ticks {
            let out = tick_actor(&mut s, 7, &reg, tick, 60, 1);
            if out
                .stage_changed
                .iter()
                .any(|e| e.from == DiseaseStage::Incubating && e.to == DiseaseStage::Manifest)
            {
                hit_manifest = true;
                break;
            }
        }
        assert!(hit_manifest, "food poisoning must reach Manifest after incubation");
    }

    #[test]
    fn completed_antibiotic_course_cures_pneumonia() {
        let reg = reg();
        let spec = reg.lookup(DiseaseKind::Pneumonia).clone();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        start_treatment(&mut s, DiseaseKind::Pneumonia, &spec, 0);
        for _ in 0..14 {
            administer_dose(&mut s, DiseaseKind::Pneumonia, 1);
        }
        // Use a seed where the success roll passes (0.95 chance — most seeds pass).
        let out = tick_actor(&mut s, 7, &reg, 2, 60, 12345);
        let recovering = out
            .stage_changed
            .iter()
            .any(|e| e.to == DiseaseStage::Recovering);
        assert!(recovering, "completed course should move to Recovering");
    }

    #[test]
    fn partial_course_relapses_with_reason_partial_course() {
        let reg = reg();
        let spec = reg.lookup(DiseaseKind::Pneumonia).clone();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        start_treatment(&mut s, DiseaseKind::Pneumonia, &spec, 0);
        for _ in 0..11 {
            administer_dose(&mut s, DiseaseKind::Pneumonia, 1);
        }
        let ev = abandon_treatment(&mut s, 7, DiseaseKind::Pneumonia, &spec, 5, 999).unwrap();
        assert_eq!(ev.reason, RelapseReason::PartialCourse);
    }

    #[test]
    fn partial_course_can_drive_resistant_strain() {
        let reg = reg();
        let spec = reg.lookup(DiseaseKind::Pneumonia).clone();
        // Search seeds for one whose resistance roll passes (< 0.25).
        let mut drove_seed = None;
        for seed in 0..200u64 {
            if deterministic_roll(seed, 7, DiseaseKind::Pneumonia, SALT_RESISTANCE) < 0.25 {
                drove_seed = Some(seed);
                break;
            }
        }
        let seed = drove_seed.expect("some seed must drive resistance");
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        start_treatment(&mut s, DiseaseKind::Pneumonia, &spec, 0);
        for _ in 0..11 {
            administer_dose(&mut s, DiseaseKind::Pneumonia, 1);
        }
        let ev = abandon_treatment(&mut s, 7, DiseaseKind::Pneumonia, &spec, 5, seed).unwrap();
        assert!(ev.drove_resistance);
        assert!(s.pending_resistant_strains.contains(&DiseaseKind::Pneumonia));
        // Next infection is a resistant strain.
        s.active.clear();
        let exp = expose(&mut s, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 100).unwrap();
        assert!(exp.resistant_strain);
    }

    #[test]
    fn sepsis_kills_within_six_hours_untreated() {
        let reg = reg();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::Sepsis, TransmissionVector::WoundInfection, None, 0);
        // Seed chosen so lethality roll passes (0.80 chance — easy).
        let mut died = false;
        for tick in 1..=(7 * 3600 * 60) as u64 {
            let out = tick_actor(&mut s, 7, &reg, tick, 60, 7);
            if out.died.iter().any(|e| e.pathogen == DiseaseKind::Sepsis) {
                let hours = tick as f32 / 60.0 / 3600.0;
                assert!(hours <= 6.5, "sepsis death should land near 6h, got {hours}h");
                died = true;
                break;
            }
        }
        assert!(died, "untreated sepsis with passing lethality roll must kill");
    }

    #[test]
    fn radiation_lethality_scales_with_dose() {
        // A higher dose factor must produce more deaths than baseline at the
        // outcome decision. Jump each actor straight to the end of Manifest.
        let reg = reg();
        let manifest_ticks = (reg.lookup(DiseaseKind::RadiationSickness).manifest_seconds * 60.0) as u64 + 2;
        let count_deaths = |dose_factor: f32| -> u32 {
            let mut deaths = 0;
            for actor in 0..400u64 {
                let mut s = ActorDiseases::with_origin(OriginId::Human);
                expose_with_dose(
                    &mut s,
                    actor,
                    DiseaseKind::RadiationSickness,
                    TransmissionVector::RadiationDose,
                    None,
                    0,
                    dose_factor,
                );
                {
                    let d = s.find_mut(DiseaseKind::RadiationSickness).unwrap();
                    d.stage = DiseaseStage::Manifest;
                    d.stage_entered_tick = 0;
                }
                let out = tick_actor(&mut s, actor, &reg, manifest_ticks, 60, actor);
                if out.stage_changed.iter().any(|e| e.to == DiseaseStage::Dying) {
                    deaths += 1;
                }
            }
            deaths
        };
        let low = count_deaths(1.0);
        let high = count_deaths(3.0);
        assert!(high > low, "3x dose must raise radiation death count ({high} vs {low})");
    }

    #[test]
    fn cured_flag_is_true_only_for_treatment_recovery() {
        let reg = reg();
        let spec = reg.lookup(DiseaseKind::Pneumonia).clone();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut s, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        start_treatment(&mut s, DiseaseKind::Pneumonia, &spec, 0);
        for _ in 0..14 {
            administer_dose(&mut s, DiseaseKind::Pneumonia, 1);
        }
        // One tick: completed course → Recovering (cured_by_treatment=true).
        let out = tick_actor(&mut s, 7, &reg, 2, 60, 12345);
        assert!(out.stage_changed.iter().any(|e| e.to == DiseaseStage::Recovering));
        // Jump to the end of the Recovering window in one tick.
        s.find_mut(DiseaseKind::Pneumonia).unwrap().stage_entered_tick = 0;
        let recov_ticks = (recovery_seconds(&spec) * 60.0) as u64 + 2;
        let out2 = tick_actor(&mut s, 7, &reg, recov_ticks, 60, 12345);
        let ev = out2.recovered.into_iter().next().expect("a recovered event must fire");
        assert!(ev.cured, "treatment-driven recovery must report cured=true");
    }

    #[test]
    fn vaccine_full_course_confers_immunity() {
        let reg = reg();
        let spec = reg.lookup(DiseaseKind::Flu).clone();
        let vaccine = spec.vaccine.clone().unwrap();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        let ev = vaccinate(&mut s, 7, DiseaseKind::Flu, &vaccine, 0, 60);
        assert!(ev.immune, "single-dose flu vaccine confers immunity");
        assert!(s.is_immune(DiseaseKind::Flu, 0));
        assert!(ev.remaining_duration_seconds > 0.0);
    }

    #[test]
    fn recovery_grants_natural_immunity() {
        let reg = reg();
        let mut s = ActorDiseases::with_origin(OriginId::Human);
        // Force a quick recovery path on common cold (lethality 0, not chronic).
        let spec = reg.lookup(DiseaseKind::CommonCold).clone();
        expose(&mut s, 7, DiseaseKind::CommonCold, TransmissionVector::CloseContact, None, 0);
        // Drive through the whole lifecycle deterministically.
        let total = (spec.total_course_seconds() * 60.0) as u64 + (recovery_seconds(&spec) * 60.0) as u64 + 10;
        for tick in 1..=total {
            tick_actor(&mut s, 7, &reg, tick, 60, 3);
        }
        assert!(s.is_immune(DiseaseKind::CommonCold, total));
    }
}
