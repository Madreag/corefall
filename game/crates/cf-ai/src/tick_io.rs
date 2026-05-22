use serde::{Deserialize, Serialize};

use cf_actor::ActorState;

use crate::{GuardStateTransition, PerceptionRecord, Tactic};

/// Inputs for one [`crate::step`] call.
#[derive(Debug, Clone, Copy)]
pub struct GuardTickInputs<'a> {
    pub tick: u64,
    pub tick_rate_hz: u32,
    pub self_actor: &'a ActorState,
    pub player: Option<&'a ActorState>,
    pub alarms: &'a [AlarmInput],
    pub last_damage_source: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AlarmInput {
    pub source_actor: u64,
    pub source_position: [f32; 2],
    pub loudness_radius: f32,
    pub alarm_event_id: Option<String>,
}

/// Outcomes of one [`crate::step`] call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnemyTickReport {
    pub state_changes: Vec<GuardStateTransition>,
    pub perception: Option<PerceptionRecord>,
    pub tactic_chosen: Option<TacticRecord>,
    pub fire: Option<FireRecord>,
    pub reload_started: bool,
    pub reload_completed: bool,
    pub dry_fire: bool,
    pub perception_signals: Vec<PerceptionSignal>,
    pub missed_shot_reason: Option<MissedShotReason>,
    pub stuck_recovery: Option<StuckRecoveryRecord>,
    pub target_acquired: Option<TargetAcquiredRecord>,
    pub target_lost: Option<TargetLostRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptionSignal {
    /// One of `"sight"`, `"sight_lost"`, `"hearing"`, `"memory_decayed"`.
    pub kind: &'static str,
    pub source_actor: Option<u64>,
    pub source_position: Option<[f32; 2]>,
    /// Confidence in `[0.0, 1.0]`. Hearing decays linearly with distance.
    pub confidence: f32,
    pub tick: u64,
    #[serde(default)]
    pub alarm_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedShotReason {
    RecoilDeviation,
    TargetMoved,
    Occlusion,
    LuckyDodge,
}

impl MissedShotReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissedShotReason::RecoilDeviation => "recoil_deviation",
            MissedShotReason::TargetMoved => "target_moved",
            MissedShotReason::Occlusion => "occlusion",
            MissedShotReason::LuckyDodge => "lucky_dodge",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StuckRecoveryRecord {
    pub stuck_ticks: u32,
    pub blocker: &'static str,
    pub action: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetAcquiredRecord {
    pub target_actor: u64,
    pub via: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetLostRecord {
    pub target_actor: u64,
    pub reason: &'static str,
}

/// Recorded `ai.tactic_chosen` payload. `score_*` fields are the utility scores
/// the scorer evaluated this tick — exposed so the run-bundle viewer can show
/// the AI's reasoning.
#[derive(Debug, Clone, PartialEq)]
pub struct TacticRecord {
    pub tactic: Tactic,
    pub reason: &'static str,
    pub score_attack: f32,
    pub score_reload: f32,
    pub score_hold: f32,
    pub score_search: f32,
}

/// Recorded enemy weapon fire. The engine spawns a projectile the player can
/// actually be hit by.
#[derive(Debug, Clone, PartialEq)]
pub struct FireRecord {
    pub muzzle_origin: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub damage: f32,
    pub miss_roll: f32,
    pub miss_threshold: f32,
    pub will_miss: bool,
    pub lifetime_ticks: u32,
}
