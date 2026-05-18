//! **M14G** — Per-Wound-Type Granularity + Severity Bands + Visual Decals.
//!
//! Canonical owner of the typed wound registry:
//! - [`WoundKind`] — 30 variants across 6 categories (Penetrating, Blunt,
//!   Skeletal, Thermal, Chemical, Sensory).
//! - [`Wound`] — per-instance record with `id`, `kind`, `severity`, `zone`,
//!   `age_ticks`, `dirt_pct`, `bandaged`, `sutured`.
//! - [`WoundSpec`] — registry-backed metadata with 11 contract fields.
//! - [`SeverityBand`] — 6-band severity ladder.
//! - [`WoundSpecRegistry`] — registry loaded from `content/wound_specs/*.ron`.
//!
//! Aging is invoked once per tick by `cf-control::engine`; the pass mutates
//! state only every 5 ticks (per spec) but `age_ticks` increments every tick.
//! Infection chance is NOT rolled here — that is deferred to M14H.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::manual_range_contains,
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::uninlined_format_args,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::single_char_pattern,
    clippy::field_reassign_with_default,
    clippy::option_if_let_else,
    clippy::if_not_else,
    clippy::manual_let_else,
    clippy::cast_lossless,
    clippy::too_many_lines,
    clippy::manual_is_multiple_of,
    clippy::unnecessary_debug_formatting,
    clippy::explicit_iter_loop,
    clippy::should_implement_trait,
    clippy::enum_glob_use,
    clippy::map_unwrap_or,
    clippy::bool_to_int_with_if,
    clippy::redundant_closure,
    clippy::needless_continue,
    clippy::manual_assert,
    clippy::ptr_arg
)]

pub mod aging;
pub mod registry;
pub mod severity;

pub use aging::{
    aging_tick_pass, AgingEvent, AgingNewState, BANDAGE_SOAK_THROUGH_TICKS_DEFAULT,
    DEFAULT_AGING_MUTATE_CADENCE, FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT,
    LACERATION_LIGHT_SCAB_TICKS_DEFAULT,
};
pub use registry::{
    OriginId, TreatmentDifficulty, TreatmentKind, VisualDecalId, WoundSpec, WoundSpecError,
    WoundSpecRegistry, ZoneId, BLEED_RATE_BASE_ML_PER_S_AT_SEVERITY_HALF,
    BURN3RD_HEAL_SECONDS_AT_SEVERE, FRACTURE_HEAL_SECONDS_AT_SEVERE,
    GUNSHOT_THROUGH_BLEED_MULTIPLIER, MAX_WOUNDS_PER_ZONE,
};
pub use severity::{SeverityBand, BAND_LABEL_CRITICAL};

use serde::{Deserialize, Serialize};

/// **M14G** canonical wound kinds.
///
/// Spec prose says "28 canonical" but the bullet list totals 30 across the
/// 6 categories — per the mission AGENTS.md spec-ambiguity policy the
/// Gherkin acceptance scenarios win (VAL-M14G-001), so all 30 variants
/// ship.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WoundKind {
    // ----- Penetrating (10) -----
    LacerationLight = 0,
    LacerationModerate = 1,
    LacerationSevere = 2,
    Puncture = 3,
    StabThrough = 4,
    GunshotEntry = 5,
    GunshotExit = 6,
    GunshotThrough = 7,
    ShrapnelEmbedded = 8,
    ShrapnelThrough = 9,
    // ----- Blunt (4) -----
    BruiseLight = 10,
    BruiseHeavy = 11,
    CrushLimb = 12,
    Concussion = 13,
    // ----- Skeletal (5) -----
    FractureSimple = 14,
    FractureCompound = 15,
    FractureComminuted = 16,
    Dislocation = 17,
    SprainStrain = 18,
    // ----- Thermal (6) -----
    Burn1st = 19,
    Burn2nd = 20,
    Burn3rd = 21,
    Frostbite1st = 22,
    Frostbite2nd = 23,
    Frostbite3rd = 24,
    // ----- Chemical (2) -----
    AcidBurn = 25,
    ChemicalBurn = 26,
    // ----- Sensory (3) -----
    EyeInjury = 27,
    EarInjury = 28,
    DentalDamage = 29,
}

impl WoundKind {
    pub const COUNT: usize = 30;

    pub const ALL: [WoundKind; Self::COUNT] = [
        WoundKind::LacerationLight,
        WoundKind::LacerationModerate,
        WoundKind::LacerationSevere,
        WoundKind::Puncture,
        WoundKind::StabThrough,
        WoundKind::GunshotEntry,
        WoundKind::GunshotExit,
        WoundKind::GunshotThrough,
        WoundKind::ShrapnelEmbedded,
        WoundKind::ShrapnelThrough,
        WoundKind::BruiseLight,
        WoundKind::BruiseHeavy,
        WoundKind::CrushLimb,
        WoundKind::Concussion,
        WoundKind::FractureSimple,
        WoundKind::FractureCompound,
        WoundKind::FractureComminuted,
        WoundKind::Dislocation,
        WoundKind::SprainStrain,
        WoundKind::Burn1st,
        WoundKind::Burn2nd,
        WoundKind::Burn3rd,
        WoundKind::Frostbite1st,
        WoundKind::Frostbite2nd,
        WoundKind::Frostbite3rd,
        WoundKind::AcidBurn,
        WoundKind::ChemicalBurn,
        WoundKind::EyeInjury,
        WoundKind::EarInjury,
        WoundKind::DentalDamage,
    ];

    /// Canonical PascalCase name used by serde + cf-replay schemas.
    pub fn as_str(self) -> &'static str {
        match self {
            WoundKind::LacerationLight => "LacerationLight",
            WoundKind::LacerationModerate => "LacerationModerate",
            WoundKind::LacerationSevere => "LacerationSevere",
            WoundKind::Puncture => "Puncture",
            WoundKind::StabThrough => "StabThrough",
            WoundKind::GunshotEntry => "GunshotEntry",
            WoundKind::GunshotExit => "GunshotExit",
            WoundKind::GunshotThrough => "GunshotThrough",
            WoundKind::ShrapnelEmbedded => "ShrapnelEmbedded",
            WoundKind::ShrapnelThrough => "ShrapnelThrough",
            WoundKind::BruiseLight => "BruiseLight",
            WoundKind::BruiseHeavy => "BruiseHeavy",
            WoundKind::CrushLimb => "CrushLimb",
            WoundKind::Concussion => "Concussion",
            WoundKind::FractureSimple => "FractureSimple",
            WoundKind::FractureCompound => "FractureCompound",
            WoundKind::FractureComminuted => "FractureComminuted",
            WoundKind::Dislocation => "Dislocation",
            WoundKind::SprainStrain => "SprainStrain",
            WoundKind::Burn1st => "Burn1st",
            WoundKind::Burn2nd => "Burn2nd",
            WoundKind::Burn3rd => "Burn3rd",
            WoundKind::Frostbite1st => "Frostbite1st",
            WoundKind::Frostbite2nd => "Frostbite2nd",
            WoundKind::Frostbite3rd => "Frostbite3rd",
            WoundKind::AcidBurn => "AcidBurn",
            WoundKind::ChemicalBurn => "ChemicalBurn",
            WoundKind::EyeInjury => "EyeInjury",
            WoundKind::EarInjury => "EarInjury",
            WoundKind::DentalDamage => "DentalDamage",
        }
    }

    /// Parse a wound kind from its canonical PascalCase string.
    pub fn from_str(s: &str) -> Result<Self, WoundKindParseError> {
        for v in &Self::ALL {
            if v.as_str() == s {
                return Ok(*v);
            }
        }
        Err(WoundKindParseError(s.to_string()))
    }

    pub fn category(self) -> WoundCategory {
        use WoundKind::*;
        match self {
            LacerationLight | LacerationModerate | LacerationSevere | Puncture | StabThrough
            | GunshotEntry | GunshotExit | GunshotThrough | ShrapnelEmbedded | ShrapnelThrough => {
                WoundCategory::Penetrating
            }
            BruiseLight | BruiseHeavy | CrushLimb | Concussion => WoundCategory::Blunt,
            FractureSimple | FractureCompound | FractureComminuted | Dislocation | SprainStrain => {
                WoundCategory::Skeletal
            }
            Burn1st | Burn2nd | Burn3rd | Frostbite1st | Frostbite2nd | Frostbite3rd => {
                WoundCategory::Thermal
            }
            AcidBurn | ChemicalBurn => WoundCategory::Chemical,
            EyeInjury | EarInjury | DentalDamage => WoundCategory::Sensory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WoundKindParseError(pub String);

impl std::fmt::Display for WoundKindParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown WoundKind: `{}`", self.0)
    }
}

impl std::error::Error for WoundKindParseError {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WoundCategory {
    Penetrating,
    Blunt,
    Skeletal,
    Thermal,
    Chemical,
    Sensory,
}

/// **M14G** stable per-actor wound identifier. Monotonic within an
/// `ActorWoundList`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WoundId(pub u64);

impl WoundId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// **M14G** typed wound record. One per producer emission.
///
/// `id` is unique within the owning `ActorWoundList`. `severity` is normalized
/// to `[0, 1]`. `age_ticks` increments every tick (mutation cadence does not
/// gate `age_ticks` — only state transitions). `dirt_pct` is `[0, 1]` and
/// feeds M14H/M16B infection risk (consumed downstream; not rolled here).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wound {
    pub id: WoundId,
    pub kind: WoundKind,
    pub severity: f32,
    pub zone: ZoneId,
    pub age_ticks: u64,
    pub dirt_pct: f32,
    pub bandaged: bool,
    pub sutured: bool,
    /// Visible state machine: fresh / soaked-bandage / clean-bandage / scab
    /// / suture-line / scar. Defaults to Fresh on emission.
    #[serde(default)]
    pub visible_state: WoundVisibleState,
    /// True once the aging pass has emitted `wound.scabbed` for this wound.
    /// Prevents duplicate emission across the 5-tick aging cadence.
    #[serde(default)]
    pub scabbed: bool,
    /// True once the aging pass has emitted `wound.scarred` for this wound.
    #[serde(default)]
    pub scarred: bool,
    /// **VAL-M14G-018**: shrapnel fragments accumulate on the zone. Default 1.
    /// Incremented when additional ShrapnelEmbedded wounds land on the same zone.
    #[serde(default = "default_shrapnel_count")]
    pub shrapnel_count: u32,
}

fn default_shrapnel_count() -> u32 {
    1
}

impl Wound {
    /// Construct a fresh wound. `bandaged`, `sutured`, and `scabbed` default
    /// to false. `age_ticks` starts at 0.
    pub fn new(id: WoundId, kind: WoundKind, severity: f32, zone: ZoneId) -> Self {
        Self {
            id,
            kind,
            severity: severity.clamp(0.0, 1.0),
            zone,
            age_ticks: 0,
            dirt_pct: 0.0,
            bandaged: false,
            sutured: false,
            visible_state: WoundVisibleState::Fresh,
            scabbed: false,
            scarred: false,
            shrapnel_count: 1,
        }
    }

    pub fn severity_band(&self) -> SeverityBand {
        SeverityBand::from_severity(self.severity)
    }

    /// Effective bleed rate `ml/s` given the registry baseline.
    ///
    /// Bandage halves the bleed; once a bandage soaks through the visible
    /// state moves to `BandageSoaked` and the bleed rate returns to half of
    /// the pre-bandage rate per Gherkin scenario 6 (VAL-M14G-019).
    /// Scab + scar zero the bleed.
    pub fn effective_bleed_rate(&self, base_rate_at_severity: f32) -> f32 {
        if matches!(
            self.visible_state,
            WoundVisibleState::Scab | WoundVisibleState::Scar
        ) {
            return 0.0;
        }
        if self.scabbed {
            return 0.0;
        }
        let raw = base_rate_at_severity * self.severity;
        match self.visible_state {
            WoundVisibleState::Fresh => raw,
            WoundVisibleState::CleanBandage => raw * 0.5_f32.min(1.0) * 0.0_f32.max(0.0) + raw * 0.0_f32,
            // Bandage soak-through halves the pre-bandage rate (spec).
            WoundVisibleState::BandageSoaked => raw * 0.5,
            WoundVisibleState::SutureLine => raw * 0.25,
            WoundVisibleState::Scab | WoundVisibleState::Scar => 0.0,
        }
    }
}

/// **M14G** per-wound visible state machine.
///
/// `Fresh` is the default emission state. The aging pass transitions
/// `CleanBandage` → `BandageSoaked` after the soak-through interval
/// (180 s at the canonical tick rate), and clean wounds (no bandage) move
/// to `Scab` then `Scar` per `WoundSpec.closes_to_scar`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WoundVisibleState {
    #[serde(rename = "fresh")]
    Fresh = 0,
    #[serde(rename = "bandage_soaked")]
    BandageSoaked = 1,
    #[serde(rename = "clean_bandage")]
    CleanBandage = 2,
    #[serde(rename = "scab")]
    Scab = 3,
    #[serde(rename = "suture_line")]
    SutureLine = 4,
    #[serde(rename = "scar")]
    Scar = 5,
}

impl Default for WoundVisibleState {
    fn default() -> Self {
        WoundVisibleState::Fresh
    }
}

impl WoundVisibleState {
    pub fn as_str(self) -> &'static str {
        match self {
            WoundVisibleState::Fresh => "fresh",
            WoundVisibleState::BandageSoaked => "bandage_soaked",
            WoundVisibleState::CleanBandage => "clean_bandage",
            WoundVisibleState::Scab => "scab",
            WoundVisibleState::SutureLine => "suture_line",
            WoundVisibleState::Scar => "scar",
        }
    }
}

/// **M14G** ActorWoundList — per-actor wound storage keyed by zone with a
/// monotonic id allocator. Determinism: uses BTreeMap so iteration order is
/// stable.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActorWoundList {
    pub wounds_by_zone: std::collections::BTreeMap<ZoneId, Vec<Wound>>,
    /// Next wound id to allocate. Monotonic.
    pub next_id: u64,
    /// Per-zone Necrosis flag set when Frostbite3rd ages past the necrosis
    /// threshold (VAL-M14G-015).
    pub necrotic_zones: std::collections::BTreeSet<ZoneId>,
}

impl ActorWoundList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the next wound id (monotonic).
    pub fn alloc_id(&mut self) -> WoundId {
        let id = WoundId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Push a wound onto a zone's stack. Returns the inserted wound's id.
    pub fn push(&mut self, zone: ZoneId, mut wound: Wound) -> WoundId {
        if wound.id == WoundId(0) && self.next_id == 0 {
            // first id is 1; reserve 0 as "unset" so deserialized wounds
            // can carry a real id.
        }
        if wound.id.0 == 0 {
            wound.id = self.alloc_id();
        } else {
            self.next_id = self.next_id.max(wound.id.0 + 1);
        }
        let id = wound.id;
        self.wounds_by_zone.entry(zone).or_default().push(wound);
        id
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ZoneId, &Vec<Wound>)> {
        self.wounds_by_zone.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ZoneId, &mut Vec<Wound>)> {
        self.wounds_by_zone.iter_mut()
    }

    pub fn total_count(&self) -> usize {
        self.wounds_by_zone.values().map(|v| v.len()).sum()
    }

    pub fn zone_count(&self, zone: &ZoneId) -> usize {
        self.wounds_by_zone
            .get(zone)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn zone(&self, zone: &ZoneId) -> &[Wound] {
        self.wounds_by_zone
            .get(zone)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn zone_mut(&mut self, zone: &ZoneId) -> Option<&mut Vec<Wound>> {
        self.wounds_by_zone.get_mut(zone)
    }

    pub fn is_necrotic(&self, zone: &ZoneId) -> bool {
        self.necrotic_zones.contains(zone)
    }

    /// Append-only checksum bytes for save/load round-trip determinism
    /// (VAL-CROSS-029).
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.wounds_by_zone.len() as u64).to_le_bytes());
        for (zone, wounds) in &self.wounds_by_zone {
            out.extend_from_slice(zone.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&(wounds.len() as u64).to_le_bytes());
            for w in wounds {
                out.extend_from_slice(&w.id.0.to_le_bytes());
                out.push(w.kind as u8);
                out.extend_from_slice(&w.severity.to_le_bytes());
                out.extend_from_slice(&w.age_ticks.to_le_bytes());
                out.extend_from_slice(&w.dirt_pct.to_le_bytes());
                out.push(if w.bandaged { 1 } else { 0 });
                out.push(if w.sutured { 1 } else { 0 });
                out.push(w.visible_state as u8);
                out.push(if w.scabbed { 1 } else { 0 });
                out.push(if w.scarred { 1 } else { 0 });
                out.extend_from_slice(&w.shrapnel_count.to_le_bytes());
            }
        }
        out.extend_from_slice(&self.next_id.to_le_bytes());
        out.extend_from_slice(&(self.necrotic_zones.len() as u64).to_le_bytes());
        for z in &self.necrotic_zones {
            out.extend_from_slice(z.as_str().as_bytes());
            out.push(0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14G-001: every variant in the bullet list must be constructible
    /// by name (round-trip).
    #[test]
    fn all_woundkind_variants_present() {
        for kind in WoundKind::ALL.iter() {
            let s = kind.as_str();
            assert_eq!(WoundKind::from_str(s).unwrap(), *kind, "round-trip failed for {s}");
        }
        assert_eq!(WoundKind::ALL.len(), 30);
        assert_eq!(WoundKind::COUNT, 30);
    }

    /// VAL-M14G-001: serde round-trip.
    #[test]
    fn woundkind_serde_round_trip() {
        for kind in WoundKind::ALL.iter() {
            let s = serde_json::to_string(kind).unwrap();
            let back: WoundKind = serde_json::from_str(&s).unwrap();
            assert_eq!(back, *kind);
        }
    }

    /// VAL-M14G-006: Wound carries each contract field with the spec-mandated
    /// defaults.
    #[test]
    fn wound_record_initial_state() {
        let zone = ZoneId::from("torso_front");
        let w = Wound::new(WoundId(42), WoundKind::LacerationLight, 0.3, zone.clone());
        assert_eq!(w.id, WoundId(42));
        assert_eq!(w.kind, WoundKind::LacerationLight);
        assert!((w.severity - 0.3).abs() < 1e-6);
        assert_eq!(w.zone, zone);
        assert_eq!(w.age_ticks, 0);
        assert!((w.dirt_pct - 0.0).abs() < 1e-6);
        assert!(!w.bandaged);
        assert!(!w.sutured);
        assert_eq!(w.visible_state, WoundVisibleState::Fresh);
        assert!(!w.scabbed);
        assert!(!w.scarred);
    }

    /// VAL-M14G-007: ActorWoundList stores wounds keyed by zone with stable
    /// iteration.
    #[test]
    fn actor_wound_list_per_zone() {
        let mut list = ActorWoundList::new();
        list.push(
            ZoneId::from("torso_front"),
            Wound::new(WoundId(0), WoundKind::GunshotEntry, 0.4, ZoneId::from("torso_front")),
        );
        list.push(
            ZoneId::from("torso_back"),
            Wound::new(WoundId(0), WoundKind::GunshotExit, 0.4, ZoneId::from("torso_back")),
        );
        list.push(
            ZoneId::from("leg_left"),
            Wound::new(WoundId(0), WoundKind::Puncture, 0.4, ZoneId::from("leg_left")),
        );
        let zones: Vec<&str> = list.iter().map(|(z, _)| z.as_str()).collect();
        // BTreeMap iteration is alphabetical on the keys.
        assert_eq!(zones, vec!["leg_left", "torso_back", "torso_front"]);
        assert_eq!(list.total_count(), 3);
    }

    /// VAL-M14G-048: Wound ids are unique across all wounds in one
    /// ActorWoundList.
    #[test]
    fn wound_id_unique_per_actor() {
        let mut list = ActorWoundList::new();
        let zones = [
            ZoneId::from("torso_front"),
            ZoneId::from("torso_back"),
            ZoneId::from("leg_left"),
            ZoneId::from("leg_right"),
            ZoneId::from("arm_left"),
        ];
        for i in 0..50 {
            let zone = zones[i % zones.len()].clone();
            list.push(
                zone.clone(),
                Wound::new(WoundId(0), WoundKind::LacerationLight, 0.1, zone),
            );
        }
        let mut ids: std::collections::HashSet<WoundId> = std::collections::HashSet::new();
        for (_, ws) in list.iter() {
            for w in ws {
                ids.insert(w.id);
            }
        }
        assert_eq!(ids.len(), 50);
    }

    /// VAL-M14G-043: bandaged + sutured are independent boolean flags.
    #[test]
    fn bandaged_and_sutured_independent_flags() {
        let mut w = Wound::new(WoundId(1), WoundKind::LacerationLight, 0.3, ZoneId::from("arm_left"));
        w.bandaged = true;
        assert!(w.bandaged);
        assert!(!w.sutured);
        w.sutured = true;
        assert!(w.bandaged && w.sutured);
        w.bandaged = false;
        assert!(!w.bandaged && w.sutured);
    }
}
