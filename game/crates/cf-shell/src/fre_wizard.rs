//! First-Run-Experience wizard polish — 6-step state machine.
//!
//! Per spec: Welcome → Profile → Accessibility calibration → Controller
//! calibration → Tutorial offer → Starter world recommendation.

use crate::state::FreStep;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FreProfile {
    pub display_name: String,
    pub preferred_origin: String,
    pub preferred_storyteller: String,
    pub accessibility_text_scale: f32,
    pub accessibility_high_contrast: bool,
    pub accessibility_reduce_motion: bool,
    pub accessibility_captions: String,
    pub controller_input_profile: String,
    pub tutorial_offered_accepted: bool,
    pub starter_world_id: String,
}

impl FreProfile {
    pub fn defaults() -> Self {
        Self {
            display_name: "Player".to_string(),
            preferred_origin: "human_baseline".to_string(),
            preferred_storyteller: "cassandra_classic".to_string(),
            accessibility_text_scale: 1.0,
            accessibility_high_contrast: false,
            accessibility_reduce_motion: false,
            accessibility_captions: "Standard".to_string(),
            controller_input_profile: "auto".to_string(),
            tutorial_offered_accepted: true,
            starter_world_id: "earth".to_string(),
        }
    }
}

/// Recommend starter world based on preferred origin.
pub fn recommend_starter_world(origin: &str) -> &'static str {
    match origin {
        "aqueous_kindred" => "europa",
        "methane_breather" => "mimas",
        "crystalline_helios" => "sol_zone",
        "heavy_biomech" => "vulcan",
        "vacuum_adapted" => "moon",
        "insectoid_swarm" => "deimos",
        "robotic_drone" => "orbital",
        "silica_xenofauna" => "venus",
        "android_synthetic" => "mars",
        _ => "earth",
    }
}

/// Get the SVG mockup for the current FRE step.
pub fn fre_screen_asset(step: FreStep) -> &'static str {
    match step {
        FreStep::Welcome => "screen_fre_welcome",
        FreStep::Profile => "screen_fre_profile",
        FreStep::AccessibilityCalibration => "screen_fre_accessibility_calibration",
        FreStep::ControllerCalibration => "screen_fre_accessibility_calibration",
        FreStep::TutorialOffer => "screen_fre_welcome",
        FreStep::StarterWorldRecommendation => "screen_fre_profile",
    }
}

/// Compute the progress percentage of the FRE wizard at a given step.
pub fn fre_progress_pct(step: FreStep) -> u32 {
    let i = step.step_index();
    let total = FreStep::step_count();
    (i * 100) / total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fre_step_advance_chain() {
        let mut step = FreStep::Welcome;
        let mut steps_visited = vec![step];
        while let Some(next) = step.next() {
            step = next;
            steps_visited.push(step);
        }
        assert_eq!(steps_visited.len(), 6);
        assert_eq!(steps_visited[0], FreStep::Welcome);
        assert_eq!(steps_visited[5], FreStep::StarterWorldRecommendation);
    }

    #[test]
    fn fre_step_back_chain() {
        let mut step = FreStep::StarterWorldRecommendation;
        let mut steps_visited = vec![step];
        while let Some(prev) = step.prev() {
            step = prev;
            steps_visited.push(step);
        }
        assert_eq!(steps_visited.len(), 6);
        assert_eq!(steps_visited.last().unwrap(), &FreStep::Welcome);
    }

    #[test]
    fn recommend_per_origin() {
        assert_eq!(recommend_starter_world("aqueous_kindred"), "europa");
        assert_eq!(recommend_starter_world("methane_breather"), "mimas");
        assert_eq!(recommend_starter_world("crystalline_helios"), "sol_zone");
        assert_eq!(recommend_starter_world("human_baseline"), "earth");
        assert_eq!(recommend_starter_world("unknown_origin"), "earth");
    }

    #[test]
    fn fre_progress_pct_advances() {
        assert_eq!(fre_progress_pct(FreStep::Welcome), 16);
        assert_eq!(fre_progress_pct(FreStep::Profile), 33);
        assert_eq!(fre_progress_pct(FreStep::StarterWorldRecommendation), 100);
    }

    #[test]
    fn fre_screen_asset_per_step() {
        assert_eq!(fre_screen_asset(FreStep::Welcome), "screen_fre_welcome");
        assert_eq!(fre_screen_asset(FreStep::Profile), "screen_fre_profile");
        assert_eq!(fre_screen_asset(FreStep::AccessibilityCalibration), "screen_fre_accessibility_calibration");
    }

    #[test]
    fn fre_profile_defaults_safe() {
        let p = FreProfile::defaults();
        assert_eq!(p.display_name, "Player");
        assert_eq!(p.accessibility_text_scale, 1.0);
        assert!(!p.accessibility_high_contrast);
        assert!(!p.accessibility_reduce_motion);
    }
}
