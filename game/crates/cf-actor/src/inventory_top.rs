use serde::{Deserialize, Serialize};

/// Item id used by the M1 inventory. Maps 1:1 to a slot index in [`Inventory::items`].
/// Resolved against per-actor item presets (`cf-equipment::RIFLE_M1_DEFAULT_ID`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemSlot(pub u32);

impl ItemSlot {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// One inventory slot with a fixed item kind and (optional) ammo state.
///
/// Kept simple in M1: each actor has up to 4 slots. The "selected" slot drives
/// `weapon_fired` / `weapon_reloaded`. Slots beyond the rifle are placeholders for M5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InventoryItem {
    Empty,
    Rifle { preset: String },
}

impl InventoryItem {
    pub fn label(&self) -> &str {
        match self {
            InventoryItem::Empty => "empty",
            InventoryItem::Rifle { .. } => "rifle",
        }
    }

    pub fn kind_label(&self) -> &str {
        match self {
            InventoryItem::Empty => "empty",
            InventoryItem::Rifle { .. } => "rifle",
        }
    }

    pub fn is_rifle(&self) -> bool {
        matches!(self, InventoryItem::Rifle { .. })
    }
}

/// Up to four inventory slots. M1 ships one rifle; remaining slots are `Empty`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub selected: ItemSlot,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: vec![InventoryItem::Empty; 8],
            selected: ItemSlot(0),
        }
    }
}

impl Inventory {
    pub fn with_rifle(preset: &str) -> Self {
        let mut inv = Self::default();
        inv.items[0] = InventoryItem::Rifle {
            preset: preset.to_string(),
        };
        inv
    }

    pub fn selected_item(&self) -> &InventoryItem {
        self.items
            .get(self.selected.0 as usize)
            .unwrap_or(&InventoryItem::Empty)
    }

    pub fn try_select(&mut self, slot: ItemSlot) -> bool {
        if (slot.0 as usize) < self.items.len() && self.selected != slot {
            self.selected = slot;
            true
        } else {
            false
        }
    }

    pub fn rifle_slot(&self) -> Option<ItemSlot> {
        self.items
            .iter()
            .enumerate()
            .find_map(|(i, it)| if it.is_rifle() { Some(ItemSlot(i as u32)) } else { None })
    }
}
