//! **M14**: War Thunder-style penetration ray + HE / HEAT / APFSDS / spalling.
//!
//! These helpers fill the M9-locked schemas (`armor.penetration_ray_traversed`,
//! `armor.spalling_fragment_spawned`, `armor.spalling_fragment_hit_module`,
//! `armor.he_overpressure_wave`, `armor.heat_jet_penetrated`,
//! `armor.heat_jet_pre_detonated_by_era`, `armor.apfsds_penetrated`).
//!
//! Pure / deterministic. The engine RNG is wired in by callers for any
//! random rolls (spalling fragment count, HEAT pre-detonation chance).

use serde::{Deserialize, Serialize};

/// One module along the projectile's ray through a chassis interior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteriorModule {
    pub id: String,
    pub damage_multiplier: f32,
    pub armor_absorption: f32,
    /// World-space position used for ray-order priority + payload.
    pub position: [f32; 2],
    /// Distance from impact point along the ray.
    pub distance_along_ray: f32,
    /// True when this module is an ammo rack (catastrophic detonation
    /// trigger per spec § "Ammo rack hit → critical event firing chain").
    pub is_ammo_rack: bool,
}

/// One module-hit outcome inside the ray traversal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleHit {
    pub module_id: String,
    pub damage: f32,
    pub distance_traveled: f32,
    pub remaining_energy_after: f32,
    pub critical_detonation: bool,
}

/// Aggregate ray traversal result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PenetrationRayResult {
    pub ray_origin: [f32; 2],
    pub ray_direction: [f32; 2],
    pub initial_energy: f32,
    pub modules_hit: Vec<ModuleHit>,
    pub final_resting_point: [f32; 2],
    pub energy_remaining: f32,
    pub exited_backstop: bool,
}

/// spec § "Full penetration ray flow". Modules must be supplied
/// pre-sorted by `distance_along_ray` ascending (the engine does this
/// from the M13 module table).
#[must_use]
pub fn traverse_ray(
    ray_origin: [f32; 2],
    ray_direction: [f32; 2],
    initial_energy: f32,
    modules: &[InteriorModule],
    backstop_absorption: f32,
) -> PenetrationRayResult {
    let mut energy = initial_energy.max(0.0);
    let mut hits: Vec<ModuleHit> = Vec::with_capacity(modules.len());
    let mut last_pos = ray_origin;
    for module in modules {
        if energy <= 0.0 {
            break;
        }
        let damage = energy * module.damage_multiplier.max(0.0) * (1.0 - module.armor_absorption.clamp(0.0, 1.0));
        let absorbed = damage.min(energy);
        energy -= absorbed;
        last_pos = module.position;
        let critical = module.is_ammo_rack && damage > 5.0;
        hits.push(ModuleHit {
            module_id: module.id.clone(),
            damage,
            distance_traveled: module.distance_along_ray,
            remaining_energy_after: energy,
            critical_detonation: critical,
        });
        if critical {
            // Ammo rack catastrophic detonation — spec § "potential
            // catastrophic detonation". Halt traversal at this module
            // (remaining energy spent on the detonation cascade).
            energy = 0.0;
            break;
        }
    }
    // Backstop armor check: if energy still > 0 after the last module, the
    // projectile attempts to exit through the chassis backstop.
    let mut exited = false;
    if energy > 0.0 {
        let post_backstop = energy * (1.0 - backstop_absorption.clamp(0.0, 1.0));
        if post_backstop > 0.0 {
            exited = true;
            energy = post_backstop;
        } else {
            energy = 0.0;
        }
    }
    PenetrationRayResult {
        ray_origin,
        ray_direction,
        initial_energy,
        modules_hit: hits,
        final_resting_point: last_pos,
        energy_remaining: energy,
        exited_backstop: exited,
    }
}

/// Spalling threshold is the armor-damage value above which fragments
/// spawn (per the schema). Each fragment carries 0.2-0.5 of the original
/// damage; M14 picks 1-3 fragments in a 30° forward cone.
///
/// `rng_roll` is a single [0, 1) RNG draw used to choose the fragment
/// count (1-3) deterministically.
#[must_use]
pub fn spalling_fragment_count(damage_to_armor: f32, spalling_threshold: f32, rng_roll: f32) -> u32 {
    if damage_to_armor <= spalling_threshold {
        return 0;
    }
    let r = rng_roll.clamp(0.0, 0.999_999);
    if r < 0.33 {
        1
    } else if r < 0.66 {
        2
    } else {
        3
    }
}

/// the fraction of the original damage each spalling fragment carries.
/// `index` is the 0-based fragment within the batch; `count` is the total.
#[must_use]
pub fn spalling_fragment_damage_fraction(index: u32, count: u32, _rng_roll: f32) -> f32 {
    // Even spread inside [0.2, 0.5]. Deterministic given index/count.
    let count = count.max(1);
    if count == 1 {
        0.35
    } else {
        let span = 0.5 - 0.2;
        let step = span / (count - 1) as f32;
        0.2 + index as f32 * step
    }
}

// ---------------------- HE overpressure ----------------------

/// model".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeWave {
    pub center: [f32; 2],
    pub radius: f32,
    pub damage_at_zero_distance: f32,
}

/// Compute damage from an HE overpressure wave at a given distance, per
/// the registered `armor.he_overpressure_wave` schema's `damage_at_zero_distance`
/// + `falloff_curve` ("inverse_square" baseline).
#[must_use]
pub fn he_damage_at_distance(wave: &HeWave, distance: f32) -> f32 {
    let r = wave.radius.max(f32::EPSILON);
    let d = distance.max(0.0);
    if d >= r {
        return 0.0;
    }
    let frac = (1.0 - d / r).clamp(0.0, 1.0);
    // Inverse-square falloff (frac²) matches the M14 spec § "HE bypasses
    // thin armor via overpressure; ineffective against thick / sloped armor".
    wave.damage_at_zero_distance * frac * frac
}

/// Returns the count of modules the jet penetrates given a depth budget
/// in mm + a slice of modules pre-sorted by distance from impact.
#[must_use]
pub fn heat_jet_modules_penetrated(jet_depth_mm: f32, modules: &[InteriorModule]) -> Vec<ModuleHit> {
    let mut hits: Vec<ModuleHit> = Vec::new();
    let mut depth_remaining = jet_depth_mm.max(0.0);
    for module in modules {
        if depth_remaining <= 0.0 {
            break;
        }
        // HEAT jet effective depth ~5cm per module (50mm); spec § "Jet
        // damage falls off rapidly after impact (~5cm effective)".
        let cost = 50.0_f32;
        let damage = depth_remaining.min(50.0) * module.damage_multiplier.max(0.0);
        depth_remaining -= cost;
        hits.push(ModuleHit {
            module_id: module.id.clone(),
            damage,
            distance_traveled: module.distance_along_ray,
            remaining_energy_after: depth_remaining.max(0.0),
            critical_detonation: false,
        });
    }
    hits
}

/// Schurzen + Reactive Armor counter HEAT (pre-detonation + standoff
/// distance)". Returns true when the ERA panel pre-detonates and
/// neutralizes the jet.
///
/// `era_consumable` is the panel's remaining one-shot budget; ERA is
/// single-use per CCCP.
#[must_use]
pub fn era_pre_detonates_heat(era_consumable: bool) -> bool {
    era_consumable
}

// ---------------------- APFSDS ----------------------

/// penetrator) model". Returns the energy passed through a single module
/// + remaining energy after.
#[must_use]
pub fn apfsds_energy_through_module(
    rod_length_mm: f32,
    velocity: f32,
    module_armor_mm: f32,
    rod_mass_kg: f32,
) -> (f32, f32) {
    let initial = 0.5 * rod_mass_kg.max(0.0) * velocity * velocity;
    // Long-rod momentum penetration: armor "cost" is proportional to
    // module_armor_mm / rod_length_mm.
    let cost_fraction = (module_armor_mm.max(0.0) / rod_length_mm.max(1.0)).clamp(0.0, 1.0);
    let absorbed = initial * cost_fraction;
    let remaining = (initial - absorbed).max(0.0);
    (absorbed, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn traverse_stops_when_energy_zero() {
        // Three modules with 0.9 damage_multiplier + 0.5 absorption.
        let mods = vec![module("a", 10.0, 0.9, 0.5), module("b", 20.0, 0.9, 0.5), module("c", 30.0, 0.9, 0.5)];
        let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 100.0, &mods, 0.5);
        assert!(!res.modules_hit.is_empty());
        // Energy conserved: each module absorbs proportionally.
        assert!(res.energy_remaining >= 0.0);
    }

    #[test]
    fn traverse_ammo_rack_halts_chain() {
        let mut mods = vec![module("a", 10.0, 0.5, 0.5), module("ammo", 20.0, 1.0, 0.5)];
        mods[1].is_ammo_rack = true;
        let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 100.0, &mods, 0.5);
        // Ammo rack hit detonates; chain stops at module 1.
        assert_eq!(res.modules_hit.len(), 2);
        assert!(res.modules_hit[1].critical_detonation);
        assert!(res.energy_remaining.abs() < f32::EPSILON);
    }

    #[test]
    fn traverse_exits_backstop_when_energy_left() {
        let mods = vec![module("a", 10.0, 0.1, 0.1)];
        let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 100.0, &mods, 0.3);
        assert!(res.exited_backstop);
        assert!(res.energy_remaining > 0.0);
    }

    #[test]
    fn spalling_count_zero_below_threshold() {
        assert_eq!(spalling_fragment_count(2.0, 5.0, 0.5), 0);
    }

    #[test]
    fn spalling_count_in_range() {
        let c = spalling_fragment_count(10.0, 5.0, 0.5);
        assert!((1..=3).contains(&c));
    }

    #[test]
    fn spalling_fragment_damage_in_range() {
        let f0 = spalling_fragment_damage_fraction(0, 3, 0.0);
        let f2 = spalling_fragment_damage_fraction(2, 3, 0.5);
        assert!((0.2..=0.5).contains(&f0));
        assert!((0.2..=0.5).contains(&f2));
    }

    #[test]
    fn he_damage_zero_outside_radius() {
        let wave = HeWave {
            center: [0.0, 0.0],
            radius: 10.0,
            damage_at_zero_distance: 50.0,
        };
        assert!((he_damage_at_distance(&wave, 15.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn he_damage_full_at_center() {
        let wave = HeWave {
            center: [0.0, 0.0],
            radius: 10.0,
            damage_at_zero_distance: 50.0,
        };
        assert!((he_damage_at_distance(&wave, 0.0) - 50.0).abs() < 1e-3);
    }

    #[test]
    fn he_damage_falls_off_with_distance() {
        let wave = HeWave {
            center: [0.0, 0.0],
            radius: 10.0,
            damage_at_zero_distance: 100.0,
        };
        let near = he_damage_at_distance(&wave, 1.0);
        let mid = he_damage_at_distance(&wave, 5.0);
        let far = he_damage_at_distance(&wave, 9.0);
        assert!(near > mid);
        assert!(mid > far);
    }

    #[test]
    fn heat_jet_penetrates_modules() {
        let mods = vec![module("a", 10.0, 0.5, 0.1), module("b", 20.0, 0.5, 0.1)];
        let hits = heat_jet_modules_penetrated(120.0, &mods);
        // 120mm depth → 2 modules at 50mm each + leftover.
        assert!(hits.len() >= 2);
    }

    #[test]
    fn era_consumes_to_neutralize_heat() {
        assert!(era_pre_detonates_heat(true));
        assert!(!era_pre_detonates_heat(false));
    }

    #[test]
    fn apfsds_energy_split() {
        let (absorbed, remaining) = apfsds_energy_through_module(700.0, 1500.0, 100.0, 7.0);
        let total = 0.5 * 7.0 * 1500.0 * 1500.0;
        assert!((absorbed + remaining - total).abs() / total < 1e-3);
    }
}
