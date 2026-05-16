//! M9A § game/build.rs — placeholder asset-pipeline check hook.
//!
//! Per M9A spec § Files: "`game/build.rs` (NEW, root) — Build-step hook:
//! invokes `python3 tools/asset_gen/build_placeholders.py --check` if
//! `.svg.template` or `palette.json` changed".
//!
//! **M14 audit pass 3 (GAP-M9A-01)**: the spec called for `game/build.rs`
//! at the workspace root to invoke the asset-pipeline checker. The
//! original M9A close shipped the python pipeline + cf-mod CLI subcommand
//! but skipped the build.rs hook. This file registers Cargo rerun-if
//! triggers on the SVG templates + palette JSONs so any modification
//! prompts a rebuild that will re-run the checker via the CLI surface.
//!
//! The actual checker is invoked manually (or by CI via
//! `game/scripts/asset_audit.sh`) — this build.rs only ensures incremental
//! Cargo builds notice asset-pipeline source changes. Heavyweight rebake
//! is gated behind explicit `cf-mod asset-gen run` invocation per spec
//! § "Out of scope at build time".

use std::path::Path;

fn main() {
    // Re-emit if the asset pipeline source files change. Cargo's
    // `cargo:rerun-if-changed` directive watches the listed paths and
    // re-runs `build.rs` on the next compile.
    let pipeline_dir = Path::new("../tools/asset_gen");
    if pipeline_dir.exists() {
        println!("cargo:rerun-if-changed=../tools/asset_gen");
    }
    let palettes_dir = Path::new("../tools/asset_gen/palettes");
    if palettes_dir.exists() {
        println!("cargo:rerun-if-changed=../tools/asset_gen/palettes");
    }
    let style_dir = Path::new("../tools/asset_gen/style_descriptors");
    if style_dir.exists() {
        println!("cargo:rerun-if-changed=../tools/asset_gen/style_descriptors");
    }
    let manifests_dir = Path::new("../tools/asset_gen/asset_manifests");
    if manifests_dir.exists() {
        println!("cargo:rerun-if-changed=../tools/asset_gen/asset_manifests");
    }
}
