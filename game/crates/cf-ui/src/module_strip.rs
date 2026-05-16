//! **M11**: module strip widget per spec § "ModuleStrip ammo + power
//! state per module" + spec § "Module strip shows up to 5 modules
//! (WEAPON/JET/SHIELD/SENSOR/REPAIR with OK/DEG/WARN/FAIL/N/A)".
//!
//! M5 chassis owns the runtime data — this widget renders the projection
//! cf-app writes from `ActorObservation::module_strip` each frame.

use bevy::prelude::*;

use crate::HudModule;

/// Per-module render bundle for the HUD.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleStripEntry {
    /// Stable id (e.g., `weapon`, `jet`, `shield`, `sensor`, `repair`).
    pub id: String,
    /// Short label for the HUD line (e.g., `WEAPON`).
    pub label: String,
    /// State tag per spec § "OK / DEG / WARN / FAIL / N/A".
    pub state: ModuleState,
    /// Module kind (per-slot semantic; e.g., `weapon`, `mobility`).
    pub kind: String,
    /// Optional ammo / power / bound-zone display string per spec items
    /// #2130 + #2132 (e.g., `30/30`, `100%`, `torso`).
    pub aux: Option<String>,
}

/// State tag for a module per spec § Module strip taxonomy.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ModuleState {
    /// Module fully operational.
    Ok,
    /// Degraded but still functional.
    Degraded,
    /// Warning — partial failure.
    Warning,
    /// Failed — non-functional.
    Failed,
    /// Not available / not attached.
    NotApplicable,
}

impl ModuleState {
    /// snake_case identifier for cfctl wire form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleState::Ok => "ok",
            ModuleState::Degraded => "degraded",
            ModuleState::Warning => "warning",
            ModuleState::Failed => "failed",
            ModuleState::NotApplicable => "n_a",
        }
    }

    /// Short ASCII tag for the HUD line (color-independent).
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            ModuleState::Ok => "OK",
            ModuleState::Degraded => "DEG",
            ModuleState::Warning => "WARN",
            ModuleState::Failed => "FAIL",
            ModuleState::NotApplicable => "N/A",
        }
    }

    /// Parse a state tag (lower or upper).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "ok" | "nominal" => ModuleState::Ok,
            "deg" | "degraded" => ModuleState::Degraded,
            "warn" | "warning" => ModuleState::Warning,
            "fail" | "failed" | "broken" => ModuleState::Failed,
            _ => ModuleState::NotApplicable,
        }
    }
}

/// Resource projection of the module strip for the HUD. cf-app's bridge
/// writes this from the engine's `ActorObservation::module_strip` each
/// frame; cf-ui reads it to render the MODS line.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct ModuleStripState {
    pub modules: Vec<ModuleStripEntry>,
    /// `true` when the chassis is not attached (M4A placeholder).
    pub placeholder: bool,
}

impl ModuleStripState {
    /// Compose the MODS line. Each slot reads `<LABEL>:<TAG>[:aux]`.
    #[must_use]
    pub fn mods_line(&self) -> String {
        if self.modules.is_empty() {
            return "MODS: --".to_string();
        }
        let parts: Vec<String> = self
            .modules
            .iter()
            .map(|m| match &m.aux {
                Some(a) => format!("{}:{}:{}", m.label, m.state.tag(), a),
                None => format!("{}:{}", m.label, m.state.tag()),
            })
            .collect();
        format!("MODS: {}", parts.join("  "))
    }

    /// Replace the strip from a flat HudModule projection (M4A back-compat).
    pub fn set_from_hud_modules(&mut self, src: &[HudModule], placeholder: bool) {
        self.modules = src
            .iter()
            .map(|m| ModuleStripEntry {
                id: m.id.clone(),
                label: m.label.clone(),
                state: ModuleState::from_str(&m.state),
                kind: m.kind.clone(),
                aux: None,
            })
            .collect();
        self.placeholder = placeholder;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_strip_renders_placeholder() {
        let s = ModuleStripState::default();
        assert_eq!(s.mods_line(), "MODS: --");
    }

    #[test]
    fn full_strip_renders_all_modules() {
        let s = ModuleStripState {
            modules: vec![
                ModuleStripEntry {
                    id: "weapon".into(),
                    label: "WEAPON".into(),
                    state: ModuleState::Ok,
                    kind: "weapon".into(),
                    aux: Some("30/30".into()),
                },
                ModuleStripEntry {
                    id: "jet".into(),
                    label: "JET".into(),
                    state: ModuleState::Degraded,
                    kind: "mobility".into(),
                    aux: None,
                },
            ],
            placeholder: false,
        };
        let line = s.mods_line();
        assert!(line.contains("WEAPON:OK:30/30"));
        assert!(line.contains("JET:DEG"));
    }

    #[test]
    fn module_state_from_str_handles_aliases() {
        assert_eq!(ModuleState::from_str("OK"), ModuleState::Ok);
        assert_eq!(ModuleState::from_str("nominal"), ModuleState::Ok);
        assert_eq!(ModuleState::from_str("warning"), ModuleState::Warning);
        assert_eq!(ModuleState::from_str("FAIL"), ModuleState::Failed);
        assert_eq!(ModuleState::from_str("unknown"), ModuleState::NotApplicable);
    }
}
