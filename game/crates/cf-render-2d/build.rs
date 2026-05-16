//! cf-render-2d build hook — M9A asset pipeline tracking.
//!
//! Mirrors the workspace-level intent declared in `game/build.rs` (which
//! cargo ignores at the workspace root). When the M9A pipeline's palette
//! JSONs, style descriptors, or asset manifests change, this hook emits
//! `cargo:rerun-if-changed` directives so the cf-render-2d build cache
//! invalidates and the placeholder SVGs are reloaded on next launch.
//!
//! The hook itself never regenerates assets — that's the job of
//! `tools/asset_gen/build_placeholders.py` invoked via
//! `cf-mod asset-gen run` or `game/scripts/regen_all_assets.sh`.

use std::path::PathBuf;

fn main() {
    let pipeline_root = PathBuf::from("../../../tools/asset_gen");
    if !pipeline_root.exists() {
        return;
    }
    for sub in &["palettes", "style_descriptors", "asset_manifests", "schemas"] {
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
    let content_assets = PathBuf::from("../../../content/assets/placeholders");
    if content_assets.exists() {
        println!("cargo:rerun-if-changed={}", content_assets.display());
    }
}
