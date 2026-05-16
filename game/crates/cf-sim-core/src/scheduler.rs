//! M8A § Parallel scheduler (canonical SimStage dependency graph).
//!
//! Per M8A spec § Notes for the implementer / Parallel scheduler:
//!
//! ```text
//! PreSim
//!   ↓
//! Input
//!   ↓
//! ActorPrePass
//!   ↓
//! [parallel: ActorTick, AITick, ProjectileTick]
//!   ↓
//! TerrainMutation
//!   ↓
//! [parallel: HazardContact, AnchorContact]
//!   ↓
//! ActorPostPass
//!   ↓
//! MissionTick
//!   ↓
//! RecorderMerge
//!   ↓
//! ChecksumEmit
//!   ↓
//! [parallel: PerfSampleEmit, GpuParticleStep]
//! ```
//!
//! M8A ships the canonical stage enum + the dependency graph; M9+ wires
//! Bevy's scheduler to drive each stage as a `SystemSet`.

use serde::{Deserialize, Serialize};

/// Canonical sim stage in the M8A dependency graph. Stages run in the
/// declared order; parallel groups are identified by
/// `SimStage::parallel_group_id()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SimStage {
    /// Tick counter advance, RNG re-seed, dirty buffer reset.
    PreSim,
    /// cfctl + keyboard intent → `ControlIntent` on player entity.
    Input,
    /// Status precompute from previous-tick HP, mass-scaled accel.
    ActorPrePass,
    /// Per-actor `apply_intent` + `step_kinematics` + `derive_status`.
    /// Parallel with `AITick` + `ProjectileTick`.
    ActorTick,
    /// Per-guard 5-layer thinking stack tick. Parallel with `ActorTick`
    /// + `ProjectileTick`.
    AITick,
    /// Per-projectile sweep + terrain penetration. Parallel with
    /// `ActorTick` + `AITick`.
    ProjectileTick,
    /// Single-writer chunk mutation (dig / blast / fill / settle).
    TerrainMutation,
    /// Per-hazard contact resolution. Parallel with `AnchorContact`.
    HazardContact,
    /// Per-anchor contact resolution. Parallel with `HazardContact`.
    AnchorContact,
    /// Apply hits, derive new status, latch dying / dwell.
    ActorPostPass,
    /// Mission director tick (objectives, timer, lifecycle).
    MissionTick,
    /// Per-thread shards → canonical event stream, sorted by event_id.
    RecorderMerge,
    /// Per-cadence determinism.sim_checksum emit.
    ChecksumEmit,
    /// Cosmetic perf sample emit. Parallel with `GpuParticleStep`.
    PerfSampleEmit,
    /// GPU compute particle step (cosmetic). Parallel with
    /// `PerfSampleEmit`.
    GpuParticleStep,
}

impl SimStage {
    /// Parallel-group identifier. Stages with the same group id may run
    /// concurrently when their query signatures permit.
    pub const fn parallel_group_id(self) -> u8 {
        match self {
            SimStage::ActorTick | SimStage::AITick | SimStage::ProjectileTick => 1,
            SimStage::HazardContact | SimStage::AnchorContact => 2,
            SimStage::PerfSampleEmit | SimStage::GpuParticleStep => 3,
            _ => 0,
        }
    }

    /// Declared linear order of all stages.
    pub const ALL: [SimStage; 15] = [
        SimStage::PreSim,
        SimStage::Input,
        SimStage::ActorPrePass,
        SimStage::ActorTick,
        SimStage::AITick,
        SimStage::ProjectileTick,
        SimStage::TerrainMutation,
        SimStage::HazardContact,
        SimStage::AnchorContact,
        SimStage::ActorPostPass,
        SimStage::MissionTick,
        SimStage::RecorderMerge,
        SimStage::ChecksumEmit,
        SimStage::PerfSampleEmit,
        SimStage::GpuParticleStep,
    ];
}

/// Dependency-graph descriptor. M9+ engine-host integration walks this
/// at engine init to register each stage with Bevy's scheduler.
#[derive(Debug, Clone, Copy)]
pub struct StageDep {
    pub stage: SimStage,
    pub depends_on: &'static [SimStage],
}

pub const STAGE_DEPS: &[StageDep] = &[
    StageDep {
        stage: SimStage::PreSim,
        depends_on: &[],
    },
    StageDep {
        stage: SimStage::Input,
        depends_on: &[SimStage::PreSim],
    },
    StageDep {
        stage: SimStage::ActorPrePass,
        depends_on: &[SimStage::Input],
    },
    StageDep {
        stage: SimStage::ActorTick,
        depends_on: &[SimStage::ActorPrePass],
    },
    StageDep {
        stage: SimStage::AITick,
        depends_on: &[SimStage::ActorPrePass],
    },
    StageDep {
        stage: SimStage::ProjectileTick,
        depends_on: &[SimStage::ActorPrePass],
    },
    StageDep {
        stage: SimStage::TerrainMutation,
        depends_on: &[SimStage::ActorTick, SimStage::AITick, SimStage::ProjectileTick],
    },
    StageDep {
        stage: SimStage::HazardContact,
        depends_on: &[SimStage::TerrainMutation],
    },
    StageDep {
        stage: SimStage::AnchorContact,
        depends_on: &[SimStage::TerrainMutation],
    },
    StageDep {
        stage: SimStage::ActorPostPass,
        depends_on: &[SimStage::HazardContact, SimStage::AnchorContact],
    },
    StageDep {
        stage: SimStage::MissionTick,
        depends_on: &[SimStage::ActorPostPass],
    },
    StageDep {
        stage: SimStage::RecorderMerge,
        depends_on: &[SimStage::MissionTick],
    },
    StageDep {
        stage: SimStage::ChecksumEmit,
        depends_on: &[SimStage::RecorderMerge],
    },
    StageDep {
        stage: SimStage::PerfSampleEmit,
        depends_on: &[SimStage::ChecksumEmit],
    },
    StageDep {
        stage: SimStage::GpuParticleStep,
        depends_on: &[SimStage::ChecksumEmit],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_stages_appear_in_stage_deps() {
        for stage in &SimStage::ALL {
            assert!(STAGE_DEPS.iter().any(|d| d.stage == *stage));
        }
    }

    #[test]
    fn parallel_group_assignments() {
        assert_eq!(SimStage::ActorTick.parallel_group_id(), 1);
        assert_eq!(SimStage::AITick.parallel_group_id(), 1);
        assert_eq!(SimStage::ProjectileTick.parallel_group_id(), 1);
        assert_eq!(SimStage::HazardContact.parallel_group_id(), 2);
        assert_eq!(SimStage::AnchorContact.parallel_group_id(), 2);
        assert_eq!(SimStage::PreSim.parallel_group_id(), 0);
    }
}
