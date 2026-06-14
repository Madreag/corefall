//! **M11 / c4b4ea0**: Time-To-Death contract surface for the Triage Window
//! HUD widget.
//!
//! M17 owns the full per-affliction × per-origin × per-difficulty TTD
//! math. M11 ships a data-only interim consumer so the Triage Window can
//! render today without coupling to M17 internals. The trait surface is
//! locked here; M17 will replace the impl by swapping the
//! [`InterimTtdContract`] data file without changing the signature.

use std::collections::BTreeMap;

/// Spec § Affliction kinds used by the TTD floors interim table. A
/// canonical snake_case identifier that the .ron file and engine consumers
/// agree on. M17 extends this enum with additional kinds. Distinct from
/// [`crate::AfflictionKind`] because the M11 surface stacks "compound
/// effects" (e.g. `Bleed2W` is two wounds, not one affliction) and
/// includes terminal categories (`OxygenEmpty`) the existing enum routes
/// through separate carriers (`Asphyxiating`, `Hypoxia`).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TtdAfflictionKind {
    /// Two bleeding wounds (human).
    Bleed2W,
    /// Burning damage-over-time (any origin).
    Burning,
    /// Concussion grace period before KO.
    ConcussionGrace,
    /// Oxygen reservoir empty.
    OxygenEmpty,
    /// Acute hypoxia (atmosphere-limited).
    Hypoxia,
    /// Robot oil reservoir empty.
    OilEmpty,
    /// Robot power drain critical.
    PowerDrainCrit,
    /// Robot heat overheat band.
    HeatOverheat,
}

impl TtdAfflictionKind {
    /// snake_case identifier the .ron data file uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TtdAfflictionKind::Bleed2W => "bleed_2w",
            TtdAfflictionKind::Burning => "burning",
            TtdAfflictionKind::ConcussionGrace => "concussion_grace",
            TtdAfflictionKind::OxygenEmpty => "oxygen_empty",
            TtdAfflictionKind::Hypoxia => "hypoxia",
            TtdAfflictionKind::OilEmpty => "oil_empty",
            TtdAfflictionKind::PowerDrainCrit => "power_drain_crit",
            TtdAfflictionKind::HeatOverheat => "heat_overheat",
        }
    }
}

/// Origin for TTD lookup. The three-class roll-up (Human / Robot / Android)
/// the M11 interim ships, plus the M17 canonical extras (PoweredOrganic /
/// HeavyBiomech / MethaneBreather) used by the per-damage-type table.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TtdOrigin {
    Human,
    Robot,
    Android,
    PoweredOrganic,
    HeavyBiomech,
    MethaneBreather,
}

impl TtdOrigin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TtdOrigin::Human => "human",
            TtdOrigin::Robot => "robot",
            TtdOrigin::Android => "android",
            TtdOrigin::PoweredOrganic => "powered_organic",
            TtdOrigin::HeavyBiomech => "heavy_biomech",
            TtdOrigin::MethaneBreather => "methane_breather",
        }
    }

    /// Map a canonical origin id string onto the TTD origin class.
    #[must_use]
    pub fn from_origin_id(origin_id: &str) -> Self {
        match origin_id {
            "robot" | "synth" | "drone" | "construct" | "crystalline" => TtdOrigin::Robot,
            "android" | "hybrid" => TtdOrigin::Android,
            "powered_organic" | "powered" | "cyber_human" => TtdOrigin::PoweredOrganic,
            "heavy_biomech" | "biomech" => TtdOrigin::HeavyBiomech,
            "methane" | "methane_breather" => TtdOrigin::MethaneBreather,
            _ => TtdOrigin::Human,
        }
    }
}

/// AI difficulty preset (mirrors cf-ai's `DifficultyPreset` ids).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AiDifficulty {
    Cakewalk,
    ToughCrowd,
    Veteran,
    Hardcore,
}

impl AiDifficulty {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AiDifficulty::Cakewalk => "cakewalk",
            AiDifficulty::ToughCrowd => "tough_crowd",
            AiDifficulty::Veteran => "veteran",
            AiDifficulty::Hardcore => "hardcore",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "cakewalk" => AiDifficulty::Cakewalk,
            "veteran" => AiDifficulty::Veteran,
            "hardcore" => AiDifficulty::Hardcore,
            _ => AiDifficulty::ToughCrowd,
        }
    }

    /// System-wide TTD multiplier (spec § "Difficulty multipliers"):
    /// Cakewalk 1.5× / Tough Crowd 1.0× / Veteran 0.65× / Hardcore 0.4×.
    #[must_use]
    pub fn ttd_multiplier(self) -> f32 {
        match self {
            AiDifficulty::Cakewalk => 1.5,
            AiDifficulty::ToughCrowd => 1.0,
            AiDifficulty::Veteran => 0.65,
            AiDifficulty::Hardcore => 0.4,
        }
    }

    /// Compound-TTD floor (seconds): the p99 player-reaction guarantee.
    /// Default ≥ 8s, Veteran ≥ 5s, Hardcore none (real physics).
    #[must_use]
    pub fn compound_floor_seconds(self) -> f32 {
        match self {
            AiDifficulty::Cakewalk => 12.0,
            AiDifficulty::ToughCrowd => 8.0,
            AiDifficulty::Veteran => 5.0,
            AiDifficulty::Hardcore => 0.0,
        }
    }
}

/// Damage source categories for the M17 canonical per-damage-type × per-origin
/// TTD table (spec § "TTD budget table"). Distinct from [`TtdAfflictionKind`]:
/// these are the *wound / hazard sources* the M11 Triage Window renders.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DamageType {
    HeartWound,
    ArterialWound,
    MultiWoundTorso,
    ConcussionKo,
    VacuumExposure,
    FireTile,
    ReactorCascade,
    AcidTile,
    ElectricTile,
    AfflictionStack3,
    /// Robot-only remap target for `HeartWound` (robots have no heart;
    /// the wound routes to the primary power cable).
    PrimaryPowerCable,
}

impl DamageType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DamageType::HeartWound => "heart_wound",
            DamageType::ArterialWound => "arterial_wound",
            DamageType::MultiWoundTorso => "multi_wound_torso",
            DamageType::ConcussionKo => "concussion_ko",
            DamageType::VacuumExposure => "vacuum_exposure",
            DamageType::FireTile => "fire_tile",
            DamageType::ReactorCascade => "reactor_cascade",
            DamageType::AcidTile => "acid_tile",
            DamageType::ElectricTile => "electric_tile",
            DamageType::AfflictionStack3 => "affliction_stack_3",
            DamageType::PrimaryPowerCable => "primary_power_cable",
        }
    }

    /// Robots reject organic-only damage types; the wound is remapped onto a
    /// circuit/cable equivalent. Returns the remapped type (or `self`).
    #[must_use]
    pub fn robot_remap(self) -> Self {
        match self {
            DamageType::HeartWound | DamageType::ArterialWound => DamageType::PrimaryPowerCable,
            other => other,
        }
    }
}

/// Stack penalty for `n` simultaneous afflictions (spec § compound TTD math):
/// 1 = 1.0× / 2 = 0.85× / 3 = 0.7× / 4+ = 0.55×.
#[must_use]
pub fn stack_penalty(count: usize) -> f32 {
    match count {
        0 | 1 => 1.0,
        2 => 0.85,
        3 => 0.70,
        _ => 0.55,
    }
}

/// Canonical per-damage-type × per-origin TTD at the Tough Crowd baseline
/// (seconds). `None` = the source does not apply to this origin (robots reject
/// organic wounds; vacuum-immune origins ignore vacuum). Where the spec gives a
/// range the floor is used (the conservative p99 reaction guarantee).
#[must_use]
pub fn damage_type_ttd_base(damage: DamageType, origin: TtdOrigin) -> Option<f32> {
    use DamageType as D;
    use TtdOrigin as O;
    let v = match (damage, origin) {
        // Single heart wound (bleeding).
        (D::HeartWound, O::Human) => 90.0,
        (D::HeartWound, O::Android) => 110.0,
        (D::HeartWound, O::PoweredOrganic) => 90.0,
        (D::HeartWound, O::HeavyBiomech) => 180.0,
        (D::HeartWound, O::MethaneBreather) => 90.0,
        (D::HeartWound, O::Robot) => return None, // robots have no heart
        // Arterial wound (4× bleed).
        (D::ArterialWound, O::Human) => 25.0,
        (D::ArterialWound, O::Android) => 30.0,
        (D::ArterialWound, O::PoweredOrganic) => 25.0,
        (D::ArterialWound, O::HeavyBiomech) => 50.0,
        (D::ArterialWound, O::MethaneBreather) => 25.0,
        (D::ArterialWound, O::Robot) => return None,
        // Multi-wound torso (3+ shrapnel).
        (D::MultiWoundTorso, O::Human) => 30.0,
        (D::MultiWoundTorso, O::Android) => 35.0,
        (D::MultiWoundTorso, O::PoweredOrganic) => 30.0,
        (D::MultiWoundTorso, O::HeavyBiomech) => 50.0,
        (D::MultiWoundTorso, O::MethaneBreather) => 30.0,
        (D::MultiWoundTorso, O::Robot) => return None,
        // Concussion KO grace.
        (D::ConcussionKo, O::Robot) => return None, // internal_shock instead
        (D::ConcussionKo, _) => 8.0,
        // Vacuum exposure (helmet breach).
        (D::VacuumExposure, O::Human) => 15.0,
        (D::VacuumExposure, O::Android) => 30.0,
        (D::VacuumExposure, O::PoweredOrganic) => 30.0,
        (D::VacuumExposure, O::HeavyBiomech) => 20.0,
        (D::VacuumExposure, O::Robot) => return None, // vacuum-immune
        (D::VacuumExposure, O::MethaneBreather) => return None, // vacuum-breathable
        // Fire tile (uncapped dwell).
        (D::FireTile, O::Human) => 30.0,
        (D::FireTile, O::Android) => 45.0,
        (D::FireTile, O::PoweredOrganic) => 30.0,
        (D::FireTile, O::HeavyBiomech) => 40.0,
        (D::FireTile, O::MethaneBreather) => 90.0,
        (D::FireTile, O::Robot) => 90.0,
        // Reactor cascade explosion.
        (D::ReactorCascade, _) => 5.0,
        // Acid tile.
        (D::AcidTile, O::Human) => 60.0,
        (D::AcidTile, O::Android) => 90.0,
        (D::AcidTile, O::PoweredOrganic) => 60.0,
        (D::AcidTile, O::HeavyBiomech) => 90.0,
        (D::AcidTile, O::MethaneBreather) => 60.0,
        (D::AcidTile, O::Robot) => 30.0,
        // Electric tile.
        (D::ElectricTile, O::Human) => 30.0,
        (D::ElectricTile, O::Android) => 50.0,
        (D::ElectricTile, O::PoweredOrganic) => 30.0,
        (D::ElectricTile, O::HeavyBiomech) => 50.0,
        (D::ElectricTile, O::MethaneBreather) => 30.0,
        (D::ElectricTile, O::Robot) => 15.0,
        // Affliction stack (3+ simultaneous).
        (D::AfflictionStack3, O::Human) => 12.0,
        (D::AfflictionStack3, O::Android) => 15.0,
        (D::AfflictionStack3, O::PoweredOrganic) => 12.0,
        (D::AfflictionStack3, O::HeavyBiomech) => 18.0,
        (D::AfflictionStack3, O::MethaneBreather) => 12.0,
        (D::AfflictionStack3, O::Robot) => 8.0,
        // Robot primary power cable (heart-wound remap target).
        (D::PrimaryPowerCable, _) => 40.0,
    };
    Some(v)
}

/// Per-damage-type × per-origin × per-difficulty TTD (seconds). Robots auto-
/// remap organic wounds (heart → primary power cable). Returns `None` when the
/// source does not apply.
#[must_use]
pub fn damage_type_ttd(damage: DamageType, origin: TtdOrigin, difficulty: AiDifficulty) -> Option<f32> {
    let resolved = if matches!(origin, TtdOrigin::Robot) {
        damage.robot_remap()
    } else {
        damage
    };
    damage_type_ttd_base(resolved, origin).map(|base| base * difficulty.ttd_multiplier())
}

/// Compound TTD over a set of active per-source TTDs, applying the stack
/// penalty and the per-difficulty floor (the p99 reaction guarantee).
/// `min(individual) × stack_penalty(count)`, floored.
#[must_use]
pub fn compound_ttd_floored(individual_ttds: &[f32], difficulty: AiDifficulty) -> f32 {
    let finite: Vec<f32> = individual_ttds.iter().copied().filter(|t| t.is_finite()).collect();
    if finite.is_empty() {
        return f32::INFINITY;
    }
    let min_ttd = finite.iter().copied().fold(f32::INFINITY, f32::min);
    let raw = min_ttd * stack_penalty(finite.len());
    raw.max(difficulty.compound_floor_seconds())
}

/// Data-only TTD contract. M11 reads via this trait; M17 replaces the impl.
pub trait TtdContract {
    /// Floor TTD in seconds for one affliction × one origin × one
    /// difficulty. Returns `f32::INFINITY` when the lookup is absent (the
    /// affliction is not lethal under that origin / difficulty combo).
    fn ttd_seconds(&self, affliction: TtdAfflictionKind, origin: TtdOrigin, difficulty: AiDifficulty) -> f32;

    /// Compound TTD for an unordered stack of afflictions. Default impl is
    /// `min(individual TTDs)` with optional cross-affliction multipliers
    /// surfaced through [`Self::compound_modifier`].
    fn compound_ttd_seconds(&self, stack: &[TtdAfflictionKind], origin: TtdOrigin, difficulty: AiDifficulty) -> f32 {
        if stack.is_empty() {
            return f32::INFINITY;
        }
        let mut min_ttd = f32::INFINITY;
        for &a in stack {
            let t = self.ttd_seconds(a, origin, difficulty);
            if t < min_ttd {
                min_ttd = t;
            }
        }
        let mut modifier = 1.0_f32;
        for i in 0..stack.len() {
            for j in (i + 1)..stack.len() {
                modifier *= self.compound_modifier(stack[i], stack[j]);
            }
        }
        min_ttd * modifier
    }

    /// Multiplicative modifier when two afflictions co-exist in a stack.
    /// Default 1.0 (no modifier). Implementations override for known
    /// interactions (e.g., bleed + burning = 0.75).
    fn compound_modifier(&self, _a: TtdAfflictionKind, _b: TtdAfflictionKind) -> f32 {
        1.0
    }
}

/// per-difficulty table from `game/content/balance/ttd_floors_interim.ron`.
/// The values mirror the floors in the .ron file (kept in sync by hand;
/// M17's loader will read the .ron at scenario-load time and replace
/// `InterimTtdContract`).
#[derive(Debug, Clone)]
pub struct InterimTtdContract {
    floors: BTreeMap<(&'static str, &'static str, &'static str), f32>,
    compound: BTreeMap<(&'static str, &'static str), f32>,
}

impl InterimTtdContract {
    /// Default floors per spec § Compound TTD math contract.
    #[must_use]
    pub fn new() -> Self {
        let mut floors = BTreeMap::new();
        // Human floors (tough_crowd baseline; per M17 § difficulty multipliers
        // cakewalk = 1.5× / veteran = 0.65× / hardcore = 0.4×).
        for (diff, scale) in [
            ("cakewalk", 1.5_f32),
            ("tough_crowd", 1.0),
            ("veteran", 0.65),
            ("hardcore", 0.4),
        ] {
            floors.insert(("bleed_2w", "human", diff), 18.0 * scale);
            floors.insert(("burning", "human", diff), 32.0 * scale);
            floors.insert(("concussion_grace", "human", diff), 8.0 * scale);
            floors.insert(("oxygen_empty", "human", diff), 10.0 * scale);
            floors.insert(("hypoxia", "human", diff), 25.0 * scale);
            // robots
            floors.insert(("oil_empty", "robot", diff), 60.0 * scale);
            floors.insert(("power_drain_crit", "robot", diff), 18.0 * scale);
            floors.insert(("heat_overheat", "robot", diff), 12.0 * scale);
            floors.insert(("burning", "robot", diff), 24.0 * scale);
            // android hybrid (split-the-difference)
            floors.insert(("bleed_2w", "android", diff), 22.0 * scale);
            floors.insert(("burning", "android", diff), 28.0 * scale);
            floors.insert(("oxygen_empty", "android", diff), 14.0 * scale);
            floors.insert(("power_drain_crit", "android", diff), 22.0 * scale);
        }
        let mut compound = BTreeMap::new();
        compound.insert(("bleed_2w", "burning"), 0.75);
        compound.insert(("burning", "bleed_2w"), 0.75);
        compound.insert(("concussion_grace", "bleed_2w"), 0.6);
        compound.insert(("bleed_2w", "concussion_grace"), 0.6);
        compound.insert(("oxygen_empty", "hypoxia"), 0.5);
        compound.insert(("hypoxia", "oxygen_empty"), 0.5);
        Self { floors, compound }
    }
}

impl Default for InterimTtdContract {
    fn default() -> Self {
        Self::new()
    }
}

impl TtdContract for InterimTtdContract {
    fn ttd_seconds(&self, affliction: TtdAfflictionKind, origin: TtdOrigin, difficulty: AiDifficulty) -> f32 {
        self.floors
            .get(&(affliction.as_str(), origin.as_str(), difficulty.as_str()))
            .copied()
            .unwrap_or(f32::INFINITY)
    }

    fn compound_modifier(&self, a: TtdAfflictionKind, b: TtdAfflictionKind) -> f32 {
        self.compound.get(&(a.as_str(), b.as_str())).copied().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bleed_2w_tough_crowd_floor_is_18s() {
        let c = InterimTtdContract::new();
        let ttd = c.ttd_seconds(TtdAfflictionKind::Bleed2W, TtdOrigin::Human, AiDifficulty::ToughCrowd);
        assert!((ttd - 18.0).abs() < f32::EPSILON);
    }

    #[test]
    fn compound_bleed_and_burning_is_minimum_times_modifier() {
        let c = InterimTtdContract::new();
        let stack = [TtdAfflictionKind::Bleed2W, TtdAfflictionKind::Burning];
        let compound = c.compound_ttd_seconds(&stack, TtdOrigin::Human, AiDifficulty::ToughCrowd);
        assert!((compound - 13.5).abs() < 0.01);
    }

    #[test]
    fn empty_stack_returns_infinity() {
        let c = InterimTtdContract::new();
        let ttd = c.compound_ttd_seconds(&[], TtdOrigin::Human, AiDifficulty::ToughCrowd);
        assert!(ttd.is_infinite());
    }

    #[test]
    fn veteran_difficulty_shortens_ttd() {
        let c = InterimTtdContract::new();
        let tough = c.ttd_seconds(TtdAfflictionKind::Bleed2W, TtdOrigin::Human, AiDifficulty::ToughCrowd);
        let veteran = c.ttd_seconds(TtdAfflictionKind::Bleed2W, TtdOrigin::Human, AiDifficulty::Veteran);
        assert!(veteran < tough);
    }

    #[test]
    fn robot_oil_empty_floor_present() {
        let c = InterimTtdContract::new();
        let ttd = c.ttd_seconds(TtdAfflictionKind::OilEmpty, TtdOrigin::Robot, AiDifficulty::ToughCrowd);
        assert!((ttd - 60.0).abs() < f32::EPSILON);
    }
}
