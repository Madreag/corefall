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

/// **M14G** material-reaction typed wound emit candidate. Severity is
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

/// **M15** § per-material reaction definition.
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
/// ## Pixel-transformation convention (M15 kernel)
///
/// When the per-tick reaction evaluator fires this rule for an adjacent
/// pixel pair (pa, pb):
/// - the pixel matching `input_a` is rewritten to `output`,
/// - the pixel matching `input_b` is rewritten to `byproduct` when
///   `byproduct.is_some()`, otherwise the pixel keeps its `input_b`
///   material (catalyst semantics for cascades like
///   `oil + fire → fire`).
///
/// The optional `byproduct` field is M15's hook for two-output reactions
/// (e.g. `acid + iron → rust + hydrogen` — iron→rust via output, acid→H2
/// via byproduct). It is `serde(default)` so legacy single-output JSON
/// entries round-trip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialReaction {
    /// Stable id (e.g. `"rxn.corrosion.acid_iron"`).
    pub id: String,
    pub input_a: MaterialId,
    pub input_b: MaterialId,
    pub output: MaterialId,
    #[serde(default)]
    pub byproduct: Option<MaterialId>,
    pub energy_release_j: f32,
    /// Per-second rate. The CA evaluator gates per-tick triggers on a
    /// deterministic threshold derived from this rate × dt_s.
    pub rate_per_s: f32,
    #[serde(default)]
    pub auto_ignite: bool,
    #[serde(default)]
    pub min_temperature_k: Option<f32>,
    #[serde(default)]
    pub propagates: bool,
}

impl MaterialReaction {
    /// True when this reaction can fire given the local temperature and
    /// the unordered material pair `{a, b}`. Returns `false` when either
    /// the pair doesn't match or the temperature gate is unmet.
    #[must_use]
    pub fn matches(&self, a: MaterialId, b: MaterialId, temperature_k: f32) -> bool {
        let pair_ok = (self.input_a == a && self.input_b == b) || (self.input_a == b && self.input_b == a);
        if !pair_ok {
            return false;
        }
        !matches!(self.min_temperature_k, Some(min) if temperature_k < min)
    }
}

/// **M15** § the per-run reaction registry. The launch set has 30+
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

    /// Find the first reaction matching the unordered pair `{a, b}` at
    /// the given temperature. Returns `None` if no reaction fires.
    #[must_use]
    pub fn evaluate(&self, a: MaterialId, b: MaterialId, temperature_k: f32) -> Option<&MaterialReaction> {
        self.reactions.iter().find(|r| r.matches(a, b, temperature_k))
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

/// **M15** § the canonical launch reaction registry. Returns 30+ entries
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
            energy_release_j: 89_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
        },
        // 2. Phase: water + lava → steam + obsidian (instant; exothermic).
        // Water pixel → steam (gas; rises), lava pixel → obsidian (cooled solid).
        MaterialReaction {
            id: "rxn.phase.water_lava".to_string(),
            input_a: 13, // water
            input_b: 26, // lava
            output: 50,  // steam
            byproduct: Some(70), // obsidian
            energy_release_j: 2_260_000.0,
            rate_per_s: 1000.0,
            auto_ignite: false,
            min_temperature_k: Some(1373.0),
            propagates: false,
        },
        // 3. Extinguish: water (input_a) + fire (input_b) → steam (output) + air (byproduct).
        // Per spec gherkin "fire extinguished; steam spawns (rises)".
        MaterialReaction {
            id: "rxn.extinguish.water_fire".to_string(),
            input_a: 13, // water
            input_b: 65, // fire_intense
            output: 50,  // steam
            byproduct: Some(0), // air (extinguishes fire)
            energy_release_j: -2_260_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
        },
        // 4. Cascade: oil (input_a) + fire (input_b) → fire (output) + (fire stays).
        // Oil pixel becomes fire; fire pixel stays as fire so cascade continues
        // through adjacent oil. byproduct=None means input_b stays unchanged.
        MaterialReaction {
            id: "rxn.ignition.oil_fire".to_string(),
            input_a: 19, // oil
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            energy_release_j: 200_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
        },
        // 5. Wood (input_a) + fire → charcoal (output) + (fire stays).
        // Spec literal: "wood + fire → charcoal + ash (rate 0.6; burns away)".
        // The pixel transforms to charcoal; the cascade through wood is gated
        // by adjacent fire (no infinite cascade through wood).
        MaterialReaction {
            id: "rxn.ignition.wood_fire".to_string(),
            input_a: 8,  // wood
            input_b: 65, // fire
            output: 41,  // charcoal
            byproduct: None,
            energy_release_j: 180_000.0,
            rate_per_s: 0.6,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 6. Paper (input_a) + fire → ash. Fire stays.
        MaterialReaction {
            id: "rxn.ignition.paper_fire".to_string(),
            input_a: 47, // paper
            input_b: 65, // fire
            output: 40,  // ash
            byproduct: None,
            energy_release_j: 120_000.0,
            rate_per_s: 0.95,
            auto_ignite: true,
            min_temperature_k: Some(506.0),
            propagates: true,
        },
        // 7. Fabric (input_a) + fire → ash. Fire stays.
        MaterialReaction {
            id: "rxn.ignition.fabric_fire".to_string(),
            input_a: 49, // fabric
            input_b: 65, // fire
            output: 40,  // ash
            byproduct: None,
            energy_release_j: 150_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(510.0),
            propagates: true,
        },
        // 8. Cascade: fuel (input_a) + fire → fire (output) + (fire stays).
        MaterialReaction {
            id: "rxn.ignition.fuel_fire".to_string(),
            input_a: 20, // fuel
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            energy_release_j: 250_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(483.0),
            propagates: true,
        },
        // 9. Alcohol (input_a) + fire → steam (output) + (fire stays).
        // Ethanol combusts cleanly to water (steam) + CO2.
        MaterialReaction {
            id: "rxn.ignition.alcohol_fire".to_string(),
            input_a: 24, // alcohol
            input_b: 65, // fire
            output: 50,  // steam
            byproduct: None,
            energy_release_j: 137_000.0,
            rate_per_s: 0.9,
            auto_ignite: true,
            min_temperature_k: Some(638.0),
            propagates: true,
        },
        // 10. Explosion: gunpowder (input_a) + fire → smoke (output) + (fire stays).
        // Energy release is the explosive impulse; the engine layer routes
        // energy_release_j into a blast event.
        MaterialReaction {
            id: "rxn.explosion.gunpowder_fire".to_string(),
            input_a: 48, // gunpowder
            input_b: 65, // fire
            output: 62,  // smoke
            byproduct: None,
            energy_release_j: 1_850_000.0,
            rate_per_s: 5.0,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 11. Combustion: methane (input_a) + oxygen (input_b) → CO2 (output) + steam (byproduct).
        // CH4 + 2 O2 → CO2 + 2 H2O.
        MaterialReaction {
            id: "rxn.combustion.methane_o2".to_string(),
            input_a: 54, // methane
            input_b: 51, // oxygen
            output: 53,  // co2
            byproduct: Some(50), // steam
            energy_release_j: 890_400.0,
            rate_per_s: 0.9,
            auto_ignite: false,
            min_temperature_k: Some(573.0),
            propagates: false,
        },
        // 12. Combustion: H2 (input_a) + O2 (input_b) → steam (output) + air (byproduct).
        // 2 H2 + O2 → 2 H2O. Both pixels consumed.
        MaterialReaction {
            id: "rxn.combustion.h2_o2".to_string(),
            input_a: 55, // hydrogen
            input_b: 51, // oxygen
            output: 50,  // steam
            byproduct: Some(0), // air (O2 consumed; only steam remains nearby)
            energy_release_j: 483_600.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: Some(773.0),
            propagates: false,
        },
        // 13. Combustion: coal (input_a) + O2 (input_b) → ash (output) + CO2 (byproduct).
        // Coal solid pixel → ash residue; O2 pixel → CO2.
        MaterialReaction {
            id: "rxn.combustion.coal_o2".to_string(),
            input_a: 33, // coal
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            energy_release_j: 393_500.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(973.0),
            propagates: false,
        },
        // 14. Neutralization: acid (input_a) + alkali (input_b) → brine (output) + brine (byproduct).
        // Both pixels neutralize to brine.
        MaterialReaction {
            id: "rxn.neutralization.acid_alkali".to_string(),
            input_a: 21, // acid
            input_b: 22, // alkali
            output: 67,  // neutralized_brine
            byproduct: Some(67), // neutralized_brine
            energy_release_j: 57_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: Some(253.0),
            propagates: false,
        },
        // 15. Smelting: ore_iron (input_a) + fire (input_b) → iron (output); fire stays.
        MaterialReaction {
            id: "rxn.phase.iron_smelt".to_string(),
            input_a: 34, // ore_iron
            input_b: 65, // fire
            output: 68,  // iron
            byproduct: None,
            energy_release_j: -247_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(1811.0),
            propagates: false,
        },
        // 16. Smelting: ore_gold (input_a) + fire (input_b) → gold (output); fire stays.
        MaterialReaction {
            id: "rxn.phase.gold_smelt".to_string(),
            input_a: 35, // ore_gold
            input_b: 65, // fire
            output: 73,  // gold
            byproduct: None,
            energy_release_j: -63_000.0,
            rate_per_s: 0.25,
            auto_ignite: false,
            min_temperature_k: Some(1337.0),
            propagates: false,
        },
        // 17. Dissolution: salt (input_a) + water (input_b) → brine (output) + brine (byproduct).
        // Both pixels become brine (mass-conservation tracking via energy budget).
        MaterialReaction {
            id: "rxn.dissolution.salt_water".to_string(),
            input_a: 42, // salt
            input_b: 13, // water
            output: 67,  // neutralized_brine
            byproduct: Some(67),
            energy_release_j: 4_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 18. Dissolution: sugar (input_a) + water (input_b) → polluted_water (output) + polluted_water (byproduct).
        MaterialReaction {
            id: "rxn.dissolution.sugar_water".to_string(),
            input_a: 43, // sugar
            input_b: 13, // water
            output: 66,  // polluted_water (syrup proxy)
            byproduct: Some(66),
            energy_release_j: 12_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 19. Ignition: coal (input_a) + fire (input_b) → fire (output) cascade.
        // Coal needs higher temp than wood (973K vs 573K).
        MaterialReaction {
            id: "rxn.ignition.coal_fire".to_string(),
            input_a: 33, // coal
            input_b: 65, // fire
            output: 65,  // fire (cascade)
            byproduct: None,
            energy_release_j: 393_500.0,
            rate_per_s: 0.5,
            auto_ignite: true,
            min_temperature_k: Some(973.0),
            propagates: true,
        },
        // 20. Combustion: fabric (input_a) + O2 (input_b) → ash (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.fabric_o2".to_string(),
            input_a: 49, // fabric
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            energy_release_j: 150_000.0,
            rate_per_s: 0.6,
            auto_ignite: false,
            min_temperature_k: Some(510.0),
            propagates: false,
        },
        // 21. Combustion: wood (input_a) + O2 (input_b) → charcoal (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.wood_o2".to_string(),
            input_a: 8,  // wood
            input_b: 51, // oxygen
            output: 41,  // charcoal
            byproduct: Some(53), // co2
            energy_release_j: 280_000.0,
            rate_per_s: 0.55,
            auto_ignite: false,
            min_temperature_k: Some(573.0),
            propagates: false,
        },
        // 22. Corrosion: concrete (input_a) + acid (input_b) → loose_fill (output) + steam (byproduct).
        // Acid dissolves concrete to rubble + steam vent.
        MaterialReaction {
            id: "rxn.corrosion.acid_concrete".to_string(),
            input_a: 2,  // concrete
            input_b: 21, // acid
            output: 5,   // loose_fill
            byproduct: Some(50), // steam
            energy_release_j: 30_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 23. Phase: lava (input_a) + ice (input_b) → obsidian (output) + water (byproduct).
        // Lava cools to obsidian crust; ice melts to water.
        MaterialReaction {
            id: "rxn.phase.lava_ice".to_string(),
            input_a: 26, // lava
            input_b: 15, // ice
            output: 70,  // obsidian
            byproduct: Some(13), // water
            energy_release_j: -334_000.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 24. Combustion: oil (input_a) + O2 (input_b) → CO2 (output) + steam (byproduct).
        // C8H18 + 12.5 O2 → 8 CO2 + 9 H2O.
        MaterialReaction {
            id: "rxn.combustion.oil_o2".to_string(),
            input_a: 19, // oil
            input_b: 51, // oxygen
            output: 53,  // co2
            byproduct: Some(50), // steam
            energy_release_j: 5_470_000.0,
            rate_per_s: 0.8,
            auto_ignite: false,
            min_temperature_k: Some(633.0),
            propagates: false,
        },
        // 25. Combustion: charcoal (input_a) + O2 (input_b) → ash (output) + CO2 (byproduct).
        MaterialReaction {
            id: "rxn.combustion.charcoal_o2".to_string(),
            input_a: 41, // charcoal
            input_b: 51, // oxygen
            output: 40,  // ash
            byproduct: Some(53), // co2
            energy_release_j: 393_500.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(600.0),
            propagates: false,
        },
        // 26. Ignition: wood (input_a) + lava (input_b) → fire (output); lava stays.
        // Same as wood+fire but with lava as the ignition catalyst.
        MaterialReaction {
            id: "rxn.ignition.lava_wood".to_string(),
            input_a: 8,  // wood
            input_b: 26, // lava
            output: 65,  // fire
            byproduct: None,
            energy_release_j: 180_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 27. Ignition: oil (input_a) + lava (input_b) → fire (output); lava stays.
        MaterialReaction {
            id: "rxn.ignition.lava_oil".to_string(),
            input_a: 19, // oil
            input_b: 26, // lava
            output: 65,  // fire
            byproduct: None,
            energy_release_j: 250_000.0,
            rate_per_s: 1.0,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
        },
        // 28. Corrosion: ore_copper (input_a) + acid (input_b) → polluted_water (output) + ammonia (byproduct).
        // CuSO4 dissolution + SO2 toxic byproduct.
        MaterialReaction {
            id: "rxn.corrosion.acid_copper".to_string(),
            input_a: 36, // ore_copper
            input_b: 21, // acid
            output: 66,  // polluted_water
            byproduct: Some(61), // ammonia
            energy_release_j: 130_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
        },
        // 29. Chemical: chlorine (input_a) + ammonia (input_b) → smoke (output) + smoke (byproduct).
        // Toxic cloud.
        MaterialReaction {
            id: "rxn.chem.chlorine_ammonia".to_string(),
            input_a: 60, // chlorine
            input_b: 61, // ammonia
            output: 62,  // smoke
            byproduct: Some(62), // smoke
            energy_release_j: 460_000.0,
            rate_per_s: 0.3,
            auto_ignite: false,
            min_temperature_k: Some(293.0),
            propagates: false,
        },
        // 30. Ignition: gunpowder (input_a) + lava (input_b) → smoke (output)
        // explosive byproduct. Lava ignites gunpowder directly.
        MaterialReaction {
            id: "rxn.explosion.gunpowder_lava".to_string(),
            input_a: 48, // gunpowder
            input_b: 26, // lava
            output: 62,  // smoke
            byproduct: None,
            energy_release_j: 1_850_000.0,
            rate_per_s: 5.0,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 31. Precipitation: smoke (input_a; pollutant proxy) + steam (input_b) →
        // polluted_water (output) + polluted_water (byproduct).
        MaterialReaction {
            id: "rxn.precipitation.acid_rain".to_string(),
            input_a: 62, // smoke / pollutant_x
            input_b: 50, // steam
            output: 66,  // polluted_water
            byproduct: Some(66),
            energy_release_j: -22_000.0,
            rate_per_s: 0.05,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 32. Electric arc: water (input_a) + electric_arc (input_b) → polluted_water (output);
        // arc stays. Models conductive pool electrification per spec.
        MaterialReaction {
            id: "rxn.electric.water_arc".to_string(),
            input_a: 13, // water
            input_b: 63, // electric_arc
            output: 66,  // polluted_water (conductive pool proxy)
            byproduct: None,
            energy_release_j: 50_000.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
        },
    ];
    ReactionRegistry::new(raw.to_vec())
}

/// **M15** § per-tick reaction event emitted by the CA stepper.
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
    pub pos: [i32; 2],
    pub energy_release_j: f32,
    pub auto_ignite: bool,
    pub tick: u64,
}

/// **M15** § build a [`ReactionTriggeredEvent`] from a matched reaction.
#[must_use]
pub fn reaction_event(reaction: &MaterialReaction, pos: [i32; 2], tick: u64) -> ReactionTriggeredEvent {
    ReactionTriggeredEvent {
        reaction_id: reaction.id.clone(),
        material_a: reaction.input_a,
        material_b: reaction.input_b,
        output: reaction.output,
        byproduct: reaction.byproduct,
        pos,
        energy_release_j: reaction.energy_release_j,
        auto_ignite: reaction.auto_ignite,
        tick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14G-029: acid → AcidBurn, refrigerant → ChemicalBurn.
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

    /// VAL-M15-001: default reaction registry has at least 30 entries
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

    /// VAL-M15-002: acid + iron → rust reaction. Per the M15 spec gherkin
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

    /// VAL-M15-003: water + fire extinguish reaction is symmetric (a/b
    /// pair lookup works regardless of order).
    #[test]
    fn water_fire_reaction_is_symmetric() {
        let r = default_reaction_registry();
        let a = r.evaluate(13, 65, 1500.0).expect("water+fire matches");
        let b = r.evaluate(65, 13, 1500.0).expect("fire+water matches");
        assert_eq!(a.id, b.id);
        assert_eq!(a.output, 50, "steam");
    }

    /// VAL-M15-004: lava + water → steam + obsidian.
    #[test]
    fn lava_water_phase_reaction() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.phase.water_lava").expect("present");
        assert_eq!(rxn.output, 50, "steam id");
        assert_eq!(rxn.byproduct, Some(70), "obsidian id");
        assert!(rxn.min_temperature_k.unwrap() >= 1373.0);
    }

    /// VAL-M15-005: gunpowder + fire is an explosive (high energy + high rate)
    #[test]
    fn gunpowder_fire_is_explosive() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.explosion.gunpowder_fire").expect("present");
        assert!(rxn.energy_release_j >= 1_000_000.0);
        assert!(rxn.rate_per_s >= 5.0);
        assert!(rxn.propagates);
        assert!(rxn.auto_ignite);
    }

    /// VAL-M15-006: acid + alkali neutralizes (no exothermic cascade)
    #[test]
    fn acid_alkali_neutralizes() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.neutralization.acid_alkali").expect("present");
        assert!(!rxn.auto_ignite);
        assert_eq!(rxn.output, 67, "neutralized_brine id");
    }

    /// VAL-M15-007: min_temperature_k gate suppresses below-threshold matches.
    #[test]
    fn temperature_gate_blocks_under_threshold() {
        let r = default_reaction_registry();
        // h2 + o2 needs 773K. At 500K must not match.
        assert!(r.evaluate(55, 51, 500.0).is_none());
        assert!(r.evaluate(55, 51, 800.0).is_some());
    }

    /// VAL-M15-008: reactions register a stable id; round-trips via serde.
    #[test]
    fn reaction_registry_round_trips_via_serde() {
        let r = default_reaction_registry();
        let json = serde_json::to_string(&r).expect("ser");
        let back: ReactionRegistry = serde_json::from_str(&json).expect("de");
        assert_eq!(back.len(), r.len());
    }

    /// VAL-M15-009: reaction_event produces a stable event payload.
    #[test]
    fn reaction_event_constructs_payload() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.extinguish.water_fire").expect("present");
        let evt = reaction_event(rxn, [10, 20], 42);
        assert_eq!(evt.tick, 42);
        assert_eq!(evt.pos, [10, 20]);
        assert_eq!(evt.output, 50);
    }

    /// VAL-M15-010: lookup by id returns canonical entry.
    #[test]
    fn lookup_by_id_works() {
        let r = default_reaction_registry();
        assert!(r.by_id("rxn.corrosion.acid_iron").is_some());
        assert!(r.by_id("rxn.does_not_exist").is_none());
    }
}
