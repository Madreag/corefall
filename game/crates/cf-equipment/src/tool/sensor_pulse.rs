//! M6: Sensor pulse — reveal enemies + hazards in radius for 5 s.

use super::{ToolKind, ToolPreset, SENSOR_PULSE_M6_DEFAULT_ID};

/// Spec § "Sensor pulse reveals enemies + hazards in radius for 5s".
pub const SENSOR_PULSE_REVEAL_SECONDS: f32 = 5.0;

/// Spec § "Sensor pulse reveals enemies + hazards in radius": 96 world units.
pub const SENSOR_PULSE_REVEAL_RADIUS: f32 = 96.0;

#[must_use]
pub fn sensor_pulse_m6_default() -> ToolPreset {
    ToolPreset {
        id: SENSOR_PULSE_M6_DEFAULT_ID.to_string(),
        display_name: "Sensor Pulse".to_string(),
        kind: ToolKind::SensorPulse,
        wear_per_use: 12.5,
        max_durability: 100.0,
        radius: 96.0,
        spawns_material_id: String::new(),
        heat_generating: false,
        reveals_enemies: true,
        persistent_marker: false,
        mass_kg: 1.2,
    }
}
