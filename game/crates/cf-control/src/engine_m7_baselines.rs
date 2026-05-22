//! M7 + M7B baseline emitters.
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
    pub(crate) fn emit_m7_mission_director_baselines(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<&str>) {
        let parent = parent_event_id.map(|s| s.to_string());
        let (phase_baseline, boss_baseline) = match self.state.read() {
            Ok(s) => {
                let phase = s.m7_ai_world.phase.as_ref().map(|p| {
                    let event = cf_mission::PhaseChangedEvent {
                        from: p.current,
                        to: p.current,
                        tick: tick.0,
                        cause: "scenario_start".to_string(),
                    };
                    crate::m7_ai::phase_changed_payload(&event)
                });
                let boss = s.m7_ai_world.boss.as_ref().map(|b| {
                    let event = cf_mission::BossPhaseChangedEvent {
                        actor_id: b.actor_id,
                        from: b.current_phase,
                        to: b.current_phase,
                        hp_fraction: b.hp_fraction(),
                        tick: tick.0,
                    };
                    crate::m7_ai::boss_phase_changed_payload(&event)
                });
                (phase, boss)
            }
            Err(_) => return,
        };
        if let Some(payload) = phase_baseline {
            self.recorder
                .record(tick, sim_time_ms, "mission", "phase_changed", payload, parent.clone());
        }
        if let Some(payload) = boss_baseline {
            self.recorder
                .record(tick, sim_time_ms, "boss", "phase_changed", payload, parent);
        }
    }

    /// `ai.mood_changed`, and `ai.stress_threshold_crossed` for the initial
    /// state of every spawned bot + the seeded faction matrix. This gives
    /// each event family a deterministic production emission site at run
    /// start so replay viewers + the audit harness see the expected
    /// "scene-start" snapshot for personality / faction state.
    pub(crate) fn emit_m7b_personality_faction_baselines(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<&str>) {
        let parent = parent_event_id.map(|s| s.to_string());
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return,
        };
        let bot_ids: Vec<(cf_actor::ActorId, cf_ai::Archetype, cf_ai::PersonalityProfile)> = state
            .m7_ai_world
            .bots
            .iter()
            .map(|(id, bot)| (*id, bot.archetype, bot.personality.clone()))
            .collect();
        let factions = state.m7_ai_world.factions.clone();
        drop(state);
        for (actor_id, archetype, personality) in &bot_ids {
            // Personality baseline. Default traits come from the archetype's
            // canonical personality bundle (M7-A / cf-ai); modifier defaults
            // to Neutral until the player applies one via cfctl.
            let traits: Vec<cf_ai::PersonalityTrait> = personality.traits.clone();
            let payload = crate::m7_ai::personality_changed_payload(
                actor_id.0,
                &traits,
                Some(cf_priority::PersonalityModifier::Neutral),
                "scenario_start",
            );
            // Mirror archetype on the payload to keep it useful even with
            // empty traits.
            let mut p = payload;
            if let serde_json::Value::Object(ref mut m) = p {
                m.insert("archetype".to_string(), json!(archetype.as_str()));
            }
            self.recorder
                .record(tick, sim_time_ms, "ai", "personality_changed", p, parent.clone());
            // Mood baseline (delta=0 means "snapshot of current value").
            let mood = personality.mood;
            let payload = crate::m7_ai::mood_changed_payload(actor_id.0, 0.0, mood, "scenario_start_baseline");
            self.recorder
                .record(tick, sim_time_ms, "ai", "mood_changed", payload, parent.clone());
            // Stress threshold baseline — emit the current band so observers
            // know the starting state without inferring it.
            let stress = personality.stress;
            let threshold = if stress >= 75.0 {
                crate::m7_ai::StressThreshold::Broken
            } else if stress >= 50.0 {
                crate::m7_ai::StressThreshold::Depressed
            } else if stress >= 25.0 {
                crate::m7_ai::StressThreshold::Stressed
            } else {
                crate::m7_ai::StressThreshold::Calm
            };
            let payload = crate::m7_ai::stress_threshold_crossed_payload(actor_id.0, threshold, true, stress);
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "stress_threshold_crossed",
                payload,
                parent.clone(),
            );
        }
        // Faction baseline — emit one allegiance-changed event for each
        // ordered faction pair (only 3 unique combinations + 3 self-pairs;
        // skip self-pairs since allegiance(self,self)=100 is constant).
        let factions_vec = cf_ai::FactionId::ALL.to_vec();
        for a in &factions_vec {
            for b in &factions_vec {
                if a == b {
                    continue;
                }
                // Only emit one direction per pair (a < b ordinal) to keep
                // the snapshot deterministic + non-redundant.
                if a.ordinal() > b.ordinal() {
                    continue;
                }
                let value = factions.get(*a, *b);
                let payload = crate::m7_ai::faction_allegiance_changed_payload(*a, *b, 0, value, "scenario_start");
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ai",
                    "faction_allegiance_changed",
                    payload,
                    parent.clone(),
                );
            }
        }
    }

}
