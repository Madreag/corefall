//! M2 objective grammar — `Objective`, `ObjectiveKind`, `ObjectiveStatus`,
//! `FailSensor`. Split out of `lib.rs` for the 2k-LOC ceiling. Public API
//! is re-exported at the crate root.

use serde::{Deserialize, Serialize};

/// One objective the player must clear (or, if `optional`, may skip).
///
/// M1.5 supports three kinds:
///
/// - `BreachBarrier { target }` — break the breach strip with the given id.
/// - `NeutralizeActor { target }` — drive the named actor to `Status::Dead`.
/// - `ReachZone { min, max }` — the player's position lies inside the AABB.
///
/// M7's full mission director extends this to typed mission manifests; the
/// objective ids and status names ship unchanged so M1.5 evidence stays valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub kind: ObjectiveKind,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub status: ObjectiveStatus,
    /// **M1.5**: highest progress milestone emitted so far for this objective.
    /// 0 = none, 1 = 25%, 2 = 50%, 3 = 75%, 4 = 100% (the 100% milestone fires
    /// in lockstep with `objective_completed`). Tracked so `mission.objective_updated`
    /// fires once per crossed quartile.
    #[serde(default)]
    pub progress_milestone_index: u8,
    /// **M2 re-audit (2026-05-13)**: continuous progress fraction (0.0..1.0).
    /// PROGRESS_QUARTILES = [0.25, 0.5, 0.75, 1.0] drives the M2 quartile
    /// event emission. Mirrors the spec's `Objective.progress: f32`.
    #[serde(default)]
    pub progress: f32,
    /// **M2 re-audit (2026-05-13)**: optional fail-sensor descriptor per
    /// the spec literal `Objective { id, kind, status, progress, fail_sensor }`.
    /// `None` for objectives without an explicit fail-sensor (the kind's
    /// implicit fail-sensor still applies — e.g. DefendReactor fails on
    /// reactor destruction). M7+ uses this for declarative fail-sensors
    /// (`FailSensor::TimerWindow { from_tick, threshold_ticks }`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_sensor: Option<FailSensor>,
}

/// **M2 re-audit (2026-05-13)**: declarative fail-sensor descriptor. M7+
/// extends with richer sensors; M2 ships the type so scenario manifests can
/// reference it without a schema bump.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FailSensor {
    /// Fail if `current_tick - from_tick > threshold_ticks`.
    TimerWindow { from_tick: u64, threshold_ticks: u64 },
    /// Fail when the target actor's HP reaches zero.
    ActorHpZero { target: u64 },
}

/// Kind of objective. Discriminator names match the canonical roadmap glossary so
/// M7's typed manifest can read M2 scenario files without migrating ids.
///
/// **M2 re-audit pass 3 (2026-05-13)**: spec literal at M2.md line 109 calls
/// for `ReachZone, KillActor, SurviveTimer, DefendActor, EscortActor`. The
/// codebase predates the spec rename so `NeutralizeActor` / `DefendReactor`
/// are still the canonical Rust identifiers — but `kill_actor` and
/// `defend_actor` are accepted as JSON discriminator aliases via `serde(alias)`
/// so scenario manifests authored against the spec deserialize cleanly. The
/// underlying behaviour is identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveKind {
    BreachBarrier {
        target: String,
    },
    #[serde(alias = "kill_actor")]
    NeutralizeActor {
        target: u64,
    },
    ReachZone {
        min: [f32; 2],
        max: [f32; 2],
    },
    /// M2.5: defend a reactor (named static actor) until either the mission
    /// timer expires (success) or the reactor's hp reaches zero (failure).
    /// `target` is the reactor id. The `defend_actor` JSON discriminator
    /// (M2 spec literal) is NOT aliased here — see the dedicated
    /// `DefendActor` variant below for the M9-canonical generic form.
    DefendReactor {
        target: String,
    },
    /// **M14 audit pass 3 (GAP-M9-01)**: M9 spec § "ObjectiveKind enum"
    /// lists `DefendActor { actor_id, until_tick }` as the generic
    /// command-core / Bunker-Defense surface (forward-compat for M25+).
    /// Distinct from `DefendReactor` (which is the M2.5 reactor-specific
    /// specialization keyed by reactor name); `DefendActor` is keyed by
    /// actor id + `until_tick` deadline.
    ///
    /// **M14 audit pass 4 (Finding 4)**: schema-code drift — fields
    /// match published `cf-mission/v1/ObjectiveDefendActor` schema:
    ///   - `actor_id: String` (matches schema type "string")
    ///   - `until_tick: Option<u64>` (schema marks as optional; falls
    ///     back to mission's time_limit_ticks when absent)
    ///   - `loss_on_destroyed: bool` (schema default true)
    ///   - `tutorial_safety: bool` (schema default false)
    DefendActor {
        actor_id: String,
        #[serde(default)]
        until_tick: Option<u64>,
        #[serde(default = "default_loss_on_destroyed")]
        loss_on_destroyed: bool,
        #[serde(default)]
        tutorial_safety: bool,
    },
    /// **M2 re-audit (2026-05-13)**: spec literal — "ObjectiveKind enum:
    /// ReachZone, KillActor, SurviveTimer, DefendActor, EscortActor". The
    /// variant completes when `current_tick - started_at_tick >= survive_ticks`
    /// AND the actor is still alive.
    SurviveTimer {
        survive_ticks: u64,
    },
    /// **M2 re-audit (2026-05-13)**: escort `target` actor until they
    /// reach `destination` AABB. Fails if `target` dies during transit.
    EscortActor {
        target: u64,
        destination_min: [f32; 2],
        destination_max: [f32; 2],
    },
}

impl ObjectiveKind {
    pub fn category(&self) -> &'static str {
        match self {
            ObjectiveKind::BreachBarrier { .. } => "breach_barrier",
            ObjectiveKind::NeutralizeActor { .. } => "neutralize_actor",
            ObjectiveKind::ReachZone { .. } => "reach_zone",
            ObjectiveKind::DefendReactor { .. } => "defend_reactor",
            ObjectiveKind::DefendActor { .. } => "defend_actor",
            ObjectiveKind::SurviveTimer { .. } => "survive_timer",
            ObjectiveKind::EscortActor { .. } => "escort_actor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

impl Default for ObjectiveStatus {
    fn default() -> Self {
        ObjectiveStatus::Pending
    }
}

impl ObjectiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectiveStatus::Pending => "pending",
            ObjectiveStatus::Active => "active",
            ObjectiveStatus::Completed => "completed",
            ObjectiveStatus::Failed => "failed",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, ObjectiveStatus::Completed | ObjectiveStatus::Failed)
    }
}

pub(crate) fn default_loss_on_destroyed() -> bool {
    true
}
