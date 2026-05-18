//! **M14A** § "cf-ui::walk_strip" — sprite walk-cycle strip preview for the
//! HUD overlay (debug + tutorial). Exposes the stride sprite frame index for
//! consumers that render small live walk-cycle previews.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WalkStripHud {
    pub frame: u8,
    pub stride_frame_just_fired: bool,
    pub stride_timer_ms: u32,
    pub move_state: u8,
}

impl WalkStripHud {
    pub fn from_observe_payload(payload: &serde_json::Value) -> Self {
        let stride_frame = payload
            .get("stride_frame")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let timer = payload.get("stride_timer_ms").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let move_state_s = payload.get("move_state").and_then(|v| v.as_str()).unwrap_or("stand");
        let move_state = match move_state_s {
            "no_move" => 0,
            "stand" => 1,
            "walk" => 2,
            "crouch" => 3,
            "crawl" => 4,
            "arm_crawl" => 5,
            "climb" => 6,
            "jump" => 7,
            "dislodge" => 8,
            "hover" => 9,
            _ => 1,
        };
        Self {
            frame: ((timer / 100) % 6) as u8,
            stride_frame_just_fired: stride_frame,
            stride_timer_ms: timer,
            move_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_stride_state() {
        let h = WalkStripHud::from_observe_payload(&json!({
            "stride_frame": true,
            "stride_timer_ms": 250,
            "move_state": "walk"
        }));
        assert!(h.stride_frame_just_fired);
        assert_eq!(h.move_state, 2);
    }
}
