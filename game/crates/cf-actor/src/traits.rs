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
    pub const RETIRED_VETERAN: &str = "retired_veteran";

    // ---- M16C psych-condition trait prefixes ----
    // These mirror `cf_mental_health::ConditionKind::{recovered,chronic,
    // refractory}_trait()` exactly: the lifecycle grants `recovered_from_*` on
    // remission, `chronic_*` on chronic entry, `refractory_*` on refractory
    // entry. M14I scar-record + M41 veteran dossier gate on the prefixes.
    pub const RECOVERED_FROM_PREFIX: &str = "recovered_from_";
    pub const CHRONIC_PREFIX: &str = "chronic_";
    pub const REFRACTORY_PREFIX: &str = "refractory_";

    // Chronic (one per condition). `chronic_pain` is the M16C Pain affliction
    // analogue carried as a long-term trait.
    pub const CHRONIC_PAIN: &str = "chronic_pain";
    pub const CHRONIC_PTSD: &str = "chronic_ptsd";
    pub const CHRONIC_ANXIETY_DISORDER: &str = "chronic_anxiety_disorder";
    pub const CHRONIC_DEPRESSION: &str = "chronic_depression";
    pub const CHRONIC_ADDICTION: &str = "chronic_addiction";
    pub const CHRONIC_WITHDRAWAL: &str = "chronic_withdrawal";
    pub const CHRONIC_INSOMNIA: &str = "chronic_insomnia";
    pub const CHRONIC_PANIC_DISORDER: &str = "chronic_panic_disorder";
    pub const CHRONIC_ACUTE_STRESS_REACTION: &str = "chronic_acute_stress_reaction";

    // Recovered (one per condition).
    pub const RECOVERED_FROM_PTSD: &str = "recovered_from_ptsd";
    pub const RECOVERED_FROM_ANXIETY_DISORDER: &str = "recovered_from_anxiety_disorder";
    pub const RECOVERED_FROM_DEPRESSION: &str = "recovered_from_depression";
    pub const RECOVERED_FROM_ADDICTION: &str = "recovered_from_addiction";
    pub const RECOVERED_FROM_WITHDRAWAL: &str = "recovered_from_withdrawal";
    pub const RECOVERED_FROM_INSOMNIA: &str = "recovered_from_insomnia";
    pub const RECOVERED_FROM_PANIC_DISORDER: &str = "recovered_from_panic_disorder";
    pub const RECOVERED_FROM_ACUTE_STRESS_REACTION: &str = "recovered_from_acute_stress_reaction";

    // Refractory (one per condition).
    pub const REFRACTORY_PTSD: &str = "refractory_ptsd";
    pub const REFRACTORY_ANXIETY_DISORDER: &str = "refractory_anxiety_disorder";
    pub const REFRACTORY_DEPRESSION: &str = "refractory_depression";
    pub const REFRACTORY_ADDICTION: &str = "refractory_addiction";
    pub const REFRACTORY_WITHDRAWAL: &str = "refractory_withdrawal";
    pub const REFRACTORY_INSOMNIA: &str = "refractory_insomnia";
    pub const REFRACTORY_PANIC_DISORDER: &str = "refractory_panic_disorder";
    pub const REFRACTORY_ACUTE_STRESS_REACTION: &str = "refractory_acute_stress_reaction";

    /// Legacy alias for [`CHRONIC_ANXIETY_DISORDER`] (pre-M16C short form).
    pub const CHRONIC_ANXIETY: &str = "chronic_anxiety";
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

    /// Any trait whose id starts with `"recovered_from_"` (M16C remission).
    pub fn has_recovered(&self) -> bool {
        self.traits.iter().any(|t| t.starts_with(ids::RECOVERED_FROM_PREFIX))
    }

    /// Any trait whose id starts with `"refractory_"` (M16C treatment-resistant).
    pub fn has_refractory(&self) -> bool {
        self.traits.iter().any(|t| t.starts_with(ids::REFRACTORY_PREFIX))
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
    fn m16c_psych_trait_ids_follow_prefix_convention() {
        // Each per-condition id must equal prefix + condition (so they match
        // `cf_mental_health::ConditionKind::{recovered,chronic,refractory}_trait`).
        assert_eq!(ids::CHRONIC_PTSD, format!("{}ptsd", ids::CHRONIC_PREFIX));
        assert_eq!(ids::CHRONIC_ADDICTION, format!("{}addiction", ids::CHRONIC_PREFIX));
        assert_eq!(
            ids::RECOVERED_FROM_PTSD,
            format!("{}ptsd", ids::RECOVERED_FROM_PREFIX)
        );
        assert_eq!(
            ids::REFRACTORY_PANIC_DISORDER,
            format!("{}panic_disorder", ids::REFRACTORY_PREFIX)
        );
        assert_eq!(
            ids::RECOVERED_FROM_ACUTE_STRESS_REACTION,
            format!("{}acute_stress_reaction", ids::RECOVERED_FROM_PREFIX)
        );
    }

    #[test]
    fn recovered_and_refractory_prefix_helpers() {
        let mut t = TraitSet::new();
        t.insert(ids::RECOVERED_FROM_PTSD);
        assert!(t.has_recovered());
        assert!(!t.has_refractory());
        t.insert(ids::REFRACTORY_DEPRESSION);
        assert!(t.has_refractory());
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
