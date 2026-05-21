use serde::{Deserialize, Serialize};

/// **M13** § "Drone allies — 4 modes + autonomous behavior".
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroneMode {
    #[default]
    Follow = 0,
    AutoMine = 1,
    AutoRepair = 2,
    AutoCarry = 3,
}

impl DroneMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DroneMode::Follow => "follow",
            DroneMode::AutoMine => "auto_mine",
            DroneMode::AutoRepair => "auto_repair",
            DroneMode::AutoCarry => "auto_carry",
        }
    }

    pub fn parse(s: &str) -> Option<DroneMode> {
        match s.to_ascii_lowercase().as_str() {
            "follow" => Some(DroneMode::Follow),
            "auto_mine" | "auto-mine" | "mine" => Some(DroneMode::AutoMine),
            "auto_repair" | "auto-repair" | "repair" => Some(DroneMode::AutoRepair),
            "auto_carry" | "auto-carry" | "carry" => Some(DroneMode::AutoCarry),
            _ => None,
        }
    }
}

/// **M13** § "Drone allies — Drone has limited fuel + battery (drains while
/// active; ~5 minutes per full charge)". Runtime drone ally state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DroneAllyState {
    pub mode: DroneMode,
    /// Fuel level 0..1. Drains roughly 1.0 / 300 s while active.
    pub fuel: f32,
    /// True after at least one `drone.task_completed` event has been emitted.
    #[serde(default)]
    pub task_completed: bool,
    /// True after the drone took terminal damage.
    #[serde(default)]
    pub destroyed: bool,
}

impl Default for DroneAllyState {
    fn default() -> Self {
        Self {
            mode: DroneMode::Follow,
            fuel: 1.0,
            task_completed: false,
            destroyed: false,
        }
    }
}

impl DroneAllyState {
    /// Drain fuel by one tick; returns `true` iff the drone just crossed the
    /// 0.2 low-fuel threshold (emit `drone.fuel_low` once).
    pub fn tick_fuel(&mut self, tick_rate_hz: u32) -> bool {
        if self.destroyed {
            return false;
        }
        let prev = self.fuel;
        // Full charge = 300s (~5 minutes); per-tick drain = 1.0 / (300 * tick_rate).
        let drain = 1.0 / (300.0 * tick_rate_hz.max(1) as f32);
        self.fuel = (self.fuel - drain).max(0.0);
        prev > 0.2 && self.fuel <= 0.2
    }
}
