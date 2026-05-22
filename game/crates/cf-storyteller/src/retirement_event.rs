//! **M14I** § retirement narrative event registration.
//!
//! When an actor reaches `retirement_age + 5` and the player commits to
//! retirement via `act.player.retire_veteran`, the engine emits a
//! `veteran.retired` replay event. The storyteller subscribes to that
//! event via the canonical narrative-event id defined here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::NarrativeEventKind;

/// Canonical narrative-event id for "veteran retired". Storyteller mods
/// reference this string when registering a narrative beat to fire.
pub const NARRATIVE_EVENT_ID_VETERAN_RETIRED: &str = "narrative.veteran_retired";

/// the M14I retire dispatcher fires + consumed by the storyteller / UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetirementNarrative {
    pub actor_id: u64,
    pub age_in_game_years: f32,
    pub retired_tick: u64,
    pub narrative_event_id: String,
}

impl RetirementNarrative {
    pub fn new(actor_id: u64, age_in_game_years: f32, retired_tick: u64) -> Self {
        Self {
            actor_id,
            age_in_game_years,
            retired_tick,
            narrative_event_id: NARRATIVE_EVENT_ID_VETERAN_RETIRED.to_string(),
        }
    }

    pub fn kind(&self) -> NarrativeEventKind {
        NarrativeEventKind::VeteranRetired
    }
}

/// actor id. Cleared once a downstream consumer (storyteller) commits
/// the narrative beat to a save.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetirementNarrativeRegistry {
    pub by_actor: BTreeMap<u64, RetirementNarrative>,
}

impl RetirementNarrativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.by_actor.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_actor.is_empty()
    }

    pub fn get(&self, actor_id: u64) -> Option<&RetirementNarrative> {
        self.by_actor.get(&actor_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &RetirementNarrative)> {
        self.by_actor.iter()
    }
}

/// Register a retirement narrative for `actor_id`. Returns the inserted
/// record; replaces any previous record for the same actor.
pub fn register_retirement_narrative(
    registry: &mut RetirementNarrativeRegistry,
    actor_id: u64,
    age_in_game_years: f32,
    retired_tick: u64,
) -> RetirementNarrative {
    let narrative = RetirementNarrative::new(actor_id, age_in_game_years, retired_tick);
    registry
        .by_actor
        .insert(actor_id, narrative.clone());
    narrative
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrative_event_id_is_canonical() {
        assert_eq!(NARRATIVE_EVENT_ID_VETERAN_RETIRED, "narrative.veteran_retired");
    }

    #[test]
    fn registry_round_trip() {
        let mut r = RetirementNarrativeRegistry::new();
        assert!(r.is_empty());
        let narrative = register_retirement_narrative(&mut r, 42, 61.0, 999);
        assert_eq!(narrative.actor_id, 42);
        assert_eq!(narrative.age_in_game_years, 61.0);
        assert_eq!(narrative.retired_tick, 999);
        assert_eq!(narrative.kind(), NarrativeEventKind::VeteranRetired);
        assert_eq!(r.len(), 1);
        assert_eq!(r.get(42), Some(&narrative));
    }

    #[test]
    fn register_replaces_existing() {
        let mut r = RetirementNarrativeRegistry::new();
        register_retirement_narrative(&mut r, 1, 60.0, 100);
        register_retirement_narrative(&mut r, 1, 65.0, 200);
        assert_eq!(r.len(), 1);
        let n = r.get(1).unwrap();
        assert_eq!(n.age_in_game_years, 65.0);
        assert_eq!(n.retired_tick, 200);
    }
}
