//! M7-A: per-actor `PriorityTable` (22-task × 1-9 weight grid).
//!
//! The Priority Table feeds the Utility scorer as a multiplicative weight prior:
//!
//! `final_score(task) = base_utility(task, world) × priority_weight[task] / 5.0`
//!
//! Priority weight 5 = neutral (1.0×); 9 = strong preference (1.8×);
//! 1 = strong avoidance (0.2×); 0 = disabled (0.0×, never auto-act).
//!
//! M7-B promotes this surface into a dedicated `cf-priority` crate with full
//! cfctl methods + persistence. M7-A ships the type in-crate so the 5-layer
//! thinking stack + archetype templates work without a circular dep.

use serde::{Deserialize, Serialize};

use crate::task::TaskType;

/// **M7-A**: 22-task weight grid (per-actor).
///
/// Weights are `u8` in `0..=9`. The `weights` array is indexed by
/// `TaskType::ordinal()` so the layout is byte-stable across builds and
/// preserves replay determinism even when M7-B refactors the storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PriorityTable {
    pub weights: [u8; TaskType::COUNT],
}

impl PriorityTable {
    /// Build a neutral table (every task weighted 5).
    pub fn neutral() -> Self {
        Self {
            weights: [NEUTRAL_WEIGHT; TaskType::COUNT],
        }
    }

    /// Build a zero-everywhere table (Manual autonomy default before role
    /// template fills in).
    pub fn empty() -> Self {
        Self {
            weights: [0; TaskType::COUNT],
        }
    }

    pub fn get(&self, task: TaskType) -> u8 {
        self.weights[task.ordinal()]
    }

    /// Set the weight for one task. Clamps to `0..=9`.
    pub fn set(&mut self, task: TaskType, weight: u8) {
        self.weights[task.ordinal()] = weight.min(9);
    }

    /// Apply the +/- delta across the table, clamping each weight to `[0, 9]`.
    /// `selector` controls which task ordinals receive the shift; common
    /// callers pass `|_| true` to shift everything (quick preset Custom) or
    /// a closure that filters by task family.
    pub fn shift(&mut self, delta: i8, mut selector: impl FnMut(TaskType) -> bool) {
        for (i, w) in self.weights.iter_mut().enumerate() {
            let task = TaskType::ALL[i];
            if !selector(task) {
                continue;
            }
            let next = (*w as i16 + delta as i16).clamp(0, 9) as u8;
            *w = next;
        }
    }

    /// Multiplicative weight feeding the Utility scorer. Matches the spec
    /// literal: `priority_weight[task] / 5.0`.
    pub fn multiplier(&self, task: TaskType) -> f32 {
        let w = self.get(task) as f32;
        w / NEUTRAL_WEIGHT_F32
    }

    /// Bytes for the determinism checksum. Layout = ordinal-indexed weight
    /// array. Stable across builds.
    pub fn checksum_bytes(&self) -> [u8; TaskType::COUNT] {
        self.weights
    }
}

const NEUTRAL_WEIGHT: u8 = 5;
const NEUTRAL_WEIGHT_F32: f32 = 5.0;

impl Default for PriorityTable {
    fn default() -> Self {
        Self::neutral()
    }
}

/// **M7-A**: 5 quick presets that bias a base template via `shift()`.
/// Spec § Smart commandable AI — Quick presets (Aggressive / Defensive /
/// Scout / Berserk / Custom).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QuickPreset {
    /// No shift — table is whatever the role template + player overrides
    /// produced.
    #[default]
    Custom,
    Aggressive,
    Defensive,
    Scout,
    Berserk,
}

impl QuickPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            QuickPreset::Custom => "custom",
            QuickPreset::Aggressive => "aggressive",
            QuickPreset::Defensive => "defensive",
            QuickPreset::Scout => "scout",
            QuickPreset::Berserk => "berserk",
        }
    }

    pub fn from_str(value: &str) -> Option<QuickPreset> {
        Some(match value {
            "custom" => QuickPreset::Custom,
            "aggressive" => QuickPreset::Aggressive,
            "defensive" => QuickPreset::Defensive,
            "scout" => QuickPreset::Scout,
            "berserk" => QuickPreset::Berserk,
            _ => return None,
        })
    }

    /// Apply this preset's spec-mandated shift to `table` in place. The
    /// shift biases relevant task families ±2 per spec.
    pub fn apply(self, table: &mut PriorityTable) {
        match self {
            QuickPreset::Custom => {}
            QuickPreset::Aggressive => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::EngageVisibleEnemy
                            | TaskType::FlankTarget
                            | TaskType::ThrowGrenade
                            | TaskType::Demolish
                            | TaskType::SharpshootTarget
                    )
                });
                table.shift(-2, |t| {
                    matches!(t, TaskType::HoldCover | TaskType::RetreatToCover | TaskType::Patrol)
                });
            }
            QuickPreset::Defensive => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::HoldCover
                            | TaskType::SuppressFire
                            | TaskType::CoverAlly
                            | TaskType::DefendBrainActor
                            | TaskType::RetreatToCover
                    )
                });
                table.shift(-2, |t| {
                    matches!(t, TaskType::FlankTarget | TaskType::Demolish | TaskType::Patrol)
                });
            }
            QuickPreset::Scout => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::ScoutAhead | TaskType::MarkThreats | TaskType::InvestigateSound | TaskType::Patrol
                    )
                });
                table.shift(-2, |t| {
                    matches!(t, TaskType::SuppressFire | TaskType::Demolish | TaskType::ThrowGrenade)
                });
            }
            QuickPreset::Berserk => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::EngageVisibleEnemy
                            | TaskType::FlankTarget
                            | TaskType::ThrowGrenade
                            | TaskType::Demolish
                    )
                });
                table.shift(-2, |t| {
                    matches!(
                        t,
                        TaskType::HoldCover | TaskType::RetreatToCover | TaskType::HealSelf | TaskType::CoverAlly
                    )
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_multiplier_is_one() {
        let t = PriorityTable::neutral();
        for task in TaskType::ALL.iter() {
            assert!((t.multiplier(*task) - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn weight_nine_yields_1_8_multiplier() {
        let mut t = PriorityTable::neutral();
        t.set(TaskType::TriageDownedAlly, 9);
        assert!((t.multiplier(TaskType::TriageDownedAlly) - 1.8).abs() < 0.001);
    }

    #[test]
    fn weight_one_yields_0_2_multiplier() {
        let mut t = PriorityTable::neutral();
        t.set(TaskType::EngageVisibleEnemy, 1);
        assert!((t.multiplier(TaskType::EngageVisibleEnemy) - 0.2).abs() < 0.001);
    }

    #[test]
    fn weight_clamped_to_nine() {
        let mut t = PriorityTable::neutral();
        t.set(TaskType::SharpshootTarget, 250);
        assert_eq!(t.get(TaskType::SharpshootTarget), 9);
    }

    #[test]
    fn aggressive_quick_preset_boosts_engage() {
        let mut t = PriorityTable::neutral();
        QuickPreset::Aggressive.apply(&mut t);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 7);
        assert_eq!(t.get(TaskType::HoldCover), 3);
    }
}
