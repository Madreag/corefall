//! refresh_hud_chassis_banners + refresh_hud_caches.
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
    pub(crate) fn refresh_hud_chassis_banners(&self, state: &mut EngineMutable, tick: Tick) {
        let now_tick = tick.0;
        let player_id = state.player_actor;
        let Some(sim) = state.actor_state.as_ref() else { return };
        let Some(pid) = player_id else { return };
        let Some(actor) = sim.world.actors.get(&pid) else {
            return;
        };
        // has no chassis yet, but the HUD must still surface the
        // "Boarding..." banner. Raise it BEFORE the chassis early-return
        // below so the banner fires regardless of chassis presence.
        if actor.boarding_ticks_remaining > 0 {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "boarding".to_string(),
                    severity: "info".to_string(),
                    label: "Boarding...".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.boarding".to_string(),
                },
            );
        }
        let Some(chassis) = actor.chassis.as_ref() else { return };
        let prev_stage = state.hud_last_chassis_stage;
        let prev_pilot = state.hud_last_pilot_state;
        let cur_stage = chassis.stage;
        let cur_pilot = chassis.pilot_state;
        // Stage transition banner.
        if Some(cur_stage) != prev_stage {
            if let Some(banner) = chassis_stage_banner(cur_stage, now_tick) {
                push_banner(&mut state.hud_banners, banner);
            }
        }
        // banner fires when the chassis enters the eject stage but the pilot
        // is still bound (window-open phase). The existing `eject_active`
        // banner covers the ejecting state itself.
        if matches!(cur_stage, cf_chassis::ChassisStage::Eject)
            && matches!(
                cur_pilot,
                cf_chassis::PilotState::Bound | cf_chassis::PilotState::Injured
            )
        {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "eject_window_open".to_string(),
                    severity: "critical".to_string(),
                    label: "EJECT_WINDOW_OPEN".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.eject_window_open".to_string(),
                },
            );
        }
        // Pilot eject banner (during the active eject window).
        if matches!(cur_pilot, cf_chassis::PilotState::Ejecting) {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "eject_active".to_string(),
                    severity: "critical".to_string(),
                    label: format!("EJECTING — {} TICKS", chassis.eject_window.ticks_remaining),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 30),
                    accessibility_id: "hud.banner.eject_active".to_string(),
                },
            );
        }
        // player IS the brain AND HP < 30%.
        if actor.is_brain && actor.hp_max > 0.0 && actor.hp / actor.hp_max < 0.3 {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "brain_at_risk".to_string(),
                    severity: "critical".to_string(),
                    label: "BRAIN_AT_RISK".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.brain_at_risk".to_string(),
                },
            );
        }
        if chassis.camera_anchor == cf_chassis::CameraAnchor::Cockpit {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "cockpit_anchor".to_string(),
                    severity: "info".to_string(),
                    label: "COCKPIT".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.cockpit_anchor".to_string(),
                },
            );
        }
        // "Boarding..." / "Exiting...".
        if chassis.boarding_ticks_remaining > 0 {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "boarding".to_string(),
                    severity: "info".to_string(),
                    label: "Boarding...".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.boarding".to_string(),
                },
            );
        }
        if chassis.disembarking_ticks_remaining > 0 {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "disembarking".to_string(),
                    severity: "info".to_string(),
                    label: "Exiting...".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 60),
                    accessibility_id: "hud.banner.disembarking".to_string(),
                },
            );
        }
        let destroyed = chassis.destroyed_zones();
        let both_legs_lost =
            destroyed.contains(&cf_chassis::BodyZone::LegLeft) && destroyed.contains(&cf_chassis::BodyZone::LegRight);
        if both_legs_lost {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "crawling".to_string(),
                    severity: "critical".to_string(),
                    label: "CRAWLING — both legs lost".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                    accessibility_id: "hud.banner.crawling".to_string(),
                },
            );
        }
        let both_arms_lost =
            destroyed.contains(&cf_chassis::BodyZone::ArmLeft) && destroyed.contains(&cf_chassis::BodyZone::ArmRight);
        if both_arms_lost {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "disarmed".to_string(),
                    severity: "critical".to_string(),
                    label: "DISARMED — both arms lost".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                    accessibility_id: "hud.banner.disarmed".to_string(),
                },
            );
        }
        if Some(cur_pilot) != prev_pilot {
            if let Some(banner) = chassis_pilot_banner(cur_pilot, now_tick) {
                push_banner(&mut state.hud_banners, banner);
            }
        }
        // Weapon jam banner.
        if chassis.weapon_jammed {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "weapon_jammed".to_string(),
                    severity: "warning".to_string(),
                    label: "WEAPON JAMMED — CLEAR".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                    accessibility_id: "hud.banner.weapon_jammed".to_string(),
                },
            );
        }
        state.hud_last_chassis_stage = Some(cur_stage);
        state.hud_last_pilot_state = Some(cur_pilot);
    }

    /// M4A HUD cache refresh. Called once per drive_tick after every category's
    /// events have been emitted. Updates `hud_banners`, `hud_captions`, and the
    /// `hud_last_*` diffing cursors. The HUD + `cfctl observe` reads the cache
    /// directly during `snapshot()`.
    pub(crate) fn refresh_hud_caches(&self, state: &mut EngineMutable, tick: Tick, sim_time_ms: f64) {
        // Drain expired banners + captions. Collect evicted ids for M11
        // ux.banner_dismissed emission below.
        let now_tick = tick.0;
        let pre_banner_snapshot: Vec<(String, u64)> = state
            .hud_banners
            .iter()
            .map(|b| (b.id.clone(), b.raised_at_tick))
            .collect();
        state.hud_banners.retain(|b| match b.expires_at_tick {
            Some(exp) => now_tick < exp,
            None => true,
        });
        let post_banner_ids: std::collections::HashSet<String> =
            state.hud_banners.iter().map(|b| b.id.clone()).collect();
        let pre_caption_ids: std::collections::HashSet<String> =
            state.hud_captions.iter().map(|c| c.id.clone()).collect();
        state
            .hud_captions
            .retain(|c| now_tick.saturating_sub(c.raised_at_tick) < M4A_CAPTION_EXPIRY_TICKS);
        // M11 § DR-012: emit `ux.banner_dismissed` for each evicted banner.
        for (banner_id, raised_at) in &pre_banner_snapshot {
            if !post_banner_ids.contains(banner_id) {
                self.recorder.record_cosmetic(
                    tick,
                    sim_time_ms,
                    "ux",
                    "banner_dismissed",
                    json!({
                        "banner_id": banner_id,
                        "reason": "expired",
                        "raised_at_tick": raised_at,
                        "dismissed_at_tick": now_tick,
                    }),
                    None,
                );
            }
        }

        // Status-change banners. The previous tick's status is cached in
        // `hud_last_status`; raise a banner whenever the player's status
        // worsens (Stable -> Unstable / Unstable -> Downed / any -> Dead).
        if let Some(sim) = state.actor_state.as_ref() {
            // Snapshot the status diff out of the borrow so we can push to the
            // banner queue (which lives on the same `state` borrow).
            let mut player_dead = false;
            let mut player_downed = false;
            let mut player_unstable = false;
            let mut diffs: Vec<(ActorId, cf_actor::Status)> = Vec::new();
            for (id, actor) in &sim.world.actors {
                let prev = state.hud_last_status.get(id).copied();
                let cur = actor.status;
                if prev.is_some() && prev != Some(cur) {
                    diffs.push((*id, cur));
                    if Some(*id) == sim.world.player {
                        match cur {
                            cf_actor::Status::Dead | cf_actor::Status::Dying => player_dead = true,
                            cf_actor::Status::Downed => player_downed = true,
                            cf_actor::Status::Unstable => player_unstable = true,
                            cf_actor::Status::Stable | cf_actor::Status::Inactive => {}
                        }
                    }
                }
            }
            if player_dead {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "eject_now".to_string(),
                        severity: "critical".to_string(),
                        label: "EJECT NOW".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: None,
                        accessibility_id: "hud.banner.eject_now".to_string(),
                    },
                );
            } else if player_downed {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "armor_cracked".to_string(),
                        severity: "critical".to_string(),
                        label: "ARMOR CRACKED".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                        accessibility_id: "hud.banner.armor_cracked".to_string(),
                    },
                );
            } else if player_unstable {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "hp_low".to_string(),
                        severity: "warning".to_string(),
                        label: "HP LOW".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                        accessibility_id: "hud.banner.hp_low".to_string(),
                    },
                );
            }
            // Per-tick caption emission for status changes (audio-bound at BP6+;
            // the captions surface lands at M4A so the contract is testable).
            for (id, st) in diffs {
                let label = format!("actor {} → {}", id.0, st.as_str());
                push_caption(
                    &mut state.hud_captions,
                    crate::state::CaptionView {
                        id: format!("status_changed.{}", id.0),
                        label,
                        raised_at_tick: now_tick,
                        accessibility_id: format!("hud.caption.status_changed.{}", id.0),
                    },
                );
            }

            // AMMO OUT banner: triggered when the selected rifle hits 0/cap with no reload in progress.
            if let Some(player_id) = sim.world.player {
                if let Some(rifle) = sim.rifles.get(&player_id) {
                    if rifle.spec.mag_capacity > 0 && rifle.ammo_in_mag == 0 && rifle.reload_remaining_ticks == 0 {
                        push_banner_dedup(
                            &mut state.hud_banners,
                            crate::state::HudBannerView {
                                id: "ammo_out".to_string(),
                                severity: "warning".to_string(),
                                label: "AMMO OUT — RELOAD".to_string(),
                                raised_at_tick: now_tick,
                                expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                                accessibility_id: "hud.banner.ammo_out".to_string(),
                            },
                        );
                    }
                }
            }
        }

        // Mission resolution banner.
        let cur_mission_result = state.mission.as_ref().map(|m| match &m.result {
            cf_mission::MissionResult::Won => "won".to_string(),
            cf_mission::MissionResult::Lost { .. } => "lost".to_string(),
            cf_mission::MissionResult::InProgress => "in_progress".to_string(),
            cf_mission::MissionResult::Aborted => "aborted".to_string(),
        });
        if state.hud_last_mission_result != cur_mission_result {
            if let Some(result) = cur_mission_result.as_deref() {
                if result == "won" {
                    push_banner(
                        &mut state.hud_banners,
                        crate::state::HudBannerView {
                            id: "mission_won".to_string(),
                            severity: "info".to_string(),
                            label: "MISSION WON".to_string(),
                            raised_at_tick: now_tick,
                            expires_at_tick: None,
                            accessibility_id: "hud.banner.mission_won".to_string(),
                        },
                    );
                } else if result == "lost" {
                    push_banner(
                        &mut state.hud_banners,
                        crate::state::HudBannerView {
                            id: "mission_failed".to_string(),
                            severity: "critical".to_string(),
                            label: "MISSION FAILED".to_string(),
                            raised_at_tick: now_tick,
                            expires_at_tick: None,
                            accessibility_id: "hud.banner.mission_failed".to_string(),
                        },
                    );
                }
            }
            state.hud_last_mission_result = cur_mission_result;
        }

        // Refresh hud_last_status for next tick.
        state.hud_last_status.clear();
        if let Some(sim) = state.actor_state.as_ref() {
            for (id, actor) in &sim.world.actors {
                state.hud_last_status.insert(*id, actor.status);
            }
        }
        // M11 § DR-012: emit `ux.captions_shown` for each newly-surfaced
        // caption (captions whose id was not in the pre-snapshot). The
        // verbosity_mode mirrors the caption-buffer policy (standard at M11).
        let caption_snapshot: Vec<(String, String, u64)> = state
            .hud_captions
            .iter()
            .map(|c| (c.id.clone(), c.label.clone(), c.raised_at_tick))
            .collect();
        for (cid, ctext, raised_at) in &caption_snapshot {
            if !pre_caption_ids.contains(cid) {
                self.recorder.record_cosmetic(
                    tick,
                    sim_time_ms,
                    "ux",
                    "captions_shown",
                    json!({
                        "caption_text": ctext,
                        "event_source_id": cid,
                        "verbosity_mode": "standard",
                        "category": "system",
                        "raised_at_tick": raised_at,
                    }),
                    None,
                );
            }
        }
    }

}
