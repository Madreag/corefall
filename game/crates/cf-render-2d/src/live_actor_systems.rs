use std::collections::HashMap;

use bevy::prelude::*;

use crate::live_camera_effects::{apply_camera_effects, CameraFollow, CameraShake, HitStop};
use crate::live_chassis_layout::{
    actor_color, breach_color, chassis_scale_multiplier, chassis_stage_tint, module_pip_color, module_pip_offset,
    stance_offset, stance_tint_for, walk_cycle_offsets, zone_color, ACTOR_SILHOUETTE_BASE_SCALE, CHASSIS_ZONE_LAYOUT,
};
use crate::live_render_state::{
    ActorChassisZoneView, ActorRenderState, ActorRenderTag, BreachRenderTag, ChassisModuleRenderTag,
    ChassisZoneRenderTag, ExtractionZoneTag, FloorRenderTag, HeldRifleRenderTag, MuzzleFlashTag, ReticleRenderTag,
};
use crate::live_sprite_image::{build_solid_sprite_image, solid_sprite, SolidSpriteImage};

/// Plugin that wires actor / floor / reticle rendering. Call after [`CfRenderPlugin`].
pub struct ActorSpritePlugin;

impl Plugin for ActorSpritePlugin {
    fn build(&self, app: &mut App) {
        // M1 Gap E: defensively init the camera-effect resources so unit
        // tests that load ActorSpritePlugin without the parent CfRenderPlugin
        // still have working defaults.
        app.init_resource::<ActorRenderState>()
            .init_resource::<SolidSpriteImage>()
            .init_resource::<CameraShake>()
            .init_resource::<CameraFollow>()
            .init_resource::<HitStop>()
            .add_systems(Startup, (build_solid_sprite_image, spawn_floor_and_reticle).chain())
            .add_systems(
                Update,
                (
                    sync_actor_sprites,
                    sync_chassis_zone_sprites,
                    sync_chassis_module_sprites,
                    sync_held_rifle_sprites,
                    sync_breach_sprites,
                    sync_extraction_zone,
                    // Gap E1-E3: camera punch / hit-stop / follow runs AFTER the
                    // chain so the player position is already up to date and the
                    // camera lerp catches the same frame.
                    apply_camera_effects,
                    update_reticle_color,
                    update_muzzle_flash,
                )
                    .chain(),
            );
    }
}

/// Some(false)`, otherwise restore the canonical white tint. Friendly-fire
/// color hook lands at M1.5 when teams ship.
fn update_reticle_color(state: Res<ActorRenderState>, mut q: Query<&mut Sprite, With<ReticleRenderTag>>) {
    let color = match state.tool_valid {
        Some(false) => Color::srgb(1.0, 0.25, 0.25),
        _ => Color::srgb(1.0, 1.0, 1.0),
    };
    for mut sprite in &mut q {
        if sprite.color != color {
            sprite.color = color;
        }
    }
}

/// every `equipment.weapon_fired`. cf-app populates
/// `ActorRenderState::muzzle_flash`; this system spawns a transient sprite
/// at the muzzle origin and decays it.
fn update_muzzle_flash(
    mut commands: Commands,
    mut state: ResMut<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    existing: Query<Entity, With<MuzzleFlashTag>>,
) {
    // Despawn any existing flash entity each frame; we re-spawn fresh
    // whenever `muzzle_flash` is `Some`. Cheap because flashes live <= 3
    // ticks at 60 Hz.
    for e in existing.iter() {
        commands.entity(e).despawn();
    }
    if let Some(flash) = state.muzzle_flash.take() {
        let alpha = (flash.remaining_ticks as f32 / 3.0).clamp(0.0, 1.0);
        commands.spawn((
            solid_sprite(&solid, Color::srgba(1.0, 0.9, 0.4, alpha), Vec2::new(10.0, 6.0)),
            Transform::from_translation(Vec3::new(flash.origin.x, flash.origin.y, 1.5)),
            MuzzleFlashTag,
            Name::new("cf::render::muzzle_flash"),
        ));
    }
}

fn spawn_floor_and_reticle(mut commands: Commands, solid: Res<SolidSpriteImage>) {
    // Floor (placeholder; real chunked terrain lands at M2).
    commands.spawn((
        solid_sprite(&solid, Color::srgb(0.15, 0.18, 0.20), Vec2::new(2048.0, 8.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.5)),
        FloorRenderTag,
        Name::new("cf::render::floor"),
    ));
    commands.spawn((
        solid_sprite(&solid, Color::srgb(0.95, 0.65, 0.30), Vec2::new(4.0, 4.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        Visibility::Hidden,
        ReticleRenderTag,
        Name::new("cf::render::reticle"),
    ));
}

#[allow(clippy::too_many_arguments)]
fn sync_actor_sprites(
    mut commands: Commands,
    mut state: ResMut<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut actor_query: Query<(Entity, &ActorRenderTag, &mut Transform, &mut Sprite)>,
    mut floor_query: Query<
        (&mut Transform, &mut Sprite),
        (With<FloorRenderTag>, Without<ActorRenderTag>, Without<ReticleRenderTag>),
    >,
    mut reticle_query: Query<
        (&mut Transform, &mut Visibility),
        (With<ReticleRenderTag>, Without<ActorRenderTag>, Without<FloorRenderTag>),
    >,
    mut camera_query: Query<
        &mut Transform,
        (
            With<Camera2d>,
            Without<ActorRenderTag>,
            Without<FloorRenderTag>,
            Without<ReticleRenderTag>,
        ),
    >,
) {
    // Place the floor centred under the play region. The region's bottom-left
    // anchor may be non-zero, so derive the world-space centre from
    // `region_anchor + region_size * 0.5` instead of assuming `(0, 0)`.
    if state.region_width > 0.0 {
        let region_center_x = state.region_anchor_x + state.region_width * 0.5;
        let region_center_y = state.region_anchor_y + state.region_height * 0.5;
        if let Some((mut transform, mut sprite)) = floor_query.iter_mut().next() {
            transform.translation = Vec3::new(region_center_x, state.floor_y - 4.0, -0.5);
            sprite.custom_size = Some(Vec2::new(state.region_width, 8.0));
        }
        // M1 Gap E2: the CameraFollow system owns camera positioning during
        // gameplay. We only seed the camera once at startup (when the camera
        // is still at the world origin) so the very first frame doesn't show
        // a half-empty viewport. Any subsequent frame, CameraFollow lerps.
        if let Some(mut camera_transform) = camera_query.iter_mut().next() {
            if camera_transform.translation.x.abs() < 0.5 && camera_transform.translation.y.abs() < 0.5 {
                camera_transform.translation.x = region_center_x;
                camera_transform.translation.y = region_center_y;
            }
        }
    }

    let mut existing: HashMap<u64, Entity> = HashMap::new();
    for (entity, tag, _, _) in actor_query.iter() {
        existing.insert(tag.id, entity);
    }

    let mut player_position: Option<Vec2> = None;
    let mut player_aim: Option<Vec2> = None;

    let mut keep: HashMap<u64, ()> = HashMap::new();
    for actor in &state.actors {
        keep.insert(actor.id, ());
        let pos = Vec2::new(actor.position[0], actor.position[1]);
        // `sync_chassis_zone_sprites`). Chassis-attached actors use real
        // per-zone hp; chassis-less actors use a synthetic intact body
        // derived from HP.
        //
        // tinted silhouette so the actor renders as a visible body, not a
        // ghost rectangle behind the chassis pips. The tint varies by
        // stance (Idle / Walking / Running / Airborne / KnockedDown /
        // Downed / Dead) so the visual identity tracks the sim. Actors
        // WITH a chassis keep the transparent parent so the per-zone pips
        // remain authoritative (M5 chassis grammar owns the silhouette).
        let parent_color = if actor.chassis.is_some() {
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        } else {
            stance_tint_for(&actor.stance, &actor.status)
        };
        if let Some(entity) = existing.get(&actor.id) {
            if let Ok((_, _, mut transform, mut sprite)) = actor_query.get_mut(*entity) {
                transform.translation = Vec3::new(pos.x, pos.y, 0.5);
                sprite.color = parent_color;
            }
        } else {
            let mut entity_commands = commands.spawn((
                solid_sprite(&solid, parent_color, Vec2::new(16.0, 32.0)),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 0.5)),
                ActorRenderTag { id: actor.id },
                Name::new(format!("cf::render::actor::{}", actor.id)),
            ));
            entity_commands.insert(if actor.controllable {
                Name::new(format!("cf::render::actor::{}::player", actor.id))
            } else {
                Name::new(format!("cf::render::actor::{}::npc", actor.id))
            });
        }
        if Some(actor.id) == state.player_actor_id {
            player_position = Some(pos);
            player_aim = Some(Vec2::new(actor.aim[0], actor.aim[1]));
        }
    }

    // Despawn actors that left the world.
    for (entity, tag, _, _) in actor_query.iter() {
        if !keep.contains_key(&tag.id) {
            commands.entity(entity).despawn();
        }
    }

    // Reticle follows the player's aim. M1 Gap E4: scale by bloom factor
    // (with sharp-aim tightening) and tint red when tool_validity says the
    // current action would refuse.
    let player_actor_ref = state
        .player_actor_id
        .and_then(|id| state.actors.iter().find(|a| a.id == id));
    if let (Some(pos), Some(aim)) = (player_position, player_aim) {
        if let Some((mut transform, mut visibility)) = reticle_query.iter_mut().next() {
            let aim_unit = if aim.length_squared() > 1e-6 {
                aim.normalize()
            } else {
                Vec2::new(1.0, 0.0)
            };
            transform.translation = Vec3::new(pos.x + aim_unit.x * 32.0, pos.y + aim_unit.y * 32.0, 1.0);
            let bloom = player_actor_ref.map(|p| p.bloom_factor).unwrap_or(1.0);
            let sharp = player_actor_ref.map(|p| p.sharp_aim_progress).unwrap_or(0.0);
            let final_scale = (bloom * (1.0 - 0.6 * sharp)).clamp(0.4, 10.0);
            transform.scale = Vec3::new(final_scale, final_scale, 1.0);
            *visibility = Visibility::Visible;
        }
        // Reticle color: red when tool refused, white otherwise.
        if let Some((_, _)) = reticle_query.iter_mut().next() {
            // ReticleRenderTag entity has Sprite; we don't carry it in this
            // query. The Sprite query is one block above (mut_query). We
            // separately update color in `update_reticle_color` below.
        }
    } else if let Some((_, mut visibility)) = reticle_query.iter_mut().next() {
        *visibility = Visibility::Hidden;
    }

    // Mark the resource clean for the next bridge write.
    state.set_changed();
}

/// actor. Each pip is a small colored rect anchored at the zone's anatomical
/// offset (Head at top, Hand-Right on the right side, Foot-Left at the bottom-
/// left, etc.) inside the actor's silhouette. The pip color reflects the zone's
/// external_integrity + destroyed flag + chassis stage tint, so a player
/// watching the wreck_eject scenario can SEE the right forearm pip turn black
/// when that zone is destroyed, and the whole silhouette turn red when the
/// chassis transitions to Wreck stage. This closes the M5-DC-3 gap from the
/// audit ("static sliding pawn" / "visible actor is still a M1 rectangle").
#[allow(clippy::too_many_arguments)]
fn sync_chassis_zone_sprites(
    mut commands: Commands,
    state: Res<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut zone_query: Query<(
        Entity,
        &ChassisZoneRenderTag,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    use std::collections::{HashMap, HashSet};

    // Map every existing zone-pip entity by (actor_id, zone) so we can update,
    // hide, or despawn per-frame.
    let mut existing: HashMap<(u64, String), Entity> = HashMap::new();
    for (entity, tag, _, _, _) in zone_query.iter() {
        existing.insert((tag.actor_id, tag.zone.clone()), entity);
    }

    let mut keep: HashSet<(u64, String)> = HashSet::new();

    for actor in &state.actors {
        // CHASSIS_ZONE_LAYOUT — chassis-attached actors use real per-zone
        // hp from `chassis.zones[]`; chassis-less actors (M1 baseline,
        // micro_breach, m2/m2.5/m4a scenarios) use a synthetic intact
        // chassis-view derived from the actor's HP so the visible body
        // STILL renders as a body, not a flat colored rectangle. This
        // closes the M5-DC-3 "static sliding pawn" gap for ALL scenarios,
        // not just M5.
        let (chassis_kind, chassis_stage) = if let Some(c) = actor.chassis.as_ref() {
            (c.kind.as_str(), c.stage.as_str())
        } else {
            ("infantry", "nominal")
        };
        let base_color = actor_color(actor);
        let stage_tint = chassis_stage_tint(chassis_stage);
        // Apply BOTH the chassis kind multiplier AND the global base scale
        // (`ACTOR_SILHOUETTE_BASE_SCALE`) so the on-screen silhouette is
        // visible at battlefield zoom.
        let scale = chassis_scale_multiplier(chassis_kind) * ACTOR_SILHOUETTE_BASE_SCALE;
        let (off_x, off_y, height_scale) = stance_offset(&actor.stance);
        let actor_pos = Vec2::new(actor.position[0], actor.position[1]);

        // Build a zone-data lookup so the 15-pip layout can pull
        // per-zone external/internal/core/wound integrity + destroyed flag.
        // For chassis-attached actors, populate from `chassis.zones[]`. For
        // chassis-less actors (M1 baseline, micro_breach, etc.), synthesize
        // from actor HP so the silhouette dims as the actor takes damage —
        // a fallback that keeps M1+ scenarios humanoid-looking without
        // requiring a real chassis grammar at those scenes.
        let mut zone_lookup: HashMap<&str, ActorChassisZoneView> = HashMap::new();
        if let Some(c) = actor.chassis.as_ref() {
            for z in &c.zones {
                zone_lookup.insert(
                    z.zone.as_str(),
                    ActorChassisZoneView {
                        zone: z.zone.clone(),
                        external_integrity: z.external_integrity,
                        internal_integrity: z.internal_integrity,
                        core_integrity: z.core_integrity,
                        wound_integrity: z.wound_integrity,
                        destroyed: z.destroyed,
                    },
                );
            }
        } else {
            // M1 baseline fallback: derive a synthetic intact body from
            // actor HP. The whole body dims uniformly as HP drops — no
            // per-zone damage, just enough to make the silhouette visible
            // and react to overall actor health.
            let hp_pct = if actor.hp_max > 0.0 {
                (actor.hp / actor.hp_max).clamp(0.0, 1.0)
            } else {
                1.0
            };
            for (zone_name, _, _, _, _) in CHASSIS_ZONE_LAYOUT {
                zone_lookup.insert(
                    zone_name,
                    ActorChassisZoneView {
                        zone: (*zone_name).to_string(),
                        external_integrity: hp_pct,
                        internal_integrity: hp_pct,
                        core_integrity: hp_pct,
                        wound_integrity: hp_pct,
                        destroyed: false,
                    },
                );
            }
        }

        // Walk-cycle leg offsets when stance is locomotive (Walking/Running).
        // Velocity sign is mirrored so legs cycle correctly when running L/R.
        let velocity_x = actor.velocity[0];
        let (left_leg_dy, right_leg_dy) = walk_cycle_offsets(state.tick, &actor.stance, velocity_x);

        for (zone_name, dx, dy, w, h) in CHASSIS_ZONE_LAYOUT {
            let default_view = ActorChassisZoneView {
                zone: (*zone_name).to_string(),
                external_integrity: 1.0,
                internal_integrity: 1.0,
                core_integrity: 1.0,
                wound_integrity: 1.0,
                destroyed: false,
            };
            let view = zone_lookup.get(zone_name).unwrap_or(&default_view);
            let color = zone_color(base_color, view, stage_tint);

            // Per-zone walk-cycle Y offset: shins+feet on each side bounce in
            // opposite phase so legs visibly alternate during locomotion.
            let walk_dy = match *zone_name {
                "leg_left" | "shin_left" | "foot_left" => left_leg_dy * 0.6,
                "leg_right" | "shin_right" | "foot_right" => right_leg_dy * 0.6,
                _ => 0.0,
            };

            // Apply stance offset + chassis kind scale + walk cycle.
            let pip_x = actor_pos.x + (dx * scale) + off_x;
            let pip_y = actor_pos.y + (dy * scale * height_scale) + off_y + walk_dy;
            let pip_w = w * scale;
            let pip_h = h * scale * height_scale;

            // Backpack renders behind torso (z = 0.45 vs 0.55 for others).
            let z = if *zone_name == "backpack" { 0.45 } else { 0.55 };

            let key = (actor.id, zone_name.to_string());
            keep.insert(key.clone());

            if let Some(entity) = existing.get(&key) {
                if let Ok((_, _, mut transform, mut sprite, mut visibility)) = zone_query.get_mut(*entity) {
                    transform.translation = Vec3::new(pip_x, pip_y, z);
                    sprite.color = color;
                    sprite.custom_size = Some(Vec2::new(pip_w, pip_h));
                    *visibility = Visibility::Inherited;
                }
            } else {
                commands.spawn((
                    solid_sprite(&solid, color, Vec2::new(pip_w, pip_h)),
                    Transform::from_translation(Vec3::new(pip_x, pip_y, z)),
                    ChassisZoneRenderTag {
                        actor_id: actor.id,
                        zone: zone_name.to_string(),
                    },
                    Name::new(format!("cf::render::chassis_zone::{}::{}", actor.id, zone_name)),
                ));
            }
        }
    }

    // Despawn pips whose owning actor + zone is no longer present (actor left
    // the world, chassis detached on eject, etc.).
    for (entity, tag, _, _, _) in zone_query.iter() {
        if !keep.contains(&(tag.actor_id, tag.zone.clone())) {
            commands.entity(entity).despawn();
        }
    }
}

/// sensor / weapon_mount / repair_drone) per chassis-attached actor. The pip
/// color reflects the module state and position follows the bound zone. This
/// surfaces simulation depth Cortex doesn't have — the silhouette visibly
/// shows which modules are still healthy, which are degraded, which failed.
#[allow(clippy::too_many_arguments)]
fn sync_chassis_module_sprites(
    mut commands: Commands,
    state: Res<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut module_query: Query<(
        Entity,
        &ChassisModuleRenderTag,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    use std::collections::{HashMap, HashSet};

    let mut existing: HashMap<(u64, String), Entity> = HashMap::new();
    for (entity, tag, _, _, _) in module_query.iter() {
        existing.insert((tag.actor_id, tag.module_id.clone()), entity);
    }
    let mut keep: HashSet<(u64, String)> = HashSet::new();

    for actor in &state.actors {
        let Some(chassis) = actor.chassis.as_ref() else {
            continue;
        };
        let actor_pos = Vec2::new(actor.position[0], actor.position[1]);
        let scale = chassis_scale_multiplier(&chassis.kind) * ACTOR_SILHOUETTE_BASE_SCALE;
        let (off_x, off_y, _height_scale) = stance_offset(&actor.stance);

        // Build zone position lookup so each module renders at its bound
        // zone's offset.
        let zone_offset = |zone: &str| -> (f32, f32) {
            for (zname, dx, dy, _, _) in CHASSIS_ZONE_LAYOUT {
                if *zname == zone {
                    return (*dx, *dy);
                }
            }
            (0.0, 0.0)
        };

        for module in &chassis.modules {
            let Some(color) = module_pip_color(&module.state) else {
                continue;
            };
            let (zdx, zdy) = zone_offset(&module.bound_zone);
            let (mdx, mdy, msize) = module_pip_offset(&module.kind);
            let pip_x = actor_pos.x + (zdx + mdx) * scale + off_x;
            let pip_y = actor_pos.y + (zdy + mdy) * scale + off_y;
            let pip_size = msize * scale;
            let z = 0.65; // Modules render in FRONT of zone pips.

            let key = (actor.id, module.id.clone());
            keep.insert(key.clone());

            if let Some(entity) = existing.get(&key) {
                if let Ok((_, _, mut transform, mut sprite, mut visibility)) = module_query.get_mut(*entity) {
                    transform.translation = Vec3::new(pip_x, pip_y, z);
                    sprite.color = color;
                    sprite.custom_size = Some(Vec2::new(pip_size, pip_size));
                    *visibility = Visibility::Inherited;
                }
            } else {
                commands.spawn((
                    solid_sprite(&solid, color, Vec2::new(pip_size, pip_size)),
                    Transform::from_translation(Vec3::new(pip_x, pip_y, z)),
                    ChassisModuleRenderTag {
                        actor_id: actor.id,
                        module_id: module.id.clone(),
                    },
                    Name::new(format!("cf::render::chassis_module::{}::{}", actor.id, module.id)),
                ));
            }
        }
    }

    for (entity, tag, _, _, _) in module_query.iter() {
        if !keep.contains(&(tag.actor_id, tag.module_id.clone())) {
            commands.entity(entity).despawn();
        }
    }
}

/// inventory carries a rifle. The rifle pip is a 12×3 rectangle anchored at
/// the right-hand zone (or torso for chassis-less actors), rotated to point
/// along the actor's aim vector. Without this, the actor has NO visible
/// weapon despite firing — only the projectile + muzzle flash hint at it.
fn sync_held_rifle_sprites(
    mut commands: Commands,
    state: Res<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut rifle_query: Query<(
        Entity,
        &HeldRifleRenderTag,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
) {
    use std::collections::{HashMap, HashSet};

    let mut existing: HashMap<u64, Entity> = HashMap::new();
    for (entity, tag, _, _, _) in rifle_query.iter() {
        existing.insert(tag.actor_id, entity);
    }
    let mut keep: HashSet<u64> = HashSet::new();

    for actor in &state.actors {
        // Only actors holding a rifle render a held weapon. `selected_item`
        // is the label produced by `InventoryItem::label()` — "rifle" for the
        // Rifle variant. Future melee/sidearm items follow the same pattern.
        if actor.selected_item != "rifle" {
            continue;
        }
        let actor_pos = Vec2::new(actor.position[0], actor.position[1]);
        let aim = Vec2::new(actor.aim[0], actor.aim[1]);
        let aim_unit = if aim.length_squared() > 1e-6 {
            aim.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };

        // Anchor at right-hand zone for every actor (chassis-attached uses
        // its kind multiplier; chassis-less uses Infantry default 0.9). Both
        // multiply by ACTOR_SILHOUETTE_BASE_SCALE so the rifle pip stays
        // proportional to the silhouette.
        let chassis_kind = actor.chassis.as_ref().map(|c| c.kind.as_str()).unwrap_or("infantry");
        let scale = chassis_scale_multiplier(chassis_kind) * ACTOR_SILHOUETTE_BASE_SCALE;
        let anchor_dx = 7.0 * scale;
        let anchor_dy = -3.0 * scale;
        let (off_x, off_y, _height_scale) = stance_offset(&actor.stance);
        // Rifle extends 8 px forward from the hand along aim direction.
        let muzzle_extend = 8.0 * scale;
        let rifle_center_x = actor_pos.x + anchor_dx + off_x + aim_unit.x * muzzle_extend * 0.5;
        let rifle_center_y = actor_pos.y + anchor_dy + off_y + aim_unit.y * muzzle_extend * 0.5;

        let rifle_color = if matches!(actor.team.as_str(), "blue") {
            Color::srgb(0.18, 0.20, 0.24)
        } else if matches!(actor.team.as_str(), "red") {
            Color::srgb(0.24, 0.18, 0.18)
        } else {
            Color::srgb(0.20, 0.20, 0.20)
        };
        let rifle_w = (12.0 * scale).max(8.0);
        let rifle_h = (2.5 * scale).max(2.0);
        let angle = aim_unit.y.atan2(aim_unit.x);

        keep.insert(actor.id);

        if let Some(entity) = existing.get(&actor.id) {
            if let Ok((_, _, mut transform, mut sprite, mut visibility)) = rifle_query.get_mut(*entity) {
                transform.translation = Vec3::new(rifle_center_x, rifle_center_y, 0.70);
                transform.rotation = Quat::from_rotation_z(angle);
                sprite.color = rifle_color;
                sprite.custom_size = Some(Vec2::new(rifle_w, rifle_h));
                *visibility = Visibility::Inherited;
            }
        } else {
            let mut transform = Transform::from_translation(Vec3::new(rifle_center_x, rifle_center_y, 0.70));
            transform.rotation = Quat::from_rotation_z(angle);
            commands.spawn((
                solid_sprite(&solid, rifle_color, Vec2::new(rifle_w, rifle_h)),
                transform,
                HeldRifleRenderTag { actor_id: actor.id },
                Name::new(format!("cf::render::held_rifle::{}", actor.id)),
            ));
        }
    }

    for (entity, tag, _, _, _) in rifle_query.iter() {
        if !keep.contains(&tag.actor_id) {
            commands.entity(entity).despawn();
        }
    }
}

/// M1.5: spawn / update / despawn breach strip sprites from the engine snapshot.
fn sync_breach_sprites(
    mut commands: Commands,
    state: Res<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut breach_query: Query<(Entity, &BreachRenderTag, &mut Transform, &mut Sprite)>,
) {
    use std::collections::HashMap;
    let mut existing: HashMap<String, Entity> = HashMap::new();
    for (entity, tag, _, _) in breach_query.iter() {
        existing.insert(tag.id.clone(), entity);
    }
    let mut keep: HashMap<String, ()> = HashMap::new();
    for breach in &state.breaches {
        keep.insert(breach.id.clone(), ());
        let centre = Vec2::new(
            (breach.bbox_min[0] + breach.bbox_max[0]) * 0.5,
            (breach.bbox_min[1] + breach.bbox_max[1]) * 0.5,
        );
        let size = Vec2::new(
            (breach.bbox_max[0] - breach.bbox_min[0]).max(1.0),
            (breach.bbox_max[1] - breach.bbox_min[1]).max(1.0),
        );
        let color = breach_color(breach);
        if let Some(entity) = existing.get(&breach.id) {
            if let Ok((_, _, mut transform, mut sprite)) = breach_query.get_mut(*entity) {
                transform.translation = Vec3::new(centre.x, centre.y, -0.25);
                sprite.color = color;
                sprite.custom_size = Some(size);
            }
        } else {
            commands.spawn((
                solid_sprite(&solid, color, size),
                Transform::from_translation(Vec3::new(centre.x, centre.y, -0.25)),
                BreachRenderTag { id: breach.id.clone() },
                Name::new(format!("cf::render::breach::{}", breach.id)),
            ));
        }
    }
    for (entity, tag, _, _) in breach_query.iter() {
        if !keep.contains_key(&tag.id) {
            commands.entity(entity).despawn();
        }
    }
}

/// M1.5: spawn / update / despawn the extraction-zone sprite.
fn sync_extraction_zone(
    mut commands: Commands,
    state: Res<ActorRenderState>,
    solid: Res<SolidSpriteImage>,
    mut zone_query: Query<(Entity, &mut Transform, &mut Sprite), With<ExtractionZoneTag>>,
) {
    match (&state.extraction_zone, zone_query.iter_mut().next()) {
        (Some(zone), Some((_, mut transform, mut sprite))) => {
            let centre = Vec2::new((zone.min[0] + zone.max[0]) * 0.5, (zone.min[1] + zone.max[1]) * 0.5);
            let size = Vec2::new(
                (zone.max[0] - zone.min[0]).max(1.0),
                (zone.max[1] - zone.min[1]).max(1.0),
            );
            transform.translation = Vec3::new(centre.x, centre.y, -0.4);
            sprite.color = if zone.completed {
                Color::srgba(0.30, 0.95, 0.50, 0.40)
            } else {
                Color::srgba(0.30, 0.85, 0.30, 0.25)
            };
            sprite.custom_size = Some(size);
        }
        (Some(zone), None) => {
            let centre = Vec2::new((zone.min[0] + zone.max[0]) * 0.5, (zone.min[1] + zone.max[1]) * 0.5);
            let size = Vec2::new(
                (zone.max[0] - zone.min[0]).max(1.0),
                (zone.max[1] - zone.min[1]).max(1.0),
            );
            commands.spawn((
                solid_sprite(&solid, Color::srgba(0.30, 0.85, 0.30, 0.25), size),
                Transform::from_translation(Vec3::new(centre.x, centre.y, -0.4)),
                ExtractionZoneTag,
                Name::new("cf::render::extraction_zone"),
            ));
        }
        (None, Some((entity, _, _))) => {
            commands.entity(entity).despawn();
        }
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_sprite_plugin_initialises_state() {
        let mut app = App::new();
        // Bevy 0.18: ActorSpritePlugin needs Assets<Image> for the
        // SolidSpriteImage 1x1 white texture (see SolidSpriteImage doc-comment).
        // MinimalPlugins doesn't include AssetPlugin/ImagePlugin, so we add the
        // asset registration manually for the unit test.
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(ActorSpritePlugin);
        app.update();
        let state = app.world().resource::<ActorRenderState>();
        assert!(state.actors.is_empty());
    }
}
