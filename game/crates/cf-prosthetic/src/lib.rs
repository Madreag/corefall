//! **M14I** — Prosthetic replacement loop.
//!
//! Canonical owner of the prosthetic catalog (10 launch items) plus the
//! `install / maintain / tune` state machine. Storage of the actor's
//! installed prosthetics lives in `cf-actor::long_term`.
//!
//! Tiers:
//! - `T1` — biological / mechanical baseline. Spec § "Prosthetic t1
//!   functional restoration = 70%".
//! - `T2` — cybernetic upgrade. Spec § "Prosthetic t2 functional
//!   restoration = 90%".
//! - `T3` — endgame upgrade (catalog includes T1 / T2 launch items only;
//!   T3 reserved for M48-tier overclock minigame).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::manual_range_contains,
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::uninlined_format_args,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::option_if_let_else,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::should_implement_trait,
    clippy::unnecessary_debug_formatting,
    clippy::unnested_or_patterns,
    clippy::missing_const_for_fn
)]

pub mod install;

pub use install::{
    maintain_prosthetic, InstallError, InstallSession, MaintenanceError, MaintenanceOutcome,
    ProstheticInstance, PROSTHETIC_INSTALL_SECONDS, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS,
    PROSTHETIC_MALFUNCTION_THRESHOLD,
};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_wound::registry::{OriginId, ZoneId};

/// **M14I** § prosthetic tier.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProstheticTier {
    T1 = 0,
    T2 = 1,
    T3 = 2,
}

impl ProstheticTier {
    pub fn as_str(self) -> &'static str {
        match self {
            ProstheticTier::T1 => "t1",
            ProstheticTier::T2 => "t2",
            ProstheticTier::T3 => "t3",
        }
    }

    /// Functional-restoration ratio per tier. Spec table:
    /// T1=0.70, T2=0.90, T3=0.97 (reserved).
    pub fn functional_restoration(self) -> f32 {
        match self {
            ProstheticTier::T1 => 0.70,
            ProstheticTier::T2 => 0.90,
            ProstheticTier::T3 => 0.97,
        }
    }
}

/// **M14I** § canonical prosthetic ids (launch catalog: 10 items).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProstheticKind {
    ProstheticLegT1 = 0,
    ProstheticArmT1 = 1,
    ProstheticEyeT1 = 2,
    ProstheticEarT1 = 3,
    ProstheticToothFullSet = 4,
    CyberneticLegT2 = 5,
    CyberneticArmT2 = 6,
    CyberneticEyeT2Thermal = 7,
    CyberneticLungs = 8,
    CyberneticKidneys = 9,
}

impl ProstheticKind {
    pub const COUNT: usize = 10;

    pub const ALL: [ProstheticKind; Self::COUNT] = [
        ProstheticKind::ProstheticLegT1,
        ProstheticKind::ProstheticArmT1,
        ProstheticKind::ProstheticEyeT1,
        ProstheticKind::ProstheticEarT1,
        ProstheticKind::ProstheticToothFullSet,
        ProstheticKind::CyberneticLegT2,
        ProstheticKind::CyberneticArmT2,
        ProstheticKind::CyberneticEyeT2Thermal,
        ProstheticKind::CyberneticLungs,
        ProstheticKind::CyberneticKidneys,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ProstheticKind::ProstheticLegT1 => "prosthetic_leg_t1",
            ProstheticKind::ProstheticArmT1 => "prosthetic_arm_t1",
            ProstheticKind::ProstheticEyeT1 => "prosthetic_eye_t1",
            ProstheticKind::ProstheticEarT1 => "prosthetic_ear_t1",
            ProstheticKind::ProstheticToothFullSet => "prosthetic_tooth_full_set",
            ProstheticKind::CyberneticLegT2 => "cybernetic_leg_t2",
            ProstheticKind::CyberneticArmT2 => "cybernetic_arm_t2",
            ProstheticKind::CyberneticEyeT2Thermal => "cybernetic_eye_t2_thermal",
            ProstheticKind::CyberneticLungs => "cybernetic_lungs",
            ProstheticKind::CyberneticKidneys => "cybernetic_kidneys",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        for k in &Self::ALL {
            if k.as_str() == s {
                return Some(*k);
            }
        }
        None
    }

    pub fn tier(self) -> ProstheticTier {
        match self {
            ProstheticKind::ProstheticLegT1
            | ProstheticKind::ProstheticArmT1
            | ProstheticKind::ProstheticEyeT1
            | ProstheticKind::ProstheticEarT1
            | ProstheticKind::ProstheticToothFullSet => ProstheticTier::T1,
            ProstheticKind::CyberneticLegT2
            | ProstheticKind::CyberneticArmT2
            | ProstheticKind::CyberneticEyeT2Thermal
            | ProstheticKind::CyberneticLungs
            | ProstheticKind::CyberneticKidneys => ProstheticTier::T2,
        }
    }
}

/// **M14I** § per-prosthetic contract spec record (modder data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProstheticSpec {
    pub kind: ProstheticKind,
    pub display_name: String,
    pub tier: ProstheticTier,
    /// Body zones this prosthetic replaces (e.g. "leg_right").
    pub target_zones: Vec<ZoneId>,
    /// Origins allowed to receive this prosthetic. Spec § "Per-origin
    /// compatibility: only human / android / powered_organic accept
    /// biological prosthetics".
    pub compatible_origins: Vec<OriginId>,
    /// Functional restoration ratio (defaults to tier's value).
    pub functional_restoration: f32,
    /// Maintenance interval seconds (defaults to 7 in-game days =
    /// 7 × 24 × 3600 = 604800 sim seconds).
    pub maintenance_interval_seconds: f32,
    /// Install duration seconds (defaults to 60s per spec table).
    pub install_seconds: f32,
}

impl ProstheticSpec {
    pub fn defaults_for(kind: ProstheticKind) -> Self {
        let tier = kind.tier();
        let (zones, compatibility) = catalog_compatibility(kind);
        Self {
            kind,
            display_name: kind.as_str().to_string(),
            tier,
            target_zones: zones,
            compatible_origins: compatibility,
            functional_restoration: tier.functional_restoration(),
            maintenance_interval_seconds: PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS,
            install_seconds: PROSTHETIC_INSTALL_SECONDS,
        }
    }
}

/// Origin allow-list per prosthetic. Spec § "Per-origin compatibility:
/// only human / android / powered_organic accept biological prosthetics".
/// Cybernetic items add `heavy_biomech` (mechanical-organic hybrids).
fn catalog_compatibility(kind: ProstheticKind) -> (Vec<ZoneId>, Vec<OriginId>) {
    let bio = vec![
        OriginId::from("human"),
        OriginId::from("android_organic_side"),
        OriginId::from("powered_organic"),
    ];
    let cyber = vec![
        OriginId::from("human"),
        OriginId::from("android_organic_side"),
        OriginId::from("powered_organic"),
        OriginId::from("heavy_biomech"),
    ];
    match kind {
        ProstheticKind::ProstheticLegT1 => (
            vec![ZoneId::from("leg_left"), ZoneId::from("leg_right")],
            bio,
        ),
        ProstheticKind::ProstheticArmT1 => (
            vec![ZoneId::from("arm_left"), ZoneId::from("arm_right")],
            bio,
        ),
        ProstheticKind::ProstheticEyeT1 => (vec![ZoneId::from("head")], bio),
        ProstheticKind::ProstheticEarT1 => (vec![ZoneId::from("head")], bio),
        ProstheticKind::ProstheticToothFullSet => (vec![ZoneId::from("head")], bio),
        ProstheticKind::CyberneticLegT2 => (
            vec![ZoneId::from("leg_left"), ZoneId::from("leg_right")],
            cyber,
        ),
        ProstheticKind::CyberneticArmT2 => (
            vec![ZoneId::from("arm_left"), ZoneId::from("arm_right")],
            cyber,
        ),
        ProstheticKind::CyberneticEyeT2Thermal => (vec![ZoneId::from("head")], cyber),
        ProstheticKind::CyberneticLungs => (vec![ZoneId::from("torso_front")], cyber),
        ProstheticKind::CyberneticKidneys => (vec![ZoneId::from("torso_back")], cyber),
    }
}

/// **M14I** § canonical prosthetic catalog. Baked defaults loaded from
/// `content/prosthetics/*.ron` at runtime.
#[must_use]
pub fn prosthetic_catalog() -> Vec<ProstheticSpec> {
    ProstheticKind::ALL
        .iter()
        .copied()
        .map(ProstheticSpec::defaults_for)
        .collect()
}

/// **M14I** § look up the canonical spec for a kind.
#[must_use]
pub fn prosthetic_spec(kind: ProstheticKind) -> ProstheticSpec {
    ProstheticSpec::defaults_for(kind)
}

/// **M14I** § registry loaded from `content/prosthetics/*.ron`.
#[derive(Debug, Clone, Default)]
pub struct ProstheticSpecRegistry {
    pub by_kind: BTreeMap<ProstheticKind, ProstheticSpec>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProstheticSpecError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("duplicate ProstheticKind: {0:?}")]
    DuplicateKind(ProstheticKind),
}

impl ProstheticSpecRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn baked_default() -> Self {
        let mut r = Self::new();
        for s in prosthetic_catalog() {
            r.by_kind.insert(s.kind, s);
        }
        r
    }

    pub fn load(dir: &std::path::Path) -> Result<Self, ProstheticSpecError> {
        let mut registry = Self::new();
        let entries =
            std::fs::read_dir(dir).map_err(|e| ProstheticSpecError::Io(e.to_string()))?;
        let mut paths: Vec<std::path::PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ProstheticSpecError::Io(e.to_string()))?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("ron") {
                continue;
            }
            paths.push(p);
        }
        paths.sort();
        for path in paths {
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| ProstheticSpecError::Io(e.to_string()))?;
            let spec: ProstheticSpec = ron::from_str(&raw)
                .map_err(|e| ProstheticSpecError::Parse(format!("{:?}: {}", path, e)))?;
            if registry.by_kind.contains_key(&spec.kind) {
                return Err(ProstheticSpecError::DuplicateKind(spec.kind));
            }
            registry.by_kind.insert(spec.kind, spec);
        }
        Ok(registry)
    }

    pub fn get(&self, kind: ProstheticKind) -> Option<&ProstheticSpec> {
        self.by_kind.get(&kind)
    }

    pub fn len(&self) -> usize {
        self.by_kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_10_items() {
        let c = prosthetic_catalog();
        assert_eq!(c.len(), 10);
        for k in ProstheticKind::ALL.iter() {
            assert!(c.iter().any(|s| s.kind == *k));
        }
    }

    #[test]
    fn kind_round_trip() {
        for k in ProstheticKind::ALL.iter() {
            let s = k.as_str();
            assert_eq!(ProstheticKind::from_str(s), Some(*k));
        }
    }

    #[test]
    fn t1_70pct_t2_90pct() {
        assert!((ProstheticTier::T1.functional_restoration() - 0.70).abs() < 1e-6);
        assert!((ProstheticTier::T2.functional_restoration() - 0.90).abs() < 1e-6);
    }

    #[test]
    fn robot_origin_blocked_from_bio_prosthetic() {
        let spec = prosthetic_spec(ProstheticKind::ProstheticLegT1);
        assert!(!spec.compatible_origins.contains(&OriginId::from("robot")));
        assert!(spec.compatible_origins.contains(&OriginId::from("human")));
    }
}
