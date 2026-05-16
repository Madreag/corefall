//! M9 — Reactor pressure HUD line widget.
//!
//! Spec § HUD readability + observability — the HUD reactor pressure line
//! reads "REACTOR: <STATE>" in a color tinted by the current pressure
//! state (Nominal → green / Stressed → yellow / Critical → orange /
//! Venting → red blinking / Destroyed → red fixed). Pulls from
//! observe.mission.reactor.pressure_state.

use bevy::prelude::*;

/// Tint for the pressure line text. Matches the spec's REACTOR-line
/// color grammar.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PressureTint {
    Green,
    Yellow,
    Orange,
    RedBlinking,
    RedFixed,
}

impl PressureTint {
    pub fn as_str(&self) -> &'static str {
        match self {
            PressureTint::Green => "green",
            PressureTint::Yellow => "yellow",
            PressureTint::Orange => "orange",
            PressureTint::RedBlinking => "red_blinking",
            PressureTint::RedFixed => "red_fixed",
        }
    }

    #[must_use]
    pub fn from_pressure_state(state: &str) -> Self {
        match state {
            "Nominal" => PressureTint::Green,
            "Stressed" => PressureTint::Yellow,
            "Critical" => PressureTint::Orange,
            "Venting" => PressureTint::RedBlinking,
            "Destroyed" => PressureTint::RedFixed,
            _ => PressureTint::Green,
        }
    }
}

/// Pressure-line widget state. The HUD draws `format!("REACTOR: {}",
/// label)` tinted by `tint`.
#[derive(Resource, Debug, Clone, Default)]
pub struct ReactorPressureLineState {
    pub label: String,
    pub tint: Option<PressureTint>,
}

impl ReactorPressureLineState {
    pub fn update(&mut self, pressure_state: &str) {
        self.tint = Some(PressureTint::from_pressure_state(pressure_state));
        self.label = match pressure_state {
            "Nominal" => "NOMINAL".to_string(),
            "Stressed" => "STRESSED".to_string(),
            "Critical" => "CRITICAL".to_string(),
            "Venting" => "VENTING".to_string(),
            "Destroyed" => "DESTROYED".to_string(),
            other => other.to_uppercase(),
        };
    }

    pub fn hud_line(&self) -> String {
        format!("REACTOR: {}", self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_tint_grammar() {
        assert_eq!(PressureTint::from_pressure_state("Nominal"), PressureTint::Green);
        assert_eq!(PressureTint::from_pressure_state("Stressed"), PressureTint::Yellow);
        assert_eq!(PressureTint::from_pressure_state("Critical"), PressureTint::Orange);
        assert_eq!(PressureTint::from_pressure_state("Venting"), PressureTint::RedBlinking);
        assert_eq!(PressureTint::from_pressure_state("Destroyed"), PressureTint::RedFixed);
    }

    #[test]
    fn hud_line_format() {
        let mut s = ReactorPressureLineState::default();
        s.update("Critical");
        assert_eq!(s.hud_line(), "REACTOR: CRITICAL");
        assert_eq!(s.tint, Some(PressureTint::Orange));
    }
}
