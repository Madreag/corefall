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
    clippy::needless_pass_by_value
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

// **M9** § Reactor visual feedback — sprite swap on pressure_state,
// bullet-impact sparks on hit, explosion VFX on destruction.
#[cfg(feature = "bevy_render")]
pub mod reactor_explosion;
#[cfg(feature = "bevy_render")]
pub mod reactor_sparks;
#[cfg(feature = "bevy_render")]
pub mod reactor_sprite;

// **M12** § Juice rules + dynamic color grading. Per spec § Juice rules
// (per DR-055 + DR-046) + Dynamic color grading per scene-mood.
#[cfg(feature = "bevy_render")]
pub mod color_grading;
#[cfg(feature = "bevy_render")]
pub mod juice;

// **M12C** § Cinematic camera takeover — separate optional resource the
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
    camera_shake_amplitude, chromatic_aberration_amplitude, glow_halo_alpha, scale_at, screen_flash_alpha, slide_offset,
    tick_juice, weapon_swap_streak_intensity, JuiceAccessibility, JuiceKind, JuicePlugin, JuicePulse, JuiceState,
};
#[cfg(feature = "bevy_render")]
pub use camera_takeover::{
    CinematicCameraPlugin, CinematicCameraTakeover, CinematicTakeoverSnapshot, ColorGradeSnapshot,
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

// =============================================================================
// Bevy-coupled top-level live-renderer surface. Everything below this
// boundary depends on bevy + cf-actor + cf-terrain and is therefore
// gated to the `bevy_render` feature. We wrap the entire body in an
// inline `mod live_render` so a single `#[cfg]` controls the whole
// section; the contents are re-exported back to the crate root for
// API compatibility with cf-app + downstream consumers.
// =============================================================================

#[cfg(feature = "bevy_render")]
pub use live_render::*;

#[cfg(feature = "bevy_render")]
#[allow(unused_imports)]
mod live_render {
    use std::collections::HashMap;

    use bevy::asset::RenderAssetUsages;
    use bevy::image::Image;
    use bevy::prelude::*;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    use cf_actor::ActorObservation;

    use crate::asset_loader::{
        category_subdir, load_ledger_index, resolve_placeholder_path, AssetIndex, AssetIndexEntry, AssetIndexPlugin,
    };
    use crate::color_grading::{grade_for_mood, ColorGrade, ColorGradingPlugin, ColorGradingState, SceneMood};
    use crate::debris::{
        DebrisPlugin, DebrisSpawnQueue, DebrisSpawnRequest, LooseDebris, RenderDebrisCappedEvent, DEBRIS_CAP,
    };
    use crate::dig_preview::{probe_dig_validity, DigPreviewGhost, DigPreviewPlugin, DigPreviewTarget};
    use crate::juice::{
        chromatic_aberration_amplitude, glow_halo_alpha, scale_at, screen_flash_alpha, slide_offset, tick_juice,
        weapon_swap_streak_intensity, JuiceAccessibility, JuiceKind, JuicePlugin, JuicePulse, JuiceState,
    };
    use crate::overlay::{
        material_tint, tactical_overlay_chevrons, OverlayMode, OverlayModePlugin, OverlayModeState,
        TacticalChevronSprite,
    };
    use crate::palette_swap::{
        build_role_swap, parse_hex_rgb, Palette, PaletteEntry, PaletteRegistry, PaletteSwap, OVERLAY_TINT_BUILD_REPAIR,
        OVERLAY_TINT_HAZARD, OVERLAY_TINT_INTEGRITY, OVERLAY_TINT_MOBILITY, OVERLAY_TINT_PATHABILITY,
    };
    use crate::reactor_explosion::{
        ExplosionParticle, ExplosionState, EXPLOSION_DEBRIS_CAP_PER_HIT, EXPLOSION_MAX_DURATION_MS,
    };
    use crate::reactor_sparks::{SparkEmitterState, SparkParticle, SPARK_CAP_PER_HIT};
    use crate::reactor_sprite::{ReactorSprite, ReactorSpriteState};
    use crate::reticle::{reticle_color_for_validity, reticle_pixel_radius};
    use crate::terrain::{
        build_chunk_image, material_rgba, ChunkRenderTag, ChunkUpdate, ChunkedTerrainRendererPlugin,
        ChunkedTerrainSnapshot,
    };

    /// Pixel-art-friendly cleared background. Hex `#0d121a` (deep slate).
    pub const M0_CLEAR_COLOR: Color = Color::srgb(0.051, 0.071, 0.102);

    pub struct CfRenderPlugin {
        pub clear_color: Color,
    }

    impl Default for CfRenderPlugin {
        fn default() -> Self {
            Self {
                clear_color: M0_CLEAR_COLOR,
            }
        }
    }

    impl Plugin for CfRenderPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(ClearColor(self.clear_color))
                .insert_resource(CameraShake::default())
                .insert_resource(CameraFollow::default())
                .insert_resource(HitStop::default())
                .add_systems(Startup, spawn_camera);
        }
    }

    /// **M2**: composite plugin wiring the chunked-terrain renderer, the 5-mode
    /// material overlay, the loose-pixel debris system, and the tool-validity
    /// ghost preview. cf-app adds this alongside [`CfRenderPlugin`] +
    /// [`ActorSpritePlugin`].
    pub struct ChunkedTerrainPlugin;

    impl Plugin for ChunkedTerrainPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins((
                ChunkedTerrainRendererPlugin,
                OverlayModePlugin,
                DebrisPlugin,
                DigPreviewPlugin,
            ));
        }
    }

    /// **M9** § Reactor visual feedback — wires the reactor sprite +
    /// bullet-impact spark emitter + destruction explosion VFX resources.
    /// cf-app spawns sparks on `combat.projectile_hit` (target_kind="reactor")
    /// and the explosion burst on `mission.reactor_destroyed`; this plugin's
    /// `tick_reactor_vfx` system advances + retires particles per frame so
    /// the live VFX terminates within 1s of the triggering event.
    pub struct ReactorVfxPlugin;

    impl Plugin for ReactorVfxPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<ReactorSpriteState>()
                .init_resource::<SparkEmitterState>()
                .init_resource::<ExplosionState>()
                .add_systems(Update, tick_reactor_vfx);
        }
    }

    fn tick_reactor_vfx(time: Res<Time>, mut sparks: ResMut<SparkEmitterState>, mut explosion: ResMut<ExplosionState>) {
        let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
        if dt_ms == 0 {
            return;
        }
        sparks.tick(dt_ms);
        explosion.tick(dt_ms);
    }

    /// Bevy 0.18 silently drops sprites whose `Handle<Image>` does not resolve to a
    /// loaded `GpuImage` — `Sprite::from_color` returns a defaulted handle that the
    /// `bevy_sprite_render::queue_sprites` pass `continue`s on, so a "color-only"
    /// sprite never makes it to the draw queue. To get solid-color rectangles back
    /// we register a 1x1 fully-white RGBA image at startup and route every cosmetic
    /// sprite through `solid_sprite()` below, keeping `sprite.color` as the tint.
    #[derive(Resource, Clone, Default)]
    pub struct SolidSpriteImage {
        pub handle: Handle<Image>,
    }

    fn build_solid_sprite_image(mut images: ResMut<Assets<Image>>, mut handle: ResMut<SolidSpriteImage>) {
        let image = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        handle.handle = images.add(image);
    }

    /// Helper: build a Bevy 0.18 `Sprite` that actually renders as a solid color.
    /// Wraps the `SolidSpriteImage` 1x1 white texture so `sprite.color` shows up
    /// correctly without dragging an asset path through every callsite.
    pub fn solid_sprite(image: &SolidSpriteImage, color: Color, size: Vec2) -> Sprite {
        Sprite {
            image: image.handle.clone(),
            color,
            custom_size: Some(size),
            ..default()
        }
    }

    fn spawn_camera(mut commands: Commands) {
        commands.spawn((Camera2d, Name::new("cf::render::main_camera")));
    }

    // ---------------------------------------------------------------------------
    // M1 Gap E1-E3: camera shake / hit-stop / follow-with-deadzone resources
    // ---------------------------------------------------------------------------

    /// **M1 Gap E1**: camera shake state. cf-app writes `pending_magnitude` (in
    /// pixels) when an `ux.camera_punch_requested` event fires; the
    /// `apply_camera_effects` system decays the magnitude by ~exp(-dt/0.2s) and
    /// applies a per-frame random offset to the Camera2d. Setting
    /// `reduce_camera_shake_pct=1.0` zeroes the magnitude on intake so the
    /// camera never moves (accessibility floor).
    #[derive(Resource, Debug, Clone, Default)]
    pub struct CameraShake {
        pub magnitude_px: f32,
        pub reduce_pct: f32,
        /// 64-bit xorshift state for the per-frame jitter. Seeded by cf-app from
        /// the engine RNG at startup so the visual is deterministic given the
        /// same input event stream.
        pub rng_state: u64,
    }

    impl CameraShake {
        fn next_jitter(&mut self) -> (f32, f32) {
            // xorshift64* — deterministic and cheap.
            let mut s = if self.rng_state == 0 {
                0x9E3779B97F4A7C15
            } else {
                self.rng_state
            };
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            self.rng_state = s;
            let x = (s as f32) / (u32::MAX as f32 * 2.0) - 0.5;
            let y = ((s >> 32) as f32) / (u32::MAX as f32 * 2.0) - 0.5;
            (x, y)
        }
    }

    /// **M1 Gap E2**: camera-follow-with-deadzone target + tuning. cf-app updates
    /// `target` each frame from the player actor position. The render system
    /// lerps the camera position toward the target whenever the target leaves
    /// the deadzone rectangle.
    #[derive(Resource, Debug, Clone)]
    pub struct CameraFollow {
        pub target: Option<Vec2>,
        pub deadzone_half_width_px: f32,
        pub deadzone_half_height_px: f32,
        /// Per-frame lerp factor (0..1) applied when target is outside the
        /// deadzone. 0.18 = ~5-frame catch-up at 60Hz; tweak in cvars when E2
        /// promoted to a settings cvar.
        pub lerp_factor: f32,
    }

    impl Default for CameraFollow {
        fn default() -> Self {
            Self {
                target: None,
                deadzone_half_width_px: 40.0,
                deadzone_half_height_px: 30.0,
                lerp_factor: 0.18,
            }
        }
    }

    /// **M1 Gap E3**: hit-stop state. cf-app writes `remaining_ms` when an
    /// `ux.hit_stop_requested` event fires. The `apply_camera_effects` system
    /// uses Bevy's `Time::set_relative_speed` to slow the world during the
    /// freeze window. M1 ships even a single-tick pause as proof of life.
    #[derive(Resource, Debug, Clone, Default)]
    pub struct HitStop {
        pub remaining_ms: f32,
    }

    /// Drive camera shake + camera follow + hit-stop per frame. cf-app populates
    /// the input resources; the render system applies effects to the Camera2d
    /// transform and the global Bevy `Time` resource.
    pub fn apply_camera_effects(
        time: Res<Time>,
        mut camera_query: Query<&mut Transform, With<Camera2d>>,
        mut shake: ResMut<CameraShake>,
        follow: Res<CameraFollow>,
        mut hit_stop: ResMut<HitStop>,
        mut time_speed: ResMut<Time<Virtual>>,
    ) {
        if let Some(mut camera_transform) = camera_query.iter_mut().next() {
            // Camera follow: lerp toward target when outside the deadzone.
            if let Some(target) = follow.target {
                let cam = camera_transform.translation.truncate();
                let dx = target.x - cam.x;
                let dy = target.y - cam.y;
                let target_x = if dx.abs() > follow.deadzone_half_width_px {
                    cam.x + dx * follow.lerp_factor.clamp(0.0, 1.0)
                } else {
                    cam.x
                };
                let target_y = if dy.abs() > follow.deadzone_half_height_px {
                    cam.y + dy * follow.lerp_factor.clamp(0.0, 1.0)
                } else {
                    cam.y
                };
                camera_transform.translation.x = target_x;
                camera_transform.translation.y = target_y;
            }
            // Camera shake: decay magnitude then apply random offset.
            let dt = time.delta_secs();
            // Tau ~0.2s exponential decay (shake ends within ~200ms).
            let decay = (-dt / 0.2).exp();
            shake.magnitude_px *= decay;
            if shake.magnitude_px > 0.05 {
                let (jx, jy) = shake.next_jitter();
                let scale = (1.0 - shake.reduce_pct.clamp(0.0, 1.0)).max(0.0);
                camera_transform.translation.x += jx * shake.magnitude_px * scale;
                camera_transform.translation.y += jy * shake.magnitude_px * scale;
            } else {
                shake.magnitude_px = 0.0;
            }
        }
        // Hit-stop: freeze the virtual clock for the requested window.
        if hit_stop.remaining_ms > 0.0 {
            time_speed.set_relative_speed(0.05);
            hit_stop.remaining_ms = (hit_stop.remaining_ms - time.delta_secs() * 1000.0).max(0.0);
        } else if time_speed.relative_speed() < 0.99 {
            time_speed.set_relative_speed(1.0);
        }
    }

    // ---------------------------------------------------------------------------
    // M1: actor sprite rendering
    // ---------------------------------------------------------------------------

    /// Snapshot of the engine's actor world, written each frame by the `cf-app` bridge
    /// system. The render layer reads this and updates Bevy entities without ever owning
    /// authoritative state.
    #[derive(Resource, Debug, Clone, Default)]
    pub struct ActorRenderState {
        pub actors: Vec<ActorObservation>,
        pub player_actor_id: Option<u64>,
        pub region_width: f32,
        pub region_height: f32,
        /// Bottom-left anchor of the play region in world space. Mirrors
        /// `M0EngineConfig::region_anchor_{x,y}` so the render layer can centre the
        /// floor + camera over the actual region for scenarios that don't anchor at
        /// the world origin.
        pub region_anchor_x: f32,
        pub region_anchor_y: f32,
        pub floor_y: f32,
        /// M1.5 breach strips (id + bbox + hp). Empty for non-breach scenarios.
        pub breaches: Vec<BreachRender>,
        /// M1.5 extraction zone if the scenario carries a `ReachZone` objective.
        pub extraction_zone: Option<ExtractionRender>,
        /// **M5**: current sim tick. Drives the walk-cycle phase for chassis-
        /// attached actors (legs alternate at ~8-tick cadence during Walking /
        /// Running stances). Without this, the silhouette stands still even
        /// while the position moves — which would re-introduce the
        /// "static sliding pawn" M5-DC-3 failure mode the chassis pips exist
        /// to close.
        pub tick: u64,
        /// **M1 Gap E4**: tool-validity flag mirrored from
        /// `ObserveFrame::tool_validity::valid`. Drives the reticle color (red
        /// when false). `None` = no tool-validity tracking active (default).
        pub tool_valid: Option<bool>,
        /// **M1 Gap J2**: most recent muzzle-flash payload (origin + decay).
        /// cf-app writes this when `equipment.weapon_fired` fires; cleared each
        /// frame after rendering.
        pub muzzle_flash: Option<MuzzleFlashRender>,
    }

    /// **M1 Gap J2**: muzzle-flash projection.
    #[derive(Debug, Clone)]
    pub struct MuzzleFlashRender {
        pub origin: Vec2,
        pub remaining_ticks: u32,
    }

    /// M1.5 render-side projection of a breach strip.
    #[derive(Debug, Clone)]
    pub struct BreachRender {
        pub id: String,
        pub bbox_min: [f32; 2],
        pub bbox_max: [f32; 2],
        pub hp: f32,
        pub max_hp: f32,
        pub broken: bool,
        pub refusal_reason: Option<String>,
    }

    /// M1.5 render-side projection of the extraction zone.
    #[derive(Debug, Clone)]
    pub struct ExtractionRender {
        pub min: [f32; 2],
        pub max: [f32; 2],
        pub completed: bool,
    }

    /// Spawned per actor; carries the actor id so the render system can update or
    /// despawn the right entity when the world changes.
    #[derive(Component, Debug, Clone, Copy)]
    pub struct ActorRenderTag {
        pub id: u64,
    }

    /// Marker for the floor sprite (M1 stand-in for chunked terrain).
    #[derive(Component, Debug)]
    pub struct FloorRenderTag;

    /// Marker for the aim reticle sprite that follows the player's aim direction.
    #[derive(Component, Debug)]
    pub struct ReticleRenderTag;

    /// Marker for the M1.5 extraction zone sprite (the green goal box).
    #[derive(Component, Debug)]
    pub struct ExtractionZoneTag;

    /// Marker for one M1.5 breach strip rendered as a colored block.
    #[derive(Component, Debug, Clone)]
    pub struct BreachRenderTag {
        pub id: String,
    }

    /// **M5**: per-chassis-zone overlay pip. cf-render-2d spawns 15 of these per
    /// chassis-attached actor, one per body zone (Head / Torso / ArmLeft / ArmRight
    /// / ForearmLeft / ForearmRight / HandLeft / HandRight / LegLeft / LegRight /
    /// ShinLeft / ShinRight / FootLeft / FootRight / Backpack). Each pip's color
    /// reflects the zone's external_integrity + destroyed flag so a player watching
    /// a wreck_eject scenario can SEE the right forearm pip turn black when the
    /// chassis zone gets destroyed — the visible-limb proof for M5-DC-3 that
    /// closes the "static sliding pawn" gap from the audit. The pip's position
    /// follows the actor's transform + stance offset.
    #[derive(Component, Debug, Clone)]
    pub struct ChassisZoneRenderTag {
        pub actor_id: u64,
        pub zone: String,
    }

    /// **M5** per-chassis-module overlay pip. Spawns 1 per `ChassisView.modules`
    /// entry per chassis-attached actor (jet, shield, sensor, weapon_mount,
    /// repair_drone). Color reflects module state (Nominal → green, Degraded
    /// → yellow, Warning → orange, Failed → red, NotPresent → not rendered).
    /// Position follows the bound zone + kind-specific offset. Surfaces sim
    /// depth Cortex doesn't have: every chassis carries 5 damageable modules
    /// whose health drives gameplay (jet failure grounds the actor, sensor
    /// failure blanks the HUD radar, weapon-mount failure jams the rifle).
    #[derive(Component, Debug, Clone)]
    pub struct ChassisModuleRenderTag {
        pub actor_id: u64,
        pub module_id: String,
    }

    /// **M5** held rifle sprite. Spawns 1 per actor whose `actor.selected_item`
    /// resolves to a rifle. Position follows the right-hand zone (or torso for
    /// chassis-less actors) + aim vector; rotation matches aim direction so the
    /// rifle visibly points where the player aims. Without this, the actor has
    /// NO visible weapon despite firing (only projectiles + muzzle flash hint
    /// at the rifle's existence).
    #[derive(Component, Debug, Clone)]
    pub struct HeldRifleRenderTag {
        pub actor_id: u64,
    }

    /// **M5** canonical body-zone layout in actor-local coordinates. Each entry is
    /// `(zone_name, dx, dy, width, height)` describing a small rect anchored at the
    /// actor's center. The layout assembles a 14-pip humanoid silhouette inside the
    /// actor's ~16×32 sprite bounds (PoweredArmor scale); LightMech scales 2.25× via
    /// the chassis kind multiplier in `chassis_scale_multiplier`. The Backpack pip
    /// sits behind the torso at slightly negative z so it renders underneath.
    const CHASSIS_ZONE_LAYOUT: &[(&str, f32, f32, f32, f32)] = &[
        // (zone, dx, dy, width, height)
        ("head", 0.0, 12.0, 6.0, 4.0),
        ("torso", 0.0, 4.0, 8.0, 10.0),
        ("arm_left", -6.0, 6.0, 3.0, 5.0),
        ("arm_right", 6.0, 6.0, 3.0, 5.0),
        ("forearm_left", -7.0, 1.0, 3.0, 4.0),
        ("forearm_right", 7.0, 1.0, 3.0, 4.0),
        ("hand_left", -7.0, -3.0, 3.0, 3.0),
        ("hand_right", 7.0, -3.0, 3.0, 3.0),
        ("leg_left", -2.0, -3.0, 3.0, 5.0),
        ("leg_right", 2.0, -3.0, 3.0, 5.0),
        ("shin_left", -2.0, -8.0, 3.0, 4.0),
        ("shin_right", 2.0, -8.0, 3.0, 4.0),
        ("foot_left", -2.0, -12.0, 4.0, 3.0),
        ("foot_right", 2.0, -12.0, 4.0, 3.0),
        ("backpack", 0.0, 5.0, 6.0, 8.0),
    ];

    /// **M5** scale multiplier per chassis kind. Powered Armor = 1.0 (baseline,
    /// scaled by `ACTOR_SILHOUETTE_BASE_SCALE` so the silhouette is visible at
    /// battlefield zoom); LightMech = 2.25× (matches the sim's `attach_chassis`
    /// half_extents: LightMech 18×36 vs PoweredArmor 10×20). Infantry default
    /// = 0.9× (slightly smaller than PoweredArmor).
    fn chassis_scale_multiplier(kind: &str) -> f32 {
        match kind {
            "light_mech" => 2.25,
            "powered_armor" => 1.0,
            _ => 0.9,
        }
    }

    /// **M5** base actor silhouette scale multiplier applied to every chassis
    /// pip (and to chassis-less actors via `infantry_default_silhouette`). The
    /// CHASSIS_ZONE_LAYOUT geometry is authored at ~16×28 px bounds (PoweredArmor
    /// baseline); scaling by 2.0 produces a ~32×56 px on-screen silhouette that
    /// reads clearly at 1280×720 capture resolution. Without this multiplier the
    /// limbs are sub-pixel-thin and visual review tools (humans + AI agents
    /// reading capture frames) can't verify destroyed-limb / stance / module
    /// state at-a-glance — defeating the M5-DC-3 / M5-DC-4 visual closure goal.
    pub const ACTOR_SILHOUETTE_BASE_SCALE: f32 = 2.0;

    /// **M5** stance offset applied to the entire chassis silhouette. Moves the
    /// 15 pips together so the player can SEE crouch/climb/jet/eject states.
    fn stance_offset(stance: &str) -> (f32, f32, f32) {
        match stance {
            "crouching" => (0.0, -4.0, 0.65),
            "climbing" => (3.0, 2.0, 1.0),
            "jetting" => (0.0, 6.0, 1.0),
            "ejecting" => (0.0, 8.0, 0.9),
            "downed" => (0.0, -8.0, 1.0),
            "dead" => (0.0, -10.0, 1.0),
            _ => (0.0, 0.0, 1.0),
        }
    }

    /// **M5** stage tint applied to every zone pip. Stage transitions become
    /// instantly visible: Nominal=untinted; Degraded=slight yellow; Disabled=orange;
    /// Wreck=red; Gibbed=very-dark-red.
    fn chassis_stage_tint(stage: &str) -> Color {
        match stage {
            "nominal" => Color::srgb(1.0, 1.0, 1.0),
            "degraded" | "module_warning" | "module_failed" | "weapon_jammed" => Color::srgb(1.0, 0.95, 0.55),
            "armor_cracked" | "disabled" | "pilot_injured" => Color::srgb(1.0, 0.65, 0.30),
            "eject" => Color::srgb(0.95, 0.40, 0.20),
            "bail_too_late" | "wreck" => Color::srgb(0.85, 0.20, 0.20),
            "gibbed" => Color::srgb(0.50, 0.10, 0.10),
            _ => Color::srgb(1.0, 1.0, 1.0),
        }
    }

    /// **M5** per-zone anatomical tint. Pixel-sim battlefield per DR-019 wants the
    /// silhouette to read as a humanoid at-a-glance — head/helmet darker, chest
    /// armor plate brighter (Cortex's ChestPlateA overlay pattern), arms/legs at
    /// muted shade so they recede behind the torso, hands/feet at the brightest
    /// tip of the limb so silhouette reads as a clear armored figure, backpack
    /// distinct so jetpack is visible. Returns a per-zone (r_mul, g_mul, b_mul)
    /// triple applied on top of the team base color.
    fn zone_anatomical_tint(zone: &str) -> (f32, f32, f32) {
        match zone {
            // Head + helmet: darker shade so the silhouette has a recognizable
            // helmet contrast against the brighter torso.
            "head" => (0.55, 0.55, 0.65),
            // Torso = primary chest armor; brightest of the limbs.
            "torso" => (1.05, 1.05, 1.10),
            // Upper arms: thinner muted shade so they recede behind torso.
            "arm_left" | "arm_right" => (0.80, 0.80, 0.85),
            // Forearms: slightly brighter than upper arm so the elbow joint reads.
            "forearm_left" | "forearm_right" => (0.85, 0.85, 0.90),
            // Hands: bright tip; carries weapon visibility for the rifle-hand side.
            "hand_left" | "hand_right" => (0.95, 0.90, 0.75),
            // Thighs: same shade as upper arms.
            "leg_left" | "leg_right" => (0.80, 0.80, 0.85),
            // Shins: slightly brighter than thigh.
            "shin_left" | "shin_right" => (0.85, 0.85, 0.90),
            // Boots: dark contrast so footing is readable at battlefield zoom.
            "foot_left" | "foot_right" => (0.45, 0.45, 0.50),
            // Backpack/jetpack: distinct cool shade so the silhouette has a
            // visible pack behind the torso when the actor faces the camera.
            "backpack" => (0.40, 0.55, 0.75),
            _ => (1.0, 1.0, 1.0),
        }
    }

    /// **M5** per-zone color: encodes the 3-layer armor state (External →
    /// Internal → Core → wound) through color. Corefall surfaces simulation
    /// depth Cortex/Soldat/Noita don't have: every zone has 3 stacked armor
    /// layers + a wound container, and the silhouette must show which deepest
    /// layer is still intact at a glance.
    ///
    /// Color progression as armor degrades:
    ///   External fully intact (>=0.66 hp)   → bright team color + anatomical tint
    ///   External breached, Internal intact  → mid-shade (60% brightness, cooler)
    ///   Internal breached, Core intact      → dark + warning tint
    ///   Core breached, wound bleeding       → red wound color
    ///   Wound drained → zone destroyed      → transparent (visible limb-loss gap)
    ///
    /// Stage tint multiplies on top so a Disabled chassis renders orange-tinted
    /// even if individual zones are healthy.
    fn zone_color(base: Color, zone: &ActorChassisZoneView, stage_tint: Color) -> Color {
        if zone.destroyed {
            // Destroyed zone: transparent void where the limb used to be.
            // Matches M5-DC-4 ("limb damage has visible/mechanical consequences:
            // limp, crawl, one-arm handling, dropped gear, disabled grip").
            return Color::srgba(0.0, 0.0, 0.0, 0.0);
        }
        let (br, bg, bb, _) = base.to_srgba().to_f32_array().into();
        let (tr, tg, tb, _) = stage_tint.to_srgba().to_f32_array().into();
        let (ar, ag, ab) = zone_anatomical_tint(zone.zone.as_str());

        // Find the deepest-intact layer; its hp% drives the visible brightness.
        // Below 0.05 we treat the layer as breached and progress to the next.
        let (layer_bright, layer_warm_shift) = if zone.external_integrity > 0.05 {
            // External layer intact: full brightness, no warm shift.
            (zone.external_integrity.clamp(0.0, 1.0).max(0.40), 0.0)
        } else if zone.internal_integrity > 0.05 {
            // External breached, Internal intact: ~60% brightness + slight warm.
            (zone.internal_integrity.clamp(0.0, 1.0).max(0.30) * 0.65, 0.15)
        } else if zone.core_integrity > 0.05 {
            // Internal breached, Core intact: ~40% brightness + strong warm.
            (zone.core_integrity.clamp(0.0, 1.0).max(0.25) * 0.45, 0.35)
        } else {
            // Core breached, wound bleeding: red wound color regardless of base.
            let wound = zone.wound_integrity.clamp(0.0, 1.0);
            return Color::srgb(0.85 * wound.max(0.30), 0.10, 0.10);
        };

        // Apply warm shift: bias red up, blue down as armor degrades.
        let warm_r = 1.0 + layer_warm_shift;
        let warm_b = 1.0 - layer_warm_shift * 0.8;
        Color::srgb(
            (br * tr * ar * layer_bright * warm_r).clamp(0.0, 1.0),
            (bg * tg * ag * layer_bright).clamp(0.0, 1.0),
            (bb * tb * ab * layer_bright * warm_b).clamp(0.0, 1.0),
        )
    }

    /// **M5** module pip color by module state. Module pips render as small
    /// overlay sprites on the chassis at their bound zone, so a player watching
    /// a wreck_eject scenario can SEE the jet module turn red when the backpack
    /// zone takes damage. This is a Corefall-only feature — Cortex actors don't
    /// surface module state at all.
    fn module_pip_color(state: &str) -> Option<Color> {
        match state {
            "nominal" => Some(Color::srgb(0.30, 0.95, 0.40)),
            "degraded" => Some(Color::srgb(0.95, 0.85, 0.30)),
            "warning" => Some(Color::srgb(0.95, 0.60, 0.20)),
            "failed" => Some(Color::srgb(0.85, 0.20, 0.20)),
            _ => None, // NotPresent or unknown — don't render
        }
    }

    /// **M5** module → bound-zone offset. Returns the (dx, dy) offset relative to
    /// the zone's center where the module pip should render. Multiple modules
    /// can be bound to the same zone (e.g., shield + sensor on torso); the
    /// caller cycles through positions.
    fn module_pip_offset(kind: &str) -> (f32, f32, f32) {
        // (dx, dy, size). Position relative to the bound zone's center.
        match kind {
            "jet" => (0.0, 2.0, 3.0),           // jet pip slightly above backpack center
            "shield" => (0.0, 0.0, 2.0),        // shield emitter in front of torso
            "sensor" => (0.0, 2.0, 2.0),        // sensor antenna above head
            "weapon_mount" => (2.0, 0.0, 2.5),  // weapon mount on hand side
            "repair_drone" => (0.0, -2.0, 2.5), // repair drone below backpack
            _ => (0.0, 0.0, 2.0),
        }
    }

    /// **M5** walk-cycle phase for leg/shin/foot pips. Driven by tick number +
    /// horizontal velocity sign so the legs visibly alternate during locomotion.
    /// Returns (left_leg_dy, right_leg_dy) — opposite phase between legs.
    fn walk_cycle_offsets(tick: u64, stance: &str, velocity_x: f32) -> (f32, f32) {
        // Only animate during locomotion stances; static stances hold pose.
        let moving = matches!(stance, "walking" | "running") && velocity_x.abs() > 1.0;
        if !moving {
            return (0.0, 0.0);
        }
        // ~8-tick walk cycle (4 ticks per step at 60Hz = ~133ms cadence; ~6Hz
        // step rate, matches infantry walk feel without looking jittery).
        let phase = (tick % 8) as f32;
        let amplitude = if stance == "running" { 2.5 } else { 1.5 };
        let cycle = ((phase / 8.0) * std::f32::consts::TAU).sin();
        (cycle * amplitude, -cycle * amplitude)
    }

    /// **M5** local proxy type so `zone_color` can take the chassis zone view
    /// without depending on cf-actor's exact struct. Matches the field set we
    /// read from `ActorObservation.chassis.zones[]`.
    pub(crate) struct ActorChassisZoneView {
        pub zone: String,
        pub external_integrity: f32,
        pub internal_integrity: f32,
        pub core_integrity: f32,
        pub wound_integrity: f32,
        pub destroyed: bool,
    }

    /// Plugin that wires actor / floor / reticle rendering. Call after [`CfRenderPlugin`].
    pub struct ActorSpritePlugin;

    impl Plugin for ActorSpritePlugin {
        fn build(&self, app: &mut App) {
            // M1 Gap E: defensively init the camera-effect resources so unit
            // tests that load ActorSpritePlugin without the parent CfRenderPlugin
            // still have working defaults.
            app.init_resource::<ActorRenderState>()
                .init_resource::<SolidSpriteImage>()
                .init_resource::<CameraShake>()
                .init_resource::<CameraFollow>()
                .init_resource::<HitStop>()
                .add_systems(Startup, (build_solid_sprite_image, spawn_floor_and_reticle).chain())
                .add_systems(
                    Update,
                    (
                        sync_actor_sprites,
                        sync_chassis_zone_sprites,
                        sync_chassis_module_sprites,
                        sync_held_rifle_sprites,
                        sync_breach_sprites,
                        sync_extraction_zone,
                        // Gap E1-E3: camera punch / hit-stop / follow runs AFTER the
                        // chain so the player position is already up to date and the
                        // camera lerp catches the same frame.
                        apply_camera_effects,
                        update_reticle_color,
                        update_muzzle_flash,
                    )
                        .chain(),
                );
        }
    }

    /// **M1 Gap E4**: tint the reticle red when `ActorRenderState::tool_valid ==
    /// Some(false)`, otherwise restore the canonical white tint. Friendly-fire
    /// color hook lands at M1.5 when teams ship.
    fn update_reticle_color(state: Res<ActorRenderState>, mut q: Query<&mut Sprite, With<ReticleRenderTag>>) {
        let color = match state.tool_valid {
            Some(false) => Color::srgb(1.0, 0.25, 0.25),
            _ => Color::srgb(1.0, 1.0, 1.0),
        };
        for mut sprite in &mut q {
            if sprite.color != color {
                sprite.color = color;
            }
        }
    }

    /// **M1 Gap J2**: render the muzzle-flash sprite for a couple of ticks after
    /// every `equipment.weapon_fired`. cf-app populates
    /// `ActorRenderState::muzzle_flash`; this system spawns a transient sprite
    /// at the muzzle origin and decays it.
    fn update_muzzle_flash(
        mut commands: Commands,
        mut state: ResMut<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        existing: Query<Entity, With<MuzzleFlashTag>>,
    ) {
        // Despawn any existing flash entity each frame; we re-spawn fresh
        // whenever `muzzle_flash` is `Some`. Cheap because flashes live <= 3
        // ticks at 60 Hz.
        for e in existing.iter() {
            commands.entity(e).despawn();
        }
        if let Some(flash) = state.muzzle_flash.take() {
            let alpha = (flash.remaining_ticks as f32 / 3.0).clamp(0.0, 1.0);
            commands.spawn((
                solid_sprite(&solid, Color::srgba(1.0, 0.9, 0.4, alpha), Vec2::new(10.0, 6.0)),
                Transform::from_translation(Vec3::new(flash.origin.x, flash.origin.y, 1.5)),
                MuzzleFlashTag,
                Name::new("cf::render::muzzle_flash"),
            ));
        }
    }

    #[derive(Component, Debug)]
    pub struct MuzzleFlashTag;

    /// **M1 Gap J1**: per-stance silhouette tint for chassis-less actors. The
    /// pixel-art sprite frames under `content/sprites/actor_m1/` are reserved
    /// for the asset loader at BP4+; M1 ships the visible-silhouette stance
    /// swap so the player no longer renders as a transparent ghost rectangle.
    fn stance_tint_for(stance: &str, status: &str) -> Color {
        if status == "dead" {
            return Color::srgb(0.05, 0.05, 0.05);
        }
        if status == "dying" {
            return Color::srgb(0.35, 0.05, 0.05);
        }
        if status == "downed" {
            return Color::srgb(0.30, 0.10, 0.10);
        }
        if status == "inactive" {
            return Color::srgb(0.35, 0.35, 0.40);
        }
        match stance {
            "idle" => Color::srgb(0.55, 0.58, 0.65),
            "walking" => Color::srgb(0.50, 0.65, 0.78),
            "running" => Color::srgb(0.35, 0.65, 0.85),
            "airborne" => Color::srgb(0.80, 0.75, 0.30),
            "knocked_down" => Color::srgb(0.80, 0.25, 0.25),
            "crouching" => Color::srgb(0.45, 0.55, 0.50),
            "climbing" => Color::srgb(0.55, 0.45, 0.65),
            "jetting" => Color::srgb(0.90, 0.55, 0.20),
            "ejecting" => Color::srgb(0.95, 0.90, 0.25),
            _ => Color::srgb(0.50, 0.55, 0.60),
        }
    }

    fn spawn_floor_and_reticle(mut commands: Commands, solid: Res<SolidSpriteImage>) {
        // Floor (placeholder; real chunked terrain lands at M2).
        commands.spawn((
            solid_sprite(&solid, Color::srgb(0.15, 0.18, 0.20), Vec2::new(2048.0, 8.0)),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.5)),
            FloorRenderTag,
            Name::new("cf::render::floor"),
        ));
        commands.spawn((
            solid_sprite(&solid, Color::srgb(0.95, 0.65, 0.30), Vec2::new(4.0, 4.0)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            Visibility::Hidden,
            ReticleRenderTag,
            Name::new("cf::render::reticle"),
        ));
    }

    #[allow(clippy::too_many_arguments)]
    fn sync_actor_sprites(
        mut commands: Commands,
        mut state: ResMut<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut actor_query: Query<(Entity, &ActorRenderTag, &mut Transform, &mut Sprite)>,
        mut floor_query: Query<
            (&mut Transform, &mut Sprite),
            (With<FloorRenderTag>, Without<ActorRenderTag>, Without<ReticleRenderTag>),
        >,
        mut reticle_query: Query<
            (&mut Transform, &mut Visibility),
            (With<ReticleRenderTag>, Without<ActorRenderTag>, Without<FloorRenderTag>),
        >,
        mut camera_query: Query<
            &mut Transform,
            (
                With<Camera2d>,
                Without<ActorRenderTag>,
                Without<FloorRenderTag>,
                Without<ReticleRenderTag>,
            ),
        >,
    ) {
        // Place the floor centred under the play region. The region's bottom-left
        // anchor may be non-zero, so derive the world-space centre from
        // `region_anchor + region_size * 0.5` instead of assuming `(0, 0)`.
        if state.region_width > 0.0 {
            let region_center_x = state.region_anchor_x + state.region_width * 0.5;
            let region_center_y = state.region_anchor_y + state.region_height * 0.5;
            if let Some((mut transform, mut sprite)) = floor_query.iter_mut().next() {
                transform.translation = Vec3::new(region_center_x, state.floor_y - 4.0, -0.5);
                sprite.custom_size = Some(Vec2::new(state.region_width, 8.0));
            }
            // M1 Gap E2: the CameraFollow system owns camera positioning during
            // gameplay. We only seed the camera once at startup (when the camera
            // is still at the world origin) so the very first frame doesn't show
            // a half-empty viewport. Any subsequent frame, CameraFollow lerps.
            if let Some(mut camera_transform) = camera_query.iter_mut().next() {
                if camera_transform.translation.x.abs() < 0.5 && camera_transform.translation.y.abs() < 0.5 {
                    camera_transform.translation.x = region_center_x;
                    camera_transform.translation.y = region_center_y;
                }
            }
        }

        let mut existing: HashMap<u64, Entity> = HashMap::new();
        for (entity, tag, _, _) in actor_query.iter() {
            existing.insert(tag.id, entity);
        }

        let mut player_position: Option<Vec2> = None;
        let mut player_aim: Option<Vec2> = None;

        let mut keep: HashMap<u64, ()> = HashMap::new();
        for actor in &state.actors {
            keep.insert(actor.id, ());
            let pos = Vec2::new(actor.position[0], actor.position[1]);
            // **M5**: every actor renders via the 15 per-zone pips (see
            // `sync_chassis_zone_sprites`). Chassis-attached actors use real
            // per-zone hp; chassis-less actors use a synthetic intact body
            // derived from HP.
            //
            // **M1 Gap J1**: chassis-less actors (M1's player) get a Stance-
            // tinted silhouette so the actor renders as a visible body, not a
            // ghost rectangle behind the chassis pips. The tint varies by
            // stance (Idle / Walking / Running / Airborne / KnockedDown /
            // Downed / Dead) so the visual identity tracks the sim. Actors
            // WITH a chassis keep the transparent parent so the per-zone pips
            // remain authoritative (M5 chassis grammar owns the silhouette).
            let parent_color = if actor.chassis.is_some() {
                Color::srgba(0.0, 0.0, 0.0, 0.0)
            } else {
                stance_tint_for(&actor.stance, &actor.status)
            };
            if let Some(entity) = existing.get(&actor.id) {
                if let Ok((_, _, mut transform, mut sprite)) = actor_query.get_mut(*entity) {
                    transform.translation = Vec3::new(pos.x, pos.y, 0.5);
                    sprite.color = parent_color;
                }
            } else {
                let mut entity_commands = commands.spawn((
                    solid_sprite(&solid, parent_color, Vec2::new(16.0, 32.0)),
                    Transform::from_translation(Vec3::new(pos.x, pos.y, 0.5)),
                    ActorRenderTag { id: actor.id },
                    Name::new(format!("cf::render::actor::{}", actor.id)),
                ));
                entity_commands.insert(if actor.controllable {
                    Name::new(format!("cf::render::actor::{}::player", actor.id))
                } else {
                    Name::new(format!("cf::render::actor::{}::npc", actor.id))
                });
            }
            if Some(actor.id) == state.player_actor_id {
                player_position = Some(pos);
                player_aim = Some(Vec2::new(actor.aim[0], actor.aim[1]));
            }
        }

        // Despawn actors that left the world.
        for (entity, tag, _, _) in actor_query.iter() {
            if !keep.contains_key(&tag.id) {
                commands.entity(entity).despawn();
            }
        }

        // Reticle follows the player's aim. M1 Gap E4: scale by bloom factor
        // (with sharp-aim tightening) and tint red when tool_validity says the
        // current action would refuse.
        let player_actor_ref = state
            .player_actor_id
            .and_then(|id| state.actors.iter().find(|a| a.id == id));
        if let (Some(pos), Some(aim)) = (player_position, player_aim) {
            if let Some((mut transform, mut visibility)) = reticle_query.iter_mut().next() {
                let aim_unit = if aim.length_squared() > 1e-6 {
                    aim.normalize()
                } else {
                    Vec2::new(1.0, 0.0)
                };
                transform.translation = Vec3::new(pos.x + aim_unit.x * 32.0, pos.y + aim_unit.y * 32.0, 1.0);
                let bloom = player_actor_ref.map(|p| p.bloom_factor).unwrap_or(1.0);
                let sharp = player_actor_ref.map(|p| p.sharp_aim_progress).unwrap_or(0.0);
                let final_scale = (bloom * (1.0 - 0.6 * sharp)).clamp(0.4, 10.0);
                transform.scale = Vec3::new(final_scale, final_scale, 1.0);
                *visibility = Visibility::Visible;
            }
            // Reticle color: red when tool refused, white otherwise.
            if let Some((_, _)) = reticle_query.iter_mut().next() {
                // ReticleRenderTag entity has Sprite; we don't carry it in this
                // query. The Sprite query is one block above (mut_query). We
                // separately update color in `update_reticle_color` below.
            }
        } else if let Some((_, mut visibility)) = reticle_query.iter_mut().next() {
            *visibility = Visibility::Hidden;
        }

        // Mark the resource clean for the next bridge write.
        state.set_changed();
    }

    /// **M5**: spawn / update / despawn the 15 chassis-zone pips per chassis-attached
    /// actor. Each pip is a small colored rect anchored at the zone's anatomical
    /// offset (Head at top, Hand-Right on the right side, Foot-Left at the bottom-
    /// left, etc.) inside the actor's silhouette. The pip color reflects the zone's
    /// external_integrity + destroyed flag + chassis stage tint, so a player
    /// watching the wreck_eject scenario can SEE the right forearm pip turn black
    /// when that zone is destroyed, and the whole silhouette turn red when the
    /// chassis transitions to Wreck stage. This closes the M5-DC-3 gap from the
    /// audit ("static sliding pawn" / "visible actor is still a M1 rectangle").
    #[allow(clippy::too_many_arguments)]
    fn sync_chassis_zone_sprites(
        mut commands: Commands,
        state: Res<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut zone_query: Query<(
            Entity,
            &ChassisZoneRenderTag,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        )>,
    ) {
        use std::collections::{HashMap, HashSet};

        // Map every existing zone-pip entity by (actor_id, zone) so we can update,
        // hide, or despawn per-frame.
        let mut existing: HashMap<(u64, String), Entity> = HashMap::new();
        for (entity, tag, _, _, _) in zone_query.iter() {
            existing.insert((tag.actor_id, tag.zone.clone()), entity);
        }

        let mut keep: HashSet<(u64, String)> = HashSet::new();

        for actor in &state.actors {
            // **M5**: every actor renders as a humanoid silhouette via the
            // CHASSIS_ZONE_LAYOUT — chassis-attached actors use real per-zone
            // hp from `chassis.zones[]`; chassis-less actors (M1 baseline,
            // micro_breach, m2/m2.5/m4a scenarios) use a synthetic intact
            // chassis-view derived from the actor's HP so the visible body
            // STILL renders as a body, not a flat colored rectangle. This
            // closes the M5-DC-3 "static sliding pawn" gap for ALL scenarios,
            // not just M5.
            let (chassis_kind, chassis_stage) = if let Some(c) = actor.chassis.as_ref() {
                (c.kind.as_str(), c.stage.as_str())
            } else {
                ("infantry", "nominal")
            };
            let base_color = actor_color(actor);
            let stage_tint = chassis_stage_tint(chassis_stage);
            // Apply BOTH the chassis kind multiplier AND the global base scale
            // (`ACTOR_SILHOUETTE_BASE_SCALE`) so the on-screen silhouette is
            // visible at battlefield zoom.
            let scale = chassis_scale_multiplier(chassis_kind) * ACTOR_SILHOUETTE_BASE_SCALE;
            let (off_x, off_y, height_scale) = stance_offset(&actor.stance);
            let actor_pos = Vec2::new(actor.position[0], actor.position[1]);

            // Build a zone-data lookup so the 15-pip layout can pull
            // per-zone external/internal/core/wound integrity + destroyed flag.
            // For chassis-attached actors, populate from `chassis.zones[]`. For
            // chassis-less actors (M1 baseline, micro_breach, etc.), synthesize
            // from actor HP so the silhouette dims as the actor takes damage —
            // a fallback that keeps M1+ scenarios humanoid-looking without
            // requiring a real chassis grammar at those scenes.
            let mut zone_lookup: HashMap<&str, ActorChassisZoneView> = HashMap::new();
            if let Some(c) = actor.chassis.as_ref() {
                for z in &c.zones {
                    zone_lookup.insert(
                        z.zone.as_str(),
                        ActorChassisZoneView {
                            zone: z.zone.clone(),
                            external_integrity: z.external_integrity,
                            internal_integrity: z.internal_integrity,
                            core_integrity: z.core_integrity,
                            wound_integrity: z.wound_integrity,
                            destroyed: z.destroyed,
                        },
                    );
                }
            } else {
                // M1 baseline fallback: derive a synthetic intact body from
                // actor HP. The whole body dims uniformly as HP drops — no
                // per-zone damage, just enough to make the silhouette visible
                // and react to overall actor health.
                let hp_pct = if actor.hp_max > 0.0 {
                    (actor.hp / actor.hp_max).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                for (zone_name, _, _, _, _) in CHASSIS_ZONE_LAYOUT {
                    zone_lookup.insert(
                        zone_name,
                        ActorChassisZoneView {
                            zone: (*zone_name).to_string(),
                            external_integrity: hp_pct,
                            internal_integrity: hp_pct,
                            core_integrity: hp_pct,
                            wound_integrity: hp_pct,
                            destroyed: false,
                        },
                    );
                }
            }

            // Walk-cycle leg offsets when stance is locomotive (Walking/Running).
            // Velocity sign is mirrored so legs cycle correctly when running L/R.
            let velocity_x = actor.velocity[0];
            let (left_leg_dy, right_leg_dy) = walk_cycle_offsets(state.tick, &actor.stance, velocity_x);

            for (zone_name, dx, dy, w, h) in CHASSIS_ZONE_LAYOUT {
                let default_view = ActorChassisZoneView {
                    zone: (*zone_name).to_string(),
                    external_integrity: 1.0,
                    internal_integrity: 1.0,
                    core_integrity: 1.0,
                    wound_integrity: 1.0,
                    destroyed: false,
                };
                let view = zone_lookup.get(zone_name).unwrap_or(&default_view);
                let color = zone_color(base_color, view, stage_tint);

                // Per-zone walk-cycle Y offset: shins+feet on each side bounce in
                // opposite phase so legs visibly alternate during locomotion.
                let walk_dy = match *zone_name {
                    "leg_left" | "shin_left" | "foot_left" => left_leg_dy * 0.6,
                    "leg_right" | "shin_right" | "foot_right" => right_leg_dy * 0.6,
                    _ => 0.0,
                };

                // Apply stance offset + chassis kind scale + walk cycle.
                let pip_x = actor_pos.x + (dx * scale) + off_x;
                let pip_y = actor_pos.y + (dy * scale * height_scale) + off_y + walk_dy;
                let pip_w = w * scale;
                let pip_h = h * scale * height_scale;

                // Backpack renders behind torso (z = 0.45 vs 0.55 for others).
                let z = if *zone_name == "backpack" { 0.45 } else { 0.55 };

                let key = (actor.id, zone_name.to_string());
                keep.insert(key.clone());

                if let Some(entity) = existing.get(&key) {
                    if let Ok((_, _, mut transform, mut sprite, mut visibility)) = zone_query.get_mut(*entity) {
                        transform.translation = Vec3::new(pip_x, pip_y, z);
                        sprite.color = color;
                        sprite.custom_size = Some(Vec2::new(pip_w, pip_h));
                        *visibility = Visibility::Inherited;
                    }
                } else {
                    commands.spawn((
                        solid_sprite(&solid, color, Vec2::new(pip_w, pip_h)),
                        Transform::from_translation(Vec3::new(pip_x, pip_y, z)),
                        ChassisZoneRenderTag {
                            actor_id: actor.id,
                            zone: zone_name.to_string(),
                        },
                        Name::new(format!("cf::render::chassis_zone::{}::{}", actor.id, zone_name)),
                    ));
                }
            }
        }

        // Despawn pips whose owning actor + zone is no longer present (actor left
        // the world, chassis detached on eject, etc.).
        for (entity, tag, _, _, _) in zone_query.iter() {
            if !keep.contains(&(tag.actor_id, tag.zone.clone())) {
                commands.entity(entity).despawn();
            }
        }
    }

    /// **M5**: spawn / update / despawn module overlay pips (jet / shield /
    /// sensor / weapon_mount / repair_drone) per chassis-attached actor. The pip
    /// color reflects the module state and position follows the bound zone. This
    /// surfaces simulation depth Cortex doesn't have — the silhouette visibly
    /// shows which modules are still healthy, which are degraded, which failed.
    #[allow(clippy::too_many_arguments)]
    fn sync_chassis_module_sprites(
        mut commands: Commands,
        state: Res<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut module_query: Query<(
            Entity,
            &ChassisModuleRenderTag,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        )>,
    ) {
        use std::collections::{HashMap, HashSet};

        let mut existing: HashMap<(u64, String), Entity> = HashMap::new();
        for (entity, tag, _, _, _) in module_query.iter() {
            existing.insert((tag.actor_id, tag.module_id.clone()), entity);
        }
        let mut keep: HashSet<(u64, String)> = HashSet::new();

        for actor in &state.actors {
            let Some(chassis) = actor.chassis.as_ref() else {
                continue;
            };
            let actor_pos = Vec2::new(actor.position[0], actor.position[1]);
            let scale = chassis_scale_multiplier(&chassis.kind) * ACTOR_SILHOUETTE_BASE_SCALE;
            let (off_x, off_y, _height_scale) = stance_offset(&actor.stance);

            // Build zone position lookup so each module renders at its bound
            // zone's offset.
            let zone_offset = |zone: &str| -> (f32, f32) {
                for (zname, dx, dy, _, _) in CHASSIS_ZONE_LAYOUT {
                    if *zname == zone {
                        return (*dx, *dy);
                    }
                }
                (0.0, 0.0)
            };

            for module in &chassis.modules {
                let Some(color) = module_pip_color(&module.state) else {
                    continue;
                };
                let (zdx, zdy) = zone_offset(&module.bound_zone);
                let (mdx, mdy, msize) = module_pip_offset(&module.kind);
                let pip_x = actor_pos.x + (zdx + mdx) * scale + off_x;
                let pip_y = actor_pos.y + (zdy + mdy) * scale + off_y;
                let pip_size = msize * scale;
                let z = 0.65; // Modules render in FRONT of zone pips.

                let key = (actor.id, module.id.clone());
                keep.insert(key.clone());

                if let Some(entity) = existing.get(&key) {
                    if let Ok((_, _, mut transform, mut sprite, mut visibility)) = module_query.get_mut(*entity) {
                        transform.translation = Vec3::new(pip_x, pip_y, z);
                        sprite.color = color;
                        sprite.custom_size = Some(Vec2::new(pip_size, pip_size));
                        *visibility = Visibility::Inherited;
                    }
                } else {
                    commands.spawn((
                        solid_sprite(&solid, color, Vec2::new(pip_size, pip_size)),
                        Transform::from_translation(Vec3::new(pip_x, pip_y, z)),
                        ChassisModuleRenderTag {
                            actor_id: actor.id,
                            module_id: module.id.clone(),
                        },
                        Name::new(format!("cf::render::chassis_module::{}::{}", actor.id, module.id)),
                    ));
                }
            }
        }

        for (entity, tag, _, _, _) in module_query.iter() {
            if !keep.contains(&(tag.actor_id, tag.module_id.clone())) {
                commands.entity(entity).despawn();
            }
        }
    }

    /// **M5**: spawn / update / despawn the held rifle sprite per actor whose
    /// inventory carries a rifle. The rifle pip is a 12×3 rectangle anchored at
    /// the right-hand zone (or torso for chassis-less actors), rotated to point
    /// along the actor's aim vector. Without this, the actor has NO visible
    /// weapon despite firing — only the projectile + muzzle flash hint at it.
    fn sync_held_rifle_sprites(
        mut commands: Commands,
        state: Res<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut rifle_query: Query<(
            Entity,
            &HeldRifleRenderTag,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        )>,
    ) {
        use std::collections::{HashMap, HashSet};

        let mut existing: HashMap<u64, Entity> = HashMap::new();
        for (entity, tag, _, _, _) in rifle_query.iter() {
            existing.insert(tag.actor_id, entity);
        }
        let mut keep: HashSet<u64> = HashSet::new();

        for actor in &state.actors {
            // Only actors holding a rifle render a held weapon. `selected_item`
            // is the label produced by `InventoryItem::label()` — "rifle" for the
            // Rifle variant. Future melee/sidearm items follow the same pattern.
            if actor.selected_item != "rifle" {
                continue;
            }
            let actor_pos = Vec2::new(actor.position[0], actor.position[1]);
            let aim = Vec2::new(actor.aim[0], actor.aim[1]);
            let aim_unit = if aim.length_squared() > 1e-6 {
                aim.normalize()
            } else {
                Vec2::new(1.0, 0.0)
            };

            // Anchor at right-hand zone for every actor (chassis-attached uses
            // its kind multiplier; chassis-less uses Infantry default 0.9). Both
            // multiply by ACTOR_SILHOUETTE_BASE_SCALE so the rifle pip stays
            // proportional to the silhouette.
            let chassis_kind = actor.chassis.as_ref().map(|c| c.kind.as_str()).unwrap_or("infantry");
            let scale = chassis_scale_multiplier(chassis_kind) * ACTOR_SILHOUETTE_BASE_SCALE;
            let anchor_dx = 7.0 * scale;
            let anchor_dy = -3.0 * scale;
            let (off_x, off_y, _height_scale) = stance_offset(&actor.stance);
            // Rifle extends 8 px forward from the hand along aim direction.
            let muzzle_extend = 8.0 * scale;
            let rifle_center_x = actor_pos.x + anchor_dx + off_x + aim_unit.x * muzzle_extend * 0.5;
            let rifle_center_y = actor_pos.y + anchor_dy + off_y + aim_unit.y * muzzle_extend * 0.5;

            let rifle_color = if matches!(actor.team.as_str(), "blue") {
                Color::srgb(0.18, 0.20, 0.24)
            } else if matches!(actor.team.as_str(), "red") {
                Color::srgb(0.24, 0.18, 0.18)
            } else {
                Color::srgb(0.20, 0.20, 0.20)
            };
            let rifle_w = (12.0 * scale).max(8.0);
            let rifle_h = (2.5 * scale).max(2.0);
            let angle = aim_unit.y.atan2(aim_unit.x);

            keep.insert(actor.id);

            if let Some(entity) = existing.get(&actor.id) {
                if let Ok((_, _, mut transform, mut sprite, mut visibility)) = rifle_query.get_mut(*entity) {
                    transform.translation = Vec3::new(rifle_center_x, rifle_center_y, 0.70);
                    transform.rotation = Quat::from_rotation_z(angle);
                    sprite.color = rifle_color;
                    sprite.custom_size = Some(Vec2::new(rifle_w, rifle_h));
                    *visibility = Visibility::Inherited;
                }
            } else {
                let mut transform = Transform::from_translation(Vec3::new(rifle_center_x, rifle_center_y, 0.70));
                transform.rotation = Quat::from_rotation_z(angle);
                commands.spawn((
                    solid_sprite(&solid, rifle_color, Vec2::new(rifle_w, rifle_h)),
                    transform,
                    HeldRifleRenderTag { actor_id: actor.id },
                    Name::new(format!("cf::render::held_rifle::{}", actor.id)),
                ));
            }
        }

        for (entity, tag, _, _, _) in rifle_query.iter() {
            if !keep.contains(&tag.actor_id) {
                commands.entity(entity).despawn();
            }
        }
    }

    /// M1.5: spawn / update / despawn breach strip sprites from the engine snapshot.
    fn sync_breach_sprites(
        mut commands: Commands,
        state: Res<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut breach_query: Query<(Entity, &BreachRenderTag, &mut Transform, &mut Sprite)>,
    ) {
        use std::collections::HashMap;
        let mut existing: HashMap<String, Entity> = HashMap::new();
        for (entity, tag, _, _) in breach_query.iter() {
            existing.insert(tag.id.clone(), entity);
        }
        let mut keep: HashMap<String, ()> = HashMap::new();
        for breach in &state.breaches {
            keep.insert(breach.id.clone(), ());
            let centre = Vec2::new(
                (breach.bbox_min[0] + breach.bbox_max[0]) * 0.5,
                (breach.bbox_min[1] + breach.bbox_max[1]) * 0.5,
            );
            let size = Vec2::new(
                (breach.bbox_max[0] - breach.bbox_min[0]).max(1.0),
                (breach.bbox_max[1] - breach.bbox_min[1]).max(1.0),
            );
            let color = breach_color(breach);
            if let Some(entity) = existing.get(&breach.id) {
                if let Ok((_, _, mut transform, mut sprite)) = breach_query.get_mut(*entity) {
                    transform.translation = Vec3::new(centre.x, centre.y, -0.25);
                    sprite.color = color;
                    sprite.custom_size = Some(size);
                }
            } else {
                commands.spawn((
                    solid_sprite(&solid, color, size),
                    Transform::from_translation(Vec3::new(centre.x, centre.y, -0.25)),
                    BreachRenderTag { id: breach.id.clone() },
                    Name::new(format!("cf::render::breach::{}", breach.id)),
                ));
            }
        }
        for (entity, tag, _, _) in breach_query.iter() {
            if !keep.contains_key(&tag.id) {
                commands.entity(entity).despawn();
            }
        }
    }

    /// M1.5: spawn / update / despawn the extraction-zone sprite.
    fn sync_extraction_zone(
        mut commands: Commands,
        state: Res<ActorRenderState>,
        solid: Res<SolidSpriteImage>,
        mut zone_query: Query<(Entity, &mut Transform, &mut Sprite), With<ExtractionZoneTag>>,
    ) {
        match (&state.extraction_zone, zone_query.iter_mut().next()) {
            (Some(zone), Some((_, mut transform, mut sprite))) => {
                let centre = Vec2::new((zone.min[0] + zone.max[0]) * 0.5, (zone.min[1] + zone.max[1]) * 0.5);
                let size = Vec2::new(
                    (zone.max[0] - zone.min[0]).max(1.0),
                    (zone.max[1] - zone.min[1]).max(1.0),
                );
                transform.translation = Vec3::new(centre.x, centre.y, -0.4);
                sprite.color = if zone.completed {
                    Color::srgba(0.30, 0.95, 0.50, 0.40)
                } else {
                    Color::srgba(0.30, 0.85, 0.30, 0.25)
                };
                sprite.custom_size = Some(size);
            }
            (Some(zone), None) => {
                let centre = Vec2::new((zone.min[0] + zone.max[0]) * 0.5, (zone.min[1] + zone.max[1]) * 0.5);
                let size = Vec2::new(
                    (zone.max[0] - zone.min[0]).max(1.0),
                    (zone.max[1] - zone.min[1]).max(1.0),
                );
                commands.spawn((
                    solid_sprite(&solid, Color::srgba(0.30, 0.85, 0.30, 0.25), size),
                    Transform::from_translation(Vec3::new(centre.x, centre.y, -0.4)),
                    ExtractionZoneTag,
                    Name::new("cf::render::extraction_zone"),
                ));
            }
            (None, Some((entity, _, _))) => {
                commands.entity(entity).despawn();
            }
            (None, None) => {}
        }
    }

    fn breach_color(breach: &BreachRender) -> Color {
        if breach.broken {
            Color::srgba(0.40, 0.30, 0.20, 0.25)
        } else if breach.refusal_reason.is_some() {
            Color::srgb(0.55, 0.55, 0.60)
        } else {
            let pct = if breach.max_hp > 0.0 {
                (breach.hp / breach.max_hp).clamp(0.0, 1.0)
            } else {
                1.0
            };
            // Solid concrete tone darkens as the strip is dug down.
            let v = 0.25 + 0.40 * pct;
            Color::srgb(v, v * 0.85, v * 0.70)
        }
    }

    fn actor_color(actor: &ActorObservation) -> Color {
        let base = match actor.team.as_str() {
            "blue" => Color::srgb(0.30, 0.55, 0.95),
            "red" => Color::srgb(0.85, 0.30, 0.30),
            _ => Color::srgb(0.70, 0.70, 0.70),
        };
        if actor.status == "dead" {
            Color::srgb(0.20, 0.20, 0.20)
        } else if actor.status == "downed" {
            let s = base.to_srgba();
            Color::srgb(s.red * 0.5, s.green * 0.5, s.blue * 0.5)
        } else {
            base
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn plugin_inserts_clear_color() {
            let mut app = App::new();
            app.add_plugins(CfRenderPlugin::default());
            let cc = app.world().resource::<ClearColor>();
            let bg = cc.0.to_srgba();
            let target = M0_CLEAR_COLOR.to_srgba();
            assert!((bg.red - target.red).abs() < 1e-3);
            assert!((bg.green - target.green).abs() < 1e-3);
            assert!((bg.blue - target.blue).abs() < 1e-3);
        }

        #[test]
        fn actor_sprite_plugin_initialises_state() {
            let mut app = App::new();
            // Bevy 0.18: ActorSpritePlugin needs Assets<Image> for the
            // SolidSpriteImage 1x1 white texture (see SolidSpriteImage doc-comment).
            // MinimalPlugins doesn't include AssetPlugin/ImagePlugin, so we add the
            // asset registration manually for the unit test.
            app.add_plugins((MinimalPlugins, AssetPlugin::default()))
                .init_asset::<Image>()
                .add_plugins(ActorSpritePlugin);
            app.update();
            let state = app.world().resource::<ActorRenderState>();
            assert!(state.actors.is_empty());
        }
    }
} // end mod live_render
