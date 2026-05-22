use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bevy::prelude::Resource;

use cf_control::{engine::M0Engine, server::ShutdownSignal};

#[derive(Resource)]
pub(crate) struct EngineHolder(pub(crate) Arc<M0Engine>);

/// loop. Wraps [`crate::quicksave::QuicksaveLoopState`] in a Bevy resource.
#[derive(Resource, Default)]
pub(crate) struct QuicksaveLoopResource(pub(crate) crate::quicksave::QuicksaveLoopState);

#[derive(Resource)]
pub(crate) struct AppRuntime {
    pub(crate) duration_ticks: u64,
    pub(crate) last_announced_tick: u64,
    /// many sim ticks per Bevy frame as the engine's clock budget allows
    /// (capped at `unpaced_max_ticks_per_frame`). Without this, cf-app
    /// drives exactly one tick per Bevy frame (~60Hz wall-clock), which
    /// makes 18000-tick endurance scripts take 300s of wall-clock and
    /// cf-e2e's 180s default timeout kill them. With this flag the engine
    /// races through pending budget so 18000 ticks complete in seconds.
    pub(crate) unpaced: bool,
    /// Safety cap on ticks-per-frame so a runaway budget can't starve
    /// Bevy's other systems. Defaults to 1024 which is plenty for the M1
    /// endurance script (~18 Bevy frames to finish 18000 ticks).
    pub(crate) unpaced_max_ticks_per_frame: u32,
}

#[derive(Resource)]
pub(crate) struct ControlRuntime {
    pub(crate) _runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) bound_addr: SocketAddr,
    pub(crate) server_handle: Mutex<Option<tokio::task::JoinHandle<std::io::Result<()>>>>,
    pub(crate) shutdown_tx: ShutdownSignal,
}

impl Drop for ControlRuntime {
    fn drop(&mut self) {
        cf_control::server::trigger_shutdown(&self.shutdown_tx);
        if let Some(handle) = self.server_handle.lock().ok().and_then(|mut g| g.take()) {
            handle.abort();
        }
        tracing::info!(target: "cf::app", bind = %self.bound_addr, "control server stopped");
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct LocalInputEnabled(pub(crate) bool);

/// engine tick advance so we don't take an extra read lock every render
/// frame (the cfctl `observe.once` poll path needs the same read lock at
/// 15 ms cadence; under paced 60 Hz with long `sim.run_for_ticks(N)`
/// windows, lock contention starved cfctl observe polls).
#[derive(Resource, Default)]
pub(crate) struct TerrainBridgeCursor {
    pub(crate) last_tick: u64,
    pub(crate) initialized: bool,
}

#[derive(Resource, Default)]
pub(crate) struct CaptureRecorderCursor(pub(crate) usize);

/// each ux.* / equipment.weapon_fired event is consumed exactly once.
#[derive(Resource, Default)]
pub(crate) struct RenderEffectsCursor(pub(crate) usize);

/// Cached workspace root for the M12 audio path resolver. cf-app inserts
/// this resource at startup (same value used to configure `AssetPlugin`).
#[derive(Resource, Debug, Clone)]
pub(crate) struct WorkspaceAssetRoot(pub(crate) PathBuf);

#[derive(Resource)]
pub(crate) struct M12aAudioRegistryRes(pub(crate) cf_audio::AudioRegistry);

#[derive(Resource)]
pub(crate) struct M12aSfxPoolRes(pub(crate) cf_audio::SfxPool);

#[derive(Resource)]
pub(crate) struct M12aCaptionRegistryRes(pub(crate) cf_audio::CaptionRegistry);

#[derive(Resource)]
pub(crate) struct M12aAudioQueueRes(pub(crate) cf_audio::AudioReplayQueue);

#[derive(Resource)]
#[allow(dead_code)]
pub(crate) struct M12aMixBusesRes(pub(crate) cf_audio::MixBuses);

/// `Arc<HrirTable>` shared across systems; future bevy_audio integration
/// invokes `adapter.resolve(envelope)` per audio cue to derive the
/// per-source HRIR samples + Doppler pitch + low-pass cutoff.
#[derive(Resource)]
#[allow(dead_code)]
pub(crate) struct M12bHrirAdapterRes(pub(crate) cf_app::HrirConvolutionAdapter);

/// that swaps the active IR when the listener crosses a room boundary
/// per M19G boundary detection; the swap cross-fades over
/// `IR_CROSS_FADE_MS` (250 ms per spec) to avoid clicks.
#[derive(Resource)]
#[allow(dead_code)]
pub(crate) struct M12bReverbSendBusRes(pub(crate) cf_app::ReverbSendBus);
