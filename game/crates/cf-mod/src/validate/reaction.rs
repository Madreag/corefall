//! M15D § cf-mod validator for `content/reactions/*.ron`.
//!
//! Enforces mass-balance per spec § Acceptance criteria:
//! > Given a mod ships content/reactions/custom_alchemy.ron with
//! >   inputs_mass != outputs_mass
//! > When cf-mod validation runs
//! > Then reaction_mass_balance_violation fires with a diagnostic on stderr
//! > And the mod's reaction is excluded
//! > And the launch 55-reaction registry remains intact

use std::{fs, path::Path};

use cf_material::{registry_molar_mass_lookup, MaterialRegistry, ReactionDef, M15DReactionRegistry};

use crate::report::ValidationReport;

const TOLERANCE: f32 = M15DReactionRegistry::MASS_BALANCE_TOLERANCE_G_PER_MOL;

/// Validate a single `.ron` reaction file. Reports PASS when the
/// reaction parses + mass-balance closes within 0.01 g/mol; reports
/// FAIL with the `reaction_mass_balance_violation` diagnostic
/// otherwise. Per spec § "Mod with broken mass balance is rejected".
pub(crate) fn validate_reaction_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let rxn: ReactionDef = match ron::from_str(&raw) {
        Ok(r) => r,
        Err(err) => {
            report.add_error(
                path.to_path_buf(),
                format!("ron parse failed: {err}"),
            );
            return;
        }
    };
    // rxn.radio.uranium_fission is mass-defect-exempt per spec (E=mc²).
    if rxn.id == "rxn.radio.uranium_fission" {
        report.add_pass(
            path.to_path_buf(),
            format!("reaction {} loaded (mass-defect-exempt)", rxn.id),
        );
        return;
    }

    let material_lookup = MaterialRegistry::locate_default()
        .and_then(|p| cf_material::load_registry_from_file(&p).ok())
        .map(|(reg, _)| reg);

    let lookup_box: Box<dyn Fn(&str) -> Option<f32>> = match material_lookup {
        Some(reg) => Box::new(move |name: &str| {
            let f = registry_molar_mass_lookup(&reg);
            f(name)
        }),
        None => Box::new(|_| None),
    };

    let inp = rxn.input_mass_g_per_mol(&|n: &str| lookup_box(n));
    let out = rxn.output_mass_g_per_mol(&|n: &str| lookup_box(n));
    let delta = inp - out;
    if delta.abs() > TOLERANCE {
        // Construct the canonical event payload (M15D §
        // reaction.mass_balance_violation) so cfctl / CI tooling can
        // parse the diagnostic as JSON. The validator emits the same
        // structured payload that the runtime registry loader emits
        // when a mod's reaction is rejected.
        let evt = cf_material::reaction_mass_balance_violation_event(
            &rxn.id,
            inp,
            out,
            TOLERANCE,
            Some(path.display().to_string()),
            0,
        );
        let payload = serde_json::to_string(&evt).unwrap_or_default();
        report.add_error(
            path.to_path_buf(),
            format!(
                "reaction_mass_balance_violation: id={}, input_mass={:.4}, output_mass={:.4}, delta={:.4} g/mol (tol={TOLERANCE}); payload={}",
                rxn.id, inp, out, delta, payload
            ),
        );
        return;
    }
    report.add_pass(
        path.to_path_buf(),
        format!(
            "reaction {} balanced: in={:.4} out={:.4} delta={:.4} g/mol",
            rxn.id, inp, out, delta
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::write_tmp;

    #[test]
    fn validates_balanced_h2_o2_reaction() {
        let content = r#"(
    id: "test.balanced.h2_o2",
    display_name: "Test H2 O2",
    inputs: [
        (material: "hydrogen", moles: 2.0, molar_mass_g_per_mol: Some(2.016)),
        (material: "oxygen",   moles: 1.0, molar_mass_g_per_mol: Some(31.998)),
    ],
    outputs: [
        (material: "steam", moles: 2.0, molar_mass_g_per_mol: Some(18.015)),
    ],
    delta_h_kj_per_mol: -483.6,
    activation_energy_kj_per_mol: 50.0,
    rate_constant_per_s: 1.0,
    min_temperature_k: Some(700.0),
    min_pressure_kpa: None,
    catalyst: None,
    variant: Both,
    emits_event: true,
    propagates: true,
    auto_ignite: false,
)
"#;
        let path = write_tmp("rxn_test_balanced.ron", content);
        let mut report = ValidationReport::default();
        validate_reaction_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "balanced reaction must pass");
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn rejects_unbalanced_reaction_with_mass_balance_violation() {
        let content = r#"(
    id: "test.unbalanced.bad",
    display_name: "Test Unbalanced",
    inputs: [
        (material: "iron", moles: 1.0, molar_mass_g_per_mol: Some(55.845)),
    ],
    outputs: [
        (material: "rust", moles: 99.0, molar_mass_g_per_mol: Some(159.687)),
    ],
    delta_h_kj_per_mol: 0.0,
    activation_energy_kj_per_mol: 0.0,
    rate_constant_per_s: 0.0,
    min_temperature_k: None,
    min_pressure_kpa: None,
    catalyst: None,
    variant: Both,
    emits_event: true,
    propagates: false,
    auto_ignite: false,
)
"#;
        let path = write_tmp("rxn_test_unbalanced.ron", content);
        let mut report = ValidationReport::default();
        validate_reaction_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "unbalanced reaction must fail");
        let msg = &report.entries[0].message;
        assert!(
            msg.contains("reaction_mass_balance_violation"),
            "error must surface the spec diagnostic; got: {msg}"
        );
    }

    #[test]
    fn rejects_bad_ron_syntax() {
        let path = write_tmp("rxn_test_garbage.ron", "this is not ron");
        let mut report = ValidationReport::default();
        validate_reaction_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("ron parse failed"));
    }
}
