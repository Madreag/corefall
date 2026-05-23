//! M16 § Artifacts — rare loot dropped near anomalies that grants passive
//! bonuses while carried.
//!
//! 20+ launch artifacts across 5 rarity tiers (Common / Magic / Rare /
//! Legendary / Unique). Each artifact carries a bonus profile applied to
//! the carrier (max_hp, aim_accuracy, drop_rate, etc.). The bonus is
//! applied at pickup and removed at drop; engine wires the events.

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

/// Locked artifact rarity tiers per spec § "Artifacts have rarity tiers
/// (Common / Magic / Rare / Legendary / Unique)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRarity {
    Common,
    Magic,
    Rare,
    Legendary,
    Unique,
}

impl ArtifactRarity {
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactRarity::Common => "common",
            ArtifactRarity::Magic => "magic",
            ArtifactRarity::Rare => "rare",
            ArtifactRarity::Legendary => "legendary",
            ArtifactRarity::Unique => "unique",
        }
    }
}

/// Passive bonus profile applied to the carrier while held. Additive
/// fields combine across multiple artifacts; multiplicative fields
/// multiply. Resistance fields are clamped to [0, 0.95] after summation.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ArtifactBonus {
    /// Flat HP added to the carrier's max_hp.
    #[serde(default)]
    pub max_hp_bonus: f32,
    /// Multiplier on aim accuracy (e.g. 0.1 = +10%).
    #[serde(default)]
    pub aim_accuracy_bonus_pct: f32,
    /// Multiplier on currency/credit drops (e.g. 0.5 = +50%).
    #[serde(default)]
    pub drop_rate_bonus_pct: f32,
    /// Radiation resistance [0, 1].
    #[serde(default)]
    pub radiation_resistance: f32,
    /// Cold resistance [0, 1].
    #[serde(default)]
    pub cold_resistance: f32,
    /// Fire resistance [0, 1].
    #[serde(default)]
    pub fire_resistance: f32,
    /// Electric resistance [0, 1].
    #[serde(default)]
    pub electric_resistance: f32,
    /// Toxic / chemical resistance [0, 1].
    #[serde(default)]
    pub toxic_resistance: f32,
    /// Tool durability multiplier (1.0 = unaffected; 2.0 = 2x durability).
    #[serde(default)]
    pub tool_durability_multiplier: f32,
    /// Chassis battery capacity multiplier (1.0 = unaffected).
    #[serde(default)]
    pub battery_capacity_multiplier: f32,
    /// Stamina regeneration rate multiplier (1.0 = unaffected).
    #[serde(default)]
    pub stamina_regen_multiplier: f32,
    /// Sprint speed multiplier (1.0 = unaffected).
    #[serde(default)]
    pub sprint_speed_multiplier: f32,
    /// Carry weight multiplier (1.0 = unaffected; 1.5 = 50% more carry).
    #[serde(default)]
    pub carry_weight_multiplier: f32,
    /// True when the artifact reveals nearby anomalies on the minimap.
    #[serde(default)]
    pub reveals_anomalies: bool,
    /// True when the artifact passively absorbs damage at a cost (the
    /// Soul-Eater pattern).
    #[serde(default)]
    pub damage_absorption_pct: f32,
}

impl ArtifactBonus {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sum two bonuses field-wise. Multiplicative fields default to 1.0
    /// when zero (so adding an empty bonus is a no-op).
    pub fn combine(&self, other: &Self) -> Self {
        let mul = |a: f32, b: f32| {
            let a = if a == 0.0 { 1.0 } else { a };
            let b = if b == 0.0 { 1.0 } else { b };
            a * b
        };
        Self {
            max_hp_bonus: self.max_hp_bonus + other.max_hp_bonus,
            aim_accuracy_bonus_pct: self.aim_accuracy_bonus_pct + other.aim_accuracy_bonus_pct,
            drop_rate_bonus_pct: self.drop_rate_bonus_pct + other.drop_rate_bonus_pct,
            radiation_resistance: (self.radiation_resistance + other.radiation_resistance).min(0.95),
            cold_resistance: (self.cold_resistance + other.cold_resistance).min(0.95),
            fire_resistance: (self.fire_resistance + other.fire_resistance).min(0.95),
            electric_resistance: (self.electric_resistance + other.electric_resistance).min(0.95),
            toxic_resistance: (self.toxic_resistance + other.toxic_resistance).min(0.95),
            tool_durability_multiplier: mul(self.tool_durability_multiplier, other.tool_durability_multiplier),
            battery_capacity_multiplier: mul(self.battery_capacity_multiplier, other.battery_capacity_multiplier),
            stamina_regen_multiplier: mul(self.stamina_regen_multiplier, other.stamina_regen_multiplier),
            sprint_speed_multiplier: mul(self.sprint_speed_multiplier, other.sprint_speed_multiplier),
            carry_weight_multiplier: mul(self.carry_weight_multiplier, other.carry_weight_multiplier),
            reveals_anomalies: self.reveals_anomalies || other.reveals_anomalies,
            damage_absorption_pct: (self.damage_absorption_pct + other.damage_absorption_pct).min(0.75),
        }
    }
}

/// Spec for one artifact. Loaded from `content/artifacts/*.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactSpec {
    pub id: String,
    pub display_name: String,
    pub rarity: ArtifactRarity,
    pub bonus: ArtifactBonus,
    /// Anomaly kinds (string-encoded) where this artifact may drop.
    pub spawn_near_anomalies: Vec<String>,
    /// 0..=1 weight for the drop roll within its rarity tier.
    #[serde(default = "default_spawn_weight")]
    pub spawn_weight: f32,
}

fn default_spawn_weight() -> f32 {
    1.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactRegistry {
    pub specs: BTreeMap<String, ArtifactSpec>,
}

impl ArtifactRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for spec in launch_artifacts() {
            specs.insert(spec.id.clone(), spec);
        }
        Self { specs }
    }

    pub fn lookup(&self, id: &str) -> Option<&ArtifactSpec> {
        self.specs.get(id)
    }

    pub fn load_dir(dir: &Path) -> Result<Self, ArtifactLoadError> {
        let mut reg = Self::default_registry();
        if !dir.exists() {
            return Ok(reg);
        }
        let read_dir = fs::read_dir(dir).map_err(|e| ArtifactLoadError::Io(dir.to_path_buf(), e.to_string()))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            let body = fs::read_to_string(&path).map_err(|e| ArtifactLoadError::Io(path.clone(), e.to_string()))?;
            let spec: ArtifactSpec =
                ron::from_str(&body).map_err(|e| ArtifactLoadError::Parse(path.clone(), e.to_string()))?;
            reg.specs.insert(spec.id.clone(), spec);
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ArtifactLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

/// 20 launch artifacts per spec literal § "Artifact loot (20+ artifacts
/// from anomaly proximity)". Spec literal mentions:
/// Stone Blood, Soul, Goldfish, Compass, Bubble, Snowflake, Flame,
/// Wrench, Battery (9) + "11+ more". This function lists 20 to satisfy
/// the spec literal.
pub fn launch_artifacts() -> Vec<ArtifactSpec> {
    vec![
        // The spec-named 9:
        ArtifactSpec {
            id: "stone_blood".to_string(),
            display_name: "Stone Blood".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                max_hp_bonus: 20.0,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["bloodsucker_lair".to_string()],
            spawn_weight: 1.0,
        },
        ArtifactSpec {
            id: "soul".to_string(),
            display_name: "Soul".to_string(),
            rarity: ArtifactRarity::Legendary,
            bonus: ArtifactBonus {
                aim_accuracy_bonus_pct: 0.10,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["psy_storm".to_string()],
            spawn_weight: 0.4,
        },
        ArtifactSpec {
            id: "goldfish".to_string(),
            display_name: "Goldfish".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                drop_rate_bonus_pct: 0.50,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["chemical_anomaly".to_string()],
            spawn_weight: 0.8,
        },
        ArtifactSpec {
            id: "compass".to_string(),
            display_name: "Compass".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                reveals_anomalies: true,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["time_anomaly".to_string()],
            spawn_weight: 0.6,
        },
        ArtifactSpec {
            id: "bubble".to_string(),
            display_name: "Bubble".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                radiation_resistance: 0.30,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["chemical_anomaly".to_string()],
            spawn_weight: 0.8,
        },
        ArtifactSpec {
            id: "snowflake".to_string(),
            display_name: "Snowflake".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                cold_resistance: 0.40,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["psy_storm".to_string()],
            spawn_weight: 0.9,
        },
        ArtifactSpec {
            id: "flame".to_string(),
            display_name: "Flame".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                fire_resistance: 0.40,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["electric_anomaly".to_string()],
            spawn_weight: 0.9,
        },
        ArtifactSpec {
            id: "wrench".to_string(),
            display_name: "Wrench".to_string(),
            rarity: ArtifactRarity::Common,
            bonus: ArtifactBonus {
                tool_durability_multiplier: 1.50,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["gravity_anomaly".to_string()],
            spawn_weight: 1.2,
        },
        ArtifactSpec {
            id: "battery".to_string(),
            display_name: "Battery".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                battery_capacity_multiplier: 1.40,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["electric_anomaly".to_string()],
            spawn_weight: 0.7,
        },
        // 11 more per spec "11+ more":
        ArtifactSpec {
            id: "moonlight".to_string(),
            display_name: "Moonlight".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                stamina_regen_multiplier: 1.30,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["psy_storm".to_string()],
            spawn_weight: 0.6,
        },
        ArtifactSpec {
            id: "spring".to_string(),
            display_name: "Spring".to_string(),
            rarity: ArtifactRarity::Common,
            bonus: ArtifactBonus {
                sprint_speed_multiplier: 1.20,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["gravity_anomaly".to_string()],
            spawn_weight: 1.0,
        },
        ArtifactSpec {
            id: "fireball".to_string(),
            display_name: "Fireball".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                fire_resistance: 0.55,
                cold_resistance: -0.10,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["electric_anomaly".to_string()],
            spawn_weight: 0.5,
        },
        ArtifactSpec {
            id: "ice_drop".to_string(),
            display_name: "Ice Drop".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                cold_resistance: 0.55,
                fire_resistance: -0.10,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["psy_storm".to_string()],
            spawn_weight: 0.5,
        },
        ArtifactSpec {
            id: "sparkler".to_string(),
            display_name: "Sparkler".to_string(),
            rarity: ArtifactRarity::Common,
            bonus: ArtifactBonus {
                electric_resistance: 0.30,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["electric_anomaly".to_string()],
            spawn_weight: 1.0,
        },
        ArtifactSpec {
            id: "thorn".to_string(),
            display_name: "Thorn".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                toxic_resistance: 0.35,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["chemical_anomaly".to_string()],
            spawn_weight: 0.8,
        },
        ArtifactSpec {
            id: "atlas".to_string(),
            display_name: "Atlas".to_string(),
            rarity: ArtifactRarity::Rare,
            bonus: ArtifactBonus {
                carry_weight_multiplier: 1.50,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["gravity_anomaly".to_string()],
            spawn_weight: 0.6,
        },
        ArtifactSpec {
            id: "shell".to_string(),
            display_name: "Shell".to_string(),
            rarity: ArtifactRarity::Legendary,
            bonus: ArtifactBonus {
                damage_absorption_pct: 0.15,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["bloodsucker_lair".to_string()],
            spawn_weight: 0.3,
        },
        ArtifactSpec {
            id: "soul_eater".to_string(),
            display_name: "Soul Eater".to_string(),
            rarity: ArtifactRarity::Unique,
            bonus: ArtifactBonus {
                max_hp_bonus: 50.0,
                radiation_resistance: 0.20,
                damage_absorption_pct: 0.10,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["psy_storm".to_string()],
            spawn_weight: 0.1,
        },
        ArtifactSpec {
            id: "mama_bead".to_string(),
            display_name: "Mama's Beads".to_string(),
            rarity: ArtifactRarity::Magic,
            bonus: ArtifactBonus {
                radiation_resistance: 0.25,
                toxic_resistance: 0.20,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["chemical_anomaly".to_string()],
            spawn_weight: 0.7,
        },
        ArtifactSpec {
            id: "night_star".to_string(),
            display_name: "Night Star".to_string(),
            rarity: ArtifactRarity::Legendary,
            bonus: ArtifactBonus {
                max_hp_bonus: 10.0,
                aim_accuracy_bonus_pct: 0.05,
                reveals_anomalies: true,
                ..Default::default()
            },
            spawn_near_anomalies: vec!["time_anomaly".to_string(), "bloodsucker_lair".to_string()],
            spawn_weight: 0.2,
        },
    ]
}

pub type ArtifactInstanceId = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactInstance {
    pub instance_id: ArtifactInstanceId,
    pub spec_id: String,
    /// World position if dropped; None when held in an inventory.
    pub world_position: Option<[f32; 2]>,
    /// Owner actor id if carried; None when dropped.
    pub carrier: Option<u64>,
    pub spawned_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactSpawnedEvent {
    pub instance_id: ArtifactInstanceId,
    pub spec_id: String,
    pub rarity: ArtifactRarity,
    pub position: [f32; 2],
    pub source_anomaly_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPickedUpEvent {
    pub instance_id: ArtifactInstanceId,
    pub spec_id: String,
    pub actor_id: u64,
    pub rarity: ArtifactRarity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactCarriedBonusEvent {
    pub instance_id: ArtifactInstanceId,
    pub spec_id: String,
    pub actor_id: u64,
    pub bonus_snapshot: ArtifactBonus,
}

/// World state — every spawned + held artifact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactWorld {
    pub instances: BTreeMap<ArtifactInstanceId, ArtifactInstance>,
    pub next_id: ArtifactInstanceId,
}

impl ArtifactWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(
        &mut self,
        registry: &ArtifactRegistry,
        spec_id: &str,
        position: [f32; 2],
        tick: u64,
        source_anomaly_id: Option<u64>,
    ) -> Option<ArtifactSpawnedEvent> {
        let spec = registry.lookup(spec_id)?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.instances.insert(
            id,
            ArtifactInstance {
                instance_id: id,
                spec_id: spec_id.to_string(),
                world_position: Some(position),
                carrier: None,
                spawned_at_tick: tick,
            },
        );
        Some(ArtifactSpawnedEvent {
            instance_id: id,
            spec_id: spec_id.to_string(),
            rarity: spec.rarity,
            position,
            source_anomaly_id,
        })
    }

    pub fn pickup(
        &mut self,
        registry: &ArtifactRegistry,
        instance_id: ArtifactInstanceId,
        actor_id: u64,
    ) -> Option<(ArtifactPickedUpEvent, ArtifactCarriedBonusEvent)> {
        let inst = self.instances.get_mut(&instance_id)?;
        let spec = registry.lookup(&inst.spec_id)?.clone();
        inst.carrier = Some(actor_id);
        inst.world_position = None;
        Some((
            ArtifactPickedUpEvent {
                instance_id,
                spec_id: spec.id.clone(),
                actor_id,
                rarity: spec.rarity,
            },
            ArtifactCarriedBonusEvent {
                instance_id,
                spec_id: spec.id,
                actor_id,
                bonus_snapshot: spec.bonus,
            },
        ))
    }

    pub fn drop_at(
        &mut self,
        instance_id: ArtifactInstanceId,
        position: [f32; 2],
    ) -> Option<ArtifactInstance> {
        let inst = self.instances.get_mut(&instance_id)?;
        inst.carrier = None;
        inst.world_position = Some(position);
        Some(inst.clone())
    }

    /// Returns the carried-bonus aggregate for an actor (sum across every
    /// artifact in their inventory). M16 spec § "Artifacts have rarity
    /// tiers ... provides passive bonuses when carried".
    pub fn aggregate_bonus_for_actor(&self, registry: &ArtifactRegistry, actor_id: u64) -> ArtifactBonus {
        let mut agg = ArtifactBonus::empty();
        for inst in self.instances.values() {
            if inst.carrier != Some(actor_id) {
                continue;
            }
            if let Some(spec) = registry.lookup(&inst.spec_id) {
                agg = agg.combine(&spec.bonus);
            }
        }
        agg
    }

    /// Returns the list of artifact instance ids carried by an actor.
    pub fn carried_by(&self, actor_id: u64) -> Vec<ArtifactInstanceId> {
        self.instances
            .values()
            .filter(|i| i.carrier == Some(actor_id))
            .map(|i| i.instance_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_at_least_20_artifacts() {
        let reg = ArtifactRegistry::default_registry();
        assert!(reg.specs.len() >= 20, "expected ≥20 artifacts; got {}", reg.specs.len());
    }

    #[test]
    fn spec_named_artifacts_present() {
        let reg = ArtifactRegistry::default_registry();
        let names = [
            "stone_blood",
            "soul",
            "goldfish",
            "compass",
            "bubble",
            "snowflake",
            "flame",
            "wrench",
            "battery",
        ];
        for n in names {
            assert!(reg.lookup(n).is_some(), "missing spec-named artifact {n}");
        }
    }

    #[test]
    fn stone_blood_adds_20_hp() {
        let reg = ArtifactRegistry::default_registry();
        let stone = reg.lookup("stone_blood").expect("stone_blood present");
        assert!((stone.bonus.max_hp_bonus - 20.0).abs() < 1e-3);
    }

    #[test]
    fn pickup_then_drop_round_trips_world_position() {
        let reg = ArtifactRegistry::default_registry();
        let mut world = ArtifactWorld::new();
        let spawn = world.spawn(&reg, "compass", [10.0, 20.0], 0, None).expect("spawned");
        let (pickup_ev, _carry_ev) = world.pickup(&reg, spawn.instance_id, 7).expect("picked up");
        assert_eq!(pickup_ev.actor_id, 7);
        let dropped = world.drop_at(spawn.instance_id, [50.0, 60.0]).expect("dropped");
        assert_eq!(dropped.world_position, Some([50.0, 60.0]));
        assert!(dropped.carrier.is_none());
    }

    #[test]
    fn carry_bonus_aggregates_multiple_artifacts() {
        let reg = ArtifactRegistry::default_registry();
        let mut world = ArtifactWorld::new();
        let stone = world.spawn(&reg, "stone_blood", [0.0, 0.0], 0, None).expect("spawn");
        let soul = world.spawn(&reg, "soul", [0.0, 0.0], 0, None).expect("spawn");
        world.pickup(&reg, stone.instance_id, 1).expect("pickup");
        world.pickup(&reg, soul.instance_id, 1).expect("pickup");
        let agg = world.aggregate_bonus_for_actor(&reg, 1);
        assert!((agg.max_hp_bonus - 20.0).abs() < 1e-3);
        assert!((agg.aim_accuracy_bonus_pct - 0.10).abs() < 1e-3);
    }

    #[test]
    fn rarity_tiers_cover_5_levels() {
        let reg = ArtifactRegistry::default_registry();
        let mut found = [false; 5];
        for spec in reg.specs.values() {
            match spec.rarity {
                ArtifactRarity::Common => found[0] = true,
                ArtifactRarity::Magic => found[1] = true,
                ArtifactRarity::Rare => found[2] = true,
                ArtifactRarity::Legendary => found[3] = true,
                ArtifactRarity::Unique => found[4] = true,
            }
        }
        assert!(found.iter().all(|f| *f), "expected all 5 rarities populated, got {found:?}");
    }

    #[test]
    fn compass_reveals_anomalies() {
        let reg = ArtifactRegistry::default_registry();
        let compass = reg.lookup("compass").expect("compass present");
        assert!(compass.bonus.reveals_anomalies);
    }
}
