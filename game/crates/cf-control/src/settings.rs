//! Accessibility/settings surface.
//!
//! M0 locked the original 6 flags (`ui_scale`, `high_contrast`, `captions`,
//! `reduced_motion`, `reduced_shake`, `reduced_flash`) per DR-012; M4A added
//! `hold_to_confirm` + `hold_threshold_ms` + `key_remap_enabled` so the
//! ACC-A-05 "remap and holds" surface contract is testable end-to-end.
//! Flags are observable via `cfctl observe --settings --once` and recorded
//! in `run_manifest.json.settings`. `act.settings.set` round-trips them
//! live; cf-app + cf-ui mirror them every frame.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SUPPORTED_KEY_BINDING_ACTIONS: &[&str] = &[
    "jump",
    "fire",
    "fire_alt",
    "reload",
    "dig",
    "reset",
    "select_slot_0",
    "select_slot_1",
    "select_slot_2",
    "select_slot_3",
    "move_left",
    "move_right",
    "move_up",
    "move_down",
    "aim_left",
    "aim_right",
    "aim_up",
    "aim_down",
];

pub const SUPPORTED_KEY_CODE_NAMES: &[&str] = &[
    "Space",
    "Enter",
    "Tab",
    "Escape",
    "Backspace",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Settings {
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
    /// M4A: hold-to-press alternative for tap-to-press actions (ACC-A-05).
    /// When `true`, edge-triggered actions (jump / fire / reload / dig /
    /// reset / select_slot_*) require holding the key/button for
    /// `hold_threshold_ms` before firing instead of triggering on the press
    /// edge. Default: off (tap semantics). cf-app's `ingest_player_input`
    /// honors this through the `HoldTracker` resource; cfctl-driven
    /// dispatches bypass the hold gate (control-plane is already explicit
    /// press/release semantics).
    #[serde(default)]
    pub hold_to_confirm: bool,
    /// M4A: hold threshold in milliseconds. Default 250 ms; clamped to
    /// `[50..2000]` by `apply_settings_patch`.
    #[serde(default = "default_hold_threshold_ms")]
    pub hold_threshold_ms: u32,
    /// M4A: when `true`, cf-app's keyboard layer reads its action bindings
    /// from `key_bindings` instead of the built-in defaults. When `false`,
    /// the built-in defaults apply (Space=jump, Enter|J=fire, R=reload,
    /// G=dig, L=reset, 1..4=select_slot_0..3). The remap UI editor lands
    /// at M8; M4A ships the cfctl-settable surface so the contract is
    /// behavior-testable today.
    #[serde(default)]
    pub key_remap_enabled: bool,
    /// M4A: per-action key binding overrides. Action names are limited to
    /// [`SUPPORTED_KEY_BINDING_ACTIONS`]; KeyCode names are limited to
    /// [`SUPPORTED_KEY_CODE_NAMES`]. Empty by default; `act.settings.set`
    /// replaces the table only after validation so unsupported remaps reject
    /// at the control boundary instead of silently falling back in cf-app.
    #[serde(default)]
    pub key_bindings: BTreeMap<String, String>,
}

fn default_hold_threshold_ms() -> u32 {
    250
}

/// ACC-A UI scale floor. Values entering `Settings` through `act.settings.set`
/// are clamped to this bound so `observe.settings`, `observe.accessibility`,
/// and cf-ui render state all report the same applied scale.
pub const UI_SCALE_MIN: f32 = 0.5;
/// ACC-A UI scale ceiling. See [`UI_SCALE_MIN`].
pub const UI_SCALE_MAX: f32 = 4.0;

/// M4A: built-in default action → KeyCode bindings. Action names are stable
/// strings the cfctl + replay surface refers to; values are KeyCode variant
/// names (e.g. `Space`, `Enter`, `KeyJ`, `KeyR`). cf-app maps the names back
/// to `bevy::prelude::KeyCode` via cf-app's parser. Unknown names are rejected
/// by [`validate_key_bindings`] before they can enter live `Settings`.
///
/// Audit fix round-5 (2026-05-10): the remap surface now covers continuous
/// actions (move_left/move_right/move_up/move_down + aim_*) in addition to
/// discrete actions (jump/fire/reload/dig/reset/select_slot_N). `cf-app::
/// ingest_player_input` consults its `key_for_action` helper for movement
/// and aim every frame so left-handed users can swap WASD ↔ arrows or
/// rebind aim to numpad without code changes. Movement/aim are CONTINUOUS
/// (held-key → analog axis), so the remap honors `key.pressed(...)` per-
/// frame rather than the `just_pressed(...)` edge-trigger of discrete
/// actions.
pub fn default_key_bindings() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    // Discrete actions (edge-triggered)
    m.insert("jump".into(), "Space".into());
    m.insert("fire".into(), "Enter".into());
    m.insert("fire_alt".into(), "KeyJ".into());
    m.insert("reload".into(), "KeyR".into());
    m.insert("dig".into(), "KeyG".into());
    m.insert("reset".into(), "KeyL".into());
    m.insert("select_slot_0".into(), "Digit1".into());
    m.insert("select_slot_1".into(), "Digit2".into());
    m.insert("select_slot_2".into(), "Digit3".into());
    m.insert("select_slot_3".into(), "Digit4".into());
    // Continuous actions (held-key → analog) — primary WASD bindings
    m.insert("move_left".into(), "KeyA".into());
    m.insert("move_right".into(), "KeyD".into());
    m.insert("move_up".into(), "KeyW".into());
    m.insert("move_down".into(), "KeyS".into());
    // Continuous actions — aim with arrow keys (left-hand-friendly default)
    m.insert("aim_left".into(), "ArrowLeft".into());
    m.insert("aim_right".into(), "ArrowRight".into());
    m.insert("aim_up".into(), "ArrowUp".into());
    m.insert("aim_down".into(), "ArrowDown".into());
    m
}

pub fn is_supported_key_binding_action(action: &str) -> bool {
    SUPPORTED_KEY_BINDING_ACTIONS.contains(&action)
}

pub fn is_supported_key_code_name(key: &str) -> bool {
    SUPPORTED_KEY_CODE_NAMES.contains(&key)
}

pub fn validate_key_bindings(bindings: &BTreeMap<String, String>) -> Result<(), String> {
    for (action, key) in bindings {
        if !is_supported_key_binding_action(action) {
            return Err(format!("key_binding_unknown_action:{action}"));
        }
        if !is_supported_key_code_name(key) {
            return Err(format!("key_binding_unknown_key:{action}={key}"));
        }
    }
    let mut effective = default_key_bindings();
    for (action, key) in bindings {
        effective.insert(action.clone(), key.clone());
    }
    let mut key_owner: BTreeMap<String, String> = BTreeMap::new();
    for (action, key) in effective {
        if let Some(first_action) = key_owner.insert(key.clone(), action.clone()) {
            return Err(format!("key_binding_duplicate_key:{key}={first_action},{action}"));
        }
    }
    Ok(())
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            high_contrast: false,
            captions: true,
            reduced_motion: false,
            reduced_shake: false,
            reduced_flash: false,
            hold_to_confirm: false,
            hold_threshold_ms: default_hold_threshold_ms(),
            key_remap_enabled: false,
            key_bindings: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_dr012_lock() {
        let s = Settings::default();
        assert!((s.ui_scale - 1.0).abs() < f32::EPSILON);
        assert!(!s.high_contrast);
        assert!(s.captions, "captions default-on per DR-012 lean");
        assert!(!s.reduced_motion);
        assert!(!s.reduced_shake);
        assert!(!s.reduced_flash);
        assert!(!s.hold_to_confirm);
        assert_eq!(s.hold_threshold_ms, 250);
        assert!(!s.key_remap_enabled);
    }

    #[test]
    fn settings_serialize_to_flat_kv() {
        let s = Settings::default();
        let v = serde_json::to_value(&s).unwrap();
        for k in [
            "ui_scale",
            "high_contrast",
            "captions",
            "reduced_motion",
            "reduced_shake",
            "reduced_flash",
            "hold_to_confirm",
            "hold_threshold_ms",
            "key_remap_enabled",
            "key_bindings",
        ] {
            assert!(v.get(k).is_some(), "missing key: {k}");
        }
    }

    #[test]
    fn default_key_bindings_cover_every_m4a_action() {
        let b = default_key_bindings();
        for action in SUPPORTED_KEY_BINDING_ACTIONS {
            assert!(b.contains_key(*action), "missing default binding for {action}");
        }
        assert_eq!(b.len(), SUPPORTED_KEY_BINDING_ACTIONS.len());
    }

    #[test]
    fn supported_key_names_cover_default_bindings_and_numpad() {
        let b = default_key_bindings();
        for (action, key) in &b {
            assert!(
                is_supported_key_code_name(key),
                "default binding {action}={key} must be accepted by live validation"
            );
        }
        assert!(is_supported_key_code_name("Numpad8"));
    }

    #[test]
    fn validate_key_bindings_rejects_unknown_action() {
        let mut b = BTreeMap::new();
        b.insert("frie".to_string(), "KeyF".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_unknown_action:frie"
        );
    }

    #[test]
    fn validate_key_bindings_rejects_unknown_key() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "BogusKey".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_unknown_key:fire=BogusKey"
        );
    }

    #[test]
    fn validate_key_bindings_accepts_numpad_remap() {
        let mut b = BTreeMap::new();
        b.insert("aim_up".to_string(), "Numpad8".to_string());
        validate_key_bindings(&b).unwrap();
    }

    #[test]
    fn validate_key_bindings_rejects_collisions_with_default_bindings() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "KeyA".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_duplicate_key:KeyA=fire,move_left"
        );
    }

    #[test]
    fn validate_key_bindings_accepts_full_swap_without_collision() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "KeyA".to_string());
        b.insert("move_left".to_string(), "Enter".to_string());
        validate_key_bindings(&b).unwrap();
    }
}
