//! M7-A: 6 enemy archetypes (Rifleman / Sniper / Assault / Engineer / Spotter / Medic).
//!
//! Each archetype carries a Priority Table role template (22-task weight grid)
//! that the Utility scorer multiplies the base utility by:
//!
//! `final_score(task) = base_utility(task, world) × priority_weight[task] / 5.0`
//!
//! Weight 5 is neutral (1.0×); 9 is strong preference (1.8×); 1 is strong
//! avoidance (0.2×); 0 is disabled (0.0×). Templates are tuned per spec
//! § Smart commandable AI — 6 archetypes.

use serde::{Deserialize, Serialize};

use crate::priority::PriorityTable;
use crate::task::TaskType;

/// **M7-A**: 6 enemy archetypes. Drives FOV / range / reaction time / HP +
/// Priority Table role template + behavior tree library.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Archetype {
    #[default]
    Rifleman,
    Sniper,
    Assault,
    Engineer,
    Spotter,
    Medic,
}

impl Archetype {
    pub const ALL: [Archetype; 6] = [
        Archetype::Rifleman,
        Archetype::Sniper,
        Archetype::Assault,
        Archetype::Engineer,
        Archetype::Spotter,
        Archetype::Medic,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Archetype::Rifleman => "rifleman",
            Archetype::Sniper => "sniper",
            Archetype::Assault => "assault",
            Archetype::Engineer => "engineer",
            Archetype::Spotter => "spotter",
            Archetype::Medic => "medic",
        }
    }

    pub fn from_str(value: &str) -> Option<Archetype> {
        Some(match value {
            "rifleman" => Archetype::Rifleman,
            "sniper" => Archetype::Sniper,
            "assault" => Archetype::Assault,
            "engineer" => Archetype::Engineer,
            "spotter" => Archetype::Spotter,
            "medic" => Archetype::Medic,
            _ => return None,
        })
    }

    /// Spec-mandated default HP per archetype.
    pub fn default_hp(self) -> f32 {
        match self {
            Archetype::Rifleman => 80.0,
            Archetype::Sniper => 60.0,
            Archetype::Assault => 90.0,
            Archetype::Engineer => 70.0,
            Archetype::Spotter => 50.0,
            Archetype::Medic => 60.0,
        }
    }

    /// Spec-mandated sight cone (degrees) per archetype.
    pub fn default_fov_degrees(self) -> f32 {
        match self {
            Archetype::Rifleman => 180.0,
            Archetype::Sniper => 240.0,
            Archetype::Assault => 120.0,
            Archetype::Engineer => 180.0,
            Archetype::Spotter => 200.0,
            Archetype::Medic => 160.0,
        }
    }

    /// Spec-mandated sight range (world units) per archetype.
    pub fn default_sight_range(self) -> f32 {
        match self {
            Archetype::Rifleman => 320.0,
            Archetype::Sniper => 800.0,
            Archetype::Assault => 200.0,
            Archetype::Engineer => 280.0,
            Archetype::Spotter => 360.0,
            Archetype::Medic => 280.0,
        }
    }

    /// Spec-mandated reaction time (ticks) per archetype.
    pub fn default_reaction_ticks(self) -> u32 {
        match self {
            Archetype::Rifleman => 12,
            Archetype::Sniper => 18,
            Archetype::Assault => 6,
            Archetype::Engineer => 12,
            Archetype::Spotter => 12,
            Archetype::Medic => 12,
        }
    }

    /// Build the archetype's Priority Table role template — a 22-task
    /// weight grid pre-tuned per spec § Smart commandable AI — 6 archetypes.
    /// Defaults are documented in the spec table; weights not called out
    /// stay at the neutral baseline of 5.
    pub fn role_template(self) -> PriorityTable {
        let mut t = PriorityTable::neutral();
        match self {
            Archetype::Rifleman => {
                t.set(TaskType::EngageVisibleEnemy, 7);
                t.set(TaskType::SuppressFire, 7);
                t.set(TaskType::HoldCover, 6);
                t.set(TaskType::FlankTarget, 6);
                t.set(TaskType::InvestigateSound, 5);
                t.set(TaskType::Patrol, 5);
                t.set(TaskType::RetreatToCover, 6);
            }
            Archetype::Sniper => {
                t.set(TaskType::SharpshootTarget, 9);
                t.set(TaskType::MarkThreats, 7);
                t.set(TaskType::ScoutAhead, 7);
                t.set(TaskType::HoldCover, 8);
                t.set(TaskType::EngageVisibleEnemy, 5);
                t.set(TaskType::SuppressFire, 3);
                t.set(TaskType::FlankTarget, 2);
                t.set(TaskType::RetreatToCover, 7);
            }
            Archetype::Assault => {
                t.set(TaskType::FlankTarget, 8);
                t.set(TaskType::ThrowGrenade, 8);
                t.set(TaskType::EngageVisibleEnemy, 8);
                t.set(TaskType::Demolish, 7);
                t.set(TaskType::HoldCover, 3);
                t.set(TaskType::SuppressFire, 4);
                t.set(TaskType::RetreatToCover, 3);
            }
            Archetype::Engineer => {
                t.set(TaskType::RepairChassisModule, 9);
                t.set(TaskType::RepairTerrainBreach, 8);
                t.set(TaskType::SetTrap, 7);
                t.set(TaskType::DigCover, 6);
                t.set(TaskType::EngageVisibleEnemy, 3);
                t.set(TaskType::HoldCover, 5);
                t.set(TaskType::CoverAlly, 5);
            }
            Archetype::Spotter => {
                t.set(TaskType::MarkThreats, 9);
                t.set(TaskType::InvestigateSound, 7);
                t.set(TaskType::ScoutAhead, 7);
                t.set(TaskType::DefendBrainActor, 9);
                t.set(TaskType::EngageVisibleEnemy, 4);
                t.set(TaskType::HoldCover, 6);
            }
            Archetype::Medic => {
                t.set(TaskType::TriageDownedAlly, 9);
                t.set(TaskType::HealSelf, 8);
                t.set(TaskType::CoverAlly, 7);
                t.set(TaskType::DefendBrainActor, 9);
                t.set(TaskType::EngageVisibleEnemy, 3);
                t.set(TaskType::HoldCover, 5);
                t.set(TaskType::FollowOrder, 6);
            }
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_archetypes() {
        assert_eq!(Archetype::ALL.len(), 6);
        for a in &Archetype::ALL {
            assert_eq!(Archetype::from_str(a.as_str()), Some(*a));
        }
    }

    #[test]
    fn medic_role_template_high_triage() {
        let t = Archetype::Medic.role_template();
        assert_eq!(t.get(TaskType::TriageDownedAlly), 9);
        assert_eq!(t.get(TaskType::HealSelf), 8);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 3);
    }

    #[test]
    fn engineer_role_template_high_repair() {
        let t = Archetype::Engineer.role_template();
        assert_eq!(t.get(TaskType::RepairChassisModule), 9);
        assert_eq!(t.get(TaskType::RepairTerrainBreach), 8);
        assert_eq!(t.get(TaskType::SetTrap), 7);
    }

    #[test]
    fn sniper_role_template_high_sharpshoot() {
        let t = Archetype::Sniper.role_template();
        assert_eq!(t.get(TaskType::SharpshootTarget), 9);
        assert_eq!(t.get(TaskType::HoldCover), 8);
    }
}
