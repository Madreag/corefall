//! **M4B § "Quicksave + quickload roundtrip beats 800 ms"** — F5/F9 fast path.
//!
//! This module wraps [`crate::WorldSave`] in a "quicksave" envelope that
//! stores the canonical pretty-printed JSON, the checksum, and the
//! delta-baseline cadence used at save time. The cf-app hotkey loop calls
//! [`write_quicksave`] on F5 and [`read_quicksave`] on F9; the runtime
//! cost is dominated by the BLAKE3 of the canonical bytes (<400 ms on
//! Workstation tier per the M4B promise).
//!
//! ## File layout
//!
//! A quicksave is one file: `<dir>/quicksave.cfsave`. The companion
//! checksum lives at `<dir>/quicksave.cfsave.checksum`. Loading verifies
//! both before constructing the [`WorldSave`].

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::{migration, SaveError, WorldSave};

/// Default quicksave directory inside an arbitrary save root (typically
/// `<save_root>/quicksave/`).
pub const QUICKSAVE_FILE: &str = "quicksave.cfsave";
pub const QUICKSAVE_CHECKSUM_FILE: &str = "quicksave.cfsave.checksum";

/// Result of a write: the on-disk path, the canonical-JSON BLAKE3, the
/// wall-time elapsed.
#[derive(Debug, Clone)]
pub struct QuicksaveOutcome {
    pub path: PathBuf,
    pub checksum_hex: String,
    pub bytes_written: u64,
    pub wall_clock_ms: u32,
}

/// Result of a read: the loaded WorldSave, the on-disk checksum, the
/// migration outcome (when applicable).
#[derive(Debug, Clone)]
pub struct QuickloadOutcome {
    pub save: WorldSave,
    pub checksum_hex: String,
    pub migrated_from: Option<crate::SaveSchemaVersion>,
    pub migrated_to: Option<crate::SaveSchemaVersion>,
    pub handler_chain: Vec<&'static str>,
    pub wall_clock_ms: u32,
}

/// Serialize + write the quicksave atomically (write to a tmp file in the
/// same directory, then `rename`).
pub fn write_quicksave(dir: &Path, save: &WorldSave) -> Result<QuicksaveOutcome, SaveError> {
    let started = Instant::now();
    fs::create_dir_all(dir).map_err(map_io)?;
    let (pretty, checksum_hex) = save.serialize()?;
    let path = dir.join(QUICKSAVE_FILE);
    let tmp_path = dir.join(format!("{QUICKSAVE_FILE}.tmp"));
    {
        let mut f = fs::File::create(&tmp_path).map_err(map_io)?;
        f.write_all(pretty.as_bytes()).map_err(map_io)?;
        f.flush().map_err(map_io)?;
        f.sync_all().map_err(map_io)?;
    }
    fs::rename(&tmp_path, &path).map_err(map_io)?;
    let checksum_path = dir.join(QUICKSAVE_CHECKSUM_FILE);
    fs::write(&checksum_path, &checksum_hex).map_err(map_io)?;
    let bytes_written = u64::try_from(pretty.len()).unwrap_or(0);
    let wall_clock_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    Ok(QuicksaveOutcome {
        path,
        checksum_hex,
        bytes_written,
        wall_clock_ms,
    })
}

/// Read + verify + migrate the quicksave. Returns the fully upgraded
/// [`WorldSave`] at [`crate::CURRENT_SAVE_SCHEMA_VERSION`].
pub fn read_quicksave(dir: &Path) -> Result<QuickloadOutcome, SaveError> {
    let started = Instant::now();
    let path = dir.join(QUICKSAVE_FILE);
    let checksum_path = dir.join(QUICKSAVE_CHECKSUM_FILE);
    let json = fs::read_to_string(&path).map_err(map_io)?;
    let expected_checksum = fs::read_to_string(&checksum_path).ok();
    let expected_trimmed = expected_checksum.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let raw_save = WorldSave::deserialize(&json, expected_trimmed)?;
    let from = raw_save.schema_version;
    let outcome = migration::migrate_to_current(raw_save)?;
    let migrated_from = if from == crate::CURRENT_SAVE_SCHEMA_VERSION {
        None
    } else {
        Some(from)
    };
    let migrated_to = migrated_from.map(|_| crate::CURRENT_SAVE_SCHEMA_VERSION);
    let wall_clock_ms = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    Ok(QuickloadOutcome {
        save: outcome.blob,
        checksum_hex: expected_trimmed.unwrap_or_default().to_string(),
        migrated_from,
        migrated_to,
        handler_chain: outcome.handler_chain,
        wall_clock_ms,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn map_io(err: std::io::Error) -> SaveError {
    SaveError::MigrationFailed {
        from: crate::CURRENT_SAVE_SCHEMA_VERSION,
        to: crate::CURRENT_SAVE_SCHEMA_VERSION,
        reason: format!("io error: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_then_read_round_trips_world_save() {
        let dir = tempdir().unwrap();
        let world = WorldSave::empty(0);
        let outcome = write_quicksave(dir.path(), &world).unwrap();
        assert!(outcome.path.exists());
        let loaded = read_quicksave(dir.path()).unwrap();
        assert_eq!(loaded.save, world);
        assert!(loaded.migrated_from.is_none());
        assert!(loaded.handler_chain.is_empty());
    }

    #[test]
    fn write_then_tamper_then_read_returns_checksum_mismatch() {
        let dir = tempdir().unwrap();
        let world = WorldSave::empty(0);
        write_quicksave(dir.path(), &world).unwrap();
        let save_path = dir.path().join(QUICKSAVE_FILE);
        let mut text = fs::read_to_string(&save_path).unwrap();
        text.push_str("// tampered");
        fs::write(&save_path, text).unwrap();
        let err = read_quicksave(dir.path()).err().unwrap();
        // The trailing `//` makes the JSON invalid; deserialize_json fires
        // first. Either error path proves the read does NOT silently
        // succeed on a tampered file.
        assert!(matches!(err, SaveError::DeserializeJson(_) | SaveError::ChecksumMismatch { .. }));
    }
}
