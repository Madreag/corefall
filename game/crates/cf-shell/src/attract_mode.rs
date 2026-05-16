//! Attract mode — title-screen background that plays a real M9 reactor
//! defense bundle at 0.5x speed via M40A spectator director.
//!
//! When `Settings.reduce_motion=true`, falls back to a static SVG splash.
//! Per-faction music rotation per session.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttractModeBundle {
    pub bundle_id: String,
    pub scenario_name: String,
    pub playback_speed_factor: f32,
    pub camera_director_preset: String,
    pub looping: bool,
}

impl AttractModeBundle {
    pub fn default_m9_reactor() -> Self {
        Self {
            bundle_id: "attract_m9_reactor_defense_v1".to_string(),
            scenario_name: "Mars Reactor Defense".to_string(),
            playback_speed_factor: 0.5,
            camera_director_preset: "cinematic_wide_to_close_orbit".to_string(),
            looping: true,
        }
    }
}

/// Per-faction music rotation — one of 8 faction themes plays per session.
/// Rotation is deterministic given session-start UTC seconds.
pub fn rotate_faction_music(session_start_utc_secs: u64) -> &'static str {
    let factions = [
        "music_faction_coalition", "music_faction_frontier", "music_faction_ronin",
        "music_faction_synth", "music_faction_collective", "music_faction_husks",
        "music_faction_collegium", "music_faction_starlight",
    ];
    factions[(session_start_utc_secs as usize / 60) % factions.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bundle_uses_m9() {
        let b = AttractModeBundle::default_m9_reactor();
        assert_eq!(b.playback_speed_factor, 0.5);
        assert!(b.looping);
    }

    #[test]
    fn faction_music_deterministic() {
        let m1 = rotate_faction_music(120);
        let m2 = rotate_faction_music(120);
        assert_eq!(m1, m2);
    }

    #[test]
    fn faction_music_rotates() {
        let m1 = rotate_faction_music(0);
        let m2 = rotate_faction_music(60);
        let m3 = rotate_faction_music(120);
        // At least 2 of these should differ (rotation cycle = 60s per faction)
        let arr = [m1, m2, m3];
        let distinct: std::collections::HashSet<_> = arr.iter().collect();
        assert!(distinct.len() >= 2);
    }

    #[test]
    fn faction_music_is_known_id() {
        let m = rotate_faction_music(300);
        assert!(m.starts_with("music_faction_"));
    }
}
