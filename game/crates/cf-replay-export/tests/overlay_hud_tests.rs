//! M10B HUD overlay integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export overlay_hud`
//! (expect: hud_layer_toggles PASS).
//!
//! VAL-M10B-OVERLAY-HUD-FILE: composition graph contains/omits the
//! `hud` layer per `--overlay hud` / `--no-overlay hud` flag.

use cf_replay_export::overlay_graph::{OverlayGraphBuilder, HUD_OVERLAY_NAME, HUD_Z_ORDER};

#[test]
fn overlay_hud_hud_layer_toggles() {
    let graph_on = OverlayGraphBuilder::new()
        .enable(HUD_OVERLAY_NAME)
        .build()
        .expect("enable hud must build");
    assert!(graph_on.contains(HUD_OVERLAY_NAME), "hud layer must be present");
    let layer = graph_on.layer(HUD_OVERLAY_NAME).unwrap();
    assert_eq!(layer.z_order, HUD_Z_ORDER);

    let graph_off = OverlayGraphBuilder::new()
        .disable(HUD_OVERLAY_NAME)
        .build()
        .expect("disable hud must build");
    assert!(!graph_off.contains(HUD_OVERLAY_NAME), "hud layer must be omitted");
}

#[test]
fn overlay_hud_default_graph_contains_hud() {
    let graph = OverlayGraphBuilder::new().build().expect("default graph must build");
    assert!(graph.contains(HUD_OVERLAY_NAME));
}
