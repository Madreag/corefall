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
/// The optional `byproduct` field is M15's hook for two-output reactions
/// (e.g. `oil + fire → larger_fire + smoke`). It is `serde(default)` so
/// legacy single-output JSON entries round-trip.
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
        // 1. Corrosion: acid + iron → rust (slow propagation)
        MaterialReaction {
            id: "rxn.corrosion.acid_iron".to_string(),
            input_a: 21,
            input_b: 68,
            output: 38,
            byproduct: Some(55),
            energy_release_j: 89_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
        },
        // 2. Phase: water + lava → steam + obsidian (instant, exothermic)
        MaterialReaction {
            id: "rxn.phase.water_lava".to_string(),
            input_a: 13,
            input_b: 26,
            output: 50,
            byproduct: Some(70),
            energy_release_j: 2_260_000.0,
            rate_per_s: 1000.0,
            auto_ignite: false,
            min_temperature_k: Some(1373.0),
            propagates: false,
        },
        // 3. Extinguish: water + fire → steam (and fire disappears)
        MaterialReaction {
            id: "rxn.extinguish.water_fire".to_string(),
            input_a: 13,
            input_b: 65,
            output: 50,
            byproduct: None,
            energy_release_j: -2_260_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
        },
        // 4. Cascade: oil + fire → larger fire + smoke
        MaterialReaction {
            id: "rxn.ignition.oil_fire".to_string(),
            input_a: 19,
            input_b: 65,
            output: 65,
            byproduct: Some(62),
            energy_release_j: 200_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
        },
        // 5. Cascade: wood + fire → fire + charcoal
        MaterialReaction {
            id: "rxn.ignition.wood_fire".to_string(),
            input_a: 8,
            input_b: 65,
            output: 41,
            byproduct: Some(40),
            energy_release_j: 180_000.0,
            rate_per_s: 0.6,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 6. Cascade: paper + fire → ash
        MaterialReaction {
            id: "rxn.ignition.paper_fire".to_string(),
            input_a: 47,
            input_b: 65,
            output: 40,
            byproduct: None,
            energy_release_j: 120_000.0,
            rate_per_s: 0.95,
            auto_ignite: true,
            min_temperature_k: Some(506.0),
            propagates: true,
        },
        // 7. Cascade: fabric + fire → ash
        MaterialReaction {
            id: "rxn.ignition.fabric_fire".to_string(),
            input_a: 49,
            input_b: 65,
            output: 40,
            byproduct: None,
            energy_release_j: 150_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(510.0),
            propagates: true,
        },
        // 8. Cascade: fuel + fire → larger fire + smoke
        MaterialReaction {
            id: "rxn.ignition.fuel_fire".to_string(),
            input_a: 20,
            input_b: 65,
            output: 65,
            byproduct: Some(62),
            energy_release_j: 250_000.0,
            rate_per_s: 0.85,
            auto_ignite: true,
            min_temperature_k: Some(483.0),
            propagates: true,
        },
        // 9. Cascade: alcohol + fire → fire + steam
        MaterialReaction {
            id: "rxn.ignition.alcohol_fire".to_string(),
            input_a: 24,
            input_b: 65,
            output: 65,
            byproduct: Some(50),
            energy_release_j: 137_000.0,
            rate_per_s: 0.9,
            auto_ignite: true,
            min_temperature_k: Some(638.0),
            propagates: true,
        },
        // 10. Explosion: gunpowder + fire → fire + smoke (boom)
        MaterialReaction {
            id: "rxn.explosion.gunpowder_fire".to_string(),
            input_a: 48,
            input_b: 65,
            output: 65,
            byproduct: Some(62),
            energy_release_j: 1_850_000.0,
            rate_per_s: 5.0,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 11. Combustion: methane + oxygen → CO2 (need fire/heat)
        MaterialReaction {
            id: "rxn.combustion.methane_o2".to_string(),
            input_a: 54,
            input_b: 51,
            output: 53,
            byproduct: Some(50),
            energy_release_j: 890_400.0,
            rate_per_s: 0.9,
            auto_ignite: false,
            min_temperature_k: Some(573.0),
            propagates: false,
        },
        // 12. Combustion: hydrogen + oxygen → steam (huge ΔH)
        MaterialReaction {
            id: "rxn.combustion.h2_o2".to_string(),
            input_a: 55,
            input_b: 51,
            output: 50,
            byproduct: None,
            energy_release_j: 483_600.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: Some(773.0),
            propagates: false,
        },
        // 13. Combustion: coal + oxygen → CO2 (slow)
        MaterialReaction {
            id: "rxn.combustion.coal_o2".to_string(),
            input_a: 33,
            input_b: 51,
            output: 53,
            byproduct: Some(40),
            energy_release_j: 393_500.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(973.0),
            propagates: false,
        },
        // 14. Neutralization: acid + alkali → brine
        MaterialReaction {
            id: "rxn.neutralization.acid_alkali".to_string(),
            input_a: 21,
            input_b: 22,
            output: 67,
            byproduct: None,
            energy_release_j: 57_000.0,
            rate_per_s: 5.0,
            auto_ignite: false,
            min_temperature_k: Some(253.0),
            propagates: false,
        },
        // 15. Smelting: ore_iron + heat → iron (forward-compat for M25)
        MaterialReaction {
            id: "rxn.phase.iron_smelt".to_string(),
            input_a: 34,
            input_b: 65,
            output: 68,
            byproduct: None,
            energy_release_j: -247_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(1811.0),
            propagates: false,
        },
        // 16. Smelting: ore_gold + heat → (gold uses iron id slot for now since gold not in registry)
        MaterialReaction {
            id: "rxn.phase.gold_smelt".to_string(),
            input_a: 35,
            input_b: 65,
            output: 68,
            byproduct: None,
            energy_release_j: -63_000.0,
            rate_per_s: 0.25,
            auto_ignite: false,
            min_temperature_k: Some(1337.0),
            propagates: false,
        },
        // 17. Dissolution: salt + water → brine (mass conservation)
        MaterialReaction {
            id: "rxn.dissolution.salt_water".to_string(),
            input_a: 42,
            input_b: 13,
            output: 67,
            byproduct: None,
            energy_release_j: 4_000.0,
            rate_per_s: 0.5,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 18. Dissolution: sugar + water → polluted_water (syrup analog)
        MaterialReaction {
            id: "rxn.dissolution.sugar_water".to_string(),
            input_a: 43,
            input_b: 13,
            output: 66,
            byproduct: None,
            energy_release_j: 12_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 19. Phase: ice + heat(>273) → water (handled by phase machine; reaction form for symmetry)
        MaterialReaction {
            id: "rxn.thermal.ice_melt".to_string(),
            input_a: 15,
            input_b: 65,
            output: 13,
            byproduct: None,
            energy_release_j: -334_000.0,
            rate_per_s: 0.3,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 20. Phase: steam + cold air → water (condensation; uses air id 0)
        MaterialReaction {
            id: "rxn.thermal.steam_condense".to_string(),
            input_a: 50,
            input_b: 0,
            output: 13,
            byproduct: None,
            energy_release_j: -2_260_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
        },
        // 21. Biology: blood + air → frozen_blood (M15 uses frozen_blood as cold-coagulated form)
        MaterialReaction {
            id: "rxn.bio.blood_coagulate".to_string(),
            input_a: 23,
            input_b: 0,
            output: 72,
            byproduct: None,
            energy_release_j: -2_000.0,
            rate_per_s: 0.05,
            auto_ignite: false,
            min_temperature_k: Some(253.0),
            propagates: false,
        },
        // 22. Corrosion: acid + iron rust cascade — same reaction by id, alt form for adjacent rust pixels (skip)
        // 23. Corrosion: acid + concrete → loose_fill + steam (concrete dissolves)
        MaterialReaction {
            id: "rxn.corrosion.acid_concrete".to_string(),
            input_a: 21,
            input_b: 2,
            output: 5,
            byproduct: Some(50),
            energy_release_j: 30_000.0,
            rate_per_s: 0.2,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 24. Lava cascade: lava + ice → water + obsidian
        MaterialReaction {
            id: "rxn.phase.lava_ice".to_string(),
            input_a: 26,
            input_b: 15,
            output: 13,
            byproduct: Some(70),
            energy_release_j: -334_000.0,
            rate_per_s: 1.0,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: false,
        },
        // 25. Combustion: oil + oxygen → CO2 + steam (no fire required if hot)
        MaterialReaction {
            id: "rxn.combustion.oil_o2".to_string(),
            input_a: 19,
            input_b: 51,
            output: 53,
            byproduct: Some(50),
            energy_release_j: 5_470_000.0,
            rate_per_s: 0.8,
            auto_ignite: false,
            min_temperature_k: Some(633.0),
            propagates: false,
        },
        // 26. Ignition: charcoal + oxygen → CO2 (slow burn)
        MaterialReaction {
            id: "rxn.combustion.charcoal_o2".to_string(),
            input_a: 41,
            input_b: 51,
            output: 53,
            byproduct: Some(40),
            energy_release_j: 393_500.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(600.0),
            propagates: false,
        },
        // 27. Lava + wood → fire + ash
        MaterialReaction {
            id: "rxn.ignition.lava_wood".to_string(),
            input_a: 26,
            input_b: 8,
            output: 65,
            byproduct: Some(40),
            energy_release_j: 180_000.0,
            rate_per_s: 0.8,
            auto_ignite: true,
            min_temperature_k: Some(573.0),
            propagates: true,
        },
        // 28. Lava + oil → larger fire + smoke
        MaterialReaction {
            id: "rxn.ignition.lava_oil".to_string(),
            input_a: 26,
            input_b: 19,
            output: 65,
            byproduct: Some(62),
            energy_release_j: 250_000.0,
            rate_per_s: 1.0,
            auto_ignite: true,
            min_temperature_k: Some(533.0),
            propagates: true,
        },
        // 29. Corrosion: acid + ore_copper → polluted_water + ammonia (toxic byproduct stand-in)
        MaterialReaction {
            id: "rxn.corrosion.acid_copper".to_string(),
            input_a: 21,
            input_b: 36,
            output: 66,
            byproduct: Some(61),
            energy_release_j: 130_000.0,
            rate_per_s: 0.4,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
            propagates: true,
        },
        // 30. Chlorine + ammonia → smoke (toxic cloud)
        MaterialReaction {
            id: "rxn.chem.chlorine_ammonia".to_string(),
            input_a: 60,
            input_b: 61,
            output: 62,
            byproduct: None,
            energy_release_j: 460_000.0,
            rate_per_s: 0.3,
            auto_ignite: false,
            min_temperature_k: Some(293.0),
            propagates: false,
        },
        // 31. Lava cools to obsidian when in contact with cold air
        MaterialReaction {
            id: "rxn.phase.lava_cools".to_string(),
            input_a: 26,
            input_b: 0,
            output: 70,
            byproduct: None,
            energy_release_j: -41_000.0,
            rate_per_s: 0.05,
            auto_ignite: false,
            min_temperature_k: None,
            propagates: false,
        },
        // 32. Acid rain reaction: pollutant_x (smoke as proxy) + steam → polluted_water
        MaterialReaction {
            id: "rxn.precipitation.acid_rain".to_string(),
            input_a: 62,
            input_b: 50,
            output: 66,
            byproduct: None,
            energy_release_j: -22_000.0,
            rate_per_s: 0.05,
            auto_ignite: false,
            min_temperature_k: Some(273.0),
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

    /// VAL-M15-002: acid + iron → rust reaction is present + matches the
    /// spec rate (0.5/s).
    #[test]
    fn acid_iron_rust_reaction_matches_spec() {
        let r = default_reaction_registry();
        let rxn = r.by_id("rxn.corrosion.acid_iron").expect("present");
        assert_eq!(rxn.input_a, 21, "acid id");
        assert_eq!(rxn.input_b, 68, "iron id");
        assert_eq!(rxn.output, 38, "rust id");
        assert!((rxn.rate_per_s - 0.5).abs() < 1e-6);
        // Per spec § "Acid + iron → rust + acid_residue (rate 0.5; corrosion gameplay)".
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
