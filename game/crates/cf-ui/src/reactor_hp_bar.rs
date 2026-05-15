//! M9 — Reactor HP bar HUD widget (defended-actor zone).
//!
//! Spec § HUD readability + observability — the reactor strip shows
//! current HP + max + percent, three armor-layer pips (External / Internal /
//! Core) tinted by 5-tier integrity band, and a pressure-state color tint
//! over the whole bar. The widget reads observe.mission.reactor and
//! refreshes within 1 tick of any reactor.hp change (HUD updates AFTER
//! sim tick, never mid-tick).

use bevy::prelude::*;

/// 5-tier integrity band per M9 spec § Destructible terrain — also reused
/// for the reactor armor pip tinting so the player learns ONE damage
/// grammar.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IntegrityBand {
    Pristine,
    Scratched,
    Cracked,
    Critical,
    Destroyed,
}

impl IntegrityBand {
    /// Resolve a band from the layer's hp_percent (0..=1). Matches the
    /// terrain integrity bands so the player sees ONE color signature
    /// (`terrain.material_state_changed` events use the same enum).
    #[must_use]
    pub fn from_hp_percent(hp_pct: f32) -> Self {
        if hp_pct <= 0.0 {
            IntegrityBand::Destroyed
        } else if hp_pct < 0.25 {
            IntegrityBand::Critical
        } else if hp_pct < 0.50 {
            IntegrityBand::Cracked
        } else if hp_pct < 0.75 {
            IntegrityBand::Scratched
        } else {
            IntegrityBand::Pristine
        }
    }

    /// Stable canonical name for replay / cfctl payload.
    pub fn as_str(&self) -> &'static str {
        match self {
            IntegrityBand::Pristine => "Pristine",
            IntegrityBand::Scratched => "Scratched",
            IntegrityBand::Cracked => "Cracked",
            IntegrityBand::Critical => "Critical",
            IntegrityBand::Destroyed => "Destroyed",
        }
    }
}

/// One armor pip display (External / Internal / Core).
#[derive(Debug, Clone, PartialEq)]
pub struct ArmorPipView {
    pub kind: &'static str,
    pub hp: f32,
    pub max_hp: f32,
    pub hp_percent: f32,
    pub band: IntegrityBand,
}

/// Reactor HP bar widget resource. Reads from observe.mission.reactor.
#[derive(Resource, Debug, Clone, Default)]
pub struct ReactorHpBarState {
    pub hp: f32,
    pub max_hp: f32,
    pub hp_percent: f32,
    pub pressure_state: String,
    pub pips: Vec<ArmorPipView>,
}

impl ReactorHpBarState {
    pub fn update(&mut self, hp: f32, max_hp: f32, pressure_state: &str, pips: Vec<ArmorPipView>) {
        self.hp = hp.max(0.0);
        self.max_hp = max_hp.max(0.0);
        self.hp_percent = if self.max_hp > 0.0 {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.pressure_state = pressure_state.to_string();
        self.pips = pips;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_band_thresholds() {
        assert_eq!(IntegrityBand::from_hp_percent(1.0), IntegrityBand::Pristine);
        assert_eq!(IntegrityBand::from_hp_percent(0.80), IntegrityBand::Pristine);
        assert_eq!(IntegrityBand::from_hp_percent(0.74), IntegrityBand::Scratched);
        assert_eq!(IntegrityBand::from_hp_percent(0.50), IntegrityBand::Scratched);
        assert_eq!(IntegrityBand::from_hp_percent(0.49), IntegrityBand::Cracked);
        assert_eq!(IntegrityBand::from_hp_percent(0.25), IntegrityBand::Cracked);
        assert_eq!(IntegrityBand::from_hp_percent(0.10), IntegrityBand::Critical);
        assert_eq!(IntegrityBand::from_hp_percent(0.0), IntegrityBand::Destroyed);
    }

    #[test]
    fn reactor_hp_bar_update_clamps() {
        let mut bar = ReactorHpBarState::default();
        bar.update(
            70.0,
            100.0,
            "Stressed",
            vec![ArmorPipView {
                kind: "External",
                hp: 40.0,
                max_hp: 60.0,
                hp_percent: 0.6667,
                band: IntegrityBand::Scratched,
            }],
        );
        assert!((bar.hp_percent - 0.7).abs() < 1e-3);
        assert_eq!(bar.pressure_state, "Stressed");
        assert_eq!(bar.pips.len(), 1);
    }

    #[test]
    fn reactor_hp_bar_handles_zero_max() {
        let mut bar = ReactorHpBarState::default();
        bar.update(0.0, 0.0, "Destroyed", Vec::new());
        assert_eq!(bar.hp_percent, 0.0);
    }
}
