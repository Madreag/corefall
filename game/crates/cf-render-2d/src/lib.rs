//! M0-S05: minimal clear-screen Bevy plugin.
//!
//! Spawns a 2D camera with a fixed `ClearColor` so the window has a defined
//! cleared frame at every render pass. The chunked terrain pipeline lands at M2;
//! M0 only needs the cleared frame plus camera so cf-app can satisfy the
//! "open a window, clear screen, fixed title/version, ESC exits" gate.

use bevy::prelude::*;

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

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), Name::new("cf::render::main_camera")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_inserts_clear_color() {
        let mut app = App::new();
        app.add_plugins(CfRenderPlugin::default());
        let cc = app.world().resource::<ClearColor>();
        // Compare components individually to avoid f32 PartialEq fragility on Color types.
        let bg = cc.0.to_srgba();
        let target = M0_CLEAR_COLOR.to_srgba();
        assert!((bg.red - target.red).abs() < 1e-3);
        assert!((bg.green - target.green).abs() < 1e-3);
        assert!((bg.blue - target.blue).abs() < 1e-3);
    }
}
