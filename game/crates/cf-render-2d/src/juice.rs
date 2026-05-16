//! **M12** § Juice rules per DR-055 + DR-046.
//!
//! Per-element animation curves for the seven canonical juice surfaces:
//!
//! - **Button hover** — scale 1.0 → 1.05 over 80 ms ease-out + glow halo.
//! - **Click punch** — scale 1.0 → 0.95 → 1.0 over 120 ms.
//! - **Banner slide-in** — slide from edge over 200 ms ease-in-out.
//! - **Critical-hit punch** — hit-stop + screen flash + chromatic aberration.
//! - **Reload completion ding** — subtle SFX cue (audio-only; this module
//!   exposes the rule for the cosmetic decay state so the HUD can mirror it).
//! - **Weapon swap whoosh** — audio + brief light streak.
//! - **Pickup glow** — item pulses on hover.
//!
//! All rules respect three accessibility flags (mirrored from
//! `cf-control::Settings`):
//!
//! - `reduce_motion` — collapses scale/slide animations to their end-state
//!   immediately (no ease curve).
//! - `reduce_shake` — disables hit-stop camera shake amplitude.
//! - `reduce_flash` — disables screen-flash + chromatic aberration on
//!   critical hits.
//!
//! When any of those flags suppresses a rule, the [`JuiceRule::applied_kind`]
//! still returns the canonical name AND the matching
//! [`JuicePulse::accessibility_suppressed`] flag is set so the
//! `ux.juice_applied` replay event records the suppression.

use bevy::prelude::*;

/// **M12**: every juice rule recognized by the renderer. Mirrors the
/// `ux.juice_applied` schema enum so the snake_case identifier round-trips
/// through the replay surface.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum JuiceKind {
    ButtonHover,
    ClickPunch,
    BannerSlideIn,
    CriticalHitPunch,
    ReloadCompletedDing,
    WeaponSwapWhoosh,
    PickupGlow,
}

impl JuiceKind {
    /// Canonical snake_case identifier used by the replay payload.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            JuiceKind::ButtonHover => "button_hover",
            JuiceKind::ClickPunch => "click_punch",
            JuiceKind::BannerSlideIn => "banner_slide_in",
            JuiceKind::CriticalHitPunch => "critical_hit_punch",
            JuiceKind::ReloadCompletedDing => "reload_completed_ding",
            JuiceKind::WeaponSwapWhoosh => "weapon_swap_whoosh",
            JuiceKind::PickupGlow => "pickup_glow",
        }
    }

    /// Canonical full animation duration (ms) before accessibility scaling.
    /// Per spec § Juice rules.
    #[must_use]
    pub fn duration_ms(self) -> u32 {
        match self {
            JuiceKind::ButtonHover => 80,
            JuiceKind::ClickPunch => 120,
            JuiceKind::BannerSlideIn => 200,
            JuiceKind::CriticalHitPunch => 180,
            JuiceKind::ReloadCompletedDing => 250,
            JuiceKind::WeaponSwapWhoosh => 220,
            JuiceKind::PickupGlow => 600,
        }
    }

    /// True when this rule is gated by `reduce_motion` (scale / slide
    /// animations collapse to end-state).
    #[must_use]
    pub fn is_motion_gated(self) -> bool {
        matches!(
            self,
            JuiceKind::ButtonHover
                | JuiceKind::ClickPunch
                | JuiceKind::BannerSlideIn
                | JuiceKind::WeaponSwapWhoosh
                | JuiceKind::PickupGlow
        )
    }

    /// True when this rule is gated by `reduce_flash` (screen flash +
    /// chromatic aberration suppressed).
    #[must_use]
    pub fn is_flash_gated(self) -> bool {
        matches!(self, JuiceKind::CriticalHitPunch)
    }

    /// True when this rule is gated by `reduce_shake` (hit-stop shake
    /// amplitude zeroed).
    #[must_use]
    pub fn is_shake_gated(self) -> bool {
        matches!(self, JuiceKind::CriticalHitPunch)
    }

    /// Parse from the snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(value: &str) -> Option<JuiceKind> {
        Some(match value {
            "button_hover" => JuiceKind::ButtonHover,
            "click_punch" => JuiceKind::ClickPunch,
            "banner_slide_in" => JuiceKind::BannerSlideIn,
            "critical_hit_punch" => JuiceKind::CriticalHitPunch,
            "reload_completed_ding" => JuiceKind::ReloadCompletedDing,
            "weapon_swap_whoosh" => JuiceKind::WeaponSwapWhoosh,
            "pickup_glow" => JuiceKind::PickupGlow,
            _ => return None,
        })
    }
}

/// **M12**: M12 accessibility flags mirror. cf-app writes this each
/// frame from the live `cf-control::Settings` snapshot. Default = all
/// flags off (full juice).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct JuiceAccessibility {
    pub reduce_motion: bool,
    pub reduce_shake: bool,
    pub reduce_flash: bool,
}

/// **M12**: one canonical scale-curve sample at `t` in `[0, 1]` for the
/// given rule. Per spec:
///
/// - `ButtonHover`: ease-out from 1.0 → 1.05.
/// - `ClickPunch`: 1.0 → 0.95 → 1.0 piecewise.
/// - `BannerSlideIn`: 1.0 (no scale; slide is handled by [`slide_offset`]).
/// - Other rules: 1.0 (no scale by default).
#[must_use]
pub fn scale_at(rule: JuiceKind, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match rule {
        JuiceKind::ButtonHover => {
            let eased = 1.0 - (1.0 - t) * (1.0 - t);
            1.0 + 0.05 * eased
        }
        JuiceKind::ClickPunch => {
            if t < 0.5 {
                1.0 - 0.05 * (t / 0.5)
            } else {
                0.95 + 0.05 * ((t - 0.5) / 0.5)
            }
        }
        JuiceKind::PickupGlow => {
            // Pulse — sin-shaped 1.0 → 1.04 → 1.0.
            let pulse = (t * std::f32::consts::PI).sin();
            1.0 + 0.04 * pulse
        }
        _ => 1.0,
    }
}

/// **M12**: ease-in-out curve sample for the banner slide (spec § Banner
/// slide-in: "slide from right edge over 200ms ease-in-out").
///
/// Returns the normalized offset from the off-screen anchor to the on-screen
/// rest position: `0.0` means fully off-screen, `1.0` means fully settled.
#[must_use]
pub fn slide_offset(rule: JuiceKind, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if rule != JuiceKind::BannerSlideIn {
        return 1.0;
    }
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// **M12** § Button hover juice — "glow halo appears". Returns the
/// halo alpha [0..1] over the lifetime of a hover pulse. The curve
/// rises sharply (matching the 80 ms ease-out scale) and stays solid
/// until the pulse expires.
#[must_use]
pub fn glow_halo_alpha(rule: JuiceKind, t: f32) -> f32 {
    if rule != JuiceKind::ButtonHover && rule != JuiceKind::PickupGlow {
        return 0.0;
    }
    let t = t.clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    eased.clamp(0.0, 1.0)
}

/// **M12** § Critical-hit punch juice — "screen flash". Returns the
/// screen-flash alpha [0..1]. Spikes at `t=0` (instant flash) and
/// decays linearly. Suppressed when `reduce_flash=true` (the JuicePulse
/// suppression flag already handles that — this function returns 0 when
/// the pulse is suppressed).
#[must_use]
pub fn screen_flash_alpha(pulse: &JuicePulse) -> f32 {
    if pulse.accessibility_suppressed {
        return 0.0;
    }
    if pulse.kind != JuiceKind::CriticalHitPunch {
        return 0.0;
    }
    let t = pulse.progress().clamp(0.0, 1.0);
    (1.0 - t).powi(2)
}

/// **M12** § Critical-hit punch juice — "chromatic aberration". Returns
/// the chromatic-aberration amplitude in arbitrary screen-space units
/// (0.0 = none, ~6.0 = strong split). Suppressed when `reduce_flash=true`.
#[must_use]
pub fn chromatic_aberration_amplitude(pulse: &JuicePulse) -> f32 {
    if pulse.accessibility_suppressed {
        return 0.0;
    }
    if pulse.kind != JuiceKind::CriticalHitPunch {
        return 0.0;
    }
    let t = pulse.progress().clamp(0.0, 1.0);
    6.0 * (1.0 - t)
}

/// **M12** § Weapon swap whoosh — "brief light streak". Returns the
/// streak intensity [0..1]. Suppressed when `reduce_motion=true`.
#[must_use]
pub fn weapon_swap_streak_intensity(pulse: &JuicePulse) -> f32 {
    if pulse.accessibility_suppressed {
        return 0.0;
    }
    if pulse.kind != JuiceKind::WeaponSwapWhoosh {
        return 0.0;
    }
    let t = pulse.progress().clamp(0.0, 1.0);
    if t < 0.4 {
        t / 0.4
    } else {
        ((1.0 - t) / 0.6).max(0.0)
    }
}

/// **M12**: live state for a juice pulse the renderer is currently driving.
/// Owned by the [`JuiceState`] resource; cf-app pushes new pulses via
/// [`JuiceState::push`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JuicePulse {
    pub kind: JuiceKind,
    /// Remaining duration in ms (decays per frame).
    pub remaining_ms: u32,
    /// True when reduce_motion / reduce_shake / reduce_flash short-circuited
    /// the animation (the pulse still ticks for one frame so consumers can
    /// register the event without rendering motion).
    pub accessibility_suppressed: bool,
}

impl JuicePulse {
    /// Construct a fresh pulse for the given rule, honoring the supplied
    /// accessibility flags. When the rule is suppressed, the returned pulse
    /// has `remaining_ms = 1` so it surfaces as a one-frame event but does
    /// not animate.
    #[must_use]
    pub fn new(kind: JuiceKind, acc: JuiceAccessibility) -> Self {
        let suppressed = (kind.is_motion_gated() && acc.reduce_motion)
            || (kind.is_flash_gated() && acc.reduce_flash)
            || (kind.is_shake_gated() && acc.reduce_shake);
        let remaining_ms = if suppressed { 1 } else { kind.duration_ms() };
        Self {
            kind,
            remaining_ms,
            accessibility_suppressed: suppressed,
        }
    }

    /// Progress fraction in `[0, 1]`; `0.0` = just started, `1.0` = ended.
    #[must_use]
    pub fn progress(&self) -> f32 {
        let total = self.kind.duration_ms().max(1) as f32;
        let elapsed = total - self.remaining_ms as f32;
        (elapsed / total).clamp(0.0, 1.0)
    }
}

/// **M12**: collected juice pulses currently driving HUD animation.
/// cf-app pushes new pulses, the [`tick_juice`] system decays them per
/// frame, and the HUD consumers read [`JuiceState::scale_for`] /
/// [`JuiceState::slide_for`] to derive visual state.
#[derive(Resource, Debug, Default, Clone)]
pub struct JuiceState {
    pulses: Vec<(String, JuicePulse)>,
}

impl JuiceState {
    /// Push a new pulse targeting the given HUD node id.
    pub fn push(&mut self, target_node: impl Into<String>, pulse: JuicePulse) {
        let node = target_node.into();
        // Replace any existing pulse on the same node + kind so repeated
        // hover events don't pile up.
        if let Some(existing) = self
            .pulses
            .iter_mut()
            .find(|(n, p)| *n == node && p.kind == pulse.kind)
        {
            *existing = (node, pulse);
        } else {
            self.pulses.push((node, pulse));
        }
    }

    /// Active pulses on `node` (one per kind).
    pub fn pulses_for<'a>(&'a self, node: &'a str) -> impl Iterator<Item = &'a JuicePulse> + 'a {
        self.pulses.iter().filter_map(move |(n, p)| (n == node).then_some(p))
    }

    /// Composite scale (product across active scale-driving pulses) on
    /// `node`. `1.0` when nothing animates.
    #[must_use]
    pub fn scale_for(&self, node: &str) -> f32 {
        let mut scale = 1.0_f32;
        for p in self.pulses_for(node) {
            if p.accessibility_suppressed {
                continue;
            }
            scale *= scale_at(p.kind, p.progress());
        }
        scale
    }

    /// **M12**: composite glow halo alpha on `node` (max across active
    /// glow-emitting pulses). Used by the renderer to overlay a soft
    /// halo around hovered buttons / picked-up items.
    #[must_use]
    pub fn glow_halo_for(&self, node: &str) -> f32 {
        let mut alpha = 0.0_f32;
        for p in self.pulses_for(node) {
            if p.accessibility_suppressed {
                continue;
            }
            alpha = alpha.max(glow_halo_alpha(p.kind, p.progress()));
        }
        alpha.clamp(0.0, 1.0)
    }

    /// **M12**: peak screen-flash alpha across every active
    /// `CriticalHitPunch` pulse (typically pinned to the global
    /// `ux.critical_hit` node). Used by the post-process pass to drive
    /// the cosmetic flash + chromatic aberration.
    #[must_use]
    pub fn screen_flash(&self) -> f32 {
        let mut peak = 0.0_f32;
        for (_, p) in self.pulses.iter() {
            peak = peak.max(screen_flash_alpha(p));
        }
        peak
    }

    /// **M12**: peak chromatic-aberration amplitude across active
    /// `CriticalHitPunch` pulses. Same nodeless lookup as
    /// [`Self::screen_flash`].
    #[must_use]
    pub fn chromatic_aberration(&self) -> f32 {
        let mut peak = 0.0_f32;
        for (_, p) in self.pulses.iter() {
            peak = peak.max(chromatic_aberration_amplitude(p));
        }
        peak
    }

    /// Slide offset (banner only). Returns `1.0` when no slide is active
    /// (banner rests on-screen).
    #[must_use]
    pub fn slide_for(&self, node: &str) -> f32 {
        for p in self.pulses_for(node) {
            if p.kind == JuiceKind::BannerSlideIn {
                if p.accessibility_suppressed {
                    return 1.0;
                }
                return slide_offset(JuiceKind::BannerSlideIn, p.progress());
            }
        }
        1.0
    }

    /// Drop expired pulses + decay timers. cf-app calls this per frame via
    /// [`tick_juice`].
    pub fn tick(&mut self, dt_ms: u32) {
        for (_node, pulse) in self.pulses.iter_mut() {
            pulse.remaining_ms = pulse.remaining_ms.saturating_sub(dt_ms);
        }
        self.pulses.retain(|(_, p)| p.remaining_ms > 0);
    }

    /// Number of currently-active pulses.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pulses.len()
    }

    /// **M12** § cf-app integration — iterate every (node, pulse) pair.
    /// cf-app's audio dispatcher uses this to emit one `AudioCue::Juice`
    /// per newly-fired pulse.
    pub fn for_each_active_pulse<F: FnMut(&str, &JuicePulse)>(&self, mut f: F) {
        for (node, pulse) in self.pulses.iter() {
            f(node.as_str(), pulse);
        }
    }

    /// Whether any pulses are active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pulses.is_empty()
    }
}

/// Bevy system that decays juice pulses every frame.
pub fn tick_juice(time: Res<Time>, mut state: ResMut<JuiceState>) {
    let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
    if dt_ms == 0 {
        return;
    }
    state.tick(dt_ms);
}

/// **M12**: juice plugin wiring `JuiceState` + `JuiceAccessibility`.
pub struct JuicePlugin;

impl Plugin for JuicePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JuiceState>()
            .init_resource::<JuiceAccessibility>()
            .add_systems(Update, tick_juice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips_through_str() {
        for k in [
            JuiceKind::ButtonHover,
            JuiceKind::ClickPunch,
            JuiceKind::BannerSlideIn,
            JuiceKind::CriticalHitPunch,
            JuiceKind::ReloadCompletedDing,
            JuiceKind::WeaponSwapWhoosh,
            JuiceKind::PickupGlow,
        ] {
            let s = k.as_str();
            assert_eq!(JuiceKind::from_str(s), Some(k), "round-trip failed for {s}");
        }
        assert_eq!(JuiceKind::from_str("nonsense"), None);
    }

    #[test]
    fn durations_match_spec() {
        assert_eq!(JuiceKind::ButtonHover.duration_ms(), 80);
        assert_eq!(JuiceKind::ClickPunch.duration_ms(), 120);
        assert_eq!(JuiceKind::BannerSlideIn.duration_ms(), 200);
    }

    #[test]
    fn button_hover_eases_to_one_oh_five() {
        let s_start = scale_at(JuiceKind::ButtonHover, 0.0);
        let s_end = scale_at(JuiceKind::ButtonHover, 1.0);
        assert!((s_start - 1.0).abs() < 1e-4);
        assert!((s_end - 1.05).abs() < 1e-4);
    }

    #[test]
    fn click_punch_dips_to_zero_point_nine_five_then_returns() {
        let s_start = scale_at(JuiceKind::ClickPunch, 0.0);
        let s_mid = scale_at(JuiceKind::ClickPunch, 0.5);
        let s_end = scale_at(JuiceKind::ClickPunch, 1.0);
        assert!((s_start - 1.0).abs() < 1e-4);
        assert!((s_mid - 0.95).abs() < 1e-4);
        assert!((s_end - 1.0).abs() < 1e-4);
    }

    #[test]
    fn banner_slide_in_eases() {
        let off = slide_offset(JuiceKind::BannerSlideIn, 0.0);
        let mid = slide_offset(JuiceKind::BannerSlideIn, 0.5);
        let on = slide_offset(JuiceKind::BannerSlideIn, 1.0);
        assert!(off < 0.01);
        assert!(mid > 0.4 && mid < 0.6);
        assert!(on > 0.99);
    }

    #[test]
    fn slide_offset_off_for_non_slide_rules() {
        assert!((slide_offset(JuiceKind::ButtonHover, 0.5) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn reduce_motion_suppresses_motion_rules() {
        let acc = JuiceAccessibility {
            reduce_motion: true,
            ..Default::default()
        };
        let p = JuicePulse::new(JuiceKind::ButtonHover, acc);
        assert!(p.accessibility_suppressed);
        // Pulse still surfaces as a one-frame event.
        assert_eq!(p.remaining_ms, 1);
    }

    #[test]
    fn reduce_flash_suppresses_only_critical_hit() {
        let acc = JuiceAccessibility {
            reduce_flash: true,
            ..Default::default()
        };
        let crit = JuicePulse::new(JuiceKind::CriticalHitPunch, acc);
        assert!(crit.accessibility_suppressed);
        let hover = JuicePulse::new(JuiceKind::ButtonHover, acc);
        assert!(!hover.accessibility_suppressed);
    }

    #[test]
    fn reduce_shake_suppresses_only_critical_hit_shake() {
        let acc = JuiceAccessibility {
            reduce_shake: true,
            ..Default::default()
        };
        let crit = JuicePulse::new(JuiceKind::CriticalHitPunch, acc);
        assert!(crit.accessibility_suppressed);
    }

    #[test]
    fn juice_state_tracks_pulses_per_node() {
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility::default();
        s.push(
            "menu.new_game",
            JuicePulse::new(JuiceKind::ButtonHover, acc),
        );
        s.push("menu.continue", JuicePulse::new(JuiceKind::ButtonHover, acc));
        assert_eq!(s.len(), 2);
        let scale_a = s.scale_for("menu.new_game");
        assert!(scale_a >= 1.0);
        let scale_b = s.scale_for("menu.unknown");
        assert!((scale_b - 1.0).abs() < 1e-4);
    }

    #[test]
    fn repeated_pulses_on_same_node_and_kind_replace_each_other() {
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility::default();
        s.push("menu.foo", JuicePulse::new(JuiceKind::ButtonHover, acc));
        s.push("menu.foo", JuicePulse::new(JuiceKind::ButtonHover, acc));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn pulses_decay_and_expire() {
        // ButtonHover has 80 ms duration. We tick 30 ms three times so the
        // pulse hits 0 ms on the third tick and gets retired by `retain`.
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility::default();
        s.push("menu.foo", JuicePulse::new(JuiceKind::ButtonHover, acc));
        assert_eq!(s.len(), 1);
        s.tick(30);
        s.tick(30);
        assert_eq!(s.len(), 1, "pulse should still be active at 60/80 ms");
        s.tick(30);
        assert_eq!(s.len(), 0, "pulse should expire after total 90/80 ms");
    }

    #[test]
    fn slide_for_returns_one_when_no_slide_active() {
        let s = JuiceState::default();
        assert!((s.slide_for("banner.alert") - 1.0).abs() < 1e-4);
    }

    #[test]
    fn slide_for_returns_one_when_suppressed() {
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility {
            reduce_motion: true,
            ..Default::default()
        };
        s.push(
            "banner.alert",
            JuicePulse::new(JuiceKind::BannerSlideIn, acc),
        );
        assert!((s.slide_for("banner.alert") - 1.0).abs() < 1e-4);
    }

    #[test]
    fn motion_gating_does_not_apply_to_critical_hit() {
        // CriticalHit is gated by flash/shake, NOT motion — the screen
        // flash/chromatic aberration suppression is the accessibility lever.
        let acc = JuiceAccessibility {
            reduce_motion: true,
            ..Default::default()
        };
        let crit = JuicePulse::new(JuiceKind::CriticalHitPunch, acc);
        assert!(!crit.accessibility_suppressed);
    }

    #[test]
    fn glow_halo_rises_on_button_hover() {
        let start = glow_halo_alpha(JuiceKind::ButtonHover, 0.0);
        let end = glow_halo_alpha(JuiceKind::ButtonHover, 1.0);
        assert!(start < 0.01);
        assert!(end > 0.99);
    }

    #[test]
    fn glow_halo_only_fires_for_hover_and_pickup() {
        for kind in [
            JuiceKind::ClickPunch,
            JuiceKind::BannerSlideIn,
            JuiceKind::CriticalHitPunch,
            JuiceKind::ReloadCompletedDing,
            JuiceKind::WeaponSwapWhoosh,
        ] {
            assert!(glow_halo_alpha(kind, 0.5).abs() < 1e-4, "{kind:?} should not emit glow halo");
        }
        assert!(glow_halo_alpha(JuiceKind::ButtonHover, 0.5) > 0.0);
        assert!(glow_halo_alpha(JuiceKind::PickupGlow, 0.5) > 0.0);
    }

    #[test]
    fn screen_flash_peaks_at_start_of_critical_hit() {
        let mut pulse = JuicePulse::new(JuiceKind::CriticalHitPunch, JuiceAccessibility::default());
        let peak = screen_flash_alpha(&pulse);
        assert!(peak > 0.99, "should flash hard at start");
        pulse.remaining_ms = 1;
        let near_end = screen_flash_alpha(&pulse);
        assert!(near_end < 0.01, "should fade by end");
    }

    #[test]
    fn screen_flash_zero_when_suppressed() {
        let acc = JuiceAccessibility {
            reduce_flash: true,
            ..Default::default()
        };
        let pulse = JuicePulse::new(JuiceKind::CriticalHitPunch, acc);
        assert!((screen_flash_alpha(&pulse)).abs() < 1e-4);
        assert!((chromatic_aberration_amplitude(&pulse)).abs() < 1e-4);
    }

    #[test]
    fn chromatic_aberration_only_fires_for_critical_hit() {
        let acc = JuiceAccessibility::default();
        let pulse = JuicePulse::new(JuiceKind::ButtonHover, acc);
        assert!((chromatic_aberration_amplitude(&pulse)).abs() < 1e-4);
        let crit = JuicePulse::new(JuiceKind::CriticalHitPunch, acc);
        assert!(chromatic_aberration_amplitude(&crit) > 0.0);
    }

    #[test]
    fn weapon_swap_streak_peaks_around_40_percent() {
        let acc = JuiceAccessibility::default();
        let mut pulse = JuicePulse::new(JuiceKind::WeaponSwapWhoosh, acc);
        pulse.remaining_ms = pulse.kind.duration_ms() * 6 / 10;
        let intensity = weapon_swap_streak_intensity(&pulse);
        assert!(intensity > 0.9, "intensity at 40% = {intensity}");
    }

    #[test]
    fn juice_state_screen_flash_aggregates_critical_hits() {
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility::default();
        s.push("ux.critical_hit", JuicePulse::new(JuiceKind::CriticalHitPunch, acc));
        assert!(s.screen_flash() > 0.9);
        assert!(s.chromatic_aberration() > 5.0);
    }

    #[test]
    fn juice_state_glow_halo_returns_max_across_pulses() {
        let mut s = JuiceState::default();
        let acc = JuiceAccessibility::default();
        s.push("menu.new_game", JuicePulse::new(JuiceKind::ButtonHover, acc));
        let alpha = s.glow_halo_for("menu.new_game");
        assert!(alpha >= 0.0 && alpha <= 1.0);
    }
}
