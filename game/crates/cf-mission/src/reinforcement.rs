//! M7: Mission director v0.5 — reinforcement waves.
//!
//! Spec § Mission director v0.5 — reinforcements spawn based on phase +
//! player progress (kills); emit `mission.reinforcement_wave_spawned`.

use serde::{Deserialize, Serialize};

use crate::phases::MissionPhase;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinforcementWave {
    pub id: String,
    pub phase: MissionPhase,
    pub trigger_kill_count: u32,
    pub dropship_zone: [f32; 2],
    pub spawn_count: u32,
    pub spawned: bool,
}

impl ReinforcementWave {
    pub fn new(id: impl Into<String>, phase: MissionPhase, trigger_kill_count: u32, dropship_zone: [f32; 2]) -> Self {
        Self {
            id: id.into(),
            phase,
            trigger_kill_count,
            dropship_zone,
            spawn_count: 3,
            spawned: false,
        }
    }

    pub fn should_spawn(&self, current_phase: MissionPhase, kill_count: u32) -> bool {
        !self.spawned && current_phase == self.phase && kill_count >= self.trigger_kill_count
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinforcementWaveSpawnedEvent {
    pub wave_id: String,
    pub phase: MissionPhase,
    pub spawn_count: u32,
    pub dropship_zone: [f32; 2],
    pub tick: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReinforcementRegistry {
    pub waves: Vec<ReinforcementWave>,
}

impl ReinforcementRegistry {
    pub fn push(&mut self, wave: ReinforcementWave) {
        self.waves.push(wave);
    }

    /// Find the next wave that should spawn given the current phase +
    /// kill count. Marks it `spawned = true` and returns its event payload.
    pub fn try_spawn_next(
        &mut self,
        phase: MissionPhase,
        kill_count: u32,
        tick: u64,
    ) -> Option<ReinforcementWaveSpawnedEvent> {
        for w in self.waves.iter_mut() {
            if w.should_spawn(phase, kill_count) {
                w.spawned = true;
                return Some(ReinforcementWaveSpawnedEvent {
                    wave_id: w.id.clone(),
                    phase,
                    spawn_count: w.spawn_count,
                    dropship_zone: w.dropship_zone,
                    tick,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_triggers_when_phase_and_kill_count_match() {
        let mut reg = ReinforcementRegistry::default();
        reg.push(ReinforcementWave::new("alpha", MissionPhase::Buildup, 3, [100.0, 0.0]));
        assert!(reg.try_spawn_next(MissionPhase::Setup, 5, 100).is_none());
        let ev = reg.try_spawn_next(MissionPhase::Buildup, 3, 200);
        assert!(ev.is_some());
        // Re-spawn attempt is idempotent.
        assert!(reg.try_spawn_next(MissionPhase::Buildup, 10, 300).is_none());
    }
}
