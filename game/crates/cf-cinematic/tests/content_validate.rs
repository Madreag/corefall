//! **M12C** content-validation integration test.  Verifies every
//! authored cinematic RON file under `game/content/cinematics/` parses,
//! validates per the script-loader contract, and lands inside the
//! source's required duration window (30-60s opening / 15-30s between
//! / 120-300s ending).

use std::fs;
use std::path::{Path, PathBuf};

use cf_cinematic::{CinematicScript, NarrationTrack, ScriptSource};

fn workspace_root() -> PathBuf {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_root
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("workspace root resolves")
}

fn content_dir() -> PathBuf {
    workspace_root().join("content").join("cinematics")
}

fn iter_ron_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.ends_with(".cinematic.ron"))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_opening_cinematic_parses_and_validates_duration() {
    let dir = content_dir().join("opening");
    let files = iter_ron_files(&dir);
    assert!(
        files.len() >= 30,
        "spec § '30+ launch missions each have a <mission_id>.cinematic.ron script': found {} (< 30)",
        files.len()
    );
    for path in files {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let script = CinematicScript::from_ron(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        assert_eq!(script.source, ScriptSource::Opening, "{}", path.display());
        let total = script.total_duration_ms();
        assert!(
            (30_000..=60_000).contains(&total),
            "{}: opening duration {} outside [30000,60000]",
            path.display(),
            total
        );
    }
}

#[test]
fn every_between_cinematic_parses_and_validates_duration() {
    let dir = content_dir().join("between");
    let files = iter_ron_files(&dir);
    assert!(
        files.len() >= 15,
        "spec § '5 storytellers × 3 variants = 15': found {} (< 15)",
        files.len()
    );
    for path in files {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let script = CinematicScript::from_ron(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        assert_eq!(script.source, ScriptSource::Between, "{}", path.display());
        let total = script.total_duration_ms();
        assert!(
            (15_000..=30_000).contains(&total),
            "{}: between duration {} outside [15000,30000]",
            path.display(),
            total
        );
    }
}

#[test]
fn every_ending_cinematic_parses_and_validates_duration() {
    let dir = content_dir().join("ending");
    let files = iter_ron_files(&dir);
    assert_eq!(
        files.len(),
        5,
        "spec § '5 storyteller-specific finales': expected 5 ending files, got {}",
        files.len()
    );
    for path in files {
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let script = CinematicScript::from_ron(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        assert_eq!(script.source, ScriptSource::Ending, "{}", path.display());
        let total = script.total_duration_ms();
        assert!(
            (120_000..=300_000).contains(&total),
            "{}: ending duration {} outside [120000,300000]",
            path.display(),
            total
        );
    }
}

#[test]
fn every_narration_track_parses_if_present() {
    let narration_root = workspace_root()
        .join("content")
        .join("audio")
        .join("voice")
        .join("cinematic");
    if !narration_root.exists() {
        return;
    }
    for entry in fs::read_dir(&narration_root).unwrap() {
        let path = entry.unwrap().path();
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        if !name.ends_with(".narration_track.json") {
            continue;
        }
        let bytes = fs::read(&path).expect("read narration");
        let _track =
            NarrationTrack::from_json(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    }
}
