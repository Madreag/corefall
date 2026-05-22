//! M7: Mission director v0.5 — mini-boss multi-phase pattern.
//!
//! Spec § Mission director v0.5 — Mini-boss patterns. M7 ships ONE mini-boss
//! per scenario (Spotter as launch boss) with HP-threshold-driven phase
//! transitions (>75% phase 1; <75% phase 2 with shield; <25% enraged).
//!
//! The schema migrates to M25's canonical `BossDef` when that ships; M7
//! ships the pre-schema form per spec.

use serde::{Deserialize, Serialize};

/// `boss.phase_changed.phase` payload field.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BossPhase {
    Phase1 = 1,
    Phase2 = 2,
    Phase3 = 3,
}

impl BossPhase {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BossPhase::Phase1 => "phase_1",
            BossPhase::Phase2 => "phase_2",
            BossPhase::Phase3 => "phase_3",
        }
    }
}

/// emits `boss.phase_changed` on transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossState {
    pub actor_id: u64,
    pub display_name: String,
    pub current_phase: BossPhase,
    pub max_hp: f32,
    pub current_hp: f32,
    /// True once the boss is defeated.
    pub defeated: bool,
    /// HP fraction thresholds. Defaults: 0.75, 0.25 per spec.
    pub phase_2_hp_threshold: f32,
    pub phase_3_hp_threshold: f32,
}

impl BossState {
    pub fn new(actor_id: u64, display_name: impl Into<String>, max_hp: f32) -> Self {
        Self {
            actor_id,
            display_name: display_name.into(),
            current_phase: BossPhase::Phase1,
            max_hp,
            current_hp: max_hp,
            defeated: false,
            phase_2_hp_threshold: 0.75,
            phase_3_hp_threshold: 0.25,
        }
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.current_hp / self.max_hp).clamp(0.0, 1.0)
        }
    }

    /// Compute the phase the current HP fraction implies, without mutating.
    pub fn implied_phase(&self) -> BossPhase {
        let f = self.hp_fraction();
        if f < self.phase_3_hp_threshold {
            BossPhase::Phase3
        } else if f < self.phase_2_hp_threshold {
            BossPhase::Phase2
        } else {
            BossPhase::Phase1
        }
    }

    /// Apply damage; return Some(BossPhase) if the phase changed.
    pub fn apply_damage(&mut self, damage: f32) -> Option<BossPhase> {
        if self.defeated {
            return None;
        }
        self.current_hp = (self.current_hp - damage.max(0.0)).max(0.0);
        if self.current_hp <= 0.0 {
            self.defeated = true;
        }
        let new_phase = self.implied_phase();
        if new_phase != self.current_phase {
            self.current_phase = new_phase;
            return Some(new_phase);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossPhaseChangedEvent {
    pub actor_id: u64,
    pub from: BossPhase,
    pub to: BossPhase,
    pub hp_fraction: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossSpecialAbilityEvent {
    pub actor_id: u64,
    pub phase: BossPhase,
    pub ability: String,
    pub tick: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_transitions_at_thresholds() {
        let mut b = BossState::new(1, "spotter", 100.0);
        assert_eq!(b.current_phase, BossPhase::Phase1);
        let p = b.apply_damage(30.0);
        // HP=70/100 = 0.70 (< 0.75) → Phase2
        assert_eq!(p, Some(BossPhase::Phase2));
        let p = b.apply_damage(50.0);
        // HP=20/100 = 0.20 (< 0.25) → Phase3
        assert_eq!(p, Some(BossPhase::Phase3));
        b.apply_damage(40.0);
        assert!(b.defeated);
    }
}
