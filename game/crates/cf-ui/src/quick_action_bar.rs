//! **M14A** § "cf-ui::quick_action_bar" — 8-slot always-visible bar widget.

use serde::{Deserialize, Serialize};

/// Per-slot HUD render data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickActionBarSlotHud {
    pub slot_index: u8,
    pub kind: String,
    pub item_id: String,
    pub icon_asset_id: String,
    pub cooldown_fill: f32,
    pub ammo_text: String,
    pub disabled: bool,
    pub highlighted: bool,
}

/// HUD render struct for the 8-slot quick action bar.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickActionBarHud {
    pub slots: [QuickActionBarSlotHud; 8],
    pub last_used_slot: u8,
}

impl QuickActionBarHud {
    pub fn from_observe_payload(payload: &serde_json::Value) -> Self {
        let mut out = Self::default();
        if let Some(slots) = payload.get("bar_slots").and_then(|v| v.as_array()) {
            for (i, s) in slots.iter().take(8).enumerate() {
                out.slots[i] = QuickActionBarSlotHud {
                    slot_index: (i as u8) + 1,
                    kind: s.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    item_id: s.get("item_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    icon_asset_id: format!(
                        "icon.{}",
                        s.get("item_id").and_then(|v| v.as_str()).unwrap_or("empty")
                    ),
                    cooldown_fill: {
                        let r = s.get("cooldown_ticks_remaining").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let t = s.get("cooldown_total_ticks").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        if t > 0.0 {
                            (r / t) as f32
                        } else {
                            0.0
                        }
                    },
                    ammo_text: {
                        let ammo = s.get("ammo").and_then(|v| v.as_u64()).unwrap_or(0);
                        let max = s.get("ammo_max").and_then(|v| v.as_u64()).unwrap_or(0);
                        if max > 0 {
                            format!("{}/{}", ammo, max)
                        } else {
                            String::new()
                        }
                    },
                    disabled: s.get("disabled_by_hazard").and_then(|v| v.as_bool()).unwrap_or(false),
                    highlighted: false,
                };
            }
        }
        out.last_used_slot = payload
            .get("last_used_slot")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_payload_with_slots() {
        let payload = json!({
            "bar_slots": [
                {"slot": 1, "kind": "weapon", "item_id": "rifle", "cooldown_ticks_remaining": 0,
                 "cooldown_total_ticks": 0, "ammo": 24, "ammo_max": 30, "disabled_by_hazard": false}
            ],
            "last_used_slot": 1
        });
        let hud = QuickActionBarHud::from_observe_payload(&payload);
        assert_eq!(hud.slots[0].item_id, "rifle");
        assert_eq!(hud.slots[0].ammo_text, "24/30");
        assert_eq!(hud.last_used_slot, 1);
    }
}
