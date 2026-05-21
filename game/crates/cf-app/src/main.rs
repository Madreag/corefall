#![allow(
    clippy::doc_lazy_continuation,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use anyhow::Result;
use clap::Parser;

use cf_replay::diagnostics;

mod app;
mod cli;
mod gamepad_focus;
mod headless;
mod hold_tracker;
mod input;
mod m12;
mod quicksave;
mod systems_sync;

use crate::cli::{CaptureOptions, Cli};
use crate::headless::{
    build_config, locate_scenario, reject_capture_grid_with_headless_smoke, run_headless, run_headless_server,
};

fn main() -> Result<()> {
    diagnostics::init("cf::app");
    let cli = Cli::parse();
    let scenario_path = locate_scenario(&cli.scenario)?;
    let config = build_config(&cli, scenario_path)?;
    let capture_opts = CaptureOptions::from_cli(&cli);
    reject_capture_grid_with_headless_smoke(&cli)?;
    tracing::info!(target: "cf::app", scenario = %cli.scenario, headless_smoke = cli.headless_smoke, control_api = cli.control_api, tick_rate_hz = cli.tick_rate_hz, capture_grid = cli.capture_grid, "cf-app M0 starting");

    match (cli.headless_smoke, cli.control_api) {
        (true, true) => run_headless_server(
            config,
            cli.control_port,
            cli.control_uds.clone(),
            cli.control_port_file.clone(),
            cli.unpaced,
        ),
        (true, false) => run_headless(config),
        (false, _) => crate::app::run_bevy(
            config,
            cli.control_api,
            cli.control_port,
            cli.control_uds.clone(),
            cli.control_port_file.clone(),
            !cli.disable_local_input,
            capture_opts,
            cli.unpaced,
        ),
    }
}
