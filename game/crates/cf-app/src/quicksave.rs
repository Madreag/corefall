//! **M4B § "F5 / F9 hotkeys"** — cf-app integration glue.
//!
//! Bevy systems live in [`cf-app/src/main.rs`]; this module is the
//! deterministic + UI-independent inner loop the system delegates to.
//! Tested directly without spinning up Bevy.

#![allow(dead_code)] // public surface used by F5/F9 system + future HUD wiring.

use std::{path::Path, time::Duration};

use cf_control::m4b_save::LastSaveMetadata;
use cf_save::SaveError;

/// Per-frame state machine for the F5 / F9 hotkeys + the autosave timer.
/// The cf-app Bevy system stamps current Instant, current sim tick, and the
/// keyboard state; this module returns the resulting [`QuicksaveAction`].
#[derive(Debug, Default)]
pub struct QuicksaveLoopState {
    /// Sim tick at which the last autosave fired (defaults to 0).
    pub last_autosave_tick: u64,
    /// Last F5 / F9 result for the corruption modal + the migration banner.
    pub last_outcome: Option<QuicksaveOutcomeUi>,
    /// Whether the player's "Replay migrated" banner has already been shown.
    pub migration_banner_shown: bool,
}

#[derive(Debug, Clone)]
pub enum QuicksaveAction {
    None,
    Quicksave,
    Quickload,
    Autosave,
}

#[derive(Debug, Clone)]
pub enum QuicksaveOutcomeUi {
    /// F5 succeeded; show transient confirmation toast.
    SaveOk {
        path: String,
        wall_clock_ms: u32,
    },
    /// F9 succeeded; show transient confirmation toast.
    LoadOk {
        path: String,
        wall_clock_ms: u32,
        migrated_from: Option<String>,
        migrated_to: Option<String>,
    },
    /// **M4B § "Save corruption is detectable"** — corruption modal text.
    ChecksumMismatch { expected: String, actual: String },
    /// **M4B § "Save from a future version is rejected clearly"**.
    UnsupportedFutureVersion {
        found: String,
        max_supported: String,
    },
    /// Migration failed mid-walk.
    MigrationFailed {
        from: String,
        to: String,
        reason: String,
    },
    /// Any other error surfaces as a generic toast.
    OtherError(String),
}

impl QuicksaveOutcomeUi {
    pub fn from_save_error(err: &SaveError) -> Self {
        match err {
            SaveError::ChecksumMismatch { expected, actual } => Self::ChecksumMismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            },
            SaveError::UnsupportedFutureVersion { found, max_supported } => Self::UnsupportedFutureVersion {
                found: found.as_string(),
                max_supported: max_supported.as_string(),
            },
            SaveError::MigrationFailed { from, to, reason } => Self::MigrationFailed {
                from: from.as_string(),
                to: to.as_string(),
                reason: reason.clone(),
            },
            other => Self::OtherError(other.to_string()),
        }
    }

    /// **M4B § "cf-app renders the plain-language modal"** — the exact
    /// string the corruption / future-version modal displays.
    pub fn modal_plain_language(&self) -> Option<String> {
        match self {
            Self::ChecksumMismatch { .. } => Some(
                "Save file appears corrupted (checksum mismatch). Try another slot.".to_string(),
            ),
            Self::UnsupportedFutureVersion { found, .. } => Some(format!(
                "This save was created in a newer game version ({found}). Update Corefall to load it."
            )),
            Self::MigrationFailed { from, to, reason } => Some(format!(
                "Save migration failed ({from} -> {to}): {reason}"
            )),
            Self::OtherError(msg) => Some(format!("Save failed: {msg}")),
            _ => None,
        }
    }

    /// **M4B § "Replay migrated banner"** — the one-line banner the viewer
    /// header renders when a load triggered a migration step.
    pub fn migration_banner(&self) -> Option<String> {
        match self {
            Self::LoadOk {
                migrated_from: Some(from),
                migrated_to: Some(to),
                ..
            } => Some(format!("Replay migrated from {from} -> {to}")),
            _ => None,
        }
    }
}

/// Decide whether the current frame should fire a quicksave / quickload /
/// autosave. The Bevy system passes `f5_just_pressed`, `f9_just_pressed`,
/// and the current sim tick.
pub fn next_action(
    state: &QuicksaveLoopState,
    f5_just_pressed: bool,
    f9_just_pressed: bool,
    current_tick: u64,
    tick_rate_hz: u32,
) -> QuicksaveAction {
    if f5_just_pressed {
        return QuicksaveAction::Quicksave;
    }
    if f9_just_pressed {
        return QuicksaveAction::Quickload;
    }
    if cf_control::m4b_save::autosave_due_at_rate(state.last_autosave_tick, current_tick, tick_rate_hz) {
        return QuicksaveAction::Autosave;
    }
    QuicksaveAction::None
}

/// Update [`QuicksaveLoopState`] after firing an autosave (so the next
/// autosave is exactly one cadence later).
pub fn record_autosave(state: &mut QuicksaveLoopState, current_tick: u64) {
    state.last_autosave_tick = current_tick;
}

/// Construct the "last-save toast" string surfaced in the HUD when F5 or
/// F9 completes.
pub fn toast_for(meta: &LastSaveMetadata) -> Option<String> {
    let path = meta.path.as_deref()?;
    let blake3 = meta.blake3.as_deref()?;
    let wall_ms = meta.wall_clock_ms.unwrap_or(0);
    let op = meta.last_operation.as_deref().unwrap_or("save");
    Some(format!("{op} -> {path} blake3={} ({wall_ms} ms)", &blake3[..blake3.len().min(8)]))
}

/// The 800 ms budget the M4B spec mandates for quicksave + quickload on the
/// reference Workstation tier. The cf-app HUD surfaces a warning when a
/// save/load exceeds this.
pub const WORKSTATION_BUDGET: Duration = Duration::from_millis(800);

/// Default save directory under `<save_root>/quicksave/`. Used by F5/F9
/// when the player has not picked a custom slot.
pub fn default_quicksave_dir(save_root: &Path) -> std::path::PathBuf {
    save_root.join("quicksave")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_picks_quicksave_then_quickload_then_autosave() {
        let mut state = QuicksaveLoopState::default();
        assert!(matches!(next_action(&state, true, false, 0, 60), QuicksaveAction::Quicksave));
        assert!(matches!(next_action(&state, false, true, 0, 60), QuicksaveAction::Quickload));
        assert!(matches!(next_action(&state, false, false, 0, 60), QuicksaveAction::None));
        state.last_autosave_tick = 0;
        assert!(matches!(
            next_action(&state, false, false, 60 * 60, 60),
            QuicksaveAction::Autosave
        ));
        // **M4B Notes**: 60-second timer honors tick_rate_hz.
        assert!(matches!(
            next_action(&state, false, false, 120 * 60, 120),
            QuicksaveAction::Autosave
        ));
        assert!(matches!(
            next_action(&state, false, false, 120 * 60 - 1, 120),
            QuicksaveAction::None
        ));
    }

    #[test]
    fn modal_plain_language_returns_corruption_string_for_checksum_mismatch() {
        let ui = QuicksaveOutcomeUi::ChecksumMismatch {
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };
        let modal = ui.modal_plain_language().unwrap();
        assert!(modal.contains("Save file appears corrupted"));
        assert!(modal.contains("checksum mismatch"));
    }

    #[test]
    fn modal_plain_language_quotes_future_version_in_the_message() {
        let ui = QuicksaveOutcomeUi::UnsupportedFutureVersion {
            found: "v99.0.0".to_string(),
            max_supported: "v2.0.0".to_string(),
        };
        let modal = ui.modal_plain_language().unwrap();
        assert!(modal.contains("v99.0.0"));
        assert!(modal.contains("Update Corefall"));
    }

    #[test]
    fn migration_banner_only_renders_when_load_triggered_migration() {
        let no_migration = QuicksaveOutcomeUi::LoadOk {
            path: "/tmp/q.cfsave".to_string(),
            wall_clock_ms: 200,
            migrated_from: None,
            migrated_to: None,
        };
        assert!(no_migration.migration_banner().is_none());
        let migrated = QuicksaveOutcomeUi::LoadOk {
            path: "/tmp/q.cfsave".to_string(),
            wall_clock_ms: 250,
            migrated_from: Some("v1.0.0".to_string()),
            migrated_to: Some("v2.0.0".to_string()),
        };
        let banner = migrated.migration_banner().unwrap();
        assert_eq!(banner, "Replay migrated from v1.0.0 -> v2.0.0");
    }

    #[test]
    fn from_save_error_maps_every_variant_to_ui() {
        let cs = QuicksaveOutcomeUi::from_save_error(&SaveError::ChecksumMismatch {
            expected: "a".to_string(),
            actual: "b".to_string(),
        });
        assert!(matches!(cs, QuicksaveOutcomeUi::ChecksumMismatch { .. }));
        let future = QuicksaveOutcomeUi::from_save_error(&SaveError::UnsupportedFutureVersion {
            found: cf_save::SaveSchemaVersion::new(99, 0, 0),
            max_supported: cf_save::CURRENT_SAVE_SCHEMA_VERSION,
        });
        assert!(matches!(future, QuicksaveOutcomeUi::UnsupportedFutureVersion { .. }));
        let mig = QuicksaveOutcomeUi::from_save_error(&SaveError::MigrationFailed {
            from: cf_save::V1_0_0,
            to: cf_save::V2_0_0,
            reason: "demo".to_string(),
        });
        assert!(matches!(mig, QuicksaveOutcomeUi::MigrationFailed { .. }));
    }
}
