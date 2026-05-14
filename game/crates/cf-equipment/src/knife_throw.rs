//! M6: knife throw + retrieve.
//!
//! Spec § "Knife throw: throw equipped melee as projectile (50% damage; can
//! be retrieved from wall)".

use serde::{Deserialize, Serialize};

/// Damage scalar applied to thrown-knife hits (50% of melee dmg per spec).
pub const KNIFE_THROW_DAMAGE_FACTOR: f32 = 0.5;
/// Maximum knife projectile flight time (seconds).
pub const KNIFE_THROW_MAX_FLIGHT_SECONDS: f32 = 2.5;

/// Knife throw lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KnifeThrowState {
    #[default]
    Idle = 0,
    InFlight = 1,
    StuckInWall = 2,
    StuckInActor = 3,
    Retrieved = 4,
}

impl KnifeThrowState {
    pub fn as_str(self) -> &'static str {
        match self {
            KnifeThrowState::Idle => "idle",
            KnifeThrowState::InFlight => "in_flight",
            KnifeThrowState::StuckInWall => "stuck_in_wall",
            KnifeThrowState::StuckInActor => "stuck_in_actor",
            KnifeThrowState::Retrieved => "retrieved",
        }
    }
}

/// One thrown-knife projectile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KnifeProjectile {
    pub projectile_id: u64,
    pub owner_actor: u64,
    pub origin_x: f32,
    pub origin_y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub damage: f32,
    pub remaining_seconds: f32,
    pub state: KnifeThrowState,
}

impl KnifeProjectile {
    pub fn new(projectile_id: u64, owner_actor: u64, origin: (f32, f32), aim: (f32, f32), base_damage: f32) -> Self {
        let speed = 800.0;
        let aim_len = (aim.0 * aim.0 + aim.1 * aim.1).sqrt().max(1e-6);
        let vx = (aim.0 / aim_len) * speed;
        let vy = (aim.1 / aim_len) * speed;
        Self {
            projectile_id,
            owner_actor,
            origin_x: origin.0,
            origin_y: origin.1,
            velocity_x: vx,
            velocity_y: vy,
            damage: base_damage * KNIFE_THROW_DAMAGE_FACTOR,
            remaining_seconds: KNIFE_THROW_MAX_FLIGHT_SECONDS,
            state: KnifeThrowState::InFlight,
        }
    }

    /// Mark as stuck in wall (retrievable by approaching + pressing E).
    pub fn stick_in_wall(&mut self) {
        self.state = KnifeThrowState::StuckInWall;
        self.velocity_x = 0.0;
        self.velocity_y = 0.0;
    }

    pub fn stick_in_actor(&mut self) {
        self.state = KnifeThrowState::StuckInActor;
        self.velocity_x = 0.0;
        self.velocity_y = 0.0;
    }

    /// True if retrievable by player walking up + pressing E.
    pub fn is_retrievable(self) -> bool {
        self.state == KnifeThrowState::StuckInWall
    }

    pub fn retrieve(&mut self) -> bool {
        if !self.is_retrievable() {
            return false;
        }
        self.state = KnifeThrowState::Retrieved;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_is_50_percent_of_base() {
        let k = KnifeProjectile::new(1, 1, (0.0, 0.0), (1.0, 0.0), 20.0);
        assert!((k.damage - 10.0).abs() < 1e-3);
    }

    #[test]
    fn stuck_in_wall_is_retrievable() {
        let mut k = KnifeProjectile::new(1, 1, (0.0, 0.0), (1.0, 0.0), 20.0);
        k.stick_in_wall();
        assert!(k.is_retrievable());
        assert!(k.retrieve());
        assert_eq!(k.state, KnifeThrowState::Retrieved);
    }

    #[test]
    fn stuck_in_actor_not_retrievable() {
        let mut k = KnifeProjectile::new(1, 1, (0.0, 0.0), (1.0, 0.0), 20.0);
        k.stick_in_actor();
        assert!(!k.is_retrievable());
    }
}
