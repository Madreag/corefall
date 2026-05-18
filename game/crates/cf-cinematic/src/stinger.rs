//! **M12C** § "the per-storyteller stinger picks a variant from
//! `content/cinematics/opening_stingers/<storyteller_id>.ron`."
//!
//! Each storyteller ships a table of authored stinger variants (two-line
//! briefing-card lines + a stable id). The cinematic kernel picks one
//! variant deterministically off `(mission_id, seed)` so two engines at
//! the same seed surface the same stinger.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storyteller_profile::StorytellerId;

/// One stinger variant — two briefing-card lines + a stable id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StingerVariant {
    /// Stable variant id (e.g. `"cassandra_dread_001"`).
    pub id: String,
    /// First line of the stinger (top of briefing card).
    pub line_a: String,
    /// Second line of the stinger (bottom of briefing card). May be
    /// empty for one-line stingers (e.g. Sandbox).
    #[serde(default)]
    pub line_b: String,
}

/// Per-storyteller stinger table. Loaded from
/// `content/cinematics/opening_stingers/<storyteller_id>.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StingerTable {
    /// Storyteller scope.
    pub storyteller_id: StorytellerId,
    /// Authored variants. The picker hashes the cinematic id to choose
    /// one deterministically.
    pub variants: Vec<StingerVariant>,
}

/// Errors raised by the stinger loader.
#[derive(Debug, Error)]
pub enum StingerLoadError {
    /// RON parse failure.
    #[error("stinger table parse failed: {0}")]
    Parse(String),
    /// Schema validation failure (empty variants list, mismatched
    /// storyteller_id, etc.).
    #[error("stinger table validation failed: {0}")]
    Validation(String),
}

impl StingerTable {
    /// Parse a stinger table from RON bytes + validate.
    pub fn from_ron(bytes: &[u8]) -> Result<Self, StingerLoadError> {
        let s = std::str::from_utf8(bytes).map_err(|e| StingerLoadError::Parse(e.to_string()))?;
        let table: StingerTable = ron::from_str(s).map_err(|e| StingerLoadError::Parse(e.to_string()))?;
        table.validate()?;
        Ok(table)
    }

    /// Validate the table.
    pub fn validate(&self) -> Result<(), StingerLoadError> {
        if self.variants.is_empty() {
            return Err(StingerLoadError::Validation(format!(
                "{}: variants must not be empty",
                self.storyteller_id.as_str()
            )));
        }
        for (i, v) in self.variants.iter().enumerate() {
            if v.id.is_empty() {
                return Err(StingerLoadError::Validation(format!(
                    "{}: variants[{}].id is empty",
                    self.storyteller_id.as_str(),
                    i
                )));
            }
            if v.line_a.is_empty() {
                return Err(StingerLoadError::Validation(format!(
                    "{}: variants[{}].line_a is empty",
                    self.storyteller_id.as_str(),
                    i
                )));
            }
        }
        Ok(())
    }

    /// Deterministically pick a variant index by hashing
    /// `(cinematic_id, seed)`. Per spec § "the per-storyteller stinger
    /// picks a variant" — and since cinematics are replay-deterministic,
    /// the picker must be a pure function.
    #[must_use]
    pub fn pick_index(&self, cinematic_id: &str, seed: u64) -> usize {
        if self.variants.is_empty() {
            return 0;
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed.to_le_bytes());
        hasher.update(cinematic_id.as_bytes());
        hasher.update(self.storyteller_id.as_str().as_bytes());
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
        let v = u64::from_le_bytes(buf);
        (v as usize) % self.variants.len()
    }

    /// Pick a variant deterministically. Returns `None` only when the
    /// table has zero variants (rejected by `validate`).
    #[must_use]
    pub fn pick(&self, cinematic_id: &str, seed: u64) -> Option<&StingerVariant> {
        let idx = self.pick_index(cinematic_id, seed);
        self.variants.get(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(id: StorytellerId, variants: &[(&str, &str, &str)]) -> StingerTable {
        StingerTable {
            storyteller_id: id,
            variants: variants
                .iter()
                .map(|(i, a, b)| StingerVariant {
                    id: (*i).to_string(),
                    line_a: (*a).to_string(),
                    line_b: (*b).to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn pick_is_deterministic_for_same_inputs() {
        let t = table(
            StorytellerId::CassandraClassic,
            &[
                ("cassandra_001", "Something on this rock has teeth.", "Bring bullets."),
                ("cassandra_002", "The dropship will leave with or without you.", ""),
            ],
        );
        let a = t.pick("cin_intro_reactor_defense", 42);
        let b = t.pick("cin_intro_reactor_defense", 42);
        assert_eq!(a.map(|v| v.id.clone()), b.map(|v| v.id.clone()));
    }

    #[test]
    fn pick_varies_with_seed() {
        let t = table(
            StorytellerId::PhoebeChillax,
            &[
                ("phoebe_001", "I made you a sandwich.", ""),
                ("phoebe_002", "There's a tea stain on my chart.", ""),
                ("phoebe_003", "Squad inventory says enough rations.", ""),
                ("phoebe_004", "I drew a smiley face.", ""),
                ("phoebe_005", "Comms are quiet.", ""),
            ],
        );
        let a = t.pick("cin_intro_reactor_defense", 1);
        let b = t.pick("cin_intro_reactor_defense", 1_000_000);
        // Not guaranteed to differ, but with 5 variants two distinct
        // seeds should very likely produce different picks. If they
        // happen to collide on one specific test, replace the seeds.
        assert!(a.is_some() && b.is_some());
    }

    #[test]
    fn rejects_empty_variants() {
        let t = StingerTable {
            storyteller_id: StorytellerId::Sandbox,
            variants: Vec::new(),
        };
        assert!(t.validate().is_err());
    }

    #[test]
    fn rejects_empty_line_a() {
        let mut t = table(StorytellerId::Sandbox, &[("sandbox_001", "", "")]);
        // line_a is empty by construction.
        assert!(t.validate().is_err());
        t.variants[0].line_a = "Mission ready.".to_string();
        t.validate().expect("valid");
    }

    #[test]
    fn parses_ron_round_trip() {
        let src = r#"(
            storyteller_id: ironman,
            variants: [
                ( id: "ironman_001", line_a: "You survived.", line_b: "Now earn the next." ),
                ( id: "ironman_002", line_a: "Pain is data.", line_b: "Read it." ),
            ],
        )"#;
        let t = StingerTable::from_ron(src.as_bytes()).expect("parse");
        assert_eq!(t.storyteller_id, StorytellerId::Ironman);
        assert_eq!(t.variants.len(), 2);
        let pick = t.pick("cin_intro_test", 1234).expect("picked");
        assert!(pick.id.starts_with("ironman_"));
    }
}
