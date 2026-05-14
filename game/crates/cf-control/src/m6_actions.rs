//! M6 cfctl actions — the 30+ new methods that extend M1's 9-action controller.
//!
//! Each method registered in `server.rs` constructs an [`M6Action`] and
//! dispatches via [`crate::server::ControlCommand::ActM6`]. The engine
//! consumes the action and updates `ActorState` flags + emits replay events.

use serde::{Deserialize, Serialize};

/// Sticky / edge action discriminator for the M6 cfctl surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action_kind", rename_all = "snake_case")]
pub enum M6Action {
    /// Toggle sprint (sticky).
    Sprint { active: bool },
    /// Toggle prone stance (sticky).
    Prone { active: bool },
    /// Edge-trigger slide (only valid from Sprint).
    Slide,
    /// Edge-trigger vault over low cover.
    Vault,
    /// Edge-trigger ledge climb up.
    ClimbUp,
    /// Edge-trigger ledge climb down.
    ClimbDown,
    /// Edge-trigger evasive dive.
    Dive,
    /// Sticky lean direction (`-1.0` left, `+1.0` right, `0.0` none).
    Lean { direction: f32 },
    /// Edge-trigger stealth kill on the actor in front + within reach.
    StealthKill,
    /// Edge-trigger knife throw (uses equipped melee).
    KnifeThrow,
    /// Edge-trigger weapon swap to a specific slot (0..7).
    WeaponSwap { slot: u8 },
    /// Edge-trigger drop currently selected inventory slot.
    DropItem { slot: Option<u8> },
    /// Edge-trigger pickup the nearest dropped item.
    Pickup,
    /// Edge-trigger emit a "friendly here" radio chirp.
    SignalFriendly,
    /// Edge-trigger emit a "spotted hostile" radio chirp.
    SignalEnemySpotted,
    /// Edge-trigger drop a persistent map waypoint.
    MarkWaypoint { x: f32, y: f32 },
    /// Edge-trigger deploy bipod (must be crouched/prone).
    DeployBipod,
    /// Edge-trigger stow bipod.
    StowBipod,
    /// Edge-trigger cycle weapon fire mode.
    CycleFireMode,
    /// Edge-trigger cook grenade (extra fuse depletion before throw).
    CookGrenade,
    /// Edge-trigger throw the equipped grenade.
    ThrowGrenade,
    /// Edge-trigger rifle bash / equipped melee swing.
    MeleeBash,
    /// Edge-trigger close-range kick.
    MeleeKick,
    /// Edge-trigger use the equipped tool by id.
    UseTool { tool_kind: String },
    /// Edge-trigger attach a suppressor to the currently selected weapon.
    AttachSuppressor,
    /// Edge-trigger detach the suppressor from the currently selected weapon.
    DetachSuppressor,
    /// Edge-trigger set side-view facing direction explicitly (debug/cfctl).
    SetFacing { facing: String },
}

impl M6Action {
    /// Cfctl method string for this action (used by the engine to record the
    /// `control.command_accepted` event).
    pub fn method_name(&self) -> &'static str {
        match self {
            M6Action::Sprint { .. } => "act.player.sprint",
            M6Action::Prone { .. } => "act.player.prone",
            M6Action::Slide => "act.player.slide",
            M6Action::Vault => "act.player.vault",
            M6Action::ClimbUp => "act.player.climb_up",
            M6Action::ClimbDown => "act.player.climb_down",
            M6Action::Dive => "act.player.dive",
            M6Action::Lean { .. } => "act.player.lean",
            M6Action::StealthKill => "act.player.stealth_kill",
            M6Action::KnifeThrow => "act.player.knife_throw",
            M6Action::WeaponSwap { .. } => "act.player.weapon_swap",
            M6Action::DropItem { .. } => "act.player.drop_item",
            M6Action::Pickup => "act.player.pickup",
            M6Action::SignalFriendly => "act.player.signal_friendly",
            M6Action::SignalEnemySpotted => "act.player.signal_enemy_spotted",
            M6Action::MarkWaypoint { .. } => "act.player.mark_waypoint",
            M6Action::DeployBipod => "act.player.deploy_bipod",
            M6Action::StowBipod => "act.player.stow_bipod",
            M6Action::CycleFireMode => "act.player.cycle_fire_mode",
            M6Action::CookGrenade => "act.player.cook_grenade",
            M6Action::ThrowGrenade => "act.player.throw_grenade",
            M6Action::MeleeBash => "act.player.melee_bash",
            M6Action::MeleeKick => "act.player.melee_kick",
            M6Action::UseTool { .. } => "act.player.use_tool",
            M6Action::AttachSuppressor => "act.player.attach_suppressor",
            M6Action::DetachSuppressor => "act.player.detach_suppressor",
            M6Action::SetFacing { .. } => "act.player.set_facing",
        }
    }
}

/// Per-action params for the cfctl envelope. Wraps [`M6Action`] with an
/// optional `schema_version` so JSON-RPC schema-guard logic stays consistent
/// with M1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M6ActionParams {
    pub action: M6Action,
}

/// Squad-command action (separate from M6Action because it's `act.squad.*`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadCommandKindOverWire {
    FollowLeader,
    HoldPosition,
    DefendPoint,
    PushToWaypoint,
}

impl SquadCommandKindOverWire {
    pub fn as_str(&self) -> &'static str {
        match self {
            SquadCommandKindOverWire::FollowLeader => "follow_leader",
            SquadCommandKindOverWire::HoldPosition => "hold_position",
            SquadCommandKindOverWire::DefendPoint => "defend_point",
            SquadCommandKindOverWire::PushToWaypoint => "push_to_waypoint",
        }
    }

    pub fn requires_waypoint(&self) -> bool {
        matches!(
            self,
            SquadCommandKindOverWire::DefendPoint | SquadCommandKindOverWire::PushToWaypoint
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActSquadIssueCommandParams {
    pub bot_actor: Option<u64>,
    pub kind: SquadCommandKindOverWire,
    pub waypoint: Option<(f32, f32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names_are_unique() {
        let actions = [
            M6Action::Sprint { active: true },
            M6Action::Prone { active: true },
            M6Action::Slide,
            M6Action::Vault,
            M6Action::ClimbUp,
            M6Action::ClimbDown,
            M6Action::Dive,
            M6Action::Lean { direction: 0.0 },
            M6Action::StealthKill,
            M6Action::KnifeThrow,
            M6Action::WeaponSwap { slot: 0 },
            M6Action::DropItem { slot: None },
            M6Action::Pickup,
            M6Action::SignalFriendly,
            M6Action::SignalEnemySpotted,
            M6Action::MarkWaypoint { x: 0.0, y: 0.0 },
            M6Action::DeployBipod,
            M6Action::StowBipod,
            M6Action::CycleFireMode,
            M6Action::CookGrenade,
            M6Action::ThrowGrenade,
            M6Action::MeleeBash,
            M6Action::MeleeKick,
            M6Action::UseTool {
                tool_kind: "drill".into(),
            },
            M6Action::AttachSuppressor,
            M6Action::DetachSuppressor,
            M6Action::SetFacing { facing: "right".into() },
        ];
        let names: std::collections::BTreeSet<&str> = actions.iter().map(M6Action::method_name).collect();
        assert_eq!(names.len(), actions.len());
    }

    #[test]
    fn squad_requires_waypoint_known() {
        assert!(SquadCommandKindOverWire::DefendPoint.requires_waypoint());
        assert!(SquadCommandKindOverWire::PushToWaypoint.requires_waypoint());
        assert!(!SquadCommandKindOverWire::FollowLeader.requires_waypoint());
    }
}
