//! **M9C closure feature cross-area tests** — drives the M9B trench
//! template grammar against the post-M9C build to confirm:
//!
//! - **VAL-CROSS-001** — `forward_outpost_with_mgnest` resolves the
//!   embedded `mg_nest_static` placeholder to a real placed
//!   fortification (no `missing_fortification` warning).
//! - **VAL-CROSS-NEW-014** — every M9B trench template's non-MG-nest
//!   M9C placeholders (`sandbag_mid`, `ammo_box_mg`, …) also
//!   resolve cleanly post-M9C.
//! - **VAL-CROSS-003** — `parapet_raised` dig does NOT emit
//!   `requires_m9c=true` post-M9C (the m9c feature flag is on
//!   default + cf-trench's `parapet_raised_dig_validate` returns Ok).
//!
//! Uses `cf_content::TrenchTemplate::instantiate` with the
//! engine-side `resolved_fortifications_for_build` helper's analogue:
//! every fortification kind in `cf_fortification::FortificationKind::ALL`
//! must be in the resolved set.

use std::collections::HashSet;
use std::path::PathBuf;

use cf_content::{TrenchTemplate, TrenchTemplateInstantiation};

fn game_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("game root walking up from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn resolved_fortifications_for_build() -> HashSet<String> {
    cf_fortification::FortificationKind::ALL
        .iter()
        .map(|k| k.as_str().to_string())
        .collect()
}

fn load_template(id: &str) -> TrenchTemplate {
    let path = game_root().join(format!("content/trench_templates/{id}.trench.ron"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    TrenchTemplate::from_ron_str(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// **VAL-CROSS-001**: the `forward_outpost_with_mgnest` template
/// resolves its embedded `mg_nest_static` placeholder to a real
/// placed fortification post-M9C; no `missing_fortification` warning
/// fires for that id.
#[test]
fn forward_outpost_with_mgnest_resolves_mg_nest_static_post_m9c() {
    let template = load_template("forward_outpost_with_mgnest");
    let request = TrenchTemplateInstantiation {
        template: &template,
        origin: (50, 30),
        resolved_fortifications: resolved_fortifications_for_build(),
        instance_id_base: 1_000,
    };
    let inst = template.instantiate(&request);

    assert!(
        inst.missing_fortifications.is_empty(),
        "VAL-CROSS-001: post-M9C the forward_outpost_with_mgnest template must \
         resolve every placeholder; got {} missing: {:?}",
        inst.missing_fortifications.len(),
        inst.missing_fortifications
    );
    let mg_nest_count = inst
        .placed_fortifications
        .iter()
        .filter(|p| p.fortification_id == "mg_nest_static")
        .count();
    assert!(
        mg_nest_count >= 1,
        "VAL-CROSS-001: at least one mg_nest_static instance must be placed; \
         placed_fortifications: {:?}",
        inst.placed_fortifications
    );
    for placed in &inst.placed_fortifications {
        assert!(
            placed.instance_id > 0,
            "VAL-CROSS-001: placed fortification must carry a non-zero instance_id"
        );
    }
}

/// **VAL-CROSS-NEW-014**: every M9B trench template that references
/// non-MG-nest M9C placeholders resolves all of them post-M9C —
/// nothing degrades to a warning. Tests all 4 launch templates so a
/// regression in any one fires.
#[test]
fn every_m9b_template_resolves_all_m9c_placeholders_post_m9c() {
    let templates = [
        "wwi_frontline_a",
        "wwi_frontline_b_two_line",
        "reactor_defense_zigzag",
        "forward_outpost_with_mgnest",
    ];
    for id in templates {
        let template = load_template(id);
        let request = TrenchTemplateInstantiation {
            template: &template,
            origin: (50, 30),
            resolved_fortifications: resolved_fortifications_for_build(),
            instance_id_base: 2_000,
        };
        let inst = template.instantiate(&request);
        assert!(
            inst.missing_fortifications.is_empty(),
            "VAL-CROSS-NEW-014: template `{id}` must resolve every M9C placeholder \
             post-M9C; got {} missing: {:?}",
            inst.missing_fortifications.len(),
            inst.missing_fortifications
        );
    }
}

/// **VAL-CROSS-003**: `parapet_raised` dig validates post-M9C
/// without emitting the `requires_m9c=true` warning (the m9c
/// feature flag is default-on; `parapet_raised_dig_validate` returns
/// `Ok(())`).
#[test]
fn parapet_raised_dig_validate_post_m9c_is_ok() {
    let result = cf_trench::parapet_raised_forward_compat::parapet_raised_dig_validate();
    assert!(
        result.is_ok(),
        "VAL-CROSS-003: parapet_raised_dig_validate must return Ok(()) post-M9C \
         (m9c Cargo feature default-on); got {result:?}"
    );
}

/// **VAL-CROSS-NEW-014 / cross-check**: the engine-side resolver
/// returns all 23 canonical M9C fortification kinds. The test asserts
/// the set is non-empty AND contains every kind the M9B template
/// grammar might reference.
#[test]
fn resolved_fortifications_for_build_returns_all_23_kinds() {
    let resolved = resolved_fortifications_for_build();
    assert_eq!(
        resolved.len(),
        23,
        "VAL-CROSS-NEW-014: resolved set must contain all 23 M9C fortification kinds"
    );
    for required in [
        "mg_nest_static",
        "ammo_box_mg",
        "sandbag_mid",
        "sandbag_high",
        "watchtower_t1",
        "barbed_wire",
        "bunker_firing_slit",
    ] {
        assert!(
            resolved.contains(required),
            "VAL-CROSS-NEW-014: resolved set missing `{required}`; got {resolved:?}"
        );
    }
}
