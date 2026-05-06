use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "cf-e2e",
    about = "Scripted end-to-end runner. Stubbed in M0; real cfctl-driven runner lands in M1.5."
)]
struct Cli {
    #[arg(long)]
    scenario: Option<String>,
    #[arg(long)]
    script: Option<String>,
    #[arg(long, action = clap::ArgAction::Append)]
    expect: Vec<String>,
    #[arg(long)]
    write_run_bundle: bool,
    #[arg(long, default_value_t = 1.0)]
    ui_scale: f32,
    #[arg(long)]
    high_contrast: bool,
    #[arg(long)]
    verify_focus: bool,
    #[arg(long)]
    save_load_roundtrip: bool,
    #[arg(long)]
    verify_checksums: bool,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::e2e", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::e2e", "cf-e2e M0 stub; full runner lands in M1.5. parsed_cli = {cli:?}");
    Ok(())
}
