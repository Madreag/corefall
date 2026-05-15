//! F5 — Physics impulses overlay. Force vectors for actors with recent
//! impulses per spec § Debug overlays.

use serde::{Deserialize, Serialize};

/// One impulse arrow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImpulseVector {
    /// Source actor id.
    pub actor_id: u64,
    /// Origin world position (arrow tail).
    pub origin: (f32, f32),
    /// Vector components (arrow direction + length scaled by magnitude).
    pub vector: (f32, f32),
    /// Tick at which this impulse landed (renderer fades over time).
    pub at_tick: u64,
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PhysicsOverlayData {
    /// All impulse arrows to draw this frame.
    pub impulses: Vec<ImpulseVector>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_round_trips() {
        let i = ImpulseVector {
            actor_id: 5,
            origin: (10.0, 0.0),
            vector: (0.0, -100.0),
            at_tick: 42,
        };
        let s = serde_json::to_string(&i).unwrap();
        let back: ImpulseVector = serde_json::from_str(&s).unwrap();
        assert_eq!(i, back);
    }
}
