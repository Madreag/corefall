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

/// pointer position. cf-app owns the actual hit-box geometry; this server-
/// side helper provides a deterministic mapping based on a normalized
/// 0..1 layout grid so cfctl `act.input.mouse_click` can target any node
/// by name without a render loop in the loop.
///
/// The mapping is COARSE on purpose: cfctl ui_assert + replay-side
/// rendering use this stub to confirm that the input round-trips through
/// the engine; precise pointer-to-pixel mapping lives in cf-app when the
/// pointer actually traverses the HUD.
pub(crate) fn resolve_hud_node_at(x: f32, y: f32) -> Option<String> {
    // Negative coords mean "off-screen" — emit empty target.
    if x < 0.0 || y < 0.0 {
        return None;
    }
    // The 12 HUD_FOCUSABLE_NODES occupy a 1×N vertical band on the left
    // edge of the screen by default. The exact pixel geometry is owned by
    // cf-ui's spawn_status_strip; this is the engine-side coarse map.
    let bucket = (y / 24.0).floor() as usize;
    if bucket < HUD_FOCUSABLE_NODES.len() {
        Some(HUD_FOCUSABLE_NODES[bucket].to_string())
    } else {
        None
    }
}

pub(crate) fn push_banner(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    queue.push_back(banner);
    while queue.len() > M4A_BANNER_BUFFER {
        queue.pop_front();
    }
}

pub(crate) fn push_banner_dedup(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    if queue.iter().any(|b| b.id == banner.id) {
        return;
    }
    push_banner(queue, banner);
}

pub(crate) fn push_caption(queue: &mut VecDeque<crate::state::CaptionView>, caption: crate::state::CaptionView) {
    queue.push_back(caption);
    while queue.len() > M4A_CAPTION_BUFFER {
        queue.pop_front();
    }
}

pub(crate) fn chassis_stage_banner(stage: cf_chassis::ChassisStage, now_tick: u64) -> Option<crate::state::HudBannerView> {
    let (id, severity, label) = match stage {
        cf_chassis::ChassisStage::Nominal => return None,
        cf_chassis::ChassisStage::Degraded => return None,
        cf_chassis::ChassisStage::ModuleWarning => ("chassis_module_warning", "warning", "MODULE WARNING"),
        cf_chassis::ChassisStage::ModuleFailed => ("chassis_module_failed", "warning", "MODULE FAILED"),
        cf_chassis::ChassisStage::WeaponJammed => return None, // handled separately
        cf_chassis::ChassisStage::ArmorCracked => ("chassis_armor_cracked", "critical", "ARMOR CRACKED"),
        cf_chassis::ChassisStage::Disabled => ("chassis_disabled", "critical", "CHASSIS DISABLED"),
        cf_chassis::ChassisStage::PilotInjured => ("chassis_pilot_injured", "critical", "PILOT INJURED"),
        cf_chassis::ChassisStage::Eject => ("chassis_eject_now", "critical", "EJECT NOW"),
        cf_chassis::ChassisStage::BailTooLate => ("chassis_bail_too_late", "critical", "BAILED TOO LATE"),
        cf_chassis::ChassisStage::Wreck => ("chassis_wreck", "critical", "CHASSIS WRECKED"),
        cf_chassis::ChassisStage::Gibbed => ("chassis_gibbed", "critical", "CHASSIS DESTROYED"),
    };
    Some(crate::state::HudBannerView {
        id: id.to_string(),
        severity: severity.to_string(),
        label: label.to_string(),
        raised_at_tick: now_tick,
        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS * 2),
        accessibility_id: format!("hud.banner.{id}"),
    })
}

pub(crate) fn chassis_pilot_banner(state: cf_chassis::PilotState, now_tick: u64) -> Option<crate::state::HudBannerView> {
    let (id, severity, label) = match state {
        cf_chassis::PilotState::Bound => return None,
        cf_chassis::PilotState::Injured => ("pilot_injured", "warning", "PILOT INJURED"),
        cf_chassis::PilotState::Ejecting => ("pilot_ejecting", "critical", "EJECTING"),
        cf_chassis::PilotState::Ejected => ("pilot_ejected", "info", "PILOT EJECTED"),
        cf_chassis::PilotState::Extracted => ("pilot_extracted", "info", "PILOT EXTRACTED"),
        cf_chassis::PilotState::BailedTooLate => ("pilot_bailed_too_late", "critical", "BAILED TOO LATE"),
        cf_chassis::PilotState::Lost => ("pilot_lost", "critical", "PILOT LOST"),
    };
    Some(crate::state::HudBannerView {
        id: id.to_string(),
        severity: severity.to_string(),
        label: label.to_string(),
        raised_at_tick: now_tick,
        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS * 2),
        accessibility_id: format!("hud.banner.{id}"),
    })
}

/// Cause label for `actor.actor_status_changed` events emitted from `step_one_actor`.
///
/// In M1 the only mutator inside `step_one_actor` that touches `actor.status` is
/// `actor.reset()` (called when the player issues `act.player.reset`). Damage-driven
/// transitions are emitted from a separate projectile-hit loop with cause
/// `projectile_hit`, never via this helper. Future milestones (M5 chassis ejection,
/// M5.6 hazard contact, etc.) MUST extend [`ActorTickOutcome`] with an explicit
/// cause discriminant rather than relying on a generic catch-all label here, so
/// the cause-chain stays semantically correct for replay analysis.
///
/// flag (latched by `cf-actor::sim` when an UNSTABLE actor takes
/// travel-impulse damage per CCCP `Actor.cpp:1199`).
pub(crate) fn status_change_cause(outcome: &ActorTickOutcome) -> &'static str {
    if outcome.travel_impulse_damage {
        "travel_impulse"
    } else if outcome.reset {
        "reset"
    } else {
        // Defensive fallback: if a future milestone introduces another
        // status-mutating path inside `step_one_actor` without extending
        // `ActorTickOutcome` with an explicit cause discriminant,
        // surfacing `unknown` makes the contract gap visible in the run
        // bundle so it can be caught and fixed.
        "unknown"
    }
}

