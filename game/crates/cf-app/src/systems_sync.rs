use bevy::{app::AppExit, prelude::*};

use cf_capture::{CaptureClock, CaptureConfig, CaptureKeyframeRequested};
use cf_control::EngineHandle;
use cf_render_2d::{
    asset_loader::{load_ledger_index, AssetIndex},
    ActorRenderState, BreachRender, CameraFollow, CameraShake, ChunkUpdate, ChunkedTerrainSnapshot, DebrisSpawnQueue,
    DebrisSpawnRequest, DigPreviewGhost, DigPreviewTarget, ExplosionState, ExtractionRender, HitStop,
    JuiceAccessibility, JuiceKind, JuicePulse, JuiceState, MuzzleFlashRender, OverlayMode, OverlayModeState,
    ReactorSprite, ReactorSpriteState, SparkEmitterState, EXPLOSION_DEBRIS_CAP_PER_HIT, SPARK_CAP_PER_HIT,
};
use cf_ui::{
    reactor_hp_bar::ArmorPipView, HudBanner, HudBodySilhouette, HudBreach, HudCaption, HudEnemy, HudMission, HudModule,
    HudModuleStrip, HudRifle, HudSettings, HudState, HudToolValidity, IntegrityBand, ReactorHpBarState,
    ReactorPressureLineState, TimerWarningsState, WARNING_THRESHOLDS,
};

use crate::app::resources::{
    AppRuntime, CaptureRecorderCursor, EngineHolder, RenderEffectsCursor, TerrainBridgeCursor,
};
use crate::headless::drain_pending_bundle;
use crate::input::futures_block_on;

pub(crate) fn sync_engine_tick_to_capture_clock(holder: Res<EngineHolder>, mut clock: ResMut<CaptureClock>) {
    clock.current_tick = holder.0.current_tick().0;
}

/// the cf-render-2d `AssetIndex` from the workspace's canonical
/// `content/asset_ledger/ledger.jsonl`. The replay viewer + in-game
/// death-recap modal call `AssetIndex.get(canonical_name)` to resolve
/// the PNG / SVG path for any tier-1 placeholder asset.
pub(crate) fn hydrate_asset_index_from_ledger(mut index: ResMut<AssetIndex>) {
    use std::path::PathBuf;
    let candidates: Vec<PathBuf> = {
        let mut v: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("CF_ASSET_LEDGER_PATH") {
            v.push(PathBuf::from(p));
        }
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Some(repo) = manifest_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
            v.push(repo.join("content").join("asset_ledger").join("ledger.jsonl"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            v.push(cwd.join("content").join("asset_ledger").join("ledger.jsonl"));
            if let Some(parent) = cwd.parent() {
                v.push(parent.join("content").join("asset_ledger").join("ledger.jsonl"));
            }
        }
        v
    };
    for path in &candidates {
        if !path.exists() {
            continue;
        }
        match load_ledger_index(path, &mut index) {
            Ok(n) => {
                tracing::info!(
                    target: "cf::asset_index",
                    "hydrated AssetIndex from {} ({} entries)",
                    path.display(),
                    n
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    target: "cf::asset_index",
                    "failed to hydrate AssetIndex from {}: {}",
                    path.display(),
                    e
                );
                return;
            }
        }
    }
    tracing::warn!(
        target: "cf::asset_index",
        "ledger.jsonl not found at any candidate path; AssetIndex left empty (M10 death-recap icons will fall back to symbolic placeholders)"
    );
}

pub(crate) fn sync_terrain_state_to_render(
    holder: Res<EngineHolder>,
    mut terrain_snapshot: ResMut<ChunkedTerrainSnapshot>,
    mut overlay_state: ResMut<OverlayModeState>,
    mut dig_ghost: ResMut<DigPreviewGhost>,
    mut cursor: ResMut<TerrainBridgeCursor>,
) {
    let tick_now = holder.0.current_tick().0;
    if cursor.initialized && cursor.last_tick == tick_now {
        return;
    }
    cursor.initialized = true;
    cursor.last_tick = tick_now;
    let snap = holder.0.terrain_render_snapshot();
    terrain_snapshot.active = snap.active;
    terrain_snapshot.anchor = snap.anchor;
    terrain_snapshot.updates.clear();
    for u in snap.dirty_updates {
        terrain_snapshot.updates.push(ChunkUpdate {
            cx: u.cx,
            cy: u.cy,
            dirty_rect: u.dirty_rect,
            pixels: u.pixels,
        });
    }
    overlay_state.mode = OverlayMode::parse_mode(snap.overlay_mode.as_str());
    let live_settings = holder.0.current_settings();
    dig_ghost.reduced_motion = live_settings.reduced_motion;
    dig_ghost.target = snap.dig_preview.map(|p| DigPreviewTarget {
        position: bevy::math::Vec2::new(p.position[0], p.position[1]),
        radius: p.radius,
        valid: p.valid,
        material_id: Some(p.material_id),
    });
}

/// (category, event_type) pairs that trigger a capture keyframe.
const CAPTURE_KEYFRAME_EVENT_TYPES: &[(&str, &str)] = &[
    ("mission", "objective_started"),
    ("mission", "objective_completed"),
    ("mission", "objective_failed"),
    ("mission", "mission_resolved"),
    ("terrain", "terrain_carved"),
    ("terrain", "tool_refused"),
    ("combat", "projectile_hit"),
    ("actor", "actor_status_changed"),
    ("equipment", "weapon_fired"),
    ("ai", "state_changed"),
    ("system", "panic"),
];

pub(crate) fn pump_recorder_events_into_capture_keyframes(
    holder: Res<EngineHolder>,
    config: Res<CaptureConfig>,
    mut cursor: ResMut<CaptureRecorderCursor>,
    mut writer: MessageWriter<CaptureKeyframeRequested>,
) {
    if !config.enabled || !config.event_keyframes {
        return;
    }
    let recorder = holder.0.recorder();
    let new_events = recorder.events_since(cursor.0);
    cursor.0 += new_events.len();
    for ev in new_events {
        if CAPTURE_KEYFRAME_EVENT_TYPES
            .iter()
            .any(|(cat, ty)| ev.category == *cat && ev.event_type == *ty)
        {
            let label = format!("{}::{}", ev.category, ev.event_type);
            writer.write(CaptureKeyframeRequested {
                tick: ev.tick,
                event_type: format!("{}.{}", ev.category, ev.event_type),
                label,
            });
        }
    }
}

/// since the last frame and translate them into render-layer effects
/// (CameraShake, HitStop, MuzzleFlash). Uses a per-frame cursor so each
/// event is consumed exactly once.
pub(crate) fn pump_recorder_events_into_render_effects(
    holder: Res<EngineHolder>,
    mut shake: ResMut<CameraShake>,
    mut hit_stop: ResMut<HitStop>,
    mut state: ResMut<ActorRenderState>,
    mut debris_queue: ResMut<DebrisSpawnQueue>,
    mut sparks: ResMut<SparkEmitterState>,
    mut explosion: ResMut<ExplosionState>,
    mut chem_flash: ResMut<cf_render_2d::ChemFlashState>,
    mut cursor: ResMut<RenderEffectsCursor>,
    juice_acc: Res<JuiceAccessibility>,
    mut juice_state: ResMut<JuiceState>,
) {
    let settings = futures_block_on(async { holder.0.settings_snapshot().await });
    shake.reduce_pct = settings.reduce_camera_shake_pct;
    let recorder = holder.0.recorder();
    let new_events = recorder.events_since(cursor.0);
    cursor.0 += new_events.len();
    for ev in new_events {
        match (ev.category.as_str(), ev.event_type.as_str()) {
            ("terrain", "terrain_pixel_dislodged") => {
                let pos_arr = ev.payload.get("pos").and_then(|v| v.as_array());
                let x = pos_arr
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let y = pos_arr
                    .and_then(|arr| arr.get(1))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let count = ev.payload.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let mat = ev
                    .payload
                    .get("spawn_material_id")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u16::try_from(n).ok())
                    .or_else(|| {
                        ev.payload
                            .get("source_material_id")
                            .and_then(|v| v.as_u64())
                            .and_then(|n| u16::try_from(n).ok())
                    })
                    .unwrap_or(cf_terrain::MATERIAL_LOOSE_FILL);
                debris_queue.pending.push_back(DebrisSpawnRequest {
                    pos: bevy::math::Vec2::new(x, y),
                    spawn_material: mat,
                    count,
                });
            }
            ("ux", "camera_punch_requested") => {
                let magnitude = ev.payload.get("magnitude").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                shake.magnitude_px = (shake.magnitude_px + magnitude * 0.05).clamp(0.0, 40.0);
            }
            ("ux", "hit_stop_requested") => {
                let dur_ms = ev.payload.get("duration_ms").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                hit_stop.remaining_ms = hit_stop.remaining_ms.max(dur_ms);
                let pulse = JuicePulse::new(JuiceKind::CriticalHitPunch, *juice_acc);
                juice_state.push("ux.critical_hit", pulse);
            }
            ("equipment", "weapon_swap_started") => {
                let pulse = JuicePulse::new(JuiceKind::WeaponSwapWhoosh, *juice_acc);
                juice_state.push("ux.weapon_swap", pulse);
            }
            ("equipment", "weapon_swap_completed") => {
                let pulse = JuicePulse::new(JuiceKind::ReloadCompletedDing, *juice_acc);
                juice_state.push("ux.reload_done", pulse);
            }
            ("equipment", "item_picked_up") => {
                let pulse = JuicePulse::new(JuiceKind::PickupGlow, *juice_acc);
                juice_state.push("ux.pickup", pulse);
            }
            ("equipment", "weapon_fired") => {
                let origin = ev.payload.get("muzzle_origin").and_then(|v| v.as_array()).map(|arr| {
                    let x = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    bevy::math::Vec2::new(x, y)
                });
                if let Some(o) = origin {
                    state.muzzle_flash = Some(MuzzleFlashRender {
                        origin: o,
                        remaining_ticks: 3,
                    });
                }
            }
            ("combat", "projectile_hit") => {
                let is_reactor = ev
                    .payload
                    .get("target_kind")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s == "reactor");
                if !is_reactor {
                    continue;
                }
                let pos = ev.payload.get("position").and_then(|v| v.as_array());
                let x = pos.and_then(|arr| arr.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.and_then(|arr| arr.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                sparks.spawn_burst([x, y], SPARK_CAP_PER_HIT, 180);
            }
            ("mission", "reactor_destroyed") => {
                let pos = ev.payload.get("position").and_then(|v| v.as_array());
                let x = pos.and_then(|arr| arr.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.and_then(|arr| arr.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                explosion.spawn([x, y], EXPLOSION_DEBRIS_CAP_PER_HIT, settings.reduce_camera_shake_pct);
                let shake_scale = (1.0 - settings.reduce_camera_shake_pct.clamp(0.0, 1.0)).max(0.0);
                shake.magnitude_px = (shake.magnitude_px + 18.0 * shake_scale).clamp(0.0, 40.0);
            }
            ("material", "violent_burst") => {
                let pos = ev.payload.get("pos").and_then(|v| v.as_array());
                let x = pos.and_then(|arr| arr.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.and_then(|arr| arr.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let energy = ev
                    .payload
                    .get("energy_release_j")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let color = ev
                    .payload
                    .get("flash_color_hex")
                    .and_then(|v| v.as_str())
                    .and_then(cf_render_2d::parse_flash_color_hex)
                    .unwrap_or([255, 255, 255, 255]);
                chem_flash.spawn([x, y], color, energy);
                let shake_scale = (1.0 - settings.reduce_camera_shake_pct.clamp(0.0, 1.0)).max(0.0);
                let mag = (energy.abs() / 200000.0).clamp(0.0, 6.0) * shake_scale;
                shake.magnitude_px = (shake.magnitude_px + mag).clamp(0.0, 40.0);
            }
            _ => {}
        }
    }
}

pub(crate) fn drive_engine_tick(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    if runtime.unpaced {
        return;
    }
    if holder.0.shutdown_requested() {
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        return;
    }
    let _ = holder.0.drive_tick();
    drain_pending_bundle(&holder.0);
    let cur = holder.0.current_tick().0;
    if runtime.duration_ticks > 0 && cur >= runtime.duration_ticks {
        runtime.last_announced_tick = cur;
    }
}

/// schedule (not FixedUpdate) so it isn't capped at `tick_rate_hz` real-time
/// firing.
pub(crate) fn drive_engine_tick_unpaced(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    if !runtime.unpaced {
        return;
    }
    if holder.0.shutdown_requested() {
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        return;
    }
    let max_ticks_this_frame = runtime.unpaced_max_ticks_per_frame.max(1);
    for _ in 0..max_ticks_this_frame {
        if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
            break;
        }
        if holder.0.drive_tick().is_none() {
            break;
        }
    }
    drain_pending_bundle(&holder.0);
    let cur = holder.0.current_tick().0;
    if runtime.duration_ticks > 0 && cur >= runtime.duration_ticks {
        runtime.last_announced_tick = cur;
    }
}

pub(crate) fn check_completion(
    holder: Res<EngineHolder>,
    runtime: Res<AppRuntime>,
    mut events: MessageWriter<AppExit>,
) {
    if holder.0.shutdown_requested() {
        drain_pending_bundle(&holder.0);
        events.write(AppExit::Success);
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        drain_pending_bundle(&holder.0);
        events.write(AppExit::Success);
    }
}

pub(crate) fn log_tick_progress(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    let cur = holder.0.current_tick().0;
    if cur >= runtime.last_announced_tick + 60 {
        tracing::debug!(target: "cf::app", tick = cur, "sim progressing");
        runtime.last_announced_tick = cur;
    }
}

/// M4A: rebuild the HUD module-strip placeholder from the player's filtered
/// rifle view.
pub(crate) fn build_hud_module_strip(rifle: Option<&cf_control::RifleHudView>) -> HudModuleStrip {
    let weapon_state = match rifle {
        Some(r) => {
            let reloading = r.reload_remaining_ticks > 0;
            let empty = r.capacity > 0 && r.ammo == 0;
            if reloading || empty {
                "warning"
            } else {
                "nominal"
            }
        }
        _ => "not_present",
    };
    let weapon_label = match rifle {
        Some(r) => {
            if r.reload_remaining_ticks > 0 {
                "RELOADING".to_string()
            } else if r.capacity > 0 && r.ammo == 0 {
                "EMPTY".to_string()
            } else {
                format!("READY {}/{}", r.ammo, r.capacity)
            }
        }
        _ => "—".to_string(),
    };
    HudModuleStrip {
        modules: vec![
            HudModule {
                id: "weapon_mount".into(),
                label: weapon_label,
                state: weapon_state.into(),
                kind: "weapon_mount".into(),
            },
            HudModule {
                id: "jet".into(),
                label: "JET N/A".into(),
                state: "not_present".into(),
                kind: "jet".into(),
            },
            HudModule {
                id: "shield".into(),
                label: "SHIELD N/A".into(),
                state: "not_present".into(),
                kind: "shield".into(),
            },
            HudModule {
                id: "sensor".into(),
                label: "SENSOR N/A".into(),
                state: "not_present".into(),
                kind: "sensor".into(),
            },
        ],
        placeholder: true,
    }
}

/// Copy the engine's actor world + rifle state into the Bevy render + HUD
/// resources every frame. The engine is the single source of truth; render +
/// HUD never own authoritative state.
pub(crate) fn sync_actor_state_to_render(
    holder: Res<EngineHolder>,
    mut render_state: ResMut<ActorRenderState>,
    mut hud_state: ResMut<HudState>,
    mut hud_settings: ResMut<HudSettings>,
    mut camera_follow: ResMut<CameraFollow>,
) {
    let snapshot = holder.0.actor_render_snapshot();
    let hud_caches = holder.0.hud_caches_snapshot();
    let live_settings = holder.0.current_settings();
    render_state.actors = snapshot.actors.clone();
    render_state.player_actor_id = snapshot.player_actor_id;
    render_state.region_width = holder.0.config().region_width;
    render_state.region_height = holder.0.config().region_height;
    render_state.region_anchor_x = holder.0.config().region_anchor_x;
    render_state.region_anchor_y = holder.0.config().region_anchor_y;
    render_state.floor_y = snapshot.floor_y;
    render_state.tick = snapshot.tick;

    let next_settings = HudSettings {
        ui_scale: live_settings.ui_scale,
        high_contrast: live_settings.high_contrast,
        captions: live_settings.captions,
        reduced_motion: live_settings.reduced_motion,
        reduced_shake: live_settings.reduced_shake,
        reduced_flash: live_settings.reduced_flash,
        reduced_g_force_blackout: live_settings.reduced_g_force_blackout,
        hold_to_confirm: live_settings.hold_to_confirm,
        hold_threshold_ms: live_settings.hold_threshold_ms,
        key_remap_enabled: live_settings.key_remap_enabled,
        focused_node: hud_caches.focused_node.clone(),
        ai_debug: live_settings.ai_debug,
        comic_style_overlay: live_settings.comic_style_overlay.as_str().to_string(),
        comic_death_recap: live_settings.comic_death_recap,
    };
    if (hud_settings.ui_scale - next_settings.ui_scale).abs() > f32::EPSILON
        || hud_settings.high_contrast != next_settings.high_contrast
        || hud_settings.captions != next_settings.captions
        || hud_settings.reduced_motion != next_settings.reduced_motion
        || hud_settings.reduced_shake != next_settings.reduced_shake
        || hud_settings.reduced_flash != next_settings.reduced_flash
        || hud_settings.hold_to_confirm != next_settings.hold_to_confirm
        || hud_settings.hold_threshold_ms != next_settings.hold_threshold_ms
        || hud_settings.key_remap_enabled != next_settings.key_remap_enabled
        || hud_settings.focused_node != next_settings.focused_node
        || hud_settings.ai_debug != next_settings.ai_debug
        || hud_settings.comic_style_overlay != next_settings.comic_style_overlay
        || hud_settings.comic_death_recap != next_settings.comic_death_recap
    {
        *hud_settings = next_settings;
    }

    hud_state.tick = snapshot.tick;
    hud_state.tick_rate_hz = holder.0.config().tick_rate_hz;
    hud_state.player = snapshot
        .player_actor_id
        .and_then(|id| snapshot.actors.iter().find(|a| a.id == id).cloned());
    hud_state.rifle = snapshot.player_rifle.as_ref().map(|r| HudRifle {
        ammo: r.ammo,
        capacity: r.capacity,
        fire_cooldown_ticks: r.fire_cooldown_ticks,
        reload_remaining_ticks: r.reload_remaining_ticks,
        reload_total_ticks: r.reload_total_ticks,
    });

    let (stance, silhouette) = match hud_state.player.as_ref() {
        Some(p) => (
            p.stance.clone(),
            HudBodySilhouette {
                head_hp_pct: p.body_silhouette.head_hp_pct,
                torso_hp_pct: p.body_silhouette.torso_hp_pct,
                arm_left_hp_pct: p.body_silhouette.arm_left_hp_pct,
                arm_right_hp_pct: p.body_silhouette.arm_right_hp_pct,
                leg_left_hp_pct: p.body_silhouette.leg_left_hp_pct,
                leg_right_hp_pct: p.body_silhouette.leg_right_hp_pct,
                placeholder: p.body_silhouette.placeholder,
            },
        ),
        None => (String::new(), HudBodySilhouette::default()),
    };
    hud_state.stance = stance;
    hud_state.body_silhouette = silhouette;
    hud_state.stability = hud_state.player.as_ref().map(|p| p.stability).unwrap_or(1.0);
    // M17 — per-origin resource bars + concussion vignette.
    hud_state.resources = match hud_state.player.as_ref() {
        Some(p) => cf_ui::HudResources {
            origin: p.m17.origin.clone(),
            blood: p.m17.blood,
            blood_max: p.m17.blood_max,
            oil: p.m17.oil,
            oil_max: p.m17.oil_max,
            power: p.m17.power,
            power_max: p.m17.power_max,
            caloric: p.m17.caloric,
            oxygen_seconds: p.m17.oxygen_seconds,
            heat: p.m17.heat,
            internal_shock_dose: p.m17.internal_shock_dose,
            power_fire_locked: p.m17.power_fire_locked,
            overclock_tier: p.m17.overclock_tier,
            throttled: p.m17.throttled,
        },
        None => cf_ui::HudResources::default(),
    };
    hud_state.concussion = match hud_state.player.as_ref() {
        Some(p) => {
            let band = cf_actor::concussion::ConcussionBand::from_dose(p.m17.concussion_dose);
            cf_ui::HudConcussion {
                dose: p.m17.concussion_dose,
                band: band.as_str().to_string(),
                vignette_fraction: band.vignette_fraction(),
                ducks_ambient: band.ducks_ambient(),
            }
        }
        None => cf_ui::HudConcussion::default(),
    };
    hud_state.modules = match hud_state.player.as_ref().and_then(|p| p.chassis.as_ref()) {
        Some(chassis) => HudModuleStrip {
            modules: chassis
                .modules
                .iter()
                .map(|m| HudModule {
                    id: m.id.clone(),
                    label: match m.kind.as_str() {
                        "weapon_mount" => "WEAPON".to_string(),
                        "jet" => "JET".to_string(),
                        "shield" => "SHIELD".to_string(),
                        "sensor" => "SENSOR".to_string(),
                        "repair_drone" => "REPAIR".to_string(),
                        _ => m.kind.to_uppercase(),
                    },
                    state: m.state.clone(),
                    kind: m.kind.clone(),
                })
                .collect(),
            placeholder: false,
        },
        None => build_hud_module_strip(snapshot.player_rifle.as_ref()),
    };

    hud_state.banners = hud_caches
        .banners
        .iter()
        .map(|b| HudBanner {
            id: b.id.clone(),
            severity: b.severity.clone(),
            label: b.label.clone(),
            raised_at_tick: b.raised_at_tick,
        })
        .collect();
    hud_state.captions = hud_caches
        .captions
        .iter()
        .map(|c| HudCaption {
            id: c.id.clone(),
            label: c.label.clone(),
            raised_at_tick: c.raised_at_tick,
        })
        .collect();
    hud_state.tool_validity =
        if hud_caches.tool_validity.last_carve_tick.is_some() || hud_caches.tool_validity.last_refusal_tick.is_some() {
            Some(HudToolValidity {
                last_carve_tick: hud_caches.tool_validity.last_carve_tick,
                last_refusal_tick: hud_caches.tool_validity.last_refusal_tick,
                last_refusal_reason: hud_caches.tool_validity.last_refusal_reason.clone(),
                last_refusal_target: hud_caches.tool_validity.last_refusal_target.clone(),
                valid: hud_caches.tool_validity.valid,
            })
        } else {
            None
        };
    hud_state.controls_captured_by = hud_caches.controls_captured_by.clone();

    if let Some(player) = hud_state.player.as_ref() {
        camera_follow.target = Some(bevy::math::Vec2::new(player.position[0], player.position[1]));
    }
    render_state.tool_valid = hud_caches
        .tool_validity
        .last_refusal_tick
        .map(|_| hud_caches.tool_validity.valid);

    render_state.breaches = snapshot
        .breaches
        .iter()
        .map(|b| BreachRender {
            id: b.id.clone(),
            bbox_min: b.bbox_min,
            bbox_max: b.bbox_max,
            hp: b.hp,
            max_hp: b.max_hp,
            broken: b.broken,
            refusal_reason: b.refusal_reason.clone(),
        })
        .collect();
    render_state.extraction_zone = snapshot.extraction_zone.as_ref().map(|z| ExtractionRender {
        min: z.min,
        max: z.max,
        completed: z.completed,
    });

    hud_state.mission = snapshot.mission.as_ref().map(|m| HudMission {
        result: m.result.clone(),
        loss_reason: m.loss_reason.clone(),
        elapsed_ticks: m.elapsed_ticks,
        time_limit_ticks: m.time_limit_ticks,
        ticks_remaining: m.ticks_remaining,
        active_objective: m.active_objective.clone(),
        last_event_label: m.last_event_label.clone(),
        show_me_why_event_id: m.show_me_why_event_id.clone(),
        show_replay_cta: m.show_replay_cta,
    });
    hud_state.last_event = snapshot.mission.as_ref().map(|m| m.last_event_label.clone());

    hud_state.enemy = snapshot.actors.iter().find(|a| !a.controllable).map(|a| {
        let enemy_view = snapshot.enemies.iter().find(|e| e.actor == a.id);
        HudEnemy {
            state: enemy_view.map(|e| e.state.clone()).unwrap_or_else(|| "—".to_string()),
            last_tactic: enemy_view
                .map(|e| e.last_tactic.clone())
                .unwrap_or_else(|| "—".to_string()),
            hp: a.hp,
            hp_max: a.hp_max,
            status: a.status.clone(),
            intent_label: enemy_view.map(|e| e.intent_label.clone()).unwrap_or_default(),
            world_position: enemy_view.and_then(|e| e.position).or(Some(a.position)),
        }
    });

    if let (Some(player), Some(_)) = (hud_state.player.as_ref(), snapshot.breaches.first()) {
        let px = player.position[0];
        let py = player.position[1];
        let aabb_distance = |b: &cf_control::BreachRenderView| -> f32 {
            let dx = (b.bbox_min[0] - px).max(0.0).max(px - b.bbox_max[0]);
            let dy = (b.bbox_min[1] - py).max(0.0).max(py - b.bbox_max[1]);
            ((dx * dx) + (dy * dy)).sqrt()
        };
        let mut best: Option<(&cf_control::BreachRenderView, f32)> = None;
        for b in &snapshot.breaches {
            let d = aabb_distance(b);
            match best {
                None => best = Some((b, d)),
                Some((_, prev)) if d < prev => best = Some((b, d)),
                _ => {}
            }
        }
        if let Some((b, d)) = best {
            hud_state.breach = Some(HudBreach {
                id: b.id.clone(),
                material: b.material.clone(),
                hp: b.hp,
                max_hp: b.max_hp,
                broken: b.broken,
                refusal_reason: b.refusal_reason.clone(),
                in_range: d <= b.dig_range,
            });
        } else {
            hud_state.breach = None;
        }
    } else {
        hud_state.breach = None;
    }
}

/// mirror the engine's reactor + timer projections into the cf-ui
/// widgets + cf-render-2d sprite resource so the HUD reactor strip + the
/// timer-warning captions + the reactor sprite swap all reflect the
/// live sim state.
pub(crate) fn sync_reactor_state_to_widgets(
    holder: Res<EngineHolder>,
    mut hp_bar: ResMut<ReactorHpBarState>,
    mut pressure_line: ResMut<ReactorPressureLineState>,
    mut timer_warnings: ResMut<TimerWarningsState>,
    mut sprite_state: ResMut<ReactorSpriteState>,
) {
    let snapshot = holder.0.actor_render_snapshot();

    match snapshot.reactor {
        Some(reactor) => {
            let pips: Vec<ArmorPipView> = reactor
                .armor_layers
                .iter()
                .map(|l| {
                    let kind: &'static str = match l.kind.as_str() {
                        "External" => "External",
                        "Internal" => "Internal",
                        "Core" => "Core",
                        _ => "External",
                    };
                    ArmorPipView {
                        kind,
                        hp: l.hp,
                        max_hp: l.max_hp,
                        hp_percent: l.hp_percent,
                        band: IntegrityBand::from_hp_percent(l.hp_percent),
                    }
                })
                .collect();
            hp_bar.update(reactor.hp, reactor.max_hp, &reactor.pressure_state, pips);
            pressure_line.update(&reactor.pressure_state);
            sprite_state.variant = ReactorSprite::from_pressure_state(&reactor.pressure_state);
            sprite_state.present = true;
        }
        None => {
            *hp_bar = ReactorHpBarState::default();
            *pressure_line = ReactorPressureLineState::default();
            sprite_state.variant = ReactorSprite::Nominal;
            sprite_state.present = false;
        }
    }

    match snapshot.timer {
        Some(timer) if timer.total_ticks > 0 && !timer.mission_terminal => {
            let remaining_s = timer.remaining_seconds;
            for (threshold_s, _severity, _caption) in WARNING_THRESHOLDS {
                if remaining_s <= *threshold_s {
                    let _ = timer_warnings.push_threshold(*threshold_s, remaining_s);
                }
            }
            timer_warnings.update_color(remaining_s);
        }
        _ => {
            timer_warnings.last_color = None;
        }
    }
}
