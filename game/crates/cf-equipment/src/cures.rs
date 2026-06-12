//! M16B — cure item families. Each cure item treats one or more diseases
//! (drug + dose schedule). Loaded from `content/cures/*.ron`, with a
//! hardcoded catalog derived from the disease registry for boot.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cf_disease::{DiseaseKind, DiseaseRegistry, TreatmentKind};
use serde::{Deserialize, Serialize};

/// One cure item family (the physical drug / kit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CureItemSpec {
    pub item_id: String,
    pub display_name: String,
    pub treatment_kind: TreatmentKind,
    pub treats: Vec<DiseaseKind>,
    pub dose_count: u8,
    pub dose_interval_hours: f32,
    pub tier: u8,
}

/// The 9 launch cure families per spec § Files.
pub fn default_cure_catalog() -> Vec<CureItemSpec> {
    let reg = DiseaseRegistry::default_registry();
    let dose = |kind: DiseaseKind| {
        let c = &reg.lookup(kind).cure;
        (c.dose_count, c.dose_interval_hours)
    };
    let pneu = dose(DiseaseKind::Pneumonia);
    let tb = dose(DiseaseKind::Tuberculosis);
    let antiviral = dose(DiseaseKind::Flu);
    let chel = dose(DiseaseKind::RadiationSickness);
    let ivr = dose(DiseaseKind::Cholera);
    let antitox = dose(DiseaseKind::Anthrax);
    let tet = dose(DiseaseKind::Tetanus);
    let inhaler = dose(DiseaseKind::Slimelung);
    let chemo = dose(DiseaseKind::Cancer);
    vec![
        CureItemSpec {
            item_id: "antibiotic_course_t1".to_string(),
            display_name: "Antibiotic Course T1".to_string(),
            treatment_kind: TreatmentKind::Antibiotic,
            treats: vec![DiseaseKind::Pneumonia, DiseaseKind::Typhoid],
            dose_count: pneu.0,
            dose_interval_hours: pneu.1,
            tier: 1,
        },
        CureItemSpec {
            item_id: "antibiotic_course_t2".to_string(),
            display_name: "Antibiotic Course T2".to_string(),
            treatment_kind: TreatmentKind::Antibiotic,
            treats: vec![
                DiseaseKind::Tuberculosis,
                DiseaseKind::BubonicPlague,
                DiseaseKind::Sepsis,
            ],
            dose_count: tb.0,
            dose_interval_hours: tb.1,
            tier: 2,
        },
        CureItemSpec {
            item_id: "antiviral_t1".to_string(),
            display_name: "Antiviral Course T1".to_string(),
            treatment_kind: TreatmentKind::Antiviral,
            treats: vec![DiseaseKind::Flu, DiseaseKind::InfluenzaPandemic],
            dose_count: antiviral.0,
            dose_interval_hours: antiviral.1,
            tier: 1,
        },
        CureItemSpec {
            item_id: "chelation_injection".to_string(),
            display_name: "Chelation Injection".to_string(),
            treatment_kind: TreatmentKind::Chelation,
            treats: vec![DiseaseKind::RadiationSickness],
            dose_count: chel.0,
            dose_interval_hours: chel.1,
            tier: 2,
        },
        CureItemSpec {
            item_id: "iv_rehydration".to_string(),
            display_name: "IV Rehydration".to_string(),
            treatment_kind: TreatmentKind::Rehydration,
            treats: vec![DiseaseKind::Cholera, DiseaseKind::FoodPoisoning],
            dose_count: ivr.0,
            dose_interval_hours: ivr.1,
            tier: 1,
        },
        CureItemSpec {
            item_id: "antitoxin_anthrax".to_string(),
            display_name: "Anthrax Antitoxin".to_string(),
            treatment_kind: TreatmentKind::Antitoxin,
            treats: vec![DiseaseKind::Anthrax],
            dose_count: antitox.0,
            dose_interval_hours: antitox.1,
            tier: 2,
        },
        CureItemSpec {
            item_id: "immunoglobulin_tetanus".to_string(),
            display_name: "Tetanus Immunoglobulin".to_string(),
            treatment_kind: TreatmentKind::Immunoglobulin,
            treats: vec![DiseaseKind::Tetanus],
            dose_count: tet.0,
            dose_interval_hours: tet.1,
            tier: 2,
        },
        CureItemSpec {
            item_id: "bronchial_inhaler".to_string(),
            display_name: "Bronchial Inhaler".to_string(),
            treatment_kind: TreatmentKind::Inhaler,
            treats: vec![DiseaseKind::Slimelung],
            dose_count: inhaler.0,
            dose_interval_hours: inhaler.1,
            tier: 1,
        },
        CureItemSpec {
            item_id: "chemotherapy_kit".to_string(),
            display_name: "Chemotherapy Kit".to_string(),
            treatment_kind: TreatmentKind::Chemotherapy,
            treats: vec![DiseaseKind::Cancer],
            dose_count: chemo.0,
            dose_interval_hours: chemo.1,
            tier: 3,
        },
    ]
}

/// Find the cure item that treats `kind`, if any.
pub fn cure_item_for(catalog: &[CureItemSpec], kind: DiseaseKind) -> Option<&CureItemSpec> {
    catalog.iter().find(|c| c.treats.contains(&kind))
}

/// Load `content/cures/*.ron`, keyed by item id. Missing dir → default.
pub fn load_cure_dir(dir: &Path) -> Result<BTreeMap<String, CureItemSpec>, CureLoadError> {
    let mut out: BTreeMap<String, CureItemSpec> = default_cure_catalog()
        .into_iter()
        .map(|c| (c.item_id.clone(), c))
        .collect();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir).map_err(|e| CureLoadError::Io(dir.to_path_buf(), e.to_string()))?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|e| CureLoadError::Io(path.clone(), e.to_string()))?;
        match ron::from_str::<CureItemSpec>(&body) {
            Ok(c) => {
                out.insert(c.item_id.clone(), c);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "cure spec parse failed");
                return Err(CureLoadError::Parse(path.clone(), e.to_string()));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CureLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_cure_families() {
        assert_eq!(default_cure_catalog().len(), 9);
    }

    #[test]
    fn pneumonia_cure_is_14_dose_antibiotic() {
        let catalog = default_cure_catalog();
        let cure = cure_item_for(&catalog, DiseaseKind::Pneumonia).unwrap();
        assert_eq!(cure.item_id, "antibiotic_course_t1");
        assert_eq!(cure.dose_count, 14);
        assert_eq!(cure.treatment_kind, TreatmentKind::Antibiotic);
    }

    #[test]
    fn every_curable_disease_has_a_cure_item() {
        let catalog = default_cure_catalog();
        // Diseases whose cure is an item (not pure bed-rest/therapy).
        for d in [
            DiseaseKind::Slimelung,
            DiseaseKind::FoodPoisoning,
            DiseaseKind::RadiationSickness,
            DiseaseKind::Flu,
            DiseaseKind::Pneumonia,
            DiseaseKind::Tuberculosis,
            DiseaseKind::Cholera,
            DiseaseKind::Typhoid,
            DiseaseKind::Tetanus,
            DiseaseKind::BubonicPlague,
            DiseaseKind::Anthrax,
            DiseaseKind::Cancer,
            DiseaseKind::Sepsis,
            DiseaseKind::InfluenzaPandemic,
        ] {
            assert!(cure_item_for(&catalog, d).is_some(), "no cure item treats {}", d.as_str());
        }
    }

    #[test]
    fn round_trips_through_ron() {
        for c in default_cure_catalog() {
            let s = ron::to_string(&c).unwrap();
            let back: CureItemSpec = ron::from_str(&s).unwrap();
            assert_eq!(c, back);
        }
    }

    #[test]
    fn every_disease_cure_item_resolves_to_a_real_item() {
        // Each disease's cure.item_required (when Some) must be a known cure
        // item OR a known vaccine item (rabies post-exposure series). This
        // guards against dangling item ids like the old `rehydration_salts`.
        let reg = DiseaseRegistry::default_registry();
        let cure_ids: std::collections::BTreeSet<String> =
            default_cure_catalog().into_iter().map(|c| c.item_id).collect();
        let vaccine_ids: std::collections::BTreeSet<String> = crate::default_vaccine_catalog()
            .into_iter()
            .map(|v| v.item_id)
            .collect();
        for &kind in DiseaseKind::all() {
            if let Some(item) = reg.lookup(kind).cure.item_required.as_ref() {
                assert!(
                    cure_ids.contains(item) || vaccine_ids.contains(item),
                    "{} cure references unknown item `{item}`",
                    kind.as_str()
                );
            }
        }
    }

    #[test]
    fn cure_catalog_treats_match_disease_cure_items() {
        // If the catalog says item X treats disease D, then D's cure must
        // reference item X (no divergence between the two sources of truth).
        let reg = DiseaseRegistry::default_registry();
        for cure in default_cure_catalog() {
            for &disease in &cure.treats {
                assert_eq!(
                    reg.lookup(disease).cure.item_required.as_deref(),
                    Some(cure.item_id.as_str()),
                    "{} cure item disagrees with catalog item {}",
                    disease.as_str(),
                    cure.item_id
                );
            }
        }
    }
}
