//! **M4B § "Ledger chain rejects tampered bundle"** — per-event BLAKE3
//! chain verifier over a run bundle's `events.jsonl`.
//!
//! Reads `<bundle>/run_manifest.json` for `run_id`, `seed`, and
//! `ledger_chain_anchor`; reads `<bundle>/events.jsonl` for the actual
//! event chain; walks the chain via `cf_save::ledger_chain::verify_chain`;
//! prints the structured JSON outcome + exits non-zero on tamper.
//!
//! The viewer-side companion is
//! `cf-tools-replay-viewer validate <bundle>` which embeds the same
//! `cf-save::ledger_chain::verify_chain` API.

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

pub fn run(bundle_dir: &Path, json_output: bool) -> Result<()> {
    let manifest_path = bundle_dir.join("run_manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_str(&manifest_text)?;
    let seed = manifest
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("manifest missing seed"))?;
    let run_id = manifest
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("manifest missing run_id"))?
        .to_string();
    let recorded_anchor = manifest
        .get("ledger_chain_anchor")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    if recorded_anchor.is_none() {
        // Bundle was recorded without chain mode. Surface a structured
        // "empty_chain" outcome rather than walking the verifier (which
        // would report `tampered` against empty chained_hash_hex fields).
        let envelope = serde_json::json!({
            "result": "empty_chain",
            "bundle": bundle_dir.display().to_string(),
            "reason": "manifest has no ledger_chain_anchor; bundle predates M4B chain mode",
        });
        if json_output {
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            println!(
                "ledger chain EMPTY: no anchor in manifest; bundle predates M4B chain mode. bundle={}",
                bundle_dir.display()
            );
        }
        return Ok(());
    }
    let events_path = bundle_dir.join("events.jsonl");
    let events_text = fs::read_to_string(&events_path)
        .with_context(|| format!("read {}", events_path.display()))?;
    let chained = read_chained_events(&events_text)?;
    let outcome = cf_save::ledger_chain::verify_chain(&run_id, seed, &chained);
    // log with anchor + total_events"** — append the structured audit
    // entry to `<bundle>/ledger_chain_audit.jsonl` regardless of result
    // (so the audit trail persists even when verification fails).
    write_audit_event(bundle_dir, &run_id, &outcome).ok();

    let envelope = match &outcome {
        cf_save::ledger_chain::VerifyOutcome::Clean {
            events_verified,
            anchor,
        } => {
            let anchor_matches = recorded_anchor.as_deref().map(|a| a == anchor);
            serde_json::json!({
                "result": "clean",
                "events_verified": events_verified,
                "anchor": anchor,
                "recorded_anchor_matches": anchor_matches,
                "bundle": bundle_dir.display().to_string(),
            })
        }
        cf_save::ledger_chain::VerifyOutcome::Tampered { first_break } => serde_json::json!({
            "result": "tampered",
            "first_break": {
                "event_id": first_break.event_id,
                "expected_hash": first_break.expected_hash,
                "actual_hash": first_break.actual_hash,
            },
            "bundle": bundle_dir.display().to_string(),
        }),
        cf_save::ledger_chain::VerifyOutcome::EmptyChain => serde_json::json!({
            "result": "empty_chain",
            "bundle": bundle_dir.display().to_string(),
        }),
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        match &outcome {
            cf_save::ledger_chain::VerifyOutcome::Clean {
                events_verified,
                anchor,
            } => {
                println!(
                    "ledger chain CLEAN: events_verified={events_verified} anchor={anchor} bundle={}",
                    bundle_dir.display()
                );
                if let Some(expected) = &recorded_anchor {
                    if expected != anchor {
                        println!("WARN: recorded anchor {expected} != computed {anchor}");
                        std::process::exit(1);
                    }
                }
            }
            cf_save::ledger_chain::VerifyOutcome::Tampered { first_break } => {
                println!(
                    "ledger chain TAMPERED at event_id={} expected={} actual={} bundle={}",
                    first_break.event_id,
                    first_break.expected_hash,
                    first_break.actual_hash,
                    bundle_dir.display(),
                );
                std::process::exit(1);
            }
            cf_save::ledger_chain::VerifyOutcome::EmptyChain => {
                println!("ledger chain EMPTY: no chained events recorded; bundle predates M4B chain mode");
            }
        }
    }

    if matches!(outcome, cf_save::ledger_chain::VerifyOutcome::Tampered { .. }) {
        std::process::exit(1);
    }
    Ok(())
}

/// entry to `<bundle>/ledger_chain_audit.jsonl`. The schema matches
/// `cf-replay/schemas/event/ledger_chain_verified.json`. We use a sidecar
/// file (not events.jsonl) because the bundle is otherwise read-only after
/// the run finished; injecting events into events.jsonl would break the
/// existing recorder's event ordering + invalidate the BLAKE3 chain.
fn write_audit_event(
    bundle_dir: &Path,
    run_id: &str,
    outcome: &cf_save::ledger_chain::VerifyOutcome,
) -> Result<()> {
    use std::io::Write as _;
    let audit_path = bundle_dir.join("ledger_chain_audit.jsonl");
    let envelope = match outcome {
        cf_save::ledger_chain::VerifyOutcome::Clean { events_verified, anchor } => {
            serde_json::json!({
                "event_type": "ledger_chain_verified",
                "run_id": run_id,
                "result": "clean",
                "events_verified": events_verified,
                "anchor": anchor,
                "verifier": "cf-mod",
            })
        }
        cf_save::ledger_chain::VerifyOutcome::Tampered { first_break } => serde_json::json!({
            "event_type": "ledger_chain_verified",
            "run_id": run_id,
            "result": "tampered",
            "first_break": {
                "event_id": first_break.event_id,
                "expected_hash": first_break.expected_hash,
                "actual_hash": first_break.actual_hash,
            },
            "verifier": "cf-mod",
        }),
        cf_save::ledger_chain::VerifyOutcome::EmptyChain => serde_json::json!({
            "event_type": "ledger_chain_verified",
            "run_id": run_id,
            "result": "empty_chain",
            "verifier": "cf-mod",
        }),
    };
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&audit_path)?;
    writeln!(f, "{}", serde_json::to_string(&envelope)?)?;
    Ok(())
}

fn read_chained_events(text: &str) -> Result<Vec<cf_save::ledger_chain::ChainedEvent>> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let env: Value =
            serde_json::from_str(trimmed).with_context(|| format!("parse event line {}", idx + 1))?;
        let event_id = env
            .get("event_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("event line {} missing event_id", idx + 1))?
            .to_string();
        let payload = env.get("payload").cloned().unwrap_or(Value::Null);
        let payload_canonical_json = serde_json::to_string(&payload)?;
        // recorded without chain mode, both are absent (`None`), and the
        // verifier will report tampered or empty_chain as appropriate.
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
