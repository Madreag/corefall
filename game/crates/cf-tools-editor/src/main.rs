use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "cf-tools-editor",
    about = "In-engine scenario/package/material workbench. Stubbed in M0; real implementation lands in M8/M8.5."
)]
struct Cli {
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    scenario: Option<String>,
    #[arg(long)]
    suite: Option<String>,
    #[arg(long)]
    write_run_bundle: bool,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::editor", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::editor", "cf-tools-editor M0 stub. parsed_cli = {cli:?}");
    Ok(())
}
