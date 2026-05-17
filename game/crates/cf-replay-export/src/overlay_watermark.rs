//! M10B tournament-watermark overlay.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Per-replay watermark.** Tournament + competitive modes embed
//! > run_id + ledger chain anchor + Corefall build version as a corner
//! > watermark, proving the clip's provenance for dispute resolution.
//!
//! Spec § "Notes for the implementer":
//!
//! > Watermark renderer uses `cf-localization` for the build-version
//! > text + falls back to en-US if no locale is loaded. Tournament
//! > watermark format: `run=<run_id[..12]> anchor=<chain_anchor[..12]>
//! > build=<build_version>` rendered in 12pt monospace at 50% opacity,
//! > bottom-right corner.
//!
//! VAL-M10B-031: "the output MP4's bottom-right corner pixel region
//! contains text matching the pattern `run=[0-9a-f]{12} anchor=[0-9a-f]{12}
//! build=[A-Za-z0-9.\-]+` in EVERY decoded frame ... a separate
//! third-party verification step parses the rendered text and asserts
//! the embedded `anchor` equals `bundle.ledger.chain_anchor[..12]` and
//! the `run` equals `bundle.manifest.run_id[..12]`."

use crate::overlay_graph::{WATERMARK_OVERLAY_NAME, WATERMARK_Z_ORDER};

/// Truncate field length per spec § Notes (`run_id[..12]`,
/// `chain_anchor[..12]`).
pub const WATERMARK_FIELD_TRUNCATE: usize = 12;

/// Default watermark AOI at 1920×1080. Bottom-right corner.
pub const WATERMARK_AOI_X: u32 = 1920 - 16 - 600;
pub const WATERMARK_AOI_Y: u32 = 1080 - 16 - 36;
pub const WATERMARK_AOI_WIDTH: u32 = 600;
pub const WATERMARK_AOI_HEIGHT: u32 = 36;

/// Bundle-derived provenance fields. The export pipeline pulls these
/// from `bundle.manifest.run_id` + `bundle.ledger.chain_anchor` +
/// `bundle.build.version`. m10b-4 wires the bundle loader to
/// construct one of these per export job; m10b-3 here ships the
/// rendering rule alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkProvenance {
    pub run_id: String,
    pub chain_anchor: String,
    pub build_version: String,
}

impl WatermarkProvenance {
    /// Truncate every field to the spec's 12-character limit (build
    /// version is uncapped per the regex `build=[A-Za-z0-9.\-]+` in
    /// VAL-M10B-031 — the spec only specifies the length cap for
    /// `run` + `anchor`).
    #[must_use]
    pub fn truncated(&self) -> Self {
        Self {
            run_id: truncate(&self.run_id, WATERMARK_FIELD_TRUNCATE),
            chain_anchor: truncate(&self.chain_anchor, WATERMARK_FIELD_TRUNCATE),
            build_version: self.build_version.clone(),
        }
    }

    /// Format the bottom-right watermark text per VAL-M10B-031's
    /// pattern: `run=<run> anchor=<anchor> build=<build>`.
    #[must_use]
    pub fn format_line(&self) -> String {
        let t = self.truncated();
        format!(
            "run={} anchor={} build={}",
            t.run_id, t.chain_anchor, t.build_version
        )
    }

    /// Verify a candidate watermark line against the provenance
    /// expected from the bundle. Returns `true` when the line's
    /// `run=...` and `anchor=...` fields equal the truncated values
    /// from `bundle.manifest.run_id[..12]` and
    /// `bundle.ledger.chain_anchor[..12]` respectively.
    #[must_use]
    pub fn verify_line(&self, line: &str) -> bool {
        let expected = self.format_line();
        expected == line
    }
}

/// Convenience: bytewise UTF-8-safe truncate, never panics on
/// non-ASCII (hex strings are pure ASCII but we keep the helper
/// defensive against build-version oddities).
fn truncate(s: &str, n: usize) -> String {
    let mut out = String::with_capacity(n);
    for (i, ch) in s.chars().enumerate() {
        if i == n {
            break;
        }
        out.push(ch);
    }
    out
}

/// The watermark overlay descriptor. Per-frame rendering is constant
/// — the same provenance line is composited into every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkOverlay {
    pub aoi_x: u32,
    pub aoi_y: u32,
    pub aoi_width: u32,
    pub aoi_height: u32,
    pub z_order: u32,
    pub provenance: WatermarkProvenance,
}

impl WatermarkOverlay {
    #[must_use]
    pub const fn name() -> &'static str {
        WATERMARK_OVERLAY_NAME
    }

    #[must_use]
    pub fn new(provenance: WatermarkProvenance) -> Self {
        Self {
            aoi_x: WATERMARK_AOI_X,
            aoi_y: WATERMARK_AOI_Y,
            aoi_width: WATERMARK_AOI_WIDTH,
            aoi_height: WATERMARK_AOI_HEIGHT,
            z_order: WATERMARK_Z_ORDER,
            provenance,
        }
    }

    /// The provenance line composited into every frame.
    #[must_use]
    pub fn line(&self) -> String {
        self.provenance.format_line()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> WatermarkProvenance {
        WatermarkProvenance {
            run_id: "abcdef0123456789".into(),
            chain_anchor: "fedcba9876543210ffffeeee".into(),
            build_version: "0.0.1-mission".into(),
        }
    }

    #[test]
    fn watermark_truncates_run_and_anchor_to_12_chars() {
        let p = provenance().truncated();
        assert_eq!(p.run_id.len(), 12);
        assert_eq!(p.chain_anchor.len(), 12);
        assert_eq!(p.run_id, "abcdef012345");
        assert_eq!(p.chain_anchor, "fedcba987654");
    }

    #[test]
    fn watermark_line_matches_spec_pattern() {
        let line = provenance().format_line();
        // run=[0-9a-f]{12} anchor=[0-9a-f]{12} build=[A-Za-z0-9.\-]+
        assert!(line.starts_with("run=abcdef012345 anchor=fedcba987654 build="));
        assert!(line.ends_with("0.0.1-mission"));
    }

    #[test]
    fn watermark_verify_line_matches_expected() {
        let p = provenance();
        let line = p.format_line();
        assert!(p.verify_line(&line));
        assert!(!p.verify_line("run=xxxx anchor=yyyy build=zzzz"));
    }

    #[test]
    fn watermark_overlay_appears_in_every_frame() {
        // The overlay is a constant pass — same line on every frame.
        // Test models "every frame" by querying 60*60=3600 ticks +
        // asserting the line is byte-equal each time.
        let overlay = WatermarkOverlay::new(provenance());
        let first = overlay.line();
        for _ in 0..3600 {
            assert_eq!(overlay.line(), first);
        }
    }
}
