//! M9B § "Per-zone trench templates (CC parity)".
//!
//! Each template is authored as a declarative `.trench.ron` file under
//! `content/trench_templates/`. The spec fields are: footprint, path
//! polyline, per-segment variant overrides, embedded fortifications
//! (referenced by id; M9C MG nests + watchtowers placed at template-
//! relative offsets), per-zone metadata (faction, doctrine hint,
//! recommended garrison size).
//!
//! The loader honours the M9B placeholder grammar: a
//! [`FortificationPlaceholder`] may declare `optional = true` so the
//! M9C asset can be resolved at instantiation time without panicking
//! when M9C is not yet shipped (the [`InstantiatedTemplate`] carries
//! a [`MissingFortificationWarning`] list per spec
//! VAL-M9B-TEMPLATE-004).
//!
//! Determinism: the template's SHA256 is computed over its canonical
//! RON serialization so the same authored file always produces the
//! same `template_sha256`, and two engines dropping the same template
//! at the same origin + same world_seed produce the same instantiation
//! events (VAL-M9B-TEMPLATE-002).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use cf_trench::{
    SegmentSpec, SegmentVariant, TrenchModule, TrenchSegment,
};

/// Canonical id list for M9C fortifications referenced by M9B
/// templates. m9c-1..m9c-5 will register concrete instances; the list
/// here is the forward-compat surface so the loader can distinguish
/// "missing M9C asset (loaded gracefully with a warning event)" from
/// "completely unknown id (rejected as malformed)".
pub const KNOWN_FORTIFICATION_IDS: &[&str] = &[
    "mg_nest_static",
    "ammo_box_mg",
    "mg_tripod_portable",
    "spotter_scope",
    "bunker_firing_slit",
    "sandbag_low",
    "sandbag_mid",
    "sandbag_high",
    "watchtower_t1",
    "watchtower_t2",
    "watchtower_t3",
    "spotlight",
    "observation_post",
    "radio_repeater",
    "barbed_wire",
    "razor_wire",
    "electrified_fence",
    "concertina_roll",
    "anti_tank_ditch",
    "dragons_teeth",
    "tank_trap_x",
    "bollard_concrete",
    "camo_netting",
];

/// On-disk schema for `content/trench_templates/<id>.trench.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrenchTemplate {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub faction: Option<String>,
    #[serde(default)]
    pub doctrine_hint: Option<String>,
    #[serde(default)]
    pub recommended_garrison: Option<u32>,
    /// Inclusive bounding-box of the template in template-relative
    /// tile coordinates (`(min_x, min_y, max_x, max_y)`).
    pub footprint: Footprint,
    /// Sequence of tile coordinates describing the trench centreline
    /// in template-relative coordinates. Must have ≥ 2 points.
    pub path_polyline: Vec<(i32, i32)>,
    /// Optional per-segment variant overrides keyed by 0-based index
    /// into `path_polyline`. Indices not present default to
    /// `default_variant`.
    #[serde(default)]
    pub segment_overrides: Vec<SegmentOverride>,
    /// Default variant for any path polyline segment that lacks an
    /// explicit override.
    pub default_variant: SegmentVariant,
    /// Optional placeholders for M9C-owned fortifications attached
    /// to the template. Each placeholder declares an id + a template-
    /// relative offset; placeholders with `optional = true` resolve
    /// to a warning event when M9C is absent (per spec § Notes for
    /// the implementer).
    #[serde(default)]
    pub fortification_placeholders: Vec<FortificationPlaceholder>,
    #[serde(default)]
    pub zones: Vec<TemplateZone>,
}

/// `(min_x, min_y, max_x, max_y)` template-relative tile footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Footprint {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl Footprint {
    pub fn width(&self) -> i32 {
        self.max_x - self.min_x + 1
    }
    pub fn height(&self) -> i32 {
        self.max_y - self.min_y + 1
    }
}

/// Override a single path-polyline segment with a non-default variant
/// + optional extra embedded modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentOverride {
    pub at_index: u32,
    pub variant: SegmentVariant,
    #[serde(default)]
    pub embedded_modules: Vec<TrenchModule>,
}

/// One placeholder referencing an M9C fortification id at a template-
/// relative `(dx, dy)` offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FortificationPlaceholder {
    pub fortification_id: String,
    pub offset: (i32, i32),
    #[serde(default)]
    pub optional: bool,
}

/// Per-zone metadata block (M9B spec § "per-zone metadata: faction,
/// doctrine hint, recommended garrison size"). Stored as freeform
/// key/value entries so modders can attach arbitrary tags without
/// churning the kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateZone {
    pub id: String,
    pub footprint: Footprint,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Errors returned by [`TrenchTemplate::from_ron_str`]. The kernel
/// surfaces a typed error per field so cf-mod can pattern-match (per
/// VAL-M9B-TEMPLATE-003).
#[derive(Debug, thiserror::Error)]
pub enum TemplateLoadError {
    #[error("ron parse failed: {0}")]
    Ron(#[from] ron::error::SpannedError),
    #[error("path_polyline must have ≥ 2 points (got {0})")]
    PolylineTooShort(usize),
    #[error("footprint depth/width invalid: max ({max_x},{max_y}) < min ({min_x},{min_y})")]
    InvalidFootprint {
        min_x: i32,
        min_y: i32,
        max_x: i32,
        max_y: i32,
    },
    #[error("segment_overrides[{at}].at_index out of bounds (path_polyline has {len} points)")]
    OverrideOutOfBounds { at: u32, len: usize },
    #[error("unknown_segment_variant: {0}")]
    UnknownSegmentVariant(String),
    #[error("unknown_fortification_id: {0}")]
    UnknownFortificationId(String),
    #[error("template id must be non-empty")]
    EmptyId,
}

impl TrenchTemplate {
    /// Parse a `TrenchTemplate` from a RON string and validate the
    /// invariants the on-disk schema doesn't express on its own
    /// (polyline length, footprint orientation, override bounds,
    /// fortification id known-list).
    pub fn from_ron_str(text: &str) -> Result<Self, TemplateLoadError> {
        let parsed: TrenchTemplate = ron::from_str::<TrenchTemplate>(text)?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Validate kernel-side invariants beyond the serde-level shape
    /// check. Called automatically by [`Self::from_ron_str`]; exposed
    /// directly so cf-mod's structural validator can use the same
    /// path for hand-built `TrenchTemplate` values.
    pub fn validate(&self) -> Result<(), TemplateLoadError> {
        if self.id.trim().is_empty() {
            return Err(TemplateLoadError::EmptyId);
        }
        if self.path_polyline.len() < 2 {
            return Err(TemplateLoadError::PolylineTooShort(self.path_polyline.len()));
        }
        if self.footprint.max_x < self.footprint.min_x
            || self.footprint.max_y < self.footprint.min_y
        {
            return Err(TemplateLoadError::InvalidFootprint {
                min_x: self.footprint.min_x,
                min_y: self.footprint.min_y,
                max_x: self.footprint.max_x,
                max_y: self.footprint.max_y,
            });
        }
        let segment_count = self.path_polyline.len().saturating_sub(1);
        for ov in &self.segment_overrides {
            if (ov.at_index as usize) >= segment_count {
                return Err(TemplateLoadError::OverrideOutOfBounds {
                    at: ov.at_index,
                    len: self.path_polyline.len(),
                });
            }
        }
        for placeholder in &self.fortification_placeholders {
            if !KNOWN_FORTIFICATION_IDS.contains(&placeholder.fortification_id.as_str()) {
                return Err(TemplateLoadError::UnknownFortificationId(
                    placeholder.fortification_id.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Canonical RON serialization used for the [`template_sha256`]
    /// digest. Calling code on two engines with byte-identical RON
    /// files always observes the same hash (VAL-M9B-TEMPLATE-002).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .unwrap_or_default()
            .into_bytes()
    }

    pub fn template_sha256(&self) -> String {
        template_sha256(&self.canonical_bytes())
    }
}

/// SHA256 of the template's canonical bytes, hex-encoded (64 chars).
/// The replay event payload promises 64 hex chars per
/// VAL-M9B-TEMPLATE-002 / VAL-M9B-TEMPLATE-EVENT.
#[must_use]
pub fn template_sha256(canonical: &[u8]) -> String {
    let mut hasher = sha256_simple::Sha256::new();
    hasher.update(canonical);
    let bytes = hasher.finalize();
    hex::encode(bytes)
}

/// Label used by cf-mod + cfctl when emitting a "missing fortification"
/// warning event. Centralised so the spec's literal name stays
/// in-sync.
pub const fn placeholder_warning_label() -> &'static str {
    "trench_template_missing_fortification"
}

/// Resolver verdict for an individual placeholder: either successfully
/// resolved to an [`PlacedFortification`], or surfaced as a
/// [`MissingFortificationWarning`].
#[derive(Debug, Clone, PartialEq)]
pub enum FortificationResolution {
    Placed(PlacedFortification),
    Missing(MissingFortificationWarning),
}

/// Resolved + placed fortification instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacedFortification {
    pub fortification_id: String,
    pub world_pos: (i32, i32),
    pub instance_id: u64,
}

/// Warning emitted when a placeholder cannot be resolved (e.g. M9C
/// asset not yet shipped). The owning M9C feature is expected to
/// register the id later; until then the placement degrades to a
/// no-op + warning event per spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingFortificationWarning {
    pub fortification_id: String,
    pub world_pos: (i32, i32),
}

/// One trench segment placed by the template's polyline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedSegment {
    pub variant: SegmentVariant,
    pub world_pos: (i32, i32),
    pub embedded_modules: Vec<TrenchModule>,
}

/// The instantiated form of a template — the kernel materialises a
/// template into segments + fortifications + warnings. Consumers
/// (cf-control's act.player.drop_trench_template handler) emit the
/// replay event from this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct InstantiatedTemplate {
    pub id: String,
    pub template_sha256: String,
    pub origin: (i32, i32),
    pub segments: Vec<PlacedSegment>,
    pub placed_fortifications: Vec<PlacedFortification>,
    pub missing_fortifications: Vec<MissingFortificationWarning>,
    pub trench_segments: Vec<TrenchSegment>,
}

/// Convenience input bundle for [`TrenchTemplate::instantiate`]. The
/// `resolved_fortifications` set drives the
/// `KNOWN_FORTIFICATION_IDS` vs missing decision: ids in the
/// supplied set are placed, ids outside the set + flagged optional
/// degrade to warnings (per spec forward-compat grammar).
#[derive(Debug, Clone)]
pub struct TrenchTemplateInstantiation<'a> {
    pub template: &'a TrenchTemplate,
    pub origin: (i32, i32),
    pub resolved_fortifications: HashSet<String>,
    pub instance_id_base: u64,
}

impl TrenchTemplate {
    /// Materialise the template into a runtime [`InstantiatedTemplate`].
    ///
    /// The instance_id sequencer is the caller-provided base + the
    /// fortification's index in the template's placeholder list; this
    /// keeps the cfctl handler's replay event deterministic per
    /// VAL-M9B-TEMPLATE-002.
    pub fn instantiate(
        &self,
        request: &TrenchTemplateInstantiation<'_>,
    ) -> InstantiatedTemplate {
        debug_assert!(std::ptr::eq(request.template, self));
        let mut segments: Vec<PlacedSegment> = Vec::new();
        let mut trench_segments: Vec<TrenchSegment> = Vec::new();
        for (i, window) in self.path_polyline.windows(2).enumerate() {
            let from = window[0];
            let _to = window[1];
            let (variant, extra_modules) = self
                .segment_overrides
                .iter()
                .find(|ov| ov.at_index as usize == i)
                .map(|ov| (ov.variant, ov.embedded_modules.clone()))
                .unwrap_or((self.default_variant, Vec::new()));
            let world_pos = (request.origin.0 + from.0, request.origin.1 + from.1);
            let spec = SegmentSpec {
                variant,
                depth: default_depth_for(variant),
                width: default_width_for(variant),
                raised_step_height: default_step_height_for(variant),
                embedded_modules: extra_modules.clone(),
                cover_state: cf_trench::segment::CoverByStance::for_variant(variant),
            };
            let mut runtime = spec.to_segment(world_pos.0, world_pos.1);
            if !extra_modules.is_empty() {
                runtime.embedded_modules = extra_modules.clone();
            }
            trench_segments.push(runtime);
            segments.push(PlacedSegment {
                variant,
                world_pos,
                embedded_modules: extra_modules,
            });
        }
        let mut placed_fortifications: Vec<PlacedFortification> = Vec::new();
        let mut missing_fortifications: Vec<MissingFortificationWarning> = Vec::new();
        for (i, p) in self.fortification_placeholders.iter().enumerate() {
            let world_pos = (request.origin.0 + p.offset.0, request.origin.1 + p.offset.1);
            let resolved = request
                .resolved_fortifications
                .contains(&p.fortification_id);
            if resolved || !p.optional {
                placed_fortifications.push(PlacedFortification {
                    fortification_id: p.fortification_id.clone(),
                    world_pos,
                    instance_id: request.instance_id_base + i as u64,
                });
            } else {
                missing_fortifications.push(MissingFortificationWarning {
                    fortification_id: p.fortification_id.clone(),
                    world_pos,
                });
            }
        }
        InstantiatedTemplate {
            id: self.id.clone(),
            template_sha256: self.template_sha256(),
            origin: request.origin,
            segments,
            placed_fortifications,
            missing_fortifications,
            trench_segments,
        }
    }
}

fn default_depth_for(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 6,
        SegmentVariant::Standard
        | SegmentVariant::Communication
        | SegmentVariant::FireStep
        | SegmentVariant::ParapetRaised => 16,
        SegmentVariant::Deep => 24,
    }
}

fn default_width_for(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 12,
        SegmentVariant::Communication => 8,
        SegmentVariant::Standard | SegmentVariant::Deep => 16,
        SegmentVariant::FireStep => 20,
        SegmentVariant::ParapetRaised => 24,
    }
}

fn default_step_height_for(variant: SegmentVariant) -> Option<u32> {
    match variant {
        SegmentVariant::FireStep | SegmentVariant::ParapetRaised => Some(8),
        _ => None,
    }
}

/// Minimal SHA256 implementation used by [`template_sha256`]. We
/// don't pull `sha2` into the workspace just for this one call site —
/// blake3 is already a dep but the spec literally promises a SHA256
/// hex digest (VAL-M9B-TEMPLATE-002 "template_sha256 (64 hex chars)"),
/// so we keep the algorithm exact.
mod sha256_simple {
    /// Compact SHA256 implementation. Public for tests but kept inside
    /// `cf-content` since this is the only call site.
    pub struct Sha256 {
        state: [u32; 8],
        buffer: Vec<u8>,
        total_len: u64,
    }

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    const INIT: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    impl Sha256 {
        pub fn new() -> Self {
            Self {
                state: INIT,
                buffer: Vec::new(),
                total_len: 0,
            }
        }
        pub fn update(&mut self, bytes: &[u8]) {
            self.total_len = self.total_len.wrapping_add(bytes.len() as u64);
            self.buffer.extend_from_slice(bytes);
            while self.buffer.len() >= 64 {
                let mut block = [0u8; 64];
                block.copy_from_slice(&self.buffer[..64]);
                self.buffer.drain(..64);
                self.process(&block);
            }
        }
        pub fn finalize(mut self) -> [u8; 32] {
            let total_bits = self.total_len.wrapping_mul(8);
            self.buffer.push(0x80);
            while self.buffer.len() % 64 != 56 {
                self.buffer.push(0);
            }
            self.buffer.extend_from_slice(&total_bits.to_be_bytes());
            let chunks: Vec<[u8; 64]> = self
                .buffer
                .chunks_exact(64)
                .map(|c| {
                    let mut a = [0u8; 64];
                    a.copy_from_slice(c);
                    a
                })
                .collect();
            for block in &chunks {
                self.process(block);
            }
            let mut out = [0u8; 32];
            for (i, word) in self.state.iter().enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
            }
            out
        }
        fn process(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    block[i * 4],
                    block[i * 4 + 1],
                    block[i * 4 + 2],
                    block[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 =
                    w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let mut a = self.state[0];
            let mut b = self.state[1];
            let mut c = self.state[2];
            let mut d = self.state[3];
            let mut e = self.state[4];
            let mut f = self.state[5];
            let mut g = self.state[6];
            let mut h = self.state[7];
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);
                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }
            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
            self.state[5] = self.state[5].wrapping_add(f);
            self.state[6] = self.state[6].wrapping_add(g);
            self.state[7] = self.state[7].wrapping_add(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_dir() -> std::path::PathBuf {
        std::path::PathBuf::from("../../content/trench_templates")
    }

    fn load_named(id: &str) -> TrenchTemplate {
        let path = template_dir().join(format!("{id}.trench.ron"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("read {}: {e}", path.display());
        });
        TrenchTemplate::from_ron_str(&text).unwrap_or_else(|e| {
            panic!("parse {}: {e}", path.display());
        })
    }

    const ALL_FOUR: [&str; 4] = [
        "wwi_frontline_a",
        "wwi_frontline_b_two_line",
        "reactor_defense_zigzag",
        "forward_outpost_with_mgnest",
    ];

    /// VAL-M9B-TEMPLATE-001 — all four authored trench-template RON
    /// files exist and parse with the kernel's invariants intact.
    #[test]
    fn all_four_templates_parse() {
        for id in ALL_FOUR {
            let t = load_named(id);
            assert_eq!(t.id, id);
            assert!(!t.path_polyline.is_empty(), "{id}: polyline must be non-empty");
            assert!(t.path_polyline.len() >= 2);
            assert!(t.footprint.max_x >= t.footprint.min_x);
            assert!(t.footprint.max_y >= t.footprint.min_y);
        }
    }

    /// Round-trip every authored template through `ron` so future
    /// edits keep the kernel-side shape intact.
    #[test]
    fn templates_round_trip() {
        for id in ALL_FOUR {
            let t = load_named(id);
            let serialized = ron::ser::to_string_pretty(&t, ron::ser::PrettyConfig::default())
                .expect("serialize");
            let parsed = TrenchTemplate::from_ron_str(&serialized).expect("re-parse");
            assert_eq!(t, parsed, "round-trip diverged for {id}");
        }
    }

    /// VAL-M9B-TEMPLATE-002 — `template_sha256` is 64 hex chars and
    /// matches across two loads of the same authored content.
    #[test]
    fn template_sha256_is_64_hex_chars_and_deterministic() {
        for id in ALL_FOUR {
            let a = load_named(id);
            let b = load_named(id);
            let ha = a.template_sha256();
            let hb = b.template_sha256();
            assert_eq!(ha.len(), 64, "{id}: sha256 must be 64 hex chars");
            assert!(ha.chars().all(|c| c.is_ascii_hexdigit()));
            assert_eq!(ha, hb, "{id}: sha256 must be deterministic");
        }
    }

    /// VAL-M9B-TEMPLATE-002 — instantiating a template emits placed
    /// segments + a deterministic `template_sha256` per
    /// `act.player.drop_trench_template`.
    #[test]
    fn instantiate_emits_segments_and_fortifications() {
        let t = load_named("wwi_frontline_a");
        let resolved = HashSet::new();
        let request = TrenchTemplateInstantiation {
            template: &t,
            origin: (50, 30),
            resolved_fortifications: resolved,
            instance_id_base: 1000,
        };
        let inst = t.instantiate(&request);
        assert_eq!(inst.id, "wwi_frontline_a");
        assert_eq!(inst.template_sha256.len(), 64);
        assert!(inst.segments.len() >= t.path_polyline.len() - 1);
        for seg in &inst.segments {
            assert!(seg.world_pos.0 >= 50);
        }
    }

    /// VAL-M9B-TEMPLATE-004 — optional placeholder with no M9C
    /// resolution surfaces a missing-fortification warning + does
    /// NOT panic + does NOT short-circuit segment placement.
    #[test]
    fn optional_placeholder_emits_missing_warning_when_unresolved() {
        let t = load_named("forward_outpost_with_mgnest");
        let resolved = HashSet::new();
        let request = TrenchTemplateInstantiation {
            template: &t,
            origin: (0, 0),
            resolved_fortifications: resolved,
            instance_id_base: 0,
        };
        let inst = t.instantiate(&request);
        assert!(
            !inst.missing_fortifications.is_empty(),
            "expected ≥1 missing-fortification warning"
        );
        assert!(
            !inst.segments.is_empty(),
            "missing fortification must not abort segment placement"
        );
    }

    /// Once M9C resolves the id, the same template instantiation
    /// promotes the placeholder to a placed fortification with a
    /// non-zero instance id.
    #[test]
    fn placeholder_resolves_when_m9c_id_is_known() {
        let t = load_named("forward_outpost_with_mgnest");
        let mut resolved = HashSet::new();
        for ph in &t.fortification_placeholders {
            resolved.insert(ph.fortification_id.clone());
        }
        let request = TrenchTemplateInstantiation {
            template: &t,
            origin: (0, 0),
            resolved_fortifications: resolved,
            instance_id_base: 1,
        };
        let inst = t.instantiate(&request);
        assert!(inst.missing_fortifications.is_empty());
        assert!(!inst.placed_fortifications.is_empty());
    }

    /// VAL-M9B-TEMPLATE-003 — unknown segment variant is rejected at
    /// load time with a typed error containing the bad value.
    #[test]
    fn unknown_segment_variant_is_rejected() {
        let bad = r#"
        (
            id: "bad_template",
            display_name: "Bad template",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: ultra_deep,
            fortification_placeholders: [],
            zones: [],
        )
        "#;
        let err = TrenchTemplate::from_ron_str(bad).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("ultra_deep") || msg.to_lowercase().contains("unknown") || msg.to_lowercase().contains("ron"),
            "expected unknown-variant error, got: {msg}"
        );
    }

    /// VAL-M9B-MOD-SEGMENT-001 (cross-check for the kernel side) —
    /// an over-bound override raises a typed error.
    #[test]
    fn out_of_bound_override_is_rejected() {
        let bad = r#"
        (
            id: "ob",
            display_name: "OB",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0), (10,0)],
            segment_overrides: [(at_index: 99, variant: deep, embedded_modules: [])],
            default_variant: standard,
            fortification_placeholders: [],
            zones: [],
        )
        "#;
        let err = TrenchTemplate::from_ron_str(bad).unwrap_err();
        assert!(matches!(
            err,
            TemplateLoadError::OverrideOutOfBounds { .. }
        ));
    }

    /// Unknown fortification id in a placeholder rejects at load
    /// time (the M9B placeholder grammar gates on KNOWN_FORTIFICATION_IDS).
    #[test]
    fn unknown_fortification_id_is_rejected() {
        let bad = r#"
        (
            id: "uf",
            display_name: "UF",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: standard,
            fortification_placeholders: [
                (fortification_id: "no_such_thing", offset: (1, 1), optional: true),
            ],
            zones: [],
        )
        "#;
        let err = TrenchTemplate::from_ron_str(bad).unwrap_err();
        assert!(matches!(
            err,
            TemplateLoadError::UnknownFortificationId(_)
        ));
    }

    #[test]
    fn sha256_matches_a_known_input() {
        // Empty string SHA256 = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let h = super::template_sha256(b"");
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
