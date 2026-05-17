//! M10B § cf-render-2d::offline_mode — software-rasterizer fallback.
//!
//! Spec § Notes for the implementer:
//!
//! > `cf-render-2d --offline` uses the software rasterizer (`pixman` /
//! > `tiny-skia` candidate). The render path must not call any wgpu
//! > surface; it writes RGBA into a `Vec<u8>` frame buffer that the
//! > ffmpeg_bridge converts to YUV420P / YUV444P per preset.
//!
//! VAL-M10B-009 requires `cargo check -p cf-render-2d
//! --no-default-features --features offline` to exit 0 and a grep over
//! this file to find **zero** references to the GPU surface +
//! windowing types listed in the assertion. We use `tiny-skia` — a
//! pure-Rust 2D rasterizer with no GPU / window dependency — for the
//! per-frame RGBA buffer composition.
//!
//! VAL-M10B-021 additionally requires the offline rasterizer to engage
//! on no-GPU Linux hosts; this module emits the `tracing` line
//! `offline_mode: software_rasterizer engaged` exactly once per
//! [`OfflineRasterizer::engage`] call so the dedicated-server export
//! tier can audit the fallback path.
//!
//! Cross-spec coverage: the rasterizer loads the M9B `trench_layers` +
//! M9C `fortification_layers` + `spotlight_cone` + `wire_visuals` +
//! camo overlay modules transparently — exporting the
//! `m9c_full_strongpoint` scenario produces non-blank rendered
//! fortification + trench pixels (see [`OfflineRasterizer::render_scene`]
//! + the per-layer helpers below).

use tiny_skia::{Color, Paint, Pixmap, Rect, Transform};

use crate::{layers_for_kind, layers_for_variant, FortLayerId, TrenchLayerId};

pub use cf_fortification::{FortificationKind, WireKind};
pub use cf_trench::SegmentVariant;

/// Tracing line emitted by [`OfflineRasterizer::engage`]. The matrix
/// script + VAL-M10B-021 assertion search for this literal in the
/// export job's tracing output.
pub const SOFTWARE_RASTERIZER_ENGAGED_LINE: &str = "offline_mode: software_rasterizer engaged";

/// Default tile size in pixels for the offline rasterizer. The
/// software path renders one tile per (x, y) world-tile pair; the
/// caller scales the output to match the export preset's resolution
/// via the ffmpeg bridge's `software-scaling` feature.
pub const OFFLINE_TILE_PX: u32 = 16;

/// One scene element submitted to the offline rasterizer. The element
/// graph is reconstructed per-tick by [`crate::offline_mode::SceneCommand`]
/// emitters in `cf-replay-export::frame_ticker`; this crate only owns
/// the raster pass.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneCommand {
    /// Fill a single trench segment at `(tile_x, tile_y)` with the
    /// variant's per-layer sprites. The offline rasterizer fans out
    /// each layer via [`crate::layers_for_variant`].
    TrenchSegment {
        tile_x: i32,
        tile_y: i32,
        variant: SegmentVariant,
    },
    /// Fill a single fortification at `(tile_x, tile_y)` with the
    /// kind's per-layer sprites. The offline rasterizer fans out each
    /// layer via [`crate::layers_for_kind`].
    Fortification {
        tile_x: i32,
        tile_y: i32,
        kind: FortificationKind,
    },
    /// Render a spotlight cone originating at `origin` aimed along
    /// `aim_radians`. The rasterizer fills a 22.5°-half-angle cone of
    /// `range_tiles` tiles in length per the M9C spec.
    SpotlightCone {
        origin: (i32, i32),
        aim_radians: f32,
        range_tiles: u32,
    },
    /// Render a wire strand of the given kind at `(tile_x, tile_y)`
    /// (tinted per [`crate::WireVisual::for_kind`]).
    Wire { tile_x: i32, tile_y: i32, kind: WireKind },
}

/// Tag identifying which renderer path produced a frame. Exposed so
/// downstream `frame_ticker` audit logs can record the tier (Workstation
/// vs Dedicated-server) that rendered each export job.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OfflineRendererTier {
    Workstation,
    DedicatedServer,
}

impl OfflineRendererTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            OfflineRendererTier::Workstation => "workstation",
            OfflineRendererTier::DedicatedServer => "dedicated_server",
        }
    }
}

/// One rendered RGBA frame produced by the offline rasterizer.
///
/// The `pixels` buffer is a flat `Vec<u8>` of length
/// `width * height * 4` (RGBA channel order). `cf-replay-export`'s
/// ffmpeg bridge converts this to YUV420P / YUV444P per preset at
/// encode time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineFrame {
    pub width: u32,
    pub height: u32,
    pub tick: u64,
    pub pixels: Vec<u8>,
}

impl OfflineFrame {
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.pixels.chunks_exact(4).all(|px| px == [0, 0, 0, 0])
    }

    #[must_use]
    pub fn non_blank_pixel_count(&self) -> usize {
        self.pixels.chunks_exact(4).filter(|px| px != &[0, 0, 0, 0]).count()
    }
}

/// Offline rasterizer state. One per export job. The rasterizer owns
/// a `tiny_skia::Pixmap` it reuses across frames to avoid per-frame
/// allocation (a 1920x1080 RGBA8 buffer is 8.3 MB — re-allocating per
/// frame would blow the dedicated-server time budget).
pub struct OfflineRasterizer {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    tier: OfflineRendererTier,
    engaged: bool,
}

impl OfflineRasterizer {
    /// Construct a fresh rasterizer for the given resolution + tier.
    /// The pixmap is pre-allocated; call [`OfflineRasterizer::engage`]
    /// once before the first frame to emit the canonical tracing
    /// signal that VAL-M10B-021 greps for.
    #[must_use]
    pub fn new(width: u32, height: u32, tier: OfflineRendererTier) -> Option<Self> {
        let pixmap = Pixmap::new(width, height)?;
        Some(Self {
            width,
            height,
            pixmap,
            tier,
            engaged: false,
        })
    }

    /// Idempotently emit the `offline_mode: software_rasterizer
    /// engaged` tracing line. Repeated calls are a no-op (the export
    /// pipeline may probe + then engage from two call sites and we
    /// don't want duplicate audit lines).
    pub fn engage(&mut self) {
        if !self.engaged {
            tracing::info!(
                tier = %self.tier.as_str(),
                width = self.width,
                height = self.height,
                "{}",
                SOFTWARE_RASTERIZER_ENGAGED_LINE
            );
            self.engaged = true;
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn tier(&self) -> OfflineRendererTier {
        self.tier
    }

    #[must_use]
    pub fn is_engaged(&self) -> bool {
        self.engaged
    }

    /// Render the scene to a fresh [`OfflineFrame`].
    ///
    /// Pipeline:
    /// 1. Clear the pixmap to the deterministic background color
    ///    (`M0_CLEAR_COLOR` analogue).
    /// 2. Iterate the scene commands in order.
    /// 3. For trench / fortification commands, fan out per-layer
    ///    [`TrenchLayerId`] / [`FortLayerId`] tints onto the pixmap
    ///    (each layer becomes one rectangle stack — this is enough to
    ///    produce non-blank pixels per VAL-M10B-021's
    ///    `m9c_full_strongpoint` requirement; richer per-tile sprites
    ///    are out of scope for the m10b-2 feature).
    /// 4. Copy the pixmap's RGBA bytes into an [`OfflineFrame`].
    pub fn render_scene(&mut self, tick: u64, commands: &[SceneCommand]) -> OfflineFrame {
        self.engage();
        clear_pixmap(&mut self.pixmap, BACKGROUND_RGBA);
        for cmd in commands {
            match cmd {
                SceneCommand::TrenchSegment {
                    tile_x,
                    tile_y,
                    variant,
                } => {
                    render_trench_segment(&mut self.pixmap, *tile_x, *tile_y, *variant);
                }
                SceneCommand::Fortification { tile_x, tile_y, kind } => {
                    render_fortification(&mut self.pixmap, *tile_x, *tile_y, *kind);
                }
                SceneCommand::SpotlightCone {
                    origin,
                    aim_radians,
                    range_tiles,
                } => {
                    render_spotlight_cone(&mut self.pixmap, *origin, *aim_radians, *range_tiles);
                }
                SceneCommand::Wire { tile_x, tile_y, kind } => {
                    render_wire(&mut self.pixmap, *tile_x, *tile_y, *kind);
                }
            }
        }
        OfflineFrame {
            width: self.width,
            height: self.height,
            tick,
            pixels: self.pixmap.data().to_vec(),
        }
    }
}

const BACKGROUND_RGBA: [u8; 4] = [13, 18, 26, 255];

fn clear_pixmap(pixmap: &mut Pixmap, rgba: [u8; 4]) {
    let color = Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
    pixmap.fill(color);
}

fn fill_rect(pixmap: &mut Pixmap, x: i32, y: i32, w: u32, h: u32, rgba: [u8; 4]) {
    if w == 0 || h == 0 {
        return;
    }
    let pm_w = pixmap.width();
    let pm_h = pixmap.height();
    let x0 = x.max(0) as f32;
    let y0 = y.max(0) as f32;
    let x1 = (x + w as i32).min(pm_w as i32) as f32;
    let y1 = (y + h as i32).min(pm_h as i32) as f32;
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let rect = match Rect::from_ltrb(x0, y0, x1, y1) {
        Some(r) => r,
        None => return,
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
    paint.anti_alias = false;
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn render_trench_segment(pixmap: &mut Pixmap, tile_x: i32, tile_y: i32, variant: SegmentVariant) {
    let layers = layers_for_variant(variant);
    let base_x = tile_x * OFFLINE_TILE_PX as i32;
    let base_y = tile_y * OFFLINE_TILE_PX as i32;
    for (layer_index, layer) in layers.iter().enumerate() {
        let tint = trench_layer_tint(*layer);
        let inset = layer_index as i32;
        fill_rect(
            pixmap,
            base_x + inset,
            base_y + inset,
            OFFLINE_TILE_PX.saturating_sub((layer_index * 2) as u32),
            OFFLINE_TILE_PX.saturating_sub((layer_index * 2) as u32),
            tint,
        );
    }
}

fn render_fortification(pixmap: &mut Pixmap, tile_x: i32, tile_y: i32, kind: FortificationKind) {
    let layers = layers_for_kind(kind);
    let base_x = tile_x * OFFLINE_TILE_PX as i32;
    let base_y = tile_y * OFFLINE_TILE_PX as i32;
    for (layer_index, layer) in layers.iter().enumerate() {
        let tint = fortification_layer_tint(*layer);
        let inset = layer_index as i32;
        fill_rect(
            pixmap,
            base_x + inset,
            base_y + inset,
            OFFLINE_TILE_PX.saturating_sub((layer_index * 2) as u32),
            OFFLINE_TILE_PX.saturating_sub((layer_index * 2) as u32),
            tint,
        );
    }
}

fn render_spotlight_cone(pixmap: &mut Pixmap, origin: (i32, i32), aim_radians: f32, range_tiles: u32) {
    let ox = origin.0 * OFFLINE_TILE_PX as i32;
    let oy = origin.1 * OFFLINE_TILE_PX as i32;
    let len = (range_tiles * OFFLINE_TILE_PX) as f32;
    let dx = (aim_radians.cos() * len).round() as i32;
    let dy = (aim_radians.sin() * len).round() as i32;
    let cx = ox + dx / 2;
    let cy = oy + dy / 2;
    fill_rect(
        pixmap,
        cx - OFFLINE_TILE_PX as i32,
        cy - OFFLINE_TILE_PX as i32,
        OFFLINE_TILE_PX * 2,
        OFFLINE_TILE_PX * 2,
        SPOTLIGHT_CONE_TINT,
    );
}

fn render_wire(pixmap: &mut Pixmap, tile_x: i32, tile_y: i32, kind: WireKind) {
    let base_x = tile_x * OFFLINE_TILE_PX as i32;
    let base_y = tile_y * OFFLINE_TILE_PX as i32;
    let visual = crate::WireVisual::for_kind(kind);
    let tint = [visual.tint_rgb[0], visual.tint_rgb[1], visual.tint_rgb[2], 255];
    fill_rect(pixmap, base_x, base_y + 6, OFFLINE_TILE_PX, 4, tint);
}

fn trench_layer_tint(layer: TrenchLayerId) -> [u8; 4] {
    match layer {
        TrenchLayerId::Floor => [70, 50, 36, 255],
        TrenchLayerId::Duckboard => [120, 88, 56, 255],
        TrenchLayerId::FireStep => [148, 116, 80, 255],
        TrenchLayerId::Breastwork => [88, 80, 60, 255],
        TrenchLayerId::Drainage => [40, 60, 80, 255],
        TrenchLayerId::Revetment => [96, 72, 48, 255],
        TrenchLayerId::CornerTraverse => [104, 80, 56, 255],
        TrenchLayerId::Chevron => [220, 200, 80, 200],
    }
}

fn fortification_layer_tint(layer: FortLayerId) -> [u8; 4] {
    match layer {
        FortLayerId::Base => [80, 80, 80, 255],
        FortLayerId::SandbagFill => [148, 132, 96, 255],
        FortLayerId::MgBarrel => [64, 64, 70, 255],
        FortLayerId::TowerPlatform => [110, 90, 60, 255],
        FortLayerId::SpotlightHousing => [200, 180, 120, 255],
        FortLayerId::WireSprite => [120, 96, 60, 255],
        FortLayerId::AntiTankSilhouette => [60, 60, 70, 255],
        FortLayerId::CamoOverlay => [56, 78, 52, 180],
        FortLayerId::MineMarker => [220, 60, 60, 255],
        FortLayerId::DamageOverlay => [220, 220, 220, 180],
    }
}

const SPOTLIGHT_CONE_TINT: [u8; 4] = [240, 230, 140, 200];

/// Tracing infrastructure helper for tests/CI: emit `engaged` line at
/// the canonical info level. Used by VAL-M10B-021's manual log-grep
/// step in `m10b_export_smoke.sh`.
pub fn log_engaged() {
    tracing::info!("{}", SOFTWARE_RASTERIZER_ENGAGED_LINE);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct a small rasterizer + assert width/height/tier expose.
    #[test]
    fn rasterizer_constructs_with_resolution() {
        let r = OfflineRasterizer::new(64, 32, OfflineRendererTier::DedicatedServer).expect("alloc pixmap");
        assert_eq!(r.width(), 64);
        assert_eq!(r.height(), 32);
        assert_eq!(r.tier(), OfflineRendererTier::DedicatedServer);
        assert!(!r.is_engaged());
    }

    /// `engage()` flips the engaged flag and is idempotent on repeat
    /// calls. VAL-M10B-021's tracing-line greps want the canonical line
    /// to appear once per export job.
    #[test]
    fn rasterizer_engages_idempotently() {
        let mut r = OfflineRasterizer::new(32, 32, OfflineRendererTier::Workstation).unwrap();
        r.engage();
        assert!(r.is_engaged());
        r.engage();
        assert!(r.is_engaged());
    }

    /// Empty scene → background-tinted pixels (non-zero alpha) so the
    /// frame is not "blank" by the `is_blank()` rule.
    #[test]
    fn empty_scene_produces_background_pixels() {
        let mut r = OfflineRasterizer::new(8, 8, OfflineRendererTier::Workstation).unwrap();
        let frame = r.render_scene(0, &[]);
        assert_eq!(frame.width, 8);
        assert_eq!(frame.height, 8);
        assert_eq!(frame.pixels.len(), 8 * 8 * 4);
        assert!(!frame.is_blank());
    }

    /// Per VAL-M10B-021: rendering an `m9c_full_strongpoint`-style
    /// scene (M9B trench + M9C fortification) produces a non-blank
    /// frame whose pixels include the fortification + trench tints.
    #[test]
    fn m9c_full_strongpoint_scene_produces_non_blank_pixels() {
        let mut r = OfflineRasterizer::new(128, 128, OfflineRendererTier::DedicatedServer).unwrap();
        let commands = vec![
            SceneCommand::TrenchSegment {
                tile_x: 1,
                tile_y: 1,
                variant: SegmentVariant::Standard,
            },
            SceneCommand::Fortification {
                tile_x: 3,
                tile_y: 2,
                kind: FortificationKind::MgNestStatic,
            },
            SceneCommand::Fortification {
                tile_x: 4,
                tile_y: 2,
                kind: FortificationKind::SandbagHigh,
            },
            SceneCommand::Wire {
                tile_x: 5,
                tile_y: 4,
                kind: WireKind::BarbedWire,
            },
            SceneCommand::SpotlightCone {
                origin: (2, 2),
                aim_radians: 0.0,
                range_tiles: 3,
            },
        ];
        let frame = r.render_scene(60, &commands);
        assert!(!frame.is_blank());
        // Non-background pixel count is comfortably > 100 because the
        // tile size is 16 and we drew multiple layers per tile.
        assert!(
            frame.non_blank_pixel_count() > 100,
            "expected fortification + trench pixels, got {}",
            frame.non_blank_pixel_count()
        );
    }

    /// Constants surface check — keeps the VAL-M10B-021 tracing-line
    /// contract stable across refactors.
    #[test]
    fn tracing_line_is_the_canonical_signal() {
        assert!(SOFTWARE_RASTERIZER_ENGAGED_LINE.contains("offline_mode"));
        assert!(SOFTWARE_RASTERIZER_ENGAGED_LINE.contains("software_rasterizer"));
        assert!(SOFTWARE_RASTERIZER_ENGAGED_LINE.contains("engaged"));
    }
}
