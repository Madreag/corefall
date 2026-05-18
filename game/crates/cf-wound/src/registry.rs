//! **M14G** § WoundSpec registry.
//!
//! `WoundSpec` is the per-`WoundKind` metadata record loaded from
//! `content/wound_specs/*.ron`. The struct exposes the 11 contract fields
//! from the spec's locked schema:
//!
//! ```text
//! pub struct WoundSpec {
//!     pub kind: WoundKind,
//!     pub bleed_rate_ml_per_s_per_severity: f32,
//!     pub pain_contribution_per_severity: f32,
//!     pub infection_base_chance_per_tick: f32,
//!     pub heal_time_seconds_at_band: [f32; 6],
//!     pub treatment_difficulty: TreatmentDifficulty,
//!     pub allowed_zones: BTreeSet<ZoneId>,
//!     pub decal_id: VisualDecalId,
//!     pub clears_via: BTreeSet<TreatmentKind>,
//!     pub closes_to_scar: bool,
//!     pub forbids_origin: BTreeSet<OriginId>,
//! }
//! ```
//!
//! Loading is via `WoundSpecRegistry::load(dir)` or
//! `WoundSpecRegistry::from_baked_specs()` (the latter pulls the bundled
//! defaults compiled into the workspace).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::severity::SeverityBand;
use crate::WoundKind;

pub const BLEED_RATE_BASE_ML_PER_S_AT_SEVERITY_HALF: f32 = 2.0;
pub const GUNSHOT_THROUGH_BLEED_MULTIPLIER: f32 = 2.0;
pub const BURN3RD_HEAL_SECONDS_AT_SEVERE: f32 = 6.0 * 3600.0;
pub const FRACTURE_HEAL_SECONDS_AT_SEVERE: f32 = 24.0 * 3600.0;
pub const MAX_WOUNDS_PER_ZONE: usize = 5;

/// Stable per-zone identifier — wrapper around a static lowercase snake_case
/// label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ZoneId(pub String);

impl ZoneId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ZoneId {
    fn from(s: &str) -> Self {
        ZoneId(s.to_string())
    }
}

impl From<String> for ZoneId {
    fn from(s: String) -> Self {
        ZoneId(s)
    }
}

impl std::fmt::Display for ZoneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable per-decal identifier. Maps 1:1 to the M45A art pipeline + M11
/// silhouette badge atlas.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VisualDecalId(pub String);

impl VisualDecalId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for VisualDecalId {
    fn from(s: &str) -> Self {
        VisualDecalId(s.to_string())
    }
}

/// Stable per-origin identifier. Origins are the actor archetype identifiers
/// from M17 (`human`, `robot`, `pilot`, `commander`, ...). M14G only consults
/// `forbids_origin` to gate emission per-kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OriginId(pub String);

impl OriginId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for OriginId {
    fn from(s: &str) -> Self {
        OriginId(s.to_string())
    }
}

/// Treatment kind enum — referenced by `WoundSpec.clears_via`. The actual
/// treatment producers ship at M14H; M14G defines the enum so specs can
/// reference treatment kinds by name.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentKind {
    Bandage,
    SutureKit,
    SurgeryKit,
    DebridementKit,
    AntibioticPatch,
    Tourniquet,
    BurnGel,
    FrostbiteHeater,
    AntidoteSerum,
    SplintBone,
    EyePatch,
    EarPlug,
    DentalKit,
    ShrapnelExtractor,
}

impl TreatmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentKind::Bandage => "bandage",
            TreatmentKind::SutureKit => "suture_kit",
            TreatmentKind::SurgeryKit => "surgery_kit",
            TreatmentKind::DebridementKit => "debridement_kit",
            TreatmentKind::AntibioticPatch => "antibiotic_patch",
            TreatmentKind::Tourniquet => "tourniquet",
            TreatmentKind::BurnGel => "burn_gel",
            TreatmentKind::FrostbiteHeater => "frostbite_heater",
            TreatmentKind::AntidoteSerum => "antidote_serum",
            TreatmentKind::SplintBone => "splint_bone",
            TreatmentKind::EyePatch => "eye_patch",
            TreatmentKind::EarPlug => "ear_plug",
            TreatmentKind::DentalKit => "dental_kit",
            TreatmentKind::ShrapnelExtractor => "shrapnel_extractor",
        }
    }
}

/// Treatment difficulty band — referenced by `WoundSpec.treatment_difficulty`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentDifficulty {
    Trivial,
    Easy,
    Moderate,
    Hard,
    Surgical,
    Specialist,
}

impl TreatmentDifficulty {
    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentDifficulty::Trivial => "trivial",
            TreatmentDifficulty::Easy => "easy",
            TreatmentDifficulty::Moderate => "moderate",
            TreatmentDifficulty::Hard => "hard",
            TreatmentDifficulty::Surgical => "surgical",
            TreatmentDifficulty::Specialist => "specialist",
        }
    }
}

/// **M14G** per-wound contract record loaded from `content/wound_specs/*.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WoundSpec {
    pub kind: WoundKind,
    pub bleed_rate_ml_per_s_per_severity: f32,
    pub pain_contribution_per_severity: f32,
    pub infection_base_chance_per_tick: f32,
    pub heal_time_seconds_at_band: [f32; 6],
    pub treatment_difficulty: TreatmentDifficulty,
    pub allowed_zones: BTreeSet<ZoneId>,
    pub decal_id: VisualDecalId,
    pub clears_via: BTreeSet<TreatmentKind>,
    pub closes_to_scar: bool,
    pub forbids_origin: BTreeSet<OriginId>,
}

impl WoundSpec {
    pub fn heal_time_at_band(&self, band: SeverityBand) -> f32 {
        self.heal_time_seconds_at_band[band as usize]
    }

    pub fn is_zone_allowed(&self, zone: &ZoneId) -> bool {
        self.allowed_zones.is_empty() || self.allowed_zones.contains(zone)
    }

    pub fn forbids_origin(&self, origin: &OriginId) -> bool {
        self.forbids_origin.contains(origin)
    }
}

/// **M14G** WoundSpec registry, keyed by `WoundKind`. Provides 1:1
/// kind→spec lookup with deterministic iteration order.
#[derive(Debug, Clone, Default)]
pub struct WoundSpecRegistry {
    pub(crate) by_kind: BTreeMap<WoundKind, WoundSpec>,
}

impl WoundSpecRegistry {
    pub fn new() -> Self {
        Self {
            by_kind: BTreeMap::new(),
        }
    }

    /// Load the registry by walking `dir`/*.ron and parsing each file.
    pub fn load(dir: &std::path::Path) -> Result<Self, WoundSpecError> {
        let mut registry = WoundSpecRegistry::new();
        let entries = std::fs::read_dir(dir).map_err(|e| WoundSpecError::Io(e.to_string()))?;
        let mut paths: Vec<std::path::PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| WoundSpecError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("ron") {
                continue;
            }
            paths.push(path);
        }
        paths.sort();
        for path in paths {
            let raw = std::fs::read_to_string(&path).map_err(|e| WoundSpecError::Io(e.to_string()))?;
            let spec: WoundSpec = ron::from_str(&raw).map_err(|e| WoundSpecError::Parse(format!("{:?}: {}", path, e)))?;
            if registry.by_kind.contains_key(&spec.kind) {
                return Err(WoundSpecError::DuplicateKind(spec.kind));
            }
            registry.by_kind.insert(spec.kind, spec);
        }
        Ok(registry)
    }

    /// Build the registry from compiled-in baked defaults (one spec per
    /// `WoundKind`). Used by callers that don't want to depend on the
    /// filesystem (tests, headless drives, default engine boot).
    pub fn baked_default() -> Self {
        let mut registry = WoundSpecRegistry::new();
        for kind in WoundKind::ALL.iter() {
            registry.by_kind.insert(*kind, default_spec(*kind));
        }
        registry
    }

    pub fn get(&self, kind: WoundKind) -> Option<&WoundSpec> {
        self.by_kind.get(&kind)
    }

    pub fn len(&self) -> usize {
        self.by_kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&WoundKind, &WoundSpec)> {
        self.by_kind.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WoundSpecError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("duplicate WoundKind: {0:?}")]
    DuplicateKind(WoundKind),
    #[error("unknown WoundKind name: {0}")]
    UnknownKindName(String),
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("heal_time_seconds_at_band must have length 6, got {0}")]
    BadHealTimeArrayLength(usize),
}

fn zone_set(zones: &[&str]) -> BTreeSet<ZoneId> {
    zones.iter().map(|z| ZoneId::from(*z)).collect()
}

fn origin_set(origins: &[&str]) -> BTreeSet<OriginId> {
    origins.iter().map(|o| OriginId::from(*o)).collect()
}

fn treatment_set(items: &[TreatmentKind]) -> BTreeSet<TreatmentKind> {
    items.iter().copied().collect()
}

/// Heal-time array for one wound kind — `[Scratch, Light, Moderate, Severe,
/// Critical, Lethal]` in seconds. `Lethal` is conventionally larger but
/// finite so the aging pass has a defined value.
fn heal_array(severe_seconds: f32) -> [f32; 6] {
    [
        severe_seconds * 0.05,
        severe_seconds * 0.1,
        severe_seconds * 0.3,
        severe_seconds,
        severe_seconds * 1.5,
        severe_seconds * 2.0,
    ]
}

const ALL_HUMAN_ZONES: &[&str] = &[
    "head_front",
    "head_back",
    "head",
    "torso_front",
    "torso_back",
    "torso",
    "arm_left",
    "arm_right",
    "forearm_left",
    "forearm_right",
    "hand_left",
    "hand_right",
    "leg_left",
    "leg_right",
    "shin_left",
    "shin_right",
    "foot_left",
    "foot_right",
];

const LIMB_ZONES: &[&str] = &[
    "arm_left",
    "arm_right",
    "forearm_left",
    "forearm_right",
    "hand_left",
    "hand_right",
    "leg_left",
    "leg_right",
    "shin_left",
    "shin_right",
    "foot_left",
    "foot_right",
];

const HEAD_FACE_ZONES: &[&str] = &["head_front", "head_back", "head"];

fn default_spec(kind: WoundKind) -> WoundSpec {
    match kind {
        WoundKind::LacerationLight => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 1.0,
            pain_contribution_per_severity: 0.05,
            infection_base_chance_per_tick: 1e-5,
            heal_time_seconds_at_band: heal_array(30.0 * 60.0),
            treatment_difficulty: TreatmentDifficulty::Trivial,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.laceration.light"),
            clears_via: treatment_set(&[TreatmentKind::Bandage]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::LacerationModerate => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 4.0,
            pain_contribution_per_severity: 0.10,
            infection_base_chance_per_tick: 2e-5,
            heal_time_seconds_at_band: heal_array(2.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.laceration.moderate"),
            clears_via: treatment_set(&[TreatmentKind::Bandage, TreatmentKind::SutureKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::LacerationSevere => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 8.0,
            pain_contribution_per_severity: 0.20,
            infection_base_chance_per_tick: 5e-5,
            heal_time_seconds_at_band: heal_array(8.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.laceration.severe"),
            clears_via: treatment_set(&[TreatmentKind::SutureKit, TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Puncture => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 3.0,
            pain_contribution_per_severity: 0.12,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(3.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.puncture"),
            clears_via: treatment_set(&[TreatmentKind::Bandage, TreatmentKind::SutureKit]),
            closes_to_scar: false,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::StabThrough => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 6.0,
            pain_contribution_per_severity: 0.18,
            infection_base_chance_per_tick: 4e-5,
            heal_time_seconds_at_band: heal_array(6.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.stab_through"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SutureKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::GunshotEntry => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 4.0,
            pain_contribution_per_severity: 0.15,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(4.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.gunshot_entry"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SutureKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::GunshotExit => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 6.0,
            pain_contribution_per_severity: 0.20,
            infection_base_chance_per_tick: 4e-5,
            heal_time_seconds_at_band: heal_array(5.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.gunshot_exit"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SutureKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::GunshotThrough => WoundSpec {
            kind,
            // 2× the GunshotEntry baseline per spec tunable defaults.
            bleed_rate_ml_per_s_per_severity: 4.0 * GUNSHOT_THROUGH_BLEED_MULTIPLIER,
            pain_contribution_per_severity: 0.25,
            infection_base_chance_per_tick: 5e-5,
            heal_time_seconds_at_band: heal_array(8.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.gunshot_through"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SutureKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::ShrapnelEmbedded => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 2.0,
            pain_contribution_per_severity: 0.10,
            infection_base_chance_per_tick: 8e-5,
            heal_time_seconds_at_band: heal_array(6.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.shrapnel_embedded"),
            clears_via: treatment_set(&[TreatmentKind::ShrapnelExtractor, TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::ShrapnelThrough => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 5.0,
            pain_contribution_per_severity: 0.18,
            infection_base_chance_per_tick: 6e-5,
            heal_time_seconds_at_band: heal_array(8.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.shrapnel_through"),
            clears_via: treatment_set(&[TreatmentKind::ShrapnelExtractor, TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::BruiseLight => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.03,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(30.0 * 60.0),
            treatment_difficulty: TreatmentDifficulty::Trivial,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.bruise.light"),
            clears_via: treatment_set(&[TreatmentKind::Bandage]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::BruiseHeavy => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.10,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(2.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.bruise.heavy"),
            clears_via: treatment_set(&[TreatmentKind::Bandage]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::CrushLimb => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 2.0,
            pain_contribution_per_severity: 0.25,
            infection_base_chance_per_tick: 4e-5,
            heal_time_seconds_at_band: heal_array(12.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.crush_limb"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SplintBone]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::Concussion => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.20,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(4.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(HEAD_FACE_ZONES),
            decal_id: VisualDecalId::from("decal.concussion"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit]),
            closes_to_scar: false,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::FractureSimple => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.25,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(FRACTURE_HEAL_SECONDS_AT_SEVERE),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.fracture.simple"),
            clears_via: treatment_set(&[TreatmentKind::SplintBone]),
            closes_to_scar: false,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::FractureCompound => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 3.0,
            pain_contribution_per_severity: 0.35,
            infection_base_chance_per_tick: 1e-4,
            heal_time_seconds_at_band: heal_array(FRACTURE_HEAL_SECONDS_AT_SEVERE * 1.5),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.fracture.compound"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::SplintBone]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::FractureComminuted => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 4.0,
            pain_contribution_per_severity: 0.40,
            infection_base_chance_per_tick: 1e-4,
            heal_time_seconds_at_band: heal_array(FRACTURE_HEAL_SECONDS_AT_SEVERE * 2.0),
            treatment_difficulty: TreatmentDifficulty::Surgical,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.fracture.comminuted"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::Dislocation => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.18,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(2.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.dislocation"),
            clears_via: treatment_set(&[TreatmentKind::SplintBone]),
            closes_to_scar: false,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::SprainStrain => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.10,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(1.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Trivial,
            allowed_zones: zone_set(LIMB_ZONES),
            decal_id: VisualDecalId::from("decal.sprain_strain"),
            clears_via: treatment_set(&[TreatmentKind::Bandage]),
            closes_to_scar: false,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::Burn1st => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.08,
            infection_base_chance_per_tick: 1e-5,
            heal_time_seconds_at_band: heal_array(20.0 * 60.0),
            treatment_difficulty: TreatmentDifficulty::Trivial,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.burn.first"),
            clears_via: treatment_set(&[TreatmentKind::BurnGel]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Burn2nd => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.20,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(2.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.burn.second"),
            clears_via: treatment_set(&[TreatmentKind::BurnGel, TreatmentKind::Bandage]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Burn3rd => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 1.5,
            pain_contribution_per_severity: 0.40,
            infection_base_chance_per_tick: 8e-5,
            heal_time_seconds_at_band: heal_array(BURN3RD_HEAL_SECONDS_AT_SEVERE),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.burn.third"),
            clears_via: treatment_set(&[TreatmentKind::SurgeryKit, TreatmentKind::BurnGel]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Frostbite1st => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.06,
            infection_base_chance_per_tick: 0.0,
            heal_time_seconds_at_band: heal_array(20.0 * 60.0),
            treatment_difficulty: TreatmentDifficulty::Trivial,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.frostbite.first"),
            clears_via: treatment_set(&[TreatmentKind::FrostbiteHeater]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Frostbite2nd => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.15,
            infection_base_chance_per_tick: 1e-5,
            heal_time_seconds_at_band: heal_array(1.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Easy,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.frostbite.second"),
            clears_via: treatment_set(&[TreatmentKind::FrostbiteHeater]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::Frostbite3rd => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.0,
            pain_contribution_per_severity: 0.35,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(4.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Hard,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.frostbite.third"),
            clears_via: treatment_set(&[TreatmentKind::FrostbiteHeater, TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::AcidBurn => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.30,
            infection_base_chance_per_tick: 5e-5,
            heal_time_seconds_at_band: heal_array(4.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.acid_burn"),
            clears_via: treatment_set(&[TreatmentKind::AntidoteSerum, TreatmentKind::BurnGel]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::ChemicalBurn => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.25,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(3.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(ALL_HUMAN_ZONES),
            decal_id: VisualDecalId::from("decal.chemical_burn"),
            clears_via: treatment_set(&[TreatmentKind::AntidoteSerum, TreatmentKind::BurnGel]),
            closes_to_scar: true,
            forbids_origin: origin_set(&[]),
        },
        WoundKind::EyeInjury => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.30,
            infection_base_chance_per_tick: 2e-5,
            heal_time_seconds_at_band: heal_array(8.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Surgical,
            allowed_zones: zone_set(HEAD_FACE_ZONES),
            decal_id: VisualDecalId::from("decal.eye_injury"),
            clears_via: treatment_set(&[TreatmentKind::EyePatch, TreatmentKind::SurgeryKit]),
            closes_to_scar: true,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::EarInjury => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.3,
            pain_contribution_per_severity: 0.18,
            infection_base_chance_per_tick: 1e-5,
            heal_time_seconds_at_band: heal_array(4.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(HEAD_FACE_ZONES),
            decal_id: VisualDecalId::from("decal.ear_injury"),
            clears_via: treatment_set(&[TreatmentKind::EarPlug, TreatmentKind::SurgeryKit]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
        WoundKind::DentalDamage => WoundSpec {
            kind,
            bleed_rate_ml_per_s_per_severity: 0.5,
            pain_contribution_per_severity: 0.20,
            infection_base_chance_per_tick: 3e-5,
            heal_time_seconds_at_band: heal_array(6.0 * 3600.0),
            treatment_difficulty: TreatmentDifficulty::Moderate,
            allowed_zones: zone_set(HEAD_FACE_ZONES),
            decal_id: VisualDecalId::from("decal.dental_damage"),
            clears_via: treatment_set(&[TreatmentKind::DentalKit, TreatmentKind::SurgeryKit]),
            closes_to_scar: false,
            forbids_origin: origin_set(&["robot"]),
        },
    }
}

/// Serialize a registry entry to canonical RON for content baking.
pub fn spec_to_ron(spec: &WoundSpec) -> String {
    ron::ser::to_string_pretty(spec, ron::ser::PrettyConfig::default()).expect("serialize WoundSpec")
}

/// Resolve a `WoundKind` from a candidate emit context with `forbids_origin`
/// substitution. If the kind is forbidden for the actor's origin, the
/// returned `Option<WoundKind>` is the substituted kind per the registry
/// (or `None` to suppress emission entirely).
pub fn resolve_emit_kind(
    registry: &WoundSpecRegistry,
    candidate: WoundKind,
    actor_origin: &OriginId,
) -> Option<WoundKind> {
    if let Some(spec) = registry.get(candidate) {
        if spec.forbids_origin(actor_origin) {
            // VAL-M14G-021: substitute LacerationLight on robots with CrushLimb.
            if matches!(
                candidate,
                WoundKind::LacerationLight
                    | WoundKind::LacerationModerate
                    | WoundKind::LacerationSevere
            ) && actor_origin.as_str() == "robot"
            {
                return Some(WoundKind::CrushLimb);
            }
            // For other forbidden combinations, suppress.
            return None;
        }
    }
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// VAL-M14G-002: WoundSpec round-trips through RON.
    #[test]
    fn woundspec_round_trip_ron() {
        for kind in WoundKind::ALL.iter() {
            let spec = default_spec(*kind);
            let s = ron::to_string(&spec).unwrap();
            let back: WoundSpec = ron::from_str(&s).unwrap();
            assert_eq!(spec, back, "round-trip diverged for {kind:?}");
        }
    }

    /// VAL-M14G-003: registry baked default has one spec per WoundKind.
    #[test]
    fn baked_registry_has_one_spec_per_kind() {
        let registry = WoundSpecRegistry::baked_default();
        assert_eq!(registry.len(), WoundKind::COUNT);
        for kind in WoundKind::ALL.iter() {
            assert!(registry.get(*kind).is_some(), "missing spec for {kind:?}");
        }
    }

    /// VAL-M14G-040: every decal_id in the registry is pairwise distinct.
    #[test]
    fn decal_id_one_to_one_mapping() {
        let registry = WoundSpecRegistry::baked_default();
        let mut seen: HashSet<VisualDecalId> = HashSet::new();
        for (_, spec) in registry.iter() {
            assert!(seen.insert(spec.decal_id.clone()), "duplicate decal {:?}", spec.decal_id);
        }
        assert_eq!(seen.len(), WoundKind::COUNT);
    }

    /// VAL-M14G-021: per-origin forbiddance on robots → substitution to CrushLimb.
    #[test]
    fn origin_forbidden_robot_no_lacerations() {
        let registry = WoundSpecRegistry::baked_default();
        let robot_origin = OriginId::from("robot");
        let kind = resolve_emit_kind(&registry, WoundKind::LacerationLight, &robot_origin);
        assert_eq!(kind, Some(WoundKind::CrushLimb));
    }

    /// VAL-M14G-032: tunable defaults table.
    #[test]
    fn tunable_defaults_match_spec() {
        let registry = WoundSpecRegistry::baked_default();
        let gs_through = registry.get(WoundKind::GunshotThrough).unwrap();
        assert!(
            (gs_through.bleed_rate_ml_per_s_per_severity - 4.0 * GUNSHOT_THROUGH_BLEED_MULTIPLIER).abs() < 1e-6
        );
        let burn3rd = registry.get(WoundKind::Burn3rd).unwrap();
        let severe_h = burn3rd.heal_time_at_band(SeverityBand::Severe);
        assert!((severe_h - BURN3RD_HEAL_SECONDS_AT_SEVERE).abs() < 1e-3);
        let fracture = registry.get(WoundKind::FractureSimple).unwrap();
        let fracture_severe = fracture.heal_time_at_band(SeverityBand::Severe);
        assert!((fracture_severe - FRACTURE_HEAL_SECONDS_AT_SEVERE).abs() < 1e-3);
        assert_eq!(MAX_WOUNDS_PER_ZONE, 5);
        assert!((BLEED_RATE_BASE_ML_PER_S_AT_SEVERITY_HALF - 2.0).abs() < 1e-6);
    }

    /// VAL-M14G-045: heal_time_at_band returns the array element at the
    /// band's discriminant.
    #[test]
    fn heal_time_at_band_index_alignment() {
        let mut spec = default_spec(WoundKind::LacerationLight);
        spec.heal_time_seconds_at_band = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert!((spec.heal_time_at_band(SeverityBand::Scratch) - 1.0).abs() < 1e-6);
        assert!((spec.heal_time_at_band(SeverityBand::Light) - 2.0).abs() < 1e-6);
        assert!((spec.heal_time_at_band(SeverityBand::Moderate) - 3.0).abs() < 1e-6);
        assert!((spec.heal_time_at_band(SeverityBand::Severe) - 4.0).abs() < 1e-6);
        assert!((spec.heal_time_at_band(SeverityBand::Critical) - 5.0).abs() < 1e-6);
        assert!((spec.heal_time_at_band(SeverityBand::Lethal) - 6.0).abs() < 1e-6);
    }

    /// VAL-M14G-044: WoundSpec.allowed_zones gates producer emit.
    #[test]
    fn producer_respects_allowed_zones() {
        let registry = WoundSpecRegistry::baked_default();
        let dental = registry.get(WoundKind::DentalDamage).unwrap();
        assert!(!dental.is_zone_allowed(&ZoneId::from("leg_left")));
        assert!(dental.is_zone_allowed(&ZoneId::from("head_front")));
    }
}
