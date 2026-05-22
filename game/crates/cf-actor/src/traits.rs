//! **M14I** § per-actor trait registry — extended.
//!
//! Traits are stable string ids attached to an actor that bias the AI +
//! UI pass without redefining stat fields. M14I adds long-term-
//! consequence traits to the registry the AI and UI already consume:
//!
//! - `phantom_limb` — fires per-week panic-attack rolls when severed.
//! - `memory_loss_minor` / `memory_loss_major` — accumulated concussion
//!   penalty.
//! - `chronic_<condition>` — chronic conditions from M16C lifecycle
//!   (chronic_pain, chronic_depression, chronic_insomnia, etc.).
//! - `retired_veteran` — actor opted into retirement (advisor NPC).
//!
//! The trait registry itself is intentionally open: new chronic-
//! condition labels can be added without bumping a schema. Callers
//! gate their behavior on a fixed substring (`"chronic_"`).

use serde::{Deserialize, Serialize};

/// Locked trait id strings (so call sites don't typo).
pub mod ids {
    pub const PHANTOM_LIMB: &str = "phantom_limb";
    pub const MEMORY_LOSS_MINOR: &str = "memory_loss_minor";
    pub const MEMORY_LOSS_MAJOR: &str = "memory_loss_major";
    pub const CHRONIC_PREFIX: &str = "chronic_";
    pub const CHRONIC_DEPRESSION: &str = "chronic_depression";
    pub const CHRONIC_PAIN: &str = "chronic_pain";
    pub const CHRONIC_INSOMNIA: &str = "chronic_insomnia";
    pub const CHRONIC_ANXIETY: &str = "chronic_anxiety";
    pub const RETIRED_VETERAN: &str = "retired_veteran";
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TraitSet {
    /// Sorted list of trait ids — sorted so the checksum is deterministic.
    pub traits: Vec<String>,
}

impl TraitSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the trait was newly inserted.
    pub fn insert(&mut self, trait_id: impl Into<String>) -> bool {
        let id = trait_id.into();
        if self.traits.iter().any(|t| t == &id) {
            return false;
        }
        self.traits.push(id);
        self.traits.sort();
        true
    }

    /// Returns true if the trait was present and removed.
    pub fn remove(&mut self, trait_id: &str) -> bool {
        if let Some(idx) = self.traits.iter().position(|t| t == trait_id) {
            self.traits.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn has(&self, trait_id: &str) -> bool {
        self.traits.iter().any(|t| t == trait_id)
    }

    /// Any trait whose id starts with `"chronic_"`.
    pub fn has_chronic(&self) -> bool {
        self.traits.iter().any(|t| t.starts_with(ids::CHRONIC_PREFIX))
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.traits.iter()
    }

    pub fn len(&self) -> usize {
        self.traits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.traits.is_empty()
    }

    /// Append-only checksum bytes for save / load round-trip.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.traits.len() as u64).to_le_bytes());
        for t in &self.traits {
            out.extend_from_slice(t.as_bytes());
            out.push(0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_idempotent() {
        let mut t = TraitSet::new();
        assert!(t.insert(ids::PHANTOM_LIMB));
        assert!(!t.insert(ids::PHANTOM_LIMB));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn chronic_prefix_detected() {
        let mut t = TraitSet::new();
        t.insert(ids::CHRONIC_DEPRESSION);
        assert!(t.has_chronic());
    }

    #[test]
    fn ordering_deterministic() {
        let mut a = TraitSet::new();
        let mut b = TraitSet::new();
        a.insert("zeta");
        a.insert("alpha");
        b.insert("alpha");
        b.insert("zeta");
        assert_eq!(a.traits, b.traits);
        assert_eq!(a.checksum_bytes(), b.checksum_bytes());
    }
}
