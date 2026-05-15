//! M8 — 8-slot hotbar HUD widget.
//!
//! Per spec § UX widgets: bottom row icons + ammo count + suppressor /
//! bipod state badges; the active slot is highlighted. The widget keeps
//! its own state struct so cf-app's renderer can mirror inventory + the
//! active slot without touching the legacy [`crate::HudState`] shape.

use bevy::prelude::*;

/// Spec-mandated number of hotbar slots (8 inventory + 3 reserved tank
/// slots; the hotbar surfaces the 8 inventory slots only).
pub const HOTBAR_SLOTS: usize = 8;

/// Per-slot HUD model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HotbarSlot {
    /// Slot index (0..HOTBAR_SLOTS).
    pub index: u8,
    /// Display label for the icon (item name or short).
    pub label: Option<String>,
    /// Ammo count or unit count when applicable.
    pub ammo_count: Option<u32>,
    /// Whether a suppressor is attached to the equipped weapon.
    pub suppressor_attached: bool,
    /// Whether the weapon's bipod is deployed.
    pub bipod_deployed: bool,
}

/// Hotbar widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct HotbarState {
    /// 8 slots in display order.
    pub slots: [HotbarSlot; HOTBAR_SLOTS],
    /// Index of the currently-active slot.
    pub active_slot: u8,
    /// Whether the hotbar is rendered (false during photo / killcam mode).
    pub visible: bool,
}

impl Default for HotbarState {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|i| HotbarSlot {
                index: i as u8,
                ..HotbarSlot::default()
            }),
            active_slot: 0,
            visible: true,
        }
    }
}

impl HotbarState {
    /// Set the active slot (clamped to `HOTBAR_SLOTS - 1`).
    pub fn set_active(&mut self, slot: u8) {
        self.active_slot = (slot as usize).min(HOTBAR_SLOTS - 1) as u8;
    }

    /// Update one slot's contents in place.
    pub fn set_slot(&mut self, slot: u8, contents: HotbarSlot) {
        let idx = (slot as usize).min(HOTBAR_SLOTS - 1);
        self.slots[idx] = HotbarSlot {
            index: slot,
            ..contents
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_8_slots() {
        let s = HotbarState::default();
        assert_eq!(s.slots.len(), HOTBAR_SLOTS);
        assert_eq!(s.active_slot, 0);
    }

    #[test]
    fn set_active_clamps() {
        let mut s = HotbarState::default();
        s.set_active(99);
        assert_eq!(s.active_slot, (HOTBAR_SLOTS - 1) as u8);
    }

    #[test]
    fn set_slot_writes_in_place() {
        let mut s = HotbarState::default();
        s.set_slot(
            3,
            HotbarSlot {
                index: 3,
                label: Some("Rifle".into()),
                ammo_count: Some(30),
                suppressor_attached: true,
                bipod_deployed: false,
            },
        );
        assert_eq!(s.slots[3].label.as_deref(), Some("Rifle"));
        assert!(s.slots[3].suppressor_attached);
    }
}
