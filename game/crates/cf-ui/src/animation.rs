//! **M12** § UI panel transitions + animation hooks.
//!
//! Per DR-046 + M12 spec § Animation system:
//! - UI panel transitions (slide + skew + ease).
//! - Per-element animation hooks (entry, exit, hover, focus, click, drag).
//! - Animation system respects `reduce_motion` setting (instant transitions).
//!
//! Mirrors `cf-render-2d::juice` but at the cf-ui layer where individual
//! HUD panels (banner stack, captions strip, mission strip, settings tabs)
//! drive their own entry/exit timelines without going through the shared
//! juice queue. The two modules co-exist by purpose:
//!
//! - cf-render-2d::juice handles **transient impact pulses** (button
//!   hover, click punch, hit-stop, banner slide-in).
//! - cf-ui::animation handles **panel lifecycle** (entry, exit, drag,
//!   focus-ring fade) across the HUD's larger UI containers.

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum AnimationHook {
    /// Panel appearing (slide + fade-in).
    Entry,
    /// Panel disappearing (slide + fade-out).
    Exit,
    /// Hover state engaged.
    Hover,
    /// Focus state engaged (keyboard / controller).
    Focus,
    /// Click state engaged (press).
    Click,
    /// Drag state (e.g. window-move-via-handle).
    Drag,
}

impl AnimationHook {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AnimationHook::Entry => "entry",
            AnimationHook::Exit => "exit",
            AnimationHook::Hover => "hover",
            AnimationHook::Focus => "focus",
            AnimationHook::Click => "click",
            AnimationHook::Drag => "drag",
        }
    }

    /// Canonical full duration in ms.
    #[must_use]
    pub fn default_duration_ms(self) -> u32 {
        match self {
            AnimationHook::Entry => 200,
            AnimationHook::Exit => 180,
            AnimationHook::Hover => 80,
            AnimationHook::Focus => 80,
            AnimationHook::Click => 120,
            AnimationHook::Drag => 60,
        }
    }

    /// Parse from snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(value: &str) -> Option<AnimationHook> {
        Some(match value {
            "entry" => AnimationHook::Entry,
            "exit" => AnimationHook::Exit,
            "hover" => AnimationHook::Hover,
            "focus" => AnimationHook::Focus,
            "click" => AnimationHook::Click,
            "drag" => AnimationHook::Drag,
            _ => return None,
        })
    }
}

/// Ease-in-out curve `[0..=1] → [0..=1]`. Used for panel entry/exit.
#[must_use]
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Ease-out cubic curve `[0..=1] → [0..=1]`. Used for hover / click.
#[must_use]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    1.0 - u * u * u
}

/// ease per DR-046)". The skew rotation that pairs with the slide
/// during a panel `Entry` / `Exit`. Returns the skew angle in radians,
/// peaks at `~0.12 rad` mid-transition then settles back to zero. The
/// skew is `0.0` when the pulse is suppressed by `reduce_motion`.
///
/// Per the M12 vault reference `cortext_command_vault/engine/rendering-audio-input-ui.md`
/// — panels arriving "tilt then settle" sells the cinematic feel.
#[must_use]
pub fn panel_skew_radians(pulse: &AnimationPulse) -> f32 {
    if pulse.accessibility_suppressed {
        return 0.0;
    }
    if !matches!(pulse.hook, AnimationHook::Entry | AnimationHook::Exit) {
        return 0.0;
    }
    let t = pulse.progress().clamp(0.0, 1.0);
    // Triangular wave peaking at t=0.5, max amplitude 0.12 rad.
    let amp = 1.0 - (2.0 * t - 1.0).abs();
    if pulse.hook == AnimationHook::Exit {
        // Exit skews in the opposite direction so the panel leans away
        // before leaving the screen.
        -0.12 * amp
    } else {
        0.12 * amp
    }
}

/// The slide-axis offset that pairs with [`panel_skew_radians`]. Returns
/// the normalized progress from off-screen (0.0) to settled (1.0)
/// during Entry; inverted (1.0 → 0.0) during Exit.
#[must_use]
pub fn panel_slide_offset(pulse: &AnimationPulse) -> f32 {
    if pulse.accessibility_suppressed {
        return 1.0;
    }
    let eased = pulse.eased();
    match pulse.hook {
        AnimationHook::Entry => eased,
        AnimationHook::Exit => 1.0 - eased,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPulse {
    pub hook: AnimationHook,
    /// Total duration the curve spans (0 when suppressed by reduce_motion).
    pub total_ms: u32,
    /// Elapsed time within the curve.
    pub elapsed_ms: u32,
    /// True when reduce_motion short-circuited the animation. Consumers
    /// can still observe the hook for one frame to register the
    /// state-change.
    pub accessibility_suppressed: bool,
}

impl AnimationPulse {
    /// Construct a fresh pulse for `hook`, honoring `reduce_motion`.
    #[must_use]
    pub fn new(hook: AnimationHook, reduce_motion: bool) -> Self {
        if reduce_motion {
            Self {
                hook,
                total_ms: 0,
                elapsed_ms: 0,
                accessibility_suppressed: true,
            }
        } else {
            Self {
                hook,
                total_ms: hook.default_duration_ms(),
                elapsed_ms: 0,
                accessibility_suppressed: false,
            }
        }
    }

    /// Progress `[0..1]`. Suppressed pulses always report `1.0` (end-state).
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.accessibility_suppressed || self.total_ms == 0 {
            return 1.0;
        }
        (self.elapsed_ms as f32 / self.total_ms as f32).clamp(0.0, 1.0)
    }

    /// Eased progress (ease-in-out for entry/exit, ease-out-cubic for hover/click).
    #[must_use]
    pub fn eased(&self) -> f32 {
        let t = self.progress();
        match self.hook {
            AnimationHook::Entry | AnimationHook::Exit | AnimationHook::Drag => ease_in_out(t),
            AnimationHook::Hover | AnimationHook::Focus | AnimationHook::Click => ease_out_cubic(t),
        }
    }

    /// True when the pulse is fully consumed.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.accessibility_suppressed || self.elapsed_ms >= self.total_ms
    }
}

/// progress for each (node_id, hook) pair to drive opacity / scale / skew
/// on the corresponding Bevy entity.
#[derive(Resource, Debug, Default, Clone)]
pub struct AnimationState {
    pulses: Vec<(String, AnimationPulse)>,
}

impl AnimationState {
    /// Trigger an animation hook on `node_id`. Replaces any existing pulse
    /// on the same (node, hook) pair (the most recent state-change wins).
    pub fn trigger(&mut self, node_id: impl Into<String>, hook: AnimationHook, reduce_motion: bool) {
        let node = node_id.into();
        let pulse = AnimationPulse::new(hook, reduce_motion);
        if let Some(existing) = self
            .pulses
            .iter_mut()
            .find(|(n, p)| *n == node && p.hook == hook)
        {
            *existing = (node, pulse);
        } else {
            self.pulses.push((node, pulse));
        }
    }

    /// Tick every pulse by `dt_ms`. Done pulses linger until consumed —
    /// callers drain them via [`Self::take_done`].
    pub fn tick(&mut self, dt_ms: u32) {
        for (_node, pulse) in self.pulses.iter_mut() {
            pulse.elapsed_ms = pulse.elapsed_ms.saturating_add(dt_ms);
        }
    }

    /// Number of pulses tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pulses.len()
    }

    /// Whether anything is tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pulses.is_empty()
    }

    /// Find a single pulse by (node, hook).
    #[must_use]
    pub fn pulse_for(&self, node_id: &str, hook: AnimationHook) -> Option<&AnimationPulse> {
        self.pulses
            .iter()
            .find(|(n, p)| n == node_id && p.hook == hook)
            .map(|(_, p)| p)
    }

    /// Eased progress for the requested hook on `node_id`. Returns `0.0`
    /// when the hook is not active.
    #[must_use]
    pub fn eased_progress(&self, node_id: &str, hook: AnimationHook) -> f32 {
        self.pulse_for(node_id, hook)
            .map(AnimationPulse::eased)
            .unwrap_or(0.0)
    }

    /// Drain finished pulses (callers fire `ux.juice_applied` / other
    /// state-change events as the pulses retire).
    pub fn take_done(&mut self) -> Vec<(String, AnimationPulse)> {
        let mut drained = Vec::new();
        let mut i = 0;
        while i < self.pulses.len() {
            if self.pulses[i].1.is_done() {
                drained.push(self.pulses.remove(i));
            } else {
                i += 1;
            }
        }
        drained
    }
}

/// Per-frame system that ticks the animation table. cf-app or cf-shell
/// schedules this in `Update`.
pub fn tick_animations(time: Res<Time>, mut state: ResMut<AnimationState>) {
    let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
    if dt_ms == 0 {
        return;
    }
    state.tick(dt_ms);
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimationState>()
            .add_systems(Update, tick_animations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_round_trips() {
        for h in [
            AnimationHook::Entry,
            AnimationHook::Exit,
            AnimationHook::Hover,
            AnimationHook::Focus,
            AnimationHook::Click,
            AnimationHook::Drag,
        ] {
            assert_eq!(AnimationHook::from_str(h.as_str()), Some(h));
        }
    }

    #[test]
    fn ease_in_out_runs_zero_to_one() {
        assert!((ease_in_out(0.0)).abs() < 1e-4);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-4);
        let mid = ease_in_out(0.5);
        assert!(mid > 0.49 && mid < 0.51);
    }

    #[test]
    fn ease_out_cubic_runs_zero_to_one() {
        assert!(ease_out_cubic(0.0).abs() < 1e-4);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn pulse_progress_advances_with_tick() {
        let mut state = AnimationState::default();
        state.trigger("hud.banner", AnimationHook::Entry, false);
        assert_eq!(state.len(), 1);
        let p0 = state.pulse_for("hud.banner", AnimationHook::Entry).unwrap();
        assert!(p0.progress() < 0.1);
        state.tick(100);
        let p1 = state.pulse_for("hud.banner", AnimationHook::Entry).unwrap();
        assert!(p1.progress() > 0.4 && p1.progress() < 0.6);
        state.tick(200);
        let p2 = state.pulse_for("hud.banner", AnimationHook::Entry).unwrap();
        assert!(p2.is_done());
    }

    #[test]
    fn reduce_motion_collapses_to_end_state() {
        let mut state = AnimationState::default();
        state.trigger("hud.banner", AnimationHook::Entry, true);
        let p = state.pulse_for("hud.banner", AnimationHook::Entry).unwrap();
        assert!(p.accessibility_suppressed);
        assert!((p.eased() - 1.0).abs() < 1e-4);
        assert!(p.is_done());
    }

    #[test]
    fn take_done_drains_finished_pulses_only() {
        let mut state = AnimationState::default();
        state.trigger("a", AnimationHook::Entry, false);
        state.trigger("b", AnimationHook::Hover, true); // suppressed = done immediately
        let done = state.take_done();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].0, "b");
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn repeated_trigger_replaces_active_pulse() {
        let mut state = AnimationState::default();
        state.trigger("hud.banner", AnimationHook::Entry, false);
        state.tick(100);
        state.trigger("hud.banner", AnimationHook::Entry, false);
        // Replaced — elapsed back to zero.
        let p = state.pulse_for("hud.banner", AnimationHook::Entry).unwrap();
        assert_eq!(p.elapsed_ms, 0);
    }

    #[test]
    fn eased_progress_zero_when_no_pulse() {
        let state = AnimationState::default();
        assert!(state.eased_progress("nothing", AnimationHook::Entry).abs() < 1e-4);
    }

    #[test]
    fn panel_skew_peaks_midway_on_entry() {
        let mut pulse = AnimationPulse::new(AnimationHook::Entry, false);
        pulse.elapsed_ms = pulse.total_ms / 2;
        let skew = panel_skew_radians(&pulse);
        assert!(skew > 0.10 && skew < 0.13, "skew at mid = {skew}");
    }

    #[test]
    fn panel_skew_is_negative_on_exit() {
        let mut pulse = AnimationPulse::new(AnimationHook::Exit, false);
        pulse.elapsed_ms = pulse.total_ms / 2;
        let skew = panel_skew_radians(&pulse);
        assert!(skew < -0.10 && skew > -0.13, "exit skew at mid = {skew}");
    }

    #[test]
    fn panel_skew_zero_when_reduce_motion() {
        let pulse = AnimationPulse::new(AnimationHook::Entry, true);
        assert!(panel_skew_radians(&pulse).abs() < 1e-4);
    }

    #[test]
    fn panel_skew_zero_for_non_entry_exit() {
        for hook in [AnimationHook::Hover, AnimationHook::Focus, AnimationHook::Click, AnimationHook::Drag] {
            let pulse = AnimationPulse::new(hook, false);
            assert!(panel_skew_radians(&pulse).abs() < 1e-4, "hook {hook:?} should not skew");
        }
    }

    #[test]
    fn panel_slide_offset_reverses_on_exit() {
        let pulse = AnimationPulse::new(AnimationHook::Entry, false);
        let entry_t0 = panel_slide_offset(&pulse);
        assert!(entry_t0 < 0.01);
        let pulse = AnimationPulse::new(AnimationHook::Exit, false);
        let exit_t0 = panel_slide_offset(&pulse);
        assert!(exit_t0 > 0.99);
    }
}
