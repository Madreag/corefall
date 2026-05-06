use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "cf-server",
    about = "Dedicated server binary (DR-034). Stubbed in M0; real modes (coop_room/pvp_arena/lan_room/mmo_shard/lobby_directory) land in M9..M12."
)]
struct Cli {
    #[arg(long)]
    mode: Option<String>,
    #[arg(long)]
    config: Option<String>,
    #[arg(long)]
    validate_config_only: bool,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::server", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::server", "cf-server M0 stub. parsed_cli = {cli:?}");
    Ok(())
}
