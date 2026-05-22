use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

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

pub(crate) fn build_solid_sprite_image(mut images: ResMut<Assets<Image>>, mut handle: ResMut<SolidSpriteImage>) {
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

pub(crate) fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Name::new("cf::render::main_camera")));
}
