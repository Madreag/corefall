//! M7B: doctrine compatibility matrix.
//!
//! Some squad verbs are incompatible with the current doctrine. Press Attack
//! under Defensive, for example, is a contradiction; the engine vetoes the
//! command and surfaces a `squad.command_vetoed { reason_label }`. The Tab
//! overlay flashes the reason. Re-issuing requires the player promote the
//! doctrine first.
//!
//! Spec § "Veto reason labels use `doctrine_<X>_blocks_<Y>` convention so
//! M23B can adapt."

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::autonomy::DoctrineMode;

/// **M7B**: veto reason emitted by the matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VetoReason {
    /// Convention `doctrine_<X>_blocks_<Y>`.
    DoctrineBlocks {
        doctrine: DoctrineMode,
        verb_id: String,
    },
}

impl VetoReason {
    /// Render the reason as the canonical `doctrine_<X>_blocks_<Y>` label.
    pub fn into_label(self, doctrine: DoctrineMode, verb_id: &str) -> String {
        let _ = self; // reason is recoverable from the doctrine + verb
        format!("doctrine_{}_blocks_{}", doctrine.as_str(), verb_id)
    }
}

/// **M7B**: per-doctrine veto set. A verb listed under a doctrine is vetoed
/// when that doctrine is active.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DoctrineCompatMatrix {
    pub vetoes: BTreeMap<String, Vec<String>>,
}

impl DoctrineCompatMatrix {
    pub fn new() -> Self {
        Self {
            vetoes: BTreeMap::new(),
        }
    }

    /// Returns `Some(VetoReason)` when the verb is blocked under doctrine.
    pub fn veto_reason(&self, doctrine: DoctrineMode, verb_id: &str) -> Option<VetoReason> {
        let doctrine_key = doctrine.as_str().to_string();
        let vetoed = self.vetoes.get(&doctrine_key)?;
        if vetoed.iter().any(|v| v == verb_id) {
            Some(VetoReason::DoctrineBlocks {
                doctrine,
                verb_id: verb_id.to_string(),
            })
        } else {
            None
        }
    }

    /// Spec-mandated default matrix:
    /// - Defensive blocks `press_attack`, `advance`, `storm_building`,
    ///   `heavy_forward`.
    /// - Aggressive blocks `hold_fire`, `withdraw`, `retreat_in_order`,
    ///   `fall_back`.
    /// - Scout blocks `frag_out`, `suppress_target`, `suppress_window`,
    ///   `press_attack`.
    pub fn builtin() -> Self {
        let mut m = Self::new();
        m.vetoes.insert(
            DoctrineMode::Defensive.as_str().to_string(),
            vec![
                "press_attack".to_string(),
                "advance".to_string(),
                "storm_building".to_string(),
                "heavy_forward".to_string(),
            ],
        );
        m.vetoes.insert(
            DoctrineMode::Aggressive.as_str().to_string(),
            vec![
                "hold_fire".to_string(),
                "withdraw".to_string(),
                "retreat_in_order".to_string(),
                "fall_back".to_string(),
            ],
        );
        m.vetoes.insert(
            DoctrineMode::Scout.as_str().to_string(),
            vec![
                "frag_out".to_string(),
                "suppress_target".to_string(),
                "suppress_window".to_string(),
                "press_attack".to_string(),
            ],
        );
        m
    }

    pub fn from_ron(src: &str) -> Result<Self, String> {
        ron::from_str(src).map_err(|e| format!("ron parse failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defensive_blocks_press_attack() {
        let m = DoctrineCompatMatrix::builtin();
        let reason = m.veto_reason(DoctrineMode::Defensive, "press_attack").expect("veto");
        let label = reason.into_label(DoctrineMode::Defensive, "press_attack");
        assert_eq!(label, "doctrine_defensive_blocks_press_attack");
    }

    #[test]
    fn aggressive_blocks_withdraw() {
        let m = DoctrineCompatMatrix::builtin();
        assert!(m.veto_reason(DoctrineMode::Aggressive, "withdraw").is_some());
    }

    #[test]
    fn scout_blocks_suppress_target() {
        let m = DoctrineCompatMatrix::builtin();
        assert!(m.veto_reason(DoctrineMode::Scout, "suppress_target").is_some());
    }

    #[test]
    fn defensive_allows_move_to() {
        let m = DoctrineCompatMatrix::builtin();
        assert!(m.veto_reason(DoctrineMode::Defensive, "move_to").is_none());
    }
}
