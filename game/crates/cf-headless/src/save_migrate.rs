//! **M4B** — `cf-headless save migrate` + `cf-headless save inspect` +
//! `cf-headless replay --skip-chain-verify` glue.
//!
//! The migrate path runs the canonical `cf_save::migration::registry()`
//! against `<dir>/quicksave.cfsave` and writes the result back. The
//! inspect path prints schema_version + delta chain depth + ledger
//! anchor. The chain-verify path walks the bundle's `events.jsonl` chain
//! against the manifest's `seed` + `run_id` + `ledger_chain_anchor`.

use std::{fs, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;

pub fn run_migrate(dir: &Path, target_version: Option<&str>) -> Result<()> {
    let outcome = cf_save::quicksave::read_quicksave(dir)
        .map_err(|e| anyhow!("read quicksave from {}: {e}", dir.display()))?;
    if let Some(target) = target_version {
        let requested = parse_version(target)?;
        if outcome.save.schema_version != requested {
            // The current registry only knows v1 -> v2; future schema bumps
            // extend the registry. If a future caller asks for a target
            // newer than this build supports we bail loudly.
            bail!(
                "requested target version {target} != migrated schema {} (no handler chain reaches that target in this build)",
                outcome.save.schema_version.as_string()
            );
        }
    }
    let write =
        cf_save::quicksave::write_quicksave(dir, &outcome.save).map_err(|e| anyhow!("write quicksave: {e}"))?;
    let envelope = serde_json::json!({
        "path": write.path.display().to_string(),
        "schema_version": [
            outcome.save.schema_version.major,
            outcome.save.schema_version.minor,
            outcome.save.schema_version.patch,
        ],
        "blake3": write.checksum_hex,
        "size_bytes": write.bytes_written,
        "migrated_from": outcome.migrated_from.map(|v| [v.major, v.minor, v.patch]),
        "migrated_to": outcome.migrated_to.map(|v| [v.major, v.minor, v.patch]),
        "handler_chain": outcome.handler_chain,
    });
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

pub fn run_inspect(dir: &Path) -> Result<()> {
    // **M4B § "cf-headless save inspect"** — works on both `.cfsave` save
    // directories AND run-bundle directories. Save-dir contents take
    // priority when both are present.
    let save_path = dir.join(cf_save::quicksave::QUICKSAVE_FILE);
    let mut envelope = serde_json::json!({
        "path": dir.display().to_string(),
    });
    if save_path.exists() {
        let bytes = fs::read(&save_path).with_context(|| format!("read {}", save_path.display()))?;
        let raw: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse {} as JSON", save_path.display()))?;
        envelope["save_path"] = Value::String(save_path.display().to_string());
        envelope["size_bytes"] = serde_json::json!(bytes.len());
        envelope["schema_version"] = raw.get("schema_version").cloned().unwrap_or(Value::Null);
        envelope["world_tick"] = raw.get("world_tick").cloned().unwrap_or(Value::Null);
        envelope["actor_count"] = serde_json::json!(
            raw.get("actors").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)
        );
    }
    // Run-bundle chain summary (works for both save-dirs that happen to
    // have an events.jsonl sibling AND pure run-bundle dirs).
    let events_path = dir.join("events.jsonl");
    if events_path.exists() {
        let chain_summary = summarize_snapshot_chain(&events_path)?;
        envelope["delta_chain_depth"] = serde_json::json!(chain_summary.delta_chain_depth);
        envelope["last_baseline_tick"] = serde_json::json!(chain_summary.last_baseline_tick);
        envelope["baseline_count"] = serde_json::json!(chain_summary.baseline_count);
        envelope["baseline_ticks"] = serde_json::json!(chain_summary.baseline_ticks);
        envelope["delta_count_total"] = serde_json::json!(chain_summary.delta_count_total);
    }
    let manifest_path = dir.join("run_manifest.json");
    if manifest_path.exists() {
        let manifest_text = fs::read_to_string(&manifest_path)?;
        let manifest: Value = serde_json::from_str(&manifest_text)?;
        envelope["run_id"] = manifest.get("run_id").cloned().unwrap_or(Value::Null);
        envelope["save_schema_version"] = manifest.get("save_schema_version").cloned().unwrap_or(Value::Null);
        envelope["delta_baseline_cadence_ticks"] = manifest
            .get("delta_baseline_cadence_ticks")
            .cloned()
            .unwrap_or(Value::Null);
        if let Some(anchor) = manifest.get("ledger_chain_anchor").and_then(|v| v.as_str()) {
            envelope["ledger_chain_anchor"] = Value::String(anchor.to_string());
        }
    }
    if !save_path.exists() && !events_path.exists() && !manifest_path.exists() {
        bail!(
            "save inspect: {} contains neither quicksave.cfsave nor a run-bundle (events.jsonl + run_manifest.json)",
            dir.display()
        );
    }
    println!("{}", serde_json::to_string_pretty(&envelope)?);
    Ok(())
}

pub fn verify_chain_in_bundle(bundle_dir: &Path) -> Result<()> {
    let manifest_path = bundle_dir.join("run_manifest.json");
    if !manifest_path.exists() {
        // No manifest = nothing to verify. Caller handles this further down.
        return Ok(());
    }
    let manifest_text = fs::read_to_string(&manifest_path)?;
    let manifest: Value = serde_json::from_str(&manifest_text)?;
    let anchor = match manifest.get("ledger_chain_anchor").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Ok(()), // Dev-mode bundles have no chain.
    };
    let seed = manifest
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("manifest missing seed"))?;
    let run_id = manifest
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("manifest missing run_id"))?
        .to_string();
    let events_path = bundle_dir.join("events.jsonl");
    let chained = read_chained_events(&events_path)?;
    let outcome = cf_save::ledger_chain::verify_chain(&run_id, seed, &chained);
    match outcome {
        cf_save::ledger_chain::VerifyOutcome::Clean {
            events_verified,
            anchor: actual_anchor,
        } => {
            if actual_anchor != anchor {
                bail!(
                    "ledger chain anchor mismatch: manifest={anchor}, computed={actual_anchor} (events_verified={events_verified})"
                );
            }
            tracing::info!(
                target: "cf::headless::chain",
                events_verified,
                anchor = %actual_anchor,
                "ledger_chain.verified"
            );
            Ok(())
        }
        cf_save::ledger_chain::VerifyOutcome::Tampered { first_break } => bail!(
            "ledger chain tampered at event_id={}: expected={}, actual={}",
            first_break.event_id,
            first_break.expected_hash,
            first_break.actual_hash
        ),
        cf_save::ledger_chain::VerifyOutcome::EmptyChain => Ok(()),
    }
}

fn parse_version(s: &str) -> Result<cf_save::SaveSchemaVersion> {
    let s = s.trim_start_matches('v');
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        bail!("version must be major.minor.patch: got {s}");
    }
    let major = parts[0].parse::<u16>().context("major")?;
    let minor = parts[1].parse::<u16>().context("minor")?;
    let patch = parts[2].parse::<u16>().context("patch")?;
    Ok(cf_save::SaveSchemaVersion::new(major, minor, patch))
}

/// Parse `events.jsonl` into the `ChainedEvent` form expected by
/// `cf_save::ledger_chain::verify_chain`. Each line is one event envelope
/// whose `payload` becomes the canonical-JSON payload for the chain
/// hash; `prev_event_hash` + the envelope's `chained_hash_hex` carry the
/// chain state.
fn read_chained_events(path: &Path) -> Result<Vec<cf_save::ledger_chain::ChainedEvent>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let env: Value = serde_json::from_str(trimmed)
            .with_context(|| format!("parse line {} of {}", idx + 1, path.display()))?;
        let event_id = env
            .get("event_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("event line {} missing event_id", idx + 1))?
            .to_string();
        let payload = env.get("payload").cloned().unwrap_or(Value::Null);
        let payload_canonical_json = serde_json::to_string(&payload)?;
        let prev_event_hash = env
            .get("prev_event_hash")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        let chained_hash_hex = env
            .get("chained_hash_hex")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or_default();
        out.push(cf_save::ledger_chain::ChainedEvent {
            event_id,
            payload_canonical_json,
            prev_event_hash,
            chained_hash_hex,
        });
    }
    Ok(out)
}

struct SnapshotChainSummary {
    pub delta_chain_depth: u64,
    pub baseline_count: u64,
    pub last_baseline_tick: Option<u64>,
    /// **M4B § "Delta baseline cadence is enforced"** — the ticks at which
    /// every `snapshot.baseline_emitted` event fired, in order. The
    /// Gherkin's "exactly 7 ... at ticks 0, 600, 1200, 1800, 2400, 3000,
    /// 3600" assertion reads off this list.
    pub baseline_ticks: Vec<u64>,
    pub delta_count_total: u64,
}

fn summarize_snapshot_chain(events_path: &Path) -> Result<SnapshotChainSummary> {
    let text = fs::read_to_string(events_path)?;
    let mut delta_chain_depth: u64 = 0;
    let mut baseline_count: u64 = 0;
    let mut last_baseline_tick: Option<u64> = None;
    let mut baseline_ticks: Vec<u64> = Vec::new();
    let mut delta_count_total: u64 = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let env: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let category = env.get("category").and_then(Value::as_str).unwrap_or("");
        let event_type = env.get("event_type").and_then(Value::as_str).unwrap_or("");
        if category != "snapshot" {
            continue;
        }
        let tick = env.get("tick").and_then(Value::as_u64).unwrap_or(0);
        match event_type {
            "baseline_emitted" => {
                baseline_count += 1;
                last_baseline_tick = Some(tick);
                baseline_ticks.push(tick);
                delta_chain_depth = 0;
            }
            "delta_emitted" => {
                delta_chain_depth += 1;
                delta_count_total += 1;
            }
            _ => {}
        }
    }
    Ok(SnapshotChainSummary {
        delta_chain_depth,
        baseline_count,
        last_baseline_tick,
        baseline_ticks,
        delta_count_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_version_supports_v_prefix_and_bare() {
        assert_eq!(parse_version("v1.2.3").unwrap(), cf_save::SaveSchemaVersion::new(1, 2, 3));
        assert_eq!(parse_version("2.0.0").unwrap(), cf_save::SaveSchemaVersion::new(2, 0, 0));
    }

    #[test]
    fn parse_version_rejects_malformed() {
        assert!(parse_version("1.2").is_err());
        assert!(parse_version("v1.x.0").is_err());
    }

    #[test]
    fn run_migrate_writes_back_canonical_save() -> Result<()> {
        let dir = tempdir().unwrap();
        let world = cf_save::WorldSave::empty(0);
        cf_save::quicksave::write_quicksave(dir.path(), &world).map_err(|e| anyhow!("write: {e}"))?;
        run_migrate(dir.path(), Some("v2.0.0"))?;
        let read_back = cf_save::quicksave::read_quicksave(dir.path()).map_err(|e| anyhow!("read: {e}"))?;
        assert_eq!(read_back.save.schema_version, cf_save::CURRENT_SAVE_SCHEMA_VERSION);
        Ok(())
    }

    #[test]
    fn summarize_snapshot_chain_counts_baselines_and_deltas() -> Result<()> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let baseline = serde_json::json!({"category": "snapshot", "event_type": "baseline_emitted", "tick": 0});
        let delta = serde_json::json!({"category": "snapshot", "event_type": "delta_emitted", "tick": 1});
        let baseline2 = serde_json::json!({"category": "snapshot", "event_type": "baseline_emitted", "tick": 600});
        let other = serde_json::json!({"category": "system", "event_type": "run_started", "tick": 0});
        fs::write(
            &path,
            format!(
                "{}\n{}\n{}\n{}\n{}\n",
                baseline, delta, delta, baseline2, other
            ),
        )?;
        let s = summarize_snapshot_chain(&path)?;
        assert_eq!(s.baseline_count, 2);
        assert_eq!(s.delta_chain_depth, 0); // reset by the second baseline
        assert_eq!(s.last_baseline_tick, Some(600));
        Ok(())
    }

}
