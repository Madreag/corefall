use bevy::prelude::*;

/// pixels) when an `ux.camera_punch_requested` event fires; the
/// `apply_camera_effects` system decays the magnitude by ~exp(-dt/0.2s) and
/// applies a per-frame random offset to the Camera2d. Setting
/// `reduce_camera_shake_pct=1.0` zeroes the magnitude on intake so the
/// camera never moves (accessibility floor).
#[derive(Resource, Debug, Clone, Default)]
pub struct CameraShake {
    pub magnitude_px: f32,
    pub reduce_pct: f32,
    /// 64-bit xorshift state for the per-frame jitter. Seeded by cf-app from
    /// the engine RNG at startup so the visual is deterministic given the
    /// same input event stream.
    pub rng_state: u64,
}

impl CameraShake {
    fn next_jitter(&mut self) -> (f32, f32) {
        // xorshift64* — deterministic and cheap.
        let mut s = if self.rng_state == 0 {
            0x9E3779B97F4A7C15
        } else {
            self.rng_state
        };
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.rng_state = s;
        let x = (s as f32) / (u32::MAX as f32 * 2.0) - 0.5;
        let y = ((s >> 32) as f32) / (u32::MAX as f32 * 2.0) - 0.5;
        (x, y)
    }
}

/// `target` each frame from the player actor position. The render system
/// lerps the camera position toward the target whenever the target leaves
/// the deadzone rectangle.
#[derive(Resource, Debug, Clone)]
pub struct CameraFollow {
    pub target: Option<Vec2>,
    pub deadzone_half_width_px: f32,
    pub deadzone_half_height_px: f32,
    /// Per-frame lerp factor (0..1) applied when target is outside the
    /// deadzone. 0.18 = ~5-frame catch-up at 60Hz; tweak in cvars when E2
    /// promoted to a settings cvar.
    pub lerp_factor: f32,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            target: None,
            deadzone_half_width_px: 40.0,
            deadzone_half_height_px: 30.0,
            lerp_factor: 0.18,
        }
    }
}

/// `ux.hit_stop_requested` event fires. The `apply_camera_effects` system
/// uses Bevy's `Time::set_relative_speed` to slow the world during the
/// freeze window. M1 ships even a single-tick pause as proof of life.
#[derive(Resource, Debug, Clone, Default)]
pub struct HitStop {
    pub remaining_ms: f32,
}

/// Drive camera shake + camera follow + hit-stop per frame. cf-app populates
/// the input resources; the render system applies effects to the Camera2d
/// transform and the global Bevy `Time` resource.
pub fn apply_camera_effects(
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut shake: ResMut<CameraShake>,
    follow: Res<CameraFollow>,
    mut hit_stop: ResMut<HitStop>,
    mut time_speed: ResMut<Time<Virtual>>,
) {
    if let Some(mut camera_transform) = camera_query.iter_mut().next() {
        // Camera follow: lerp toward target when outside the deadzone.
        if let Some(target) = follow.target {
            let cam = camera_transform.translation.truncate();
            let dx = target.x - cam.x;
            let dy = target.y - cam.y;
            let target_x = if dx.abs() > follow.deadzone_half_width_px {
                cam.x + dx * follow.lerp_factor.clamp(0.0, 1.0)
            } else {
                cam.x
            };
            let target_y = if dy.abs() > follow.deadzone_half_height_px {
                cam.y + dy * follow.lerp_factor.clamp(0.0, 1.0)
            } else {
                cam.y
            };
            camera_transform.translation.x = target_x;
            camera_transform.translation.y = target_y;
        }
        // Camera shake: decay magnitude then apply random offset.
        let dt = time.delta_secs();
        // Tau ~0.2s exponential decay (shake ends within ~200ms).
        let decay = (-dt / 0.2).exp();
        shake.magnitude_px *= decay;
        if shake.magnitude_px > 0.05 {
            let (jx, jy) = shake.next_jitter();
            let scale = (1.0 - shake.reduce_pct.clamp(0.0, 1.0)).max(0.0);
            camera_transform.translation.x += jx * shake.magnitude_px * scale;
            camera_transform.translation.y += jy * shake.magnitude_px * scale;
        } else {
            shake.magnitude_px = 0.0;
        }
    }
    // Hit-stop: freeze the virtual clock for the requested window.
    if hit_stop.remaining_ms > 0.0 {
        time_speed.set_relative_speed(0.05);
        hit_stop.remaining_ms = (hit_stop.remaining_ms - time.delta_secs() * 1000.0).max(0.0);
    } else if time_speed.relative_speed() < 0.99 {
        time_speed.set_relative_speed(1.0);
    }
}
