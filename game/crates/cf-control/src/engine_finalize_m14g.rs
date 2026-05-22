//! Run finalize + later m14g accessor methods.
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
    pub fn record_run_finished(&self, exit_code: i32) {
        // Always emit one final `determinism.sim_checksum` so every bundle has at least one
        // checksum and `summary.json.final_sim_checksum` is never null on a valid run.
        // (Acceptance fix M2 from the M0 review.)
        self.emit_final_checksum();
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let run_aborted = state.run_aborted;
        drop(state);
        // **M14 audit pass 2 (GAP-M4-02 HIGH fix)**: spec § Expected
        // outcome + system events lists three outcomes — clean, panic,
        // abort. Previously hardcoded clean/panic only; aborted runs
        // surfaced as clean. Now: exit_code != 0 → panic; else if
        // run_aborted → abort; else → clean.
        let outcome = if exit_code != 0 {
            "panic"
        } else if run_aborted {
            "abort"
        } else {
            "clean"
        };
        // **M4 § Expected outcome + system events**: spec literal payload
        // is `{ outcome, ticks_run, wall_seconds, final_sim_checksum }`.
        // ticks_run is the last advanced tick; wall_seconds comes from
        // the engine's started_instant; final_sim_checksum is the latest
        // emitted determinism.sim_checksum.
        let wall_seconds = self.started_instant.elapsed().as_secs_f64();
        let final_sim_checksum = self.recorder.final_checksum_hex().unwrap_or_default();
        self.recorder.record(
            tick,
            sim_time_ms,
            "system",
            "run_finished",
            json!({
                "outcome": outcome,
                "exit_code": exit_code,
                "ticks_run": tick.0,
                "wall_seconds": wall_seconds,
                "final_sim_checksum": final_sim_checksum,
            }),
            None,
        );
    }

    /// Emit one `determinism.sim_checksum` event at the current tick, regardless of cadence.
    /// Idempotent within a tick (we still always emit; the recorder will give it a unique seq).
    pub fn emit_final_checksum(&self) {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let actor_bytes = build_checksum_bytes(&state);
        let cs = sim_state_v1(tick, &state.rng, &actor_bytes);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "determinism",
            "sim_checksum",
            json!({
                "checksum_hex": cs.to_hex(),
                "algorithm": CHECKSUM_ALGORITHM,
                "scope": CHECKSUM_SCOPE,
                "cadence_ticks": self.config.checksum_cadence_ticks,
                "tick_rate_hz": self.config.tick_rate_hz,
                "seed": self.config.seed,
                "kind": "final",
            }),
            None,
        );
    }

    pub fn current_tick(&self) -> Tick {
        self.state.read().expect("engine state poisoned").clock.tick()
    }

    /// **M14G test helper**: read the per-engine wound-aging pass
    /// invocation counter (VAL-M14G-046).
    pub fn m14g_wound_aging_invocations(&self) -> u64 {
        self.state
            .read()
            .map(|s| s.m14g_wound_aging_invocations)
            .unwrap_or(0)
    }

    /// **M14G test helper**: append a typed wound to an actor's
    /// `m14g_wound_list`. Returns the allocated wound id.
    pub fn m14g_inject_wound(
        &self,
        actor_id: u64,
        kind: cf_wound::WoundKind,
        zone: &str,
        severity: f32,
    ) -> Option<cf_wound::WoundId> {
        let mut s = self.state.write().ok()?;
        let sim = s.actor_state.as_mut()?;
        let actor = sim.world.actors.get_mut(&cf_actor::ActorId(actor_id))?;
        Some(actor.m14g_wound_list.push(
            cf_wound::registry::ZoneId::from(zone),
            cf_wound::Wound::new(
                cf_wound::WoundId(0),
                kind,
                severity,
                cf_wound::registry::ZoneId::from(zone),
            ),
        ))
    }

    /// **M14G test helper**: latest computed checksum hex over the
    /// current engine state — exercises `build_checksum_bytes` directly
    /// without depending on the periodic `determinism.sim_checksum`
    /// event. Used by save/load round-trip + determinism tests.
    pub fn m14g_compute_checksum_hex(&self) -> String {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let actor_bytes = build_checksum_bytes(&state);
        let cs = sim_state_v1(tick, &state.rng, &actor_bytes);
        cs.to_hex()
    }

    /// **M14G test helper**: read an actor's wound list (cloned).
    pub fn m14g_actor_wound_list(&self, actor_id: u64) -> Option<cf_wound::ActorWoundList> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&cf_actor::ActorId(actor_id))?;
        Some(actor.m14g_wound_list.clone())
    }

    /// **M14G test helper**: overwrite an actor's wound list (used by
    /// save/load round-trip tests).
    pub fn m14g_set_actor_wound_list(&self, actor_id: u64, list: cf_wound::ActorWoundList) -> bool {
        let mut s = match self.state.write() {
            Ok(s) => s,
            Err(_) => return false,
        };
        if let Some(sim) = s.actor_state.as_mut() {
            if let Some(actor) = sim.world.actors.get_mut(&cf_actor::ActorId(actor_id)) {
                actor.m14g_wound_list = list;
                return true;
            }
        }
        false
    }

    /// **M14G § VAL-M14G-023 test helper**: dispatch a one-shot
    /// `MeleeShoulderCheck` via the M6 dispatch path so the engine's
    /// melee-resolve code (including the blunt-face dental-damage emit)
    /// fires exactly the same way it would for a cfctl-driven hit.
    /// Returns `true` when the dispatch was Accepted by the engine
    /// (player + target present + actions allowed).
    pub fn m14g_dispatch_melee_shoulder_check(&self) -> bool {
        let tick = Tick(self.current_tick.load(std::sync::atomic::Ordering::Relaxed));
        let sim_time_ms = tick.0 as f64 * (1000.0 / f64::from(self.config.tick_rate_hz.max(1)));
        let state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let result = self.dispatch_m6_action(
            crate::m6_actions::M6Action::MeleeShoulderCheck,
            cf_actor::IntentSource::Cfctl,
            tick,
            sim_time_ms,
            state,
        );
        matches!(result.status, crate::state::ControlEnvelopeStatus::Accepted)
    }

}
