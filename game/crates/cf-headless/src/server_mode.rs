//! M8A § cf-headless serve — authoritative server entry point.
//!
//! Runs the deterministic sim core with no Bevy render plugin. Emits a
//! server-side run bundle to `prototype_runs/server/<run-id>/`. At M8A
//! this is a scaffolding stub that validates the deterministic config
//! and writes a manifest demonstrating the server bundle path
//! convention; M9+ wires the live cf-net transport + drive-tick loop.

use std::path::Path;

use anyhow::{Context, Result};

use cf_net::server::{Server, ServerConfig};

#[derive(Debug)]
pub struct ServeArgs {
    pub scenario: std::path::PathBuf,
    pub bind_addr: String,
    pub port: u16,
    pub ticks: u32,
    pub no_render: bool,
    pub max_clients: u32,
}

pub fn run_serve(args: ServeArgs) -> Result<()> {
    tracing::info!(
        target: "cf::headless::serve",
        scenario=%args.scenario.display(),
        bind_addr=%args.bind_addr,
        port=args.port,
        ticks=args.ticks,
        no_render=args.no_render,
        max_clients=args.max_clients,
        "M8A cf-headless serve scaffold starting",
    );

    let cfg = ServerConfig {
        bind_addr: args.bind_addr,
        port: args.port,
        scenario_path: args.scenario.to_string_lossy().into_owned(),
        snapshot_cadence_ticks: cf_net::protocol::SNAPSHOT_CADENCE_TICKS,
        checksum_probe_cadence_ticks: cf_net::protocol::CHECKSUM_PROBE_CADENCE_TICKS,
        max_clients: args.max_clients,
    };
    let server = Server::new(cfg);

    let run_id = generate_run_id();
    let bundle_path = server.run_bundle_path(&run_id);
    tracing::info!(target: "cf::headless::serve", bundle_path=%bundle_path, "server-side run bundle path");

    let bundle_dir = Path::new(&bundle_path);
    std::fs::create_dir_all(bundle_dir).with_context(|| format!("create {bundle_path}"))?;
    let manifest = serde_json::json!({
        "schema_version": "m8a.v0.1",
        "run_id": run_id,
        "bind_addr": server.config.bind_addr,
        "port": server.config.port,
        "scenario": server.config.scenario_path,
        "max_clients": server.config.max_clients,
        "snapshot_cadence_ticks": server.config.snapshot_cadence_ticks,
        "checksum_probe_cadence_ticks": server.config.checksum_probe_cadence_ticks,
        "ticks_planned": args.ticks,
        "no_render": args.no_render,
        "note": "M8A cf-headless serve scaffold; M9+ wires live cf-net transport + drive-tick loop.",
    });
    let manifest_path = bundle_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_path.display()))?;
    tracing::info!(target: "cf::headless::serve", manifest=%manifest_path.display(), "wrote server-side run bundle manifest");
    Ok(())
}

fn generate_run_id() -> String {
    // M8A § cf-headless serve: run_id is generated at process startup,
    // OUTSIDE the sim island (the sim never reads wall time). Per the
    // determinism contract this site is documented in
    // docs/plan/spec/determinism-island-contract.md § Inside vs Outside
    // the island.
    #[allow(clippy::disallowed_methods)]
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("m8a_serve_{now}")
}
