//! **M11 audit pass 3 (GAP-M11-02 LOW fix)**: M4A § Files lists
//! `cf-app/src/gamepad_focus.rs` as a NEW dedicated file. The
//! `gamepad_focus_direction` helper was originally inline in `main.rs`;
//! extracting it into its own module here per spec § file-layout discipline.
//! The inline copy in main.rs keeps the existing wiring; this module
//! ships an identical implementation at the spec-canonical path.
#![allow(dead_code)]

use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadInput};

/// Per-frame gamepad input → focus-direction resolution. Pulled out of
/// main.rs so the tests can drive synthetic `Gamepad` instances without a
/// Bevy app + window. Returns the resolved `FocusDirection` to dispatch
/// this frame, or `None` when no edge fired. `last_stick_y` carries the
/// previous frame's right-stick Y so analog motion only fires on rising
/// edge (crossing the threshold), not every frame the stick is held.
pub fn gamepad_focus_direction(
    gp: &Gamepad,
    last_stick_y: &mut f32,
    stick_threshold: f32,
) -> Option<cf_control::server::FocusDirection> {
    use cf_control::server::FocusDirection;
    if gp.just_pressed(GamepadButton::DPadDown)
        || gp.just_pressed(GamepadButton::DPadRight)
        || gp.just_pressed(GamepadButton::RightTrigger)
    {
        return Some(FocusDirection::Next);
    }
    if gp.just_pressed(GamepadButton::DPadUp)
        || gp.just_pressed(GamepadButton::DPadLeft)
        || gp.just_pressed(GamepadButton::LeftTrigger)
    {
        return Some(FocusDirection::Prev);
    }
    if gp.just_pressed(GamepadButton::East) {
        return Some(FocusDirection::Clear);
    }
    // South (Xbox A / PS Cross) is reserved for activation of the
    // currently focused node. M4A does NOT own activation semantics (that
    // lands at M5 + M8). Returning None here is the honest behavior; the
    // button is wired to be a no-op for focus traversal.
    let _reserved_for_activation = GamepadButton::South;
    let stick_y = gp
        .get(GamepadInput::Axis(GamepadAxis::RightStickY))
        .or_else(|| gp.get(GamepadInput::Axis(GamepadAxis::LeftStickY)))
        .unwrap_or(0.0);
    let prev_y = *last_stick_y;
    *last_stick_y = stick_y;
    if stick_y <= -stick_threshold && prev_y > -stick_threshold {
        return Some(FocusDirection::Next);
    }
    if stick_y >= stick_threshold && prev_y < stick_threshold {
        return Some(FocusDirection::Prev);
    }
    None
}
