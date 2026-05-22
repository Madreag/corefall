//! **M4B § "Quicksave + quickload roundtrip beats 800 ms"** + § "system
//! save events" — cf-control save subcommand surface.
//!
//! Owns the in-memory `LastSaveMetadata` projection (returned by
//! `observe.save.last`) + the engine-side wiring for cfctl save
//! subcommands. The actual filesystem write goes through
//! [`cf_save::quicksave`]; the event recording goes through the cf-replay
//! recorder so any save / load / migrate operation surfaces in the run
//! bundle.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use cf_replay::Recorder;
use cf_save::{
    quicksave::{read_quicksave, write_quicksave, QuickloadOutcome, QuicksaveOutcome},
    SaveError, WorldSave,
};
use cf_sim_core::Tick;
use serde::{Deserialize, Serialize};

/// Snapshot of the last completed save operation. Returned verbatim by the
/// `observe.save.last` surface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LastSaveMetadata {
    pub schema_version: u32,
    /// Absolute path to the last quicksave on disk.
    pub path: Option<String>,
    /// SaveSchemaVersion the last save was written under, formatted as
    /// `[major, minor, patch]`.
    pub save_schema_version: Option<[u16; 3]>,
    /// Canonical-JSON BLAKE3 of the last save.
    pub blake3: Option<String>,
    /// On-disk size in bytes.
    pub size_bytes: Option<u64>,
    /// Wall-clock duration of the last save operation, in milliseconds.
    pub wall_clock_ms: Option<u32>,
    /// Operation kind: "save" / "load" / "migrate" / "autosave".
    pub last_operation: Option<String>,
}

impl LastSaveMetadata {
    pub fn fresh() -> Self {
        Self {
            schema_version: 1,
            ..Self::default()
        }
    }
}

/// In-memory cache shared between the engine + the JSON-RPC surface.
#[derive(Debug, Default)]
pub struct LastSaveCache(Mutex<LastSaveMetadata>);

impl LastSaveCache {
    pub fn new() -> Self {
        Self(Mutex::new(LastSaveMetadata::fresh()))
    }

    pub fn snapshot(&self) -> LastSaveMetadata {
        self.0.lock().expect("last-save mutex poisoned").clone()
    }

    pub fn record_save(&self, outcome: &QuicksaveOutcome, save: &WorldSave) {
        let mut guard = self.0.lock().expect("last-save mutex poisoned");
        guard.path = Some(outcome.path.display().to_string());
        guard.save_schema_version = Some(version_tuple(save));
        guard.blake3 = Some(outcome.checksum_hex.clone());
        guard.size_bytes = Some(outcome.bytes_written);
        guard.wall_clock_ms = Some(outcome.wall_clock_ms);
        guard.last_operation = Some("save".to_string());
    }

    pub fn record_load(&self, path: &Path, outcome: &QuickloadOutcome) {
        let mut guard = self.0.lock().expect("last-save mutex poisoned");
        guard.path = Some(path.display().to_string());
        guard.save_schema_version = Some(version_tuple(&outcome.save));
        guard.blake3 = Some(outcome.checksum_hex.clone());
        guard.size_bytes = Some(0);
        guard.wall_clock_ms = Some(outcome.wall_clock_ms);
        guard.last_operation = Some(if outcome.migrated_from.is_some() {
            "migrate".to_string()
        } else {
            "load".to_string()
        });
    }
}

fn version_tuple(save: &WorldSave) -> [u16; 3] {
    [
        save.schema_version.major,
        save.schema_version.minor,
        save.schema_version.patch,
    ]
}

/// autosave interval in seconds (engine clock, NOT wall clock per the
/// spec Notes). The actual tick interval is derived per-run from
/// `tick_rate_hz` so the 60-second contract holds at any tick rate.
pub const AUTOSAVE_INTERVAL_SECONDS: u64 = 60;

/// Convenience constant for the 60 Hz default (most scenarios). 60 s ×
/// 60 ticks/s = 3600 ticks. cf-app + cfctl that override tick_rate_hz
/// call [`autosave_interval_ticks`] with the right rate.
pub const AUTOSAVE_INTERVAL_TICKS: u64 = AUTOSAVE_INTERVAL_SECONDS * 60;

/// clock, not wall clock; this preserves determinism for
/// replay-against-autosave testing." This helper converts the 60-second
/// interval into the per-tick-rate tick count so the timer fires at the
/// same wall-clock cadence regardless of the configured tick_rate_hz.
pub fn autosave_interval_ticks(tick_rate_hz: u32) -> u64 {
    AUTOSAVE_INTERVAL_SECONDS * u64::from(tick_rate_hz.max(1))
}

/// True when the autosave timer has elapsed under the supplied tick rate.
pub fn autosave_due_at_rate(last_autosave_tick: u64, current_tick: u64, tick_rate_hz: u32) -> bool {
    current_tick >= last_autosave_tick.saturating_add(autosave_interval_ticks(tick_rate_hz))
}

/// **Backwards-compat** convenience for 60 Hz callers. New callers should
/// prefer [`autosave_due_at_rate`] so the 60-second contract holds at
/// non-60Hz tick rates.
pub fn autosave_due(last_autosave_tick: u64, current_tick: u64) -> bool {
    autosave_due_at_rate(last_autosave_tick, current_tick, 60)
}

/// Perform a quicksave and emit the `system.save_completed` event into
/// the supplied recorder.
pub fn fire_quicksave(
    recorder: &Recorder,
    tick: Tick,
    sim_time_ms: f64,
    cache: &LastSaveCache,
    dir: &Path,
    save: &WorldSave,
    kind: &str,
) -> Result<QuicksaveOutcome, SaveError> {
    let outcome = write_quicksave(dir, save)?;
    cache.record_save(&outcome, save);
    let payload = serde_json::json!({
        "kind": kind,
        "path": outcome.path.display().to_string(),
        "blake3": outcome.checksum_hex,
        "size_bytes": outcome.bytes_written,
        "wall_clock_ms": outcome.wall_clock_ms,
        "save_schema_version": version_tuple(save),
    });
    recorder.record(tick, sim_time_ms, "system", "save_completed", payload, None);
    Ok(outcome)
}

/// Perform a quickload and emit the `system.save_loaded` event. When the
/// load triggered a migration, ALSO emit `system.save_migrated`.
pub fn fire_quickload(
    recorder: &Recorder,
    tick: Tick,
    sim_time_ms: f64,
    cache: &LastSaveCache,
    dir: &Path,
) -> Result<QuickloadOutcome, SaveError> {
    let outcome = read_quicksave(dir)?;
    cache.record_load(&dir.join("quicksave.cfsave"), &outcome);
    let payload = serde_json::json!({
        "path": dir.join("quicksave.cfsave").display().to_string(),
        "blake3": outcome.checksum_hex,
        "wall_clock_ms": outcome.wall_clock_ms,
        "save_schema_version": version_tuple(&outcome.save),
    });
    recorder.record(tick, sim_time_ms, "system", "save_loaded", payload, None);
    if let (Some(from), Some(to)) = (outcome.migrated_from, outcome.migrated_to) {
        let migrated_payload = serde_json::json!({
            "from": [from.major, from.minor, from.patch],
            "to": [to.major, to.minor, to.patch],
            "handler_chain": outcome.handler_chain,
        });
        recorder.record(tick, sim_time_ms, "system", "save_migrated", migrated_payload, None);
    }
    Ok(outcome)
}

/// Standalone migrate-only path: load `<path>/quicksave.cfsave`, run the
/// migration registry, write back under `<path>/quicksave.cfsave`. Used by
/// `cfctl save migrate <path> --to <version>` and `cf-headless save
/// migrate`.
pub fn fire_migrate(
    recorder: &Recorder,
    tick: Tick,
    sim_time_ms: f64,
    cache: &LastSaveCache,
    dir: &PathBuf,
) -> Result<QuickloadOutcome, SaveError> {
    let outcome = read_quicksave(dir)?;
    // Persist back to disk so the migrated form is canonical.
    let write = write_quicksave(dir, &outcome.save)?;
    cache.record_save(&write, &outcome.save);
    if let (Some(from), Some(to)) = (outcome.migrated_from, outcome.migrated_to) {
        let payload = serde_json::json!({
            "from": [from.major, from.minor, from.patch],
            "to": [to.major, to.minor, to.patch],
            "handler_chain": outcome.handler_chain,
            "path": write.path.display().to_string(),
        });
        recorder.record(tick, sim_time_ms, "system", "save_migrated", payload, None);
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_save::{quicksave::QUICKSAVE_FILE, WorldSave};
    use tempfile::tempdir;

    #[test]
    fn fire_quicksave_then_fire_quickload_round_trips_through_cache() {
        let dir = tempdir().unwrap();
        let recorder = Recorder::new("m4b_save_cache_test".to_string());
        let cache = LastSaveCache::new();
        let save = WorldSave::empty(120);
        let out = fire_quicksave(&recorder, Tick(0), 0.0, &cache, dir.path(), &save, "quicksave").unwrap();
        assert!(dir.path().join(QUICKSAVE_FILE).exists());
        let snapshot = cache.snapshot();
        assert_eq!(snapshot.last_operation.as_deref(), Some("save"));
        assert_eq!(snapshot.blake3, Some(out.checksum_hex.clone()));
        let load_out = fire_quickload(&recorder, Tick(1), 16.6, &cache, dir.path()).unwrap();
        assert_eq!(load_out.save, save);
        let snapshot = cache.snapshot();
        assert_eq!(snapshot.last_operation.as_deref(), Some("load"));
    }

    #[test]
    fn autosave_due_fires_after_one_minute_interval_at_60hz() {
        assert!(!autosave_due(0, 60 * 60 - 1));
        assert!(autosave_due(0, 60 * 60));
        assert!(autosave_due(0, 60 * 60 + 1));
    }

    /// the same 60-second interval is 7200 ticks, not 3600.
    #[test]
    fn autosave_due_honors_tick_rate_hz() {
        // 60 Hz path.
        assert!(autosave_due_at_rate(0, 3600, 60));
        assert!(!autosave_due_at_rate(0, 3599, 60));
        // 120 Hz path.
        assert!(autosave_due_at_rate(0, 7200, 120));
        assert!(!autosave_due_at_rate(0, 7199, 120));
        // 30 Hz path.
        assert!(autosave_due_at_rate(0, 1800, 30));
        assert!(!autosave_due_at_rate(0, 1799, 30));
    }

    #[test]
    fn autosave_interval_ticks_clamps_zero_tick_rate_to_one() {
        // Defensive: 0 → 1 to avoid divide-by-zero / always-elapsed.
        assert_eq!(autosave_interval_ticks(0), AUTOSAVE_INTERVAL_SECONDS);
        assert_eq!(autosave_interval_ticks(60), AUTOSAVE_INTERVAL_SECONDS * 60);
    }

    #[test]
    fn last_save_cache_starts_fresh_with_schema_version_1() {
        let c = LastSaveCache::new();
        let s = c.snapshot();
        assert_eq!(s.schema_version, 1);
        assert!(s.path.is_none());
        assert!(s.blake3.is_none());
    }
}
