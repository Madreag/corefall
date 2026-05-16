//! Splash screen — 3s brand reveal + engine init bar; press any key skips.

use crate::state::{ShellScreen, ShellState, ShellTransition, TransitionSource};
use bevy::prelude::*;

pub const SPLASH_DURATION_MS: u32 = 3000;

#[derive(Debug, Clone)]
pub struct SplashState {
    pub elapsed_ms: u32,
    pub progress: f32,
    pub brand_logo_id: &'static str,
}

impl Default for SplashState {
    fn default() -> Self {
        Self {
            elapsed_ms: 0,
            progress: 0.0,
            brand_logo_id: "menu_brand_logo_full",
        }
    }
}

/// Tick splash elapsed time. When elapsed >= SPLASH_DURATION_MS, transition
/// to title.
pub fn tick_splash(
    time: Res<Time>,
    mut state: ResMut<ShellState>,
    mut transitions: MessageWriter<ShellTransition>,
) {
    if state.current != ShellScreen::Splash {
        return;
    }
    let dt_ms = (time.delta_secs() * 1000.0) as u32;
    state.splash_elapsed_ms = state.splash_elapsed_ms.saturating_add(dt_ms);
    if state.splash_elapsed_ms >= SPLASH_DURATION_MS {
        transitions.write(ShellTransition {
            to: ShellScreen::Title,
            source: TransitionSource::SplashTimeout,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::apply_shell_transitions;

    #[test]
    fn splash_state_default_is_zero() {
        let s = SplashState::default();
        assert_eq!(s.elapsed_ms, 0);
        assert_eq!(s.progress, 0.0);
    }

    #[test]
    fn splash_advances_after_3s() {
        let mut app = App::new();
        app.init_resource::<ShellState>()
            .add_message::<ShellTransition>()
            .insert_resource(Time::<()>::default())
            .add_systems(Update, (tick_splash, apply_shell_transitions).chain());

        // Manually drive time forward by ~3.5s in 100ms increments
        for _ in 0..36 {
            let mut time = app.world_mut().resource_mut::<Time<()>>();
            time.advance_by(std::time::Duration::from_millis(100));
            app.update();
        }
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::Title);
    }
}
