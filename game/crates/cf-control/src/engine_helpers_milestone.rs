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
use crate::engine_helpers::{
    MILESTONE_INDEX_M0, MILESTONE_INDEX_M1, MILESTONE_INDEX_M1_5, MILESTONE_INDEX_M2,
    MILESTONE_INDEX_M3A, MILESTONE_INDEX_UNKNOWN,
};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

pub(crate) fn milestone_order_index(milestone: &str) -> u32 {
    match milestone.trim().to_lowercase().as_str() {
        "" | "m0" => MILESTONE_INDEX_M0,
        "m1" => MILESTONE_INDEX_M1,
        "m1.5" => MILESTONE_INDEX_M1_5,
        "m2" => MILESTONE_INDEX_M2,
        "m2.5" => 4,
        "m3a" => MILESTONE_INDEX_M3A,
        "m3b" => 6,
        "m4a" => 7,
        "m4b" => 8,
        "m5" => 9,
        "m5.5" => 10,
        "m5.5.5" => 11,
        "m5.6" => 12,
        "m5.7" => 13,
        "m5.8" => 14,
        "m5.9" => 15,
        "m5.9.5" => 16,
        "m5.10" => 17,
        "m6" => 18,
        "m6.5" => 19,
        "m6.6" => 20,
        "m7" => 21,
        "m7.5" => 22,
        "m7.7" => 23,
        "m8" => 24,
        "m8.5" => 25,
        "m8.6" => 26,
        "m9" => 27,
        "m9.5" => 28,
        "m10" => 29,
        "m11" => 30,
        "m12" => 31,
        _ => MILESTONE_INDEX_UNKNOWN,
    }
}

pub(crate) fn prototype_slice_for_milestone(milestone: &str) -> String {
    let normalized = milestone.trim().to_lowercase();
    if normalized.is_empty() {
        return "M0".to_string();
    }
    // `format!("M{rest}")` produced lowercase letter suffixes (`m3a` → `M3a`)
    // because `rest` retained the lowercased form from `normalized`. Letter-
    // suffixed milestones (M3A/M3B/M4A/M4B) must produce uppercase suffixes
    // to match the canonical roadmap naming + the source-truthful evidence
    // contract in AGENTS.md (run_manifest.json.prototype_slice ↔ roadmap id).
    if let Some(rest) = normalized.strip_prefix('m') {
        return format!("M{}", rest.to_uppercase());
    }
    normalized.to_uppercase()
}

/// Per-milestone "what to do next" line written into `summary.json.next_actions`.
/// Stale "Proceed to M1 task cards" boilerplate masqueraded as M0 metadata in
/// every bundle through M2.5; the canonical roadmap (Build Points table) is the
/// source of truth and we pin the next milestone here so an offline reviewer
/// can read the bundle and immediately see what the implementer was supposed
/// to ship next.
pub(crate) fn next_actions_for_milestone(milestone: &str) -> Vec<String> {
    let normalized = milestone.trim().to_lowercase();
    let next = match normalized.as_str() {
        "" | "m0" => "Proceed to M1 task cards in spec/native-implementation-backlog.",
        "m1" => "Proceed to M1.5 (Micro Breach Fun Slice) per spec/prototype-roadmap.md#BP1.",
        "m1.5" => "Proceed to BP2 (M2 + M2.5 + M3A) per spec/prototype-roadmap.md#BP2.",
        "m2" => "Proceed to M2.5 (Micro Reactor Defense Fun Slice) per spec/prototype-roadmap.md#BP2.",
        "m2.5" => "Proceed to M3A (Event Recorder Core) per spec/prototype-roadmap.md#BP2.",
        "m3a" => "Proceed to BP3 (M3B + M4A + M5) per spec/prototype-roadmap.md#BP3.",
        "m3b" => "Proceed to M4A (Readability And ACC-A Floor) per spec/prototype-roadmap.md#BP3.",
        "m4a" => "Proceed to M5 (Equipment, Chassis, And Damage Grammar) per spec/prototype-roadmap.md#BP3.",
        "m5" => "Proceed to BP4 (M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8) per spec/prototype-roadmap.md#BP4.",
        _ => "Proceed to the next assigned milestone per spec/prototype-roadmap.md.",
    };
    vec![next.to_string()]
}

/// Per-milestone notes-addendum prose written into `notes.md` after the
/// scenario-author rows (Good/Bad/Meh/Evidence). The historical
/// `m0_notes_addendum` baked the M0 staging story ("M2/M3 will append terrain
/// bytes; all without bumping the suffix") into every bundle, which became
/// flat-out wrong once M2 / M2.5 / M3A landed. This helper returns the
/// up-to-date DR-002 + DR-012 lock prose AND the milestone's own pinned
/// contract addendum (e.g. material schema for M2+, expected-outcome contract
/// for M3A+).
pub(crate) fn notes_addendum_for_milestone(milestone: &str) -> String {
    let normalized = milestone.trim().to_lowercase();
    // ALL 12 event categories ship at every milestone is wrong (M0 only ships
    // system / control / determinism; terrain / material / mission / ai are
    // M1.5+; snapshot is M3A+). Build the per-milestone category list so the
    // notes addendum reflects what actually fired in this run, not the union
    // across the whole roadmap. Layer is append-only: each milestone inherits
    // every prior category.
    //
    // arms (which silently broke for M3B / M4A / M4B / M6+ that weren't
    // enumerated) to an ordering-based comparison via `milestone_order_index`.
    // The order index is the canonical roadmap progression and any new
    // milestone is added in one place rather than scattered across 4 match
    // statements that each had to be kept in sync.
    let idx = milestone_order_index(&normalized);
    let mut categories: Vec<&'static str> = vec!["system", "control", "determinism"];
    if idx >= MILESTONE_INDEX_M1 {
        categories.extend(["actor", "combat", "equipment", "input"]);
    }
    if idx >= MILESTONE_INDEX_M1_5 {
        categories.extend(["ai", "mission", "terrain"]);
    }
    if idx >= MILESTONE_INDEX_M2 {
        categories.push("material");
    }
    if idx >= MILESTONE_INDEX_M3A {
        categories.push("snapshot");
    }
    let categories_inline = categories
        .iter()
        .map(|c| format!("`{c}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut s = String::new();
    s.push_str("## DR-002 schema lock\n\n");
    s.push_str("- Event envelope: `{schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload, parent_event_id?, dropped_count?}`.\n");
    s.push_str(&format!(
        "- Categories shipped through this milestone: {categories_inline}. Future categories layer in additively without breaking v1 envelope readers.\n"
    ));
    s.push_str("- Checksum: `algorithm=blake3`, `scope=sim_state_v1`. Layout is append-only: M0 (`tick_counter || rng_state_bytes`) || M1 (actor / inventory / projectile bytes) || M1.5 (breach + guards + mission bytes) || M2 (chunked-terrain bytes) || M2.5 (reactor-world bytes). Layout-breaking bumps go to `_v2`.\n");
    s.push_str("- Manifest extensions: `checksum.{algorithm,scope,cadence_ticks}`, `settings:{...}` block, `expected_outcome:{clean|panic|abort}` (M3A).\n");
    s.push_str("- Summary extensions: `final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`, `artifacts.items[]` populated from `captures/` when present (M2+).\n");
    s.push_str("- M3A picks up headless replay verification: `cf-headless replay <bundle> --scenario-path <path>` reconstructs commands from `control.command_accepted` and asserts the cadence checksums tick-for-tick.\n");
    s.push_str("\n## DR-012 floor lock\n\n");
    s.push_str("- Six accessibility flags wired into `cf-control::Settings` and `run_manifest.json.settings`.\n");
    s.push_str("- Settings can be live-updated via `act.settings.set` and re-read via `observe.settings`.\n");
    s.push_str(
        "- Localization deferred to M4 — the discipline rule (no baked English-only player-facing strings) applies.\n",
    );
    // material system shape is. Every M2+ bundle that has material events
    // in events.jsonl benefits from seeing it, including milestones that
    // RUN ON TOP OF chunked terrain (M3B replay viewer, M4A readability)
    // and milestones that EXTEND it (M5.5 collision + materials, M5.6
    // material kernel, M6.6 AI material competence, M7.5 base atmospherics,
    // M8.5 material lab, M8.6 mining + refining).
    //
    // allowlist that stopped at M5.10 — when M6.6 / M7.5 / M8.5 / M8.6
    // (all of which clearly extend or work with materials) ship, they
    // would have silently missed the addendum. The fix matches the
    // category-layering pattern: `idx >= MILESTONE_INDEX_M2` so every
    // milestone past M2 in roadmap order inherits the material reference.
    // Unknown milestones map to MILESTONE_INDEX_UNKNOWN (post-M12) so
    // future milestones default to including the addendum.
    if idx >= MILESTONE_INDEX_M2 {
        s.push_str("\n## DR-007 launch material set\n\n");
        s.push_str("- 8 launch materials (ids 0..7): `air`, `dirt`, `concrete`, `metal_nohook`, `hazard`, `loose_fill`, `repair_fill`, `anchor`. `material_schema_version=cf-terrain-launch-v1`.\n");
        s.push_str("- Per-material affordances cover solid/diggable/hardness/anchorable/hazard/path_cost/overlay_rgba/refusal_reason.\n");
    }
    s
}

