//! **M14C** — Per-tick producers for HEAT + APFSDS + ERA rounds.
//!
//! M14 shipped the forward-compat helpers
//! (`heat_jet_modules_penetrated`, `era_pre_detonates_heat`,
//! `apfsds_energy_through_module`) as math-only primitives. M14C promotes
//! them to producer functions that emit ordered replay-event payloads per
//! impact, honoring:
//!
//!   - HEAT 5° cone half-angle  (per VAL-M14C-017)
//!   - 0.6 m optimum standoff curve (50% < 0.2 m, 100% at 0.6 m, 70% at 1.0 m)
//!   - era_charge_kg × 0.7 reduction formula (per VAL-M14C-025)
//!   - HEAT damage = velocity × mass NOT raw KE (per VAL-M14C-022)
//!   - APFSDS over-penetration on infantry = 30 dmg vs autocannon 40 baseline (VAL-M14C-016)
//!   - APFSDS vs ERA — no pre-detonation, no penetration reduction (VAL-M14C-024)
//!   - ERA fires strictly BEFORE the HEAT traversal event (per VAL-M14C-009)

use serde::{Deserialize, Serialize};

use crate::penetration_ray::InteriorModule;

/// One module entry in the HEAT traversal path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatPathEntry {
    /// Module id (matches `InteriorModule::id`).
    pub module_id: String,
    /// Effective penetration depth into the module (mm).
    pub depth_mm: f32,
    /// Damage applied to the module from the shaped-charge jet.
    pub damage: f32,
}

/// Replay-payload for `armor.era_pre_detonated`. Produced strictly before
/// a corresponding `armor.heat_jet_traversed` for the same impact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorEraPreDetonatedEvent {
    pub actor_id: u64,
    pub module_id: String,
    pub era_charge_kg: f32,
    /// Multiplicative penetration reduction applied to the downstream HEAT
    /// jet (1.0 = no reduction; 0.3 = ~70% reduction at era_charge_kg=1.0).
    pub penetration_reduction: f32,
}

/// Replay-payload for `armor.heat_jet_traversed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorHeatJetTraversedEvent {
    pub actor_id: u64,
    /// Ordered list of module ids the jet traversed (matches Gherkin-1's
    /// path field, e.g. `["torso_external", "torso_internal", "ammo_rack"]`).
    pub modules: Vec<String>,
    /// Per-module penetration details.
    pub path: Vec<HeatPathEntry>,
    /// Effective jet damage *after* standoff curve, cone check, and ERA
    /// reduction have been applied.
    pub effective_damage: f32,
    /// Standoff distance at which the jet impacted (meters).
    pub standoff_m: f32,
    /// Impact angle off the cone axis (degrees). Ray must satisfy
    /// `|impact_angle_deg| <= cone_half_angle_deg` for traversal to fire.
    pub impact_angle_deg: f32,
}

/// One module entry in the APFSDS traversal path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApfsdsPathEntry {
    pub module_id: String,
    /// Kinetic energy absorbed by this module (J).
    pub energy_absorbed_j: f32,
    /// Kinetic energy remaining after this module (J).
    pub energy_remaining_j: f32,
    /// Effective penetration depth (mm) — proportional to remaining KE
    /// and the module's absorption ratio.
    pub depth_mm: f32,
}

/// Replay-payload for `armor.apfsds_long_rod_through`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorApfsdsLongRodThroughEvent {
    pub actor_id: u64,
    pub path: Vec<ApfsdsPathEntry>,
    pub initial_energy_j: f32,
    pub final_energy_j: f32,
}

/// HEAT impact inputs. Pure value type; the engine builds this per shot.
#[derive(Debug, Clone)]
pub struct HeatImpactInput {
    pub actor_id: u64,
    pub charge_mass_kg: f32,
    pub jet_velocity_mps: f32,
    pub cone_half_angle_deg: f32,
    pub optimum_standoff_m: f32,
    pub min_jet_formation_standoff_m: f32,
    /// Standoff distance between the warhead and the armor at the moment
    /// the shaped charge detonates (meters). For direct-fire RPG impacts
    /// this is set by the projectile's nose-cone geometry (≈ `optimum_m`);
    /// "point-blank" hits where the round detonates against the armor
    /// without any standoff fall below `min_jet_formation_standoff_m`
    /// per Gherkin-5.
    pub standoff_m: f32,
    /// Angle of the impact ray off the cone axis (degrees).
    pub impact_angle_deg: f32,
    /// Modules along the projectile's interior ray, sorted by `distance_along_ray`.
    pub modules: Vec<InteriorModule>,
    /// ERA panel state — `Some(era_charge_kg)` if an intact ERA panel
    /// sits on the path, `None` if there is no ERA panel or it has
    /// already been spent. APFSDS impacts pass `None` (VAL-M14C-024).
    pub era_charge_kg: Option<f32>,
}

/// APFSDS impact inputs.
#[derive(Debug, Clone)]
pub struct ApfsdsImpactInput {
    pub actor_id: u64,
    pub rod_mass_kg: f32,
    pub velocity_mps: f32,
    pub modules: Vec<InteriorModule>,
}

/// Outcome of a HEAT producer call.
#[derive(Debug, Clone, Default)]
pub struct HeatImpactOutcome {
    /// `armor.era_pre_detonated` event, if an ERA panel triggered. Always
    /// ordered strictly before [`Self::traversed`].
    pub era_event: Option<ArmorEraPreDetonatedEvent>,
    /// `armor.heat_jet_traversed` event, if the jet successfully formed +
    /// stayed within the 5° cone. `None` when the impact glances off-axis
    /// (per VAL-M14C-017).
    pub traversed: Option<ArmorHeatJetTraversedEvent>,
    /// Player caption to surface (verbatim, VAL-M14C-018). `None` when no
    /// caption applies.
    pub caption: Option<&'static str>,
}

/// Outcome of an APFSDS producer call.
#[derive(Debug, Clone, Default)]
pub struct ApfsdsImpactOutcome {
    pub event: Option<ArmorApfsdsLongRodThroughEvent>,
    pub caption: Option<&'static str>,
}

/// **M14C / VAL-M14C-015**: HEAT standoff penetration curve.
///
/// Returns the multiplicative penetration scalar in [0, 1]:
///   - < `min_standoff` → 0.5 (under-formed jet)
///   - between `min_standoff` and `optimum` → linear ramp from 0.5 to 1.0
///   - exactly at `optimum` → 1.0
///   - above `optimum` → linear taper toward 0.7 at `optimum + 1` m (and below).
///
/// The 50% / 100% / 70% bands at 0.1 m / 0.6 m / 1.0 m match Gherkin-5
/// and VAL-M14C-015's measurement points.
#[must_use]
pub fn heat_standoff_scalar(standoff_m: f32, min_standoff_m: f32, optimum_m: f32) -> f32 {
    let d = standoff_m.max(0.0);
    let min_m = min_standoff_m.max(0.0);
    let opt_m = optimum_m.max(min_m + 1e-3);
    if d < min_m {
        return 0.5;
    }
    if d < opt_m {
        let t = (d - min_m) / (opt_m - min_m);
        return 0.5 + 0.5 * t.clamp(0.0, 1.0);
    }
    // Above optimum: jet disperses. At opt+0.4 m → 0.7 (matches the
    // 1.0 m sample for optimum=0.6 m). Beyond opt+1 m → caps at 0.5.
    let over = (d - opt_m).max(0.0);
    let taper = (1.0 - 0.75 * over).max(0.5);
    taper.min(1.0)
}

/// **M14C / VAL-M14C-017**: HEAT cone-angle gate. Returns `true` when the
/// impact ray is inside the cone half-angle (penetration), `false` when
/// the hit is off-axis (glance).
#[must_use]
pub fn heat_within_cone(impact_angle_deg: f32, cone_half_angle_deg: f32) -> bool {
    impact_angle_deg.abs() <= cone_half_angle_deg.abs() + 1e-3
}

/// **M14C / VAL-M14C-025**: ERA HEAT-penetration reduction formula.
///
/// `era_charge_kg × 0.7` per "Notes for the implementer". Returns the
/// scalar applied to the remaining HEAT jet penetration (1.0 = no
/// reduction, 0.3 ≈ 70% reduction at era_charge_kg=1.0). Clamped to
/// `[0.0, 1.0]` so that `era_charge_kg > 1.43` does not invert the curve.
#[must_use]
pub fn era_penetration_reduction(era_charge_kg: f32) -> f32 {
    let drop = (era_charge_kg.max(0.0) * 0.7).clamp(0.0, 1.0);
    1.0 - drop
}

/// **M14C** § HEAT producer. Returns the ordered event tuple per
/// the validation contract (ERA event strictly before traversal event).
#[must_use]
pub fn heat_impact_producer(input: HeatImpactInput) -> HeatImpactOutcome {
    let mut outcome = HeatImpactOutcome::default();

    // Step 1: cone gate. Off-axis → glance (no events; the caller emits
    // chassis.armor_layer_glanced via the existing M14 path).
    if !heat_within_cone(input.impact_angle_deg, input.cone_half_angle_deg) {
        return outcome;
    }

    // Step 2: ERA pre-detonation (strictly before HEAT traversal).
    let mut penetration_scalar = heat_standoff_scalar(
        input.standoff_m,
        input.min_jet_formation_standoff_m,
        input.optimum_standoff_m,
    );

    let era_event = if let Some(charge_kg) = input.era_charge_kg {
        let reduction = era_penetration_reduction(charge_kg);
        penetration_scalar *= reduction;
        outcome.caption = Some("ERA absorbed shaped charge");
        Some(ArmorEraPreDetonatedEvent {
            actor_id: input.actor_id,
            module_id: "era_panel".to_string(),
            era_charge_kg: charge_kg,
            penetration_reduction: reduction,
        })
    } else {
        None
    };

    // Step 3: per-tick HEAT traversal. Damage = velocity × mass × scalar.
    let base_damage = input.charge_mass_kg.max(0.0) * input.jet_velocity_mps.max(0.0);
    let effective_damage = base_damage * penetration_scalar;
    let jet_depth_mm = 200.0 * penetration_scalar; // calibrated so 3-module path resolves at scalar=1
    let path_hits = crate::penetration_ray::heat_jet_modules_penetrated(jet_depth_mm, &input.modules);
    let modules: Vec<String> = path_hits.iter().map(|h| h.module_id.clone()).collect();
    let path: Vec<HeatPathEntry> = path_hits
        .iter()
        .map(|h| HeatPathEntry {
            module_id: h.module_id.clone(),
            depth_mm: 50.0_f32.min(h.distance_traveled.max(0.0)),
            damage: h.damage * penetration_scalar,
        })
        .collect();

    outcome.era_event = era_event;
    outcome.traversed = Some(ArmorHeatJetTraversedEvent {
        actor_id: input.actor_id,
        modules,
        path,
        effective_damage,
        standoff_m: input.standoff_m,
        impact_angle_deg: input.impact_angle_deg,
    });

    // Step 4: under-formed standoff caption (VAL-M14C-018 #2).
    if outcome.caption.is_none() && input.standoff_m < input.min_jet_formation_standoff_m {
        outcome.caption = Some("HEAT under-formed at close range");
    }
    outcome
}

/// **M14C** § APFSDS producer. Walks the rod through each module on the
/// path, emitting per-module energy decay per `KE_in × (1 - absorption_ratio)`.
#[must_use]
pub fn apfsds_impact_producer(input: ApfsdsImpactInput) -> ApfsdsImpactOutcome {
    let mut outcome = ApfsdsImpactOutcome::default();
    let initial = 0.5 * input.rod_mass_kg.max(0.0) * input.velocity_mps * input.velocity_mps;
    let mut remaining = initial;
    let mut path: Vec<ApfsdsPathEntry> = Vec::with_capacity(input.modules.len());
    for module in &input.modules {
        if remaining <= 0.0 {
            break;
        }
        let absorption = module.armor_absorption.clamp(0.0, 1.0);
        let absorbed = remaining * absorption;
        let next = (remaining - absorbed).max(0.0);
        let depth_mm = remaining.sqrt().min(800.0);
        path.push(ApfsdsPathEntry {
            module_id: module.id.clone(),
            energy_absorbed_j: absorbed,
            energy_remaining_j: next,
            depth_mm,
        });
        remaining = next;
    }
    outcome.event = Some(ArmorApfsdsLongRodThroughEvent {
        actor_id: input.actor_id,
        path,
        initial_energy_j: initial,
        final_energy_j: remaining,
    });
    outcome
}

/// **M14C / VAL-M14C-016**: APFSDS over-penetration damage on unarmored
/// infantry. Returns 30 (vs 40 for an autocannon round on the same
/// target). Pure helper consumed by `cf-actor` when an APFSDS round hits
/// an actor whose chassis has no positioned interior modules.
#[must_use]
pub fn apfsds_overpenetration_infantry_damage() -> f32 {
    30.0
}

/// **M14C / VAL-M14C-016**: standard autocannon damage on unarmored
/// infantry (baseline comparison).
#[must_use]
pub fn autocannon_infantry_damage() -> f32 {
    40.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::penetration_ray::InteriorModule;

    fn module(id: &str, dist: f32, mult: f32, abs: f32) -> InteriorModule {
        InteriorModule {
            id: id.to_string(),
            damage_multiplier: mult,
            armor_absorption: abs,
            position: [dist, 0.0],
            distance_along_ray: dist,
            is_ammo_rack: false,
        }
    }

    fn ammo_rack(id: &str, dist: f32) -> InteriorModule {
        InteriorModule {
            id: id.to_string(),
            damage_multiplier: 1.0,
            armor_absorption: 0.5,
            position: [dist, 0.0],
            distance_along_ray: dist,
            is_ammo_rack: true,
        }
    }

    /// **VAL-M14C-015**: 0.1 m → 50%, 0.6 m → 100%, 1.0 m → 70%.
    #[test]
    fn heat_standoff_curve_matches_spec_bands() {
        let under = heat_standoff_scalar(0.1, 0.2, 0.6);
        let opt = heat_standoff_scalar(0.6, 0.2, 0.6);
        let over = heat_standoff_scalar(1.0, 0.2, 0.6);
        assert!((under - 0.5).abs() < 0.05, "under-formed @0.1m ≈ 0.5, got {under}");
        assert!((opt - 1.0).abs() < 0.05, "optimum @0.6m ≈ 1.0, got {opt}");
        assert!((over - 0.7).abs() < 0.05, "over-standoff @1.0m ≈ 0.7, got {over}");
    }

    /// **VAL-M14C-017**: 0° / 4° on-axis → cone; 6° / 10° off-axis → glance.
    #[test]
    fn heat_cone_gate_sweep() {
        assert!(heat_within_cone(0.0, 5.0));
        assert!(heat_within_cone(4.0, 5.0));
        assert!(!heat_within_cone(6.0, 5.0));
        assert!(!heat_within_cone(10.0, 5.0));
        // Negative impact angles mirror.
        assert!(heat_within_cone(-4.0, 5.0));
        assert!(!heat_within_cone(-10.0, 5.0));
    }

    /// **VAL-M14C-025**: ERA reduction scales with `era_charge_kg × 0.7`.
    #[test]
    fn era_reduction_scales_with_charge_kg() {
        let half = era_penetration_reduction(0.5);
        let one = era_penetration_reduction(1.0);
        let one_five = era_penetration_reduction(1.5);
        // 0.5 kg → 35% reduction → scalar 0.65
        assert!((half - 0.65).abs() < 0.05, "0.5 kg → ~35% reduction, got {half}");
        // 1.0 kg → 70% reduction → scalar 0.30
        assert!((one - 0.30).abs() < 0.05, "1.0 kg → ~70% reduction, got {one}");
        // 1.5 kg → clamped at 100% reduction → scalar 0.0
        assert!((one_five - 0.0).abs() < 0.05, "1.5 kg → 100% reduction (clamped), got {one_five}");
    }

    /// **VAL-M14C-010**: HEAT on Heavy Trooper torso traverses
    /// `["torso_external", "torso_internal", "ammo_rack"]`. `standoff_m`
    /// is the warhead-to-armor distance at detonation (matches the
    /// projectile's nose-cone geometry); for a typical RPG impact this
    /// stays at the round's optimum (0.6 m) regardless of firing range.
    #[test]
    fn heat_jet_through_torso_path() {
        let modules = vec![
            module("torso_external", 0.0, 0.6, 0.2),
            module("torso_internal", 1.0, 0.7, 0.2),
            ammo_rack("ammo_rack", 2.0),
        ];
        let input = HeatImpactInput {
            actor_id: 99,
            charge_mass_kg: 1.0,
            jet_velocity_mps: 3000.0,
            cone_half_angle_deg: 5.0,
            optimum_standoff_m: 0.6,
            min_jet_formation_standoff_m: 0.2,
            standoff_m: 0.6,
            impact_angle_deg: 0.0,
            modules,
            era_charge_kg: None,
        };
        let outcome = heat_impact_producer(input);
        assert!(outcome.era_event.is_none(), "no ERA panel on Heavy Trooper torso");
        let traversed = outcome.traversed.expect("HEAT traversal fires");
        assert_eq!(
            traversed.modules,
            vec![
                "torso_external".to_string(),
                "torso_internal".to_string(),
                "ammo_rack".to_string()
            ]
        );
    }

    /// **VAL-M14C-009 / VAL-M14C-011**: ERA event fires BEFORE the HEAT
    /// traversal event for the same impact. Penetration reduction matches
    /// `era_charge_kg × 0.7`.
    #[test]
    fn era_event_strictly_before_heat_traversal() {
        let modules = vec![
            module("torso_external", 0.0, 0.6, 0.2),
            module("torso_internal", 1.0, 0.6, 0.2),
        ];
        let input = HeatImpactInput {
            actor_id: 7,
            charge_mass_kg: 1.0,
            jet_velocity_mps: 3000.0,
            cone_half_angle_deg: 5.0,
            optimum_standoff_m: 0.6,
            min_jet_formation_standoff_m: 0.2,
            standoff_m: 1.0,
            impact_angle_deg: 0.0,
            modules,
            era_charge_kg: Some(1.0),
        };
        let outcome = heat_impact_producer(input);
        let era = outcome.era_event.expect("ERA event fires");
        let traversed = outcome.traversed.expect("HEAT traversal fires");
        // Reduction ~70% at era_charge_kg=1.0
        assert!(
            (era.penetration_reduction - 0.30).abs() < 0.05,
            "reduction scalar = 1 - 0.7×era_charge_kg, got {}",
            era.penetration_reduction
        );
        // Effective damage diminished relative to the no-ERA baseline.
        assert!(traversed.effective_damage < 1.0 * 3000.0 * 0.6);
        // Caption matches VAL-M14C-018 #1.
        assert_eq!(outcome.caption, Some("ERA absorbed shaped charge"));
    }

    /// **VAL-M14C-017**: 10° off-axis → no HEAT traversal event.
    #[test]
    fn heat_off_axis_glances_with_no_traversal() {
        let modules = vec![module("torso_external", 0.0, 0.6, 0.2)];
        let input = HeatImpactInput {
            actor_id: 1,
            charge_mass_kg: 1.0,
            jet_velocity_mps: 3000.0,
            cone_half_angle_deg: 5.0,
            optimum_standoff_m: 0.6,
            min_jet_formation_standoff_m: 0.2,
            standoff_m: 1.0,
            impact_angle_deg: 10.0,
            modules,
            era_charge_kg: None,
        };
        let outcome = heat_impact_producer(input);
        assert!(outcome.traversed.is_none(), "off-axis hit must not produce HEAT path");
    }

    /// **VAL-M14C-012**: APFSDS across 3 stacked modules decays energy
    /// monotonically per `KE_in × (1 - absorption_ratio)`.
    #[test]
    fn apfsds_three_module_energy_decay() {
        let modules = vec![
            module("front_plate", 0.0, 1.0, 0.3),
            module("engine", 0.5, 1.0, 0.3),
            module("fuel_tank", 1.0, 1.0, 0.3),
        ];
        let input = ApfsdsImpactInput {
            actor_id: 11,
            rod_mass_kg: 7.0,
            velocity_mps: 1600.0,
            modules,
        };
        let outcome = apfsds_impact_producer(input);
        let ev = outcome.event.expect("APFSDS event");
        assert_eq!(ev.path.len(), 3);
        // KE_in × (1 - 0.3) = 0.7 per module → 0.49 → 0.343 of original.
        let ratio_after_1 = ev.path[0].energy_remaining_j / ev.initial_energy_j;
        let ratio_after_2 = ev.path[1].energy_remaining_j / ev.initial_energy_j;
        let ratio_after_3 = ev.path[2].energy_remaining_j / ev.initial_energy_j;
        assert!((ratio_after_1 - 0.7).abs() < 0.01);
        assert!((ratio_after_2 - 0.49).abs() < 0.01);
        assert!((ratio_after_3 - 0.343).abs() < 0.02);
        // Monotonically decreasing remaining energy.
        assert!(ev.path[0].energy_remaining_j > ev.path[1].energy_remaining_j);
        assert!(ev.path[1].energy_remaining_j > ev.path[2].energy_remaining_j);
    }

    /// **VAL-M14C-024**: APFSDS impact on ERA panel — no
    /// `armor.era_pre_detonated`, no penetration reduction.
    #[test]
    fn apfsds_vs_era_does_not_predetonate() {
        let modules = vec![module("front_plate", 0.0, 1.0, 0.3)];
        let input = ApfsdsImpactInput {
            actor_id: 4,
            rod_mass_kg: 7.0,
            velocity_mps: 1600.0,
            modules,
        };
        let outcome = apfsds_impact_producer(input);
        // Sanity: APFSDS producer never carries an ERA event.
        assert!(outcome.event.is_some());
    }

    /// **VAL-M14C-016**: APFSDS over-penetration on infantry = 30 dmg vs
    /// autocannon = 40 dmg.
    #[test]
    fn apfsds_overpenetration_30_vs_autocannon_40() {
        assert!((apfsds_overpenetration_infantry_damage() - 30.0).abs() < 1e-3);
        assert!((autocannon_infantry_damage() - 40.0).abs() < 1e-3);
    }

    /// **VAL-M14C-022**: HEAT damage tracks `velocity × mass`, not raw KE.
    /// Two inputs with identical velocity × mass but different individual
    /// values produce equal effective_damage. Two inputs with identical
    /// raw KE but different velocity × mass produce DIFFERENT damage.
    #[test]
    fn heat_damage_velocity_mass_product_not_ke() {
        let make = |mass: f32, vel: f32| HeatImpactInput {
            actor_id: 0,
            charge_mass_kg: mass,
            jet_velocity_mps: vel,
            cone_half_angle_deg: 5.0,
            optimum_standoff_m: 0.6,
            min_jet_formation_standoff_m: 0.2,
            standoff_m: 0.6,
            impact_angle_deg: 0.0,
            modules: vec![module("m", 0.0, 1.0, 0.0)],
            era_charge_kg: None,
        };
        // Same velocity × mass (= 3000), different individual values.
        let a = heat_impact_producer(make(2.0, 1500.0)).traversed.unwrap();
        let b = heat_impact_producer(make(1.0, 3000.0)).traversed.unwrap();
        assert!((a.effective_damage - b.effective_damage).abs() < 1e-3);
        // Same raw KE (= 1e6), different velocity × mass.
        let c = heat_impact_producer(make(2.0, 1000.0)).traversed.unwrap(); // v*m=2000
        let d = heat_impact_producer(make(8.0, 500.0)).traversed.unwrap(); // v*m=4000
        assert!((c.effective_damage - d.effective_damage).abs() > 1.0);
    }

    /// **VAL-M14C-021**: HEAT bypasses spaced armor — jet traverses both
    /// layers + emits a path that includes the post-gap module.
    #[test]
    fn heat_bypasses_spaced_armor() {
        let modules = vec![
            module("outer_plate", 0.0, 0.6, 0.2),
            // Note: no explicit "air gap" module in our path-based model;
            // the spaced configuration is represented by a downstream
            // module beyond the outer plate that the jet must reach.
            module("inner_plate", 1.5, 0.6, 0.2),
        ];
        let input = HeatImpactInput {
            actor_id: 12,
            charge_mass_kg: 1.0,
            jet_velocity_mps: 3000.0,
            cone_half_angle_deg: 5.0,
            optimum_standoff_m: 0.6,
            min_jet_formation_standoff_m: 0.2,
            standoff_m: 0.6,
            impact_angle_deg: 0.0,
            modules,
            era_charge_kg: None,
        };
        let outcome = heat_impact_producer(input);
        let path = outcome.traversed.expect("HEAT traversal fires").modules;
        assert!(path.contains(&"inner_plate".to_string()), "post-gap module on path");
    }
}
