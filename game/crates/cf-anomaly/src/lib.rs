//! M16 § Anomaly zones — STALKER-inspired environmental hazards.
//!
//! Six locked anomaly kinds: gravity, electric, time, chemical,
//! bloodsucker_lair, psy_storm. Each has a detection radius, per-tick
//! effect, and counter strategy.
//!
//! World maintains an `AnomalyWorld` keyed by `AnomalyId`. Producers
//! return events the engine forwards to `Recorder::record`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::similar_names
)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Locked 6-kind anomaly set per spec § "Anomaly hazards (STALKER-inspired)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyKind {
    GravityAnomaly,
    ElectricAnomaly,
    TimeAnomaly,
    ChemicalAnomaly,
    BloodsuckerLair,
    PsyStorm,
}

impl AnomalyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AnomalyKind::GravityAnomaly => "gravity_anomaly",
            AnomalyKind::ElectricAnomaly => "electric_anomaly",
            AnomalyKind::TimeAnomaly => "time_anomaly",
            AnomalyKind::ChemicalAnomaly => "chemical_anomaly",
            AnomalyKind::BloodsuckerLair => "bloodsucker_lair",
            AnomalyKind::PsyStorm => "psy_storm",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "gravity_anomaly" => AnomalyKind::GravityAnomaly,
            "electric_anomaly" => AnomalyKind::ElectricAnomaly,
            "time_anomaly" => AnomalyKind::TimeAnomaly,
            "chemical_anomaly" => AnomalyKind::ChemicalAnomaly,
            "bloodsucker_lair" => AnomalyKind::BloodsuckerLair,
            "psy_storm" => AnomalyKind::PsyStorm,
            _ => return None,
        })
    }

    pub fn all() -> &'static [AnomalyKind] {
        &[
            AnomalyKind::GravityAnomaly,
            AnomalyKind::ElectricAnomaly,
            AnomalyKind::TimeAnomaly,
            AnomalyKind::ChemicalAnomaly,
            AnomalyKind::BloodsuckerLair,
            AnomalyKind::PsyStorm,
        ]
    }
}

/// Spec for one anomaly kind. Mirrors the spec § "Anomaly hazards" table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalySpec {
    pub kind: AnomalyKind,
    /// Detection radius (game meters, ~1 m per tile baseline).
    pub detection_radius_m: f32,
    /// Damage per tick at intensity 1.0 when an actor is inside the zone.
    pub damage_per_tick: f32,
    /// Damage cadence (ticks between damage applications).
    pub damage_period_ticks: u32,
    /// Affliction kind applied (mirrors `cf-replay/schemas/event/affliction_applied.json`).
    pub on_contact_affliction: Option<String>,
    /// Counter strategy descriptor used by HUD tooltips + AI cover scoring.
    pub counter: String,
    /// True when the anomaly affects movement (gravity / time slow).
    pub affects_movement: bool,
    /// Movement multiplier inside the zone (1.0 = unaffected).
    pub movement_multiplier: f32,
    /// True when the anomaly is detectable only by an anomaly detector
    /// (bloodsucker lairs, psy storms).
    pub detector_required: bool,
}

impl AnomalySpec {
    pub fn default_for(kind: AnomalyKind) -> Self {
        match kind {
            AnomalyKind::GravityAnomaly => AnomalySpec {
                kind,
                detection_radius_m: 5.0,
                damage_per_tick: 0.0,
                damage_period_ticks: 60,
                on_contact_affliction: None,
                counter: "avoid".to_string(),
                affects_movement: true,
                movement_multiplier: 0.6,
                detector_required: false,
            },
            AnomalyKind::ElectricAnomaly => AnomalySpec {
                kind,
                detection_radius_m: 4.0,
                damage_per_tick: 8.0,
                damage_period_ticks: 30,
                on_contact_affliction: Some("electrified".to_string()),
                counter: "insulation".to_string(),
                affects_movement: false,
                movement_multiplier: 1.0,
                detector_required: false,
            },
            AnomalyKind::TimeAnomaly => AnomalySpec {
                kind,
                detection_radius_m: 6.0,
                damage_per_tick: 0.0,
                damage_period_ticks: 60,
                on_contact_affliction: None,
                counter: "avoid".to_string(),
                affects_movement: true,
                movement_multiplier: 0.5,
                detector_required: false,
            },
            AnomalyKind::ChemicalAnomaly => AnomalySpec {
                kind,
                detection_radius_m: 5.0,
                damage_per_tick: 5.0,
                damage_period_ticks: 30,
                on_contact_affliction: Some("poisoned".to_string()),
                counter: "suit_mask".to_string(),
                affects_movement: false,
                movement_multiplier: 1.0,
                detector_required: false,
            },
            AnomalyKind::BloodsuckerLair => AnomalySpec {
                kind,
                detection_radius_m: 8.0,
                damage_per_tick: 12.0,
                damage_period_ticks: 60,
                on_contact_affliction: Some("bleeding".to_string()),
                counter: "detector_light".to_string(),
                affects_movement: false,
                movement_multiplier: 1.0,
                detector_required: true,
            },
            AnomalyKind::PsyStorm => AnomalySpec {
                kind,
                detection_radius_m: 10.0,
                damage_per_tick: 3.0,
                damage_period_ticks: 60,
                on_contact_affliction: Some("sanity_low".to_string()),
                counter: "helmet_shielding".to_string(),
                affects_movement: false,
                movement_multiplier: 1.0,
                detector_required: true,
            },
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomalyRegistry {
    pub specs: BTreeMap<String, AnomalySpec>,
}

impl AnomalyRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for &k in AnomalyKind::all() {
            specs.insert(k.as_str().to_string(), AnomalySpec::default_for(k));
        }
        Self { specs }
    }

    pub fn lookup(&self, kind: AnomalyKind) -> &AnomalySpec {
        self.specs
            .get(kind.as_str())
            .expect("anomaly registry must contain every kind")
    }

    pub fn load_dir(dir: &Path) -> Result<Self, AnomalyLoadError> {
        let mut reg = Self::default_registry();
        if !dir.exists() {
            return Ok(reg);
        }
        let read_dir = fs::read_dir(dir).map_err(|e| AnomalyLoadError::Io(dir.to_path_buf(), e.to_string()))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            let body = fs::read_to_string(&path).map_err(|e| AnomalyLoadError::Io(path.clone(), e.to_string()))?;
            let spec: AnomalySpec =
                ron::from_str(&body).map_err(|e| AnomalyLoadError::Parse(path.clone(), e.to_string()))?;
            reg.specs.insert(spec.kind.as_str().to_string(), spec);
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AnomalyLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

pub type AnomalyId = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyZone {
    pub id: AnomalyId,
    pub kind: AnomalyKind,
    pub position: [f32; 2],
    pub spawned_at_tick: u64,
    pub last_damage_tick: BTreeMap<u64, u64>,
    /// Actors currently inside this zone (for entry/exit tracking).
    pub actors_inside: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyEnteredEvent {
    pub anomaly_id: AnomalyId,
    pub kind: AnomalyKind,
    pub actor_id: u64,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyDamageEvent {
    pub anomaly_id: AnomalyId,
    pub kind: AnomalyKind,
    pub actor_id: u64,
    pub damage: f32,
    pub applied_affliction: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnomalyTickOutput {
    pub entered: Vec<AnomalyEnteredEvent>,
    pub damage: Vec<AnomalyDamageEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomalyWorld {
    pub zones: BTreeMap<AnomalyId, AnomalyZone>,
    pub next_id: AnomalyId,
}

impl AnomalyWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self, kind: AnomalyKind, position: [f32; 2], tick: u64) -> AnomalyId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.zones.insert(
            id,
            AnomalyZone {
                id,
                kind,
                position,
                spawned_at_tick: tick,
                last_damage_tick: BTreeMap::new(),
                actors_inside: Vec::new(),
            },
        );
        id
    }

    /// Run one sim tick. Walks every (actor, zone) pair, applies entry +
    /// damage events. Pure function: returns events for the engine to
    /// record.
    pub fn tick(
        &mut self,
        registry: &AnomalyRegistry,
        tick: u64,
        actors: &[(u64, [f32; 2])],
    ) -> AnomalyTickOutput {
        let mut out = AnomalyTickOutput::default();
        for zone in self.zones.values_mut() {
            let spec = registry.lookup(zone.kind);
            let r2 = spec.detection_radius_m * spec.detection_radius_m;
            let mut still_inside = Vec::new();
            for &(actor_id, actor_pos) in actors {
                let dx = zone.position[0] - actor_pos[0];
                let dy = zone.position[1] - actor_pos[1];
                let inside = dx * dx + dy * dy <= r2;
                if !inside {
                    continue;
                }
                still_inside.push(actor_id);
                if !zone.actors_inside.contains(&actor_id) {
                    out.entered.push(AnomalyEnteredEvent {
                        anomaly_id: zone.id,
                        kind: zone.kind,
                        actor_id,
                        position: actor_pos,
                    });
                }
                let last_tick = zone.last_damage_tick.get(&actor_id).copied().unwrap_or(0);
                if spec.damage_per_tick > 0.0
                    && (last_tick == 0 || tick.saturating_sub(last_tick) >= spec.damage_period_ticks as u64)
                {
                    out.damage.push(AnomalyDamageEvent {
                        anomaly_id: zone.id,
                        kind: zone.kind,
                        actor_id,
                        damage: spec.damage_per_tick,
                        applied_affliction: spec.on_contact_affliction.clone(),
                    });
                    zone.last_damage_tick.insert(actor_id, tick);
                }
            }
            zone.actors_inside = still_inside;
        }
        out
    }

    /// Returns the (id, kind, position) of every anomaly within
    /// `detector_radius_m` of `position`. The anomaly detector item
    /// surfaces these on the HUD minimap.
    pub fn detector_query(&self, position: [f32; 2], detector_radius_m: f32) -> Vec<(AnomalyId, AnomalyKind, [f32; 2])> {
        let r2 = detector_radius_m * detector_radius_m;
        let mut out = Vec::new();
        for zone in self.zones.values() {
            let dx = zone.position[0] - position[0];
            let dy = zone.position[1] - position[1];
            if dx * dx + dy * dy <= r2 {
                out.push((zone.id, zone.kind, zone.position));
            }
        }
        out
    }

    /// Returns the movement multiplier that should apply to an actor at
    /// `actor_pos` given currently active anomalies. 1.0 = no effect.
    /// Multiple overlapping anomalies multiply.
    pub fn movement_multiplier_at(&self, registry: &AnomalyRegistry, actor_pos: [f32; 2]) -> f32 {
        let mut mult = 1.0_f32;
        for zone in self.zones.values() {
            let spec = registry.lookup(zone.kind);
            if !spec.affects_movement {
                continue;
            }
            let r2 = spec.detection_radius_m * spec.detection_radius_m;
            let dx = zone.position[0] - actor_pos[0];
            let dy = zone.position[1] - actor_pos[1];
            if dx * dx + dy * dy <= r2 {
                mult *= spec.movement_multiplier;
            }
        }
        mult
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_6_kinds() {
        let reg = AnomalyRegistry::default_registry();
        for k in AnomalyKind::all() {
            assert!(reg.specs.contains_key(k.as_str()), "missing {}", k.as_str());
        }
    }

    #[test]
    fn detection_radii_match_spec() {
        let reg = AnomalyRegistry::default_registry();
        assert!((reg.lookup(AnomalyKind::GravityAnomaly).detection_radius_m - 5.0).abs() < 1e-3);
        assert!((reg.lookup(AnomalyKind::ElectricAnomaly).detection_radius_m - 4.0).abs() < 1e-3);
        assert!((reg.lookup(AnomalyKind::TimeAnomaly).detection_radius_m - 6.0).abs() < 1e-3);
        assert!((reg.lookup(AnomalyKind::ChemicalAnomaly).detection_radius_m - 5.0).abs() < 1e-3);
        assert!((reg.lookup(AnomalyKind::BloodsuckerLair).detection_radius_m - 8.0).abs() < 1e-3);
        assert!((reg.lookup(AnomalyKind::PsyStorm).detection_radius_m - 10.0).abs() < 1e-3);
    }

    #[test]
    fn actor_entering_zone_emits_event() {
        let reg = AnomalyRegistry::default_registry();
        let mut world = AnomalyWorld::new();
        let id = world.spawn(AnomalyKind::ElectricAnomaly, [0.0, 0.0], 0);
        let out = world.tick(&reg, 1, &[(7, [1.0, 0.0])]);
        assert!(!out.entered.is_empty());
        assert_eq!(out.entered[0].anomaly_id, id);
        assert_eq!(out.entered[0].actor_id, 7);
    }

    #[test]
    fn electric_anomaly_damages_periodically() {
        let reg = AnomalyRegistry::default_registry();
        let mut world = AnomalyWorld::new();
        world.spawn(AnomalyKind::ElectricAnomaly, [0.0, 0.0], 0);
        let mut total = 0u32;
        for tick in 1..=180u64 {
            let out = world.tick(&reg, tick, &[(7, [1.0, 0.0])]);
            total += out.damage.len() as u32;
        }
        assert!(total >= 5, "electric anomaly should damage ≥5 times in 3 seconds @ 60Hz");
    }

    #[test]
    fn gravity_anomaly_slows_movement() {
        let reg = AnomalyRegistry::default_registry();
        let mut world = AnomalyWorld::new();
        world.spawn(AnomalyKind::GravityAnomaly, [0.0, 0.0], 0);
        let mult = world.movement_multiplier_at(&reg, [0.0, 0.0]);
        assert!(mult < 1.0);
    }

    #[test]
    fn detector_query_returns_anomalies_within_radius() {
        let mut world = AnomalyWorld::new();
        let id_far = world.spawn(AnomalyKind::PsyStorm, [100.0, 0.0], 0);
        let id_near = world.spawn(AnomalyKind::ChemicalAnomaly, [3.0, 0.0], 0);
        let hits = world.detector_query([0.0, 0.0], 10.0);
        assert!(hits.iter().any(|(id, _, _)| *id == id_near));
        assert!(!hits.iter().any(|(id, _, _)| *id == id_far));
    }
}
