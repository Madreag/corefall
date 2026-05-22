use bevy::prelude::*;

use crate::debris::DebrisPlugin;
use crate::dig_preview::DigPreviewPlugin;
use crate::live_camera_effects::{CameraFollow, CameraShake, HitStop};
use crate::live_sprite_image::spawn_camera;
use crate::overlay::OverlayModePlugin;
use crate::chem_flash::ChemFlashState;
use crate::reactor_explosion::ExplosionState;
use crate::reactor_sparks::SparkEmitterState;
use crate::reactor_sprite::ReactorSpriteState;
use crate::terrain::ChunkedTerrainRendererPlugin;

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
            .init_resource::<ChemFlashState>()
            .add_systems(Update, tick_reactor_vfx);
    }
}

fn tick_reactor_vfx(
    time: Res<Time>,
    mut sparks: ResMut<SparkEmitterState>,
    mut explosion: ResMut<ExplosionState>,
    mut chem_flash: ResMut<ChemFlashState>,
) {
    let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
    if dt_ms == 0 {
        return;
    }
    sparks.tick(dt_ms);
    explosion.tick(dt_ms);
    chem_flash.tick(dt_ms);
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
}
