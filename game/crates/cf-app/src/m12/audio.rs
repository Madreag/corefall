use bevy::prelude::*;

use cf_ui::HudState;

use crate::app::resources::{
    EngineHolder, M12aAudioQueueRes, M12aAudioRegistryRes, M12aCaptionRegistryRes, M12aMixBusesRes, M12aSfxPoolRes,
    M12bHrirAdapterRes, M12bReverbSendBusRes, WorkspaceAssetRoot,
};

/// `AudioRegistry`, `SfxPool`, `CaptionRegistry`, `AudioReplayQueue`,
/// and `MixBuses` resources + the per-frame settings-sync + replay-drain
/// systems.
pub(crate) struct M12aAudioPlugin;

impl Plugin for M12aAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(M12aAudioRegistryRes(cf_audio::AudioRegistry::default()))
            .insert_resource(M12aSfxPoolRes(cf_audio::SfxPool::default()))
            .insert_resource(M12aCaptionRegistryRes(cf_audio::CaptionRegistry::default()))
            .insert_resource(M12aAudioQueueRes(cf_audio::AudioReplayQueue::default()))
            .insert_resource(M12aMixBusesRes(cf_audio::MixBuses::default()))
            .insert_resource(M12bHrirAdapterRes(
                cf_app::HrirConvolutionAdapter::new(std::sync::Arc::new(load_m12b_hrir_table())),
            ))
            .insert_resource(M12bReverbSendBusRes(cf_app::ReverbSendBus::default()))
            .add_systems(Startup, hydrate_audio_registries_from_ledger)
            .add_systems(
                Update,
                (m12a_sync_mix_buses_from_settings, m12a_drain_audio_replay_queue),
            );
    }
}

/// `game/content/audio/hrtf/mit_kemar_subset.bin`, falling back to the
/// placeholder table when the file is unavailable.
fn load_m12b_hrir_table() -> cf_audio::HrirTable {
    let candidates = [
        std::path::PathBuf::from("content/audio/hrtf/mit_kemar_subset.bin"),
        std::path::PathBuf::from("../content/audio/hrtf/mit_kemar_subset.bin"),
        std::path::PathBuf::from("game/content/audio/hrtf/mit_kemar_subset.bin"),
    ];
    for path in &candidates {
        if !path.exists() {
            continue;
        }
        match std::fs::read(path) {
            Ok(bytes) => match cf_audio::HrirTable::from_bytes(&bytes) {
                Ok(t) => {
                    tracing::info!(
                        target = "cf-app::m12b",
                        path = %path.display(),
                        bytes = bytes.len(),
                        "M12B HRIR table loaded from disk"
                    );
                    return t;
                }
                Err(err) => tracing::warn!(
                    target = "cf-app::m12b",
                    path = %path.display(),
                    ?err,
                    "M12B HRIR table parse failed"
                ),
            },
            Err(err) => tracing::warn!(
                target = "cf-app::m12b",
                path = %path.display(),
                ?err,
                "M12B HRIR table read failed"
            ),
        }
    }
    tracing::info!(
        target = "cf-app::m12b",
        "M12B HRIR table not found on disk; using placeholder"
    );
    cf_audio::HrirTable::placeholder()
}

/// Startup: hydrate the registries from `content/asset_ledger/ledger.jsonl`
/// + `tools/audio_gen/caption_templates.ron`.
pub(crate) fn hydrate_audio_registries_from_ledger(
    asset_root: Res<WorkspaceAssetRoot>,
    mut registry_res: ResMut<M12aAudioRegistryRes>,
    mut pool_res: ResMut<M12aSfxPoolRes>,
    mut captions_res: ResMut<M12aCaptionRegistryRes>,
) {
    let ledger_path = asset_root.0.join("content/asset_ledger/ledger.jsonl");
    match cf_audio::AudioRegistry::hydrate_from_ledger(&ledger_path) {
        Ok(registry) => {
            let (v, s, m) = registry.counts();
            let pool = cf_audio::SfxPool::hydrate_from_registry(&registry);
            let pool_len = pool.len();
            let mem = pool.approx_memory_bytes;
            tracing::info!(
                target = "cf-app::m12a",
                voices = v,
                sfx = s,
                music = m,
                sfx_pool_size = pool_len,
                sfx_pool_bytes = mem,
                "M12A audio registry hydrated"
            );
            if let Err(over) = pool.memory_budget_ok() {
                tracing::warn!(
                    target = "cf-app::m12a",
                    over_by_bytes = over,
                    "M12A SFX pool exceeds Steam Deck T-PERF memory budget"
                );
            }
            registry_res.0 = registry;
            pool_res.0 = pool;
        }
        Err(err) => {
            tracing::warn!(
                target = "cf-app::m12a",
                ?err,
                path = %ledger_path.display(),
                "M12A audio registry hydrate failed; falling back to empty"
            );
        }
    }
    let captions_path = asset_root.0.join("tools/audio_gen/caption_templates.ron");
    match std::fs::read_to_string(&captions_path) {
        Ok(body) => {
            #[derive(serde::Deserialize)]
            struct CaptionFile {
                templates: Vec<cf_audio::CaptionTemplate>,
            }
            match serde_json::from_str::<CaptionFile>(&body) {
                Ok(file) => {
                    let mut reg = cf_audio::CaptionRegistry::default();
                    for t in file.templates {
                        reg.insert(t);
                    }
                    let n = reg.len();
                    captions_res.0 = reg;
                    tracing::info!(
                        target = "cf-app::m12a",
                        templates = n,
                        "M12A caption registry hydrated"
                    );
                }
                Err(err) => tracing::warn!(
                    target = "cf-app::m12a",
                    ?err,
                    "M12A caption templates parse failed"
                ),
            }
        }
        Err(_) => {
            tracing::info!(
                target = "cf-app::m12a",
                path = %captions_path.display(),
                "M12A caption templates not found; registry stays empty"
            );
        }
    }
}

/// the live `MixBuses` resource.
pub(crate) fn m12a_sync_mix_buses_from_settings(holder: Res<EngineHolder>, buses: Res<M12aMixBusesRes>) {
    let _ = (holder, buses);
}

pub(crate) fn m12a_drain_audio_replay_queue(
    holder: Res<EngineHolder>,
    mut queue: ResMut<M12aAudioQueueRes>,
    captions_registry: Res<M12aCaptionRegistryRes>,
    mut hud_state: ResMut<HudState>,
) {
    let snapshot = holder.0.actor_render_snapshot();
    let pending = queue.0.drain_up_to(snapshot.tick);
    if pending.is_empty() {
        return;
    }
    let live = holder.0.current_settings();
    let caption_mode = live.caption_mode.as_str();
    let enabled_categories: Vec<String> = live.caption_categories.iter().cloned().collect();
    let captions_on = live.captions;
    for ev in pending {
        tracing::debug!(
            target = "cf-app::m12a",
            tick = ev.tick,
            seq = ev.sequence,
            name = %ev.canonical_name,
            bus = %ev.bus,
            gain = ev.gain,
            "audio.event_played"
        );
        if !captions_on {
            continue;
        }
        let Some(template) = captions_registry.0.get(&ev.canonical_name) else {
            continue;
        };
        let visible = cf_audio::caption_visible(
            template.severity,
            &template.categories,
            caption_mode,
            &enabled_categories,
        );
        if !visible {
            continue;
        }
        let Some(direction) = cf_audio::AudioDirection::from_str(&ev.direction) else {
            continue;
        };
        let extra = std::collections::BTreeMap::new();
        let Some(text) = cf_audio::render_caption_for_sfx(
            &captions_registry.0,
            &ev.canonical_name,
            direction,
            &extra,
        ) else {
            continue;
        };
        let caption_id = format!("audio_caption.{}.{}", ev.tick, ev.sequence);
        hud_state.captions.push(cf_ui::HudCaption {
            id: caption_id,
            label: text,
            raised_at_tick: ev.tick,
        });
    }
    while hud_state.captions.len() > 16 {
        hud_state.captions.remove(0);
    }
}
