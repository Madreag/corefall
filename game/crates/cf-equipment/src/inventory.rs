//! M6: extended inventory model (8 active slots + 3 reserved tank slots + weight).
//!
//! M1 ships a 4-slot `InventoryItem` enum in `cf-actor`. M6 introduces the
//! richer 11-slot record used by the engine for the M6 player. The M1 enum
//! stays the source of truth for legacy compat; this module mirrors it with
//! the M6 surface so cf-control can serve `observe.actor.inventory_extended`.

use serde::{Deserialize, Serialize};

/// 8 active inventory slots per spec § Inventory.
pub const ACTIVE_SLOT_COUNT: usize = 8;
/// 3 reserved tank slots per spec § Tank slot reservation (M17 forward-compat).
pub const TANK_SLOT_COUNT: usize = 3;
/// Total slots an extended inventory exposes (11).
pub const TOTAL_SLOT_COUNT: usize = ACTIVE_SLOT_COUNT + TANK_SLOT_COUNT;

/// Spec § Weight system: > 30 kg forces Walk; sprint disabled.
pub const WEIGHT_FORCE_WALK_KG: f32 = 30.0;

/// Spec § Weight system: > 45 kg forces actor to crawl (M13 forward-compat number).
pub const WEIGHT_FORCE_CRAWL_KG: f32 = 45.0;

/// Categorical slot kind. Drives HUD widget + restriction logic.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotKind {
    Primary = 0,
    Secondary = 1,
    Sidearm = 2,
    Tool1 = 3,
    Tool2 = 4,
    Grenade = 5,
    Medical = 6,
    Special = 7,
    TankPrimary = 8,
    TankSecondary = 9,
    TankUtility = 10,
}

impl SlotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SlotKind::Primary => "primary",
            SlotKind::Secondary => "secondary",
            SlotKind::Sidearm => "sidearm",
            SlotKind::Tool1 => "tool1",
            SlotKind::Tool2 => "tool2",
            SlotKind::Grenade => "grenade",
            SlotKind::Medical => "medical",
            SlotKind::Special => "special",
            SlotKind::TankPrimary => "tank_primary",
            SlotKind::TankSecondary => "tank_secondary",
            SlotKind::TankUtility => "tank_utility",
        }
    }

    pub fn is_tank(self) -> bool {
        matches!(
            self,
            SlotKind::TankPrimary | SlotKind::TankSecondary | SlotKind::TankUtility
        )
    }

    pub fn is_active(self) -> bool {
        !self.is_tank()
    }
}

/// Per-slot state. M6 lock declares tank slots `locked`; M17 unlocks.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotState {
    Empty = 0,
    Occupied = 1,
    /// Tank slots at M6: locked, non-functional placeholders.
    Locked = 2,
}

impl SlotState {
    pub fn as_str(self) -> &'static str {
        match self {
            SlotState::Empty => "empty",
            SlotState::Occupied => "occupied",
            SlotState::Locked => "locked",
        }
    }
}

/// One extended inventory slot record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtendedSlot {
    pub kind: SlotKind,
    pub state: SlotState,
    /// Identifier of the held item ("" when Empty/Locked).
    pub item_id: String,
    /// Per-slot weight in kg.
    pub weight_kg: f32,
    /// Per-slot durability (used by tool slots).
    pub durability_fraction: f32,
    /// Locked-tooltip surfaced by the HUD for tank slots.
    pub locked_tooltip: Option<String>,
}

impl ExtendedSlot {
    pub fn empty(kind: SlotKind) -> Self {
        let state = if kind.is_tank() {
            SlotState::Locked
        } else {
            SlotState::Empty
        };
        let locked_tooltip = if kind.is_tank() {
            Some("Reserved — see M17 for tank ladder".to_string())
        } else {
            None
        };
        Self {
            kind,
            state,
            item_id: String::new(),
            weight_kg: 0.0,
            durability_fraction: 1.0,
            locked_tooltip,
        }
    }

    pub fn occupy(kind: SlotKind, item_id: impl Into<String>, weight_kg: f32) -> Self {
        Self {
            kind,
            state: SlotState::Occupied,
            item_id: item_id.into(),
            weight_kg: weight_kg.max(0.0),
            durability_fraction: 1.0,
            locked_tooltip: None,
        }
    }
}

/// Reason a tank-slot insert is rejected at M6.
pub const TANK_SLOT_LOCKED_REASON: &str = "tank_slot_locked_at_m2_2a";

/// Full M6 extended inventory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtendedInventory {
    pub slots: Vec<ExtendedSlot>,
    /// Selected slot index (0..ACTIVE_SLOT_COUNT). Tank slots cannot be selected.
    pub selected: u8,
    /// Cached total weight.
    pub total_weight_kg: f32,
    /// Backpack auxiliary capacity (spec § "Backpack capacity"); spare items.
    pub backpack_capacity: u32,
    pub backpack_items: Vec<String>,
}

impl Default for ExtendedInventory {
    fn default() -> Self {
        let slots = vec![
            ExtendedSlot::empty(SlotKind::Primary),
            ExtendedSlot::empty(SlotKind::Secondary),
            ExtendedSlot::empty(SlotKind::Sidearm),
            ExtendedSlot::empty(SlotKind::Tool1),
            ExtendedSlot::empty(SlotKind::Tool2),
            ExtendedSlot::empty(SlotKind::Grenade),
            ExtendedSlot::empty(SlotKind::Medical),
            ExtendedSlot::empty(SlotKind::Special),
            ExtendedSlot::empty(SlotKind::TankPrimary),
            ExtendedSlot::empty(SlotKind::TankSecondary),
            ExtendedSlot::empty(SlotKind::TankUtility),
        ];
        Self {
            slots,
            selected: 0,
            total_weight_kg: 0.0,
            backpack_capacity: 6,
            backpack_items: Vec::new(),
        }
    }
}

impl ExtendedInventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recompute_weight(&mut self) -> f32 {
        let mut sum = 0.0;
        for s in &self.slots {
            if s.state == SlotState::Occupied {
                sum += s.weight_kg.max(0.0);
            }
        }
        self.total_weight_kg = sum;
        sum
    }

    pub fn forces_walk(&self) -> bool {
        self.total_weight_kg > WEIGHT_FORCE_WALK_KG
    }

    pub fn forces_crawl(&self) -> bool {
        self.total_weight_kg > WEIGHT_FORCE_CRAWL_KG
    }

    pub fn select(&mut self, slot: u8) -> bool {
        if (slot as usize) < ACTIVE_SLOT_COUNT {
            self.selected = slot;
            true
        } else {
            false
        }
    }

    pub fn selected_slot(&self) -> Option<&ExtendedSlot> {
        self.slots.get(self.selected as usize)
    }

    /// Insert an item into the slot at index. Tank slots reject with the
    /// spec-locked reason.
    pub fn try_insert(&mut self, slot_idx: u8, item_id: impl Into<String>, weight_kg: f32) -> Result<(), String> {
        let Some(slot) = self.slots.get_mut(slot_idx as usize) else {
            return Err("slot_out_of_range".to_string());
        };
        if slot.kind.is_tank() {
            return Err(TANK_SLOT_LOCKED_REASON.to_string());
        }
        slot.state = SlotState::Occupied;
        slot.item_id = item_id.into();
        slot.weight_kg = weight_kg.max(0.0);
        slot.durability_fraction = 1.0;
        slot.locked_tooltip = None;
        self.recompute_weight();
        Ok(())
    }

    /// Drop slot contents to the world. Returns the (item_id, weight_kg)
    /// that left the inventory, or None when slot was already empty.
    pub fn drop_slot(&mut self, slot_idx: u8) -> Option<(String, f32)> {
        let slot = self.slots.get_mut(slot_idx as usize)?;
        if slot.kind.is_tank() || slot.state != SlotState::Occupied {
            return None;
        }
        let id = std::mem::take(&mut slot.item_id);
        let w = slot.weight_kg;
        slot.weight_kg = 0.0;
        slot.state = SlotState::Empty;
        self.recompute_weight();
        Some((id, w))
    }

    /// First-empty-active-slot insertion (used by pickup).
    pub fn pickup_into_first_empty(&mut self, item_id: impl Into<String>, weight_kg: f32) -> Result<u8, String> {
        let item_id_owned = item_id.into();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.kind.is_tank() {
                continue;
            }
            if slot.state == SlotState::Empty {
                slot.state = SlotState::Occupied;
                slot.item_id = item_id_owned;
                slot.weight_kg = weight_kg.max(0.0);
                self.recompute_weight();
                return Ok(i as u8);
            }
        }
        Err("inventory_full".to_string())
    }

    /// Total number of slots exposed (11).
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Count of tank slots (always 3).
    pub fn tank_slot_count(&self) -> usize {
        self.slots.iter().filter(|s| s.kind.is_tank()).count()
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &ExtendedSlot> {
        self.slots.iter().filter(|s| s.kind.is_active())
    }

    pub fn iter_tank(&self) -> impl Iterator<Item = &ExtendedSlot> {
        self.slots.iter().filter(|s| s.kind.is_tank())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eleven_slots_3_tank() {
        let inv = ExtendedInventory::new();
        assert_eq!(inv.slot_count(), 11);
        assert_eq!(inv.tank_slot_count(), 3);
    }

    #[test]
    fn tank_slots_locked_at_m6() {
        let inv = ExtendedInventory::new();
        for s in inv.iter_tank() {
            assert_eq!(s.state, SlotState::Locked);
            assert!(s.locked_tooltip.is_some());
        }
    }

    #[test]
    fn tank_slot_insert_rejected() {
        let mut inv = ExtendedInventory::new();
        let err = inv.try_insert(8, "tank.o2.civilian", 5.0).unwrap_err();
        assert_eq!(err, TANK_SLOT_LOCKED_REASON);
    }

    #[test]
    fn weight_force_walk_threshold() {
        let mut inv = ExtendedInventory::new();
        let _ = inv.try_insert(0, "rifle.heavy", 35.0);
        assert!(inv.forces_walk());
        assert!(!inv.forces_crawl());
    }

    #[test]
    fn pickup_into_first_empty() {
        let mut inv = ExtendedInventory::new();
        let _ = inv.try_insert(0, "rifle", 4.0);
        let slot = inv.pickup_into_first_empty("medkit", 1.0).unwrap();
        assert_eq!(slot, 1);
    }

    #[test]
    fn pickup_skips_tank_slots() {
        let mut inv = ExtendedInventory::new();
        for i in 0..8 {
            let _ = inv.try_insert(i, format!("item{i}"), 0.5);
        }
        let result = inv.pickup_into_first_empty("extra", 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn drop_clears_slot() {
        let mut inv = ExtendedInventory::new();
        inv.try_insert(2, "pistol", 1.0).unwrap();
        let dropped = inv.drop_slot(2).unwrap();
        assert_eq!(dropped.0, "pistol");
        assert_eq!(inv.slots[2].state, SlotState::Empty);
    }
}
