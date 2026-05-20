//! **M14I** — Scar timeline + functional-debuff layer.
//!
//! Canonical owner of:
//! - [`ScarRecord`] — per-actor scar entry produced when a wound closes
//!   via sutures / cauterize / surgery.
//! - [`FunctionalDebuff`] — locked enum of per-scar passive penalties.
//! - [`ScarTimeline`] — per-actor ordered list of scars plus the cached
//!   passive-debuff aggregate.
//! - [`functional_debuff_for`] — pure mapper from
//!   `(WoundKind, TreatmentKind, severity_at_close)` → `FunctionalDebuff`.
//!
//! Determinism: this crate is pure (no RNG, no clocks). The owning engine
//! drives scar acquisition by feeding closed wounds in event order.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
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
    clippy::map_unwrap_or,
    clippy::too_many_lines,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::missing_const_for_fn,
    clippy::bool_to_int_with_if,
    clippy::unnested_or_patterns
)]

pub mod functional_debuff;

pub use functional_debuff::{functional_debuff_for, FunctionalDebuff, SenseKind};

use serde::{Deserialize, Serialize};

use cf_wound::registry::{TreatmentKind, VisualDecalId, ZoneId};
use cf_wound::WoundKind;

/// Stable per-actor scar identifier. Monotonic within an [`ScarTimeline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScarId(pub u64);

impl ScarId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable per-narrative-event identifier. M48C pilot dossier consumer
/// renders these. Stored opaque on the M14I side.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NarrativeEventId(pub String);

impl NarrativeEventId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// **M14I** § canonical scar record.
///
/// Produced whenever a wound's visible state closes via sutures /
/// cauterize / surgery. Honors the spec's named contract fields exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScarRecord {
    pub scar_id: ScarId,
    pub source_wound_kind: WoundKind,
    pub zone: ZoneId,
    /// Severity at the moment the wound closed, in `[0, 1]`.
    pub severity_at_close: f32,
    pub closure_method: TreatmentKind,
    /// Engine tick at which the scar entered the timeline.
    pub tick_acquired: u64,
    pub functional_debuff: FunctionalDebuff,
    pub cosmetic_decal_id: VisualDecalId,
    /// Optional narrative anchor — links to the storyteller event that
    /// emitted at the moment the wound closed. None for unscripted
    /// closures.
    #[serde(default)]
    pub narrative_context: Option<NarrativeEventId>,
}

impl ScarRecord {
    /// Construct a fresh scar record from a closure event.
    pub fn new(
        scar_id: ScarId,
        source_wound_kind: WoundKind,
        zone: ZoneId,
        severity_at_close: f32,
        closure_method: TreatmentKind,
        tick_acquired: u64,
        cosmetic_decal_id: VisualDecalId,
    ) -> Self {
        let functional_debuff =
            functional_debuff_for(source_wound_kind, closure_method, severity_at_close, &zone);
        Self {
            scar_id,
            source_wound_kind,
            zone,
            severity_at_close: severity_at_close.clamp(0.0, 1.0),
            closure_method,
            tick_acquired,
            functional_debuff,
            cosmetic_decal_id,
            narrative_context: None,
        }
    }
}

/// **M14I** § per-actor scar timeline (ordered, append-only).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScarTimeline {
    pub scars: Vec<ScarRecord>,
    pub next_scar_id: u64,
}

impl ScarTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the next scar id (monotonic).
    pub fn alloc_id(&mut self) -> ScarId {
        let id = ScarId(self.next_scar_id);
        self.next_scar_id += 1;
        id
    }

    /// Append a scar record. If `record.scar_id == 0` allocates a fresh
    /// id; otherwise uses the supplied id verbatim (round-trip).
    pub fn push(&mut self, mut record: ScarRecord) -> ScarId {
        if record.scar_id.0 == 0 {
            record.scar_id = self.alloc_id();
        } else {
            self.next_scar_id = self.next_scar_id.max(record.scar_id.0 + 1);
        }
        let id = record.scar_id;
        self.scars.push(record);
        id
    }

    pub fn is_empty(&self) -> bool {
        self.scars.is_empty()
    }

    pub fn len(&self) -> usize {
        self.scars.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ScarRecord> {
        self.scars.iter()
    }

    /// Append-only checksum bytes for save / load round-trip determinism.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.scars.len() as u64).to_le_bytes());
        for s in &self.scars {
            out.extend_from_slice(&s.scar_id.0.to_le_bytes());
            out.push(s.source_wound_kind as u8);
            out.extend_from_slice(s.zone.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&s.severity_at_close.to_le_bytes());
            out.push(s.closure_method as u8);
            out.extend_from_slice(&s.tick_acquired.to_le_bytes());
            // Functional debuff
            let (tag, a, b) = s.functional_debuff.checksum_triple();
            out.push(tag);
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&b.to_le_bytes());
            out.extend_from_slice(s.cosmetic_decal_id.as_str().as_bytes());
            out.push(0);
            if let Some(n) = s.narrative_context.as_ref() {
                out.push(1);
                out.extend_from_slice(n.as_str().as_bytes());
            } else {
                out.push(0);
            }
            out.push(0);
        }
        out.extend_from_slice(&self.next_scar_id.to_le_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scar_timeline_round_trip() {
        let mut t = ScarTimeline::new();
        let zone = ZoneId::from("arm_left");
        let scar = ScarRecord::new(
            ScarId(0),
            WoundKind::LacerationSevere,
            zone.clone(),
            0.8,
            TreatmentKind::SutureKit,
            42,
            VisualDecalId::from("scar_suture_severe"),
        );
        let id = t.push(scar);
        assert_eq!(id.0, 0);
        assert_eq!(t.len(), 1);
        // VAL-M14I scenario 1: ReducedZoneStrength{arm_left, 0.05}.
        match &t.scars[0].functional_debuff {
            FunctionalDebuff::ReducedZoneStrength { zone: z, pct } => {
                assert_eq!(z.as_str(), "arm_left");
                assert!((*pct - 0.05).abs() < 1e-6);
            }
            other => panic!("expected ReducedZoneStrength got {:?}", other),
        }
    }

    #[test]
    fn scar_ids_monotonic() {
        let mut t = ScarTimeline::new();
        for i in 0..5 {
            let id = t.push(ScarRecord::new(
                ScarId(0),
                WoundKind::LacerationLight,
                ZoneId::from("torso_front"),
                0.2,
                TreatmentKind::SutureKit,
                i as u64,
                VisualDecalId::from("scar_default"),
            ));
            assert_eq!(id.0, i as u64);
        }
        assert_eq!(t.next_scar_id, 5);
    }

    #[test]
    fn checksum_deterministic() {
        let mut a = ScarTimeline::new();
        let mut b = ScarTimeline::new();
        let zone = ZoneId::from("leg_right");
        for tick in 0..3 {
            a.push(ScarRecord::new(
                ScarId(0),
                WoundKind::Burn3rd,
                zone.clone(),
                0.9,
                TreatmentKind::SurgeryKit,
                tick,
                VisualDecalId::from("scar_cauterized"),
            ));
            b.push(ScarRecord::new(
                ScarId(0),
                WoundKind::Burn3rd,
                zone.clone(),
                0.9,
                TreatmentKind::SurgeryKit,
                tick,
                VisualDecalId::from("scar_cauterized"),
            ));
        }
        assert_eq!(a.checksum_bytes(), b.checksum_bytes());
    }
}
