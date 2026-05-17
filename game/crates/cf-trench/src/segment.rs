//! M9B: trench-segment types and on-disk RON spec.
//!
//! A [`TrenchSegment`] is a placed instance of a cross-section
//! [`SegmentVariant`] in the world. Authored content under
//! `content/trench_segments/<variant>.ron` deserializes into
//! [`SegmentSpec`]; the [`SegmentSpec::to_segment`] helper instantiates
//! a runtime [`TrenchSegment`] (M9B-1 does not yet place segments — that
//! lands in m9b-2 / m9b-3 — but the data model + RON loader is in place
//! for those features to consume).

use serde::{Deserialize, Serialize};

use crate::cover_state::{cover_state, CoverState, TrenchStance};
use crate::modules::TrenchModule;

/// One of the six authored trench cross-section variants per M9B spec
/// §"Trench cross-section variants (6 authored)". The RON files
/// shipped under `content/trench_segments/` map 1:1 to these variants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentVariant {
    ShallowScrape = 0,
    Standard = 1,
    Deep = 2,
    Communication = 3,
    FireStep = 4,
    ParapetRaised = 5,
}

impl SegmentVariant {
    pub const ALL: [SegmentVariant; 6] = [
        SegmentVariant::ShallowScrape,
        SegmentVariant::Standard,
        SegmentVariant::Deep,
        SegmentVariant::Communication,
        SegmentVariant::FireStep,
        SegmentVariant::ParapetRaised,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            SegmentVariant::ShallowScrape => "shallow_scrape",
            SegmentVariant::Standard => "standard",
            SegmentVariant::Deep => "deep",
            SegmentVariant::Communication => "communication",
            SegmentVariant::FireStep => "fire_step",
            SegmentVariant::ParapetRaised => "parapet_raised",
        }
    }
}

/// On-disk schema for `content/trench_segments/<variant>.ron`. Fields
/// match the spec's table row exactly: depth, width, embedded modules,
/// per-stance cover state, and (for `fire_step`) the raised-step height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentSpec {
    pub variant: SegmentVariant,
    /// Vertical extent of the trench body in world pixels.
    pub depth: u32,
    /// Horizontal extent of the trench floor in world pixels.
    pub width: u32,
    /// `Some(h)` when the variant ships a raised firing step within the
    /// trench (`fire_step`'s 8 px step, `parapet_raised`'s 8 px breastwork
    /// rising above grade). `None` otherwise. The numeric value is the
    /// height of the addition in world pixels.
    #[serde(default)]
    pub raised_step_height: Option<u32>,
    /// Embedded module catalog references (resolved against the
    /// `content/trench_modules/` RON loader). Stored as ids so the spec
    /// stays declarative.
    #[serde(default)]
    pub embedded_modules: Vec<TrenchModule>,
    /// Explicit per-stance cover map authored alongside the variant.
    /// Loader tests check this matches the derived
    /// `cover_state(stance, variant)` so the RON content and the kernel
    /// can never drift.
    pub cover_state: CoverByStance,
}

/// Per-stance cover values authored on each trench segment RON. Fields
/// use the spec's table column names verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverByStance {
    pub standing: CoverState,
    pub crouched: CoverState,
    pub prone: CoverState,
}

impl CoverByStance {
    #[must_use]
    pub fn for_variant(variant: SegmentVariant) -> Self {
        Self {
            standing: cover_state(TrenchStance::Standing, variant),
            crouched: cover_state(TrenchStance::Crouched, variant),
            prone: cover_state(TrenchStance::Prone, variant),
        }
    }
}

impl SegmentSpec {
    /// Parse a `SegmentSpec` from a RON string. Returns a typed
    /// `ron::SpannedError` so loader tests can point at malformed enum
    /// values precisely (used by cf-mod validation in m9b-2).
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<SegmentSpec>(text)
    }

    /// Instantiate a runtime [`TrenchSegment`] at the supplied tile.
    pub fn to_segment(&self, tile_x: i32, tile_y: i32) -> TrenchSegment {
        TrenchSegment {
            variant: self.variant,
            tile_x,
            tile_y,
            depth: self.depth,
            width: self.width,
            raised_step_height: self.raised_step_height,
            embedded_modules: self.embedded_modules.clone(),
        }
    }
}

/// Runtime instance of an authored trench segment placed in the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrenchSegment {
    pub variant: SegmentVariant,
    pub tile_x: i32,
    pub tile_y: i32,
    pub depth: u32,
    pub width: u32,
    #[serde(default)]
    pub raised_step_height: Option<u32>,
    #[serde(default)]
    pub embedded_modules: Vec<TrenchModule>,
}

impl TrenchSegment {
    /// Project a runtime segment into the on-disk authoring form so
    /// callers can round-trip a placed segment through the same loader
    /// (used by m9b-2 zigzag determinism testing).
    #[must_use]
    pub fn to_spec(&self) -> SegmentSpec {
        SegmentSpec {
            variant: self.variant,
            depth: self.depth,
            width: self.width,
            raised_step_height: self.raised_step_height,
            embedded_modules: self.embedded_modules.clone(),
            cover_state: CoverByStance::for_variant(self.variant),
        }
    }
}

/// Spatial lookup the cf-actor `ActorState::cover_state(&world)` helper
/// uses. Implementations are owned by the engine crates that place
/// segments (m9b-2's procgen + m9b-3's cfctl handlers). For unit tests
/// a minimal in-memory implementation is provided below the trait.
pub trait TrenchSegmentLookup {
    /// Return the segment whose floor covers the supplied tile, or
    /// `None` if the position is open ground. World units; callers
    /// (e.g. `cf-actor`) cast `f32` actor positions through `as i32`.
    fn segment_at(&self, tile_x: i32, tile_y: i32) -> Option<&TrenchSegment>;
}

/// Convenience wrapper: lookup a segment by 1-tile vector. Used by the
/// in-process tests that simulate "actor moves between segments" without
/// needing the full world.
#[derive(Debug, Default, Clone)]
pub struct InMemorySegments {
    pub segments: Vec<TrenchSegment>,
}

impl InMemorySegments {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_segments(segments: Vec<TrenchSegment>) -> Self {
        Self { segments }
    }

    pub fn push(&mut self, seg: TrenchSegment) {
        self.segments.push(seg);
    }
}

impl TrenchSegmentLookup for InMemorySegments {
    fn segment_at(&self, tile_x: i32, tile_y: i32) -> Option<&TrenchSegment> {
        self.segments.iter().find(|s| {
            let x0 = s.tile_x;
            let x1 = s.tile_x + s.width as i32;
            let y0 = s.tile_y;
            let y1 = s.tile_y + s.depth as i32;
            tile_x >= x0 && tile_x < x1 && tile_y >= y0 && tile_y < y1
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::TrenchModule;

    fn load(variant: SegmentVariant) -> SegmentSpec {
        let rel = format!(
            "../../content/trench_segments/{}.ron",
            variant.as_str()
        );
        let bytes = std::fs::read_to_string(&rel)
            .unwrap_or_else(|e| panic!("read {}: {}", rel, e));
        SegmentSpec::from_ron_str(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {}", rel, e))
    }

    #[test]
    fn loads_shallow_scrape() {
        let s = load(SegmentVariant::ShallowScrape);
        assert_eq!(s.variant, SegmentVariant::ShallowScrape);
        assert_eq!(s.depth, 6);
        assert_eq!(s.width, 12);
        assert!(s.embedded_modules.is_empty());
        assert_eq!(s.cover_state, CoverByStance::for_variant(SegmentVariant::ShallowScrape));
    }

    #[test]
    fn loads_standard() {
        let s = load(SegmentVariant::Standard);
        assert_eq!(s.variant, SegmentVariant::Standard);
        assert_eq!(s.depth, 16);
        assert_eq!(s.width, 16);
        assert_eq!(s.embedded_modules, vec![TrenchModule::Duckboard]);
        assert_eq!(s.cover_state.standing, CoverState::Partial);
        assert_eq!(s.cover_state.crouched, CoverState::Full);
        assert_eq!(s.cover_state.prone, CoverState::Full);
    }

    #[test]
    fn loads_deep() {
        let s = load(SegmentVariant::Deep);
        assert_eq!(s.depth, 24);
        assert_eq!(s.width, 16);
        assert_eq!(
            s.embedded_modules,
            vec![TrenchModule::Duckboard, TrenchModule::DrainageSump]
        );
        assert_eq!(s.cover_state.standing, CoverState::Full);
    }

    #[test]
    fn loads_communication() {
        let s = load(SegmentVariant::Communication);
        assert_eq!(s.depth, 16);
        assert_eq!(s.width, 8);
        assert_eq!(s.embedded_modules, vec![TrenchModule::Duckboard]);
        assert_eq!(s.cover_state.standing, CoverState::Partial);
    }

    #[test]
    fn loads_fire_step() {
        let s = load(SegmentVariant::FireStep);
        assert_eq!(s.depth, 16);
        assert_eq!(s.width, 20);
        assert_eq!(s.raised_step_height, Some(8));
        assert_eq!(
            s.embedded_modules,
            vec![TrenchModule::Duckboard, TrenchModule::FireStep]
        );
        assert_eq!(s.cover_state.standing, CoverState::Exposed);
        assert_eq!(s.cover_state.prone, CoverState::Full);
    }

    #[test]
    fn loads_parapet_raised() {
        let s = load(SegmentVariant::ParapetRaised);
        assert_eq!(s.depth, 16);
        assert_eq!(s.width, 24);
        assert_eq!(s.raised_step_height, Some(8));
        assert_eq!(
            s.embedded_modules,
            vec![TrenchModule::Duckboard, TrenchModule::Breastwork]
        );
        assert_eq!(s.cover_state.standing, CoverState::Full);
    }

    /// VAL-M9B-SEGMENT-001 round-trip: every variant's authored RON
    /// re-serialises identically through `ron`.
    #[test]
    fn segment_ron_load_round_trip_all_variants() {
        for v in SegmentVariant::ALL {
            let s = load(v);
            let serialized = ron::ser::to_string_pretty(&s, ron::ser::PrettyConfig::default())
                .expect("serialize spec");
            let parsed = SegmentSpec::from_ron_str(&serialized).expect("re-parse spec");
            assert_eq!(s, parsed, "round-trip diverged for {:?}", v);
        }
    }

    #[test]
    fn in_memory_lookup_finds_overlapping_segment() {
        let seg = TrenchSegment {
            variant: SegmentVariant::Standard,
            tile_x: 10,
            tile_y: 0,
            depth: 16,
            width: 16,
            raised_step_height: None,
            embedded_modules: vec![TrenchModule::Duckboard],
        };
        let world = InMemorySegments::with_segments(vec![seg.clone()]);
        assert!(world.segment_at(10, 5).is_some());
        assert!(world.segment_at(25, 5).is_some());
        assert!(world.segment_at(9, 5).is_none(), "open ground left of segment");
        assert!(world.segment_at(26, 5).is_none(), "open ground right of segment");
    }

    #[test]
    fn segment_spec_to_segment_preserves_variant_dims_and_modules() {
        let spec = load(SegmentVariant::Standard);
        let seg = spec.to_segment(50, 30);
        assert_eq!(seg.variant, SegmentVariant::Standard);
        assert_eq!(seg.tile_x, 50);
        assert_eq!(seg.tile_y, 30);
        assert_eq!(seg.depth, 16);
        assert_eq!(seg.width, 16);
        assert_eq!(seg.embedded_modules, vec![TrenchModule::Duckboard]);
    }
}
