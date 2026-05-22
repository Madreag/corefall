use serde::{Deserialize, Serialize};

use crate::{ArmorMountAngles, BodyGraph, ChassisKind, ChassisModule, ZoneState};

/// Chassis spec — the immutable design data for one archetype. Scenarios reference
/// these by id; the runtime clones to a [`crate::ChassisState`] for each actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisSpec {
    pub id: String,
    pub kind: ChassisKind,
    pub display_name: String,
    pub body_graph: BodyGraph,
    pub zones: Vec<ZoneState>,
    pub modules: Vec<ChassisModule>,
    /// Default eject window length (ticks). 60 ticks at 60 Hz = 1 second window.
    /// At 120 Hz this becomes 120 ticks, preserving real-time semantics.
    pub eject_window_seconds: f32,
    /// Tick rate this spec was instantiated with (used to size [`crate::EjectWindow`]).
    /// Independent of the runtime tick rate; resolved on insertion.
    pub mass_kg: f32,
    /// armor mount angles drive M9 angled-armor math.
    #[serde(default)]
    pub armor_angles: ArmorMountAngles,
}
