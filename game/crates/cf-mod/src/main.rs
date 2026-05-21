#![allow(clippy::items_after_test_module)]

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Cmd};
use crate::runners::{run_asset_gen, run_audio_gen, run_ledger, run_save, run_validate, run_validate_bundle};

mod bundle_chain_verify;
mod cli;
mod report;
mod runners;
mod save_validate;
mod validate;

#[cfg(test)]
mod test_helpers;

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::mod", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    match &cli.command {
        Cmd::Validate { paths } => run_validate(paths, cli.strict, cli.json),
        Cmd::ValidateBundle { bundle_dir } => run_validate_bundle(bundle_dir, cli.json),
        Cmd::Build { pkg_dir } => {
            anyhow::bail!(
                "cf-mod build is not implemented in M0; package builder lands at M5/M8 (got {})",
                pkg_dir.display()
            );
        }
        Cmd::Inspect { cfpkg } => {
            anyhow::bail!(
                "cf-mod inspect is not implemented in M0; package format lands at M8 (got {})",
                cfpkg.display()
            );
        }
        Cmd::Ledger { action } => run_ledger(action.as_ref(), cli.strict, cli.json),
        Cmd::AssetGen { action } => run_asset_gen(action.as_ref(), cli.json),
        Cmd::AudioGen { action } => run_audio_gen(action.as_ref(), cli.json),
        Cmd::Save { action } => run_save(action, cli.json),
    }
}
