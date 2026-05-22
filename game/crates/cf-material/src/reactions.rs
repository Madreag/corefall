//! **M14G + M15** § Material reactions.
//!
//! ## M14G — typed wound producer
//!
//! Maps a per-tick "actor in contact with hazardous material X" sample to
//! a typed [`cf_wound::WoundKind`] record. Pure / deterministic.
//!
//! - Acid contact → `AcidBurn` (VAL-M14G-029).
//! - Refrigerant contact → `ChemicalBurn` (VAL-M14G-029).
//!
//! Other reactive materials (chlorine vapor, ammonia, etc.) reuse
//! `ChemicalBurn` until M16B ships a wider chemistry-affliction surface.
//!
//! ## M15 — per-tick reaction evaluator
//!
//! [`ReactionRegistry`] holds the 30+ launch reactions M15 cares about.
//! [`evaluate_reaction_pair`] is the canonical lookup the CA stepper uses
//! when two adjacent pixels meet: it returns the `MaterialReaction` whose
//! `(input_a, input_b)` matches (orientation-independent), gating on
//! `min_temperature_k` and reporting the per-tick rate.
//!
//! M15D is the canonical owner of the full 55-reaction matrix with
//! Arrhenius kinetics; M15 ships a CPU-only deterministic baseline that
//! M15D extends. The reaction definitions here are the M15 launch subset
//! pinned by the spec § "Material reactions (Noita Alchemy — full
//! reaction table)" enumeration.

use serde::{Deserialize, Serialize};

use cf_wound::registry::ZoneId;
use cf_wound::WoundKind;

use crate::MaterialId;

/// derived from the contact intensity (per-tick dwell × material reactivity).
#[derive(Debug, Clone, PartialEq)]
pub struct ReactionWoundEmit {
    pub kind: WoundKind,
    pub severity: f32,
    pub zone: ZoneId,
}

/// Map a material name (canonical lowercase) and contact intensity scalar
/// (0..1 over a single tick) to a typed wound. Returns `None` for materials
/// that do not feed a wound producer at M14G.
pub fn classify_reaction(material_name: &str, zone: ZoneId, intensity: f32) -> Option<ReactionWoundEmit> {
    let severity = intensity.clamp(0.0, 1.0);
    let kind = match material_name.to_ascii_lowercase().as_str() {
        "acid" => WoundKind::AcidBurn,
        "refrigerant" | "coolant" | "ammonia" | "chlorine" => WoundKind::ChemicalBurn,
        _ => return None,
    };
    Some(ReactionWoundEmit {
        kind,
        severity: severity.max(0.05),
        zone,
    })
}

///
/// Mirrors the spec literal:
/// ```ignore
/// pub struct MaterialReaction {
///   pub input_a: MaterialId,
///   pub input_b: MaterialId,
///   pub output: MaterialId,
///   pub energy_release_j: f32,
///   pub rate_per_s: f32,
///   pub auto_ignite: bool,
///   pub min_temperature_k: Option<f32>,
/// }
/// ```
///
/// ## Pixel-transformation convention (M15 kernel + M15B emissions)
///
/// When the per-tick reaction evaluator fires this rule for an adjacent
/// pixel pair (pa, pb):
///
/// 1. The pixel matching `input_a` is rewritten to `output`.
/// 2. The pixel matching `input_b` is rewritten to `byproduct` when
///    `byproduct.is_some()`, otherwise the pixel keeps its `input_b`
///    material (catalyst semantics for cascades like
///    `oil + fire → fire`).
/// 3. **M15B** § For every material id in `emissions`, the kernel
///    spawns one new pixel in an adjacent **air** cell (NESW search
///    order starting from `input_a`'s neighbors then `input_b`'s
///    neighbors). Empty `emissions` slot is dropped silently — the
///    reaction still fires, just without tertiary spawns.
///
/// `byproduct` is the two-output hook for reactions whose `input_b`
/// pixel TRANSFORMS in place (`acid + iron → rust + hydrogen` —
/// iron→rust via output, acid→H2 via byproduct). `emissions` is the
/// **N+ output** hook for reactions whose `input_b` STAYS unchanged
/// (cascade reactions) but that physically release additional gas
/// products into the surrounding air — e.g., `wood + fire → charcoal
/// (output) + (fire stays) + smoke (emission)`. This unlocks accurate
/// real-world chemistry — incomplete combustion, dissolution effervescence,
/// metal-fire hydrogen release — without breaking cascade propagation.
///
/// Determinism: the adjacent-cell search walks NESW in fixed order
/// (north, east, south, west) so per-tick output is byte-identical
/// across runs.
///
/// Backward compat: `emissions` is `serde(default)` so legacy two-
/// output JSON entries round-trip cleanly with an empty Vec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialReaction {
    pub id: String,
    pub input_a: MaterialId,
    pub input_b: MaterialId,
    pub output: MaterialId,
    #[serde(default)]
    pub byproduct: Option<MaterialId>,
    #[serde(default)]
    pub emissions: Vec<MaterialId>,
    pub energy_release_j: f32,
    pub rate_per_s: f32,
    #[serde(default)]
    pub auto_ignite: bool,
    #[serde(default)]
    pub min_temperature_k: Option<f32>,
    #[serde(default)]
    pub propagates: bool,
    /// Arrhenius-style temperature accel: effective_rate = rate_per_s
    /// * exp(activation_k * (1/min_T - 1/T)). 0.0 disables.
    #[serde(default)]
    pub activation_k: f32,
    /// Reactions involving gases get a pressure multiplier:
    /// effective_rate *= (P / ref_kpa)^pressure_order. 0.0 disables.
    #[serde(default)]
    pub pressure_order: f32,
    /// True for fast-exothermic reactions that should emit a violence
    /// burst event (sparks, color flash, sound cue) when they fire.
    #[serde(default)]
    pub violent: bool,
    /// Hex color flash (e.g. "FFAA00") for the violence burst. Default
    /// renderer fallback uses output material's color.
    #[serde(default)]
    pub flash_color_hex: Option<String>,
}

impl MaterialReaction {
    pub fn matches(&self, a: MaterialId, b: MaterialId, temperature_k: f32) -> bool {
        let pair_ok = (self.input_a == a && self.input_b == b) || (self.input_a == b && self.input_b == a);
        if !pair_ok {
            return false;
        }
        !matches!(self.min_temperature_k, Some(min) if temperature_k < min)
    }

    /// Per-tick effective rate after temperature + pressure modulation.
    /// Returns events/tick assuming 60 Hz; callers divide by tick rate
    /// if different.
    pub fn effective_rate_per_tick(&self, temperature_k: f32, pressure_kpa: f32) -> f32 {
        let mut rate = self.rate_per_s / 60.0;
        if self.activation_k > 0.0 {
            if let Some(min_t) = self.min_temperature_k {
                if min_t > 0.0 && temperature_k > 0.0 {
                    let inv = (1.0 / min_t) - (1.0 / temperature_k);
                    rate *= (self.activation_k * inv).exp();
                }
            }
        }
        if self.pressure_order > 0.0 && pressure_kpa > 0.0 {
            rate *= (pressure_kpa / 101.325).powf(self.pressure_order);
        }
        rate.max(0.0).min(1.0)
    }

    /// Deterministic firing decision: hash(reaction_id, tick, x, y) < threshold.
    pub fn fires_at(&self, tick: u64, x: i64, y: i64, temperature_k: f32, pressure_kpa: f32) -> bool {
        let threshold = self.effective_rate_per_tick(temperature_k, pressure_kpa);
        if threshold >= 1.0 {
            return true;
        }
        if threshold <= 0.0 {
            return false;
        }
        let interval = (1.0 / threshold).round().max(1.0) as u64;
        let mut h: u64 = 0xCBF29CE484222325;
        for b in self.id.as_bytes() {
            h = h.wrapping_mul(0x100000001B3).wrapping_add(*b as u64);
        }
        h = h.wrapping_mul(0x100000001B3).wrapping_add(x as u64);
        h = h.wrapping_mul(0x100000001B3).wrapping_add(y as u64);
        h ^= h >> 30;
        h = h.wrapping_mul(0xBF58476D1CE4E5B9);
        h ^= h >> 27;
        let phase = h % interval;
        tick % interval == phase
    }
}

/// entries via [`default_reaction_registry`]; M15D extends this without
/// reorganizing the schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactionRegistry {
    pub schema_version: u32,
    pub reactions: Vec<MaterialReaction>,
}

impl ReactionRegistry {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(reactions: Vec<MaterialReaction>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            reactions,
        }
    }

    /// **Perf hot-path optimization** § Find the first reaction matching
    /// the unordered pair `{a, b}` at the given temperature. Returns
    /// `None` if no reaction fires.
    ///
    /// Current implementation walks the reactions Vec linearly. For
    /// the M15 launch set (~38 reactions) this is `O(N)` per call.
    /// Callers in the kernel hot path should use
    /// [`Self::build_lookup`] + [`ReactionLookup::evaluate`] which
    /// provides `O(1)` average-case lookup via a 256×256 dense table.
    #[must_use]
    pub fn evaluate(&self, a: MaterialId, b: MaterialId, temperature_k: f32) -> Option<&MaterialReaction> {
        self.reactions.iter().find(|r| r.matches(a, b, temperature_k))
    }

    /// **Perf hot-path optimization** § Build a precomputed lookup
    /// table for `O(1)` reaction matching. Used by the M15 kernel
    /// orchestrator's per-pixel dispatch to replace the linear scan
    /// over `reactions` with a table-indexed access.
    ///
    /// The table is 65 KB (256×256 × ~1 byte chain ids) which fits
    /// comfortably in L2 cache. Build it once at scenario start and
    /// hand it to `kernel_step_with_lookup`.
    #[must_use]
    pub fn build_lookup(&self) -> ReactionLookup {
        ReactionLookup::from_registry(self)
    }

    /// Lookup by id. Stable across mod loads.
    #[must_use]
    pub fn by_id(&self, id: &str) -> Option<&MaterialReaction> {
        self.reactions.iter().find(|r| r.id == id)
    }

    pub fn len(&self) -> usize {
        self.reactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reactions.is_empty()
    }

    /// Build the set of materials that appear as `input_a` in any
    /// reaction in this registry. The CA kernel uses this to skip
    /// pixels that have no chance of being the "primary transforming"
    /// material — a critical bench optimization since most pixels in
    /// the world are inert (air, dirt, concrete) and never need a
    /// per-neighbor reaction check.
    #[must_use]
    pub fn primary_reactive_set(&self) -> std::collections::BTreeSet<MaterialId> {
        self.reactions.iter().flat_map(|r| [r.input_a, r.input_b]).collect()
    }

    /// **Perf hot-path optimization** § Build a 256-entry boolean
    /// bitmap of reactive materials. Replaces the `BTreeSet<u8>`
    /// `contains()` lookup (O(log N)) with an array index lookup
    /// (O(1)) in the per-pixel scan. The kernel dispatch goes from
    /// log(N)-per-pixel to constant-per-pixel.
    #[must_use]
    pub fn primary_reactive_bitmap(&self) -> Vec<bool> {
        let mut max_id: usize = 0;
        for r in &self.reactions {
            max_id = max_id.max(r.input_a as usize).max(r.input_b as usize);
        }
        let mut bitmap = vec![false; max_id + 1];
        for r in &self.reactions {
            bitmap[r.input_a as usize] = true;
            bitmap[r.input_b as usize] = true;
        }
        bitmap
    }
}

/// Built once via [`ReactionRegistry::build_lookup`] and reused per
/// tick. Provides O(1) average-case matching for `(input_a, input_b)`
/// pairs.
///
/// Layout: for each `(a, b)` pair in `[0, 256)²`, stores a small list
/// of reaction indices to check. Most pairs have zero or one entry;
/// only a few (e.g. water+fire) have two when temperature-gated
/// variants are present.
///
/// Memory: 256×256 = 65536 cells × ~16 bytes per cell = ~1 MB.
/// Acceptable cost for the bench/runtime path.
#[derive(Debug, Clone)]
pub struct ReactionLookup {
    table: std::collections::HashMap<(MaterialId, MaterialId), Vec<u32>>,
    reactive_bitmap: Vec<bool>,
}

impl ReactionLookup {
    fn from_registry(registry: &ReactionRegistry) -> Self {
        let mut table: std::collections::HashMap<(MaterialId, MaterialId), Vec<u32>> =
            std::collections::HashMap::with_capacity(registry.reactions.len() * 2);
        for (i, r) in registry.reactions.iter().enumerate() {
            table.entry((r.input_a, r.input_b)).or_default().push(i as u32);
            if r.input_a != r.input_b {
                table.entry((r.input_b, r.input_a)).or_default().push(i as u32);
            }
        }
        let reactive_bitmap = registry.primary_reactive_bitmap();
        Self { table, reactive_bitmap }
    }

    #[inline]
    #[must_use]
    pub fn evaluate<'r>(
        &self,
        registry: &'r ReactionRegistry,
        a: MaterialId,
        b: MaterialId,
        temperature_k: f32,
    ) -> Option<&'r MaterialReaction> {
        let bucket = self.table.get(&(a, b))?;
        for &rxn_idx in bucket {
            let r = &registry.reactions[rxn_idx as usize];
            if r.matches(a, b, temperature_k) {
                return Some(r);
            }
        }
        None
    }

    #[inline]
    #[must_use]
    pub fn is_reactive(&self, material: MaterialId) -> bool {
        let idx = material as usize;
        idx < self.reactive_bitmap.len() && self.reactive_bitmap[idx]
    }
}

impl ReactionRegistry {
    /// the canonical content-driven path: modders + tuners edit
    /// `content/materials/reaction_registry.json` (or a custom path) to
    /// add new reactions, tweak rates, change emissions — without
    /// touching the engine source.
    ///
    /// The file shape matches the [`ReactionRegistry`] struct exactly
    /// (schema_version + reactions array). All [`MaterialReaction`]
    /// fields use `#[serde(default)]` for back-compat (emissions,
    /// byproduct, auto_ignite, min_temperature_k, propagates).
    pub fn load_from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, ReactionRegistryLoadError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| {
            ReactionRegistryLoadError::Io {
                path: path_ref.to_path_buf(),
                source,
            }
        })?;
        let registry: ReactionRegistry = serde_json::from_str(&raw).map_err(|source| {
            ReactionRegistryLoadError::Parse {
                path: path_ref.to_path_buf(),
                source,
            }
        })?;
        if registry.schema_version != Self::SCHEMA_VERSION {
            return Err(ReactionRegistryLoadError::SchemaVersionMismatch {
                path: path_ref.to_path_buf(),
                expected: Self::SCHEMA_VERSION,
                actual: registry.schema_version,
            });
        }
        // Validate: no duplicate ids.
        let mut seen = std::collections::BTreeSet::new();
        for r in &registry.reactions {
            if !seen.insert(r.id.clone()) {
                return Err(ReactionRegistryLoadError::DuplicateReactionId {
                    path: path_ref.to_path_buf(),
                    id: r.id.clone(),
                });
            }
        }
        Ok(registry)
    }

    /// engine + cf-mod tools call this to find
    /// `content/materials/reaction_registry.json` regardless of cwd.
    /// Returns `None` if the file isn't present (caller falls back to
    /// [`default_reaction_registry`]).
    #[must_use]
    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/materials/reaction_registry.json"),
            std::path::PathBuf::from("../content/materials/reaction_registry.json"),
            std::path::PathBuf::from("game/content/materials/reaction_registry.json"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// path, or fall back to the hardcoded `default_reaction_registry`
    /// when the file isn't present. The default path-then-fallback
    /// pattern lets `cargo test` work without content/ on the path
    /// while runtime + modders edit the JSON.
    ///
    /// **Modder feedback**: when the JSON file IS present but fails to
    /// parse, this emits a `tracing::warn!` with the error before
    /// falling back to the hardcoded set. Without this, modders making
    /// a JSON typo would silently get hardcoded behavior with no
    /// indication their file was rejected.
    #[must_use]
    pub fn load_default_or_hardcoded() -> Self {
        if let Some(path) = Self::locate_default() {
            match Self::load_from_file(&path) {
                Ok(r) => return r,
                Err(err) => {
                    tracing::warn!(
                        target: "cf_material::reactions",
                        path = %path.display(),
                        error = ?err,
                        "reaction_registry.json present but failed to load — falling back to hardcoded defaults"
                    );
                }
            }
        }
        default_reaction_registry()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReactionRegistryLoadError {
    #[error("failed to read reaction registry at {}: {source}", path.display())]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse reaction registry at {}: {source}", path.display())]
    Parse {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error(
        "schema_version mismatch in {}: expected {expected}, got {actual}",
        path.display()
    )]
    SchemaVersionMismatch {
        path: std::path::PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error(
        "duplicate reaction id {id} in {}: every reaction must have a unique stable id",
        path.display()
    )]
    DuplicateReactionId {
        path: std::path::PathBuf,
        id: String,
    },
}

/// Convenience: lookup a reaction in the default registry by name pair.
/// `name_to_id` resolves the names to ids; missing names short-circuit
/// to `None`.
#[must_use]
pub fn evaluate_reaction_pair(
    registry: &ReactionRegistry,
    a: MaterialId,
    b: MaterialId,
    temperature_k: f32,
) -> Option<&MaterialReaction> {
    registry.evaluate(a, b, temperature_k)
}

/// spanning corrosion / combustion / phase / neutralization / explosion
/// / dissolution / ignition categories. IDs match the M15D matrix where
/// they overlap so consumers see the same canonical reaction id whether
/// they reach the simple M15 registry or the full M15D registry.
///
/// Material id references match `content/materials/material_registry.json`
/// after the M15 expansion:
/// - 0=air, 13=water, 19=oil, 20=fuel, 21=acid, 22=alkali,
///   23=blood, 24=alcohol, 26=lava, 33=coal, 34=ore_iron, 35=ore_gold,
///   38=rust, 40=ash, 41=charcoal, 48=gunpowder, 50=steam, 51=oxygen,
///   53=co2, 54=methane, 55=hydrogen, 62=smoke, 65=fire_intense,
///   67=neutralized_brine, 68=iron, 69=steel, 70=obsidian.
#[must_use]
pub fn default_reaction_registry() -> ReactionRegistry {
    let raw: &[MaterialReaction] = &[
        // 1. Corrosion: iron (input_a) + acid (input_b) → rust (output) + H2 (byproduct)
        // iron pixel transforms to rust per spec gherkin "the iron pixel transforms".
        MaterialReaction {
            id: "rxn.corrosion.acid_iron".to_string(),
            input_a: 68, // iron
            input_b: 21, // acid
            output: 38,  // rust
            byproduct: Some(55), // hydrogen
            emissions: vec![],
            energy_release_j: 89_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 2. Phase: water + lava → steam + obsidian (instant; exothermic).
        // Water pixel → steam (gas; rises), lava pixel → obsidian (cooled solid).
        MaterialReaction {
            id: "rxn.phase.water_lava".to_string(),
            input_a: 13, // water
            input_b: 26, // lava
            output: 50,  // steam
            byproduct: Some(70), // obsidian
            emissions: vec![],
            energy_release_j: 2_260_000.0,
            rate_per_s: 1000.0,
            auto_ignite: false,
            min_temperature_k: Some(1373.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 3a. Extinguish (water-gas shift): water (input_a) + fire (input_b) →
        // steam (output) + hydrogen (byproduct) at high temperature. Per real
        // chemistry: above ~973 K (carbon-burn temps), water on hot carbon
        // fuel runs the water-gas shift `C + H2O → CO + H2`, releasing
        // hydrogen alongside steam. This is WHY water on metal/magnesium
        // fires is dangerous — the released H2 is explosive. The high-temp
        // variant is listed FIRST so the registry's `find_map` returns it
        // when the temperature gate is met; falls through to 3b otherwise.
        MaterialReaction {
            id: "rxn.extinguish.water_fire_water_gas_shift".to_string(),
            input_a: 13, // water
            input_b: 65, // fire_intense
            output: 50,  // steam
            byproduct: Some(55), // hydrogen (water-gas shift release)
            emissions: vec![],
            energy_release_j: -1_900_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: Some(973.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 3b. Extinguish (standard): water (input_a) + fire (input_b) →
        // steam (output) + smoke (byproduct). Per real chemistry: the fire
        // dies cooled below ignition temp, leaving steam from the water +
        // smoke from the incomplete-combustion residue + ash particulates.
        // The byproduct is smoke (id=62), NOT air — fire never goes
        // straight to "clean air" in reality.
        MaterialReaction {
            id: "rxn.extinguish.water_fire".to_string(),
            input_a: 13, // water
            input_b: 65, // fire_intense
            output: 50,  // steam
            byproduct: Some(62), // smoke (incomplete-combustion residue)
            emissions: vec![],
            energy_release_j: -2_260_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 4. Cascade: oil (input_a) + fire (input_b) → fire (output) +
        // (fire stays) + [smoke, smoke, CO2] emissions.
        //
        // Per real chemistry: hydrocarbon combustion (C8H18 + 12.5 O2 →
        // 8 CO2 + 9 H2O) produces CO2 + water vapor + heavy black soot.
        // Oil's incomplete combustion is sootier than fuel's — hence
        // the dual smoke emission. The cascade is preserved (input_b
        // stays as fire) so adjacent oil pixels also ignite.
        MaterialReaction {
            id: "rxn.ignition.oil_fire".to_string(),
            input_a: 19, // oil
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            emissions: vec![62, 62, 53], // smoke + smoke + co2 (heavy black soot)
            energy_release_j: 200_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 5. Wood (input_a) + fire → charcoal (output) + (fire stays) +
        // [smoke, CO2] emissions.
        //
        // Per real chemistry: cellulose combustion (C6H10O5 + 6 O2 →
        // 6 CO2 + 5 H2O) produces CO2 + steam + smoke residue. The
        // wood pixel becomes charcoal (the solid residue); the fire
        // stays as fire (cascade); smoke + CO2 spawn in adjacent air.
        MaterialReaction {
            id: "rxn.ignition.wood_fire".to_string(),
            input_a: 8,  // wood
            input_b: 65, // fire
            output: 41,  // charcoal
            byproduct: None,
            emissions: vec![62, 53], // smoke + co2
            energy_release_j: 180_000.0,
            rate_per_s: 0.6,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 6. Paper (input_a) + fire → ash (output) + (fire stays) +
        // [smoke, CO2] emissions.
        //
        // Per real chemistry: cellulose combustion produces CO2 + H2O +
        // soot in addition to the ash residue. Paper-fire smoke is
        // notable — paper has lower density than wood so the soot
        // fraction is higher.
        MaterialReaction {
            id: "rxn.ignition.paper_fire".to_string(),
            input_a: 47, // paper
            input_b: 65, // fire
            output: 40,  // ash
            byproduct: None,
            emissions: vec![62, 53], // smoke + co2
            energy_release_j: 120_000.0,
            rate_per_s: 0.95,
            auto_ignite: true,
            min_temperature_k: Some(506.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 7. Fabric (input_a) + fire → ash (output) + (fire stays) +
        // [smoke, CO2] emissions. Same chemistry profile as paper
        // (organic polymer → CO2 + H2O + soot).
        MaterialReaction {
            id: "rxn.ignition.fabric_fire".to_string(),
            input_a: 49, // fabric
            input_b: 65, // fire
            output: 40,  // ash
            byproduct: None,
            emissions: vec![62, 53], // smoke + co2
            energy_release_j: 150_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(510.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 8. Cascade: fuel (input_a) + fire (input_b) → fire (output) +
        // (fire stays) + [smoke, CO2] emissions.
        //
        // Per real chemistry: fuel combustion is more complete than oil
        // (refined; less soot). Hence the single smoke emission vs
        // oil's double-smoke.
        MaterialReaction {
            id: "rxn.ignition.fuel_fire".to_string(),
            input_a: 20, // fuel
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            emissions: vec![62, 53], // smoke + co2
            energy_release_j: 250_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(483.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 9. Alcohol (input_a) + fire → steam (output) + co2 (byproduct).
        // Per real chemistry: C2H6O + 3O2 → 2CO2 + 3H2O. Ethanol
        // combusts to CO2 + water. The alcohol pixel becomes steam
        // (the water product); the fire pixel becomes CO2 (the
        // carbon-dioxide product). Both gases rise.
        MaterialReaction {
            id: "rxn.ignition.alcohol_fire".to_string(),
            input_a: 24, // alcohol
            input_b: 65, // fire
            output: 50,  // steam
            byproduct: Some(53), // co2 (combustion product)
            emissions: vec![],
            energy_release_j: 137_000.0,
            rate_per_s: 0.9,
            auto_ignite: true,
            min_temperature_k: Some(638.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 10. Explosion: gunpowder (input_a) + fire → smoke (output) +
        // (fire stays) + [smoke, smoke, CO2] emissions.
        //
        // Per real chemistry: black powder detonation (2 KNO3 + 3 C + S
        // → K2S + N2 + 3 CO2) releases massive volumes of gas — CO2 +
        // N2 + smoke + sulfur compounds. Energy release is the
        // explosive impulse; the engine layer routes energy_release_j
        // into a blast event. The emissions feed the visual smoke
        // plume that lingers after the detonation.
        MaterialReaction {
            id: "rxn.explosion.gunpowder_fire".to_string(),
            input_a: 48,
            input_b: 65,
            output: 62,
            byproduct: None,
            emissions: vec![62, 62, 53, 65, 65],
            energy_release_j: 1_850_000.0,
            rate_per_s: 60.0,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
            activation_k: 8000.0,
            pressure_order: 0.0,
            violent: true,
            flash_color_hex: Some("FFCC00".to_string()),
        },
        // 11. Combustion: methane (input_a) + oxygen (input_b) → CO2 (output) + steam (byproduct).
        // CH4 + 2 O2 → CO2 + 2 H2O.
        MaterialReaction {
            id: "rxn.combustion.methane_o2".to_string(),
            input_a: 54, // methane
            input_b: 51, // oxygen
            output: 53,  // co2
            byproduct: Some(50), // steam
            emissions: vec![],
            energy_release_j: 890_400.0,
            rate_per_s: 0.9,
            auto_ignite: false,
            min_temperature_k: Some(573.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 12. Combustion: H2 (input_a) + O2 (input_b) → steam (output) +
        // steam (byproduct). Per real chemistry: 2 H2 + O2 → 2 H2O. Both
        // reactants are fully consumed into water. The H2 pixel and the
        // O2 pixel BOTH become steam (water vapor) — the carbon-free
        // combustion has no other byproducts at all (no CO2, no soot,
        // no smoke).
        MaterialReaction {
            id: "rxn.combustion.h2_o2".to_string(),
            input_a: 55,
            input_b: 51,
            output: 50,
            byproduct: Some(50),
            emissions: vec![65],
            energy_release_j: 483_600.0,
            rate_per_s: 60.0,
            auto_ignite: false,
            min_temperature_k: Some(773.0),
            propagates: true,
            activation_k: 7000.0,
            pressure_order: 1.5,
            violent: true,
            flash_color_hex: Some("00CCFF".to_string()),
        },
        // 13. Combustion: coal (input_a) + O2 (input_b) → ash (output) +
        // CO2 (byproduct) + [smoke] emission.
        //
        // Per real chemistry: incomplete combustion of coal releases
        // smoke (soot + tar) in addition to the CO2. Even oxygen-fed
        // coal combustion has visible black smoke unless the burn is
        // perfectly stoichiometric — which it almost never is on a
        // pixel-CA timescale.
        MaterialReaction {
            id: "rxn.combustion.coal_o2".to_string(),
            input_a: 33, // coal
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            emissions: vec![62], // smoke (incomplete combustion soot)
            energy_release_j: 393_500.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(973.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 14. Neutralization: acid (input_a) + alkali (input_b) →
        // brine (output) + brine (byproduct) + [steam] emission.
        //
        // Per real chemistry: strong-acid + strong-base neutralization
        // (HCl + NaOH → NaCl + H2O) is highly exothermic (ΔH ≈ -57 kJ
        // /mol). At reactor concentrations the heat boils off some of
        // the water → steam plume above the reaction site.
        MaterialReaction {
            id: "rxn.neutralization.acid_alkali".to_string(),
            input_a: 21,
            input_b: 22,
            output: 67,
            byproduct: Some(67),
            emissions: vec![50, 50],
            energy_release_j: 57_000.0,
            rate_per_s: 30.0,
            auto_ignite: false,
            min_temperature_k: Some(253.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: true,
            flash_color_hex: Some("F0F0F0".to_string()),
        },
        // 15. Smelting: ore_iron (input_a) + fire (input_b) → iron (output); fire stays.
        MaterialReaction {
            id: "rxn.phase.iron_smelt".to_string(),
            input_a: 34, // ore_iron
            input_b: 65, // fire
            output: 68,  // iron
            byproduct: None,
            emissions: vec![],
            energy_release_j: -247_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(1811.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 16. Smelting: ore_gold (input_a) + fire (input_b) → gold (output); fire stays.
        MaterialReaction {
            id: "rxn.phase.gold_smelt".to_string(),
            input_a: 35, // ore_gold
            input_b: 65, // fire
            output: 73,  // gold
            byproduct: None,
            emissions: vec![],
            energy_release_j: -63_000.0,
            rate_per_s: 0.25,
            auto_ignite: false,
            min_temperature_k: Some(1337.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 17. Dissolution: salt (input_a) + water (input_b) → brine (output) + brine (byproduct).
        // Both pixels become brine (mass-conservation tracking via energy budget).
        MaterialReaction {
            id: "rxn.dissolution.salt_water".to_string(),
            input_a: 42, // salt
            input_b: 13, // water
            output: 67,  // neutralized_brine
            byproduct: Some(67),
            emissions: vec![],
            energy_release_j: 4_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 18. Dissolution: sugar (input_a) + water (input_b) → polluted_water (output) + polluted_water (byproduct).
        MaterialReaction {
            id: "rxn.dissolution.sugar_water".to_string(),
            input_a: 43, // sugar
            input_b: 13, // water
            output: 66,  // polluted_water (syrup proxy)
            byproduct: Some(66),
            emissions: vec![],
            energy_release_j: 12_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 19. Ignition: coal (input_a) + fire (input_b) → fire (output)
        // cascade + [smoke, smoke, CO2] emissions.
        //
        // Per real chemistry: coal combustion (C + O2 → CO2) plus the
        // characteristic "coal smoke" from incomplete burning of
        // bituminous coal (releases SO2 + soot + tar). Coal needs
        // higher ignition temp than wood (973 K vs 573 K) per the
        // existing M15 gate.
        MaterialReaction {
            id: "rxn.ignition.coal_fire".to_string(),
            input_a: 33, // coal
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            emissions: vec![62, 62, 53], // dense coal smoke + co2
            energy_release_j: 393_500.0,
            rate_per_s: 0.5,
            auto_ignite: true,
            min_temperature_k: Some(973.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 20. Combustion: fabric (input_a) + O2 (input_b) → ash (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.fabric_o2".to_string(),
            input_a: 49, // fabric
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            emissions: vec![],
            energy_release_j: 150_000.0,
            rate_per_s: 0.6,
            auto_ignite: false,
            min_temperature_k: Some(510.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 21. Combustion: wood (input_a) + O2 (input_b) → charcoal (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.wood_o2".to_string(),
            input_a: 8,  // wood
            input_b: 51, // oxygen
            output: 41,  // charcoal
            byproduct: Some(53), // co2
            emissions: vec![],
            energy_release_j: 280_000.0,
            rate_per_s: 0.55,
            auto_ignite: false,
            min_temperature_k: Some(573.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 22. Corrosion: concrete (input_a) + acid (input_b) → loose_fill (output) + co2 (byproduct).
        // Acid dissolves concrete (calcium carbonate) per real chemistry:
        // CaCO3 + 2HCl → CaCl2 + H2O + CO2. The visible "fizzing" when
        // acid hits concrete is CO2 effervescence, NOT steam.
        MaterialReaction {
            id: "rxn.corrosion.acid_concrete".to_string(),
            input_a: 2,  // concrete
            input_b: 21, // acid
            output: 5,   // loose_fill
            byproduct: Some(53), // co2 (carbonate dissolution)
            emissions: vec![],
            energy_release_j: 30_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 23. Phase: lava (input_a) + ice (input_b) → obsidian (output) + water (byproduct).
        // Lava cools to obsidian crust; ice melts to water.
        MaterialReaction {
            id: "rxn.phase.lava_ice".to_string(),
            input_a: 26, // lava
            input_b: 15, // ice
            output: 70,  // obsidian
            byproduct: Some(13), // water
            emissions: vec![],
            energy_release_j: -334_000.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 24. Combustion: oil (input_a) + O2 (input_b) → CO2 (output) + steam (byproduct).
        // C8H18 + 12.5 O2 → 8 CO2 + 9 H2O.
        MaterialReaction {
            id: "rxn.combustion.oil_o2".to_string(),
            input_a: 19,
            input_b: 51,
            output: 53,
            byproduct: Some(50),
            emissions: vec![62, 65],
            energy_release_j: 5_470_000.0,
            rate_per_s: 30.0,
            auto_ignite: true,
            min_temperature_k: Some(633.0),
            propagates: true,
            activation_k: 5000.0,
            pressure_order: 0.5,
            violent: true,
            flash_color_hex: Some("FF8800".to_string()),
        },
        // 25. Combustion: charcoal (input_a) + O2 (input_b) → ash (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.charcoal_o2".to_string(),
            input_a: 41, // charcoal
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            emissions: vec![],
            energy_release_j: 393_500.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(600.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 26. Ignition: wood (input_a) + lava (input_b) → fire (output); lava stays.
        // Same as wood+fire but with lava as the ignition catalyst.
        MaterialReaction {
            id: "rxn.ignition.lava_wood".to_string(),
            input_a: 8,  // wood
            input_b: 26, // lava
            output: 65,  // fire
            byproduct: None,
            emissions: vec![],
            energy_release_j: 180_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 27. Ignition: oil (input_a) + lava (input_b) → fire (output); lava stays.
        MaterialReaction {
            id: "rxn.ignition.lava_oil".to_string(),
            input_a: 19, // oil
            input_b: 26, // lava
            output: 65,  // fire
            byproduct: None,
            emissions: vec![],
            energy_release_j: 250_000.0,
            rate_per_s: 1.0,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 28. Corrosion: ore_copper (input_a) + acid (input_b) →
        // polluted_water (output) + hydrogen (byproduct).
        // Per real chemistry: Cu + H2SO4 → CuSO4 + H2 (sulfuric on
        // copper), or Cu + 2HCl → CuCl2 + H2 (hydrochloric on copper).
        // The dissolved metal salt becomes the conductive "polluted
        // water" pool; the byproduct is hydrogen (H2), NOT ammonia.
        // Ammonia (NH3) requires nitrogen, which is absent from this
        // reaction.
        MaterialReaction {
            id: "rxn.corrosion.acid_copper".to_string(),
            input_a: 36, // ore_copper
            input_b: 21, // acid
            output: 66,  // polluted_water (dissolved copper salt)
            byproduct: Some(55), // hydrogen (NOT ammonia per real chemistry)
            emissions: vec![],
            energy_release_j: 130_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 29. Chemical: chlorine (input_a) + ammonia (input_b) → smoke (output) + smoke (byproduct).
        // Toxic cloud.
        MaterialReaction {
            id: "rxn.chem.chlorine_ammonia".to_string(),
            input_a: 60,
            input_b: 61,
            output: 62,
            byproduct: Some(62),
            emissions: vec![62, 62],
            energy_release_j: 460_000.0,
            rate_per_s: 10.0,
            auto_ignite: false,
            min_temperature_k: Some(293.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: true,
            flash_color_hex: Some("AAEE88".to_string()),
        },
        // 30. Ignition: gunpowder (input_a) + lava (input_b) → smoke (output)
        // explosive byproduct. Lava ignites gunpowder directly.
        MaterialReaction {
            id: "rxn.explosion.gunpowder_lava".to_string(),
            input_a: 48,
            input_b: 26,
            output: 62,
            byproduct: None,
            emissions: vec![62, 62, 65],
            energy_release_j: 1_850_000.0,
            rate_per_s: 60.0,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
            activation_k: 6000.0,
            pressure_order: 0.0,
            violent: true,
            flash_color_hex: Some("FFAA00".to_string()),
        },
        // 31. Precipitation: smoke (input_a; pollutant proxy) + steam (input_b) →
        // polluted_water (output) + polluted_water (byproduct).
        MaterialReaction {
            id: "rxn.precipitation.acid_rain".to_string(),
            input_a: 62, // smoke / pollutant_x
            input_b: 50, // steam
            output: 66,  // polluted_water
            byproduct: Some(66),
            emissions: vec![],
            energy_release_j: -22_000.0,
            rate_per_s: 0.05,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // 32. Electric arc: water (input_a) + electric_arc (input_b) → polluted_water (output);
        // arc stays. Models conductive pool electrification per spec.
        MaterialReaction {
            id: "rxn.electric.water_arc".to_string(),
            input_a: 13,
            input_b: 63,
            output: 66,
            byproduct: None,
            emissions: vec![50, 64],
            energy_release_j: 50_000.0,
            rate_per_s: 30.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: true,
            flash_color_hex: Some("88DDFF".to_string()),
        },
        // fire (input_b) → steam (output) + hydrogen (byproduct) at
        // high temperature. Mirrors `rxn.extinguish.water_fire_water_gas_shift`
        // for the droplet form of water — rain on a metal/magnesium fire
        // releases the same explosive H2.
        MaterialReaction {
            id: "rxn.extinguish.rain_fire_water_gas_shift".to_string(),
            input_a: 87, // rain
            input_b: 65, // fire_intense
            output: 50,  // steam
            byproduct: Some(55), // hydrogen
            emissions: vec![],
            energy_release_j: -1_900_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: Some(973.0),
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // (input_b) → steam (output) + smoke (byproduct). Per real
        // chemistry: incomplete-combustion residue + ash escapes as
        // smoke when the fire dies. Per spec § "rain may extinguish
        // active fires per M15 reactions".
        MaterialReaction {
            id: "rxn.extinguish.rain_fire".to_string(),
            input_a: 87, // rain
            input_b: 65, // fire_intense
            output: 50,  // steam
            byproduct: Some(62), // smoke
            emissions: vec![],
            energy_release_j: -2_260_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // rust (output) + hydrogen (byproduct). Per spec § acceptance scenario 5:
        // "contact with metal_nohook triggers acid+iron→rust reaction per M15".
        // metal_nohook = id 3; reuses the canonical acid_iron output path.
        MaterialReaction {
            id: "rxn.corrosion.acid_droplet_metal_nohook".to_string(),
            input_a: 3,  // metal_nohook (the spec literal target)
            input_b: 88, // acid_droplet
            output: 38,  // rust
            byproduct: Some(55), // hydrogen
            emissions: vec![],
            energy_release_j: 89_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
        // (output) + hydrogen (byproduct). Same as acid+iron via
        // dropletized acid (the acid_droplet variant of rxn.corrosion.acid_iron).
        MaterialReaction {
            id: "rxn.corrosion.acid_droplet_iron".to_string(),
            input_a: 68, // iron
            input_b: 88, // acid_droplet
            output: 38,  // rust
            byproduct: Some(55), // hydrogen
            emissions: vec![],
            energy_release_j: 89_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
            activation_k: 0.0,
            pressure_order: 0.0,
            violent: false,
            flash_color_hex: None,
        },
    ];
    ReactionRegistry::new(raw.to_vec())
}

///
/// One of these is emitted on every reaction trigger (per spec):
/// > Events: `material.reaction_triggered { material_a, material_b, output,
/// > pos, energy_release_j, reaction_id }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionTriggeredEvent {
    pub reaction_id: String,
    pub material_a: MaterialId,
    pub material_b: MaterialId,
    pub output: MaterialId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byproduct: Option<MaterialId>,
    /// reaction fired. Mirrors [`MaterialReaction::emissions`]. Empty
    /// for reactions with no tertiary outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emissions: Vec<MaterialId>,
    /// were actually placed. Same length as `emissions`. Empty entries
    /// (e.g. `[i32::MIN, i32::MIN]`) indicate the emission was
    /// dropped because no adjacent air cell was available.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emission_positions: Vec<[i32; 2]>,
    pub pos: [i32; 2],
    pub energy_release_j: f32,
    pub auto_ignite: bool,
    pub tick: u64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub violent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flash_color_hex: Option<String>,
}

/// Note: this does NOT populate `emission_positions` — the kernel
/// orchestrator fills that field after it places the emission pixels
/// in adjacent air cells (since it knows the actual world coords).
/// Use [`reaction_event_with_emissions`] for the populated form.
#[must_use]
pub fn reaction_event(reaction: &MaterialReaction, pos: [i32; 2], tick: u64) -> ReactionTriggeredEvent {
    ReactionTriggeredEvent {
        reaction_id: reaction.id.clone(),
        material_a: reaction.input_a,
        material_b: reaction.input_b,
        output: reaction.output,
        byproduct: reaction.byproduct,
        emissions: reaction.emissions.clone(),
        emission_positions: vec![],
        pos,
        energy_release_j: reaction.energy_release_j,
        auto_ignite: reaction.auto_ignite,
        tick,
        violent: reaction.violent,
        flash_color_hex: reaction.flash_color_hex.clone(),
    }
}

#[must_use]
pub fn reaction_event_with_emissions(
    reaction: &MaterialReaction,
    pos: [i32; 2],
    tick: u64,
    emission_positions: Vec<[i32; 2]>,
) -> ReactionTriggeredEvent {
    ReactionTriggeredEvent {
        reaction_id: reaction.id.clone(),
        material_a: reaction.input_a,
        material_b: reaction.input_b,
        output: reaction.output,
        byproduct: reaction.byproduct,
        emissions: reaction.emissions.clone(),
        emission_positions,
        pos,
        energy_release_j: reaction.energy_release_j,
        auto_ignite: reaction.auto_ignite,
        tick,
        violent: reaction.violent,
        flash_color_hex: reaction.flash_color_hex.clone(),
    }
}

/// emission's adjacent-air-cell search failed. Consumers filter on
/// `pos == [EMISSION_DROPPED, EMISSION_DROPPED]` to detect dropped
/// emissions.
pub const EMISSION_DROPPED: i32 = i32::MIN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acid_and_refrigerant_wound_kinds() {
        let acid = classify_reaction("acid", ZoneId::from("hand_left"), 0.6).unwrap();
        assert_eq!(acid.kind, WoundKind::AcidBurn);
        let refrigerant = classify_reaction("refrigerant", ZoneId::from("hand_right"), 0.3).unwrap();
        assert_eq!(refrigerant.kind, WoundKind::ChemicalBurn);
    }

    #[test]
    fn unmatched_material_returns_none() {
        assert!(classify_reaction("water", ZoneId::from("foot_left"), 1.0).is_none());
        assert!(classify_reaction("dirt", ZoneId::from("foot_left"), 1.0).is_none());
    }

    /// per spec § "30+ launch reactions".
    #[test]
    fn default_registry_has_30_plus_reactions() {
        let r = default_reaction_registry();
        assert!(
            r.len() >= 30,
            "M15 spec requires 30+ reactions, got {}",
            r.len()
        );
    }

    /// "the iron pixel transforms", iron is `input_a` (transforms to
    /// `output=rust`); acid is `input_b` (becomes `byproduct=H2`).
    #[test]
    fn acid_iron_rust_reaction_matches_spec() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.corrosion.acid_iron").expect("present");
        assert_eq!(rxn.input_a, 68, "input_a must be iron — iron pixel transforms");
        assert_eq!(rxn.input_b, 21, "input_b is acid (catalyst-companion)");
        assert_eq!(rxn.output, 38, "output is rust");
        assert_eq!(rxn.byproduct, Some(55), "byproduct is hydrogen (acid → H2)");
        assert!((rxn.rate_per_s - 0.5).abs() < 1e-6);
    }

    /// pair lookup works regardless of order).
    #[test]
    fn water_fire_reaction_is_symmetric() {
        let r = default_reaction_registry();
        let a = r.evaluate(13, 65, 1500.0).expect("water+fire matches");
        let b = r.evaluate(65, 13, 1500.0).expect("fire+water matches");
        assert_eq!(a.id, b.id);
        assert_eq!(a.output, 50, "steam");
    }

    #[test]
    fn lava_water_phase_reaction() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.phase.water_lava").expect("present");
        assert_eq!(rxn.output, 50, "steam id");
        assert_eq!(rxn.byproduct, Some(70), "obsidian id");
        assert!(rxn.min_temperature_k.unwrap() >= 1373.0);
    }

    #[test]
    fn gunpowder_fire_is_explosive() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.explosion.gunpowder_fire").expect("present");
        assert!(rxn.energy_release_j >= 1_000_000.0);
        assert!(rxn.rate_per_s >= 5.0);
        assert!(rxn.propagates);
        assert!(rxn.auto_ignite);
    }

    #[test]
    fn acid_alkali_neutralizes() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.neutralization.acid_alkali").expect("present");
        assert!(!rxn.auto_ignite);
        assert_eq!(rxn.output, 67, "neutralized_brine id");
    }

    #[test]
    fn temperature_gate_blocks_under_threshold() {
        let r = default_reaction_registry();
        // h2 + o2 needs 773K. At 500K must not match.
        assert!(r.evaluate(55, 51, 500.0).is_none());
        assert!(r.evaluate(55, 51, 800.0).is_some());
    }

    #[test]
    fn reaction_registry_round_trips_via_serde() {
        let r = default_reaction_registry();
        let json = serde_json::to_string(&r).expect("ser");
        let back: ReactionRegistry = serde_json::from_str(&json).expect("de");
        assert_eq!(back.len(), r.len());
    }

    #[test]
    fn reaction_event_constructs_payload() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.extinguish.water_fire").expect("present");
        let evt = reaction_event(rxn, [10, 20], 42);
        assert_eq!(evt.tick, 42);
        assert_eq!(evt.pos, [10, 20]);
        assert_eq!(evt.output, 50);
    }

    #[test]
    fn lookup_by_id_works() {
        let r = default_reaction_registry();
        assert!(r.by_id("rxn.corrosion.acid_iron").is_some());
        assert!(r.by_id("rxn.does_not_exist").is_none());
    }
}
