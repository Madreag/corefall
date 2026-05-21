use serde::{Deserialize, Serialize};

/// M4A body silhouette projection. Per-zone hp percentages clamped to `[0, 1]`.
/// `placeholder = true` until M5 lands the real body graph; HUD + AI consumers
/// must treat the layout as stable but the per-zone values as derived (not
/// individually targetable yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySilhouette {
    pub head_hp_pct: f32,
    pub torso_hp_pct: f32,
    pub arm_left_hp_pct: f32,
    pub arm_right_hp_pct: f32,
    pub leg_left_hp_pct: f32,
    pub leg_right_hp_pct: f32,
    pub placeholder: bool,
}

impl Default for BodySilhouette {
    fn default() -> Self {
        Self {
            head_hp_pct: 1.0,
            torso_hp_pct: 1.0,
            arm_left_hp_pct: 1.0,
            arm_right_hp_pct: 1.0,
            leg_left_hp_pct: 1.0,
            leg_right_hp_pct: 1.0,
            placeholder: true,
        }
    }
}

/// M4A module strip placeholder. M5's chassis grammar replaces this with real
/// per-module state (see [[spec/chassis-armor-mechs-and-origins]]); M4A ships
/// the surface so HUD + `cfctl observe` consumers + accessibility tooling can
/// rely on the contract early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleStrip {
    pub modules: Vec<ModuleState>,
    pub placeholder: bool,
}

impl Default for ModuleStrip {
    fn default() -> Self {
        Self {
            modules: Vec::new(),
            placeholder: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    pub id: String,
    pub label: String,
    pub state: String,
    pub kind: String,
}
