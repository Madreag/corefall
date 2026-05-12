//! **M2**: tool-validity ghost preview.
//!
//! cf-app's bridge probes the terrain in front of the player each frame
//! (`actor.position + aim * dig_range`) and writes the result into
//! [`DigPreviewGhost`]:
//!
//! - `target: Some({ position, radius, valid, material_id })` when the dig
//!   tool is selected.
//! - `target: None` when no dig tool is held.
//!
//! The renderer draws a translucent circle at the probe point colored:
//!
//! - GREEN if `valid=true` (target material is `diggable=true`)
//! - RED if `valid=false` (refusal material like metal_nohook)
//! - Hidden when `target=None`.
//!
//! Opacity scales with `Settings::reduced_motion`: full at default,
//! halved when reduced_motion = true (accessibility floor).

use bevy::prelude::*;

use cf_terrain::MaterialId;

/// Render-side ghost preview data. cf-app writes this resource each
/// frame after computing the dig probe; the renderer reads + draws.
#[derive(Resource, Debug, Clone, Default)]
pub struct DigPreviewGhost {
    pub target: Option<DigPreviewTarget>,
    pub reduced_motion: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DigPreviewTarget {
    pub position: Vec2,
    pub radius: f32,
    pub valid: bool,
    pub material_id: Option<MaterialId>,
}

/// Marker component for the spawned ghost sprite. One per render layer
/// (we re-use the same entity each frame rather than respawn).
#[derive(Component, Debug)]
pub struct DigPreviewSprite;

/// Plugin registering the resource + per-frame update system.
pub struct DigPreviewPlugin;

impl Plugin for DigPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DigPreviewGhost>()
            .add_systems(Startup, spawn_dig_preview_sprite)
            .add_systems(Update, update_dig_preview_sprite);
    }
}

fn spawn_dig_preview_sprite(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.0),
            custom_size: Some(Vec2::new(24.0, 24.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.95)),
        Visibility::Hidden,
        DigPreviewSprite,
        Name::new("cf::render::dig_preview_ghost"),
    ));
}

fn update_dig_preview_sprite(
    ghost: Res<DigPreviewGhost>,
    mut q: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<DigPreviewSprite>>,
) {
    let Some((mut transform, mut sprite, mut visibility)) = q.iter_mut().next() else {
        return;
    };
    match ghost.target {
        None => {
            *visibility = Visibility::Hidden;
        }
        Some(t) => {
            transform.translation = Vec3::new(t.position.x, t.position.y, 0.95);
            let diameter = (t.radius * 2.0).max(8.0);
            sprite.custom_size = Some(Vec2::new(diameter, diameter));
            let alpha = if ghost.reduced_motion { 0.25 } else { 0.5 };
            sprite.color = if t.valid {
                Color::srgba(0.30, 0.85, 0.30, alpha)
            } else {
                Color::srgba(0.95, 0.30, 0.30, alpha)
            };
            *visibility = Visibility::Visible;
        }
    }
}

/// Pure helper resolving the validity of a dig probe against a material
/// affordance. Useful for tests + cf-app to compute the target without
/// duplicating the rule.
#[must_use]
pub fn probe_dig_validity(material_id: MaterialId) -> bool {
    cf_terrain::material_affordance(material_id).is_some_and(|a| a.diggable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_terrain::{MATERIAL_DIRT, MATERIAL_METAL_NOHOOK};

    fn preview_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(DigPreviewPlugin);
        app
    }

    #[test]
    fn dirt_is_diggable_metal_is_not() {
        assert!(probe_dig_validity(MATERIAL_DIRT));
        assert!(!probe_dig_validity(MATERIAL_METAL_NOHOOK));
    }

    #[test]
    fn ghost_color_flips_red_when_target_invalid() {
        let mut app = preview_test_app();
        // First tick: target is dirt (valid).
        {
            let mut ghost = app.world_mut().resource_mut::<DigPreviewGhost>();
            ghost.target = Some(DigPreviewTarget {
                position: Vec2::new(100.0, 50.0),
                radius: 12.0,
                valid: true,
                material_id: Some(MATERIAL_DIRT),
            });
        }
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(&Sprite, &Visibility, &DigPreviewSprite)>();
        let (sprite, visibility, _) = q.iter(world).next().expect("ghost spawned");
        let valid_color = sprite.color;
        assert!(matches!(visibility, Visibility::Visible));
        // Now switch to metal (invalid).
        {
            let mut ghost = app.world_mut().resource_mut::<DigPreviewGhost>();
            ghost.target = Some(DigPreviewTarget {
                position: Vec2::new(100.0, 50.0),
                radius: 12.0,
                valid: false,
                material_id: Some(MATERIAL_METAL_NOHOOK),
            });
        }
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&Sprite>();
        let mut last_color = None;
        for s in q.iter(world) {
            last_color = Some(s.color);
        }
        let invalid_color = last_color.expect("sprite still exists");
        assert_ne!(valid_color, invalid_color, "color must flip on invalid target");
        // Invalid target should bias red.
        let rgba = invalid_color.to_srgba();
        assert!(rgba.red > rgba.green && rgba.red > rgba.blue);
    }

    #[test]
    fn ghost_hidden_when_target_none() {
        let mut app = preview_test_app();
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(&Visibility, &DigPreviewSprite)>();
        let (visibility, _) = q.iter(world).next().expect("ghost spawned");
        assert!(matches!(visibility, Visibility::Hidden));
    }

    #[test]
    fn reduced_motion_halves_alpha() {
        let mut app = preview_test_app();
        {
            let mut ghost = app.world_mut().resource_mut::<DigPreviewGhost>();
            ghost.reduced_motion = false;
            ghost.target = Some(DigPreviewTarget {
                position: Vec2::ZERO,
                radius: 12.0,
                valid: true,
                material_id: Some(MATERIAL_DIRT),
            });
        }
        app.update();
        let full_alpha = {
            let world = app.world_mut();
            let mut q = world.query::<&Sprite>();
            q.iter(world).next().unwrap().color.to_srgba().alpha
        };
        {
            let mut ghost = app.world_mut().resource_mut::<DigPreviewGhost>();
            ghost.reduced_motion = true;
        }
        app.update();
        let reduced_alpha = {
            let world = app.world_mut();
            let mut q = world.query::<&Sprite>();
            q.iter(world).next().unwrap().color.to_srgba().alpha
        };
        assert!(reduced_alpha < full_alpha);
    }
}
