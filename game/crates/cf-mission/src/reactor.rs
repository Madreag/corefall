//! M9 § Layered reactor armor + pressure state machine.
//!
//! The reactor is a static actor with three armor layers (External / Internal /
//! Core) and a pressure-state ladder (Nominal / Stressed / Critical / Venting /
//! Destroyed). Damage cascades through the layers; the pressure-state advances
//! as a function of the reactor's remaining HP percentage. The forward-compat
//! seam from M9 to M13's 15-zone × 3-layer chassis model is here: same
//! `LayerState` shape, same event schemas (`armor.layer_hp_changed` +
//! `armor.layer_destroyed`).
//!
//! Cf. M9 spec § Reactor as DR-027 command-core predecessor and § Layered
//! reactor armor — External / Internal / Core damage cascade.

use serde::{Deserialize, Serialize};

/// Pressure-state ladder per M9 spec. Order matters: Nominal → Stressed →
/// Critical → Venting → Destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PressureState {
    Nominal,
    Stressed,
    Critical,
    Venting,
    Destroyed,
}

impl PressureState {
    /// Stable wire string (canonical event payload value).
    pub fn as_str(&self) -> &'static str {
        match self {
            PressureState::Nominal => "Nominal",
            PressureState::Stressed => "Stressed",
            PressureState::Critical => "Critical",
            PressureState::Venting => "Venting",
            PressureState::Destroyed => "Destroyed",
        }
    }

    /// HUD-facing label used by the REACTOR: <state> line.
    pub fn hud_label(&self) -> &'static str {
        match self {
            PressureState::Nominal => "NOMINAL",
            PressureState::Stressed => "STRESSED",
            PressureState::Critical => "CRITICAL",
            PressureState::Venting => "VENTING",
            PressureState::Destroyed => "DESTROYED",
        }
    }
}

impl Default for PressureState {
    fn default() -> Self {
        PressureState::Nominal
    }
}

/// HP-percent → PressureState mapping per M9 spec § Reactor pressure state
/// machine. Thresholds: Nominal (hp > 75%), Stressed (50-75%), Critical
/// (25-50%), Venting (>0-25%), Destroyed (=0).
#[must_use]
pub fn pressure_state_for_hp_percent(hp_pct: f32) -> PressureState {
    if hp_pct <= 0.0 {
        PressureState::Destroyed
    } else if hp_pct <= 0.25 {
        PressureState::Venting
    } else if hp_pct <= 0.50 {
        PressureState::Critical
    } else if hp_pct <= 0.75 {
        PressureState::Stressed
    } else {
        PressureState::Nominal
    }
}

/// Three armor layers per M9 spec. Forward-compat for the M13 chassis
/// 15-zone × 3-layer model (each zone reuses these three layer kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerKind {
    External,
    Internal,
    Core,
}

impl LayerKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LayerKind::External => "External",
            LayerKind::Internal => "Internal",
            LayerKind::Core => "Core",
        }
    }
}

/// Per-layer state. Mirrors the M9 spec § `LayerState{kind, hp, max_hp, hardness}`.
/// Producer events: `armor.layer_hp_changed`, `armor.layer_critical`,
/// `armor.layer_destroyed` (schemas in `cf-replay/schemas/event/armor_*`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerState {
    pub kind: LayerKind,
    pub hp: f32,
    pub max_hp: f32,
    pub hardness: f32,
}

impl LayerState {
    pub fn new(kind: LayerKind, max_hp: f32, hardness: f32) -> Self {
        Self {
            kind,
            hp: max_hp,
            max_hp,
            hardness,
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0.0
    }

    pub fn hp_percent(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }
}

/// Per-layer HP delta surfaced by `apply_damage_cascade` so the engine can
/// emit `armor.layer_hp_changed` + `armor.layer_destroyed` events with the
/// correct from/to/breach-kind triple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorLayerHpEvent {
    pub layer: LayerKind,
    pub from: f32,
    pub to: f32,
    pub destroyed: bool,
    pub critical: bool,
}

/// Summary returned by `Reactor::apply_damage_cascade`. The engine reads each
/// of the listed transitions to drive replay event emission and HUD updates.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReactorDamageReport {
    pub hp_before: f32,
    pub hp_after: f32,
    pub hp_percent_after: f32,
    pub damage_applied: f32,
    pub layer_events: Vec<ArmorLayerHpEvent>,
    /// `Some((from, to))` when the reactor crossed a pressure-state band; `None`
    /// when the state is unchanged.
    pub pressure_state_change: Option<(PressureState, PressureState)>,
    pub now_destroyed: bool,
    /// True only on the call that flipped the reactor from alive → destroyed,
    /// so the engine emits `mission.reactor_destroyed` exactly once.
    pub triggered_destruction: bool,
}

/// Timer warning thresholds per M9 spec § Player narrative flow. Each threshold
/// is fired EXACTLY ONCE per mission run when the timer crosses it (the engine
/// tracks the last threshold seen so re-runs of the same scenario emit the
/// same sequence). Severity scales with the remaining time.
pub const TIMER_WARNING_THRESHOLDS_S: &[(u32, &str, &str)] = &[
    (30, "warning", "30 SECONDS — HOLD"),
    (15, "warning", "15 SECONDS — REACTOR STRESSED"),
    (5, "critical", "5 SECONDS — HOLD THE LINE"),
];

/// Empty at M9; DR-027 fills the field shape (generators, breakers, lines).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerGridPlaceholder {}

/// shields (M13+ + M25+), repair pads (M25+), doors (M25+), affliction
/// overlay (M16+), and environment signal (M20+). Empty struct so M25+
/// can extend without bumping the schema; round-trips as `{}` via serde.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChassisModulePlaceholder {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShieldModulePlaceholder {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairPadPlaceholder {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoorPlaceholder {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AfflictionOverlayPlaceholder {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentSignalPlaceholder {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_state_matches_thresholds() {
        assert_eq!(pressure_state_for_hp_percent(1.0), PressureState::Nominal);
        assert_eq!(pressure_state_for_hp_percent(0.80), PressureState::Nominal);
        assert_eq!(pressure_state_for_hp_percent(0.75), PressureState::Stressed);
        assert_eq!(pressure_state_for_hp_percent(0.51), PressureState::Stressed);
        assert_eq!(pressure_state_for_hp_percent(0.50), PressureState::Critical);
        assert_eq!(pressure_state_for_hp_percent(0.26), PressureState::Critical);
        assert_eq!(pressure_state_for_hp_percent(0.25), PressureState::Venting);
        assert_eq!(pressure_state_for_hp_percent(0.10), PressureState::Venting);
        assert_eq!(pressure_state_for_hp_percent(0.0), PressureState::Destroyed);
    }

    #[test]
    fn layer_state_destroyed_at_zero() {
        let mut ls = LayerState::new(LayerKind::External, 60.0, 0.9);
        assert!(!ls.is_destroyed());
        ls.hp = 0.0;
        assert!(ls.is_destroyed());
    }

    #[test]
    fn layer_state_hp_percent_clamps() {
        let mut ls = LayerState::new(LayerKind::Core, 10.0, 0.5);
        ls.hp = -1.0;
        assert!((ls.hp_percent() - 0.0).abs() < f32::EPSILON);
        ls.hp = 11.0;
        assert!((ls.hp_percent() - 1.0).abs() < f32::EPSILON);
    }
}
