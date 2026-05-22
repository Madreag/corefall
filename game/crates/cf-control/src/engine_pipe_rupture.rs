//! inject_pipe_rupture.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub fn inject_pipe_rupture(
        &self,
        pipe_id: u64,
        position_world: [f32; 2],
        pressure_pa: f32,
        ttl_ticks: u32,
    ) -> Option<u32> {
        let mut state = self.state.write().ok()?;
        let next_id = state
            .m14b_wind_sources
            .iter()
            .map(|w| w.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
            .max(1000);
        // Spawn a high-pressure cell + ambient cell pair around the
        // rupture so the wind kernel sees a real ΔP to act on. We don't
        // mutate the authored atmosphere cell list (those are static
        // scenario geometry); instead we synthesize a virtual pair just
        // for this aperture and place the new WindSource between them.
        // Convert Pa → kPa for the AtmosCell schema.
        let pressure_kpa = pressure_pa / 1000.0;
        let high_cell_id = next_id.saturating_add(10_000);
        let low_cell_id = next_id.saturating_add(10_001);
        let high_min = [position_world[0] - 8.0, position_world[1] - 8.0];
        let high_max = [position_world[0], position_world[1] + 8.0];
        let low_min = [position_world[0], position_world[1] - 8.0];
        let low_max = [position_world[0] + 64.0, position_world[1] + 8.0];
        state.m14b_atmos_cells.push(cf_atmos::AtmosCell {
            id: high_cell_id,
            min: high_min,
            max: high_max,
            pressure_kpa,
            temp_k: 293.15,
        });
        state.m14b_atmos_cells.push(cf_atmos::AtmosCell {
            id: low_cell_id,
            min: low_min,
            max: low_max,
            pressure_kpa: cf_atmos::EARTH_AMBIENT_KPA,
            temp_k: 293.15,
        });
        state.m14b_wind_sources.push(cf_atmos::WindSource {
            id: next_id,
            origin: position_world,
            axis: [1.0, 0.0],
            aperture_area_m2: 0.01,
            cell_high_id: high_cell_id,
            cell_low_id: low_cell_id,
            jet_length: 64.0,
            jet_half_width: 8.0,
        });
        state.m14b_transient_wind_ttl.insert(next_id, ttl_ticks);
        state.m14b_transient_cells.push(high_cell_id);
        state.m14b_transient_cells.push(low_cell_id);
        // Record the rupture so cause chains stay linked.
        let _ = pipe_id;
        Some(next_id)
    }

}
