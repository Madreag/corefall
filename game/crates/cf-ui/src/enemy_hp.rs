//! M2 — Enemy HP HUD widget (shown when guard in LOS).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/enemy_hp.rs` is the
//! canonical home for the ENEMY_HP zone. The current implementation lives
//! in `cf-ui/src/lib.rs::enemy_line`; this module re-exports the function
//! so consumers that import per the spec path `cf_ui::enemy_hp::*` resolve
//! cleanly.

pub use crate::{enemy_line, HudEnemy};
