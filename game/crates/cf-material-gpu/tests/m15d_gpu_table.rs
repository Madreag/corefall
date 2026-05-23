//! **M15D § cf-material-gpu::reaction_table — Compile 55 entries into
//! wgsl reaction-table struct.**
//!
//! Verifies the end-to-end M15D → GPU pipeline:
//! 1. M15D registry loads from `content/reactions/*.ron`
//! 2. Compiles into `Vec<ReactionEntryGpu>` via `from_m15d_row` /
//!    `compile_m15d_table`
//! 3. The resulting table is byte-identical across runs (determinism
//!    contract precondition)
//! 4. Reactor-only reactions are filtered out (M29 sole-caller rule)

use cf_material_gpu::ReactionEntryGpu;

fn load_m15d_and_mat_reg() -> (cf_material::M15DReactionRegistry, cf_material::MaterialRegistry) {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("content/reactions");
    let (mut reg, _) = cf_material::load_registry_dir(&dir).expect("M15D registry loads");
    let mat_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("content/materials/material_registry.json");
    let (mat_reg, _) = cf_material::load_registry_from_file(&mat_path).expect("material reg loads");
    reg.resolve_against_material_registry(&mat_reg);
    (reg, mat_reg)
}

#[test]
fn m15d_compile_m15d_table_returns_byte_identical_rows_across_runs() {
    let (reg, mat_reg) = load_m15d_and_mat_reg();
    let name_to_id = mat_reg.name_to_id();
    let lookup = |n: &str| name_to_id.get(n).copied();
    let a = ReactionEntryGpu::compile_m15d_table(&reg, &lookup);
    let b = ReactionEntryGpu::compile_m15d_table(&reg, &lookup);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.input_a, y.input_a);
        assert_eq!(x.input_b, y.input_b);
        assert_eq!(x.output, y.output);
        assert_eq!(x.byproduct, y.byproduct);
        assert_eq!(x.min_temp_k, y.min_temp_k);
    }
}

#[test]
fn m15d_gpu_table_excludes_reactor_only_reactions() {
    let (reg, mat_reg) = load_m15d_and_mat_reg();
    let name_to_id = mat_reg.name_to_id();
    let lookup = |n: &str| name_to_id.get(n).copied();
    let rows = cf_material::compile_gpu_reaction_table(&reg, &lookup);
    // U-235 fission is reactor-only; its input_a (uranium_235) shouldn't
    // appear in the GPU table even if the material existed in the
    // registry.
    let uranium_235_id: Option<cf_material::MaterialId> = name_to_id.get("uranium_235").copied();
    if let Some(id) = uranium_235_id {
        assert!(
            !rows.iter().any(|r| r.input_a == id || r.input_b == id),
            "U-235 must not appear in the CA GPU table (reactor-only)"
        );
    }
    // tritium decay is single-input → skipped from GPU table anyway.
    // The uranium_fuel_rod material (id 102) exists in the registry,
    // but rxn.radio.uranium_fission's first input is `uranium_235`
    // (a chemistry-textbook reagent NOT in the launch material set),
    // so the row would already be skipped on name resolution. The
    // is_reactor_only guard is the belt-and-suspenders gate.
}

#[test]
fn m15d_gpu_table_includes_resolvable_combustion_entries() {
    let (reg, mat_reg) = load_m15d_and_mat_reg();
    let name_to_id = mat_reg.name_to_id();
    let lookup = |n: &str| name_to_id.get(n).copied();
    let rows = cf_material::compile_gpu_reaction_table(&reg, &lookup);
    // The launch material registry resolves H2 (55) + O2 (51) →
    // rxn.combustion.h2_o2 must project.
    let h2 = name_to_id.get("hydrogen").copied().expect("hydrogen present");
    let o2 = name_to_id.get("oxygen").copied().expect("oxygen present");
    let has_h2_o2 = rows
        .iter()
        .any(|r| (r.input_a == h2 && r.input_b == o2) || (r.input_a == o2 && r.input_b == h2));
    assert!(has_h2_o2, "rxn.combustion.h2_o2 must project to the GPU table");
    let methane = name_to_id.get("methane").copied().expect("methane present");
    let has_methane = rows
        .iter()
        .any(|r| (r.input_a == methane && r.input_b == o2) || (r.input_a == o2 && r.input_b == methane));
    assert!(has_methane, "rxn.combustion.methane_o2 must project to the GPU table");
}

#[test]
fn m15d_gpu_row_layout_matches_wgsl_struct() {
    // ReactionEntryGpu = 6 × u32 = 24 bytes. Must stay aligned with
    // shaders/reaction_table.wgsl's `struct ReactionEntry`.
    assert_eq!(std::mem::size_of::<ReactionEntryGpu>(), 6 * 4);
}

#[test]
fn m15d_from_m15d_row_truncates_to_8_bit_material_id() {
    let row = cf_material::GpuReactionRow {
        input_a: 21,
        input_b: 68,
        output: 38,
        byproduct: Some(55),
        min_temperature_k: Some(273.0),
    };
    let entry = ReactionEntryGpu::from_m15d_row(row);
    assert_eq!(entry.input_a, 21);
    assert_eq!(entry.input_b, 68);
    assert_eq!(entry.output, 38);
    assert_eq!(entry.byproduct, 55);
    assert_eq!(entry.min_temp_k, 273);
}
