//! M16C § Combat-trauma event registration + PTSD-eligible event tagging.
//!
//! A [`TraumaTracker`] wraps the deterministic `cf_mental_health::WitnessWindow`
//! sliding-window detector. When an actor witnesses 3+ squadmate deaths inside
//! the 60s window it emits a [`TraumaWitnessedEvent`] tagged PTSD-eligible; the
//! engine then drives `ActorMentalHealth::record_witnessed_death` (which
//! triggers the PTSD condition) and M25's storyteller runtime fires the trauma
//! narrative beat. The [`TraumaKind`] tagger classifies the other PTSD-eligible
//! triggers (surviving a critical wound, concussion at KO) onto the canonical
//! `cf_mental_health::TriggerReason`.

use std::collections::BTreeMap;

use cf_mental_health::{TriggerReason, WitnessWindow, WITNESS_DEATH_THRESHOLD};
use serde::{Deserialize, Serialize};

/// Narrative event id (locked string) M25 subscribes to.
pub const NARRATIVE_EVENT_ID_TRAUMA_WITNESSED: &str = "narrative.m16c.trauma_witnessed";

/// M11 chatter ticker line surfaced when an actor is overwhelmed by a wipe.
pub const TRAUMA_WITNESSED_CHATTER: &str = "TRAUMA: squad wipe witnessed";

/// A PTSD-eligible combat-trauma category (spec § PTSD triggers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraumaKind {
    /// Witnessed 3+ deaths within 60s.
    WitnessDeaths,
    /// Survived a Critical-band wound.
    SurvivedCriticalWound,
    /// Concussion sustained at the moment of KO.
    ConcussionAtKo,
}

impl TraumaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TraumaKind::WitnessDeaths => "witness_deaths",
            TraumaKind::SurvivedCriticalWound => "survived_critical_wound",
            TraumaKind::ConcussionAtKo => "concussion_at_ko",
        }
    }

    /// Map onto the canonical mental-health trigger reason.
    pub fn trigger_reason(self) -> TriggerReason {
        match self {
            TraumaKind::WitnessDeaths => TriggerReason::WitnessDeaths,
            TraumaKind::SurvivedCriticalWound => TriggerReason::SurviveCriticalWound,
            TraumaKind::ConcussionAtKo => TriggerReason::ConcussionAtKo,
        }
    }

    /// Every launch trauma category is PTSD-eligible.
    pub fn is_ptsd_eligible(self) -> bool {
        true
    }

    pub fn all() -> &'static [TraumaKind] {
        &[
            TraumaKind::WitnessDeaths,
            TraumaKind::SurvivedCriticalWound,
            TraumaKind::ConcussionAtKo,
        ]
    }
}

/// `psych.trauma_witnessed` payload — the squad-wipe trauma beat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraumaWitnessedEvent {
    pub witness_id: u64,
    pub tick: u64,
    /// Deaths counted inside the witness window on the firing tick.
    pub death_count: u32,
    /// Always true — a witnessed wipe is PTSD-eligible.
    pub ptsd_eligible: bool,
}

/// Per-actor witness-death tracker (wraps the deterministic window).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraumaTracker {
    pub witness_id: u64,
    pub witness: WitnessWindow,
}

impl TraumaTracker {
    pub fn new(witness_id: u64) -> Self {
        Self {
            witness_id,
            witness: WitnessWindow::default(),
        }
    }

    /// Record one witnessed squadmate death. Returns the PTSD-eligible trauma
    /// event on the death that crosses the 3-in-60s threshold (once).
    pub fn record_witnessed_death(&mut self, tick: u64, tick_rate_hz: u32) -> Option<TraumaWitnessedEvent> {
        if self.witness.record_death(tick, tick_rate_hz) {
            Some(TraumaWitnessedEvent {
                witness_id: self.witness_id,
                tick,
                death_count: WITNESS_DEATH_THRESHOLD,
                ptsd_eligible: true,
            })
        } else {
            None
        }
    }
}

/// One registered trauma narrative hook (mirrors `PandemicNarrativeRegistration`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraumaNarrativeRegistration {
    pub narrative_event_id: String,
    pub default_intensity: f32,
}

/// Registry of trauma narrative event ids for M25 storyteller directors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraumaNarrativeRegistry {
    pub by_id: BTreeMap<String, TraumaNarrativeRegistration>,
}

impl TraumaNarrativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, registration: TraumaNarrativeRegistration) {
        self.by_id.insert(registration.narrative_event_id.clone(), registration);
    }

    pub fn get(&self, id: &str) -> Option<&TraumaNarrativeRegistration> {
        self.by_id.get(id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Register the trauma narrative beat (witnessed squad wipe).
pub fn register_trauma_narratives(registry: &mut TraumaNarrativeRegistry) {
    registry.register(TraumaNarrativeRegistration {
        narrative_event_id: NARRATIVE_EVENT_ID_TRAUMA_WITNESSED.to_string(),
        default_intensity: 0.85,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_deaths_in_60s_fires_ptsd_eligible_trauma() {
        let mut t = TraumaTracker::new(7);
        assert!(t.record_witnessed_death(0, 60).is_none());
        assert!(t.record_witnessed_death(1000, 60).is_none());
        let ev = t.record_witnessed_death(2000, 60).expect("third death fires trauma");
        assert_eq!(ev.witness_id, 7);
        assert_eq!(ev.death_count, 3);
        assert!(ev.ptsd_eligible);
    }

    #[test]
    fn deaths_spread_past_window_never_fire() {
        let mut t = TraumaTracker::new(7);
        let step = 3601u64; // > 60s at 60 Hz
        assert!(t.record_witnessed_death(0, 60).is_none());
        assert!(t.record_witnessed_death(step, 60).is_none());
        assert!(t.record_witnessed_death(step * 2, 60).is_none());
    }

    #[test]
    fn trauma_kinds_map_to_trigger_reasons_and_are_ptsd_eligible() {
        assert_eq!(TraumaKind::WitnessDeaths.trigger_reason(), TriggerReason::WitnessDeaths);
        assert_eq!(
            TraumaKind::SurvivedCriticalWound.trigger_reason(),
            TriggerReason::SurviveCriticalWound
        );
        assert_eq!(TraumaKind::ConcussionAtKo.trigger_reason(), TriggerReason::ConcussionAtKo);
        for &k in TraumaKind::all() {
            assert!(k.is_ptsd_eligible());
        }
    }

    #[test]
    fn registry_populates() {
        let mut reg = TraumaNarrativeRegistry::new();
        register_trauma_narratives(&mut reg);
        assert!(reg.get(NARRATIVE_EVENT_ID_TRAUMA_WITNESSED).is_some());
        assert_eq!(reg.len(), 1);
    }
}
