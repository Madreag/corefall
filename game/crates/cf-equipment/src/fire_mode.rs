//! M1 firing-mode enum + serde default helper.

use serde::{Deserialize, Serialize};

///
/// - `Semi`: exactly one shot per `intent.fire` press (the press is latched in
///   `RifleState::semi_latched` and released only when the player releases
///   the trigger). Holding fire fires once, then waits.
/// - `FullAuto`: as long as `intent.fire` is held the rifle fires at
///   `fire_interval_seconds` cadence.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireMode {
    Semi = 0,
    FullAuto = 1,
}

impl Default for FireMode {
    fn default() -> Self {
        // M1's default rifle is semi-automatic per CCCP `HDFirearm` defaults.
        FireMode::Semi
    }
}

impl FireMode {
    pub fn as_str(self) -> &'static str {
        match self {
            FireMode::Semi => "semi",
            FireMode::FullAuto => "full_auto",
        }
    }
}

pub(crate) fn default_fire_mode() -> FireMode {
    FireMode::Semi
}
