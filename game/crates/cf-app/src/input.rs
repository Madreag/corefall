use std::collections::{HashMap, HashSet};

use bevy::{
    app::AppExit,
    input::{
        gamepad::Gamepad,
        keyboard::KeyCode,
    },
    prelude::*,
    window::{WindowCloseRequested, WindowFocused},
};

use cf_actor::IntentSource;
use cf_control::{server::ControlCommand, EngineHandle};

use crate::app::resources::{
    ControlRuntime, EngineHolder, LocalInputEnabled, QuicksaveLoopResource,
};
use crate::gamepad_focus::gamepad_focus_direction;
pub(crate) use crate::hold_tracker::HoldTracker;

/// M4A canonical action ids that drive `ingest_player_input`. Stable strings
/// shared with `Settings.key_bindings` so the cfctl + observe surface can
/// remap them by name. `fire_alt` mirrors `fire` (Enter + KeyJ are both fire
/// keys by default; remapping replaces both with the configured KeyCode).
pub const ACTION_JUMP: &str = "jump";
pub const ACTION_FIRE: &str = "fire";
pub const ACTION_FIRE_ALT: &str = "fire_alt";
pub const ACTION_RELOAD: &str = "reload";
pub const ACTION_DIG: &str = "dig";
pub const ACTION_RESET: &str = "reset";
pub const ACTION_SELECT_SLOT_0: &str = "select_slot_0";
pub const ACTION_SELECT_SLOT_1: &str = "select_slot_1";
pub const ACTION_SELECT_SLOT_2: &str = "select_slot_2";
pub const ACTION_SELECT_SLOT_3: &str = "select_slot_3";
pub const ACTION_MOVE_LEFT: &str = "move_left";
pub const ACTION_MOVE_RIGHT: &str = "move_right";
pub const ACTION_MOVE_UP: &str = "move_up";
pub const ACTION_MOVE_DOWN: &str = "move_down";
pub const ACTION_AIM_LEFT: &str = "aim_left";
pub const ACTION_AIM_RIGHT: &str = "aim_right";
pub const ACTION_AIM_UP: &str = "aim_up";
pub const ACTION_AIM_DOWN: &str = "aim_down";

/// M4A: parse a KeyCode by stable name. Returns `None` only for defensive
/// in-memory fallbacks; live settings patches validate names before dispatch.
pub(crate) fn parse_key_code(name: &str) -> Option<KeyCode> {
    match name {
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Tab" => Some(KeyCode::Tab),
        "Escape" => Some(KeyCode::Escape),
        "Backspace" => Some(KeyCode::Backspace),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        "Digit0" => Some(KeyCode::Digit0),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "Digit6" => Some(KeyCode::Digit6),
        "Digit7" => Some(KeyCode::Digit7),
        "Digit8" => Some(KeyCode::Digit8),
        "Digit9" => Some(KeyCode::Digit9),
        "Numpad0" => Some(KeyCode::Numpad0),
        "Numpad1" => Some(KeyCode::Numpad1),
        "Numpad2" => Some(KeyCode::Numpad2),
        "Numpad3" => Some(KeyCode::Numpad3),
        "Numpad4" => Some(KeyCode::Numpad4),
        "Numpad5" => Some(KeyCode::Numpad5),
        "Numpad6" => Some(KeyCode::Numpad6),
        "Numpad7" => Some(KeyCode::Numpad7),
        "Numpad8" => Some(KeyCode::Numpad8),
        "Numpad9" => Some(KeyCode::Numpad9),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        _ => None,
    }
}

/// M4A: resolve the active KeyCode for an action by reading
/// `Settings.key_bindings` (when `key_remap_enabled = true`) and falling
/// back to the hard-coded default only as a defensive in-memory fallback.
/// Live cfctl/JSON-RPC patches validate action + key names before they enter
/// Settings, so unsupported remaps reject instead of silently succeeding.
pub(crate) fn key_for_action(settings: &cf_control::Settings, action: &str) -> Option<KeyCode> {
    if settings.key_remap_enabled {
        if let Some(name) = settings.key_bindings.get(action) {
            if let Some(k) = parse_key_code(name) {
                return Some(k);
            }
            tracing::warn!(target: "cf::app", action = %action, binding = %name, "unknown key binding name; falling back to default");
        }
    }
    match action {
        ACTION_JUMP => Some(KeyCode::Space),
        ACTION_FIRE => Some(KeyCode::Enter),
        ACTION_FIRE_ALT => Some(KeyCode::KeyJ),
        ACTION_RELOAD => Some(KeyCode::KeyR),
        ACTION_DIG => Some(KeyCode::KeyG),
        ACTION_RESET => Some(KeyCode::KeyL),
        ACTION_SELECT_SLOT_0 => Some(KeyCode::Digit1),
        ACTION_SELECT_SLOT_1 => Some(KeyCode::Digit2),
        ACTION_SELECT_SLOT_2 => Some(KeyCode::Digit3),
        ACTION_SELECT_SLOT_3 => Some(KeyCode::Digit4),
        ACTION_MOVE_LEFT => Some(KeyCode::KeyA),
        ACTION_MOVE_RIGHT => Some(KeyCode::KeyD),
        ACTION_MOVE_UP => Some(KeyCode::KeyW),
        ACTION_MOVE_DOWN => Some(KeyCode::KeyS),
        ACTION_AIM_LEFT => Some(KeyCode::ArrowLeft),
        ACTION_AIM_RIGHT => Some(KeyCode::ArrowRight),
        ACTION_AIM_UP => Some(KeyCode::ArrowUp),
        ACTION_AIM_DOWN => Some(KeyCode::ArrowDown),
        _ if action == cf_shell::keybinds::ACTION_QUICKSAVE => Some(KeyCode::F5),
        _ if action == cf_shell::keybinds::ACTION_QUICKLOAD => Some(KeyCode::F9),
        _ => None,
    }
}

pub(crate) fn focus_owns_keyboard_key(key: KeyCode, focus_active: bool) -> bool {
    focus_active && matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown)
}

pub(crate) fn gameplay_key_pressed(keys: &ButtonInput<KeyCode>, key: KeyCode, focus_active: bool) -> bool {
    keys.pressed(key) && !focus_owns_keyboard_key(key, focus_active)
}

pub(crate) fn gameplay_key_just_released(keys: &ButtonInput<KeyCode>, key: KeyCode, focus_active: bool) -> bool {
    keys.just_released(key) && !focus_owns_keyboard_key(key, focus_active)
}

/// Sample the keyboard each frame and fold it into the engine's pending
/// `ControlIntent` so human input runs through exactly the same path as
/// `cfctl act.player.*` commands. Movement is continuous (held keys); jump /
/// fire / reload / select are edge-triggered.
pub(crate) fn ingest_player_input(
    holder: Res<EngineHolder>,
    keys: Res<ButtonInput<KeyCode>>,
    rt: Option<Res<ControlRuntime>>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut hold_tracker: ResMut<HoldTracker>,
    mut last_move_x: Local<f32>,
    mut last_aim: Local<(f32, f32)>,
    mut last_intent_epoch: Local<u64>,
) {
    let _ = rt;
    if !local_input_enabled.0 {
        return;
    }
    if !holder.0.config().has_actor_world {
        return;
    }
    let settings = holder.0.current_settings();
    let focus_active = holder.0.hud_caches_snapshot().focused_node.is_some();
    let key_or = |action: &str, fallback: KeyCode| key_for_action(&settings, action).unwrap_or(fallback);
    let move_left = key_or(ACTION_MOVE_LEFT, KeyCode::KeyA);
    let move_right = key_or(ACTION_MOVE_RIGHT, KeyCode::KeyD);
    let move_up = key_or(ACTION_MOVE_UP, KeyCode::KeyW);
    let move_down = key_or(ACTION_MOVE_DOWN, KeyCode::KeyS);
    let aim_left = key_or(ACTION_AIM_LEFT, KeyCode::ArrowLeft);
    let aim_right = key_or(ACTION_AIM_RIGHT, KeyCode::ArrowRight);
    let aim_up = key_or(ACTION_AIM_UP, KeyCode::ArrowUp);
    let aim_down = key_or(ACTION_AIM_DOWN, KeyCode::ArrowDown);
    let move_x = keyboard_axis_pair_gameplay(&keys, move_right, move_left, focus_active);
    let aim_x = keyboard_axis_pair_gameplay(&keys, aim_right, aim_left, focus_active);
    let aim_y = keyboard_axis_gameplay(&keys, move_up, move_down, aim_up, aim_down, focus_active);
    let engine_epoch = holder.0.intent_epoch();
    let epoch_changed = engine_epoch != *last_intent_epoch;
    if epoch_changed {
        *last_intent_epoch = engine_epoch;
        *last_move_x = 0.0;
        *last_aim = (0.0, 0.0);
    }
    let dispatch_move = (move_x - *last_move_x).abs() > f32::EPSILON || (epoch_changed && move_x.abs() > 1e-3);
    let aim_active = aim_x.abs() > 1e-3 || aim_y.abs() > 1e-3;
    let aim_changed = (aim_x - last_aim.0).abs() > f32::EPSILON || (aim_y - last_aim.1).abs() > f32::EPSILON;
    let dispatch_aim = aim_active && (aim_changed || epoch_changed);
    if dispatch_move {
        *last_move_x = move_x;
    }
    if dispatch_aim {
        *last_aim = (aim_x, aim_y);
    } else if !aim_active {
        *last_aim = (0.0, 0.0);
    }
    let block_on = futures_block_on;
    block_on(async {
        if dispatch_move {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerMove {
                    x: move_x,
                    y: 0.0,
                    source: IntentSource::Human,
                })
                .await;
        }
        if dispatch_aim {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerAim {
                    x: aim_x,
                    y: aim_y,
                    source: IntentSource::Human,
                })
                .await;
        }
        let live_settings = holder.0.current_settings();
        let mut pressed: HashSet<String> = HashSet::new();
        for action in [
            ACTION_JUMP,
            ACTION_FIRE,
            ACTION_FIRE_ALT,
            ACTION_RELOAD,
            ACTION_DIG,
            ACTION_RESET,
            ACTION_SELECT_SLOT_0,
            ACTION_SELECT_SLOT_1,
            ACTION_SELECT_SLOT_2,
            ACTION_SELECT_SLOT_3,
        ] {
            if let Some(k) = key_for_action(&live_settings, action) {
                if gameplay_key_pressed(&keys, k, focus_active) {
                    pressed.insert(action.to_string());
                }
            }
        }
        let now = std::time::Instant::now();
        let threshold = std::time::Duration::from_millis(u64::from(live_settings.hold_threshold_ms));
        let fired = hold_tracker.tick_with_state(&pressed, live_settings.hold_to_confirm, threshold, now);
        if fired.contains(ACTION_JUMP) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_FIRE) || fired.contains(ACTION_FIRE_ALT) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                ammo_kind: None,
                    source: IntentSource::Human,
                })
                .await;
        }
        let fire_primary = key_for_action(&live_settings, ACTION_FIRE);
        let fire_alt = key_for_action(&live_settings, ACTION_FIRE_ALT);
        let fire_released = fire_primary
            .map(|k| gameplay_key_just_released(&keys, k, focus_active))
            .unwrap_or(false)
            || fire_alt
                .map(|k| gameplay_key_just_released(&keys, k, focus_active))
                .unwrap_or(false);
        if fire_released {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                ammo_kind: None,
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_RELOAD) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_RESET) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_DIG) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerDig {
                    target: None,
                    source: IntentSource::Human,
                })
                .await;
        }
        for (action_id, slot) in [
            (ACTION_SELECT_SLOT_0, 0u32),
            (ACTION_SELECT_SLOT_1, 1u32),
            (ACTION_SELECT_SLOT_2, 2u32),
            (ACTION_SELECT_SLOT_3, 3u32),
        ] {
            if fired.contains(action_id) {
                let _ = holder
                    .0
                    .dispatch(ControlCommand::ActPlayerSelectItem {
                        slot,
                        source: IntentSource::Human,
                    })
                    .await;
            }
        }
    });
}

pub(crate) fn keyboard_axis_gameplay(
    keys: &ButtonInput<KeyCode>,
    pos_a: KeyCode,
    neg_a: KeyCode,
    pos_b: KeyCode,
    neg_b: KeyCode,
    focus_active: bool,
) -> f32 {
    let pos = gameplay_key_pressed(keys, pos_a, focus_active) || gameplay_key_pressed(keys, pos_b, focus_active);
    let neg = gameplay_key_pressed(keys, neg_a, focus_active) || gameplay_key_pressed(keys, neg_b, focus_active);
    axis_from_pressed(pos, neg)
}

pub(crate) fn axis_from_pressed(pos: bool, neg: bool) -> f32 {
    match (pos, neg) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

pub(crate) fn keyboard_axis_pair_gameplay(
    keys: &ButtonInput<KeyCode>,
    pos: KeyCode,
    neg: KeyCode,
    focus_active: bool,
) -> f32 {
    axis_from_pressed(
        gameplay_key_pressed(keys, pos, focus_active),
        gameplay_key_pressed(keys, neg, focus_active),
    )
}

/// Block on a single async dispatch. The control engine is used through async traits
/// even from the synchronous Bevy schedule; the body is small and all work is
/// in-process so blocking is fine.
///
/// Uses a thread-parking waker so that if any future implementation ever returns
/// `Poll::Pending` (for example, a future engine backed by `tokio::sync::RwLock`),
/// the current thread parks until the waker is signalled instead of spinning.
pub(crate) fn futures_block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};
    use std::thread::{self, Thread};

    struct ThreadWaker(Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Arc::new(ThreadWaker(thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => thread::park(),
        }
    }
}

/// **M4B § "F5 / F9 hotkeys"** — quicksave / quickload + autosave timer.
/// The deterministic state machine lives in [`crate::quicksave`]; this
/// system is the Bevy adapter.
pub(crate) fn ingest_quicksave_input(
    holder: Res<EngineHolder>,
    keys: Res<ButtonInput<KeyCode>>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut loop_state: ResMut<QuicksaveLoopResource>,
) {
    if !local_input_enabled.0 {
        return;
    }
    let engine = holder.0.clone();
    let current_tick = engine.current_tick().0;
    let settings = engine.current_settings();
    let quicksave_key =
        key_for_action(&settings, cf_shell::keybinds::ACTION_QUICKSAVE).unwrap_or(KeyCode::F5);
    let quickload_key =
        key_for_action(&settings, cf_shell::keybinds::ACTION_QUICKLOAD).unwrap_or(KeyCode::F9);
    let f5 = keys.just_pressed(quicksave_key);
    let f9 = keys.just_pressed(quickload_key);
    let action = crate::quicksave::next_action(
        &loop_state.0,
        f5,
        f9,
        current_tick,
        engine.config().tick_rate_hz,
    );
    let dir = engine.config().run_bundle_root.join("../saves/quicksave");
    let dir = if dir.is_absolute() {
        dir
    } else {
        std::env::current_dir().unwrap_or_default().join(dir)
    };
    match action {
        crate::quicksave::QuicksaveAction::None => {}
        crate::quicksave::QuicksaveAction::Quicksave => match engine.quicksave(&dir) {
            Ok(outcome) => {
                loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::SaveOk {
                    path: outcome.path.display().to_string(),
                    wall_clock_ms: outcome.wall_clock_ms,
                });
            }
            Err(err) => {
                tracing::warn!(target: "cf::app::quicksave", ?err, "quicksave failed");
                loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::from_save_error(&err));
            }
        },
        crate::quicksave::QuicksaveAction::Quickload => match engine.quickload(&dir) {
            Ok(outcome) => {
                let migrated_from = outcome.migrated_from.map(|v| v.as_string());
                let migrated_to = outcome.migrated_to.map(|v| v.as_string());
                loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::LoadOk {
                    path: dir.join(cf_save::quicksave::QUICKSAVE_FILE).display().to_string(),
                    wall_clock_ms: outcome.wall_clock_ms,
                    migrated_from,
                    migrated_to,
                });
                if loop_state.0.last_outcome.as_ref().and_then(|o| o.migration_banner()).is_some() {
                    loop_state.0.migration_banner_shown = true;
                }
            }
            Err(err) => {
                tracing::warn!(target: "cf::app::quicksave", ?err, "quickload failed");
                loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::from_save_error(&err));
            }
        },
        crate::quicksave::QuicksaveAction::Autosave => {
            match engine.autosave(&dir) {
                Ok(outcome) => {
                    crate::quicksave::record_autosave(&mut loop_state.0, current_tick);
                    loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::SaveOk {
                        path: outcome.path.display().to_string(),
                        wall_clock_ms: outcome.wall_clock_ms,
                    });
                }
                Err(err) => {
                    tracing::warn!(target: "cf::app::quicksave", ?err, "autosave failed");
                    loop_state.0.last_outcome = Some(crate::quicksave::QuicksaveOutcomeUi::from_save_error(&err));
                }
            }
        }
    }
}

/// M4A keyboard + controller focus traversal.
///
/// All routes dispatch through `act.input.focus` so the cfctl, cf-e2e,
/// keyboard, and gamepad consumers share the same code path.
pub(crate) fn ingest_focus_input(
    holder: Res<EngineHolder>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut last_stick_y: Local<HashMap<Entity, f32>>,
) {
    if !local_input_enabled.0 {
        return;
    }
    let block_on = futures_block_on;
    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let dispatch_focus = |direction: cf_control::server::FocusDirection| {
        block_on(async {
            let _ = holder
                .0
                .dispatch(cf_control::ControlCommand::ActInputFocus {
                    direction,
                    source: IntentSource::Human,
                })
                .await;
        });
    };

    let focus_active = holder.0.hud_caches_snapshot().focused_node.is_some();
    let keyboard_dir = keyboard_focus_direction(
        keys.just_pressed(KeyCode::Tab),
        shift_held,
        keys.just_pressed(KeyCode::ArrowDown),
        keys.just_pressed(KeyCode::ArrowUp),
        keys.just_pressed(KeyCode::F1),
        focus_active,
    );
    let sent_keyboard = keyboard_dir.is_some();
    if let Some(direction) = keyboard_dir {
        dispatch_focus(direction);
    }

    if sent_keyboard {
        return;
    }

    let stick_threshold = 0.5_f32;
    for (entity, gp) in gamepads.iter() {
        let prev_y = last_stick_y.entry(entity).or_insert(0.0);
        if let Some(dir) = gamepad_focus_direction(gp, prev_y, stick_threshold) {
            dispatch_focus(dir);
            return;
        }
    }
}

pub(crate) fn keyboard_focus_direction(
    tab_pressed: bool,
    shift_held: bool,
    arrow_down_pressed: bool,
    arrow_up_pressed: bool,
    f1_pressed: bool,
    focus_active: bool,
) -> Option<cf_control::server::FocusDirection> {
    use cf_control::server::FocusDirection;
    if tab_pressed {
        return Some(if shift_held {
            FocusDirection::Prev
        } else {
            FocusDirection::Next
        });
    }
    if focus_active && arrow_down_pressed {
        return Some(FocusDirection::Next);
    }
    if focus_active && arrow_up_pressed {
        return Some(FocusDirection::Prev);
    }
    if f1_pressed {
        return Some(FocusDirection::Clear);
    }
    None
}

/// DR-012 ACC-A-04 contract: Escape clears HUD focus when a focus ring is
/// active; only when there is NO focused node does Escape exit the app.
pub(crate) fn esc_or_close_to_exit(
    keys: Res<ButtonInput<KeyCode>>,
    holder: Res<EngineHolder>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut close_events: MessageReader<WindowCloseRequested>,
    mut events: MessageWriter<AppExit>,
) {
    if local_input_enabled.0 && keys.just_pressed(KeyCode::Escape) {
        let focused = holder.0.hud_caches_snapshot().focused_node;
        if focused.is_some() {
            futures_block_on(async {
                let _ = holder
                    .0
                    .dispatch(cf_control::ControlCommand::ActInputFocus {
                        direction: cf_control::server::FocusDirection::Clear,
                        source: IntentSource::Human,
                    })
                    .await;
            });
            tracing::info!(target: "cf::app", "ESC pressed; cleared HUD focus (was {:?})", focused);
        } else {
            tracing::info!(target: "cf::app", "ESC pressed; no HUD focus active; exiting");
            events.write(AppExit::Success);
        }
    }
    if close_events.read().next().is_some() {
        tracing::info!(target: "cf::app", "window close requested; exiting");
        events.write(AppExit::Success);
    }
}

/// **M1 / Gap D4**: react to window-focus events by toggling the engine's
/// `controls_captured_by` flag.
pub(crate) fn handle_window_focus_capture(
    holder: Res<EngineHolder>,
    mut focus_events: MessageReader<WindowFocused>,
) {
    for ev in focus_events.read() {
        let captured = !ev.focused;
        let label = if captured { "window_blur" } else { "" };
        let engine = holder.0.clone();
        let label = label.to_string();
        futures_block_on(async move {
            let _ = engine
                .dispatch(cf_control::ControlCommand::ActInputCaptureControls {
                    captured,
                    capturer: if captured { Some(label) } else { None },
                    source: IntentSource::Human,
                })
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::gamepad::{GamepadAxis, GamepadButton, GamepadInput};
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    fn pressed_set(actions: &[&str]) -> HashSet<String> {
        actions.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn tap_fires_on_first_pressed_frame_then_stays_silent() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let fired = t.tick_with_state(&pressed_set(&["jump"]), false, Duration::from_millis(250), now);
        assert!(fired.contains("jump"));
        let fired_next = t.tick_with_state(
            &pressed_set(&["jump"]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(16),
        );
        assert!(!fired_next.contains("jump"), "tap should fire only once per hold");
    }

    #[test]
    fn tap_fires_again_after_release_then_press() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["fire"]), false, Duration::from_millis(250), now);
        let _ = t.tick_with_state(
            &pressed_set(&[]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(16),
        );
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(32),
        );
        assert!(fired.contains("fire"), "post-release press fires again");
    }

    #[test]
    fn hold_does_not_fire_before_threshold() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let fired = t.tick_with_state(&pressed_set(&["jump"]), true, Duration::from_millis(250), now);
        assert!(!fired.contains("jump"), "hold mode must NOT fire on tap");
        let fired = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(100),
        );
        assert!(!fired.contains("jump"), "still below threshold");
    }

    #[test]
    fn hold_fires_once_at_threshold() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["jump"]), true, Duration::from_millis(250), now);
        let fired = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(260),
        );
        assert!(fired.contains("jump"), "fires exactly when threshold reached");
        let fired_next = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(500),
        );
        assert!(!fired_next.contains("jump"), "fires at most once per hold");
    }

    #[test]
    fn hold_release_before_threshold_cancels() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["fire"]), true, Duration::from_millis(250), now);
        let fired = t.tick_with_state(
            &pressed_set(&[]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(100),
        );
        assert!(!fired.contains("fire"), "no fire if released before threshold");
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(120),
        );
        assert!(!fired.contains("fire"), "new hold session, threshold not reached");
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(380),
        );
        assert!(fired.contains("fire"), "second hold completes after a fresh threshold");
    }

    #[test]
    fn key_for_action_honors_remap_when_enabled() {
        use std::collections::BTreeMap;
        let baseline = cf_control::Settings::default();
        assert_eq!(
            key_for_action(&baseline, ACTION_FIRE),
            Some(KeyCode::Enter),
            "default fire is Enter"
        );
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "KeyF".to_string());
        bindings.insert("jump".to_string(), "ShiftLeft".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_FIRE), Some(KeyCode::KeyF));
        assert_eq!(key_for_action(&s, ACTION_JUMP), Some(KeyCode::ShiftLeft));
        assert_eq!(key_for_action(&s, ACTION_RELOAD), Some(KeyCode::KeyR));
    }

    #[test]
    fn key_for_action_ignores_remap_when_disabled() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "KeyF".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: false,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(
            key_for_action(&s, ACTION_FIRE),
            Some(KeyCode::Enter),
            "remap ignored when disabled"
        );
    }

    #[test]
    fn key_for_action_warns_on_unknown_binding_name_and_falls_back() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "BogusKey".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_FIRE), Some(KeyCode::Enter));
    }

    #[test]
    fn keyboard_focus_tab_enters_focus_mode_without_arrow_stealing() {
        use cf_control::server::FocusDirection;
        assert!(matches!(
            keyboard_focus_direction(true, false, false, false, false, false),
            Some(FocusDirection::Next)
        ));
        assert!(matches!(
            keyboard_focus_direction(true, true, false, false, false, false),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn keyboard_focus_arrows_only_navigate_after_focus_is_active() {
        use cf_control::server::FocusDirection;
        assert!(
            keyboard_focus_direction(false, false, true, false, false, false).is_none(),
            "ArrowDown must remain aim-only before Tab enters focus mode"
        );
        assert!(
            keyboard_focus_direction(false, false, false, true, false, false).is_none(),
            "ArrowUp must remain aim-only before Tab enters focus mode"
        );
        assert!(matches!(
            keyboard_focus_direction(false, false, true, false, false, true),
            Some(FocusDirection::Next)
        ));
        assert!(matches!(
            keyboard_focus_direction(false, false, false, true, false, true),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn focus_mode_owns_arrow_keys_but_not_remapped_aim_keys() {
        let mut arrows = ButtonInput::<KeyCode>::default();
        arrows.press(KeyCode::ArrowDown);
        assert_eq!(
            keyboard_axis_gameplay(
                &arrows,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                false,
            ),
            -1.0,
            "without active focus ArrowDown remains default aim-down"
        );
        assert_eq!(
            keyboard_axis_gameplay(
                &arrows,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                true,
            ),
            0.0,
            "with active focus ArrowDown belongs to HUD traversal"
        );
        assert!(!gameplay_key_pressed(&arrows, KeyCode::ArrowDown, true));

        let mut remapped = ButtonInput::<KeyCode>::default();
        remapped.press(KeyCode::Numpad2);
        assert_eq!(
            keyboard_axis_gameplay(
                &remapped,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::Numpad8,
                KeyCode::Numpad2,
                true,
            ),
            -1.0,
            "focus mode only owns the physical arrow keys, not a remapped numpad aim key"
        );
    }

    fn make_gamepad_with_press(button: GamepadButton) -> Gamepad {
        let mut gp = Gamepad::default();
        gp.digital_mut().press(button);
        gp
    }

    fn make_gamepad_with_axis(axis: GamepadAxis, value: f32) -> Gamepad {
        let mut gp = Gamepad::default();
        gp.analog_mut().set(GamepadInput::Axis(axis), value);
        gp
    }

    #[test]
    fn gamepad_focus_input_dpad_down_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadDown);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_up_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadUp);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_right_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadRight);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_left_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadLeft);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_east_button_clears_focus() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::East);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Clear)
        ));
    }

    #[test]
    fn gamepad_focus_input_south_button_is_reserved_for_activation_no_focus_dispatch() {
        let gp = make_gamepad_with_press(GamepadButton::South);
        let mut prev_y = 0.0;
        assert!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none(),
            "South must be a no-op for focus traversal"
        );
    }

    #[test]
    fn gamepad_focus_input_left_bumper_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::LeftTrigger);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_bumper_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::RightTrigger);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_stick_down_rising_edge_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
        assert!((prev_y - (-0.8)).abs() < 1e-6);
    }

    #[test]
    fn gamepad_focus_input_right_stick_up_rising_edge_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, 0.9);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_stick_held_only_fires_on_rising_edge() {
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let mut prev_y = -0.7;
        assert!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none(),
            "stick already past threshold last frame; should not refire"
        );
    }

    #[test]
    fn gamepad_focus_input_right_stick_below_threshold_does_not_fire() {
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.3);
        let mut prev_y = 0.0;
        assert!(gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none());
    }

    #[test]
    fn gamepad_focus_input_no_button_no_axis_returns_none() {
        let gp = Gamepad::default();
        let mut prev_y = 0.0;
        assert!(gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none());
    }

    #[test]
    fn gamepad_focus_input_per_gamepad_debounce_isolates_idle_pad_from_active_pad() {
        use cf_control::server::FocusDirection;

        let pad_a = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let pad_b = Gamepad::default();
        let mut history_a = 0.0_f32;
        let mut history_b = 0.0_f32;

        let dir_a = gamepad_focus_direction(&pad_a, &mut history_a, 0.5);
        assert!(matches!(dir_a, Some(FocusDirection::Next)));
        let dir_b = gamepad_focus_direction(&pad_b, &mut history_b, 0.5);
        assert!(dir_b.is_none(), "idle pad B must not fire");
        assert!((history_a - (-0.8)).abs() < 1e-6);
        assert!(history_b.abs() < 1e-6, "pad B history untouched by pad A");

        let dir_a2 = gamepad_focus_direction(&pad_a, &mut history_a, 0.5);
        assert!(
            dir_a2.is_none(),
            "pad A held stick must not refire — per-pad history preserved"
        );
        let dir_b2 = gamepad_focus_direction(&pad_b, &mut history_b, 0.5);
        assert!(dir_b2.is_none());
    }

    #[test]
    fn settings_default_key_bindings_includes_movement_and_aim_actions() {
        let bindings = cf_control::default_key_bindings();
        assert_eq!(bindings.get("move_left").map(String::as_str), Some("KeyA"));
        assert_eq!(bindings.get("move_right").map(String::as_str), Some("KeyD"));
        assert_eq!(bindings.get("move_up").map(String::as_str), Some("KeyW"));
        assert_eq!(bindings.get("move_down").map(String::as_str), Some("KeyS"));
        assert_eq!(bindings.get("aim_left").map(String::as_str), Some("ArrowLeft"));
        assert_eq!(bindings.get("aim_right").map(String::as_str), Some("ArrowRight"));
        assert_eq!(bindings.get("aim_up").map(String::as_str), Some("ArrowUp"));
        assert_eq!(bindings.get("aim_down").map(String::as_str), Some("ArrowDown"));
    }

    #[test]
    fn key_for_action_honors_movement_remap_when_enabled() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("move_left".into(), "KeyH".into());
        bindings.insert("move_right".into(), "KeyL".into());
        bindings.insert("aim_up".into(), "Numpad8".into());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_MOVE_LEFT), Some(KeyCode::KeyH));
        assert_eq!(key_for_action(&s, ACTION_MOVE_RIGHT), Some(KeyCode::KeyL));
        assert_eq!(key_for_action(&s, ACTION_AIM_UP), Some(KeyCode::Numpad8));
        assert_eq!(key_for_action(&s, ACTION_MOVE_UP), Some(KeyCode::KeyW));
    }
}
