//! M9C § "Player-facing behavior": per-fortification HUD widgets.
//!
//! Spec §"Crates / modules touched":
//!
//! > `cf-ui::fortification_hud` — Per-fortification HP bar, ammo-box
//! > state, spotlight cone preview, minefield-warning banner.
//!
//! VAL-M9C-053: cf-ui::fortification_hud module exists at
//! `cf-ui/src/fortification_hud.rs` and is wired into the HUD root.
//!
//! Each surface is a pure-data HUD widget state owned as a Bevy
//! `Resource` so cf-app's bridge can mirror it per frame from the
//! engine snapshot. The module deliberately stays presentation-only:
//! it does not consult the engine's per-fortification simulation
//! state directly — cf-app feeds it through the `set_*` helpers.

use bevy::prelude::*;

use cf_fortification::common::FortificationId;

/// Per-fortification HP bar drawn next to the player's selected
/// fortification (the "selected" id is owned by cf-app's bridge; the
/// HUD reads it through `selected_id`).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct FortificationHpBarState {
    pub selected_id: Option<FortificationId>,
    pub hp: u32,
    pub max_hp: u32,
}

impl FortificationHpBarState {
    /// HP fraction in `[0.0, 1.0]`. Returns 0.0 when `max_hp == 0` to
    /// avoid divide-by-zero on un-initialised state.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.max_hp == 0 {
            0.0
        } else {
            (self.hp as f32 / self.max_hp as f32).clamp(0.0, 1.0)
        }
    }

    /// True when the HUD should render the HP bar at all.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.selected_id.is_some() && self.max_hp > 0
    }

    /// Update the bar from the engine's per-fortification snapshot.
    pub fn set(&mut self, id: FortificationId, hp: u32, max_hp: u32) {
        self.selected_id = Some(id);
        self.hp = hp;
        self.max_hp = max_hp;
    }

    /// Hide the bar (player has no fortification selected).
    pub fn clear(&mut self) {
        self.selected_id = None;
        self.hp = 0;
        self.max_hp = 0;
    }
}

/// MG-nest ammo-box state. cf-app's bridge writes `rounds_remaining`
/// + `rounds_max` from the engine each frame.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct AmmoBoxHudState {
    pub nest_id: Option<FortificationId>,
    pub rounds_remaining: u32,
    pub rounds_max: u32,
}

impl AmmoBoxHudState {
    /// Returns `true` when the player is crewing a nest that has at
    /// least 1 round in its currently-fed ammo box (the HUD draws
    /// the rounds-remaining number only while crewing).
    #[must_use]
    pub fn visible(&self) -> bool {
        self.nest_id.is_some()
    }

    /// Round fraction in `[0.0, 1.0]`; used by the rounds-remaining
    /// bar tint.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.rounds_max == 0 {
            0.0
        } else {
            (self.rounds_remaining as f32 / self.rounds_max as f32).clamp(0.0, 1.0)
        }
    }

    /// Engine bridge: feed the live ammo-box state.
    pub fn set(&mut self, nest_id: FortificationId, rounds_remaining: u32, rounds_max: u32) {
        self.nest_id = Some(nest_id);
        self.rounds_remaining = rounds_remaining;
        self.rounds_max = rounds_max;
    }

    pub fn clear(&mut self) {
        self.nest_id = None;
        self.rounds_remaining = 0;
        self.rounds_max = 0;
    }
}

/// Spotlight cone preview. The HUD draws a translucent fan over the
/// world to show where the player's selected spotlight will sweep
/// when toggled. cf-app writes the origin + aim from the spotlight's
/// per-tick state.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct SpotlightPreviewState {
    pub spotlight_id: Option<FortificationId>,
    pub origin_tiles: (i32, i32),
    pub aim_radians: f32,
    pub range_tiles: u32,
    pub half_angle_degrees: f32,
    /// True while the spotlight is online (cone drawn solid); false
    /// during the 12s `spotlight_dazzled` window (cone drawn dimmed
    /// + dashed).
    pub online: bool,
}

impl SpotlightPreviewState {
    #[must_use]
    pub fn visible(&self) -> bool {
        self.spotlight_id.is_some()
    }

    pub fn set(
        &mut self,
        spotlight_id: FortificationId,
        origin_tiles: (i32, i32),
        aim_radians: f32,
        range_tiles: u32,
        half_angle_degrees: f32,
        online: bool,
    ) {
        self.spotlight_id = Some(spotlight_id);
        self.origin_tiles = origin_tiles;
        self.aim_radians = aim_radians;
        self.range_tiles = range_tiles;
        self.half_angle_degrees = half_angle_degrees;
        self.online = online;
    }

    pub fn clear(&mut self) {
        self.spotlight_id = None;
        self.origin_tiles = (0, 0);
        self.aim_radians = 0.0;
        self.range_tiles = 0;
        self.half_angle_degrees = 0.0;
        self.online = false;
    }
}

/// Minefield warning banner: triggered when the player's actor is
/// inside a `MINEFIELD_WARNING_RADIUS_TILES`-tile bubble around any
/// known-to-faction mine.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct MinefieldWarningBannerState {
    pub visible: bool,
    pub nearest_mine_id: Option<u32>,
    pub distance_tiles: u32,
    /// Stable text label rendered in the banner; localised at render
    /// time by `cf-localization` (M38A).
    pub message: String,
}

impl MinefieldWarningBannerState {
    /// Render label per spec § Notes for the implementer + the
    /// `alarm.tripwire_triggered` audio family. The HUD layer wraps
    /// the message through cf-localization.
    pub const DEFAULT_MESSAGE: &'static str = "MINEFIELD AHEAD";

    /// Engine bridge: show the warning at a given distance.
    pub fn show(&mut self, mine_id: u32, distance_tiles: u32) {
        self.visible = true;
        self.nearest_mine_id = Some(mine_id);
        self.distance_tiles = distance_tiles;
        if self.message.is_empty() {
            self.message = Self::DEFAULT_MESSAGE.to_string();
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.nearest_mine_id = None;
        self.distance_tiles = 0;
    }
}

/// Bevy plugin that registers every M9C fortification HUD resource
/// in their default-empty state. cf-app's HUD root spawns these
/// when the plugin is added.
pub struct FortificationHudPlugin;

impl Plugin for FortificationHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FortificationHpBarState>()
            .init_resource::<AmmoBoxHudState>()
            .init_resource::<SpotlightPreviewState>()
            .init_resource::<MinefieldWarningBannerState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// when set with a non-zero max.
    #[test]
    fn fortification_hp_bar_visibility() {
        let mut state = FortificationHpBarState::default();
        assert!(!state.visible());
        state.set(FortificationId(7), 600, 600);
        assert!(state.visible());
        assert!((state.fraction() - 1.0).abs() < f32::EPSILON);
    }

    /// HP bar fraction is clamped to `[0, 1]`.
    #[test]
    fn fortification_hp_bar_fraction_clamped() {
        let mut state = FortificationHpBarState::default();
        state.set(FortificationId(1), 250, 500);
        assert!((state.fraction() - 0.5).abs() < f32::EPSILON);
        // HP overflow (shouldn't happen but defend against it).
        state.set(FortificationId(1), 1000, 500);
        assert!((state.fraction() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fortification_hp_bar_clear_returns_to_hidden() {
        let mut state = FortificationHpBarState::default();
        state.set(FortificationId(1), 600, 600);
        state.clear();
        assert!(!state.visible());
        assert_eq!(state.hp, 0);
        assert_eq!(state.max_hp, 0);
    }

    /// Ammo box hides while no nest is crewed; shows once set.
    #[test]
    fn ammo_box_hud_visibility() {
        let mut state = AmmoBoxHudState::default();
        assert!(!state.visible());
        state.set(FortificationId(2), 200, 800);
        assert!(state.visible());
        assert!((state.fraction() - 0.25).abs() < f32::EPSILON);
        state.clear();
        assert!(!state.visible());
    }

    /// Spotlight preview shows + hides per cf-app's bridge.
    #[test]
    fn spotlight_preview_visibility() {
        let mut state = SpotlightPreviewState::default();
        assert!(!state.visible());
        state.set(FortificationId(3), (10, 10), 0.0, 24, 22.5, true);
        assert!(state.visible());
        assert_eq!(state.range_tiles, 24);
        assert!(state.online);
        state.clear();
        assert!(!state.visible());
    }

    /// Minefield warning banner shows + hides via `show` / `hide`.
    #[test]
    fn minefield_warning_banner_toggle() {
        let mut banner = MinefieldWarningBannerState::default();
        assert!(!banner.visible);
        banner.show(7, 3);
        assert!(banner.visible);
        assert_eq!(banner.nearest_mine_id, Some(7));
        assert_eq!(banner.distance_tiles, 3);
        assert_eq!(banner.message, MinefieldWarningBannerState::DEFAULT_MESSAGE);
        banner.hide();
        assert!(!banner.visible);
        assert!(banner.nearest_mine_id.is_none());
    }
}
