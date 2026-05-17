//! M10B angle-selector integration tests.
//!
//! The angle selector is the multi-camera picker the editor displays
//! on the right-hand sidebar; VAL-M10B-035 references it as one of
//! the four required editor surfaces ("timeline scrub + multi-camera
//! angle selector + commentary recorder").

use cf_replay_export::camera_script::CameraKind;
use cf_tools_replay_viewer::edit_cmd::{default_angle_selector, AngleSelector};

/// Default selector seeds three canonical tracks.
#[test]
fn angle_selector_default_seeds_three_canonical_tracks() {
    let sel = default_angle_selector();
    assert_eq!(sel.tracks.len(), 3);
}

/// AngleSelector: selected_kinds() filters to the active set.
#[test]
fn angle_selector_selected_kinds_filters_active_set() {
    let mut sel = AngleSelector::default();
    sel.tracks.iter_mut().for_each(|t| t.selected = false);
    sel.tracks[0].selected = true;
    sel.tracks[2].selected = true;
    let selected: Vec<CameraKind> = sel.selected_kinds().collect();
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&sel.tracks[0].kind));
    assert!(selected.contains(&sel.tracks[2].kind));
}

/// AngleSelector: `to_camera_script` keyframes sit at the declared
/// range boundaries.
#[test]
fn angle_selector_camera_script_keyframes_align_with_ranges() {
    let mut sel = AngleSelector::default();
    let start = sel.tracks[0].range_start_tick;
    let end = sel.tracks[0].range_end_tick;
    sel.tracks.iter_mut().for_each(|t| t.selected = false);
    sel.tracks[0].selected = true;
    let script = sel.to_camera_script();
    assert_eq!(script.tracks.len(), 1);
    let kf_ticks: Vec<u64> = script.tracks[0].keyframes.iter().map(|kf| kf.tick).collect();
    assert!(kf_ticks.contains(&start));
    assert!(kf_ticks.contains(&end.saturating_sub(1)));
}
