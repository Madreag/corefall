//! M10B overlay composition graph.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Overlay composition is granular.** Author toggles HUD, kill-feed,
//! > mini-map, debug overlay, captions, accessibility ribbons, watermark,
//! > branded streamer template, and chapter timeline independently
//! > per-export; OBS-style scene compositor at offline frame cadence.
//!
//! Spec § "Notes for the implementer":
//!
//! > Mod overlay extension: mods declare overlays in their manifest
//! > under `overlays.<name>: { z_order, dyn_lib_entry_point }`; M10B
//! > looks up the entry point at export time and asks for per-tick
//! > render commands.
//!
//! VAL-M10B-034: "A test fixture mod declares
//! `overlays.custom_kill_feed: { z_order: 50, dyn_lib_entry_point:
//! "fixture_kill_feed_overlay" }` (where `kill_feed` core layer z_order
//! = 40, `watermark` core z_order = 60). Running export with
//! `--overlay custom_kill_feed --mod-load <fixture>` produces an output
//! MP4 whose per-frame composition graph (logged via `tracing` at
//! `cf-replay-export::overlay_graph`) lists layers in order `...
//! kill_feed (40) → custom_kill_feed (50) → watermark (60) ...`."
//!
//! VAL-M10B-OVERLAY-HUD-FILE / VAL-M10B-OVERLAY-KILLFEED-FILE /
//! VAL-M10B-OVERLAY-CHAPTERTL-FILE: layers toggle via `--overlay
//! <name>` (or `--no-overlay <name>`).
//!
//! The graph itself is data — `Vec<OverlayLayer>` sorted by `z_order`.
//! The per-layer rendering happens in the per-overlay modules
//! ([`crate::overlay_hud`], [`crate::overlay_kill_feed`],
//! [`crate::overlay_cause_chain`], [`crate::overlay_chapter_timeline`],
//! [`crate::overlay_watermark`]). The composition pass is a
//! deterministic stable-sort over `(z_order, declaration_order)` so
//! repeated runs emit identical layer lists per VAL-M10B-026.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

/// `tracing` event target. VAL-M10B-034 evidence inspects logs for
/// this target prefix.
pub const OVERLAY_GRAPH_TRACE_TARGET: &str = "cf-replay-export::overlay_graph";

/// Five canonical overlay names. Mod overlays add additional entries
/// with their own `z_order`; the composition graph sorts the union.
pub const HUD_OVERLAY_NAME: &str = "hud";
pub const KILL_FEED_OVERLAY_NAME: &str = "kill_feed";
pub const CAUSE_CHAIN_OVERLAY_NAME: &str = "cause_chain";
pub const CHAPTER_TIMELINE_OVERLAY_NAME: &str = "chapter_timeline";
pub const WATERMARK_OVERLAY_NAME: &str = "watermark";

/// Canonical z-orders for the five core layers. The values match the
/// VAL-M10B-034 contract: `kill_feed (40) → custom_kill_feed (50) →
/// watermark (60)`. The remaining three (`hud`, `cause_chain`,
/// `chapter_timeline`) sit elsewhere on the stack so a mod overlay can
/// slot between any two cores via its declared `z_order`.
pub const HUD_Z_ORDER: u32 = 10;
pub const CAUSE_CHAIN_Z_ORDER: u32 = 20;
pub const KILL_FEED_Z_ORDER: u32 = 40;
pub const WATERMARK_Z_ORDER: u32 = 60;
pub const CHAPTER_TIMELINE_Z_ORDER: u32 = 70;

/// Source of an overlay layer: a core M10B-shipped layer, OR a
/// mod-declared overlay that plugs in via a manifest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlaySource {
    /// One of the five core overlays. Identifies the renderer by name.
    Core,
    /// Mod-declared overlay. Carries the declared entry point so the
    /// audit log can record where the pixels came from.
    Mod {
        /// Manifest-declared `dyn_lib_entry_point` value.
        dyn_lib_entry_point: String,
    },
}

/// One composition layer. The compositor sorts these by
/// `(z_order, declaration_index)` so the per-frame output is
/// deterministic regardless of input order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayLayer {
    /// Stable identifier (`hud`, `kill_feed`, `custom_kill_feed`, ...).
    pub name: String,
    /// Z-order; lower values composite first (underneath higher ones).
    pub z_order: u32,
    /// Where this layer came from (core M10B vs mod).
    pub source: OverlaySource,
}

impl OverlayLayer {
    #[must_use]
    pub fn core(name: &str, z_order: u32) -> Self {
        Self {
            name: name.to_owned(),
            z_order,
            source: OverlaySource::Core,
        }
    }

    #[must_use]
    pub fn mod_layer(name: &str, z_order: u32, dyn_lib_entry_point: &str) -> Self {
        Self {
            name: name.to_owned(),
            z_order,
            source: OverlaySource::Mod {
                dyn_lib_entry_point: dyn_lib_entry_point.to_owned(),
            },
        }
    }
}

/// Overlay composition graph. Built by [`OverlayGraphBuilder`] which
/// resolves the user's `--overlay <name>` / `--no-overlay <name>` /
/// `--mod-load <fixture>` flags against the core + mod layer
/// declarations. The compositor walks `layers` in z_order to draw
/// per-frame RGBA pixels.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OverlayGraph {
    pub layers: Vec<OverlayLayer>,
}

impl OverlayGraph {
    /// `true` if a layer with the given name is in the composition.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.layers.iter().any(|l| l.name == name)
    }

    /// Look up a layer by name.
    #[must_use]
    pub fn layer(&self, name: &str) -> Option<&OverlayLayer> {
        self.layers.iter().find(|l| l.name == name)
    }

    /// Emit a `tracing::info!` line per layer, in z_order. Tests
    /// inspect this via the `tracing::subscriber::with_default` test
    /// harness; the production export job (m10b-4) propagates the same
    /// trace lines to its audit log.
    pub fn emit_trace(&self) {
        for layer in &self.layers {
            info!(
                target: OVERLAY_GRAPH_TRACE_TARGET,
                name = %layer.name,
                z_order = layer.z_order,
                source = %match &layer.source {
                    OverlaySource::Core => "core".to_owned(),
                    OverlaySource::Mod { dyn_lib_entry_point } => {
                        format!("mod:{dyn_lib_entry_point}")
                    }
                },
                "overlay layer composed"
            );
        }
    }

    /// Emit a `tracing::warn!` line declaring that some core layer was
    /// unloaded (e.g. on uninstall of a mod). VAL-M10B-034 requires
    /// the post-uninstall path to emit ZERO warning lines containing
    /// `orphan render command`; this function intentionally NEVER
    /// emits that phrase — see [`Self::emit_orphan_warning`] (which is
    /// only called when the graph is in a genuinely-broken state).
    pub fn emit_clean_uninstall(&self) {
        info!(
            target: OVERLAY_GRAPH_TRACE_TARGET,
            layer_count = self.layers.len(),
            "overlay graph rebuilt cleanly after mod uninstall"
        );
    }

    /// Emit the orphan-render warning. The compositor calls this only
    /// when a layer's renderer cannot be resolved (e.g. a previously
    /// loaded mod overlay is referenced but the mod is gone). Tests
    /// for the clean-uninstall path assert this is NEVER called.
    pub fn emit_orphan_warning(&self, layer_name: &str) {
        warn!(
            target: OVERLAY_GRAPH_TRACE_TARGET,
            layer = layer_name,
            "orphan render command emitted for unresolved overlay layer"
        );
    }
}

/// Builder for an [`OverlayGraph`]. The builder takes the user's
/// `--overlay` / `--no-overlay` flag set + the loaded mod manifest
/// declarations and produces a deterministic, z-order-sorted graph.
#[derive(Debug, Clone, Default)]
pub struct OverlayGraphBuilder {
    enabled: Vec<String>,
    disabled: Vec<String>,
    mods: Vec<ModOverlayDeclaration>,
}

/// Mod overlay manifest entry per spec § Notes
/// (`overlays.<name>: { z_order, dyn_lib_entry_point }`). Mod loading
/// is m10b-4's concern; this struct is the data shape the loader
/// produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModOverlayDeclaration {
    pub name: String,
    pub z_order: u32,
    pub dyn_lib_entry_point: String,
}

impl OverlayGraphBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark an overlay name as enabled (`--overlay <name>`). Returns
    /// self for fluent construction.
    #[must_use]
    pub fn enable(mut self, name: &str) -> Self {
        self.enabled.push(name.to_owned());
        self
    }

    /// Mark an overlay name as disabled (`--no-overlay <name>`).
    #[must_use]
    pub fn disable(mut self, name: &str) -> Self {
        self.disabled.push(name.to_owned());
        self
    }

    /// Register a mod overlay declaration loaded from a mod manifest
    /// (`--mod-load <fixture>` resolved to a parsed manifest entry).
    #[must_use]
    pub fn with_mod_overlay(mut self, decl: ModOverlayDeclaration) -> Self {
        self.mods.push(decl);
        self
    }

    /// Build the composition graph.
    ///
    /// Resolution rules (deterministic):
    /// 1. Start with the five core overlays at their declared
    ///    z_orders.
    /// 2. Append every mod overlay whose name appears in the enabled
    ///    set (or whose manifest declaration requested auto-enable —
    ///    not exercised here; m10b-4 wires the flag pass).
    /// 3. Apply the `--no-overlay <name>` disable filter.
    /// 4. Apply the `--overlay <name>` enable filter: a core layer is
    ///    only present in the graph if `enabled` is empty (default-on)
    ///    OR the layer name is in `enabled`.
    /// 5. Stable-sort by `(z_order, original declaration index)` so
    ///    same-z_order layers preserve their declaration order.
    pub fn build(self) -> Result<OverlayGraph, OverlayGraphError> {
        let mut candidates: Vec<OverlayLayer> = Vec::with_capacity(5 + self.mods.len());
        candidates.push(OverlayLayer::core(HUD_OVERLAY_NAME, HUD_Z_ORDER));
        candidates.push(OverlayLayer::core(CAUSE_CHAIN_OVERLAY_NAME, CAUSE_CHAIN_Z_ORDER));
        candidates.push(OverlayLayer::core(KILL_FEED_OVERLAY_NAME, KILL_FEED_Z_ORDER));
        candidates.push(OverlayLayer::core(WATERMARK_OVERLAY_NAME, WATERMARK_Z_ORDER));
        candidates.push(OverlayLayer::core(
            CHAPTER_TIMELINE_OVERLAY_NAME,
            CHAPTER_TIMELINE_Z_ORDER,
        ));
        for decl in &self.mods {
            candidates.push(OverlayLayer::mod_layer(
                &decl.name,
                decl.z_order,
                &decl.dyn_lib_entry_point,
            ));
        }
        for enabled in &self.enabled {
            let name = enabled.as_str();
            let is_known = candidates.iter().any(|c| c.name == name);
            if !is_known {
                return Err(OverlayGraphError::UnknownOverlay {
                    name: enabled.clone(),
                });
            }
        }
        let mut layers: Vec<(usize, OverlayLayer)> = Vec::with_capacity(candidates.len());
        for (index, layer) in candidates.into_iter().enumerate() {
            let is_enabled = if self.enabled.is_empty() {
                !self.disabled.iter().any(|d| d == &layer.name)
            } else {
                self.enabled.iter().any(|e| e == &layer.name)
                    && !self.disabled.iter().any(|d| d == &layer.name)
            };
            if is_enabled {
                layers.push((index, layer));
            }
        }
        layers.sort_by_key(|(idx, layer)| (layer.z_order, *idx));
        let layers: Vec<OverlayLayer> = layers.into_iter().map(|(_, l)| l).collect();
        Ok(OverlayGraph { layers })
    }
}

/// Errors raised by the overlay-graph builder. Typed-error rejection
/// per the spec § Notes "no panic on malformed input" contract.
#[derive(Debug, Error)]
pub enum OverlayGraphError {
    #[error("unknown overlay name `{name}` — must be one of: hud / kill_feed / cause_chain / chapter_timeline / watermark / a declared mod overlay")]
    UnknownOverlay { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_graph_composes_five_core_layers_in_z_order() {
        let graph = OverlayGraphBuilder::new().build().expect("build");
        let names: Vec<&str> = graph.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                HUD_OVERLAY_NAME,
                CAUSE_CHAIN_OVERLAY_NAME,
                KILL_FEED_OVERLAY_NAME,
                WATERMARK_OVERLAY_NAME,
                CHAPTER_TIMELINE_OVERLAY_NAME,
            ],
            "layers must compose in z-order"
        );
    }

    #[test]
    fn hud_layer_toggles_via_enable_flag() {
        let graph_on = OverlayGraphBuilder::new().enable(HUD_OVERLAY_NAME).build().unwrap();
        assert!(graph_on.contains(HUD_OVERLAY_NAME));
        assert!(!graph_on.contains(KILL_FEED_OVERLAY_NAME));

        let graph_off = OverlayGraphBuilder::new().disable(HUD_OVERLAY_NAME).build().unwrap();
        assert!(!graph_off.contains(HUD_OVERLAY_NAME));
        assert!(graph_off.contains(KILL_FEED_OVERLAY_NAME));
    }

    #[test]
    fn mod_overlay_slots_in_by_declared_z_order() {
        let graph = OverlayGraphBuilder::new()
            .with_mod_overlay(ModOverlayDeclaration {
                name: "custom_kill_feed".into(),
                z_order: 50,
                dyn_lib_entry_point: "fixture_kill_feed_overlay".into(),
            })
            .build()
            .unwrap();
        let names: Vec<&str> = graph.layers.iter().map(|l| l.name.as_str()).collect();
        let idx_kill = names.iter().position(|n| *n == KILL_FEED_OVERLAY_NAME).unwrap();
        let idx_custom = names.iter().position(|n| *n == "custom_kill_feed").unwrap();
        let idx_watermark = names.iter().position(|n| *n == WATERMARK_OVERLAY_NAME).unwrap();
        assert!(idx_kill < idx_custom, "custom_kill_feed must come after kill_feed");
        assert!(
            idx_custom < idx_watermark,
            "custom_kill_feed must come before watermark"
        );
    }

    #[test]
    fn unknown_overlay_name_returns_typed_error() {
        let err = OverlayGraphBuilder::new().enable("does_not_exist").build().unwrap_err();
        assert!(matches!(err, OverlayGraphError::UnknownOverlay { name } if name == "does_not_exist"));
    }

    #[test]
    fn build_is_deterministic_across_repeated_calls() {
        let a = OverlayGraphBuilder::new().build().unwrap();
        let b = OverlayGraphBuilder::new().build().unwrap();
        assert_eq!(a, b);
    }
}
