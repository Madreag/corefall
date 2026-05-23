//! M15D § Reaction registry — full 55-reaction matrix with stoichiometry,
//! Arrhenius kinetics, autoignition thresholds, and per-pixel vs.
//! per-cell variants.
//!
//! Loads from `content/reactions/*.ron` — one file per reaction. The
//! schema is **locked**:
//! ```ignore
//! pub struct ReactionDef {
//!   pub id: ReactionId,
//!   pub display_name: String,
//!   pub inputs: Vec<ReactionInput>,
//!   pub outputs: Vec<ReactionOutput>,
//!   pub delta_h_kj_per_mol: f32,
//!   pub activation_energy_kj_per_mol: f32,
//!   pub rate_constant_per_s: f32,
//!   pub min_temperature_k: Option<f32>,
//!   pub min_pressure_kpa: Option<f32>,
//!   pub catalyst: Option<MaterialId>,
//!   pub variant: ReactionVariant,
//!   pub emits_event: bool,
//!   pub propagates: bool,
//!   pub auto_ignite: bool,
//! }
//! ```
//!
//! Mass-balance validation runs at registry load time per spec § Notes.
//! Reactants that don't yet have a registry entry (KNO3, FeCl2, etc.)
//! carry an inline `molar_mass_g_per_mol` override so balance still
//! validates without requiring every chemical formula to ship a full
//! [`crate::MaterialDef`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::MaterialId;

/// One mole-coefficient input to a reaction. Material is identified by
/// canonical snake-case name; the loader resolves it to a
/// [`MaterialId`] when the material is present in the material
/// registry. When not present (e.g. `KNO3`), the per-input
/// `molar_mass_g_per_mol` carries the chemistry-textbook value so mass
/// balance still validates.
///
/// `material_id` is the spec-literal `MaterialId` form, populated by
/// [`M15DReactionRegistry::resolve_against_material_registry`] at load
/// time; absent until the registry is resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionInput {
    pub material: String,
    pub moles: f32,
    #[serde(default)]
    pub molar_mass_g_per_mol: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<MaterialId>,
}

/// One mole-coefficient output. Same shape as [`ReactionInput`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionOutput {
    pub material: String,
    pub moles: f32,
    #[serde(default)]
    pub molar_mass_g_per_mol: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<MaterialId>,
}

/// CA variant. `PerPixel` runs in M15 / M15B per-pixel CA; `PerCell`
/// runs in M19 / M3B per-cell room aggregation; `Both` runs in both
/// kernels and must produce identical aggregate energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ReactionVariant {
    PerPixel,
    PerCell,
    Both,
}

impl Default for ReactionVariant {
    fn default() -> Self {
        ReactionVariant::Both
    }
}

/// Locked schema per M15D spec § "Reaction schema (locked)".
///
/// The spec literal struct lists `catalyst: Option<MaterialId>`. To
/// keep .ron files human-readable (modders write `Some("ice")`, not
/// `Some(15)`) the on-disk field carries the snake-case material name
/// and the spec-literal `catalyst_id: Option<MaterialId>` is populated
/// from the live material registry at load time via
/// [`M15DReactionRegistry::resolve_against_material_registry`]. The
/// `MaterialId`-typed view consumers (CA kernel, GPU table, M19
/// combustion gate) read is `catalyst_id`; the spec literal compliance
/// gate is met by that field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionDef {
    pub id: String,
    pub display_name: String,
    pub inputs: Vec<ReactionInput>,
    pub outputs: Vec<ReactionOutput>,
    pub delta_h_kj_per_mol: f32,
    pub activation_energy_kj_per_mol: f32,
    pub rate_constant_per_s: f32,
    #[serde(default)]
    pub min_temperature_k: Option<f32>,
    #[serde(default)]
    pub min_pressure_kpa: Option<f32>,
    #[serde(default)]
    pub catalyst: Option<String>,
    /// Spec-literal `catalyst: Option<MaterialId>`. Populated post-load
    /// from the material registry; `None` when the catalyst name names
    /// an abstract gate (electricity, altitude, cold) rather than a
    /// concrete material.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalyst_id: Option<MaterialId>,
    #[serde(default)]
    pub variant: ReactionVariant,
    #[serde(default = "default_true")]
    pub emits_event: bool,
    #[serde(default)]
    pub propagates: bool,
    #[serde(default)]
    pub auto_ignite: bool,
}

fn default_true() -> bool {
    true
}

impl ReactionDef {
    /// resolved against the supplied registry, otherwise from each
    /// input's inline `molar_mass_g_per_mol` override.
    #[must_use]
    pub fn input_mass_g_per_mol(&self, lookup: &dyn Fn(&str) -> Option<f32>) -> f32 {
        self.inputs.iter().map(|i| i.moles * resolve_mass(i.molar_mass_g_per_mol, lookup, &i.material)).sum()
    }

    /// Total output mass in grams-per-mole-of-reaction.
    #[must_use]
    pub fn output_mass_g_per_mol(&self, lookup: &dyn Fn(&str) -> Option<f32>) -> f32 {
        self.outputs.iter().map(|o| o.moles * resolve_mass(o.molar_mass_g_per_mol, lookup, &o.material)).sum()
    }

    /// Mass-balance delta in g/mol. Mods must satisfy |delta| ≤ 0.01.
    #[must_use]
    pub fn mass_balance_delta_g_per_mol(&self, lookup: &dyn Fn(&str) -> Option<f32>) -> f32 {
        self.input_mass_g_per_mol(lookup) - self.output_mass_g_per_mol(lookup)
    }

    /// Whether the reaction's mass balance is within tolerance.
    #[must_use]
    pub fn mass_balance_ok(&self, lookup: &dyn Fn(&str) -> Option<f32>, tolerance_g_per_mol: f32) -> bool {
        self.mass_balance_delta_g_per_mol(lookup).abs() <= tolerance_g_per_mol
    }

    /// Resolve a material name on the input side to a MaterialId
    /// against the supplied name→id lookup. Returns `None` when the
    /// material isn't in the registry (e.g. abstract reagents like
    /// KNO3) — the reaction is still loadable; resolution happens at
    /// dispatch time.
    #[must_use]
    pub fn resolved_input_ids(&self, name_to_id: &dyn Fn(&str) -> Option<MaterialId>) -> Vec<Option<MaterialId>> {
        self.inputs.iter().map(|i| name_to_id(&i.material)).collect()
    }

    #[must_use]
    pub fn resolved_output_ids(&self, name_to_id: &dyn Fn(&str) -> Option<MaterialId>) -> Vec<Option<MaterialId>> {
        self.outputs.iter().map(|o| name_to_id(&o.material)).collect()
    }

    /// Arrhenius effective rate per second at the given temperature.
    #[must_use]
    pub fn effective_rate_per_s(&self, temperature_k: f32) -> f32 {
        crate::arrhenius::arrhenius_rate(
            self.rate_constant_per_s,
            self.activation_energy_kj_per_mol,
            temperature_k,
        )
    }
}

fn resolve_mass(inline: Option<f32>, lookup: &dyn Fn(&str) -> Option<f32>, name: &str) -> f32 {
    if let Some(m) = inline {
        return m;
    }
    lookup(name).unwrap_or(0.0)
}

/// Locked registry. Loaded from `content/reactions/*.ron` at scenario
/// start and frozen for the duration of the run. Mods may extend; the
/// launch 55-reaction set is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct M15DReactionRegistry {
    pub reactions: Vec<ReactionDef>,
}

impl M15DReactionRegistry {
    pub const SCHEMA_VERSION: u32 = 1;

    /// Number of reactions in the launch matrix (spec literal).
    pub const LAUNCH_REACTION_COUNT: usize = 55;

    /// Mass-balance tolerance per spec § Gherkin: "each entry's mass
    /// balance validates within 0.01 g/mol".
    pub const MASS_BALANCE_TOLERANCE_G_PER_MOL: f32 = 0.01;

    pub fn new(reactions: Vec<ReactionDef>) -> Self {
        Self { reactions }
    }

    pub fn len(&self) -> usize {
        self.reactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reactions.is_empty()
    }

    pub fn by_id(&self, id: &str) -> Option<&ReactionDef> {
        self.reactions.iter().find(|r| r.id == id)
    }

    /// Iterate IDs in deterministic load order (sorted ascending by id).
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.reactions.iter().map(|r| r.id.clone()).collect();
        ids.sort();
        ids
    }

    /// Combustion reactions per spec § "rxn.combustion.*". Used by
    /// `cf-atmos::combustion` to re-export the M15D combustion table.
    pub fn combustion(&self) -> Vec<&ReactionDef> {
        self.reactions
            .iter()
            .filter(|r| r.id.starts_with("rxn.combustion."))
            .collect()
    }

    pub fn explosion(&self) -> Vec<&ReactionDef> {
        self.reactions
            .iter()
            .filter(|r| r.id.starts_with("rxn.explosion."))
            .collect()
    }

    pub fn corrosion(&self) -> Vec<&ReactionDef> {
        self.reactions
            .iter()
            .filter(|r| r.id.starts_with("rxn.corrosion."))
            .collect()
    }

    pub fn phase(&self) -> Vec<&ReactionDef> {
        self.reactions
            .iter()
            .filter(|r| r.id.starts_with("rxn.phase."))
            .collect()
    }

    /// Resolve every input / output / catalyst material name to a
    /// `MaterialId` against the supplied material registry. Mutates the
    /// registry in place so the spec-literal `catalyst_id` +
    /// `material_id` fields are populated for consumers (CA kernel, GPU
    /// table, M19 combustion gate).
    ///
    /// Names that don't resolve (abstract reagents like `electricity`,
    /// `altitude`, or chemistry-textbook formulas like `kno3`) leave
    /// the `*_id` fields as `None` — those reactions still load + still
    /// validate mass balance via inline `molar_mass_g_per_mol`, they
    /// just don't participate in MaterialId-indexed dispatch.
    pub fn resolve_against_material_registry(&mut self, reg: &crate::MaterialRegistry) {
        let name_to_id: std::collections::BTreeMap<String, MaterialId> = reg.name_to_id();
        for r in &mut self.reactions {
            for i in &mut r.inputs {
                i.material_id = name_to_id.get(&i.material).copied();
            }
            for o in &mut r.outputs {
                o.material_id = name_to_id.get(&o.material).copied();
            }
            if let Some(name) = r.catalyst.as_ref() {
                r.catalyst_id = name_to_id.get(name).copied();
            } else {
                r.catalyst_id = None;
            }
        }
    }

    /// Whether `id` is reactor-only per spec note: `rxn.radio.uranium_fission`
    /// is gated to the M29 reactor caller. The CA kernel + atmospherics
    /// must SKIP reactor-only reactions; only M29 invokes them.
    #[must_use]
    pub fn is_reactor_only(id: &str) -> bool {
        id == "rxn.radio.uranium_fission"
    }
}

/// Load every `*.ron` in `dir` and aggregate into a single
/// [`M15DReactionRegistry`]. Per-file failures emit a `tracing::warn!`
/// and continue — the spec explicitly forbids silent fallback. Returns
/// the aggregated registry + the load report (warnings, errors,
/// mass-balance violations).
pub fn load_registry_dir(dir: &Path) -> Result<(M15DReactionRegistry, M15DLoadReport), M15DLoadError> {
    let entries = std::fs::read_dir(dir).map_err(|source| M15DLoadError::ReadDirFailed {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ron") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut report = M15DLoadReport::default();
    let mut reactions: Vec<ReactionDef> = Vec::with_capacity(paths.len());
    let mut seen_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for path in &paths {
        match std::fs::read_to_string(path) {
            Ok(raw) => match ron::from_str::<ReactionDef>(&raw) {
                Ok(rxn) => {
                    if seen_ids.contains(&rxn.id) {
                        report.duplicate_ids.push(rxn.id.clone());
                        tracing::warn!(
                            target: "cf_material::reaction_registry",
                            id = %rxn.id,
                            path = %path.display(),
                            "duplicate reaction id"
                        );
                        continue;
                    }
                    seen_ids.insert(rxn.id.clone());
                    reactions.push(rxn);
                }
                Err(err) => {
                    report.parse_failures.push((path.clone(), err.to_string()));
                    tracing::warn!(
                        target: "cf_material::reaction_registry",
                        path = %path.display(),
                        error = %err,
                        "reaction .ron file failed to parse"
                    );
                }
            },
            Err(err) => {
                report.read_failures.push((path.clone(), err.to_string()));
                tracing::warn!(
                    target: "cf_material::reaction_registry",
                    path = %path.display(),
                    error = %err,
                    "reaction .ron file failed to read"
                );
            }
        }
    }
    Ok((M15DReactionRegistry::new(reactions), report))
}

/// Locate `content/reactions/` under the workspace. Mirrors
/// [`crate::MaterialRegistry::locate_default`] but for the M15D
/// reaction registry directory.
#[must_use]
pub fn locate_default_dir() -> Option<PathBuf> {
    let candidates: &[&str] = &[
        "content/reactions",
        "../content/reactions",
        "../../content/reactions",
        "../../../content/reactions",
        "game/content/reactions",
        "../game/content/reactions",
        "../../game/content/reactions",
    ];
    for rel in candidates {
        let p = PathBuf::from(rel);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(p) = walk_up_for_reactions(&cwd, 12) {
            return Some(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    walk_up_for_reactions(&manifest, 8)
}

fn walk_up_for_reactions(start: &Path, max_hops: usize) -> Option<PathBuf> {
    let mut current = Some(start.to_path_buf());
    for _ in 0..=max_hops {
        let here = current?;
        let direct = here.join("content/reactions");
        if direct.is_dir() {
            return Some(direct);
        }
        let nested = here.join("game/content/reactions");
        if nested.is_dir() {
            return Some(nested);
        }
        current = here.parent().map(|p| p.to_path_buf());
    }
    None
}

/// Convenience: locate then load. Returns `None` when the directory
/// isn't present (caller can fall back to the hardcoded matrix or
/// fail). The returned registry has every `material_id` /
/// `catalyst_id` field pre-resolved against the live material registry
/// when one is locatable; otherwise those fields stay `None`.
pub fn load_default_dir() -> Option<(M15DReactionRegistry, M15DLoadReport)> {
    let dir = locate_default_dir()?;
    match load_registry_dir(&dir) {
        Ok((mut reg, mut report)) => {
            if let Some(mat_path) = crate::MaterialRegistry::locate_default() {
                if let Ok((mat_reg, _)) = crate::load_registry_from_file(&mat_path) {
                    reg.resolve_against_material_registry(&mat_reg);
                    let lookup = registry_molar_mass_lookup(&mat_reg);
                    let lookup_owned: Box<dyn Fn(&str) -> Option<f32>> = Box::new(move |n: &str| lookup(n));
                    // Mass-balance validation runs at registry LOAD time
                    // per spec § "Notes for the implementer".
                    let _ = validate_mass_balance(
                        &reg,
                        &mut report,
                        &|n: &str| lookup_owned(n),
                        M15DReactionRegistry::MASS_BALANCE_TOLERANCE_G_PER_MOL,
                    );
                }
            }
            Some((reg, report))
        }
        Err(err) => {
            tracing::warn!(
                target: "cf_material::reaction_registry",
                error = ?err,
                "failed to load M15D reaction registry directory"
            );
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct M15DLoadReport {
    pub read_failures: Vec<(PathBuf, String)>,
    pub parse_failures: Vec<(PathBuf, String)>,
    pub duplicate_ids: Vec<String>,
    pub mass_balance_violations: Vec<MassBalanceViolation>,
}

impl M15DLoadReport {
    pub fn is_clean(&self) -> bool {
        self.read_failures.is_empty()
            && self.parse_failures.is_empty()
            && self.duplicate_ids.is_empty()
            && self.mass_balance_violations.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct MassBalanceViolation {
    pub reaction_id: String,
    pub input_mass_g_per_mol: f32,
    pub output_mass_g_per_mol: f32,
    pub delta_g_per_mol: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum M15DLoadError {
    #[error("failed to read M15D reaction directory at {}: {source}", path.display())]
    ReadDirFailed {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Validate the mass balance of every reaction in the registry against
/// the supplied molar-mass lookup. Mutates `report` to record every
/// violation. Returns the count of clean reactions.
pub fn validate_mass_balance(
    registry: &M15DReactionRegistry,
    report: &mut M15DLoadReport,
    lookup: &dyn Fn(&str) -> Option<f32>,
    tolerance_g_per_mol: f32,
) -> usize {
    let mut clean = 0usize;
    for r in &registry.reactions {
        let inp = r.input_mass_g_per_mol(lookup);
        let out = r.output_mass_g_per_mol(lookup);
        let delta = inp - out;
        if delta.abs() > tolerance_g_per_mol {
            report.mass_balance_violations.push(MassBalanceViolation {
                reaction_id: r.id.clone(),
                input_mass_g_per_mol: inp,
                output_mass_g_per_mol: out,
                delta_g_per_mol: delta,
            });
            tracing::warn!(
                target: "cf_material::reaction_registry",
                id = %r.id,
                input_mass = inp,
                output_mass = out,
                delta = delta,
                "reaction_mass_balance_violation"
            );
        } else {
            clean += 1;
        }
    }
    clean
}

/// `MaterialRegistry`-driven molar-mass lookup. Returns the registry
/// entry's `molar_mass_g_per_mol` when present and non-zero; otherwise
/// `None` so callers fall back to an inline override on the
/// `ReactionInput` / `ReactionOutput`.
#[must_use]
pub fn registry_molar_mass_lookup(reg: &crate::MaterialRegistry) -> impl Fn(&str) -> Option<f32> + '_ {
    move |name: &str| {
        let def = reg.find_by_name(name)?;
        if def.molar_mass_g_per_mol > 0.0 {
            Some(def.molar_mass_g_per_mol)
        } else {
            None
        }
    }
}

/// Compact GPU-row for a single reaction. Used by `cf-material-gpu` to
/// upload the M15D registry into the WGSL `ReactionEntry` struct
/// (`shaders/reaction_table.wgsl`).
///
/// Per spec § "cf-material-gpu::reaction_table — MODIFY — Compile 55
/// entries into wgsl reaction-table struct".
///
/// Each row carries the four canonical pair-lookup fields: input_a +
/// input_b material ids (resolved to the registry), the primary
/// output, the optional byproduct, and the autoignition temperature
/// gate. Reactions that reference reagents absent from the material
/// registry (e.g. KNO3, tritium) are skipped — those run via the
/// CPU-only M15D path until M15C adds the missing entries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuReactionRow {
    pub input_a: u16,
    pub input_b: u16,
    pub output: u16,
    pub byproduct: Option<u16>,
    pub min_temperature_k: Option<f32>,
}

/// Compile the M15D registry into a `Vec<GpuReactionRow>` for the GPU
/// reaction-table. Skips reactions whose first two inputs can't be
/// resolved against the supplied `name_to_id` lookup.
///
/// `rxn.radio.uranium_fission` is always excluded — per spec note
/// "reactor-only; do NOT enable autoignition. M29 is sole caller", the
/// CA kernel must never see it.
#[must_use]
pub fn compile_gpu_reaction_table(
    registry: &M15DReactionRegistry,
    name_to_id: &dyn Fn(&str) -> Option<MaterialId>,
) -> Vec<GpuReactionRow> {
    let mut rows = Vec::with_capacity(registry.reactions.len());
    for r in &registry.reactions {
        if M15DReactionRegistry::is_reactor_only(&r.id) {
            continue;
        }
        let pair_inputs: Vec<&ReactionInput> = r
            .inputs
            .iter()
            .filter(|i| i.molar_mass_g_per_mol != Some(0.0))
            .take(2)
            .collect();
        if pair_inputs.len() < 2 {
            continue;
        }
        let a = match name_to_id(&pair_inputs[0].material) {
            Some(id) => id,
            None => continue,
        };
        let b = match name_to_id(&pair_inputs[1].material) {
            Some(id) => id,
            None => continue,
        };
        let mut outs = r.outputs.iter().filter(|o| o.molar_mass_g_per_mol != Some(0.0));
        let primary = outs.next();
        let secondary = outs.next();
        let output = match primary.and_then(|o| name_to_id(&o.material)) {
            Some(id) => id,
            None => continue,
        };
        let byproduct = secondary.and_then(|o| name_to_id(&o.material));
        rows.push(GpuReactionRow {
            input_a: a,
            input_b: b,
            output,
            byproduct,
            min_temperature_k: r.min_temperature_k,
        });
    }
    rows
}

/// Project the M15D registry into the legacy [`crate::ReactionRegistry`]
/// shape (paired `MaterialReaction` entries the CA kernel reads).
///
/// Per spec § Crates / modules touched: `cf-material::reactions` |
/// MODIFY | "Prose list → parsed registry of 55 entries." The legacy
/// `default_reaction_registry()` becomes a derived view of this M15D
/// projection so the CA kernel + bench + tests all read from ONE
/// source of truth.
///
/// The projection picks the first 2 inputs (skipping massless
/// catalysts marked `molar_mass_g_per_mol: Some(0.0)`) as `input_a` /
/// `input_b`, the first non-catalyst output as `output`, and the
/// second as `byproduct`. Reactions whose pair inputs can't be
/// resolved against the material registry (KNO3, hydrazine, etc.) are
/// skipped — they still ship via the M15D registry for cfctl + UI
/// surfacing, but the CA kernel can't fire them until M15C adds the
/// matching materials.
#[must_use]
pub fn project_to_legacy_registry(
    registry: &M15DReactionRegistry,
    name_to_id: &dyn Fn(&str) -> Option<MaterialId>,
) -> crate::ReactionRegistry {
    let mut paired: Vec<crate::MaterialReaction> = Vec::with_capacity(registry.reactions.len());
    for r in &registry.reactions {
        if M15DReactionRegistry::is_reactor_only(&r.id) {
            continue;
        }
        if r.inputs.len() < 2 {
            continue;
        }
        // CA pair-match accepts BOTH massless catalysts (fire_intense,
        // spark) and real reagents (oil, iron) as pair members; the
        // mass-balance gate is enforced at load time, not at dispatch.
        let a_id = match r.inputs[0].material_id.or_else(|| name_to_id(&r.inputs[0].material)) {
            Some(id) => id,
            None => continue,
        };
        let b_id = match r.inputs[1].material_id.or_else(|| name_to_id(&r.inputs[1].material)) {
            Some(id) => id,
            None => continue,
        };
        // Pick the first registry-resolvable output as `output`, the
        // second as `byproduct`. Outputs that don't resolve (e.g.
        // FeCl2, K2S — not yet in the launch material registry) are
        // skipped so the reaction still projects with a usable
        // pair-match pair. The reaction still ships in the M15D
        // registry for cfctl + UI surfacing.
        let resolved_outputs: Vec<MaterialId> = r
            .outputs
            .iter()
            .filter_map(|o| o.material_id.or_else(|| name_to_id(&o.material)))
            .collect();
        let output_id = match resolved_outputs.first().copied() {
            Some(id) => id,
            None => continue,
        };
        let byproduct_id = resolved_outputs.get(1).copied();
        let energy_release_j = (-r.delta_h_kj_per_mol) * 1000.0;
        paired.push(crate::MaterialReaction {
            id: r.id.clone(),
            input_a: a_id,
            input_b: b_id,
            output: output_id,
            byproduct: byproduct_id,
            emissions: vec![],
            energy_release_j,
            rate_per_s: r.rate_constant_per_s,
            auto_ignite: r.auto_ignite,
            min_temperature_k: r.min_temperature_k,
            propagates: r.propagates,
            activation_k: arrhenius_to_legacy_activation_k(r.activation_energy_kj_per_mol, r.min_temperature_k),
            pressure_order: 0.0,
            violent: r.delta_h_kj_per_mol <= -500.0,
            flash_color_hex: None,
        });
    }
    crate::ReactionRegistry::new(paired)
}

/// Convert M15D Ea (kJ/mol) + min_temperature_k to the legacy
/// `MaterialReaction::activation_k` parameter (the temperature
/// acceleration scalar used by the existing `effective_rate_per_tick`
/// math: `rate *= exp(activation_k * (1/min_T - 1/T))`). The mapping
/// preserves the spec's Arrhenius `exp(-Ea/(R*T))` form at the
/// `min_temperature_k` reference — activation_k = Ea / R in K units.
fn arrhenius_to_legacy_activation_k(ea_kj_per_mol: f32, _min_t_k: Option<f32>) -> f32 {
    // Ea / R, with Ea in kJ/mol and R in kJ/(mol·K) → K
    if ea_kj_per_mol <= 0.0 {
        return 0.0;
    }
    ea_kj_per_mol / crate::arrhenius::GAS_CONSTANT_R_KJ_PER_MOL_K
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reaction() -> ReactionDef {
        ReactionDef {
            id: "test.rxn.h2_o2".to_string(),
            display_name: "Stoichiometric H2 + O2".to_string(),
            inputs: vec![
                ReactionInput {
                    material: "hydrogen".to_string(),
                    moles: 2.0,
                    molar_mass_g_per_mol: Some(2.016),
                    material_id: None,
                },
                ReactionInput {
                    material: "oxygen".to_string(),
                    moles: 1.0,
                    molar_mass_g_per_mol: Some(31.998),
                    material_id: None,
                },
            ],
            outputs: vec![ReactionOutput {
                material: "steam".to_string(),
                moles: 2.0,
                molar_mass_g_per_mol: Some(18.015),
                material_id: None,
            }],
            delta_h_kj_per_mol: -483.6,
            activation_energy_kj_per_mol: 50.0,
            rate_constant_per_s: 1.0,
            min_temperature_k: Some(700.0),
            min_pressure_kpa: None,
            catalyst: None,
            catalyst_id: None,
            variant: ReactionVariant::Both,
            emits_event: true,
            propagates: false,
            auto_ignite: false,
        }
    }

    #[test]
    fn mass_balance_zero_for_h2_o2() {
        let r = sample_reaction();
        let lookup: &dyn Fn(&str) -> Option<f32> = &|_| None;
        let delta = r.mass_balance_delta_g_per_mol(lookup);
        assert!(delta.abs() < 1e-2, "H2+O2 balance must be ~0, got {delta}");
        assert!(r.mass_balance_ok(lookup, 0.01));
    }

    #[test]
    fn mass_balance_uses_registry_lookup_when_inline_absent() {
        let mut r = sample_reaction();
        for i in &mut r.inputs {
            i.molar_mass_g_per_mol = None;
        }
        for o in &mut r.outputs {
            o.molar_mass_g_per_mol = None;
        }
        let lookup_fn = |name: &str| match name {
            "hydrogen" => Some(2.016),
            "oxygen" => Some(31.998),
            "steam" => Some(18.015),
            _ => None,
        };
        let lookup: &dyn Fn(&str) -> Option<f32> = &lookup_fn;
        let delta = r.mass_balance_delta_g_per_mol(lookup);
        assert!(delta.abs() < 1e-2, "registry-resolved balance must be ~0, got {delta}");
    }

    #[test]
    fn mass_balance_violation_flagged() {
        let mut r = sample_reaction();
        r.outputs[0].moles = 3.0;
        let lookup: &dyn Fn(&str) -> Option<f32> = &|_| None;
        assert!(!r.mass_balance_ok(lookup, 0.01));
    }

    #[test]
    fn effective_rate_finite_at_high_temperature() {
        let r = sample_reaction();
        let rate = r.effective_rate_per_s(1500.0);
        assert!(rate.is_finite());
        assert!(rate > 0.0);
    }

    #[test]
    fn variant_default_is_both() {
        let v: ReactionVariant = Default::default();
        assert_eq!(v, ReactionVariant::Both);
    }

    #[test]
    fn registry_filter_categories() {
        let reg = M15DReactionRegistry::new(vec![
            ReactionDef {
                id: "rxn.combustion.h2_o2".into(),
                display_name: "H2+O2".into(),
                inputs: vec![],
                outputs: vec![],
                delta_h_kj_per_mol: 0.0,
                activation_energy_kj_per_mol: 0.0,
                rate_constant_per_s: 0.0,
                min_temperature_k: None,
                min_pressure_kpa: None,
                catalyst: None,
                catalyst_id: None,
                variant: ReactionVariant::Both,
                emits_event: true,
                propagates: false,
                auto_ignite: false,
            },
            ReactionDef {
                id: "rxn.explosion.gunpowder_spark".into(),
                display_name: "Gunpowder".into(),
                inputs: vec![],
                outputs: vec![],
                delta_h_kj_per_mol: 0.0,
                activation_energy_kj_per_mol: 0.0,
                rate_constant_per_s: 0.0,
                min_temperature_k: None,
                min_pressure_kpa: None,
                catalyst: None,
                catalyst_id: None,
                variant: ReactionVariant::PerPixel,
                emits_event: true,
                propagates: false,
                auto_ignite: true,
            },
        ]);
        assert_eq!(reg.combustion().len(), 1);
        assert_eq!(reg.explosion().len(), 1);
        assert_eq!(reg.len(), 2);
        assert!(reg.by_id("rxn.combustion.h2_o2").is_some());
    }

    #[test]
    fn ron_round_trip_preserves_schema() {
        let r = sample_reaction();
        let ser = ron::ser::to_string_pretty(&r, ron::ser::PrettyConfig::default()).expect("ser");
        let de: ReactionDef = ron::from_str(&ser).expect("de");
        assert_eq!(de, r);
    }
}
