//! **M4B § "cf-mod save validate"** — full schema + migration + checksum
//! validation pass over a single `.cfsave` file.
//!
//! Steps:
//!
//! 1. Read the file. Bail with a structured error on I/O failure.
//! 2. Parse the canonical-JSON BLAKE3 from the sidecar checksum file (if
//!    present). When the sidecar exists the loader MUST verify it.
//! 3. Deserialize to [`cf_save::WorldSave`] via the canonical loader so any
//!    `SaveError::ChecksumMismatch` / `UnsupportedFutureVersion` /
//!    `MigrationFailed` surfaces verbatim.
//! 4. Run the migration registry forward to the current build's schema.
//! 5. Emit a structured JSON envelope summarizing the outcome.

use std::{fs, path::Path};

use anyhow::{Context, Result};

pub fn run(path: &Path, json_output: bool) -> Result<()> {
    let report = validate(path);
    let envelope = serde_json::to_value(&report)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        match &report.status.as_str() {
            &"ok" => println!(
                "save VALID: schema_version={} actor_count={} blake3={} migrated_from={:?}",
                report.schema_version_pretty,
                report.actor_count,
                report.blake3_hex.as_deref().unwrap_or("(none)"),
                report.migrated_from_pretty
            ),
            _ => println!(
                "save INVALID: {}: {}",
                report.status,
                report.error.as_deref().unwrap_or("")
            ),
        }
    }
    if report.status != "ok" {
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ValidateReport {
    pub path: String,
    pub status: String,
    pub error: Option<String>,
    pub schema_version_pretty: String,
    pub schema_version: Option<[u16; 3]>,
    pub actor_count: usize,
    pub blake3_hex: Option<String>,
    pub size_bytes: u64,
    pub migrated_from_pretty: Option<String>,
    pub migrated_to_pretty: Option<String>,
    pub handler_chain: Vec<String>,
}

fn validate(path: &Path) -> ValidateReport {
    let bytes = match fs::read(path).with_context(|| format!("read {}", path.display())) {
        Ok(b) => b,
        Err(e) => return invalid(path, "io_error", &e.to_string()),
    };
    let json = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(e) => return invalid(path, "utf8_error", &e.to_string()),
    };
    // Read the sibling checksum file (if it exists) to wire the canonical
    // integrity check; this is how on-disk `.cfsave + .cfsave.checksum`
    // pairs validate.
    let checksum_path = path.with_extension(format!(
        "{}.checksum",
        path.extension().and_then(|e| e.to_str()).unwrap_or("cfsave")
    ));
    let expected_checksum = fs::read_to_string(&checksum_path)
        .ok()
        .map(|s| s.trim().to_string());
    let raw_save = match cf_save::WorldSave::deserialize(json, expected_checksum.as_deref()) {
        Ok(o) => o,
        Err(cf_save::SaveError::ChecksumMismatch { expected, actual }) => {
            return invalid(
                path,
                "checksum_mismatch",
                &format!("expected={expected}, actual={actual}"),
            );
        }
        Err(cf_save::SaveError::UnsupportedFutureVersion { found, max_supported }) => {
            return invalid(
                path,
                "unsupported_future_version",
                &format!(
                    "found={}, max_supported={}",
                    found.as_string(),
                    max_supported.as_string()
                ),
            );
        }
        Err(other) => return invalid(path, "load_error", &other.to_string()),
    };
    let outcome = match cf_save::migration::migrate_to_current(raw_save) {
        Ok(o) => cf_save::quicksave::QuickloadOutcome {
            save: o.blob,
            checksum_hex: expected_checksum.unwrap_or_default(),
            migrated_from: if o.from == cf_save::CURRENT_SAVE_SCHEMA_VERSION {
                None
            } else {
                Some(o.from)
            },
            migrated_to: if o.from == cf_save::CURRENT_SAVE_SCHEMA_VERSION {
                None
            } else {
                Some(o.to)
            },
            handler_chain: o.handler_chain,
            wall_clock_ms: 0,
        },
        Err(cf_save::SaveError::MigrationFailed { from, to, reason }) => {
            return invalid(
                path,
                "migration_failed",
                &format!("from={} to={} reason={reason}", from.as_string(), to.as_string()),
            );
        }
        Err(other) => return invalid(path, "migrate_error", &other.to_string()),
    };
    ValidateReport {
        path: path.display().to_string(),
        status: "ok".to_string(),
        error: None,
        schema_version_pretty: outcome.save.schema_version.as_string(),
        schema_version: Some([
            outcome.save.schema_version.major,
            outcome.save.schema_version.minor,
            outcome.save.schema_version.patch,
        ]),
        actor_count: outcome.save.actors.len(),
        blake3_hex: Some(outcome.checksum_hex.clone()),
        size_bytes: bytes.len() as u64,
        migrated_from_pretty: outcome.migrated_from.map(|v| v.as_string()),
        migrated_to_pretty: outcome.migrated_to.map(|v| v.as_string()),
        handler_chain: outcome.handler_chain.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn invalid(path: &Path, status: &str, error: &str) -> ValidateReport {
    ValidateReport {
        path: path.display().to_string(),
        status: status.to_string(),
        error: Some(error.to_string()),
        schema_version_pretty: "unknown".to_string(),
        schema_version: None,
        actor_count: 0,
        blake3_hex: None,
        size_bytes: 0,
        migrated_from_pretty: None,
        migrated_to_pretty: None,
        handler_chain: Vec::new(),
    }
}
