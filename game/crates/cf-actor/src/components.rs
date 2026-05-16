//! M8A § Files / cf-actor — ECS component scaffold.
//!
//! M8A's parallel-determinism refactor migrates the monolithic
//! `ActorState` into discrete ECS components: `Pos`, `Vel`, `Aim`,
//! `Stance`, `Status`, `Hp`, `Stability`, `Stamina`, `Lean`, `Cover`,
//! `StealthMeter`, `LimbLossFlags`, `InventorySlot`. The component types
//! live here so Bevy ECS (M9+ engine-host integration) can wrap them via
//! `#[derive(Component)]` newtypes in cf-app / cf-render-2d without
//! pulling Bevy into the determinism-locked cf-actor crate.
//!
//! See `docs/plan/spec/determinism-island-contract.md` § M8A extensions
//! for the snapshot-read / compute-parallel / commit-serial pattern this
//! scaffold supports.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vel {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Aim {
    pub angle_rad: f32,
    pub sharp: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Hp {
    pub current: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StabilityComponent {
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StaminaComponent {
    pub value: f32,
    pub regen_per_tick: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LeanComponent {
    pub radians: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CoverComponent {
    pub cover_level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StealthMeterComponent {
    pub raw: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct LimbLossFlags {
    pub left_arm: bool,
    pub right_arm: bool,
    pub left_leg: bool,
    pub right_leg: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct InventorySlotComponent {
    pub slot_index: u8,
    pub item_id: u32,
}

/// **M8A**: composite ECS bundle for an actor entity. M9+ engine-host
/// integration wraps this with Bevy's `#[derive(Bundle)]`; M8A keeps the
/// crate determinism-locked by exposing a plain Rust struct.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ActorBundle {
    pub pos: Pos,
    pub vel: Vel,
    pub aim: Aim,
    pub hp: Hp,
    pub stability: StabilityComponent,
    pub stamina: StaminaComponent,
    pub lean: LeanComponent,
    pub cover: CoverComponent,
    pub stealth: StealthMeterComponent,
    pub limb_loss: LimbLossFlags,
    pub inventory_slot: InventorySlotComponent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_bundle_default_round_trip() {
        let bundle = ActorBundle::default();
        let s = serde_json::to_string(&bundle).unwrap();
        let back: ActorBundle = serde_json::from_str(&s).unwrap();
        assert_eq!(bundle, back);
    }
}
