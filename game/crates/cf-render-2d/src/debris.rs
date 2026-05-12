//! **M2**: loose pixel debris particles spawned at carve sites.
//!
//! Listens for `terrain.terrain_pixel_dislodged` recorder events via the
//! cursor pattern shared with `pump_recorder_events_into_render_effects`.
//! For each event we spawn one `LooseDebris` entity at the event's `pos`
//! with downward initial velocity, color from `spawn_material →
//! MaterialAffordance::overlay_rgba`. The debris is purely cosmetic.
//!
//! - Cap: 100 concurrent entities (matches the engine's `DEBRIS_CAP`).
//! - Despawn-on-settle: velocity < `SETTLE_SPEED` for `SETTLE_TICKS`.
//! - Apply gravity each frame.
//! - Emit `RenderDebrisCappedEvent` (Bevy-only) whenever a spawn is
//!   rejected because the cap is reached — observable via `MessageReader`
//!   so tests can assert the cap fires.

use std::collections::VecDeque;

use bevy::prelude::*;

use cf_terrain::{material_affordance, MaterialId};

pub const DEBRIS_CAP: usize = 100;
const GRAVITY_Y: f32 = -980.0;
const SETTLE_SPEED: f32 = 1.0;
const SETTLE_TICKS: u32 = 10;

/// Render-side event published when a debris spawn would exceed [`DEBRIS_CAP`].
/// Bevy-only (no recorder event); tests + the cf-ui debug panel listen.
#[derive(Message, Debug, Clone)]
pub struct RenderDebrisCappedEvent {
    pub requested_count: usize,
    pub granted_count: usize,
}

/// Component tagging a render-side debris pixel. Carries velocity + age so
/// the despawn-on-settle pass can run without a per-entity `Physics`
/// dependency.
#[derive(Component, Debug)]
pub struct LooseDebris {
    pub velocity: Vec2,
    pub spawn_material: MaterialId,
    pub ticks_settled: u32,
}

/// One debris spawn payload. cf-app's bridge fills these from
/// `terrain.terrain_pixel_dislodged` events and pushes them into the
/// [`DebrisSpawnQueue`]; the render system drains the queue and spawns
/// entities subject to the cap.
#[derive(Debug, Clone)]
pub struct DebrisSpawnRequest {
    pub pos: Vec2,
    pub spawn_material: MaterialId,
    pub count: u32,
}

/// Per-frame queue of debris spawn requests. cf-app writes; the renderer
/// drains.
#[derive(Resource, Debug, Default)]
pub struct DebrisSpawnQueue {
    pub pending: VecDeque<DebrisSpawnRequest>,
}

/// Render-side plugin: registers the resources + system schedule.
pub struct DebrisPlugin;

impl Plugin for DebrisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebrisSpawnQueue>()
            .add_message::<RenderDebrisCappedEvent>()
            .add_systems(Update, (spawn_pending_debris, settle_and_despawn_debris).chain());
    }
}

fn spawn_pending_debris(
    mut commands: Commands,
    mut queue: ResMut<DebrisSpawnQueue>,
    existing: Query<Entity, With<LooseDebris>>,
    mut writer: MessageWriter<RenderDebrisCappedEvent>,
) {
    let mut current = existing.iter().count();
    while let Some(req) = queue.pending.pop_front() {
        let want = req.count.max(1) as usize;
        let allowed = DEBRIS_CAP.saturating_sub(current).min(want);
        if allowed < want {
            writer.write(RenderDebrisCappedEvent {
                requested_count: want,
                granted_count: allowed,
            });
        }
        let [r, g, b, a] = material_affordance(req.spawn_material)
            .map(|m| m.overlay_rgba)
            .unwrap_or([200, 200, 200, 0xFF]);
        let color = Color::srgba(
            (r as f32) / 255.0,
            (g as f32) / 255.0,
            (b as f32) / 255.0,
            (a as f32) / 255.0,
        );
        for i in 0..allowed {
            let jitter_x = (i as f32 * 0.7).sin() * 4.0;
            let jitter_y = (i as f32 * 0.3).cos() * 2.0;
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(2.0, 2.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(req.pos.x + jitter_x, req.pos.y + 4.0 + jitter_y, 0.85)),
                LooseDebris {
                    velocity: Vec2::new(jitter_x * 0.5, -20.0 - jitter_y.abs() * 4.0),
                    spawn_material: req.spawn_material,
                    ticks_settled: 0,
                },
                Name::new(format!("cf::render::debris::{}", req.spawn_material)),
            ));
            current += 1;
        }
        if current >= DEBRIS_CAP {
            break;
        }
    }
}

fn settle_and_despawn_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut LooseDebris)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut debris) in q.iter_mut() {
        debris.velocity.y += GRAVITY_Y * dt;
        // Lateral drag.
        debris.velocity.x *= 0.92;
        transform.translation.x += debris.velocity.x * dt;
        transform.translation.y += debris.velocity.y * dt;
        if debris.velocity.length_squared() < (SETTLE_SPEED * SETTLE_SPEED) {
            debris.ticks_settled = debris.ticks_settled.saturating_add(1);
            if debris.ticks_settled >= SETTLE_TICKS {
                commands.entity(entity).despawn();
            }
        } else {
            debris.ticks_settled = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_terrain::{MATERIAL_DIRT, MATERIAL_LOOSE_FILL};

    fn debris_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(DebrisPlugin);
        app
    }

    #[test]
    fn spawn_emits_entity_with_spawn_material_color() {
        let mut app = debris_test_app();
        {
            let mut queue = app.world_mut().resource_mut::<DebrisSpawnQueue>();
            queue.pending.push_back(DebrisSpawnRequest {
                pos: Vec2::new(100.0, 50.0),
                spawn_material: MATERIAL_DIRT,
                count: 1,
            });
        }
        app.update();
        let mut q = app.world_mut().query::<&LooseDebris>();
        let mut found = 0;
        for d in q.iter(app.world()) {
            assert_eq!(d.spawn_material, MATERIAL_DIRT);
            found += 1;
        }
        assert_eq!(found, 1);
    }

    #[test]
    fn cap_emits_render_debris_capped_event() {
        let mut app = debris_test_app();
        {
            let mut queue = app.world_mut().resource_mut::<DebrisSpawnQueue>();
            queue.pending.push_back(DebrisSpawnRequest {
                pos: Vec2::ZERO,
                spawn_material: MATERIAL_LOOSE_FILL,
                count: (DEBRIS_CAP + 50) as u32,
            });
        }
        app.update();
        let events = app
            .world()
            .resource::<bevy::ecs::message::Messages<RenderDebrisCappedEvent>>();
        let mut cursor = events.get_cursor();
        let any = cursor.read(events).next().is_some();
        assert!(any, "expected RenderDebrisCappedEvent when cap exceeded");
        // The number of LooseDebris entities is capped at DEBRIS_CAP.
        let mut q = app.world_mut().query::<&LooseDebris>();
        let count = q.iter(app.world()).count();
        assert_eq!(count, DEBRIS_CAP);
    }
}
