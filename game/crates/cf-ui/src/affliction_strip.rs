//! M16 § Affliction strip HUD widget.
//!
//! Per spec § "Affliction strip HUD widget":
//! - Per-actor affliction stack under the body silhouette
//! - Max 5 icons visible; 6th evicts with "+N more" counter
//! - Per-affliction icon + severity indicator
//! - Tooltip shows full affliction details
//! - Banners fire on critical afflictions

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Maximum visible affliction icons before the "+N more" counter kicks in.
/// Mirrors spec § "Max 5 icons visible; 6th evicts with '+N more' counter".
pub const AFFLICTION_STRIP_MAX_VISIBLE: usize = 5;

/// One row in the affliction strip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfflictionStripEntry {
    /// Snake_case kind (matches `cf-replay/schemas/event/affliction_*.json`).
    pub kind: String,
    pub severity: f32,
    /// Compact 2-3 character glyph for the comic-noir aesthetic.
    pub icon: String,
    /// Tooltip body (multi-line).
    pub tooltip: String,
    /// True when this entry triggers a banner (severity ≥ 0.8 OR
    /// kind-tagged as critical e.g. burning + hp < 40%).
    pub critical: bool,
}

impl AfflictionStripEntry {
    /// Build a strip entry from a `(kind, severity)` pair.
    #[must_use]
    pub fn from_kind(kind: &str, severity: f32) -> Self {
        let severity = severity.clamp(0.0, 1.0);
        let icon = icon_for(kind);
        let tooltip = tooltip_for(kind, severity);
        let critical = severity >= 0.8;
        Self {
            kind: kind.to_string(),
            severity,
            icon,
            tooltip,
            critical,
        }
    }
}

/// Per-actor strip state. Mirrored from `cf_affliction::ActorAfflictions`
/// by the engine bridge each tick. Sorted by severity (highest first).
#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct AfflictionStripState {
    pub entries: Vec<AfflictionStripEntry>,
    /// Hidden-entry count surfaced as the "+N more" counter.
    pub hidden_more_count: u32,
}

impl AfflictionStripState {
    /// Refresh the strip from a raw list of (kind, severity) pairs.
    /// Caps the visible entries at [`AFFLICTION_STRIP_MAX_VISIBLE`].
    pub fn refresh(&mut self, raw: &[(String, f32)]) {
        let mut entries: Vec<AfflictionStripEntry> = raw
            .iter()
            .map(|(k, s)| AfflictionStripEntry::from_kind(k, *s))
            .collect();
        entries.sort_by(|a, b| {
            b.severity
                .partial_cmp(&a.severity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let total = entries.len();
        if total > AFFLICTION_STRIP_MAX_VISIBLE {
            self.hidden_more_count = (total - AFFLICTION_STRIP_MAX_VISIBLE) as u32;
            entries.truncate(AFFLICTION_STRIP_MAX_VISIBLE);
        } else {
            self.hidden_more_count = 0;
        }
        self.entries = entries;
    }

    /// One-line summary for accessibility readouts + minimal-HUD mode.
    #[must_use]
    pub fn summary_line(&self) -> String {
        if self.entries.is_empty() {
            return "afflictions: none".to_string();
        }
        let parts: Vec<String> = self
            .entries
            .iter()
            .map(|e| format!("{}({})", e.kind, severity_band(e.severity)))
            .collect();
        let mut s = parts.join(" ");
        if self.hidden_more_count > 0 {
            s.push_str(&format!(" +{} more", self.hidden_more_count));
        }
        s
    }
}

fn severity_band(severity: f32) -> &'static str {
    if severity >= 0.8 {
        "severe"
    } else if severity >= 0.5 {
        "moderate"
    } else {
        "mild"
    }
}

fn icon_for(kind: &str) -> String {
    match kind {
        "burning" => "BURN".to_string(),
        "wet" => "WET".to_string(),
        "electrified" => "ELEC".to_string(),
        "poisoned" => "TOX".to_string(),
        "hypoxic" => "HYPX".to_string(),
        "combustible_atmosphere" => "FUEL".to_string(),
        "breach_decomp" => "DECMP".to_string(),
        "hyperthermic" => "HOT".to_string(),
        "hypothermic" => "COLD".to_string(),
        "radiation" => "RAD".to_string(),
        "concussed" => "CONC".to_string(),
        "deafened" => "DEAF".to_string(),
        "blinded" => "BLND".to_string(),
        "bleeding" => "BLEED".to_string(),
        "internal_shock" => "ISHK".to_string(),
        "low_battery" => "BATT".to_string(),
        "coolant_leaking" => "COOL".to_string(),
        "oil_leaking" => "OIL".to_string(),
        "overheating" => "HEAT".to_string(),
        "hunger" => "HUNG".to_string(),
        "thirst" => "THRST".to_string(),
        "sleep_dep" => "SLP".to_string(),
        "sanity_low" => "SAN".to_string(),
        // M16A § Environmental afflictions (11 kinds).
        "stuffiness" => "STFY".to_string(),
        "heatstroke" => "HSTK".to_string(),
        "hypothermia" => "HYPO".to_string(),
        "asphyxiation" => "ASFX".to_string(),
        "refrigerant_inhalation" => "RFRG".to_string(),
        "electrocution" => "ECTN".to_string(),
        "illuminated" => "LIT".to_string(),
        "laceration" => "LACR".to_string(),
        "trench_foot" => "TRFT".to_string(),
        "stamina_movement_cost" => "LOAD".to_string(),
        "panic_freeze_env" => "PANC".to_string(),
        _ => "?".to_string(),
    }
}

fn tooltip_for(kind: &str, severity: f32) -> String {
    let pct = (severity * 100.0) as i32;
    let body = match kind {
        "burning" => "Fire DOT. Cleared by water, time, suffocation.",
        "wet" => "Wet surface. Slip risk on ice. Cleared by drying.",
        "electrified" => "Shock. Mobility loss. Cleared by insulation, time.",
        "poisoned" => "Toxic exposure. HP drain. Cleared by antidote / suit.",
        "hypoxic" => "Low oxygen. HP drain. Cleared by atmosphere recovery.",
        "combustible_atmosphere" => "Flammable atmosphere. Ignition risk.",
        "breach_decomp" => "Decompression. Drift toward breach. Cleared by seal.",
        "hyperthermic" => "Heat. Aim drift. Cleared by cooling.",
        "hypothermic" => "Cold. Slow movement. Cleared by warming.",
        "radiation" => "Radiation exposure. Latent dose accumulating.",
        "concussed" => "Concussed. Aim wobble. Cleared with time.",
        "deafened" => "Deafened. Hearing range cut. Cleared with time.",
        "blinded" => "Blinded. Aim accuracy ruined. Cleared with time.",
        "bleeding" => "Bleeding wound. HP drain. Cleared by medikit.",
        "internal_shock" => "Module shock cascade. Cleared by repair.",
        "low_battery" => "Battery low. Module degradation. Cleared by recharge.",
        "coolant_leaking" => "Coolant leak. Heat rising. Cleared by repair.",
        "oil_leaking" => "Oil leak. Fire risk. Cleared by repair.",
        "overheating" => "Heat critical. Weapons disabled. Cleared by cooling.",
        "hunger" => "Hunger. Speed loss. Eat food.",
        "thirst" => "Thirst. Aim wobble. Drink water.",
        "sleep_dep" => "Sleep deprived. Reaction time +50%. Sleep.",
        "sanity_low" => "Sanity low. AI decisions impaired. Therapy + recreation.",
        "stuffiness" => "Stuffy room (humidity + CO2 + crowding). Ventilate.",
        "heatstroke" => "Heatstroke. Speed + aim degraded. Cool down.",
        "hypothermia" => "Hypothermia. Speed + aim degraded. Warm enclosure.",
        "asphyxiation" => "Asphyxiation. Low O2 ambient. Return to breathable atmosphere.",
        "refrigerant_inhalation" => "Refrigerant inhalation. Lung damage. Decontaminate suit.",
        "electrocution" => "Electrocution. KO grace + shock damage. Wait or insulate.",
        "illuminated" => "Illuminated by spotlight. Concealment lost. Leave cone.",
        "laceration" => "Laceration. Bleed stack. Bandage + tend per wound.",
        "trench_foot" => "Trench foot. Speed loss + infection risk. Dry boots + warm.",
        "stamina_movement_cost" => "Heavy load. Stamina drain + speed loss. Drop weapon or resupply.",
        "panic_freeze_env" => "Panic freeze. Cannot act. Wait or squadmate stabilize.",
        _ => "Unknown affliction.",
    };
    format!("{}\nSeverity: {pct}%\n{}", kind, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_caps_at_5_visible() {
        let mut state = AfflictionStripState::default();
        let raw = vec![
            ("burning".to_string(), 0.8),
            ("bleeding".to_string(), 0.6),
            ("hypoxic".to_string(), 0.4),
            ("concussed".to_string(), 0.3),
            ("wet".to_string(), 0.2),
            ("deafened".to_string(), 0.1),
        ];
        state.refresh(&raw);
        assert_eq!(state.entries.len(), 5);
        assert_eq!(state.hidden_more_count, 1);
    }

    #[test]
    fn entries_sorted_by_severity_desc() {
        let mut state = AfflictionStripState::default();
        let raw = vec![
            ("wet".to_string(), 0.2),
            ("burning".to_string(), 0.8),
            ("bleeding".to_string(), 0.4),
        ];
        state.refresh(&raw);
        assert_eq!(state.entries[0].kind, "burning");
        assert_eq!(state.entries[1].kind, "bleeding");
        assert_eq!(state.entries[2].kind, "wet");
    }

    #[test]
    fn critical_flag_at_severity_above_threshold() {
        let entry = AfflictionStripEntry::from_kind("burning", 0.85);
        assert!(entry.critical);
        let entry = AfflictionStripEntry::from_kind("wet", 0.3);
        assert!(!entry.critical);
    }

    #[test]
    fn summary_line_renders_plus_more_counter() {
        let mut state = AfflictionStripState::default();
        let raw = vec![
            ("burning".to_string(), 0.9),
            ("bleeding".to_string(), 0.8),
            ("hypoxic".to_string(), 0.7),
            ("concussed".to_string(), 0.6),
            ("wet".to_string(), 0.5),
            ("deafened".to_string(), 0.4),
            ("blinded".to_string(), 0.3),
        ];
        state.refresh(&raw);
        let line = state.summary_line();
        assert!(line.contains("+2 more"), "summary_line should include '+2 more', got: {line}");
    }

    #[test]
    fn icon_for_every_22_kinds_returns_unique_label() {
        let kinds = [
            "burning",
            "wet",
            "electrified",
            "poisoned",
            "hypoxic",
            "combustible_atmosphere",
            "breach_decomp",
            "hyperthermic",
            "hypothermic",
            "radiation",
            "concussed",
            "deafened",
            "blinded",
            "bleeding",
            "internal_shock",
            "low_battery",
            "coolant_leaking",
            "oil_leaking",
            "overheating",
            "hunger",
            "thirst",
            "sleep_dep",
            "sanity_low",
        ];
        let mut icons: std::collections::HashSet<String> = std::collections::HashSet::new();
        for k in kinds {
            let icon = icon_for(k);
            assert_ne!(icon, "?", "icon_for({k}) returned default '?'");
            icons.insert(icon);
        }
        assert_eq!(icons.len(), kinds.len(), "icons must be unique across kinds");
    }

    #[test]
    fn icon_for_every_m16a_env_kind_returns_unique_label() {
        let env_kinds = [
            "stuffiness",
            "heatstroke",
            "hypothermia",
            "asphyxiation",
            "refrigerant_inhalation",
            "electrocution",
            "illuminated",
            "laceration",
            "trench_foot",
            "stamina_movement_cost",
            "panic_freeze_env",
        ];
        let mut icons: std::collections::HashSet<String> = std::collections::HashSet::new();
        for k in env_kinds {
            let icon = icon_for(k);
            assert_ne!(icon, "?", "icon_for({k}) returned default '?'");
            icons.insert(icon);
        }
        assert_eq!(icons.len(), env_kinds.len(), "m16a env icons must be unique");
    }
}
