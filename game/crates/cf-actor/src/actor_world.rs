use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{quantize_f32, ActorId, ActorState};

/// The set of [`ActorState`]s in a scenario, plus the player actor id (if any).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorWorld {
    pub actors: BTreeMap<ActorId, ActorState>,
    pub player: Option<ActorId>,
    /// Y coordinate of the world floor (sand-table simplification for M1 — the M2 chunked
    /// terrain replaces this).
    pub floor_y: f32,
    /// Gravity in world units / s² (negative = pulls toward floor). Defaults to scenario's
    /// `gravity`. Sim systems apply this each tick.
    pub gravity: f32,
    /// **M1.5 G8**: when true, lethal damage to controllable actors caps
    /// at `Status::Dying` — the DYING dwell does NOT promote to DEAD so a
    /// tutorial player can finish their first session without restarting.
    /// Per DR-023 onboarding policy; sourced from the scenario manifest's
    /// `tutorial_safety` flag.
    #[serde(default)]
    pub tutorial_safety: bool,
}

impl ActorWorld {
    pub fn new(floor_y: f32, gravity: f32) -> Self {
        Self {
            actors: BTreeMap::new(),
            player: None,
            floor_y,
            gravity,
            tutorial_safety: false,
        }
    }

    pub fn insert(&mut self, actor: ActorState) {
        if actor.controllable && self.player.is_none() {
            self.player = Some(actor.id);
        }
        self.actors.insert(actor.id, actor);
    }

    pub fn player_actor(&self) -> Option<&ActorState> {
        self.player.and_then(|id| self.actors.get(&id))
    }

    pub fn player_actor_mut(&mut self) -> Option<&mut ActorState> {
        let id = self.player?;
        self.actors.get_mut(&id)
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.actors.len() * 96 + 16);
        out.extend_from_slice(&quantize_f32(self.floor_y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.gravity).to_le_bytes());
        for (_, actor) in &self.actors {
            out.extend_from_slice(&actor.checksum_bytes());
        }
        out
    }
}
