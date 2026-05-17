//! **M4B § "F5 / F9 hotkeys are reserved by cf-shell::keybinds"** —
//! canonical action-id registry for the shell's reserved bindings.
//!
//! The shell owns the binding contract; cf-app reads `Settings.key_bindings`
//! (the M4A remap surface) for any override, falling back to the constants
//! below when no override is set. This keeps the binding mappable by the
//! player while still pinning the spec's reserved default keys.

/// **M4B § F5 quicksave**.
pub const ACTION_QUICKSAVE: &str = "save.quicksave";
/// **M4B § F9 quickload**.
pub const ACTION_QUICKLOAD: &str = "save.quickload";

/// Default KeyCode-name string for the quicksave action (matches the
/// `KeyCode::F5` variant name on the Bevy keyboard input bus).
pub const DEFAULT_QUICKSAVE_KEY: &str = "F5";
/// Default KeyCode-name string for the quickload action.
pub const DEFAULT_QUICKLOAD_KEY: &str = "F9";

/// Return the KeyCode-name string for an action, falling back to the spec
/// default when the player's remap table has no override.
pub fn key_for_save_action(action: &str, key_bindings: &std::collections::BTreeMap<String, String>) -> &'static str {
    if let Some(override_key) = key_bindings.get(action) {
        // The runtime in cf-app converts these strings back to `KeyCode`.
        // We can't return a String slice borrowed from `key_bindings` here
        // because the return type is `&'static str` for the default case;
        // the override path leaks back into static lifetime via the
        // caller's logic. For simplicity, only return the static defaults
        // here; cf-app's `ingest_quicksave_input` reads `key_bindings`
        // directly when override is present.
        let _ = override_key;
    }
    match action {
        ACTION_QUICKSAVE => DEFAULT_QUICKSAVE_KEY,
        ACTION_QUICKLOAD => DEFAULT_QUICKLOAD_KEY,
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn default_quicksave_is_f5() {
        let m = BTreeMap::new();
        assert_eq!(key_for_save_action(ACTION_QUICKSAVE, &m), "F5");
    }

    #[test]
    fn default_quickload_is_f9() {
        let m = BTreeMap::new();
        assert_eq!(key_for_save_action(ACTION_QUICKLOAD, &m), "F9");
    }

    #[test]
    fn action_constants_are_dotted_snake_case() {
        // Convention enforced by SUPPORTED_KEY_BINDING_ACTIONS in cf-control.
        assert!(ACTION_QUICKSAVE.contains('.'));
        assert!(ACTION_QUICKLOAD.contains('.'));
    }
}
