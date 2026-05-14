//! M6: Drill — 2× faster than digger, generates heat (jams when overheat).

use super::{ToolKind, ToolPreset, DRILL_M6_DEFAULT_ID};

/// Drill heat threshold (0..1) above which it jams.
pub const DRILL_JAM_HEAT_THRESHOLD: f32 = 0.85;
/// Per-use heat gain (0..1).
pub const DRILL_HEAT_PER_USE: f32 = 0.04;
/// Per-second heat gain while continuously drilling (0..1 per s).
/// Mirrors the spec's "DRILL_HEAT_RATE_PER_S" constant referenced from the
/// engine's use_tool dispatcher.
pub const DRILL_HEAT_RATE_PER_S: f32 = 0.25;
/// Per-tick heat decay when idle.
pub const DRILL_HEAT_DECAY_PER_S: f32 = 0.15;

#[must_use]
pub fn drill_m6_default() -> ToolPreset {
    ToolPreset {
        id: DRILL_M6_DEFAULT_ID.to_string(),
        display_name: "Mining Drill".to_string(),
        kind: ToolKind::Drill,
        wear_per_use: 1.0,
        max_durability: 120.0,
        radius: 6.0,
        spawns_material_id: String::new(),
        heat_generating: true,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 4.5,
    }
}
