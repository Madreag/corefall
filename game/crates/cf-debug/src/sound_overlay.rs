//! F6 — Sound propagation overlay. Loudness ripples from sound sources
//! + occlusion visualization per spec § Debug overlays.

use serde::{Deserialize, Serialize};

/// One loudness ripple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoudnessRipple {
    /// Source actor / hazard / item id.
    pub source_id: u64,
    /// Ripple origin in world units.
    pub origin: (f32, f32),
    /// Loudness radius (world units; renderer fades along it).
    pub radius: f32,
    /// Decibel-equivalent label (e.g. 60 dB).
    pub loudness_db: f32,
}

/// One occlusion segment between source and listener.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcclusionSegment {
    /// Source position.
    pub source: (f32, f32),
    /// Listener position.
    pub listener: (f32, f32),
    /// Occlusion factor in `[0, 1]` (0 = full block, 1 = clear LOS).
    pub occlusion_factor: f32,
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SoundOverlayData {
    /// Loudness ripples to draw this frame.
    pub ripples: Vec<LoudnessRipple>,
    /// Occlusion segments (one per source/listener pair).
    pub occlusions: Vec<OcclusionSegment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripple_round_trips() {
        let r = LoudnessRipple {
            source_id: 11,
            origin: (50.0, 0.0),
            radius: 120.0,
            loudness_db: 75.0,
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: LoudnessRipple = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
