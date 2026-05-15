//! Q-hold context wheel — 8-direction radial menu of context-appropriate
//! orders for the actor under the reticle. Spec § Q-hold context wheel.

use serde::{Deserialize, Serialize};

/// 8 wheel slots (radial menu).
pub const WHEEL_SLOTS_LEN: usize = 8;

/// What kind of entity is under the reticle when the wheel opens. Drives
/// the slot population per spec table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ReticleTarget {
    /// A friendly squadmate.
    Squadmate {
        /// Squadmate actor id.
        actor_id: u64,
    },
    /// A door interactable.
    Door {
        /// Door entity id.
        entity_id: u64,
        /// Whether the player has a grenade equipped (extends slot set).
        grenade_equipped: bool,
    },
    /// An enemy / hostile.
    Enemy {
        /// Enemy actor id.
        actor_id: u64,
    },
    /// A damaged terrain breach tile.
    TerrainBreach {
        /// World position of the breach.
        position: (f32, f32),
    },
    /// A hazard tile.
    Hazard {
        /// Hazard id.
        hazard_id: u64,
    },
    /// A reactor / chassis module.
    ReactorModule {
        /// Module id.
        module_id: u64,
    },
    /// Nothing in view — defaults to general squad orders.
    None,
}

/// Context-order kinds the wheel can issue. Spec lists 25+ across the
/// per-target tables; the enum covers every spec literal so wheels can be
/// composed without string literals.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ContextOrderKind {
    MoveTo,
    Engage,
    Heal,
    Repair,
    Smoke,
    Suppress,
    Hold,
    FallBack,
    FollowMe,
    HoldPosition,
    TreatMe,
    CoverAngle,
    StayWithTarget,
    AcknowledgeOrder,
    StackLeft,
    StackRight,
    AutoStack,
    BreachKick,
    BreachC2,
    BreachFlash,
    BreachSting,
    BreachGas,
    Open,
    MirrorUnder,
    Wedge,
    FocusFire,
    EngageFree,
    HoldFire,
    Flank,
    DigHere,
    Fill,
    RepairWall,
    Anchor,
    Mark,
    Avoid,
    SuppressSpread,
    CounterTreat,
    Cool,
    PowerDown,
    Override,
    Spearhead,
    DefendBrain,
    HoldHere,
    TriageAll,
    RepairAll,
}

impl ContextOrderKind {
    /// Canonical snake_case identifier (cfctl wire form).
    pub fn as_str(self) -> &'static str {
        match self {
            ContextOrderKind::MoveTo => "move_to",
            ContextOrderKind::Engage => "engage",
            ContextOrderKind::Heal => "heal",
            ContextOrderKind::Repair => "repair",
            ContextOrderKind::Smoke => "smoke",
            ContextOrderKind::Suppress => "suppress",
            ContextOrderKind::Hold => "hold",
            ContextOrderKind::FallBack => "fall_back",
            ContextOrderKind::FollowMe => "follow_me",
            ContextOrderKind::HoldPosition => "hold_position",
            ContextOrderKind::TreatMe => "treat_me",
            ContextOrderKind::CoverAngle => "cover_angle",
            ContextOrderKind::StayWithTarget => "stay_with_target",
            ContextOrderKind::AcknowledgeOrder => "acknowledge_order",
            ContextOrderKind::StackLeft => "stack_left",
            ContextOrderKind::StackRight => "stack_right",
            ContextOrderKind::AutoStack => "auto_stack",
            ContextOrderKind::BreachKick => "breach_kick",
            ContextOrderKind::BreachC2 => "breach_c2",
            ContextOrderKind::BreachFlash => "breach_flash",
            ContextOrderKind::BreachSting => "breach_sting",
            ContextOrderKind::BreachGas => "breach_gas",
            ContextOrderKind::Open => "open",
            ContextOrderKind::MirrorUnder => "mirror_under",
            ContextOrderKind::Wedge => "wedge",
            ContextOrderKind::FocusFire => "focus_fire",
            ContextOrderKind::EngageFree => "engage_free",
            ContextOrderKind::HoldFire => "hold_fire",
            ContextOrderKind::Flank => "flank",
            ContextOrderKind::DigHere => "dig_here",
            ContextOrderKind::Fill => "fill",
            ContextOrderKind::RepairWall => "repair_wall",
            ContextOrderKind::Anchor => "anchor",
            ContextOrderKind::Mark => "mark",
            ContextOrderKind::Avoid => "avoid",
            ContextOrderKind::SuppressSpread => "suppress_spread",
            ContextOrderKind::CounterTreat => "counter_treat",
            ContextOrderKind::Cool => "cool",
            ContextOrderKind::PowerDown => "power_down",
            ContextOrderKind::Override => "override",
            ContextOrderKind::Spearhead => "spearhead",
            ContextOrderKind::DefendBrain => "defend_brain",
            ContextOrderKind::HoldHere => "hold_here",
            ContextOrderKind::TriageAll => "triage_all",
            ContextOrderKind::RepairAll => "repair_all",
        }
    }

    /// Parse from cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ContextOrderKind> {
        Some(match value {
            "move_to" => ContextOrderKind::MoveTo,
            "engage" => ContextOrderKind::Engage,
            "heal" => ContextOrderKind::Heal,
            "repair" => ContextOrderKind::Repair,
            "smoke" => ContextOrderKind::Smoke,
            "suppress" => ContextOrderKind::Suppress,
            "hold" => ContextOrderKind::Hold,
            "fall_back" => ContextOrderKind::FallBack,
            "follow_me" => ContextOrderKind::FollowMe,
            "hold_position" => ContextOrderKind::HoldPosition,
            "treat_me" => ContextOrderKind::TreatMe,
            "cover_angle" => ContextOrderKind::CoverAngle,
            "stay_with_target" => ContextOrderKind::StayWithTarget,
            "acknowledge_order" => ContextOrderKind::AcknowledgeOrder,
            "stack_left" => ContextOrderKind::StackLeft,
            "stack_right" => ContextOrderKind::StackRight,
            "auto_stack" => ContextOrderKind::AutoStack,
            "breach_kick" => ContextOrderKind::BreachKick,
            "breach_c2" => ContextOrderKind::BreachC2,
            "breach_flash" => ContextOrderKind::BreachFlash,
            "breach_sting" => ContextOrderKind::BreachSting,
            "breach_gas" => ContextOrderKind::BreachGas,
            "open" => ContextOrderKind::Open,
            "mirror_under" => ContextOrderKind::MirrorUnder,
            "wedge" => ContextOrderKind::Wedge,
            "focus_fire" => ContextOrderKind::FocusFire,
            "engage_free" => ContextOrderKind::EngageFree,
            "hold_fire" => ContextOrderKind::HoldFire,
            "flank" => ContextOrderKind::Flank,
            "dig_here" => ContextOrderKind::DigHere,
            "fill" => ContextOrderKind::Fill,
            "repair_wall" => ContextOrderKind::RepairWall,
            "anchor" => ContextOrderKind::Anchor,
            "mark" => ContextOrderKind::Mark,
            "avoid" => ContextOrderKind::Avoid,
            "suppress_spread" => ContextOrderKind::SuppressSpread,
            "counter_treat" => ContextOrderKind::CounterTreat,
            "cool" => ContextOrderKind::Cool,
            "power_down" => ContextOrderKind::PowerDown,
            "override" => ContextOrderKind::Override,
            "spearhead" => ContextOrderKind::Spearhead,
            "defend_brain" => ContextOrderKind::DefendBrain,
            "hold_here" => ContextOrderKind::HoldHere,
            "triage_all" => ContextOrderKind::TriageAll,
            "repair_all" => ContextOrderKind::RepairAll,
            _ => return None,
        })
    }
}

/// One slot of the radial wheel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WheelSlot {
    /// Slot index (0..8 starting at the 12-o'clock and rotating clockwise).
    pub index: u8,
    /// The order this slot issues.
    pub order: ContextOrderKind,
    /// Player-facing label (in lieu of an icon for accessibility).
    pub label: String,
    /// Whether this slot is selectable in the current context.
    pub enabled: bool,
}

impl WheelSlot {
    /// Build an enabled wheel slot using the order's canonical label.
    pub fn enabled_at(index: u8, order: ContextOrderKind) -> Self {
        Self {
            index,
            order,
            label: order.as_str().replace('_', " "),
            enabled: true,
        }
    }
}

/// Full radial wheel + the target it was opened on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextWheel {
    /// What was under the reticle when the wheel opened.
    pub target: ReticleTarget,
    /// 8 slots around the wheel; unused slots are disabled placeholders.
    pub slots: [WheelSlot; WHEEL_SLOTS_LEN],
}

/// Build the per-target wheel populated per spec § Q-hold context wheel
/// table.
pub fn context_wheel_for(target: ReticleTarget) -> ContextWheel {
    let orders: [ContextOrderKind; WHEEL_SLOTS_LEN] = match &target {
        ReticleTarget::Squadmate { .. } => [
            ContextOrderKind::FollowMe,
            ContextOrderKind::HoldPosition,
            ContextOrderKind::TreatMe,
            ContextOrderKind::CoverAngle,
            ContextOrderKind::StayWithTarget,
            ContextOrderKind::AcknowledgeOrder,
            ContextOrderKind::Heal,
            ContextOrderKind::Repair,
        ],
        ReticleTarget::Door { grenade_equipped, .. } => {
            if *grenade_equipped {
                [
                    ContextOrderKind::StackLeft,
                    ContextOrderKind::StackRight,
                    ContextOrderKind::AutoStack,
                    ContextOrderKind::BreachKick,
                    ContextOrderKind::BreachC2,
                    ContextOrderKind::BreachFlash,
                    ContextOrderKind::BreachSting,
                    ContextOrderKind::BreachGas,
                ]
            } else {
                [
                    ContextOrderKind::StackLeft,
                    ContextOrderKind::StackRight,
                    ContextOrderKind::AutoStack,
                    ContextOrderKind::BreachKick,
                    ContextOrderKind::BreachC2,
                    ContextOrderKind::Open,
                    ContextOrderKind::MirrorUnder,
                    ContextOrderKind::Wedge,
                ]
            }
        }
        ReticleTarget::Enemy { .. } => [
            ContextOrderKind::FocusFire,
            ContextOrderKind::Suppress,
            ContextOrderKind::EngageFree,
            ContextOrderKind::HoldFire,
            ContextOrderKind::Flank,
            ContextOrderKind::Smoke,
            ContextOrderKind::Mark,
            ContextOrderKind::Avoid,
        ],
        ReticleTarget::TerrainBreach { .. } => [
            ContextOrderKind::DigHere,
            ContextOrderKind::Fill,
            ContextOrderKind::RepairWall,
            ContextOrderKind::Anchor,
            ContextOrderKind::Mark,
            ContextOrderKind::Suppress,
            ContextOrderKind::FallBack,
            ContextOrderKind::HoldHere,
        ],
        ReticleTarget::Hazard { .. } => [
            ContextOrderKind::Mark,
            ContextOrderKind::Avoid,
            ContextOrderKind::SuppressSpread,
            ContextOrderKind::CounterTreat,
            ContextOrderKind::FallBack,
            ContextOrderKind::Smoke,
            ContextOrderKind::Cool,
            ContextOrderKind::Override,
        ],
        ReticleTarget::ReactorModule { .. } => [
            ContextOrderKind::Repair,
            ContextOrderKind::Cool,
            ContextOrderKind::PowerDown,
            ContextOrderKind::Override,
            ContextOrderKind::DefendBrain,
            ContextOrderKind::Anchor,
            ContextOrderKind::Mark,
            ContextOrderKind::HoldHere,
        ],
        ReticleTarget::None => [
            ContextOrderKind::Spearhead,
            ContextOrderKind::DefendBrain,
            ContextOrderKind::HoldHere,
            ContextOrderKind::FallBack,
            ContextOrderKind::EngageFree,
            ContextOrderKind::HoldFire,
            ContextOrderKind::TriageAll,
            ContextOrderKind::RepairAll,
        ],
    };
    let slots: [WheelSlot; WHEEL_SLOTS_LEN] = std::array::from_fn(|i| WheelSlot::enabled_at(i as u8, orders[i]));
    ContextWheel { target, slots }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squadmate_wheel_includes_follow_me() {
        let w = context_wheel_for(ReticleTarget::Squadmate { actor_id: 1 });
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::FollowMe));
    }

    #[test]
    fn door_wheel_with_grenade_includes_breach_flash() {
        let w = context_wheel_for(ReticleTarget::Door {
            entity_id: 9,
            grenade_equipped: true,
        });
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::BreachFlash));
    }

    #[test]
    fn door_wheel_without_grenade_includes_open() {
        let w = context_wheel_for(ReticleTarget::Door {
            entity_id: 9,
            grenade_equipped: false,
        });
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::Open));
    }

    #[test]
    fn enemy_wheel_includes_focus_fire() {
        let w = context_wheel_for(ReticleTarget::Enemy { actor_id: 1 });
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::FocusFire));
    }

    #[test]
    fn none_wheel_includes_spearhead_and_triage_all() {
        let w = context_wheel_for(ReticleTarget::None);
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::Spearhead));
        assert!(w.slots.iter().any(|s| s.order == ContextOrderKind::TriageAll));
    }

    #[test]
    fn order_kind_round_trips() {
        for k in [
            ContextOrderKind::MoveTo,
            ContextOrderKind::Heal,
            ContextOrderKind::BreachFlash,
            ContextOrderKind::TriageAll,
        ] {
            assert_eq!(ContextOrderKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn wheel_slots_length_is_8() {
        let w = context_wheel_for(ReticleTarget::None);
        assert_eq!(w.slots.len(), WHEEL_SLOTS_LEN);
    }
}
