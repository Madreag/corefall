//! VAL-M10B-DEFAULT-PATH: default output path resolves via dirs-next.

use cf_replay_export::default_output_path::{default_output_directory, default_output_path, CORE_FALL_OUTPUT_SUBDIR};

/// VAL-M10B-DEFAULT-PATH: the resolved directory MUST land under
/// the platform's Movies/Videos folder + the `Corefall` subdirectory.
#[test]
fn default_path_per_os() {
    let dir = match default_output_directory() {
        Some(d) => d,
        None => return, // CI host without HOME — skip
    };
    assert_eq!(
        dir.file_name().and_then(|s| s.to_str()),
        Some(CORE_FALL_OUTPUT_SUBDIR),
        "default output directory must terminate in `Corefall` subdir; got {dir:?}"
    );
    let parent_name = dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if cfg!(target_os = "macos") {
        assert_eq!(parent_name, "Movies");
    } else if cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        assert_eq!(parent_name, "Videos");
    }
}

/// VAL-M10B-DEFAULT-PATH: `default_output_path(run_id, ext)` composes
/// `<dir>/<run_id>.<ext>`.
#[test]
fn default_output_path_appends_filename() {
    let Some(dir) = default_output_directory() else {
        return;
    };
    let composed = default_output_path("run_abc", "mp4").expect("compose");
    assert_eq!(composed, dir.join("run_abc.mp4"));
}
