//! **M2**: chunked terrain renderer.
//!
//! Holds one `Handle<Image>` per allocated chunk; on a dirty signal the
//! renderer re-uploads only the dirty sub-rect to the corresponding
//! `Image::data` (NOT the whole chunk texture). Mirrors the CCCP
//! `SLTerrain::EraseSilhouette` + `AddUpdatedMaterialArea` contract.
//!
//! The renderer is sim-agnostic: cf-app bridges `ChunkedTerrain` state into
//! the [`ChunkedTerrainSnapshot`] resource each frame; the renderer reads
//! that and updates Bevy entities. The bridge also drains
//! `ChunkedTerrain::dirty_chunks` and feeds it as a Vec of `ChunkUpdate`s,
//! then calls `ChunkedTerrain::clear_dirty()` so the next frame doesn't
//! re-upload unchanged chunks (canonical contract).
//!
//! Color resolution: each material's overlay color comes from the
//! `cf_terrain::MaterialAffordance::overlay_rgba` runtime table; the live
//! shader-tint applied by `overlay.rs` overlays the canonical chunk texture
//! per-mode without re-uploading.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use cf_terrain::{material_affordance, MaterialId, CHUNK_SIZE};

/// One chunk update queued by the cf-app bridge for this frame.
#[derive(Debug, Clone)]
pub struct ChunkUpdate {
    /// Chunk coordinates in chunk-space.
    pub cx: i32,
    pub cy: i32,
    /// Inclusive pixel-space dirty rect in chunk-local coordinates,
    /// `[lx0, ly0, lx1, ly1]`. The renderer uploads only this sub-rect.
    pub dirty_rect: [u32; 4],
    /// Pixel data in row-major chunk-local order (length CHUNK_SIZE^2).
    /// The renderer reads only the dirty sub-rect from this buffer.
    pub pixels: Vec<MaterialId>,
}

/// Render-side snapshot of the engine's chunked terrain. cf-app's bridge
/// writes this each frame from `ChunkedTerrain::material_counts() / anchor /
/// drained dirty chunks`. The renderer reads it and updates per-chunk
/// `Image` textures.
#[derive(Resource, Debug, Clone, Default)]
pub struct ChunkedTerrainSnapshot {
    /// World-space bottom-left anchor of the terrain (must match the
    /// `region_anchor_*` so non-(0,0) scenarios render aligned).
    pub anchor: [f32; 2],
    /// True iff the engine has a chunked terrain loaded. When false the
    /// renderer despawns every chunk it has spawned (`scenario.reset` to a
    /// non-terrain scene path).
    pub active: bool,
    /// Per-frame chunk updates: every chunk the bridge drained from
    /// `ChunkedTerrain::dirty_chunks()` this tick. Cleared after the
    /// renderer reads them.
    pub updates: Vec<ChunkUpdate>,
    /// Bookkeeping: total dirty-rect uploads applied this frame (debug).
    pub uploads_applied_this_frame: u32,
}

/// One spawned chunk entity tag. Carries the chunk coord so the renderer
/// can update / despawn it.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChunkRenderTag {
    pub cx: i32,
    pub cy: i32,
}

/// Per-chunk image storage. Keyed by chunk coord.
#[derive(Resource, Debug, Default)]
pub struct ChunkImages {
    pub handles: HashMap<(i32, i32), Handle<Image>>,
}

/// Plugin that registers the chunked-terrain renderer. Schedules the upload
/// system on `Update` after cf-app's bridge writes the snapshot.
pub struct ChunkedTerrainRendererPlugin;

impl Plugin for ChunkedTerrainRendererPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkedTerrainSnapshot>()
            .init_resource::<ChunkImages>()
            .add_systems(Update, sync_chunk_textures);
    }
}

/// Map material id → RGBA pixel (matches the runtime affordance table).
#[must_use]
pub fn material_rgba(id: MaterialId) -> [u8; 4] {
    match material_affordance(id) {
        Some(aff) => aff.overlay_rgba,
        None => [0, 0, 0, 0],
    }
}

/// Build a fresh chunk texture image with row-major RGBA8 pixels matching
/// the supplied material grid.
#[must_use]
pub fn build_chunk_image(pixels: &[MaterialId]) -> Image {
    let size = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);
    debug_assert_eq!(pixels.len(), size);
    let mut data = Vec::with_capacity(size * 4);
    for &mat in pixels {
        let [r, g, b, a] = material_rgba(mat);
        data.extend_from_slice(&[r, g, b, a]);
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: CHUNK_SIZE,
            height: CHUNK_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.data = Some(data);
    image
}

/// Apply a dirty sub-rect to an existing chunk image. Only writes the
/// pixels inside `[lx0..=lx1, ly0..=ly1]`; the rest of the texture is
/// untouched (this is the M2 "re-upload only the dirty sub-rect" contract).
pub fn apply_dirty_rect(image: &mut Image, pixels: &[MaterialId], dirty_rect: [u32; 4]) -> u32 {
    let [lx0, ly0, lx1, ly1] = dirty_rect;
    let lx0 = lx0.min(CHUNK_SIZE - 1);
    let ly0 = ly0.min(CHUNK_SIZE - 1);
    let lx1 = lx1.min(CHUNK_SIZE - 1);
    let ly1 = ly1.min(CHUNK_SIZE - 1);
    let row_stride = (CHUNK_SIZE as usize) * 4;
    let Some(data) = image.data.as_mut() else {
        return 0;
    };
    let mut applied: u32 = 0;
    for ly in ly0..=ly1 {
        let row_off = (ly as usize) * row_stride;
        for lx in lx0..=lx1 {
            let px_off = row_off + (lx as usize) * 4;
            let mat_idx = (ly as usize) * (CHUNK_SIZE as usize) + (lx as usize);
            if mat_idx >= pixels.len() || px_off + 4 > data.len() {
                continue;
            }
            let [r, g, b, a] = material_rgba(pixels[mat_idx]);
            data[px_off] = r;
            data[px_off + 1] = g;
            data[px_off + 2] = b;
            data[px_off + 3] = a;
            applied += 1;
        }
    }
    applied
}

/// Render-side system: consume the snapshot's queued updates, spawn /
/// update per-chunk sprites, and re-upload only the dirty sub-rects.
fn sync_chunk_textures(
    mut commands: Commands,
    mut snapshot: ResMut<ChunkedTerrainSnapshot>,
    mut chunk_images: ResMut<ChunkImages>,
    mut images: ResMut<Assets<Image>>,
    mut chunks_q: Query<(Entity, &ChunkRenderTag, &mut Transform)>,
) {
    if !snapshot.active {
        if !chunk_images.handles.is_empty() {
            chunk_images.handles.clear();
            for (entity, _, _) in chunks_q.iter() {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let cs_world = CHUNK_SIZE as f32;
    let anchor = snapshot.anchor;
    let updates = std::mem::take(&mut snapshot.updates);
    let mut applied_total: u32 = 0;

    for update in updates {
        let key = (update.cx, update.cy);
        let exists = chunk_images.handles.contains_key(&key);
        if !exists {
            let image = build_chunk_image(&update.pixels);
            let handle = images.add(image);
            chunk_images.handles.insert(key, handle.clone());
            let center_x = anchor[0] + (update.cx as f32 + 0.5) * cs_world;
            let center_y = anchor[1] + (update.cy as f32 + 0.5) * cs_world;
            commands.spawn((
                Sprite {
                    image: handle,
                    custom_size: Some(Vec2::new(cs_world, cs_world)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(center_x, center_y, -0.6)),
                ChunkRenderTag {
                    cx: update.cx,
                    cy: update.cy,
                },
                Name::new(format!("cf::render::terrain::chunk_{}_{}", update.cx, update.cy)),
            ));
            applied_total = applied_total.saturating_add(CHUNK_SIZE * CHUNK_SIZE);
            continue;
        }
        let handle = chunk_images
            .handles
            .get(&key)
            .cloned()
            .expect("handle exists by virtue of contains_key");
        if let Some(image) = images.get_mut(&handle) {
            applied_total = applied_total.saturating_add(apply_dirty_rect(image, &update.pixels, update.dirty_rect));
        }
    }
    snapshot.uploads_applied_this_frame = applied_total;
    // Keep transform aligned with anchor for already-spawned chunks (handles
    // scenario.reset that may move the anchor before the bridge rebuilds).
    for (_, tag, mut transform) in chunks_q.iter_mut() {
        let center_x = anchor[0] + (tag.cx as f32 + 0.5) * cs_world;
        let center_y = anchor[1] + (tag.cy as f32 + 0.5) * cs_world;
        transform.translation.x = center_x;
        transform.translation.y = center_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_terrain::{MATERIAL_AIR, MATERIAL_CONCRETE, MATERIAL_DIRT};

    fn uniform(mat: MaterialId) -> Vec<MaterialId> {
        vec![mat; (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize)]
    }

    #[test]
    fn material_rgba_resolves_for_launch_set() {
        let dirt = material_rgba(MATERIAL_DIRT);
        let concrete = material_rgba(MATERIAL_CONCRETE);
        assert_ne!(dirt, concrete);
        // Air is transparent.
        assert_eq!(material_rgba(MATERIAL_AIR), [0, 0, 0, 0]);
    }

    #[test]
    fn build_chunk_image_produces_full_buffer() {
        let pixels = uniform(MATERIAL_DIRT);
        let image = build_chunk_image(&pixels);
        let buf = image.data.as_ref().expect("image data");
        assert_eq!(buf.len(), (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize) * 4);
        let [r, g, b, a] = material_rgba(MATERIAL_DIRT);
        assert_eq!(&buf[0..4], &[r, g, b, a]);
    }

    #[test]
    fn apply_dirty_rect_only_writes_inside_bounds() {
        // Start with a uniform AIR chunk, then mark a 100..200 horizontal
        // strip as DIRT and apply a dirty sub-rect for that strip. The
        // pixels OUTSIDE the strip must remain AIR.
        let mut pixels = uniform(MATERIAL_AIR);
        let lx0 = 100u32;
        let lx1 = 199u32;
        let ly0 = 50u32;
        let ly1 = 60u32;
        for ly in ly0..=ly1 {
            for lx in lx0..=lx1 {
                let idx = (ly as usize) * (CHUNK_SIZE as usize) + (lx as usize);
                pixels[idx] = MATERIAL_DIRT;
            }
        }
        let mut image = build_chunk_image(&uniform(MATERIAL_AIR));
        let applied = apply_dirty_rect(&mut image, &pixels, [lx0, ly0, lx1, ly1]);
        let expected = (lx1 - lx0 + 1) * (ly1 - ly0 + 1);
        assert_eq!(applied, expected);

        let buf = image.data.as_ref().expect("image data");
        let row_stride = (CHUNK_SIZE as usize) * 4;
        // Pixel at (lx0, ly0) must equal DIRT color.
        let off_dirt = (ly0 as usize) * row_stride + (lx0 as usize) * 4;
        let [dr, dg, db, da] = material_rgba(MATERIAL_DIRT);
        assert_eq!(&buf[off_dirt..off_dirt + 4], &[dr, dg, db, da]);
        // Pixel at (0, 0) must remain AIR (untouched).
        let off_outside = 0usize;
        let [ar, ag, ab, aa] = material_rgba(MATERIAL_AIR);
        assert_eq!(&buf[off_outside..off_outside + 4], &[ar, ag, ab, aa]);
    }

    #[test]
    fn snapshot_drives_chunk_spawn_and_update() {
        // Headless-friendly sim of the renderer's input + output: enqueue a
        // single chunk update against a fresh snapshot, advance the system,
        // assert the ChunkImages handle map gains an entry and the dirty
        // sub-rect uploads were applied.
        let mut app = App::new();
        // Minimal Bevy plugins: Assets<Image> is required for the renderer.
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(ChunkedTerrainRendererPlugin);
        // First write the snapshot, then call update.
        {
            let mut snap = app.world_mut().resource_mut::<ChunkedTerrainSnapshot>();
            snap.active = true;
            snap.anchor = [0.0, 0.0];
            snap.updates.push(ChunkUpdate {
                cx: 0,
                cy: 0,
                dirty_rect: [100, 50, 199, 60],
                pixels: uniform(MATERIAL_DIRT),
            });
        }
        app.update();
        let chunk_images = app.world().resource::<ChunkImages>();
        assert!(chunk_images.handles.contains_key(&(0, 0)));
        let snap = app.world().resource::<ChunkedTerrainSnapshot>();
        assert!(snap.updates.is_empty(), "renderer must drain queued updates");
        assert!(snap.uploads_applied_this_frame > 0);
    }

    #[test]
    fn snapshot_inactive_clears_chunks() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(ChunkedTerrainRendererPlugin);
        {
            let mut snap = app.world_mut().resource_mut::<ChunkedTerrainSnapshot>();
            snap.active = true;
            snap.updates.push(ChunkUpdate {
                cx: 0,
                cy: 0,
                dirty_rect: [0, 0, 255, 255],
                pixels: uniform(MATERIAL_DIRT),
            });
        }
        app.update();
        assert!(app.world().resource::<ChunkImages>().handles.contains_key(&(0, 0)));
        // Now toggle inactive — handles must clear next frame.
        {
            let mut snap = app.world_mut().resource_mut::<ChunkedTerrainSnapshot>();
            snap.active = false;
        }
        app.update();
        assert!(app.world().resource::<ChunkImages>().handles.is_empty());
    }
}
