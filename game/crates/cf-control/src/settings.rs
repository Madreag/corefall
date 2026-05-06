//! M0 accessibility/settings surface (DR-012 lock).
//!
//! Six flags exactly per the canonical CLI Reference. No UI behavior in M0;
//! flags are observable via `cfctl observe --settings --once` and recorded in
//! `run_manifest.json.settings`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Settings {
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
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
        ] {
            assert!(v.get(k).is_some(), "missing key: {k}");
        }
    }
}
