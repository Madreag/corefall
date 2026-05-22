//! Free helper fns + types extracted from engine.rs.
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

/// Build the JsonSchema-friendly mission view used by the observe envelope.
/// M4A: build the module strip placeholder for a single actor. Until M5 lands
/// real chassis modules, the strip carries one weapon_mount slot derived from
/// the actor's selected rifle, plus three `not_present` placeholder slots
/// (jet, shield, sensor) so HUD + accessibility consumers can rely on stable
/// ids before the real implementation lands.
pub(crate) fn build_module_strip_view(
    rifle: Option<&cf_equipment::RifleState>,
    has_rifle_selected: bool,
) -> crate::state::ModuleStripView {
    let weapon_state = match (rifle, has_rifle_selected) {
        (Some(r), true) => {
            let reloading = r.reload_remaining_ticks > 0;
            let empty = r.spec.mag_capacity > 0 && r.ammo_in_mag == 0;
            if reloading || empty {
                "warning"
            } else {
                "nominal"
            }
        }
        _ => "not_present",
    };
    let weapon_label = match (rifle, has_rifle_selected) {
        (Some(r), true) => {
            if r.reload_remaining_ticks > 0 {
                "RELOADING".to_string()
            } else if r.spec.mag_capacity > 0 && r.ammo_in_mag == 0 {
                "EMPTY".to_string()
            } else {
                format!("READY {}/{}", r.ammo_in_mag, r.spec.mag_capacity)
            }
        }
        _ => "—".to_string(),
    };
    let modules = vec![
        crate::state::ModuleStateView {
            id: "weapon_mount".to_string(),
            label: weapon_label,
            state: weapon_state.to_string(),
            kind: "weapon_mount".to_string(),
        },
        crate::state::ModuleStateView {
            id: "jet".to_string(),
            label: "JET N/A".to_string(),
            state: "not_present".to_string(),
            kind: "jet".to_string(),
        },
        crate::state::ModuleStateView {
            id: "shield".to_string(),
            label: "SHIELD N/A".to_string(),
            state: "not_present".to_string(),
            kind: "shield".to_string(),
        },
        crate::state::ModuleStateView {
            id: "sensor".to_string(),
            label: "SENSOR N/A".to_string(),
            state: "not_present".to_string(),
            kind: "sensor".to_string(),
        },
    ];
    crate::state::ModuleStripView {
        modules,
        placeholder: true,
    }
}

/// M4A: stable accessibility ids for every focusable HUD node, in z-order.
/// Single source: [`HUD_FOCUSABLE_NODES`]. Consumed by `cfctl ui` (M4B+),
/// `cf-e2e --verify-focus`, the live-WS acceptance tests, and cf-app's
/// keyboard focus traversal system.
pub(crate) fn hud_focusable_nodes() -> Vec<String> {
    HUD_FOCUSABLE_NODES.iter().map(|s| (*s).to_string()).collect()
}

/// sprite when `Settings.ai_debug == true`. Format mirrors the spec text
/// ("ALERT: heard_shot", "ENGAGED", "RELOADING", "STUCK: blocked"). The
/// label is also produced when ai_debug is disabled so the run bundle
/// is identical regardless of overlay state — cf-ui simply hides the
/// element when the flag is off.
pub(crate) fn ai_intent_label(guard: &cf_ai::ReactiveGuard) -> String {
    let state_label = match guard.state {
        cf_ai::GuardState::Idle => "IDLE",
        cf_ai::GuardState::Alert => "ALERT",
        cf_ai::GuardState::Engaged => "ENGAGED",
        cf_ai::GuardState::Retreating => "RETREATING",
        cf_ai::GuardState::Dying => "DYING",
        cf_ai::GuardState::Dead => "DEAD",
    };
    // "{STATE}: {REASON}" (e.g. "ALERT: heard shot", "ENGAGING",
    // "RELOADING", "STUCK: blocked"). Reason is the most recent
    // state-change cause. Fall back to the chosen-tactic vocabulary when
    // no transition has fired yet (e.g. tick 0 with no perception).
    if guard.reload_remaining_ticks > 0 {
        return format!("{state_label}: RELOADING");
    }
    if guard.stuck_recovery_latched {
        return format!("{state_label}: STUCK: blocked");
    }
    if let Some(cause) = &guard.last_state_change_cause {
        // Render reason in human-readable form (snake_case → space).
        let pretty = cause.replace('_', " ");
        return format!("{state_label}: {pretty}");
    }
    match guard.last_tactic {
        cf_ai::Tactic::Attack => format!("{state_label}: ATTACK"),
        cf_ai::Tactic::Search => format!("{state_label}: SEARCH"),
        cf_ai::Tactic::Reload => format!("{state_label}: RELOAD"),
        cf_ai::Tactic::AimSettle => format!("{state_label}: AIM"),
        cf_ai::Tactic::Hold => state_label.to_string(),
    }
}

pub(crate) fn build_mission_view(state: &cf_mission::MissionState, current_tick: u64) -> crate::state::MissionView {
    let view = cf_mission::MissionView::from_state(state, current_tick);
    let objectives = view
        .objectives
        .into_iter()
        .map(|o| crate::state::ObjectiveView {
            id: o.id,
            kind: o.kind,
            status: o.status,
            optional: o.optional,
            target_actor: o.target_actor,
            target_breach: o.target_breach,
            target_reactor: o.target_reactor,
            zone_min: o.zone_min,
            zone_max: o.zone_max,
        })
        .collect();
    crate::state::MissionView {
        result: view.result,
        loss_reason: view.loss_reason,
        elapsed_ticks: view.elapsed_ticks,
        time_limit_ticks: view.time_limit_ticks,
        ticks_remaining: view.ticks_remaining,
        active_objective: view.active_objective,
        objectives,
        last_event_tick: view.last_event_tick,
        last_event_label: view.last_event_label,
        show_me_why_event_id: view.show_me_why_event_id,
        show_replay_cta: view.show_replay_cta,
    }
}

/// Build the checksum bytes covering every M1.5 + BP2 sub-state. Layout is
/// append-only relative to M1 so the `sim_state_v1` suffix stays valid:
/// `(M0 prefix) || (M1 actor bytes) || (M1.5 breach + guards + mission) ||
/// (M2 chunked terrain) || (M2.5 reactor world)`.
pub(crate) fn parse_body_zone(s: &str) -> Option<cf_chassis::BodyZone> {
    match s {
        "head" => Some(cf_chassis::BodyZone::Head),
        "torso" => Some(cf_chassis::BodyZone::Torso),
        "arm_left" => Some(cf_chassis::BodyZone::ArmLeft),
        "arm_right" => Some(cf_chassis::BodyZone::ArmRight),
        "leg_left" => Some(cf_chassis::BodyZone::LegLeft),
        "leg_right" => Some(cf_chassis::BodyZone::LegRight),
        "backpack" => Some(cf_chassis::BodyZone::Backpack),
        _ => None,
    }
}

