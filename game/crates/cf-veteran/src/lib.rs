//! **M14I** § Veteran persistence substrate.
//!
//! Promotes M41's veteran roster stub into the functional long-term-
//! consequence layer:
//! - `VeteranDossier` — per-veteran aggregate (scars + age + prosthetics
//!   + traits + retirement-state).
//! - `VeteranRoster` — engine-side registry keyed by actor id.
//!
//! M41 consumes this data for the roster UI; M48C consumes it for the
//! pilot dossier. M14I OWNS the data.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::manual_range_contains,
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::uninlined_format_args,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::option_if_let_else,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::bool_to_int_with_if,
    clippy::missing_const_for_fn
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_aging::BiologicalAge;
use cf_prosthetic::ProstheticInstance;
use cf_scar::ScarTimeline;

/// **M14I** § per-veteran dossier — aggregates every long-term-consequence
/// signal an actor accumulates over their career.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VeteranDossier {
    /// Stable display name (mirrors `ActorState.team` / `ActorState.id`).
    #[serde(default)]
    pub display_name: String,
    pub scar_timeline: ScarTimeline,
    pub biological_age: Option<BiologicalAge>,
    pub prosthetics: Vec<ProstheticInstance>,
    /// Set when the actor crossed retirement age and elected to retire.
    pub retired: bool,
    /// Tick the retire action fired. 0 when not yet retired.
    pub retired_tick: u64,
    /// Origin label cached for cross-crate lookups.
    #[serde(default)]
    pub origin_label: String,
}

impl VeteranDossier {
    pub fn new(display_name: impl Into<String>, origin_label: impl Into<String>) -> Self {
        Self {
            display_name: display_name.into(),
            scar_timeline: ScarTimeline::new(),
            biological_age: None,
            prosthetics: Vec::new(),
            retired: false,
            retired_tick: 0,
            origin_label: origin_label.into(),
        }
    }

    /// Mark this veteran as retired (advisor NPC).
    pub fn retire(&mut self, current_tick: u64) {
        self.retired = true;
        self.retired_tick = current_tick;
    }

    /// Per-actor stable checksum bytes for save/load round-trip determinism.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.display_name.as_bytes());
        out.push(0);
        out.extend_from_slice(self.origin_label.as_bytes());
        out.push(0);
        out.extend_from_slice(&self.scar_timeline.checksum_bytes());
        out.push(if self.retired { 1 } else { 0 });
        out.extend_from_slice(&self.retired_tick.to_le_bytes());
        if let Some(age) = &self.biological_age {
            out.push(1);
            out.extend_from_slice(&age.age_in_game_years.to_le_bytes());
            out.push(age.origin as u8);
            out.push(if age.retirement_offered { 1 } else { 0 });
            out.push(if age.terminal_age_reached { 1 } else { 0 });
            out.push(if age.died_of_old_age { 1 } else { 0 });
        } else {
            out.push(0);
        }
        out.extend_from_slice(&(self.prosthetics.len() as u64).to_le_bytes());
        for p in &self.prosthetics {
            out.push(p.kind as u8);
            out.push(p.tier as u8);
            out.extend_from_slice(p.zone.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&p.wear_pct.to_le_bytes());
            out.push(if p.malfunctioning { 1 } else { 0 });
        }
        out
    }
}

/// **M14I** § engine-side veteran roster (one entry per actor with at
/// least one persistent long-term state element).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VeteranRoster {
    pub by_actor: BTreeMap<u64, VeteranDossier>,
}

impl VeteranRoster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry_mut(&mut self, actor_id: u64) -> &mut VeteranDossier {
        self.by_actor.entry(actor_id).or_default()
    }

    pub fn get(&self, actor_id: u64) -> Option<&VeteranDossier> {
        self.by_actor.get(&actor_id)
    }

    pub fn get_mut(&mut self, actor_id: u64) -> Option<&mut VeteranDossier> {
        self.by_actor.get_mut(&actor_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &VeteranDossier)> {
        self.by_actor.iter()
    }

    pub fn len(&self) -> usize {
        self.by_actor.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_actor.is_empty()
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.by_actor.len() as u64).to_le_bytes());
        for (id, d) in &self.by_actor {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&d.checksum_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dossier_round_trip() {
        let mut d = VeteranDossier::new("Hawthorne", "human");
        d.biological_age = Some(BiologicalAge::new_human(31.0));
        assert_eq!(d.display_name, "Hawthorne");
        assert!(!d.retired);
        d.retire(99);
        assert!(d.retired);
        assert_eq!(d.retired_tick, 99);
    }

    #[test]
    fn roster_checksum_stable() {
        let mut a = VeteranRoster::new();
        let mut b = VeteranRoster::new();
        a.entry_mut(42).display_name = "Alpha".into();
        b.entry_mut(42).display_name = "Alpha".into();
        assert_eq!(a.checksum_bytes(), b.checksum_bytes());
    }
}
