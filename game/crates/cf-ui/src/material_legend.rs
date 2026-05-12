//! **M2**: HUD material legend widget.
//!
//! Lists the 8 launch materials with a display label + swatch color so
//! the player can identify what each color in the integrity / pathability /
//! mobility / hazard / build_repair overlay means.
//!
//! Toggleable: cycled together with `act.player.toggle_material_overlay`.
//! When overlay mode = "off", the legend is hidden by default.

use bevy::prelude::*;

use cf_terrain::{material_affordance, MaterialAffordance, MaterialId};

/// One entry in the material legend HUD widget.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialLegendEntry {
    pub id: MaterialId,
    pub name: String,
    pub display_name: String,
    pub swatch: Color,
    pub hardness: f32,
    pub diggable: bool,
    pub anchorable: bool,
    pub hazard: bool,
    pub path_cost: f32,
    pub refusal_reason: Option<String>,
}

impl MaterialLegendEntry {
    fn from_affordance(aff: &MaterialAffordance) -> Self {
        let display_name = canonical_display_name(aff.name);
        let [r, g, b, _] = aff.overlay_rgba;
        let swatch = Color::srgb((r as f32) / 255.0, (g as f32) / 255.0, (b as f32) / 255.0);
        Self {
            id: aff.id,
            name: aff.name.to_string(),
            display_name,
            swatch,
            hardness: aff.hardness,
            diggable: aff.diggable,
            anchorable: aff.anchorable,
            hazard: aff.hazard,
            path_cost: aff.path_cost,
            refusal_reason: aff.refusal_reason.map(str::to_string),
        }
    }
}

fn canonical_display_name(name: &str) -> String {
    match name {
        "air" => "Air".to_string(),
        "dirt" => "Dirt".to_string(),
        "concrete" => "Concrete".to_string(),
        "metal_nohook" => "Reinforced Metal".to_string(),
        "hazard" => "Hazard Tile".to_string(),
        "loose_fill" => "Loose Rubble".to_string(),
        "repair_fill" => "Repair Foam".to_string(),
        "anchor" => "Anchor Rock".to_string(),
        other => other.to_string(),
    }
}

/// Build the legend for the canonical 8-material launch set.
#[must_use]
pub fn legend_entries() -> Vec<MaterialLegendEntry> {
    let mut out = Vec::with_capacity(8);
    for id in 0_u8..=7 {
        if let Some(aff) = material_affordance(id) {
            out.push(MaterialLegendEntry::from_affordance(aff));
        }
    }
    out
}

/// HUD resource carrying the current legend + visibility state. Written by
/// cf-app from `ObserveFrame.terrain.current_overlay_mode`. Toggled together
/// with `act.player.toggle_material_overlay`.
#[derive(Resource, Debug, Clone)]
pub struct MaterialLegendState {
    pub entries: Vec<MaterialLegendEntry>,
    pub visible: bool,
    pub overlay_mode: String,
}

impl Default for MaterialLegendState {
    fn default() -> Self {
        Self {
            entries: legend_entries(),
            visible: false,
            overlay_mode: "off".to_string(),
        }
    }
}

impl MaterialLegendState {
    /// Update the legend visibility + mode from the engine's overlay state.
    pub fn apply_overlay_mode(&mut self, mode: &str) {
        self.overlay_mode = mode.to_string();
        self.visible = mode != "off";
    }

    /// Returns a short caption appropriate for the active overlay mode
    /// (e.g., "INTEGRITY — color by material hardness"). Helps the HUD
    /// label the legend so the player understands the swatch semantics.
    #[must_use]
    pub fn caption(&self) -> &'static str {
        match self.overlay_mode.as_str() {
            "integrity" => "INTEGRITY — color by material hardness",
            "pathability" => "PATHABILITY — passable (green) / blocked (red)",
            "mobility" => "MOBILITY — anchorable (blue) / no-hook (gray)",
            "hazard" => "HAZARD — damage-on-touch surfaces tinted red",
            "build_repair" => "BUILD/REPAIR — where repair-fill can be placed",
            _ => "OVERLAY OFF",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_has_all_eight_launch_materials() {
        let entries = legend_entries();
        assert_eq!(entries.len(), 8);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        for expected in [
            "air",
            "dirt",
            "concrete",
            "metal_nohook",
            "hazard",
            "loose_fill",
            "repair_fill",
            "anchor",
        ] {
            assert!(names.contains(&expected), "legend missing {expected}");
        }
    }

    #[test]
    fn legend_state_toggles_visibility() {
        let mut s = MaterialLegendState::default();
        assert!(!s.visible);
        s.apply_overlay_mode("integrity");
        assert!(s.visible);
        assert_eq!(s.caption(), "INTEGRITY — color by material hardness");
        s.apply_overlay_mode("off");
        assert!(!s.visible);
    }

    #[test]
    fn metal_nohook_carries_refusal_label() {
        let entries = legend_entries();
        let metal = entries.iter().find(|e| e.name == "metal_nohook").unwrap();
        assert_eq!(metal.refusal_reason.as_deref(), Some("material_metal_nohook"));
        assert!(!metal.diggable);
    }
}
