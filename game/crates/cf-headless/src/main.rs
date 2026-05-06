use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "cf-headless",
    about = "Headless sim runner / replay verifier. Stubbed in M0; real implementation lands in M3 (replay verification) and M9 (server core)."
)]
struct Cli {
    #[arg(long)]
    scenario: Option<String>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    ticks: Option<u64>,
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: String,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::headless", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::headless", "cf-headless M0 stub. parsed_cli = {cli:?}");
    Ok(())
}
