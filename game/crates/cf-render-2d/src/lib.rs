//! Bevy rendering primitives for the corefall app shell.
//!
//! M0 ships a minimal clear-screen plugin (`CfRenderPlugin`) that inserts a
//! [`ClearColor`] resource and spawns a 2D camera. M1 adds actor sprite rendering
//! through the [`ActorSpritePlugin`]: the engine publishes its actor world into the
//! Bevy ECS via [`ActorRenderState`], and a render system spawns / updates simple
//! colored rectangles for each actor + the M1 floor + a reticle anchored at the
//! player's aim. Everything stays cosmetic-only — `cf-control`'s `M0Engine` is the
//! single source of truth for sim state.
//!
//! **M10B feature split**: the bevy-coupled live renderer above lives behind
//! the default `bevy_render` feature; the `offline` feature replaces the
//! live renderer with [`offline_mode`]'s software rasterizer so the
//! dedicated-server export tier (M36) can produce MP4 frames without a
//! GPU or window. The bevy-free helper modules (`trench_layers`,
//! `fortification_layers`, `spotlight_cone`, `wire_visuals`, `constants`)
//! compile under both features so the offline rasterizer can render M9B
//! trench + M9C fortification pixels transparently.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::field_reassign_with_default,
    clippy::doc_lazy_continuation,
    clippy::manual_range_contains
)]

// Bevy-free helper modules — available under BOTH `bevy_render` and
// `offline` features. M9B / M9C render-layer registries are pure-data
// helpers consumed by either renderer.
pub mod constants;
pub mod fortification_layers;
pub mod spotlight_cone;
pub mod trench_layers;
pub mod wire_visuals;

// M14A: helper modules (bevy-free; pure data transforms for the renderer).
pub mod armor_scratch_overlay;
pub mod limb_render;
pub mod quick_action_render;

// dual-trail termination). Bevy-free helper consumed by the live +
// offline renderers via `IntercepRenderQueue::enqueue`. Honors the
// `cosmetic: true` flag under backpressure per VAL-M14D-018.
pub mod projectile_intercept;

// falling-debris cone). Bevy-free helper consumed by the live + offline
// renderers via `TunnelCollapseQueue::enqueue_cave_in`.
pub mod tunnel_collapse;

// M10B § cf-render-2d::offline_mode — software rasterizer for the
// no-GPU export path. NO wgpu surface / winit::window references per
// VAL-M10B-009. Only compiled with `--features offline`.
#[cfg(feature = "offline")]
pub mod offline_mode;

// Bevy-coupled modules below — gated to the `bevy_render` feature.
#[cfg(feature = "bevy_render")]
pub mod asset_loader;
#[cfg(feature = "bevy_render")]
pub mod debris;
#[cfg(feature = "bevy_render")]
pub mod dig_preview;
#[cfg(feature = "bevy_render")]
pub mod gpu_overlay;
#[cfg(feature = "bevy_render")]
pub mod gpu_particles;
#[cfg(feature = "bevy_render")]
pub mod overlay;
#[cfg(feature = "bevy_render")]
pub mod palette_swap;
#[cfg(feature = "bevy_render")]
pub mod terrain_texture_array;
// M1 re-audit (2026-05-13): spec lists cf-render-2d/src/reticle.rs as a
// separate file. The helper lives there now; the bloom + tool-validity
// color logic still operates inside this lib.rs but consumers can
// `use cf_render_2d::reticle::{reticle_pixel_radius, reticle_color_for_validity}`.
#[cfg(feature = "bevy_render")]
pub mod reticle;
#[cfg(feature = "bevy_render")]
pub mod terrain;

// bullet-impact sparks on hit, explosion VFX on destruction.
#[cfg(feature = "bevy_render")]
pub mod reactor_explosion;
#[cfg(feature = "bevy_render")]
pub mod reactor_sparks;
#[cfg(feature = "bevy_render")]
pub mod reactor_sprite;

// (per DR-055 + DR-046) + Dynamic color grading per scene-mood.
#[cfg(feature = "bevy_render")]
pub mod color_grading;
#[cfg(feature = "bevy_render")]
pub mod juice;

// renderer reads first when a cinematic plays. Bevy-feature-gated; the
// Bevy-free snapshot intermediate ships with both feature sets.
#[cfg(feature = "bevy_render")]
pub mod camera_takeover;

pub use fortification_layers::{
    kind_has_layer, layers_for_kind, sandbag_fill_intact_rows, sandbag_full_row_count, FortLayerId,
};
pub use spotlight_cone::{
    SpotlightCone, SPOTLIGHT_DAZZLE_DURATION_SECONDS, SPOTLIGHT_HALF_ANGLE_DEGREES, SPOTLIGHT_RANGE_TILES,
};
pub use trench_layers::{layers_for_segment, layers_for_variant, segment_has_layer, TrenchLayerId};
pub use wire_visuals::WireVisual;

#[cfg(feature = "bevy_render")]
pub use asset_loader::{
    category_subdir, load_ledger_index, resolve_placeholder_path, AssetIndex, AssetIndexEntry, AssetIndexPlugin,
};
#[cfg(feature = "bevy_render")]
pub use camera_takeover::{
    CinematicCameraPlugin, CinematicCameraTakeover, CinematicTakeoverSnapshot, ColorGradeSnapshot,
};
#[cfg(feature = "bevy_render")]
pub use color_grading::{
    grade_for_mood, ColorGrade, ColorGradingPlugin, ColorGradingState, SceneMood, MONOCHROME_FLOOR,
};
#[cfg(feature = "bevy_render")]
pub use debris::{
    DebrisPlugin, DebrisSpawnQueue, DebrisSpawnRequest, LooseDebris, RenderDebrisCappedEvent, DEBRIS_CAP,
};
#[cfg(feature = "bevy_render")]
pub use dig_preview::{probe_dig_validity, DigPreviewGhost, DigPreviewPlugin, DigPreviewTarget};
#[cfg(feature = "bevy_render")]
pub use juice::{
    camera_shake_amplitude, chromatic_aberration_amplitude, glow_halo_alpha, scale_at, screen_flash_alpha,
    slide_offset, tick_juice, weapon_swap_streak_intensity, JuiceAccessibility, JuiceKind, JuicePlugin, JuicePulse,
    JuiceState,
};
#[cfg(feature = "bevy_render")]
pub use overlay::{
    material_tint, tactical_overlay_chevrons, OverlayMode, OverlayModePlugin, OverlayModeState, TacticalChevronSprite,
};
#[cfg(feature = "bevy_render")]
pub use palette_swap::{
    build_role_swap, parse_hex_rgb, Palette, PaletteEntry, PaletteRegistry, PaletteSwap, OVERLAY_TINT_BUILD_REPAIR,
    OVERLAY_TINT_HAZARD, OVERLAY_TINT_INTEGRITY, OVERLAY_TINT_MOBILITY, OVERLAY_TINT_PATHABILITY,
};
#[cfg(feature = "bevy_render")]
pub use reactor_explosion::{
    ExplosionParticle, ExplosionState, EXPLOSION_DEBRIS_CAP_PER_HIT, EXPLOSION_MAX_DURATION_MS,
};
#[cfg(feature = "bevy_render")]
pub use reactor_sparks::{SparkEmitterState, SparkParticle, SPARK_CAP_PER_HIT};
#[cfg(feature = "bevy_render")]
pub use reactor_sprite::{ReactorSprite, ReactorSpriteState};
#[cfg(feature = "bevy_render")]
pub use terrain::{
    build_chunk_image, material_rgba, ChunkRenderTag, ChunkUpdate, ChunkedTerrainRendererPlugin, ChunkedTerrainSnapshot,
};

// Bevy-coupled top-level live-renderer surface. Everything below this
// boundary depends on bevy + cf-actor + cf-terrain and is gated to the
// `bevy_render` feature. The live renderer is split across sibling
// modules (`live_plugins`, `live_sprite_image`, `live_camera_effects`,
// `live_render_state`, `live_chassis_layout`, `live_actor_systems`)
// and the public items are re-exported here for API compatibility
// with cf-app + downstream consumers.

#[cfg(feature = "bevy_render")]
mod live_actor_systems;
#[cfg(feature = "bevy_render")]
mod live_camera_effects;
#[cfg(feature = "bevy_render")]
mod live_chassis_layout;
#[cfg(feature = "bevy_render")]
mod live_plugins;
#[cfg(feature = "bevy_render")]
mod live_render_state;
#[cfg(feature = "bevy_render")]
mod live_sprite_image;

#[cfg(feature = "bevy_render")]
pub use live_actor_systems::ActorSpritePlugin;
#[cfg(feature = "bevy_render")]
pub use live_camera_effects::{apply_camera_effects, CameraFollow, CameraShake, HitStop};
#[cfg(feature = "bevy_render")]
pub use live_chassis_layout::ACTOR_SILHOUETTE_BASE_SCALE;
#[cfg(feature = "bevy_render")]
pub use live_plugins::{CfRenderPlugin, ChunkedTerrainPlugin, ReactorVfxPlugin, M0_CLEAR_COLOR};
#[cfg(feature = "bevy_render")]
pub use live_render_state::{
    ActorRenderState, ActorRenderTag, BreachRender, BreachRenderTag, ChassisModuleRenderTag, ChassisZoneRenderTag,
    ExtractionRender, ExtractionZoneTag, FloorRenderTag, HeldRifleRenderTag, MuzzleFlashRender, MuzzleFlashTag,
    ReticleRenderTag,
};
#[cfg(feature = "bevy_render")]
pub use live_sprite_image::{solid_sprite, SolidSpriteImage};
