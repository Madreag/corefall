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

/// Origin (human / robot / android) for TTD lookup. Mirrors the bigger
/// origin model M17 will ship; M11 uses the three-class roll-up.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TtdOrigin {
    Human,
    Robot,
    Android,
}

impl TtdOrigin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TtdOrigin::Human => "human",
            TtdOrigin::Robot => "robot",
            TtdOrigin::Android => "android",
        }
    }
}

/// AI difficulty preset (mirrors cf-ai's `DifficultyPreset` ids).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AiDifficulty {
    Cakewalk,
    ToughCrowd,
    Veteran,
}

impl AiDifficulty {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AiDifficulty::Cakewalk => "cakewalk",
            AiDifficulty::ToughCrowd => "tough_crowd",
            AiDifficulty::Veteran => "veteran",
        }
    }
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
        // Human floors (tough_crowd baseline; cakewalk = 1.5× / veteran = 0.75×).
        for (diff, scale) in [("cakewalk", 1.5_f32), ("tough_crowd", 1.0), ("veteran", 0.75)] {
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
