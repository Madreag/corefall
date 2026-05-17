//! **M6B**: Tetris-style inventory-grid HUD widget.
//!
//! The widget owns its own Bevy `Resource` state and mirrors the per-actor
//! [`cf_actor::InventoryGrid`] each frame. M6B ships the foundational
//! state struct + the read-only projection that drives the M27 drag-drop
//! UX (which adds keyboard / mouse handlers + grid-collision avoidance
//! on top of the surface defined here).
//!
//! Per spec § Crates / modules touched:
//! > `cf-ui::inventory_grid` (NEW) — Tetris-grid UX (M27 polishes)
//!
//! The HUD widgets it surfaces:
//!
//! - `grid_dimensions` — current backpack tier's grid `(w, h)`.
//! - `placed_items` — one entry per top-level placement (item id +
//!   footprint + origin tile).
//! - `total_carried_kg` / `total_carried_volume_l` — aggregated mass +
//!   bulk for the carrier-cap badge.
//! - `encumbered` / `encumbrance_band` — drives the "ENCUMBERED"
//!   warning per spec § "HUD shows 'ENCUMBERED' warning".
//! - `walk_speed_multiplier` — surfaced so the HUD can render the
//!   running-speed-icon shrinkage.

use bevy::prelude::*;

use cf_actor::{ActorState, InventoryEncumbrance, InventoryGrid, PlacedItem};
use cf_equipment::{BackpackTier, EncumbranceBand, GridDim, ItemSpec};

/// One placement projected for HUD rendering. Lighter-weight than the
/// canonical [`PlacedItem`] — the widget only needs the visual shape +
/// label.
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryGridPlacementView {
    pub instance_id: u64,
    pub item_id: String,
    pub display_name: String,
    pub origin: (u8, u8),
    pub footprint: (u8, u8),
    pub rotated: bool,
    pub mass_kg: f32,
    pub bulk_volume_l: f32,
    pub is_container: bool,
    pub nested_count: u16,
    pub stack_count: u16,
    pub current_liquid_l: f32,
}

impl InventoryGridPlacementView {
    fn from_placed(item: &PlacedItem, spec: Option<&ItemSpec>) -> Self {
        let (display_name, dims, mass, bulk, is_container, is_liquid_container) = match spec {
            Some(s) => (
                s.display_name.clone(),
                if item.rotated {
                    s.dimensions.rotated()
                } else {
                    s.dimensions
                },
                item.mass_kg(s),
                item.bulk_volume_l(s),
                s.is_container(),
                s.is_liquid_container(),
            ),
            None => (
                item.item_id.clone(),
                GridDim::new(1, 1),
                0.0,
                0.0,
                false,
                false,
            ),
        };
        let _ = is_liquid_container;
        let nested_count = item.container.as_ref().map(|c| c.items.len() as u16).unwrap_or(0);
        Self {
            instance_id: item.instance_id,
            item_id: item.item_id.clone(),
            display_name,
            origin: item.origin,
            footprint: (dims.w, dims.h),
            rotated: item.rotated,
            mass_kg: mass,
            bulk_volume_l: bulk,
            is_container,
            nested_count,
            stack_count: item.count,
            current_liquid_l: item.current_liquid_l,
        }
    }
}

/// Bevy resource carrying the live inventory-grid view.
#[derive(Resource, Debug, Clone)]
pub struct InventoryGridState {
    /// Backpack tier driving the grid dimensions.
    pub tier: BackpackTier,
    /// Grid dimensions `(w, h)` for the current tier.
    pub dimensions: (u8, u8),
    /// Top-level placements (length ≤ `dimensions.w * dimensions.h`).
    pub placements: Vec<InventoryGridPlacementView>,
    /// Carried mass aggregate in kg.
    pub total_carried_kg: f32,
    /// Carried bulk aggregate in liters.
    pub total_carried_volume_l: f32,
    /// Per-actor maximum carry mass (50 × origin modifier).
    pub max_carry_kg: f32,
    /// Per-actor maximum carry volume (60 × origin modifier).
    pub max_carry_volume_l: f32,
    /// Walk-speed multiplier from the encumbrance curve.
    pub walk_speed_multiplier: f32,
    /// Discrete encumbrance band.
    pub band: EncumbranceBand,
    /// Spec § "HUD shows 'ENCUMBERED' warning" — true at band=Heavy.
    pub encumbered: bool,
    /// Whether the grid widget is currently visible (false in photo mode
    /// / cinematics / killcam playback).
    pub visible: bool,
}

impl Default for InventoryGridState {
    fn default() -> Self {
        Self {
            tier: BackpackTier::Small,
            dimensions: BackpackTier::Small.dimensions(),
            placements: Vec::new(),
            total_carried_kg: 0.0,
            total_carried_volume_l: 0.0,
            max_carry_kg: cf_equipment::HUMAN_BASELINE_MAX_CARRY_KG,
            max_carry_volume_l: cf_equipment::HUMAN_BASELINE_MAX_CARRY_VOLUME_L,
            walk_speed_multiplier: 1.0,
            band: EncumbranceBand::None,
            encumbered: false,
            visible: true,
        }
    }
}

impl InventoryGridState {
    /// Refresh the widget from the actor's live inventory grid +
    /// encumbrance envelope. Caller drives this each frame (or each
    /// tick) from the cf-app Bevy bridge so the HUD reflects the engine
    /// snapshot.
    pub fn refresh_from_actor(&mut self, actor: &ActorState) {
        let envelope = actor.inventory_encumbrance.unwrap_or_default();
        self.max_carry_kg = envelope.max_carry_kg;
        self.max_carry_volume_l = envelope.max_carry_volume_l;
        self.walk_speed_multiplier = envelope.walk_speed_multiplier;
        self.band = envelope.band;
        self.encumbered = envelope.encumbered();
        if let Some(grid) = actor.inventory_grid() {
            self.tier = grid.tier;
            self.dimensions = grid.dimensions();
            self.placements = grid
                .items
                .iter()
                .map(|p| InventoryGridPlacementView::from_placed(p, cf_equipment::spec_for_id(&p.item_id).as_ref()))
                .collect();
            self.total_carried_kg = grid.total_mass_kg();
            self.total_carried_volume_l = grid.total_bulk_l();
        } else {
            self.placements.clear();
            self.total_carried_kg = 0.0;
            self.total_carried_volume_l = 0.0;
        }
    }

    /// Direct refresh from a grid + envelope pair (test convenience).
    pub fn refresh_from_grid(&mut self, grid: &InventoryGrid, envelope: &InventoryEncumbrance) {
        self.tier = grid.tier;
        self.dimensions = grid.dimensions();
        self.placements = grid
            .items
            .iter()
            .map(|p| InventoryGridPlacementView::from_placed(p, cf_equipment::spec_for_id(&p.item_id).as_ref()))
            .collect();
        self.total_carried_kg = grid.total_mass_kg();
        self.total_carried_volume_l = grid.total_bulk_l();
        self.max_carry_kg = envelope.max_carry_kg;
        self.max_carry_volume_l = envelope.max_carry_volume_l;
        self.walk_speed_multiplier = envelope.walk_speed_multiplier;
        self.band = envelope.band;
        self.encumbered = envelope.encumbered();
    }

    /// Carry ratio in `[0, ∞)`.
    pub fn carry_ratio(&self) -> f32 {
        if self.max_carry_kg <= 0.0 {
            return 0.0;
        }
        self.total_carried_kg / self.max_carry_kg
    }

    /// Whether the volume cap is exceeded.
    pub fn over_volume_cap(&self) -> bool {
        self.total_carried_volume_l > self.max_carry_volume_l
    }

    /// Surface the spec-mandated "ENCUMBERED" warning string when the
    /// band is Heavy; empty otherwise.
    pub fn warning_text(&self) -> &'static str {
        if self.encumbered {
            "ENCUMBERED"
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grid_is_small_empty() {
        let s = InventoryGridState::default();
        assert_eq!(s.dimensions, (4, 6));
        assert!(s.placements.is_empty());
        assert!((s.total_carried_kg - 0.0).abs() < 1e-6);
        assert!(!s.encumbered);
    }

    #[test]
    fn refresh_from_grid_pulls_placements() {
        let mut g = InventoryGrid::default();
        g.add_top_level("rifle_m1", 1, 0.0);
        g.add_top_level("water_bottle", 1, 0.5);
        let mut env = InventoryEncumbrance::for_origin("human");
        env.set_carried(g.total_mass_kg(), g.total_bulk_l());
        let mut s = InventoryGridState::default();
        s.refresh_from_grid(&g, &env);
        assert_eq!(s.placements.len(), 2);
        assert!((s.total_carried_kg - (3.5 + 0.7)).abs() < 1e-4);
        assert_eq!(s.warning_text(), "");
    }

    #[test]
    fn refresh_from_grid_renders_encumbered_warning() {
        // Scenario: Encumbrance at 100% reduces walk speed →
        //   HUD shows "ENCUMBERED" warning.
        let mut g = InventoryGrid::default();
        // 50 kg of ammo (60 * 0.012 = 0.72 kg per stack); fill with a
        // single heavy generic-mass item: 16 backpacks (~80 kg) >> 50 kg.
        for _ in 0..30 {
            g.add_top_level("rifle_m1", 1, 0.0);
        }
        let mut env = InventoryEncumbrance::for_origin("human");
        env.set_carried(g.total_mass_kg(), g.total_bulk_l());
        let mut s = InventoryGridState::default();
        s.refresh_from_grid(&g, &env);
        assert!(s.encumbered);
        assert_eq!(s.band, EncumbranceBand::Heavy);
        assert_eq!(s.warning_text(), "ENCUMBERED");
    }

    #[test]
    fn refresh_from_actor_uses_attached_grid() {
        use cf_actor::{ActorId, Inventory, Vec2};
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.inventory_grid_attach();
        {
            let g = actor.inventory_grid_mut().unwrap();
            g.add_top_level("water_bottle", 1, 1.0);
        }
        actor.recompute_inventory_encumbrance();
        let mut s = InventoryGridState::default();
        s.refresh_from_actor(&actor);
        assert_eq!(s.placements.len(), 1);
        assert!((s.total_carried_kg - 1.2).abs() < 1e-4);
    }
}
