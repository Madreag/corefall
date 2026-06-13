//! M16C — combat stim items with addiction-risk metadata. Stims grant a
//! short combat boost (accuracy / move speed / fear resistance) and carry a
//! per-dose addiction risk; 7+ doses within 30 days drive the M16C Addiction
//! condition (`cf_mental_health::ActorMentalHealth::record_stim_dose`).
//!
//! Loaded from `content/stims/*.ron`, with a hardcoded boot catalog. Mirrors
//! the `cf-equipment::cures` item-spec + `load_dir` pattern.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cf_mental_health::STIM_ADDICTION_RISK_PER_DOSE;
use serde::{Deserialize, Serialize};

/// Stim category. Combat stims are the canonical addiction driver ("7+ doses
/// of any combat stim", spec § Addiction). The `*Stim` postfix is intentional
/// (it drives the `combat_stim` / `focus_stim` / `courage_stim` wire format).
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StimKind {
    /// All-round combat boost (accuracy + move + fear resist).
    CombatStim,
    /// Accuracy-focused (steady aim).
    FocusStim,
    /// Fear / morale boost (suppression + panic resistance).
    CourageStim,
}

impl StimKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StimKind::CombatStim => "combat_stim",
            StimKind::FocusStim => "focus_stim",
            StimKind::CourageStim => "courage_stim",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "combat_stim" => StimKind::CombatStim,
            "focus_stim" => StimKind::FocusStim,
            "courage_stim" => StimKind::CourageStim,
            _ => return None,
        })
    }
}

/// One combat-stim item family (`content/stims/*.ron`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StimItemSpec {
    pub item_id: String,
    pub display_name: String,
    pub kind: StimKind,
    /// Per-dose addiction risk (feeds the M16C Addiction trigger).
    pub addiction_risk_per_dose: f32,
    /// Active duration of the boost (seconds).
    pub duration_seconds: f32,
    /// Additive accuracy bonus while active (fraction).
    pub accuracy_bonus: f32,
    /// Additive move-speed bonus while active (fraction).
    pub move_speed_bonus: f32,
    /// Additive fear / panic resistance while active (fraction).
    pub fear_resist_bonus: f32,
    pub tier: u8,
}

impl StimItemSpec {
    /// True when a dose of this stim counts toward the 30-day addiction window.
    pub fn feeds_addiction_window(&self) -> bool {
        self.addiction_risk_per_dose > 0.0
    }
}

/// The 4 launch combat-stim families (spec § Files).
pub fn default_stim_catalog() -> Vec<StimItemSpec> {
    vec![
        StimItemSpec {
            item_id: "combat_stim_t1".to_string(),
            display_name: "Combat Stim T1".to_string(),
            kind: StimKind::CombatStim,
            addiction_risk_per_dose: STIM_ADDICTION_RISK_PER_DOSE,
            duration_seconds: 60.0,
            accuracy_bonus: 0.10,
            move_speed_bonus: 0.10,
            fear_resist_bonus: 0.20,
            tier: 1,
        },
        StimItemSpec {
            item_id: "combat_stim_t2".to_string(),
            display_name: "Combat Stim T2".to_string(),
            kind: StimKind::CombatStim,
            addiction_risk_per_dose: STIM_ADDICTION_RISK_PER_DOSE,
            duration_seconds: 90.0,
            accuracy_bonus: 0.15,
            move_speed_bonus: 0.15,
            fear_resist_bonus: 0.30,
            tier: 2,
        },
        StimItemSpec {
            item_id: "focus_stim".to_string(),
            display_name: "Focus Stim".to_string(),
            kind: StimKind::FocusStim,
            addiction_risk_per_dose: 0.05,
            duration_seconds: 120.0,
            accuracy_bonus: 0.20,
            move_speed_bonus: 0.0,
            fear_resist_bonus: 0.0,
            tier: 1,
        },
        StimItemSpec {
            item_id: "courage_stim".to_string(),
            display_name: "Courage Stim".to_string(),
            kind: StimKind::CourageStim,
            addiction_risk_per_dose: 0.04,
            duration_seconds: 120.0,
            accuracy_bonus: 0.0,
            move_speed_bonus: 0.0,
            fear_resist_bonus: 0.50,
            tier: 1,
        },
    ]
}

/// Find the stim item with `item_id`, if any.
pub fn stim_item_for<'a>(catalog: &'a [StimItemSpec], item_id: &str) -> Option<&'a StimItemSpec> {
    catalog.iter().find(|s| s.item_id == item_id)
}

/// Load `content/stims/*.ron`, keyed by item id. Missing dir → default.
pub fn load_stim_dir(dir: &Path) -> Result<BTreeMap<String, StimItemSpec>, StimLoadError> {
    let mut out: BTreeMap<String, StimItemSpec> = default_stim_catalog()
        .into_iter()
        .map(|s| (s.item_id.clone(), s))
        .collect();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| StimLoadError::Io(dir.to_path_buf(), e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|e| StimLoadError::Io(path.clone(), e.to_string()))?;
        match ron::from_str::<StimItemSpec>(&body) {
            Ok(s) => {
                out.insert(s.item_id.clone(), s);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "stim spec parse failed");
                return Err(StimLoadError::Parse(path.clone(), e.to_string()));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum StimLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_stim_families() {
        assert_eq!(default_stim_catalog().len(), 4);
    }

    #[test]
    fn combat_stims_carry_the_spec_addiction_risk() {
        let cat = default_stim_catalog();
        let t1 = stim_item_for(&cat, "combat_stim_t1").unwrap();
        assert!((t1.addiction_risk_per_dose - 0.07).abs() < 1e-6);
        assert_eq!(t1.kind, StimKind::CombatStim);
        assert!(t1.feeds_addiction_window());
    }

    #[test]
    fn stim_kind_round_trips() {
        for k in [StimKind::CombatStim, StimKind::FocusStim, StimKind::CourageStim] {
            assert_eq!(StimKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn items_round_trip_through_ron() {
        for s in default_stim_catalog() {
            let r = ron::to_string(&s).unwrap();
            let back: StimItemSpec = ron::from_str(&r).unwrap();
            assert_eq!(s, back);
        }
    }
}
