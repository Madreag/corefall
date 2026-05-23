//! M15D § F8 tile inspect overlay — reactions panel.
//!
//! Per spec § Acceptance criteria § "F8 tile inspect surfaces active
//! reactions":
//! > Given the player F8-inspects an acid+iron interface
//! > Then a reactions panel shows rxn.corrosion.acid_iron with rate /s
//! >   + ETA + cumulative delta_h_kj
//! > And the panel updates each tick until completion
//!
//! The overlay's state is driven by the cf-control engine bridge which
//! mirrors a per-tile reaction status to this Bevy resource. The HUD
//! widget renders one row per active reaction at the inspected tile.

use bevy::prelude::*;

use cf_material::ReactionVariant;

/// One row of the F8 inspect reactions panel.
#[derive(Debug, Clone, PartialEq)]
pub struct InspectReactionRow {
    pub reaction_id: String,
    pub display_name: String,
    pub rate_per_s: f32,
    pub eta_ticks: Option<u64>,
    pub cumulative_delta_h_kj: f32,
    pub variant: ReactionVariant,
    pub propagates: bool,
    pub auto_ignite: bool,
}

impl InspectReactionRow {
    /// Compact one-line string for the inspect panel (used by the
    /// status strip widget). Format: `<id> | <rate>/s | ΔH <total>`.
    #[must_use]
    pub fn summary_line(&self) -> String {
        let eta = self
            .eta_ticks
            .map(|t| format!("ETA {t}t"))
            .unwrap_or_else(|| "ETA --".to_string());
        format!(
            "{} | {:.2}/s | {} | ΔH {:.1} kJ",
            self.reaction_id, self.rate_per_s, eta, self.cumulative_delta_h_kj
        )
    }

    /// Build a row from a `ReactionDef` + an in-flight reaction state
    /// (cumulative ΔH so far + projected ETA). The cumulative ΔH is in
    /// kJ — callers multiply by moles reacted before passing here.
    #[must_use]
    pub fn from_def(
        def: &cf_material::ReactionDef,
        cumulative_delta_h_kj: f32,
        eta_ticks: Option<u64>,
        temperature_k: f32,
    ) -> Self {
        let rate = def.effective_rate_per_s(temperature_k);
        Self {
            reaction_id: def.id.clone(),
            display_name: def.display_name.clone(),
            rate_per_s: rate,
            eta_ticks,
            cumulative_delta_h_kj,
            variant: def.variant,
            propagates: def.propagates,
            auto_ignite: def.auto_ignite,
        }
    }
}

/// HUD resource — the F8 inspect reactions panel state.
/// Owned by cf-app's HUD bridge; populated each tick from the
/// per-tile reaction evaluator.
#[derive(Resource, Debug, Clone, Default)]
pub struct TileInspectOverlayState {
    pub visible: bool,
    pub world_x: i32,
    pub world_y: i32,
    pub material_name: String,
    pub temperature_k: f32,
    pub active_reactions: Vec<InspectReactionRow>,
}

impl TileInspectOverlayState {
    /// Show the panel at `(world_x, world_y)` with the supplied rows.
    pub fn show_at(&mut self, world_x: i32, world_y: i32, material_name: &str, temperature_k: f32) {
        self.visible = true;
        self.world_x = world_x;
        self.world_y = world_y;
        self.material_name = material_name.to_string();
        self.temperature_k = temperature_k;
    }

    /// Hide the panel (player closed F8 or moved away).
    pub fn hide(&mut self) {
        self.visible = false;
        self.active_reactions.clear();
    }

    /// Replace the panel's reactions list. Called each tick by the
    /// engine bridge — the new list always reflects what's firing right
    /// now at the inspected tile.
    pub fn set_reactions(&mut self, rows: Vec<InspectReactionRow>) {
        self.active_reactions = rows;
    }

    /// HUD-grade one-line header for the panel: `Tile (x, y) — <material> @ <T> K`.
    #[must_use]
    pub fn header_line(&self) -> String {
        format!(
            "Tile ({}, {}) — {} @ {:.1} K",
            self.world_x, self.world_y, self.material_name, self.temperature_k
        )
    }

    /// Compact rendering of all active reactions, one per line.
    #[must_use]
    pub fn rows_text(&self) -> Vec<String> {
        self.active_reactions.iter().map(|r| r.summary_line()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_material::{ReactionDef, ReactionInput, ReactionOutput};

    fn acid_iron_def() -> ReactionDef {
        ReactionDef {
            id: "rxn.corrosion.acid_iron".to_string(),
            display_name: "Acid on Iron".to_string(),
            inputs: vec![
                ReactionInput {
                    material: "acid".into(),
                    moles: 2.0,
                    molar_mass_g_per_mol: Some(36.461),
                    material_id: None,
                },
                ReactionInput {
                    material: "iron".into(),
                    moles: 1.0,
                    molar_mass_g_per_mol: Some(55.845),
                    material_id: None,
                },
            ],
            outputs: vec![
                ReactionOutput {
                    material: "iron_chloride".into(),
                    moles: 1.0,
                    molar_mass_g_per_mol: Some(126.751),
                    material_id: None,
                },
                ReactionOutput {
                    material: "hydrogen".into(),
                    moles: 1.0,
                    molar_mass_g_per_mol: Some(2.016),
                    material_id: None,
                },
            ],
            delta_h_kj_per_mol: -89.0,
            activation_energy_kj_per_mol: 18.0,
            rate_constant_per_s: 0.5,
            min_temperature_k: Some(273.0),
            min_pressure_kpa: None,
            catalyst: None,
            catalyst_id: None,
            variant: ReactionVariant::PerPixel,
            emits_event: true,
            propagates: true,
            auto_ignite: false,
        }
    }

    #[test]
    fn summary_line_includes_reaction_id_rate_and_delta_h() {
        let def = acid_iron_def();
        let row = InspectReactionRow::from_def(&def, -44.5, Some(120), 300.0);
        let line = row.summary_line();
        assert!(line.contains("rxn.corrosion.acid_iron"));
        assert!(line.contains("ETA 120t"));
        assert!(line.contains("ΔH -44.5 kJ"));
    }

    #[test]
    fn state_shows_and_hides_at_tile() {
        let mut s = TileInspectOverlayState::default();
        assert!(!s.visible);
        s.show_at(10, 20, "iron", 295.0);
        assert!(s.visible);
        assert_eq!(s.world_x, 10);
        assert_eq!(s.material_name, "iron");

        let def = acid_iron_def();
        let row = InspectReactionRow::from_def(&def, -89.0, Some(60), 300.0);
        s.set_reactions(vec![row]);
        assert_eq!(s.active_reactions.len(), 1);
        assert!(s.rows_text()[0].contains("rxn.corrosion.acid_iron"));
        s.hide();
        assert!(!s.visible);
        assert!(s.active_reactions.is_empty());
    }

    #[test]
    fn header_line_renders_at_inspected_tile() {
        let mut s = TileInspectOverlayState::default();
        s.show_at(15, 7, "acid", 295.5);
        assert!(s.header_line().contains("Tile (15, 7)"));
        assert!(s.header_line().contains("acid"));
        assert!(s.header_line().contains("295.5"));
    }
}
