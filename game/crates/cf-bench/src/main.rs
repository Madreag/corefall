use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "cf-bench",
    about = "Performance harness. Stubbed in M0; real implementation begins in M2."
)]
struct Cli {
    #[arg(long)]
    scenario: Option<String>,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long, default_value_t = 5)]
    runs: u32,
    #[arg(long)]
    write_bench_report: bool,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::bench", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::bench", "cf-bench M0 stub. parsed_cli = {cli:?}");
    Ok(())
}
