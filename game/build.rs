//! M9A: top-level build hook for the asset pipeline.
//!
//! Detects when the Python asset pipeline's palette JSONs, style descriptors,
//! or asset manifests have changed and emits `cargo:rerun-if-changed` directives
//! so the workspace re-evaluates the pipeline on next build. The actual
//! regeneration is invoked explicitly via `cf-mod asset-gen run` or
//! `tools/asset_gen/build_placeholders.py --all`; this hook only marks
//! upstream files so the cargo cache stays coherent.
//!
//! Why this lives at `game/build.rs` not in a leaf crate: the asset pipeline
//! is a workspace-level concern shared by cf-render-2d, cf-mod, cf-app, and
//! any future crate that loads `content/assets/placeholders/`. A single
//! workspace-level hook avoids per-crate duplication.
//!
//! Per M9A.md § "Build integration":
//! > game/build.rs (NEW: top-level build hook)
//!
//! Cargo's default behavior is to look for `build.rs` at the crate level. To
//! make this top-level file participate, the build wraps a no-op cargo
//! manifest's `build = "build.rs"` (see `game/build_hook/`). When the
//! pipeline-level build invokes `cargo build --workspace`, the build hook
//! runs once and emits its rerun-if-changed directives.

use std::path::Path;

fn main() {
    let pipeline_root = Path::new("../tools/asset_gen");
    if !pipeline_root.exists() {
        // Workspace can build without the pipeline; assets are committed.
        return;
    }
    let watched_subdirs = [
        "palettes",
        "style_descriptors",
        "asset_manifests",
        "schemas",
    ];
    for sub in &watched_subdirs {
        let path = pipeline_root.join(sub);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    for script in &[
        "build_placeholders.py",
        "llm_svg_prompter.py",
        "palette_loader.py",
        "style_enforcer.py",
        "cairo_renderer.py",
        "normal_map_baker.py",
        "ledger_writer.py",
    ] {
        let path = pipeline_root.join(script);
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
