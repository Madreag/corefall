//! M15D § cf-atmos::combustion — re-export of the M15D combustion
//! reactions.
//!
//! M19's earlier 6 hardcoded combustion entries (H2/CH4 + O2 / N2O / O3
//! variants) are now sourced from the M15D `content/reactions/*.ron`
//! registry. This module is the canonical pickup point for atmospherics
//! consumers (PV=nRT pressurization, autoignition checks, M19B planet-
//! atmosphere combustion tables).
//!
//! Per M15D spec § Crates / modules touched:
//! > `cf-atmos::combustion` — MODIFY — M19's 6 hardcoded entries →
//! > re-export of `rxn.combustion.*`.

use cf_material::{load_default_dir, M15DReactionRegistry, ReactionDef};

/// Canonical autoignition temperature table (in Kelvin) for the M15D
/// combustion reactions, indexed by reaction id. Returns the `Option`
/// directly from `min_temperature_k` — callers must handle the `None`
/// case (no autoignition gate).
#[must_use]
pub fn autoignition_temperature_k(reaction_id: &str) -> Option<f32> {
    let reg = lazy_load()?;
    reg.by_id(reaction_id)?.min_temperature_k
}

/// All `rxn.combustion.*` reactions from the M15D registry. Returns an
/// empty Vec when the registry can't be loaded (callers may fall back
/// to the M19-vintage hardcoded list).
#[must_use]
pub fn combustion_reactions() -> Vec<ReactionDef> {
    match lazy_load() {
        Some(reg) => reg.combustion().into_iter().cloned().collect(),
        None => Vec::new(),
    }
}

/// Per-id combustion lookup. Returns `None` when the registry isn't
/// loadable or the id isn't a `rxn.combustion.*`.
#[must_use]
pub fn combustion_by_id(id: &str) -> Option<ReactionDef> {
    if !id.starts_with("rxn.combustion.") {
        return None;
    }
    let reg = lazy_load()?;
    reg.by_id(id).cloned()
}

/// Locked count of `rxn.combustion.*` reactions in the M15D matrix.
/// Spec § table lists 15 combustion entries. The hardcoded constant
/// here keeps M19 consumers' assertion paths cheap.
pub const M15D_COMBUSTION_COUNT: usize = 15;

fn lazy_load() -> Option<M15DReactionRegistry> {
    use std::sync::OnceLock;
    static REG: OnceLock<Option<M15DReactionRegistry>> = OnceLock::new();
    REG.get_or_init(|| load_default_dir().map(|(r, _)| r)).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combustion_h2_o2_autoignition_threshold_matches_spec() {
        if let Some(t) = autoignition_temperature_k("rxn.combustion.h2_o2") {
            assert!((t - 700.0).abs() < 1e-3);
        }
    }

    #[test]
    fn combustion_table_includes_all_15_entries_when_loaded() {
        let combustion = combustion_reactions();
        if !combustion.is_empty() {
            assert_eq!(
                combustion.len(),
                M15D_COMBUSTION_COUNT,
                "M19 must see exactly 15 M15D combustion reactions"
            );
        }
    }

    #[test]
    fn combustion_by_id_rejects_non_combustion_lookups() {
        assert!(combustion_by_id("rxn.corrosion.acid_iron").is_none());
    }
}
