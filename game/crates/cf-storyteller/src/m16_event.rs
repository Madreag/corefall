//! M16 § Storyteller narrative-event surface for hazard-spawned events.
//!
//! Per M16 spec § "Storyteller integration":
//! > Hazard-spawned events (fire_spread_event / electric_arc_cascade /
//! > acid_pool_growth / radiation_storm) register via M25's
//! > cf-storyteller::registry.register() API. M16 ships hazard mechanics
//! > + storyteller event hooks; M25 owns the storyteller runtime.
//!
//! The hooks here are the registration surface; M25 runtime consumes
//! them to drive narrative beats when hazards escalate.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// M16 hazard-narrative event ids (locked strings). M25 narrative
/// directors subscribe to these to fire intensity-spike beats.
pub const NARRATIVE_EVENT_ID_FIRE_SPREAD: &str = "narrative.m16.fire_spread";
pub const NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE: &str = "narrative.m16.electric_arc_cascade";
pub const NARRATIVE_EVENT_ID_ACID_POOL_GROWTH: &str = "narrative.m16.acid_pool_growth";
pub const NARRATIVE_EVENT_ID_RADIATION_STORM: &str = "narrative.m16.radiation_storm";
pub const NARRATIVE_EVENT_ID_ANOMALY_ENCOUNTERED: &str = "narrative.m16.anomaly_encountered";
pub const NARRATIVE_EVENT_ID_DROWNING_LETHAL: &str = "narrative.m16.drowning_lethal";

/// M16 narrative event categories. Mapped onto specific narrative_event
/// ids consumed by M25 storyteller. Each kind has a registered handler in
/// the [`M16NarrativeRegistry`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum M16NarrativeKind {
    FireSpread = 0,
    ElectricArcCascade = 1,
    AcidPoolGrowth = 2,
    RadiationStorm = 3,
    AnomalyEncountered = 4,
    DrowningLethal = 5,
}

impl M16NarrativeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            M16NarrativeKind::FireSpread => "fire_spread",
            M16NarrativeKind::ElectricArcCascade => "electric_arc_cascade",
            M16NarrativeKind::AcidPoolGrowth => "acid_pool_growth",
            M16NarrativeKind::RadiationStorm => "radiation_storm",
            M16NarrativeKind::AnomalyEncountered => "anomaly_encountered",
            M16NarrativeKind::DrowningLethal => "drowning_lethal",
        }
    }

    pub fn narrative_event_id(self) -> &'static str {
        match self {
            M16NarrativeKind::FireSpread => NARRATIVE_EVENT_ID_FIRE_SPREAD,
            M16NarrativeKind::ElectricArcCascade => NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE,
            M16NarrativeKind::AcidPoolGrowth => NARRATIVE_EVENT_ID_ACID_POOL_GROWTH,
            M16NarrativeKind::RadiationStorm => NARRATIVE_EVENT_ID_RADIATION_STORM,
            M16NarrativeKind::AnomalyEncountered => NARRATIVE_EVENT_ID_ANOMALY_ENCOUNTERED,
            M16NarrativeKind::DrowningLethal => NARRATIVE_EVENT_ID_DROWNING_LETHAL,
        }
    }
}

/// One registered narrative event hook. M25 storyteller mods can override
/// the description / intensity to drive their beats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M16NarrativeRegistration {
    pub kind: M16NarrativeKind,
    pub narrative_event_id: String,
    /// Default intensity (0..=1) the storyteller treats as the baseline
    /// excitement spike for this event. M25 can override per-mod.
    pub default_intensity: f32,
}

impl M16NarrativeRegistration {
    pub fn new(kind: M16NarrativeKind, default_intensity: f32) -> Self {
        Self {
            kind,
            narrative_event_id: kind.narrative_event_id().to_string(),
            default_intensity: default_intensity.clamp(0.0, 1.0),
        }
    }
}

/// Storyteller registry for M16 hazard narrative event ids. Engine
/// constructs one at init time + calls `register_m16_narratives()` to
/// populate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M16NarrativeRegistry {
    pub by_id: BTreeMap<String, M16NarrativeRegistration>,
}

impl M16NarrativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, registration: M16NarrativeRegistration) {
        self.by_id.insert(registration.narrative_event_id.clone(), registration);
    }

    pub fn get(&self, id: &str) -> Option<&M16NarrativeRegistration> {
        self.by_id.get(id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Spec § "Storyteller integration: Hazard-spawned events ... register via
/// M25's cf-storyteller::registry.register() API". Engine calls this once
/// at startup so M25 narrative directors see the registered ids in their
/// catalog.
pub fn register_m16_narratives(registry: &mut M16NarrativeRegistry) {
    registry.register(M16NarrativeRegistration::new(M16NarrativeKind::FireSpread, 0.45));
    registry.register(M16NarrativeRegistration::new(
        M16NarrativeKind::ElectricArcCascade,
        0.55,
    ));
    registry.register(M16NarrativeRegistration::new(M16NarrativeKind::AcidPoolGrowth, 0.35));
    registry.register(M16NarrativeRegistration::new(M16NarrativeKind::RadiationStorm, 0.75));
    registry.register(M16NarrativeRegistration::new(
        M16NarrativeKind::AnomalyEncountered,
        0.40,
    ));
    registry.register(M16NarrativeRegistration::new(M16NarrativeKind::DrowningLethal, 0.85));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_canonical() {
        assert_eq!(NARRATIVE_EVENT_ID_FIRE_SPREAD, "narrative.m16.fire_spread");
        assert_eq!(NARRATIVE_EVENT_ID_RADIATION_STORM, "narrative.m16.radiation_storm");
    }

    #[test]
    fn register_m16_narratives_populates_registry() {
        let mut reg = M16NarrativeRegistry::new();
        register_m16_narratives(&mut reg);
        assert!(reg.get(NARRATIVE_EVENT_ID_FIRE_SPREAD).is_some());
        assert!(reg.get(NARRATIVE_EVENT_ID_ELECTRIC_ARC_CASCADE).is_some());
        assert!(reg.get(NARRATIVE_EVENT_ID_ACID_POOL_GROWTH).is_some());
        assert!(reg.get(NARRATIVE_EVENT_ID_RADIATION_STORM).is_some());
        assert!(reg.get(NARRATIVE_EVENT_ID_ANOMALY_ENCOUNTERED).is_some());
        assert!(reg.get(NARRATIVE_EVENT_ID_DROWNING_LETHAL).is_some());
        assert_eq!(reg.len(), 6);
    }

    #[test]
    fn intensity_is_clamped() {
        let r = M16NarrativeRegistration::new(M16NarrativeKind::FireSpread, 99.0);
        assert!((r.default_intensity - 1.0).abs() < 1e-6);
    }
}
