//! cf-priority — commandability surface promoted from M7-A's in-cf-ai types.
//!
//! M7-A shipped `PriorityTable`, `AutonomyMode`, `Archetype`, `QuickPreset`,
//! `TaskType` inside the cf-ai crate so the 5-layer thinking stack could
//! depend on them without a circular dep. M7-B promotes the player-facing
//! commandability layer here: this crate is what cf-control's cfctl
//! `act.player.set_priority` / `apply_role_template` / `apply_quick_preset` /
//! `set_autonomy_mode` methods talk to.
//!
//! Layout:
//!
//! - Re-exports the canonical M7-A types (`PriorityTable`, `AutonomyMode`,
//!   `Archetype`, `TaskType`).
//! - Adds the spec-named `QuickPresetId` enum (Attack / Defend / Overwatch /
//!   Rescue / Salvage) — these are the five labels surfaced to players in
//!   the Tab tactical overlay (M8 owns the UI). cf-ai's existing
//!   `QuickPreset` (Aggressive / Defensive / Scout / Berserk / Custom) keeps
//!   the original M7-A engine vocabulary; `QuickPresetId` is the cfctl wire
//!   form.
//! - Adds `RoleTemplate` (one of the 6 archetypes) — a player-facing wrapper
//!   over `Archetype::role_template` so the cfctl method
//!   `act.player.apply_role_template` has a stable string surface.
//! - Adds `PersonalityModifier` — 4 personality presets (Aggressive /
//!   Cautious / Loyal / LoneWolf) that re-weight specific task families
//!   on top of the role template. M7-A shipped `PersonalityProfile` /
//!   `PersonalityTrait`; the modifier here is the explicit re-weight
//!   contract the spec mandates.
//!
//! Serde round-trip: every type is `Serialize + Deserialize` so a
//! `PriorityTable` snapshotted via JSON/RON deserialises byte-identically.
//! The 22-task layout is locked at M8A; cf-priority MUST NOT add / remove /
//! re-order task variants.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

pub use cf_ai::{Archetype, AutonomyMode, PriorityTable, QuickPreset, TaskType};

/// One of the 6 spec-mandated role templates. Maps 1:1 to `Archetype` for
/// the engine's per-bot ThinkingStack, but provides a stable string surface
/// (`rifleman` / `sniper` / `assault` / `engineer` / `spotter` / `medic`)
/// for the `act.player.apply_role_template` cfctl method.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum RoleTemplate {
    #[default]
    Rifleman,
    Sniper,
    Assault,
    Engineer,
    Spotter,
    Medic,
}

impl RoleTemplate {
    /// Every variant in declaration order.
    pub const ALL: [RoleTemplate; 6] = [
        RoleTemplate::Rifleman,
        RoleTemplate::Sniper,
        RoleTemplate::Assault,
        RoleTemplate::Engineer,
        RoleTemplate::Spotter,
        RoleTemplate::Medic,
    ];

    /// Canonical snake_case identifier surfaced to cfctl + replay bundles.
    pub fn as_str(self) -> &'static str {
        match self {
            RoleTemplate::Rifleman => "rifleman",
            RoleTemplate::Sniper => "sniper",
            RoleTemplate::Assault => "assault",
            RoleTemplate::Engineer => "engineer",
            RoleTemplate::Spotter => "spotter",
            RoleTemplate::Medic => "medic",
        }
    }

    /// Parse from the cfctl wire form (snake_case).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<RoleTemplate> {
        Some(match value {
            "rifleman" => RoleTemplate::Rifleman,
            "sniper" => RoleTemplate::Sniper,
            "assault" => RoleTemplate::Assault,
            "engineer" => RoleTemplate::Engineer,
            "spotter" => RoleTemplate::Spotter,
            "medic" => RoleTemplate::Medic,
            _ => return None,
        })
    }

    /// 1:1 mapping to the engine-side `Archetype`.
    pub fn archetype(self) -> Archetype {
        match self {
            RoleTemplate::Rifleman => Archetype::Rifleman,
            RoleTemplate::Sniper => Archetype::Sniper,
            RoleTemplate::Assault => Archetype::Assault,
            RoleTemplate::Engineer => Archetype::Engineer,
            RoleTemplate::Spotter => Archetype::Spotter,
            RoleTemplate::Medic => Archetype::Medic,
        }
    }

    /// Build the role's pre-tuned `PriorityTable`. Wraps
    /// `Archetype::role_template` so cf-control depends only on cf-priority.
    pub fn priority_table(self) -> PriorityTable {
        self.archetype().role_template()
    }

    /// Apply the role's template to `table` in-place. Overwrites every
    /// weight in the 22-task grid.
    pub fn apply_to(self, table: &mut PriorityTable) {
        *table = self.priority_table();
    }
}

/// The 5 player-facing quick presets surfaced in M8's Tab tactical overlay.
///
/// `Attack` / `Defend` map to cf-ai's `QuickPreset::Aggressive` /
/// `QuickPreset::Defensive` shift; the other three are new families that
/// shift different task subsets per spec § Smart commandable AI.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum QuickPresetId {
    #[default]
    Attack,
    Defend,
    Overwatch,
    Rescue,
    Salvage,
}

impl QuickPresetId {
    /// Every variant in declaration order.
    pub const ALL: [QuickPresetId; 5] = [
        QuickPresetId::Attack,
        QuickPresetId::Defend,
        QuickPresetId::Overwatch,
        QuickPresetId::Rescue,
        QuickPresetId::Salvage,
    ];

    /// Canonical snake_case identifier surfaced to cfctl + replay bundles.
    pub fn as_str(self) -> &'static str {
        match self {
            QuickPresetId::Attack => "attack",
            QuickPresetId::Defend => "defend",
            QuickPresetId::Overwatch => "overwatch",
            QuickPresetId::Rescue => "rescue",
            QuickPresetId::Salvage => "salvage",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<QuickPresetId> {
        Some(match value {
            "attack" => QuickPresetId::Attack,
            "defend" => QuickPresetId::Defend,
            "overwatch" => QuickPresetId::Overwatch,
            "rescue" => QuickPresetId::Rescue,
            "salvage" => QuickPresetId::Salvage,
            _ => return None,
        })
    }

    /// Apply the preset's spec-mandated shift to `table` in place. The
    /// shift biases relevant task families ±2 per spec § Quick presets.
    pub fn apply_to(self, table: &mut PriorityTable) {
        match self {
            QuickPresetId::Attack => {
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
            QuickPresetId::Defend => {
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
            QuickPresetId::Overwatch => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::SharpshootTarget
                            | TaskType::MarkThreats
                            | TaskType::ScoutAhead
                            | TaskType::HoldCover
                            | TaskType::InvestigateSound
                    )
                });
                table.shift(-2, |t| {
                    matches!(t, TaskType::FlankTarget | TaskType::Demolish | TaskType::ThrowGrenade)
                });
            }
            QuickPresetId::Rescue => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::TriageDownedAlly
                            | TaskType::CoverAlly
                            | TaskType::DefendBrainActor
                            | TaskType::HealSelf
                            | TaskType::RetreatToCover
                    )
                });
                table.shift(-2, |t| {
                    matches!(
                        t,
                        TaskType::EngageVisibleEnemy
                            | TaskType::FlankTarget
                            | TaskType::Demolish
                            | TaskType::ThrowGrenade
                    )
                });
            }
            QuickPresetId::Salvage => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::RepairChassisModule
                            | TaskType::RepairTerrainBreach
                            | TaskType::SetTrap
                            | TaskType::DigCover
                            | TaskType::ScoutAhead
                    )
                });
                table.shift(-2, |t| {
                    matches!(
                        t,
                        TaskType::EngageVisibleEnemy
                            | TaskType::FlankTarget
                            | TaskType::SuppressFire
                            | TaskType::ThrowGrenade
                    )
                });
            }
        }
    }
}

/// Personality preset that re-weights the PriorityTable on top of the role
/// template. Spec § Personality + traits — Aggressive / Cautious / Loyal /
/// LoneWolf each bias specific task families so the same `Archetype` can
/// produce distinct bots.
///
/// The preset is APPLIED ONCE at archetype assignment / personality change;
/// `PersonalityProfile`'s mood + stress still feed into per-tick scoring via
/// `mood_score_multiplier`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum PersonalityModifier {
    #[default]
    Neutral,
    Aggressive,
    Cautious,
    Loyal,
    LoneWolf,
}

impl PersonalityModifier {
    /// Every variant in declaration order.
    pub const ALL: [PersonalityModifier; 5] = [
        PersonalityModifier::Neutral,
        PersonalityModifier::Aggressive,
        PersonalityModifier::Cautious,
        PersonalityModifier::Loyal,
        PersonalityModifier::LoneWolf,
    ];

    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            PersonalityModifier::Neutral => "neutral",
            PersonalityModifier::Aggressive => "aggressive",
            PersonalityModifier::Cautious => "cautious",
            PersonalityModifier::Loyal => "loyal",
            PersonalityModifier::LoneWolf => "lone_wolf",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<PersonalityModifier> {
        Some(match value {
            "neutral" => PersonalityModifier::Neutral,
            "aggressive" => PersonalityModifier::Aggressive,
            "cautious" => PersonalityModifier::Cautious,
            "loyal" => PersonalityModifier::Loyal,
            "lone_wolf" => PersonalityModifier::LoneWolf,
            _ => return None,
        })
    }

    /// Apply the personality preset's bias to `table` in-place. The
    /// modifier shifts task families ±2 (Neutral is a no-op).
    pub fn apply_to(self, table: &mut PriorityTable) {
        match self {
            PersonalityModifier::Neutral => {}
            PersonalityModifier::Aggressive => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::EngageVisibleEnemy
                            | TaskType::FlankTarget
                            | TaskType::Demolish
                            | TaskType::ThrowGrenade
                    )
                });
                table.shift(-1, |t| matches!(t, TaskType::HoldCover | TaskType::RetreatToCover));
            }
            PersonalityModifier::Cautious => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::HoldCover | TaskType::RetreatToCover | TaskType::HealSelf | TaskType::CoverAlly
                    )
                });
                table.shift(-1, |t| matches!(t, TaskType::FlankTarget | TaskType::Demolish));
            }
            PersonalityModifier::Loyal => {
                table.shift(2, |t| {
                    matches!(
                        t,
                        TaskType::CoverAlly | TaskType::DefendBrainActor | TaskType::TriageDownedAlly
                    )
                });
                table.shift(1, |t| matches!(t, TaskType::FollowOrder));
            }
            PersonalityModifier::LoneWolf => {
                table.shift(1, |t| matches!(t, TaskType::ScoutAhead | TaskType::SharpshootTarget));
                table.shift(-2, |t| matches!(t, TaskType::FollowOrder | TaskType::CoverAlly));
            }
        }
    }
}

/// Convenience: a single re-weight pass — build the role template, apply a
/// quick preset, then apply a personality modifier. Useful for tests +
/// the cf-control engine when spawning a new bot.
pub fn build_priority_table(
    role: RoleTemplate,
    preset: Option<QuickPresetId>,
    personality: Option<PersonalityModifier>,
) -> PriorityTable {
    let mut t = role.priority_table();
    if let Some(p) = preset {
        p.apply_to(&mut t);
    }
    if let Some(m) = personality {
        m.apply_to(&mut t);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_template_round_trip_str() {
        for r in RoleTemplate::ALL.iter() {
            assert_eq!(RoleTemplate::from_str(r.as_str()), Some(*r));
        }
    }

    #[test]
    fn quick_preset_id_round_trip_str() {
        for p in QuickPresetId::ALL.iter() {
            assert_eq!(QuickPresetId::from_str(p.as_str()), Some(*p));
        }
    }

    #[test]
    fn personality_modifier_round_trip_str() {
        for m in PersonalityModifier::ALL.iter() {
            assert_eq!(PersonalityModifier::from_str(m.as_str()), Some(*m));
        }
    }

    #[test]
    fn medic_role_template_high_triage() {
        let t = RoleTemplate::Medic.priority_table();
        assert_eq!(t.get(TaskType::TriageDownedAlly), 9);
        assert_eq!(t.get(TaskType::HealSelf), 8);
    }

    #[test]
    fn quick_preset_attack_boosts_engage() {
        let mut t = PriorityTable::neutral();
        QuickPresetId::Attack.apply_to(&mut t);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 7);
        assert_eq!(t.get(TaskType::HoldCover), 3);
    }

    #[test]
    fn quick_preset_rescue_boosts_triage() {
        let mut t = PriorityTable::neutral();
        QuickPresetId::Rescue.apply_to(&mut t);
        assert_eq!(t.get(TaskType::TriageDownedAlly), 7);
        assert_eq!(t.get(TaskType::CoverAlly), 7);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 3);
    }

    #[test]
    fn quick_preset_salvage_boosts_repair() {
        let mut t = PriorityTable::neutral();
        QuickPresetId::Salvage.apply_to(&mut t);
        assert_eq!(t.get(TaskType::RepairChassisModule), 7);
        assert_eq!(t.get(TaskType::RepairTerrainBreach), 7);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 3);
    }

    #[test]
    fn quick_preset_overwatch_boosts_sharpshoot() {
        let mut t = PriorityTable::neutral();
        QuickPresetId::Overwatch.apply_to(&mut t);
        assert_eq!(t.get(TaskType::SharpshootTarget), 7);
        assert_eq!(t.get(TaskType::MarkThreats), 7);
        assert_eq!(t.get(TaskType::FlankTarget), 3);
    }

    #[test]
    fn quick_preset_defend_boosts_hold_cover() {
        let mut t = PriorityTable::neutral();
        QuickPresetId::Defend.apply_to(&mut t);
        assert_eq!(t.get(TaskType::HoldCover), 7);
        assert_eq!(t.get(TaskType::FlankTarget), 3);
    }

    #[test]
    fn aggressive_modifier_boosts_engage() {
        let mut t = PriorityTable::neutral();
        PersonalityModifier::Aggressive.apply_to(&mut t);
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 7);
        assert_eq!(t.get(TaskType::HoldCover), 4);
    }

    #[test]
    fn cautious_modifier_boosts_hold_cover() {
        let mut t = PriorityTable::neutral();
        PersonalityModifier::Cautious.apply_to(&mut t);
        assert_eq!(t.get(TaskType::HoldCover), 7);
        assert_eq!(t.get(TaskType::FlankTarget), 4);
    }

    #[test]
    fn loyal_modifier_boosts_cover_ally() {
        let mut t = PriorityTable::neutral();
        PersonalityModifier::Loyal.apply_to(&mut t);
        assert_eq!(t.get(TaskType::CoverAlly), 7);
        assert_eq!(t.get(TaskType::DefendBrainActor), 7);
    }

    #[test]
    fn lone_wolf_modifier_reduces_follow_order() {
        let mut t = PriorityTable::neutral();
        PersonalityModifier::LoneWolf.apply_to(&mut t);
        assert_eq!(t.get(TaskType::FollowOrder), 3);
        assert_eq!(t.get(TaskType::CoverAlly), 3);
        assert_eq!(t.get(TaskType::ScoutAhead), 6);
    }

    #[test]
    fn priority_table_round_trips_through_serde_json() {
        let mut t = RoleTemplate::Sniper.priority_table();
        t.set(TaskType::SharpshootTarget, 9);
        t.set(TaskType::HoldCover, 8);
        let json = serde_json::to_string(&t).expect("serialize");
        let back: PriorityTable = serde_json::from_str(&json).expect("deserialize");
        for task in TaskType::ALL.iter() {
            assert_eq!(t.get(*task), back.get(*task), "weight differs for {:?}", task);
        }
    }

    #[test]
    fn priority_table_round_trips_through_serde_ron() {
        let mut t = RoleTemplate::Engineer.priority_table();
        t.set(TaskType::SetTrap, 9);
        t.set(TaskType::Demolish, 6);
        let serialised = ron::ser::to_string(&t).expect("ron serialize");
        let back: PriorityTable = ron::de::from_str(&serialised).expect("ron deserialize");
        for task in TaskType::ALL.iter() {
            assert_eq!(t.get(*task), back.get(*task), "ron weight differs for {:?}", task);
        }
    }

    #[test]
    fn build_priority_table_composes_role_preset_personality() {
        let t = build_priority_table(
            RoleTemplate::Rifleman,
            Some(QuickPresetId::Attack),
            Some(PersonalityModifier::Aggressive),
        );
        // Rifleman base EngageVisibleEnemy = 7; +2 (Attack) = 9; +2 (Aggr) = 11 -> clamp to 9.
        assert_eq!(t.get(TaskType::EngageVisibleEnemy), 9);
    }

    #[test]
    fn autonomy_mode_re_exported_and_round_trips() {
        for m in [AutonomyMode::FullAuto, AutonomyMode::Standard, AutonomyMode::Manual] {
            assert_eq!(AutonomyMode::from_str(m.as_str()), Some(m));
        }
        assert_eq!(AutonomyMode::default(), AutonomyMode::FullAuto);
    }
}
