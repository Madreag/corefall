use serde::{Deserialize, Serialize};

use crate::{ActorId, ItemSlot, Vec2};

/// Source of a `ControlIntent` for replay/audit.
///
/// `{Player, Ai, Replay, Script}`. The original implementation diverged
/// into `{Human, Cfctl, Ai, Replay}`. We accept both the spec names AND
/// the legacy names on the input side via serde aliases, but emit only
/// the spec-canonical names on the output side via `rename`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    #[serde(rename = "player", alias = "human")]
    Human,
    #[serde(rename = "script", alias = "cfctl")]
    Cfctl,
    Ai,
    Replay,
}

impl IntentSource {
    pub fn spec_canonical_name(self) -> &'static str {
        match self {
            IntentSource::Human => "player",
            IntentSource::Cfctl => "script",
            IntentSource::Ai => "ai",
            IntentSource::Replay => "replay",
        }
    }
}

impl Default for IntentSource {
    fn default() -> Self {
        IntentSource::Human
    }
}

/// One tick's worth of player input. Produced by `cf-control` and applied by
/// [`crate::ActorWorld::tick`]. Sticky vs. edge-triggered semantics matter:
///
/// - `move_x`, `aim`: continuous (latest value wins).
/// - `jump`, `fire`, `reload`, `selected_item`, `reset`: edge-triggered (true only on
///   the tick the button was pressed; cleared by the engine after consumption).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ControlIntent {
    pub actor: ActorId,
    pub source: IntentSource,
    pub move_x: f32,
    pub jump: bool,
    pub aim: Vec2,
    pub fire: bool,
    pub reload: bool,
    pub selected_item: Option<ItemSlot>,
    pub reset: bool,
    #[serde(default)]
    pub interact: bool,
    #[serde(default)]
    pub use_tool: bool,
    #[serde(default)]
    pub crouch: bool,
    #[serde(default)]
    pub prone: bool,
    #[serde(default)]
    pub sharp_aim: bool,
    #[serde(default)]
    pub fire_held: bool,
    #[serde(default)]
    pub ammo_kind: Option<cf_equipment::RoundKind>,
}

impl ControlIntent {
    pub fn new(actor: ActorId, source: IntentSource) -> Self {
        Self {
            actor,
            source,
            ..Self::default()
        }
    }

    pub fn clear_edges(&mut self) {
        self.jump = false;
        self.fire = false;
        self.reload = false;
        self.selected_item = None;
        self.reset = false;
        self.interact = false;
        self.use_tool = false;
        self.crouch = false;
        self.prone = false;
        self.ammo_kind = None;
    }

    /// Returns true when no actively-driven input is present. `aim` is
    /// continuous and persists across ticks; sticky aim direction does not
    /// indicate the player is actively providing input. `sharp_aim` is also
    /// sticky/continuous; it is not treated as active input pressure.
    pub fn is_idle(&self) -> bool {
        self.move_x.abs() < f32::EPSILON
            && !self.jump
            && !self.fire
            && !self.reload
            && self.selected_item.is_none()
            && !self.reset
            && !self.interact
            && !self.use_tool
            && !self.crouch
            && !self.prone
    }
}
