//! Emits `content/cures/*.ron`, `content/vaccines/*.ron`, and
//! `content/equipment/medical_scanner_t1.ron` from the canonical catalogs so
//! the on-disk content is guaranteed to parse via the loaders.
//! Run: `cargo run -p cf-equipment --example gen_disease_content`.

use std::{fs, path::PathBuf};

use cf_disease::DiseaseKind;
use cf_equipment::{default_cure_catalog, default_vaccine_catalog, MedicalScannerSpec};

fn cfg() -> ron::ser::PrettyConfig {
    ron::ser::PrettyConfig::new()
        .struct_names(false)
        .separate_tuple_members(true)
        .indentor("  ".to_string())
}

fn content_dir(sub: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content").join(sub);
    fs::create_dir_all(&p).unwrap();
    p
}

fn vaccine_filename(kind: DiseaseKind) -> Option<&'static str> {
    Some(match kind {
        DiseaseKind::Flu => "flu",
        DiseaseKind::Tuberculosis => "tb",
        DiseaseKind::Cholera => "cholera",
        DiseaseKind::Typhoid => "typhoid",
        DiseaseKind::Rabies => "rabies",
        DiseaseKind::Tetanus => "tetanus",
        DiseaseKind::BubonicPlague => "plague",
        DiseaseKind::Anthrax => "anthrax",
        DiseaseKind::InfluenzaPandemic => "pandemic",
        _ => return None,
    })
}

fn main() {
    let cures = content_dir("cures");
    for cure in default_cure_catalog() {
        let body = ron::ser::to_string_pretty(&cure, cfg()).unwrap();
        let path = cures.join(format!("{}.ron", cure.item_id));
        fs::write(&path, body + "\n").unwrap();
        println!("wrote {}", path.display());
    }

    let vaccines = content_dir("vaccines");
    for vaccine in default_vaccine_catalog() {
        let Some(name) = vaccine_filename(vaccine.prevents) else {
            continue;
        };
        let body = ron::ser::to_string_pretty(&vaccine, cfg()).unwrap();
        let path = vaccines.join(format!("{name}.ron"));
        fs::write(&path, body + "\n").unwrap();
        println!("wrote {}", path.display());
    }

    let equipment = content_dir("equipment");
    let scanner = MedicalScannerSpec::t1_default();
    let body = ron::ser::to_string_pretty(&scanner, cfg()).unwrap();
    let path = equipment.join("medical_scanner_t1.ron");
    fs::write(&path, body + "\n").unwrap();
    println!("wrote {}", path.display());
}
