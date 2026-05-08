//! Bevy rendering primitives for the corefall app shell.
//!
//! M0 ships a minimal clear-screen plugin (`CfRenderPlugin`) that inserts a
//! [`ClearColor`] resource and spawns a 2D camera. M1 adds actor sprite rendering
//! through the [`ActorSpritePlugin`]: the engine publishes its actor world into the
//! Bevy ECS via [`ActorRenderState`], and a render system spawns / updates simple
//! colored rectangles for each actor + the M1 floor + a reticle anchored at the
//! player's aim. Everything stays cosmetic-only — `cf-control`'s `M0Engine` is the
//! single source of truth for sim state.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cf_actor::ActorObservation;

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
            .add_systems(Startup, spawn_camera);
    }
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

/// Plugin that wires actor / floor / reticle rendering. Call after [`CfRenderPlugin`].
pub struct ActorSpritePlugin;

impl Plugin for ActorSpritePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorRenderState>()
            .init_resource::<SolidSpriteImage>()
            .add_systems(
                Startup,
                (build_solid_sprite_image, spawn_floor_and_reticle).chain(),
            )
            .add_systems(
                Update,
                (sync_actor_sprites, sync_breach_sprites, sync_extraction_zone).chain(),
            );
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
        // Centre the 2D camera on the play region so authored scenarios in
        // bottom-left coordinates (e.g. M1's 1280x720 with target at x=900) stay
        // on-screen. The default Bevy 2D camera sits at the world origin, which
        // would clip everything past x = window_width / 2.
        if let Some(mut camera_transform) = camera_query.iter_mut().next() {
            camera_transform.translation.x = region_center_x;
            camera_transform.translation.y = region_center_y;
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
        let color = actor_color(actor);
        if let Some(entity) = existing.get(&actor.id) {
            if let Ok((_, _, mut transform, mut sprite)) = actor_query.get_mut(*entity) {
                transform.translation = Vec3::new(pos.x, pos.y, 0.5);
                sprite.color = color;
            }
        } else {
            let mut entity_commands = commands.spawn((
                solid_sprite(&solid, color, Vec2::new(16.0, 32.0)),
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

    // Reticle follows the player's aim.
    if let (Some(pos), Some(aim)) = (player_position, player_aim) {
        if let Some((mut transform, mut visibility)) = reticle_query.iter_mut().next() {
            let aim_unit = if aim.length_squared() > 1e-6 {
                aim.normalize()
            } else {
                Vec2::new(1.0, 0.0)
            };
            transform.translation = Vec3::new(pos.x + aim_unit.x * 32.0, pos.y + aim_unit.y * 32.0, 1.0);
            *visibility = Visibility::Visible;
        }
    } else if let Some((_, mut visibility)) = reticle_query.iter_mut().next() {
        *visibility = Visibility::Hidden;
    }

    // Mark the resource clean for the next bridge write.
    state.set_changed();
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
