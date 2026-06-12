//! M16B — vaccine item families. Each maps to the disease it prevents,
//! carrying procurement + side-effect + immunity-duration metadata. Derived
//! from the `cf-disease` registry and overridable via
//! `content/vaccines/*.ron`.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cf_disease::{DiseaseKind, DiseaseRegistry, VaccineProcurement};
use serde::{Deserialize, Serialize};

/// One vaccine item family.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaccineItemSpec {
    pub item_id: String,
    pub display_name: String,
    pub prevents: DiseaseKind,
    pub immunity_duration_seconds: f32,
    pub side_effect_chance: f32,
    pub doses_required: u8,
    pub procurement: VaccineProcurement,
    #[serde(default)]
    pub manufacture_lead_seconds: f32,
}

impl VaccineItemSpec {
    /// True when the vaccine must be manufactured before it is available
    /// (pandemic vaccine — delayed manufacture).
    pub fn is_delayed(&self) -> bool {
        matches!(self.procurement, VaccineProcurement::DelayedManufacture)
    }
}

/// Build the vaccine catalog from the disease registry — one entry per
/// disease that declares a vaccine.
pub fn vaccine_catalog(registry: &DiseaseRegistry) -> Vec<VaccineItemSpec> {
    let mut out = Vec::new();
    for &kind in DiseaseKind::all() {
        if let Some(v) = registry.get(kind).and_then(|s| s.vaccine.as_ref()) {
            out.push(VaccineItemSpec {
                item_id: v.vaccine_id.clone(),
                display_name: v.display_name.clone(),
                prevents: kind,
                immunity_duration_seconds: v.immunity_duration_seconds,
                side_effect_chance: v.side_effect_chance,
                doses_required: v.doses_required,
                procurement: v.procurement,
                manufacture_lead_seconds: v.manufacture_lead_seconds,
            });
        }
    }
    out
}

/// The default catalog (built from `DiseaseRegistry::default_registry`).
pub fn default_vaccine_catalog() -> Vec<VaccineItemSpec> {
    vaccine_catalog(&DiseaseRegistry::default_registry())
}

/// Load `content/vaccines/*.ron`, keyed by item id. Missing dir → default
/// catalog.
pub fn load_vaccine_dir(dir: &Path) -> Result<BTreeMap<String, VaccineItemSpec>, VaccineLoadError> {
    let mut out: BTreeMap<String, VaccineItemSpec> = default_vaccine_catalog()
        .into_iter()
        .map(|v| (v.item_id.clone(), v))
        .collect();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir).map_err(|e| VaccineLoadError::Io(dir.to_path_buf(), e.to_string()))?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|e| VaccineLoadError::Io(path.clone(), e.to_string()))?;
        match ron::from_str::<VaccineItemSpec>(&body) {
            Ok(v) => {
                out.insert(v.item_id.clone(), v);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "vaccine spec parse failed");
                return Err(VaccineLoadError::Parse(path.clone(), e.to_string()));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum VaccineLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_covers_vaccine_diseases() {
        let catalog = default_vaccine_catalog();
        let prevented: Vec<DiseaseKind> = catalog.iter().map(|v| v.prevents).collect();
        for d in [
            DiseaseKind::Flu,
            DiseaseKind::Tuberculosis,
            DiseaseKind::Cholera,
            DiseaseKind::Typhoid,
            DiseaseKind::Rabies,
            DiseaseKind::Tetanus,
            DiseaseKind::BubonicPlague,
            DiseaseKind::Anthrax,
            DiseaseKind::InfluenzaPandemic,
        ] {
            assert!(prevented.contains(&d), "missing vaccine for {}", d.as_str());
        }
        assert!(catalog.len() >= 9);
    }

    #[test]
    fn pandemic_vaccine_is_delayed() {
        let catalog = default_vaccine_catalog();
        let pandemic = catalog
            .iter()
            .find(|v| v.prevents == DiseaseKind::InfluenzaPandemic)
            .unwrap();
        assert!(pandemic.is_delayed());
        assert!(pandemic.manufacture_lead_seconds > 0.0);
    }

    #[test]
    fn round_trips_through_ron() {
        for v in default_vaccine_catalog() {
            let s = ron::to_string(&v).unwrap();
            let back: VaccineItemSpec = ron::from_str(&s).unwrap();
            assert_eq!(v, back);
        }
    }
}
