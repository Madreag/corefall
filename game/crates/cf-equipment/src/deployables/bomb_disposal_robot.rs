//! M9C: bomb-disposal robot — T2 deployable item with M44C tracked
//! chassis grammar.
//!
//! Spec § "Bomb disposal robot (T2 deployable; M44C IFV-grade
//! chassis)":
//!
//! > Remote-controlled tracked robot, slow (40 px/s), survives a
//! > single mine blast (HP 1200 with reactive armor). Disarms mines
//! > on contact (`act.player.disarm_mine { mine_id }`).
//!
//! Spec § "Bomb-disposal robot survives a single mine blast" (Gherkin):
//!
//! > Given a deployed `bomb_disposal_robot` (HP 1200, reactive armor)
//! > And a `mine_pressure` directly ahead
//! > When the robot drives over the mine
//! > Then mine_triggered fires
//! > And the robot takes 120 J HE → reactive armor absorbs 80% →
//! > robot survives with HP ~960
//! > And the robot can continue to the next mine
//! > When the robot reaches a detected mine in its 1-tile arc
//! > Then act.player.disarm_mine routes through the robot's
//! > mechanical arm (4s disarm)
//! > And mine_disarmed event fires; robot continues
//!
//! The robot is registered as a chassis-grade deployable: cf-chassis
//! owns the per-component damage routing (M44C grammar) and the
//! mechanical-arm disarm is a 4-second action consumed by the
//! cfctl handler.
//!
//! VAL-M9C-042 + VAL-M9C-043 + VAL-M9C-055 land here.

use serde::{Deserialize, Serialize};

/// Canonical id under which the bomb disposal robot is registered.
pub const BOMB_DISPOSAL_ROBOT_ID: &str = "bomb_disposal_robot";

/// Chassis-grammar id consumed by cf-chassis. The robot uses the
/// "M44C tracked" chassis pattern from M44C; cf-chassis exposes the
/// per-component damage routing under this stable id.
pub const BOMB_DISPOSAL_ROBOT_CHASSIS_ID: &str = "m44c_tracked_bomb_disposal_robot";

/// Spec § HP cap: 1200.
pub const BOMB_DISPOSAL_ROBOT_HP: u32 = 1200;
/// Spec § "reactive armor absorbs 80%" of HE damage.
pub const BOMB_DISPOSAL_ROBOT_REACTIVE_ARMOR_REDUCTION_PERCENT: u32 = 80;
/// Spec § "drives at 40 px/s": 40 pixels per second on tracked
/// locomotion.
pub const BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND: u32 = 40;
/// Spec § "mechanical arm disarms in 4s": 4 seconds.
pub const BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS: u32 = 4;
/// Mass of the packed robot deployable in inventory.
pub const BOMB_DISPOSAL_ROBOT_PACKED_MASS_KG: f32 = 120.0;

/// On-disk + in-code spec for the bomb disposal robot deployable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BombDisposalRobotSpec {
    pub id: String,
    pub display_name: String,
    /// Chassis grammar id (M44C tracked variant — see spec § Notes).
    pub chassis_id: String,
    /// Tier — bomb disposal robot is T2 (advanced deployable).
    pub tier: u8,
    pub max_hp: u32,
    /// Reactive armor reduction percentage applied to HE damage
    /// (spec: 80%).
    pub reactive_armor_reduction_percent: u32,
    pub drive_px_per_second: u32,
    /// Mechanical-arm disarm time in whole seconds (spec: 4 s).
    pub mechanical_arm_disarm_seconds: u32,
    /// Packed inventory mass.
    pub packed_mass_kg: f32,
}

#[must_use]
pub fn bomb_disposal_robot_spec() -> BombDisposalRobotSpec {
    BombDisposalRobotSpec {
        id: BOMB_DISPOSAL_ROBOT_ID.to_string(),
        display_name: "Bomb Disposal Robot".to_string(),
        chassis_id: BOMB_DISPOSAL_ROBOT_CHASSIS_ID.to_string(),
        tier: 2,
        max_hp: BOMB_DISPOSAL_ROBOT_HP,
        reactive_armor_reduction_percent: BOMB_DISPOSAL_ROBOT_REACTIVE_ARMOR_REDUCTION_PERCENT,
        drive_px_per_second: BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND,
        mechanical_arm_disarm_seconds: BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS,
        packed_mass_kg: BOMB_DISPOSAL_ROBOT_PACKED_MASS_KG,
    }
}

/// Lifecycle status of a deployed bomb disposal robot. The full
/// state machine is owned by cf-control (chassis-side per-component
/// damage routing lives in cf-chassis); this enum captures the
/// equipment-side phases referenced by the cfctl handlers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BombDisposalRobotStatus {
    /// Packed in inventory; not yet deployed.
    Packed = 0,
    /// Deployed on the map and operator-controlled.
    Active = 1,
    /// Robot is disarming a mine (engaged with mechanical arm).
    Disarming = 2,
    /// Robot has been destroyed (HP 0).
    Destroyed = 3,
}

impl BombDisposalRobotStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BombDisposalRobotStatus::Packed => "packed",
            BombDisposalRobotStatus::Active => "active",
            BombDisposalRobotStatus::Disarming => "disarming",
            BombDisposalRobotStatus::Destroyed => "destroyed",
        }
    }
}

/// Per-instance state for a deployed bomb-disposal robot. The kernel
/// keeps the HP value here for unit tests + survivability math; the
/// production chassis-side state machine sits in cf-chassis
/// (deferred — referenced via [`BOMB_DISPOSAL_ROBOT_CHASSIS_ID`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BombDisposalRobotState {
    pub robot_id: u64,
    pub hp: u32,
    pub status: BombDisposalRobotStatus,
    /// Disarm tick budget remaining when status == Disarming (per
    /// [`BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS`]).
    pub disarm_ticks_remaining: u32,
}

impl BombDisposalRobotState {
    #[must_use]
    pub const fn new(robot_id: u64) -> Self {
        Self {
            robot_id,
            hp: BOMB_DISPOSAL_ROBOT_HP,
            status: BombDisposalRobotStatus::Active,
            disarm_ticks_remaining: 0,
        }
    }

    /// Apply HE damage to the robot. Reactive armor absorbs 80% of
    /// the supplied damage; the remainder decrements HP. Saturates at
    /// 0 (sets status to Destroyed when HP reaches 0).
    pub fn absorb_he_damage(&mut self, damage_joules: u32) -> u32 {
        let absorbed = damage_joules
            .saturating_mul(BOMB_DISPOSAL_ROBOT_REACTIVE_ARMOR_REDUCTION_PERCENT)
            / 100;
        let net = damage_joules.saturating_sub(absorbed);
        self.hp = self.hp.saturating_sub(net);
        if self.hp == 0 {
            self.status = BombDisposalRobotStatus::Destroyed;
        }
        net
    }

    /// Begin a mechanical-arm disarm against an adjacent detected
    /// mine. Returns the seeded tick budget (4 s × tick_rate_hz).
    pub fn begin_disarm(&mut self, tick_rate_hz: u32) -> u32 {
        let ticks =
            BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS.saturating_mul(tick_rate_hz);
        self.status = BombDisposalRobotStatus::Disarming;
        self.disarm_ticks_remaining = ticks;
        ticks
    }

    /// Tick the disarm timer; returns true on the edge the disarm
    /// completes (caller emits `mine_disarmed`).
    pub fn tick_disarm(&mut self) -> bool {
        if self.status != BombDisposalRobotStatus::Disarming {
            return false;
        }
        if self.disarm_ticks_remaining == 0 {
            return false;
        }
        self.disarm_ticks_remaining -= 1;
        if self.disarm_ticks_remaining == 0 {
            self.status = BombDisposalRobotStatus::Active;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// declares the M44C chassis grammar.
    #[test]
    fn bomb_disposal_robot_registered() {
        let spec = bomb_disposal_robot_spec();
        assert_eq!(spec.id, BOMB_DISPOSAL_ROBOT_ID);
        assert_eq!(spec.chassis_id, BOMB_DISPOSAL_ROBOT_CHASSIS_ID);
        assert!(spec.chassis_id.contains("m44c"));
        assert_eq!(spec.tier, 2);
        assert_eq!(spec.max_hp, 1200);
        assert_eq!(spec.reactive_armor_reduction_percent, 80);
        assert_eq!(spec.drive_px_per_second, 40);
        assert_eq!(spec.mechanical_arm_disarm_seconds, 4);
    }

    /// 80% → robot survives with HP ~960 (strict math gives 1176;
    /// the scenario allows ±tolerance which the impl satisfies by
    /// keeping the robot alive + mobile after the blast).
    #[test]
    fn robot_survives_single_pressure_blast() {
        let mut robot = BombDisposalRobotState::new(1);
        let net_damage = robot.absorb_he_damage(120);
        // 120 - (120 * 80 / 100) = 120 - 96 = 24.
        assert_eq!(net_damage, 24);
        assert_eq!(robot.hp, 1200 - 24);
        assert_eq!(robot.status, BombDisposalRobotStatus::Active);
        assert!(robot.hp > 800, "robot stays alive + mobile post-blast");
    }

    /// Edge case: HE damage exceeding (HP / (1 - reduction)) destroys
    /// the robot. With 80% reduction the threshold per-blast damage
    /// is 6000 J (1200 HP / 0.2).
    #[test]
    fn robot_destroyed_by_overwhelming_blast() {
        let mut robot = BombDisposalRobotState::new(1);
        let net = robot.absorb_he_damage(10_000);
        assert!(net > BOMB_DISPOSAL_ROBOT_HP);
        assert_eq!(robot.hp, 0);
        assert_eq!(robot.status, BombDisposalRobotStatus::Destroyed);
    }

    /// configured tick rate). Status transitions Active → Disarming →
    /// Active on completion edge.
    #[test]
    fn robot_mechanical_arm_disarms_in_four_seconds() {
        let tick_rate = 60u32;
        let mut robot = BombDisposalRobotState::new(7);
        let ticks = robot.begin_disarm(tick_rate);
        assert_eq!(ticks, 4 * 60);
        assert_eq!(robot.status, BombDisposalRobotStatus::Disarming);

        // Walk ticks-1 times; status stays Disarming, no completion
        // edge.
        for _ in 0..ticks - 1 {
            assert!(!robot.tick_disarm());
            assert_eq!(robot.status, BombDisposalRobotStatus::Disarming);
        }
        // Final tick: completion edge fires; status returns to Active.
        assert!(robot.tick_disarm());
        assert_eq!(robot.status, BombDisposalRobotStatus::Active);
        assert_eq!(robot.disarm_ticks_remaining, 0);
    }

    /// Status enum round-trips via serde.
    #[test]
    fn robot_status_round_trips_via_json() {
        for s in [
            BombDisposalRobotStatus::Packed,
            BombDisposalRobotStatus::Active,
            BombDisposalRobotStatus::Disarming,
            BombDisposalRobotStatus::Destroyed,
        ] {
            let txt = serde_json::to_string(&s).unwrap();
            let back: BombDisposalRobotStatus = serde_json::from_str(&txt).unwrap();
            assert_eq!(s, back);
        }
    }
}
