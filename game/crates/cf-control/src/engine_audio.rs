//! Audio cue + M12B spatial-resolve methods extracted from engine.rs.

use serde_json::json;

use crate::engine::{push_caption, M0Engine};

impl M0Engine {
    pub fn set_audio_plugin(&self, plugin: Box<dyn cf_audio::AudioPlugin>) {
        if let Ok(mut p) = self.audio_plugin.lock() {
            *p = plugin;
        }
    }

    pub(crate) fn emit_audio_cue(&self, cue: cf_audio::AudioCue, tick: cf_sim_core::Tick) {
        if let Ok(plugin) = self.audio_plugin.lock() {
            plugin.play(&cue);
        }
        if let Ok(mut s) = self.state.write() {
            push_caption(
                &mut s.hud_captions,
                crate::state::CaptionView {
                    id: format!("audio.{}.{}", cue.stub_tag(), tick.0),
                    label: cue.caption().to_string(),
                    raised_at_tick: tick.0,
                    accessibility_id: format!("hud.caption.audio.{}", cue.stub_tag()),
                },
            );
        }
    }

    pub(crate) fn emit_audio_cue_for_actor(
        &self,
        cue: cf_audio::AudioCue,
        tick: cf_sim_core::Tick,
        sim_time_ms: f64,
        actor: cf_actor::ActorId,
    ) {
        let cue_tag = cue.stub_tag().to_string();
        self.emit_audio_cue(cue, tick);
        let (position, velocity) = self
            .state
            .read()
            .ok()
            .and_then(|s| {
                s.actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&actor))
                    .map(|a| ([a.position.x, a.position.y], [a.velocity.x, a.velocity.y]))
            })
            .unwrap_or(([0.0, 0.0], [0.0, 0.0]));
        let cue_name = format!("{cue_tag}.{}", actor.0);
        self.emit_m12b_spatial_resolve(
            tick,
            sim_time_ms,
            &cue_name,
            position,
            velocity,
            cf_audio::Medium::Air,
            &[],
            cf_audio::ReverbProfile::open_outdoor(),
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn emit_m12b_spatial_resolve(
        &self,
        tick: cf_sim_core::Tick,
        sim_time_ms: f64,
        cue_name: &str,
        source_position: [f32; 2],
        source_velocity: [f32; 2],
        medium: cf_audio::Medium,
        walls: &[cf_audio::WallAcoustics],
        reverb_profile: cf_audio::ReverbProfile,
        room_id: Option<u64>,
    ) {
        let (listener_position, listener_velocity, listener_facing) = self.m12b_listener_state();
        let source = cf_audio::SourceContext {
            position: source_position,
            velocity: source_velocity,
            base_gain: 1.0,
            propagation_range_m: 100.0,
            room_id,
        };
        let listener = cf_audio::ListenerContext {
            position: listener_position,
            velocity: listener_velocity,
            facing_rad: listener_facing,
            room_id,
        };
        let env = cf_audio::resolve_spatial(source, listener, medium, walls, reverb_profile);

        self.recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "audio",
            "spatial_resolved",
            json!({
                "canonical_name": cue_name,
                "azimuth_rad": env.azimuth_rad,
                "elevation_rad": env.elevation_rad,
                "distance_m": env.distance_m,
                "hrir_index": {
                    "azimuth_bucket": env.hrir_index.azimuth_bucket as u32,
                    "elevation_bucket": env.hrir_index.elevation_bucket as u32,
                },
                "direction": env.direction.label(),
                "gain": env.gain,
                "source_position": [source_position[0], source_position[1]],
                "listener_position": [listener_position[0], listener_position[1]],
                "listener_facing_rad": listener_facing,
            }),
            None,
        );

        self.recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "audio",
            "reverb_applied",
            json!({
                "canonical_name": cue_name,
                "room_id": room_id,
                "tail_seconds": reverb_profile.tail_seconds,
                "decay_coefficient": reverb_profile.decay_coefficient,
                "decay_band": reverb_profile.decay_band.as_str(),
                "wet_dry_mix": reverb_profile.wet_dry_mix,
                "early_reflection_delay_ms": reverb_profile.early_reflection_delay_ms,
                "aperture_attenuation_db": reverb_profile.aperture_attenuation_db,
                "reverb_send_db": env.reverb_send_db,
            }),
            None,
        );

        self.recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "audio",
            "occluded",
            json!({
                "canonical_name": cue_name,
                "occlusion_db": env.occlusion.occlusion_db,
                "low_pass_cutoff_hz": env.occlusion.low_pass_cutoff_hz,
                "wall_count": env.occlusion.wall_count,
                "clipped": env.occlusion.clipped,
                "source_position": [source_position[0], source_position[1]],
                "listener_position": [listener_position[0], listener_position[1]],
            }),
            None,
        );

        self.recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "audio",
            "doppler_shifted",
            json!({
                "canonical_name": cue_name,
                "doppler_factor": env.doppler.factor,
                "clamped": env.doppler.clamped,
                "speed_of_sound_m_per_s": env.doppler.speed_of_sound_m_per_s,
                "medium": env.medium_filter.medium.as_str(),
                "source_velocity": [source_velocity[0], source_velocity[1]],
                "listener_velocity": [listener_velocity[0], listener_velocity[1]],
            }),
            None,
        );
    }

    pub(crate) fn emit_m12b_per_tick_projectile_audio(&self, tick: cf_sim_core::Tick, sim_time_ms: f64) {
        let projectile_snapshot: Vec<(u64, [f32; 2], [f32; 2])> = match self.state.read() {
            Ok(s) => match s.actor_state.as_ref() {
                Some(sim) => sim
                    .projectiles
                    .iter()
                    .map(|p| (p.id, [p.position.x, p.position.y], [p.velocity.x, p.velocity.y]))
                    .collect(),
                None => Vec::new(),
            },
            Err(_) => Vec::new(),
        };
        if projectile_snapshot.is_empty() {
            return;
        }
        for (id, position, velocity) in projectile_snapshot {
            let cue_name = format!("projectile_fly.{id}");
            self.emit_m12b_spatial_resolve(
                tick,
                sim_time_ms,
                &cue_name,
                position,
                velocity,
                cf_audio::Medium::Air,
                &[],
                cf_audio::ReverbProfile::open_outdoor(),
                None,
            );
        }
    }

    pub(crate) fn m12b_listener_state(&self) -> ([f32; 2], [f32; 2], f32) {
        let s = match self.state.read() {
            Ok(s) => s,
            Err(_) => return ([0.0, 0.0], [0.0, 0.0], 0.0),
        };
        let Some(pid) = s.player_actor else {
            return ([0.0, 0.0], [0.0, 0.0], 0.0);
        };
        let Some(actor_state) = s.actor_state.as_ref() else {
            return ([0.0, 0.0], [0.0, 0.0], 0.0);
        };
        let Some(actor) = actor_state.world.actors.get(&pid) else {
            return ([0.0, 0.0], [0.0, 0.0], 0.0);
        };
        let facing = (actor.aim.y).atan2(actor.aim.x);
        (
            [actor.position.x, actor.position.y],
            [actor.velocity.x, actor.velocity.y],
            facing,
        )
    }
}
