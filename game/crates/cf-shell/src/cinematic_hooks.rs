//! **M12C** § Mission-load / mission-success / campaign-end → cinematic
//! kernel handoff. Per spec § Crates / modules touched:
//!
//! > `cf-shell` (MODIFY) — Hooks: mission-load → opening cinematic;
//! > mission-success → between-mission; campaign-end → ending.
//!
//! Per spec § Notes for the implementer:
//!
//! > Mission-opening cinematic boot path: `cf-shell::cinematic_hooks
//! > ::on_mission_load` reads `<mission_id>.cinematic.ron` if it exists;
//! > if missing, skips silently — never block mission boot on a missing
//! > cinematic file.
//! >
//! > Between-mission cinematics SHOULD use already-loaded base-scene
//! > actors; do NOT spawn new entities for cinematic-only purposes
//! > (perf budget: Steam Deck floor, 60 fps).
//!
//! This module owns the *resolution* layer — it picks the correct RON
//! script path + storyteller variant + narration track for the
//! triggering event — and emits a `CinematicHookRequest` event the
//! cf-app binary translates into an `engage_cinematic_kernel` call on
//! the engine.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Source classification matching the cf-replay schema enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CinematicHookSource {
    /// Mission opens.
    Opening,
    /// Between-mission monologue.
    Between,
    /// Campaign ends.
    Ending,
}

impl CinematicHookSource {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CinematicHookSource::Opening => "opening",
            CinematicHookSource::Between => "between",
            CinematicHookSource::Ending => "ending",
        }
    }
}

/// One resolved hook request — the cf-app binary translates this into
/// an `engage_cinematic_kernel` engine call.
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicHookRequest {
    /// Cinematic id (matches the on-disk RON filename stem).
    pub cinematic_id: String,
    /// Where this hook originated (mission boot / between-mission /
    /// campaign-end).
    pub source: CinematicHookSource,
    /// Filesystem path to the RON script.
    pub script_path: PathBuf,
    /// Filesystem path to the narration WAV / JSON, when present.
    /// `None` = no narration; the kernel plays silently.
    pub narration_track_id: Option<String>,
    /// Storyteller selection (`None` = the active storyteller from
    /// `Settings.gameplay.storyteller`).
    pub storyteller_id: Option<String>,
}

/// **M12C** § "Between-mission cinematic plays once per campaign-day".
/// Stable token tracked by `ShellState` so the second visit to base
/// before mission select skips the between-mission cinematic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BetweenMissionPlayedToday(pub bool);

/// Per spec § "Mission-opening cinematic boot path …". Resolves the
/// RON path under `content/cinematics/opening/<mission_id>.cinematic.ron`.
/// Returns `None` when the file is missing — the boot path skips
/// silently per the spec contract.
#[must_use]
pub fn on_mission_load(content_root: &Path, mission_id: &str) -> Option<CinematicHookRequest> {
    let script_path = content_root
        .join("cinematics")
        .join("opening")
        .join(format!("{mission_id}.cinematic.ron"));
    if !script_path.exists() {
        return None;
    }
    Some(CinematicHookRequest {
        cinematic_id: mission_id.to_string(),
        source: CinematicHookSource::Opening,
        script_path,
        narration_track_id: Some(mission_id.to_string()),
        storyteller_id: None,
    })
}

/// Per spec § "Between-mission cinematic plays once per campaign-day".
/// Resolves the between-mission RON path. Returns `None` when:
///
/// - The file is missing (no monologue authored for the storyteller).
/// - The between-mission cinematic already played this campaign-day.
///
/// `between_played_today` should be flipped to `true` after the kernel
/// engages.
#[must_use]
pub fn on_mission_success(
    content_root: &Path,
    storyteller_id: &str,
    variant_index: u32,
    between_played_today: BetweenMissionPlayedToday,
) -> Option<CinematicHookRequest> {
    if between_played_today.0 {
        return None;
    }
    // Per spec § "Cassandra delivers a dread monologue / Phoebe a
    // quirky one / Randy chaotic / Ironman a challenge / Sandbox
    // NONE (skipped; instant transition)". The Sandbox path resolves
    // via the kernel's `suppress_cinematics` flag — the hook still
    // resolves the RON because the kernel emits a parity-replay
    // `cinematic.skipped { reason: sandbox_suppressed }` event.
    let id = format!("{}_v{}", storyteller_id, variant_index);
    let script_path = content_root
        .join("cinematics")
        .join("between")
        .join(format!("{id}.cinematic.ron"));
    if !script_path.exists() {
        return None;
    }
    Some(CinematicHookRequest {
        cinematic_id: id.clone(),
        source: CinematicHookSource::Between,
        script_path,
        narration_track_id: Some(id),
        storyteller_id: Some(storyteller_id.to_string()),
    })
}

/// Per spec § "Campaign-ending cinematic is 2-5 minutes with 3-act
/// structure / 5 storyteller-specific finales / Cassandra's reads as
/// elegy; Phoebe's as quiet hope; Randy's as a cackling shrug; Ironman's
/// as a salute; Sandbox skips Acts 1-2 and runs only Act 3 painted
/// slides."
#[must_use]
pub fn on_campaign_end(content_root: &Path, storyteller_id: &str) -> Option<CinematicHookRequest> {
    let script_path = content_root
        .join("cinematics")
        .join("ending")
        .join(format!("{storyteller_id}.cinematic.ron"));
    if !script_path.exists() {
        return None;
    }
    Some(CinematicHookRequest {
        cinematic_id: storyteller_id.to_string(),
        source: CinematicHookSource::Ending,
        script_path,
        narration_track_id: Some(storyteller_id.to_string()),
        storyteller_id: Some(storyteller_id.to_string()),
    })
}

/// Per spec § "the per-storyteller stinger picks a variant from
/// `content/cinematics/opening_stingers/<storyteller_id>.ron`."
///
/// Reads the RON stinger table for `storyteller_id`. Returns the parsed
/// bytes (NOT validated here — the caller passes them to
/// `cf_cinematic::StingerTable::from_ron` which validates + parses).
/// Returns `None` when the file is missing.
#[must_use]
pub fn read_opening_stinger_table(content_root: &Path, storyteller_id: &str) -> Option<Vec<u8>> {
    let path = content_root
        .join("cinematics")
        .join("opening_stingers")
        .join(format!("{storyteller_id}.ron"));
    fs::read(&path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "(stub)").unwrap();
    }

    #[test]
    fn on_mission_load_returns_none_when_file_missing() {
        let dir = temp_root();
        let req = on_mission_load(dir.path(), "missing_mission");
        assert!(req.is_none());
    }

    #[test]
    fn on_mission_load_returns_request_when_file_present() {
        let dir = temp_root();
        touch(dir.path(), "cinematics/opening/cin_intro.cinematic.ron");
        let req = on_mission_load(dir.path(), "cin_intro").expect("present");
        assert_eq!(req.source, CinematicHookSource::Opening);
        assert_eq!(req.cinematic_id, "cin_intro");
    }

    #[test]
    fn on_mission_success_returns_none_when_already_played_today() {
        let dir = temp_root();
        touch(
            dir.path(),
            "cinematics/between/cassandra_classic_v0.cinematic.ron",
        );
        let req = on_mission_success(
            dir.path(),
            "cassandra_classic",
            0,
            BetweenMissionPlayedToday(true),
        );
        assert!(req.is_none(), "already-played-today skips between cinematic");
    }

    #[test]
    fn on_mission_success_resolves_storyteller_variant() {
        let dir = temp_root();
        touch(
            dir.path(),
            "cinematics/between/randy_random_v2.cinematic.ron",
        );
        let req = on_mission_success(
            dir.path(),
            "randy_random",
            2,
            BetweenMissionPlayedToday(false),
        )
        .expect("present");
        assert_eq!(req.cinematic_id, "randy_random_v2");
        assert_eq!(req.source, CinematicHookSource::Between);
        assert_eq!(req.storyteller_id.as_deref(), Some("randy_random"));
    }

    #[test]
    fn on_campaign_end_resolves_per_storyteller_ending() {
        let dir = temp_root();
        touch(
            dir.path(),
            "cinematics/ending/cassandra_classic.cinematic.ron",
        );
        let req = on_campaign_end(dir.path(), "cassandra_classic").expect("present");
        assert_eq!(req.source, CinematicHookSource::Ending);
        assert_eq!(req.cinematic_id, "cassandra_classic");
    }
}
