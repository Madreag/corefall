//! M8 — Grenade arc preview HUD widget.
//!
//! Per spec § UX widgets: translucent path showing predicted trajectory
//! from hand to impact; updates per aim.

use bevy::prelude::*;

/// One sample point along the predicted arc.
pub type ArcSample = (f32, f32);

/// Grenade arc widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct GrenadeArcState {
    /// Whether the arc is currently visible (player aiming a grenade).
    pub visible: bool,
    /// Sampled arc points in world space (start → impact).
    pub samples: Vec<ArcSample>,
    /// Predicted impact world position.
    pub impact: Option<ArcSample>,
}

impl GrenadeArcState {
    /// Replace the arc with a new sample list.
    pub fn set_arc(&mut self, samples: Vec<ArcSample>, impact: ArcSample) {
        self.samples = samples;
        self.impact = Some(impact);
        self.visible = true;
    }

    /// Hide the arc (player no longer aiming a grenade).
    pub fn hide(&mut self) {
        self.visible = false;
        self.samples.clear();
        self.impact = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_arc_marks_visible() {
        let mut s = GrenadeArcState::default();
        s.set_arc(vec![(0.0, 0.0), (1.0, 1.0)], (5.0, 0.0));
        assert!(s.visible);
        assert_eq!(s.impact, Some((5.0, 0.0)));
    }

    #[test]
    fn hide_clears_state() {
        let mut s = GrenadeArcState::default();
        s.set_arc(vec![(0.0, 0.0)], (1.0, 1.0));
        s.hide();
        assert!(!s.visible);
        assert!(s.samples.is_empty());
        assert!(s.impact.is_none());
    }
}
