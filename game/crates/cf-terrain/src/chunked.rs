//! M2 chunked pixel terrain.
//!
//! Per the canonical roadmap (M2 — Pixel Terrain And Materials) this module owns:
//!
//! - The chunked storage (`ChunkedTerrain`, `Chunk`, `ChunkCoord`) with sparse
//!   lazy allocation: chunks that match the default material are not stored.
//! - The DR-007 launch material set (8 ids) and the `MaterialAffordance` lookup.
//! - Carve / blast / fill primitives the engine drives from `act.player.dig`,
//!   blast outcomes, and scenario seeding.
//! - Dirty-region tracking the renderer consumes (one frame latency).
//! - Layout-stable `checksum_bytes` and JSON-serializable snapshot for M3A.
//! - Physics integration helpers (`aabb_overlaps_solid`, `column_top_solid_y`).
//!
//! Anti-scope (lands at M5.6 Material Kernel):
//! - Active CA / reaction table / phase change.
//! - Fluid/gas pressure or atmospherics.
//! - Heat conduction.
//!
//! Anti-scope (lands at M2's GPU path follow-up):
//! - GPU-assisted carving compute shader. The M2 Universal Enhancement row
//!   ("GPU compute path investigation") is satisfied by `notes.md` plus the
//!   deterministic CPU baseline implemented here. The CPU path stays the source
//!   of truth for replay determinism per DR-054.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::integrity::{
    apply_damage_formula, normalized_hardness, CascadeEvent, DamageKind, IntegrityBand, PenetrationOutcome, PixelMeta,
    PixelMetaGrid, PixelMetaKey, DEFAULT_CASCADE_DECAY_PCT, DEFAULT_CASCADE_DEPTH, DEFAULT_CASCADE_THRESHOLD,
};

pub use crate::chunked_materials::{
    material_affordance, material_id_from_name, material_name_from_id, MaterialAffordance, MaterialId,
    MATERIAL_AIR, MATERIAL_ANCHOR, MATERIAL_CONCRETE, MATERIAL_DIRT, MATERIAL_HAZARD, MATERIAL_LOOSE_FILL,
    MATERIAL_METAL_NOHOOK, MATERIAL_REPAIR_FILL, MATERIAL_SCHEMA_VERSION, MATERIAL_SUPPORT_BEAM,
};

/// 256x256 chunk size — matches the canonical roadmap M2 scope ("256×256
/// chunks; per-pixel material id; sparse storage"). Stored as `u32` so chunk
/// math doesn't sign-extend through `usize` casts.
pub const CHUNK_SIZE: u32 = 256;
const CHUNK_PIXELS: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);

/// One chunk in the terrain. Stored densely as a row-major array of `MaterialId`s.
/// Sparse: chunks with all pixels equal to the terrain's `default_material` are
/// NOT stored at all — `ChunkedTerrain::material_at` returns the default in
/// that case.
///
/// **M3 re-open (2026-05-13) fix #7**: forward-compat fields for M14 (sub-rect
/// upload) + M15 (active material kernel). All four extra fields are
/// `serde(default)` so legacy v0.1 snapshots round-trip cleanly:
/// - `active_region: bool` — M15 active-cell hint; true when any cell in this
///   chunk participates in an ongoing reaction (per CCCP active-region pattern)
/// - `last_modified_tick: u64` — bumped by every successful `set_pixel`; used
///   by M22 pathfinder + M15 active-region eviction policy
/// - `color_grid: Option<Vec<u32>>` — M14 per-pixel color cache (Noita-grade
///   visual variation); None at M3 means "render the material's color_hex"
/// - `dirty_rect: Option<DirtyRect>` — M14 sub-rect upload bound. None at M3
///   means "uploader assumes whole-chunk dirty"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pixels: Vec<MaterialId>,
    #[serde(default)]
    pub active_region: bool,
    #[serde(default)]
    pub last_modified_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_grid: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dirty_rect: Option<DirtyRect>,
}

/// **M3 re-open (2026-05-13) fix #7 + fix #6**: per-chunk dirty AABB for
/// sub-rect upload (M14 forward-compat). Coordinates are LOCAL to the chunk
/// (0..CHUNK_SIZE). `min`/`max` inclusive. None means "no pending dirty
/// region"; the chunk is considered up-to-date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRect {
    pub min: [u32; 2],
    pub max: [u32; 2],
}

impl DirtyRect {
    pub fn single(lx: u32, ly: u32) -> Self {
        Self {
            min: [lx, ly],
            max: [lx, ly],
        }
    }

    pub fn extend(&mut self, lx: u32, ly: u32) {
        self.min[0] = self.min[0].min(lx);
        self.min[1] = self.min[1].min(ly);
        self.max[0] = self.max[0].max(lx);
        self.max[1] = self.max[1].max(ly);
    }
}

impl Chunk {
    fn uniform(material: MaterialId) -> Self {
        Self {
            pixels: vec![material; CHUNK_PIXELS],
            active_region: false,
            last_modified_tick: 0,
            color_grid: None,
            dirty_rect: None,
        }
    }

    /// Material at `(lx, ly)` inside this chunk; `lx`/`ly` are local 0..CHUNK_SIZE.
    pub fn pixel(&self, lx: u32, ly: u32) -> MaterialId {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        self.pixels[(ly as usize) * (CHUNK_SIZE as usize) + (lx as usize)]
    }

    /// Set the material at `(lx, ly)`; returns true if the cell changed.
    /// **M3 re-open fix #6/#7**: also extends `dirty_rect` to cover this pixel
    /// and bumps `last_modified_tick` (caller is responsible for passing the
    /// current tick via `set_pixel_at_tick`). For backward compat, this
    /// 3-arg form leaves `last_modified_tick` alone.
    pub fn set_pixel(&mut self, lx: u32, ly: u32, mat: MaterialId) -> bool {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        let idx = (ly as usize) * (CHUNK_SIZE as usize) + (lx as usize);
        let prev = self.pixels[idx];
        if prev == mat {
            return false;
        }
        self.pixels[idx] = mat;
        // Extend the dirty rect so the M14 sub-rect upload bridge can re-upload
        // only the affected sub-region instead of the whole 256×256 chunk.
        match &mut self.dirty_rect {
            Some(rect) => rect.extend(lx, ly),
            None => self.dirty_rect = Some(DirtyRect::single(lx, ly)),
        }
        true
    }

    /// **M3 re-open fix #6/#7**: M15 forward-compat — set the material AND
    /// stamp `last_modified_tick`. Returns true if the cell changed.
    pub fn set_pixel_at_tick(&mut self, lx: u32, ly: u32, mat: MaterialId, tick: u64) -> bool {
        if self.set_pixel(lx, ly, mat) {
            self.last_modified_tick = tick;
            true
        } else {
            false
        }
    }

    /// **M3 re-open fix #6**: drain the per-chunk dirty rect (consumes it).
    /// Returns `None` when the chunk has nothing pending. Caller uploads the
    /// returned sub-rect through the render bridge.
    pub fn take_dirty_rect(&mut self) -> Option<DirtyRect> {
        self.dirty_rect.take()
    }

    /// True if every pixel equals `mat`. Used to compress chunks back to the
    /// default after a fill operation.
    pub fn is_uniform(&self, mat: MaterialId) -> bool {
        self.pixels.iter().all(|m| *m == mat)
    }

    /// Number of solid pixels in this chunk. Used by `aabb_overlaps_solid` and
    /// the run-bundle summary to track how much of the world is currently solid.
    pub fn solid_count(&self, registry: &MaterialRegistry) -> u32 {
        self.pixels.iter().filter(|m| registry.is_solid(**m)).count() as u32
    }
}

/// Chunk coordinate in chunk-space (not pixel-space). `(0, 0)` covers pixels
/// `[0..256, 0..256]`; `(1, 0)` covers `[256..512, 0..256]`; etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    pub const fn new(cx: i32, cy: i32) -> Self {
        Self { cx, cy }
    }

    /// Pixel-space origin of this chunk's `(0, 0)` corner.
    pub fn pixel_origin(&self) -> [i64; 2] {
        [
            (self.cx as i64) * (CHUNK_SIZE as i64),
            (self.cy as i64) * (CHUNK_SIZE as i64),
        ]
    }
}

/// Wrapper around `[MaterialAffordance; 8]` so consumers can ask "is this id
/// solid?" / "is this id diggable?" without touching the constants directly.
#[derive(Debug, Clone, Copy)]
pub struct MaterialRegistry;

impl MaterialRegistry {
    pub fn affordance(&self, id: MaterialId) -> Option<&'static MaterialAffordance> {
        material_affordance(id)
    }

    pub fn is_solid(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(MaterialAffordance::is_solid)
    }

    pub fn is_diggable(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.diggable)
    }

    pub fn refusal_reason(&self, id: MaterialId) -> Option<&'static str> {
        self.affordance(id).and_then(|m| m.refusal_reason)
    }

    pub fn name(&self, id: MaterialId) -> &'static str {
        self.affordance(id).map(|m| m.name).unwrap_or("unknown")
    }

    pub fn hardness(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.hardness).unwrap_or(0.0)
    }

    pub fn is_hazard(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.hazard)
    }

    pub fn damage_per_tick(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.damage_per_tick).unwrap_or(0.0)
    }

    pub fn is_anchorable(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.anchorable)
    }

    pub fn path_cost(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.path_cost).unwrap_or(1.0)
    }

    pub fn stickiness(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.stickiness).unwrap_or(0.0)
    }

    pub fn spawn_material(&self, id: MaterialId) -> Option<MaterialId> {
        self.affordance(id).and_then(|m| m.spawn_material)
    }
}

/// One outcome of a `try_carve` / `try_blast` call.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkedCarveOutcome {
    Carved(ChunkedCarveStats),
    Refused(ChunkedCarveRefusal),
    NoOp(ChunkedCarveNoOp),
}

/// One successful carve. `bbox_min/max` are inclusive pixel-space bounds
/// covering every affected pixel; `count` is the number of pixels removed.
/// `material_*` records the dominant (mode) material the carve removed so the
/// HUD + replay viewer can label the action ("dug 28 dirt pixels", etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveStats {
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    pub count: u32,
    pub dominant_material: MaterialId,
    pub dirty_chunks: Vec<ChunkCoord>,
    pub refusal_reason: Option<&'static str>,
}

/// Carve refused. Distinct from `NoOp` because refusals must be visible in the
/// HUD + replay (player learns why their dig didn't work). M2 surfaces three
/// refusal reasons in the launch set: `material_metal_nohook`, `material_hazard`,
/// and the legacy alias `out_of_range` for empty carves.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveRefusal {
    pub reason: &'static str,
    pub probe_at: [i64; 2],
    pub material: MaterialId,
}

/// Carve hit only `air` (or some other non-solid material). Engine emits this
/// silently — no `tool_refused`, no `terrain_carved`. The HUD displays an
/// "out of range" cue.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveNoOp {
    pub probe_at: [i64; 2],
}

/// One terrain stamp from the scenario manifest. Stamps run in declaration
/// order; later stamps overwrite earlier ones. Convenient for hand-authoring
/// test scenarios without a full level editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerrainStamp {
    /// Fill an axis-aligned box in pixel space with `material`.
    FillAabb {
        min: [f32; 2],
        max: [f32; 2],
        material: String,
    },
    /// Fill a circular region in pixel space with `material`.
    FillCircle {
        center: [f32; 2],
        radius: f32,
        material: String,
    },
}

/// JSON-serializable snapshot of the chunked terrain. Used by the M3A snapshot
/// pipeline + the replay viewer + the cf-headless replay verifier. Layout is
/// stable: future migrations bump `schema` and register a converter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkedTerrainSnapshot {
    pub schema: String,
    pub width_px: u32,
    pub height_px: u32,
    pub anchor: [f32; 2],
    pub default_material: MaterialId,
    pub carve_count: u64,
    pub refusal_count: u64,
    pub material_counts: BTreeMap<String, u64>,
    pub chunks: Vec<ChunkedTerrainSnapshotChunk>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkedTerrainSnapshotChunk {
    pub coord: ChunkCoord,
    /// Per-pixel material ids, row-major, length `CHUNK_SIZE * CHUNK_SIZE`.
    pub pixels: Vec<MaterialId>,
}

/// Chunked terrain core. Owns the chunk map, dirty regions, perf counters, and
/// snapshot/checksum APIs. The engine wraps this in `RwLock` so the per-tick
/// dig handler can mutate while the renderer reads.
#[derive(Debug, Clone)]
pub struct ChunkedTerrain {
    pub anchor: [f32; 2],
    pub width_px: u32,
    pub height_px: u32,
    pub default_material: MaterialId,
    chunks: BTreeMap<ChunkCoord, Chunk>,
    dirty_chunks: BTreeSet<ChunkCoord>,
    pub carve_count: u64,
    pub refusal_count: u64,
    pub registry: MaterialRegistry,
    /// **M3 re-audit pass 4 (2026-05-13)**: most-recent sim tick stamped
    /// onto chunks that were modified this tick. Engine calls
    /// `set_current_tick` from `drive_tick` so every subsequent pixel
    /// write updates the affected chunk's `last_modified_tick`. Default
    /// 0 (chunks never modified). Not serialized (transient runtime state).
    pub current_tick: u64,
    /// **M9 § Destructible terrain — 5-tier per-pixel integrity grid**.
    /// Sparse: only damaged pixels appear. Air pixels are never tracked.
    /// Pristine (1.0) pixels are implicit — they only get an entry once
    /// damage lands. Layout-stable across runs (BTreeMap ordering) so
    /// snapshot round-trips preserve determinism for M4's `sim_state_v1`
    /// checksum scope.
    pub pixel_meta_grid: PixelMetaGrid,
}

impl ChunkedTerrain {
    /// New empty terrain with `width_px x height_px` extent and a uniform
    /// default material across every cell.
    pub fn new(width_px: u32, height_px: u32, default_material: MaterialId) -> Self {
        Self {
            anchor: [0.0, 0.0],
            width_px,
            height_px,
            default_material,
            chunks: BTreeMap::new(),
            dirty_chunks: BTreeSet::new(),
            carve_count: 0,
            refusal_count: 0,
            registry: MaterialRegistry,
            current_tick: 0,
            pixel_meta_grid: PixelMetaGrid::new(),
        }
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: engine calls this each tick so
    /// the next pixel write stamps the right `last_modified_tick` on the
    /// affected chunk.
    pub fn set_current_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Apply a list of [`TerrainStamp`]s. Returns the number of pixels written.
    /// Stamps run in declaration order; later stamps overwrite earlier ones.
    pub fn apply_stamps(&mut self, stamps: &[TerrainStamp]) -> u64 {
        let mut written: u64 = 0;
        for stamp in stamps {
            match stamp {
                TerrainStamp::FillAabb { min, max, material } => {
                    let Some(mat) = material_id_from_name(material) else {
                        tracing::warn!(
                            target: "cf::terrain",
                            material = %material,
                            "fill_aabb: unknown material; skipping"
                        );
                        continue;
                    };
                    written += self.fill_aabb(*min, *max, mat);
                }
                TerrainStamp::FillCircle {
                    center,
                    radius,
                    material,
                } => {
                    let Some(mat) = material_id_from_name(material) else {
                        tracing::warn!(
                            target: "cf::terrain",
                            material = %material,
                            "fill_circle: unknown material; skipping"
                        );
                        continue;
                    };
                    written += self.fill_circle(*center, *radius, mat);
                }
            }
        }
        // Stamps are an init step; clear dirty so the renderer doesn't think
        // these are interactive carves to flash.
        self.dirty_chunks.clear();
        written
    }

    /// Fill a closed pixel-space AABB with `mat`. Returns the number of pixels
    /// changed. Coordinates are clamped to the terrain extent.
    pub fn fill_aabb(&mut self, min: [f32; 2], max: [f32; 2], mat: MaterialId) -> u64 {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        let mut written: u64 = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                if self.set_pixel_internal(px, py, mat) {
                    written += 1;
                }
            }
        }
        written
    }

    /// Fill a circular region in pixel space with `mat`. Returns the number of
    /// pixels changed.
    pub fn fill_circle(&mut self, center: [f32; 2], radius: f32, mat: MaterialId) -> u64 {
        let r = radius.max(0.0);
        let r2 = r * r;
        let cx = center[0];
        let cy = center[1];
        let min = [cx - r, cy - r];
        let max = [cx + r, cy + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: convert center world-space → pixel-space so
        // the circle test matches the iteration bounds (which are
        // pixel-space coming out of `aabb_to_pixels`).
        let center_px_x = cx - self.anchor[0];
        let center_px_y = cy - self.anchor[1];
        let mut written: u64 = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy <= r2 && self.set_pixel_internal(px, py, mat) {
                    written += 1;
                }
            }
        }
        written
    }

    /// Material at the given pixel coordinate. Returns the terrain default for
    /// out-of-range or unallocated chunks.
    pub fn material_at(&self, px: i64, py: i64) -> MaterialId {
        if !self.in_bounds(px, py) {
            return self.default_material;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        match self.chunks.get(&coord) {
            Some(c) => c.pixel(lx, ly),
            None => self.default_material,
        }
    }

    /// Material at world-space coordinates (continuous floats). The terrain
    /// anchor + integer flooring resolve world space to pixel space.
    pub fn material_at_world(&self, world_x: f32, world_y: f32) -> MaterialId {
        let px = (world_x - self.anchor[0]).floor() as i64;
        let py = (world_y - self.anchor[1]).floor() as i64;
        self.material_at(px, py)
    }

    /// Try to carve a circular region. Tool semantics:
    ///
    /// - Pixels matching the carve mask AND `is_diggable(mat) = true` become
    ///   `default_material` (typically `air`).
    /// - Pixels matching the mask AND `is_diggable(mat) = false` AND with a
    ///   `refusal_reason` short-circuit the carve and return
    ///   [`ChunkedCarveOutcome::Refused`] before any other pixel changes.
    /// - When the mask hits only non-solid (`air`) pixels, we return
    ///   [`ChunkedCarveOutcome::NoOp`] so callers can label "out_of_range".
    ///
    /// This is the same vocabulary as `cf-terrain::BreachStrip::try_dig` so
    /// existing M1.5 consumers don't need migration.
    pub fn try_carve(&mut self, origin: [f32; 2], radius: f32) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: `aabb_to_pixels` subtracts the terrain anchor
        // when converting world → pixel space, so `px` / `py` iterate in
        // pixel-space. `origin` is world-space. Compare in pixel-space by
        // pre-subtracting the anchor from the origin so the carve circle
        // isn't off-center for non-zero anchors.
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];

        // First pass: probe for refusal-reason materials. A refusal short-circuits
        // the whole carve so the player gets a clean "this won't work" event.
        let mut probe = [center_px_x.round() as i64, center_px_y.round() as i64];
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if let Some(reason) = self.registry.refusal_reason(mat) {
                    self.refusal_count += 1;
                    return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                        reason,
                        probe_at: [px, py],
                        material: mat,
                    });
                }
                probe = [px, py];
            }
        }

        // Second pass: carve diggable pixels. Track the mode material removed
        // so the event payload can label the carve.
        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        let air = self.default_material;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if !self.registry.is_diggable(mat) {
                    continue;
                }
                if self.set_pixel_internal(px, py, air) {
                    *counts.entry(mat).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp { probe_at: probe });
        }
        self.carve_count += 1;
        let dominant_material = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// Larger blast carve: same semantics as `try_carve` but with a wider
    /// default radius and ignores refusal-reason materials when `force` is
    /// strictly above the registered hardness. M2 ships a CPU baseline; the
    /// GPU compute path lands as a follow-up.
    pub fn try_blast(&mut self, origin: [f32; 2], radius: f32, force: f32) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: same world→pixel anchor subtraction as
        // try_carve. The blast circle test must compare px (pixel-space)
        // against an anchor-subtracted origin so non-zero anchors don't
        // off-center the blast.
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];

        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        let mut hardest_blocked: Option<(MaterialId, &'static str)> = None;
        let air = self.default_material;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if mat == air {
                    continue;
                }
                let aff = match self.registry.affordance(mat) {
                    Some(a) => a,
                    None => continue,
                };
                if !aff.diggable {
                    if force >= aff.hardness {
                        // Force overcomes the hardness; the blast clears it.
                    } else {
                        if let Some(reason) = aff.refusal_reason {
                            hardest_blocked = Some((mat, reason));
                        }
                        continue;
                    }
                }
                if self.set_pixel_internal(px, py, air) {
                    *counts.entry(mat).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            if let Some((mat, reason)) = hardest_blocked {
                self.refusal_count += 1;
                return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                    reason,
                    probe_at: [center_px_x.round() as i64, center_px_y.round() as i64],
                    material: mat,
                });
            }
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp {
                probe_at: [center_px_x.round() as i64, center_px_y.round() as i64],
            });
        }
        self.carve_count += 1;
        let dominant_material = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// True if any pixel inside the closed AABB `min..=max` is solid (per
    /// `MaterialRegistry::is_solid`). Used by the physics integration so an
    /// actor cannot stand on top of an air column or walk through a concrete
    /// pillar.
    pub fn aabb_overlaps_solid(&self, min: [f32; 2], max: [f32; 2]) -> bool {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        for py in y0..y1 {
            for px in x0..x1 {
                if self.registry.is_solid(self.material_at(px, py)) {
                    return true;
                }
            }
        }
        false
    }

    /// Highest solid pixel y-coordinate inside the closed pixel-x range
    /// `[x0..x1]`. `None` when the column is entirely air. Used by physics to
    /// resolve actor "stand on terrain" without per-tick contact manifold work.
    pub fn column_top_solid_y(&self, x0: i64, x1: i64, y_max: i64) -> Option<i64> {
        let mut best: Option<i64> = None;
        for px in x0..=x1 {
            for py in (0..=y_max).rev() {
                if self.registry.is_solid(self.material_at(px, py)) {
                    best = Some(best.map(|b| b.max(py)).unwrap_or(py));
                    break;
                }
            }
        }
        best
    }

    /// Iterate over the dirty chunks (consumed by render). Caller must call
    /// [`Self::clear_dirty`] after consuming.
    pub fn dirty_chunks(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.dirty_chunks.iter().copied()
    }

    pub fn dirty_chunk_count(&self) -> usize {
        self.dirty_chunks.len()
    }

    /// Clear the dirty set without touching pixel data.
    pub fn clear_dirty(&mut self) {
        self.dirty_chunks.clear();
    }

    /// **M3 re-open (2026-05-13) fix #6**: take + clear the per-chunk
    /// `dirty_rect`. Returns `None` when the chunk has been reclaimed
    /// (uniform default) OR when no pixel writes have landed since the last
    /// take. Caller uses this to upload only the affected sub-rect through
    /// the render bridge instead of the entire 256×256 chunk.
    pub fn take_chunk_dirty_rect(&mut self, cx: i32, cy: i32) -> Option<DirtyRect> {
        let coord = ChunkCoord::new(cx, cy);
        self.chunks.get_mut(&coord).and_then(Chunk::take_dirty_rect)
    }

    /// **M2 contract**: mark every chunk whose pixels intersect the closed
    /// pixel-space AABB as dirty. This is the canonical "I just touched the
    /// terrain at this region" path — any caller that mutates pixels via a
    /// shortcut (door stamp, fluid kernel, future repair tool that doesn't
    /// route through `try_carve` / `try_fill_or_repair` / `try_blast`) MUST
    /// call this so the renderer + AI pathfinder + replay see the edit.
    /// Per CCCP `SLTerrain.cpp:397-481` lesson: raw `set_material_pixel`
    /// without `add_updated_material_area` causes stale pathfinding —
    /// we will not repeat that mistake.
    ///
    /// The AABB is clamped to the terrain extent; out-of-bounds calls
    /// are silently no-op.
    pub fn add_updated_material_area(&mut self, min: [f32; 2], max: [f32; 2]) {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let cs = CHUNK_SIZE as i64;
        let cx0 = x0.div_euclid(cs);
        let cy0 = y0.div_euclid(cs);
        let cx1 = (x1 - 1).div_euclid(cs);
        let cy1 = (y1 - 1).div_euclid(cs);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                self.dirty_chunks.insert(ChunkCoord::new(cx as i32, cy as i32));
            }
        }
    }

    /// Per-chunk blake3 checksum (forward-compat for M3A determinism feed).
    /// Returns `None` for unallocated chunks; callers should treat that as
    /// "uniform default_material" and hash the default id instead.
    pub fn chunk_checksum(&self, cx: i32, cy: i32) -> Option<String> {
        let coord = ChunkCoord::new(cx, cy);
        let chunk = self.chunks.get(&coord)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&cx.to_le_bytes());
        hasher.update(&cy.to_le_bytes());
        for p in &chunk.pixels {
            hasher.update(&p.to_le_bytes());
        }
        Some(hex::encode(hasher.finalize().as_bytes()))
    }

    /// Per-chunk material grid copy. Used by `inspect.terrain.chunk` so
    /// cfctl + AI consumers can read the exact pixel layout.
    pub fn chunk_pixels(&self, cx: i32, cy: i32) -> Vec<MaterialId> {
        let coord = ChunkCoord::new(cx, cy);
        match self.chunks.get(&coord) {
            Some(c) => c.pixels.clone(),
            None => vec![self.default_material; CHUNK_PIXELS],
        }
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: read the chunk's current
    /// `dirty_rect` without taking it. Returns `None` when the chunk is
    /// unallocated OR has no pending dirty rect. Used by
    /// `inspect.terrain.chunk` so cfctl consumers can see in-flight dirt
    /// state without affecting the next render-bridge drain.
    #[must_use]
    pub fn chunk_dirty_rect(&self, cx: i32, cy: i32) -> Option<DirtyRect> {
        self.chunks.get(&ChunkCoord::new(cx, cy)).and_then(|c| c.dirty_rect)
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: read the chunk's
    /// `last_modified_tick` stamp. Returns 0 for unallocated chunks (they
    /// have never been modified).
    #[must_use]
    pub fn chunk_last_modified_tick(&self, cx: i32, cy: i32) -> u64 {
        self.chunks
            .get(&ChunkCoord::new(cx, cy))
            .map(|c| c.last_modified_tick)
            .unwrap_or(0)
    }

    /// Try to fill / repair a circular region with `material`. Mirrors
    /// `try_carve` semantics: the operation refuses when the mask overlaps a
    /// refusal-reason material (e.g., metal_nohook — can't repaint over
    /// undiggable metal). Used by future repair-fill tools and the M2 spec
    /// `try_fill_or_repair` surface.
    pub fn try_fill_or_repair(&mut self, origin: [f32; 2], radius: f32, material: MaterialId) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];
        let mut probe = [center_px_x.round() as i64, center_px_y.round() as i64];

        // Refusal probe: if the target overlaps a refusal-only material
        // (metal_nohook / anchor) we cannot overpaint without consent.
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if let Some(reason) = self.registry.refusal_reason(mat) {
                    self.refusal_count += 1;
                    return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                        reason,
                        probe_at: [px, py],
                        material: mat,
                    });
                }
                probe = [px, py];
            }
        }

        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let prev = self.material_at(px, py);
                if self.set_pixel_internal(px, py, material) {
                    *counts.entry(prev).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp { probe_at: probe });
        }
        self.carve_count += 1;
        let dominant_source = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material: dominant_source,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// True if the terrain has any allocated chunks; cheap to check before
    /// emitting an empty checksum.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Number of allocated chunks (sparse storage proof).
    pub fn allocated_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// **M15** § enumerate every allocated chunk's `(cx, cy)` in
    /// `(cx, cy)` ascending order. The CA stepper iterates this list
    /// per tick to apply Margolus rules.
    pub fn allocated_chunk_coords(&self) -> Vec<(i32, i32)> {
        self.chunks.keys().map(|c| (c.cx, c.cy)).collect()
    }

    /// Clone a chunk's pixel buffer for parallel snapshot-then-apply.
    pub fn chunk_pixels_clone(&self, cx: i32, cy: i32) -> Option<Vec<MaterialId>> {
        self.chunks.get(&ChunkCoord::new(cx, cy)).map(|c| c.pixels.clone())
    }

    /// Replace a chunk's pixel buffer atomically; updates dirty_rect to whole-chunk
    /// and bumps last_modified_tick. Used by the parallel CA stepper after off-chunk
    /// processing. Returns true if the chunk existed.
    pub fn replace_chunk_pixels_at_tick(
        &mut self,
        cx: i32,
        cy: i32,
        new_pixels: Vec<MaterialId>,
        tick: u64,
    ) -> bool {
        let coord = ChunkCoord::new(cx, cy);
        let chunk = match self.chunks.get_mut(&coord) {
            Some(c) => c,
            None => return false,
        };
        if new_pixels.len() != chunk.pixels.len() {
            return false;
        }
        if chunk.pixels == new_pixels {
            return false;
        }
        chunk.pixels = new_pixels;
        chunk.last_modified_tick = tick;
        chunk.dirty_rect = Some(DirtyRect {
            min: [0, 0],
            max: [CHUNK_SIZE - 1, CHUNK_SIZE - 1],
        });
        true
    }

    /// **M15** § enumerate awake chunks only (`active_region == true`)
    /// in `(cx, cy)` ascending order. Per the M15 spec § "Per-pixel
    /// cellular automata (Noita chunking)" rule: "Per-tick: only active
    /// chunks simulated (dirty regions + nearby chunks)". The CA stepper
    /// uses this when wake/sleep gating is active.
    pub fn awake_chunk_coords(&self) -> Vec<(i32, i32)> {
        self.chunks
            .iter()
            .filter(|(_, c)| c.active_region)
            .map(|(coord, _)| (coord.cx, coord.cy))
            .collect()
    }

    /// **M15** § set the `active_region` flag on a single allocated
    /// chunk. Per M15 spec § Preservation rule 4: "M3 always writes
    /// false; M15 sets true for chunks with falling materials". Returns
    /// true if the chunk existed; false if no chunk is allocated at
    /// `(cx, cy)` (the caller can pre-allocate via a `set_material_pixel`
    /// call first if needed).
    pub fn set_chunk_active_region(&mut self, cx: i32, cy: i32, value: bool) -> bool {
        let coord = ChunkCoord::new(cx, cy);
        match self.chunks.get_mut(&coord) {
            Some(chunk) => {
                chunk.active_region = value;
                true
            }
            None => false,
        }
    }

    /// **M15** § read the `active_region` flag on a chunk. Returns
    /// `false` for unallocated chunks (they are implicitly sleeping).
    #[must_use]
    pub fn chunk_active_region(&self, cx: i32, cy: i32) -> bool {
        self.chunks
            .get(&ChunkCoord::new(cx, cy))
            .map(|c| c.active_region)
            .unwrap_or(false)
    }

    /// **M15** § wake the chunk at `(cx, cy)` plus its 1-chunk-radius
    /// neighborhood (3×3 = 9 chunks total) to `active_region = true`.
    /// Per M15 spec § "active-chunk wake/sleep gating" + M8A's
    /// `wake_chunk_and_neighbors` semantics. Only updates already-
    /// allocated chunks; missing chunks are left as-is. Returns the
    /// number of chunks transitioned.
    pub fn wake_chunk_neighborhood(&mut self, cx: i32, cy: i32) -> u32 {
        let mut transitioned = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let coord = ChunkCoord::new(cx + dx, cy + dy);
                if let Some(chunk) = self.chunks.get_mut(&coord) {
                    if !chunk.active_region {
                        chunk.active_region = true;
                        transitioned += 1;
                    }
                }
            }
        }
        transitioned
    }

    /// **M15** § transition chunks that have been idle for at least
    /// `idle_threshold_ticks` to `active_region = false`. Per the M15
    /// spec § "Per Noita pattern: most of world sleeping; only chunks
    /// with falling materials wake up". Returns the chunks that just
    /// went to sleep.
    pub fn sleep_idle_chunks(&mut self, current_tick: u64, idle_threshold_ticks: u64) -> Vec<(i32, i32)> {
        let mut transitioned = Vec::new();
        for (coord, chunk) in &mut self.chunks {
            if chunk.active_region
                && current_tick.saturating_sub(chunk.last_modified_tick) >= idle_threshold_ticks
            {
                chunk.active_region = false;
                transitioned.push((coord.cx, coord.cy));
            }
        }
        transitioned
    }

    /// **M15** § set the material at world-space pixel `(px, py)` with
    /// an explicit tick stamp. Routes through the canonical
    /// `set_pixel_at_tick` path so the per-chunk `last_modified_tick`,
    /// `dirty_rect`, and `dirty_chunks` set all stay coherent. Per the
    /// M3 preservation rules (M15 spec § "Preservation rules from M3"
    /// rule 1) every CA / reaction / phase-change pixel write MUST go
    /// through this entry point (not the lower-level chunk APIs).
    /// Returns true if the pixel changed.
    pub fn set_material_pixel(&mut self, px: i64, py: i64, mat: MaterialId, tick: u64) -> bool {
        let prev_tick = self.current_tick;
        self.current_tick = tick;
        let changed = self.set_pixel_internal(px, py, mat);
        self.current_tick = prev_tick;
        changed
    }

    /// **M3 audit pass 5 (2026-05-13)**: per-chunk blake3 hex summaries for
    /// every allocated chunk. Used by the engine to populate the
    /// `chunk_summary` field on `determinism.sim_checksum` payloads per
    /// spec literal "And it appears in the determinism.sim_checksum
    /// payload's chunk-summary field". Returns `(cx, cy, blake3_hex)`
    /// triples ordered by (cx, cy) for deterministic JSON output.
    ///
    /// **Performance**: parallelized via rayon's `par_iter` over the chunk
    /// map's entries. Per-chunk blake3 hashing is CPU-bound + independent
    /// (no shared writes), so this scales linearly with core count. The
    /// final collect() preserves BTreeMap iteration order so the output
    /// is byte-identical to the serial path (determinism per DR-052).
    pub fn chunk_summary_entries(&self) -> Vec<(i32, i32, String)> {
        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        // Snapshot ordered entries first so par_iter operates on a
        // canonical-ordered slice; final result preserves (cx, cy)
        // ascending order without needing a post-sort.
        let entries: Vec<(&ChunkCoord, &Chunk)> = self.chunks.iter().collect();
        entries
            .into_par_iter()
            .map(|(coord, chunk)| {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&coord.cx.to_le_bytes());
                hasher.update(&coord.cy.to_le_bytes());
                for p in &chunk.pixels {
                    hasher.update(&p.to_le_bytes());
                }
                (coord.cx, coord.cy, hex::encode(hasher.finalize().as_bytes()))
            })
            .collect()
    }

    /// Per-material pixel counts across every allocated chunk + the implicit
    /// default-material area. Used for snapshot + summary stats.
    pub fn material_counts(&self) -> BTreeMap<String, u64> {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        // Total covered pixels.
        let total = (self.width_px as u64) * (self.height_px as u64);
        // Pixels in allocated chunks: sum each chunk's pixel array.
        let mut allocated_pixels: u64 = 0;
        for (coord, chunk) in &self.chunks {
            // Skip chunks fully outside the terrain extent.
            let origin = coord.pixel_origin();
            if origin[0] >= self.width_px as i64 || origin[1] >= self.height_px as i64 {
                continue;
            }
            for ly in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let px = origin[0] + lx as i64;
                    let py = origin[1] + ly as i64;
                    if !self.in_bounds(px, py) {
                        continue;
                    }
                    let mat = chunk.pixel(lx, ly);
                    let name = self.registry.name(mat);
                    *counts.entry(name.to_string()).or_insert(0) += 1;
                    allocated_pixels += 1;
                }
            }
        }
        // Pixels NOT in allocated chunks default to `default_material`.
        let default_name = self.registry.name(self.default_material);
        let default_pixels = total.saturating_sub(allocated_pixels);
        if default_pixels > 0 {
            *counts.entry(default_name.to_string()).or_insert(0) += default_pixels;
        }
        counts
    }

    /// Layout-stable bytes used by the determinism checksum. M2 appends after
    /// `cf-actor::sim::ActorSimState::checksum_bytes()` which appends after the
    /// M0 `tick_counter || rng_state` prefix; the layout is `(M0 prefix) ||
    /// (M1 actor bytes) || (M2 terrain bytes)`. M3+ snapshots hash this
    /// directly.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.chunks.len() * 32);
        out.extend_from_slice(b"cf-terrain-chunked-v2");
        out.extend_from_slice(&self.width_px.to_le_bytes());
        out.extend_from_slice(&self.height_px.to_le_bytes());
        out.extend_from_slice(&self.default_material.to_le_bytes());
        out.extend_from_slice(&self.carve_count.to_le_bytes());
        out.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
        let mut chunk_hasher = blake3::Hasher::new();
        for (coord, chunk) in &self.chunks {
            chunk_hasher.update(&coord.cx.to_le_bytes());
            chunk_hasher.update(&coord.cy.to_le_bytes());
            for p in &chunk.pixels {
                chunk_hasher.update(&p.to_le_bytes());
            }
        }
        out.extend_from_slice(chunk_hasher.finalize().as_bytes());
        out
    }

    /// Full snapshot, suitable for the M3A `snapshot.snapshot_terrain_chunk`
    /// event payload. JSON-serializable. Only allocated chunks are emitted;
    /// the default material covers the rest.
    pub fn snapshot(&self) -> ChunkedTerrainSnapshot {
        let chunks = self
            .chunks
            .iter()
            .map(|(coord, chunk)| ChunkedTerrainSnapshotChunk {
                coord: *coord,
                pixels: chunk.pixels.clone(),
            })
            .collect();
        ChunkedTerrainSnapshot {
            schema: MATERIAL_SCHEMA_VERSION.to_string(),
            width_px: self.width_px,
            height_px: self.height_px,
            anchor: self.anchor,
            default_material: self.default_material,
            carve_count: self.carve_count,
            refusal_count: self.refusal_count,
            material_counts: self.material_counts(),
            chunks,
        }
    }

    /// **M9 § Destructible terrain — 5-tier per-pixel integrity**: read
    /// the integrity at `(px, py)`. Returns `1.0` (Pristine) when the
    /// pixel has no metadata entry — Pristine pixels are implicit. Air
    /// pixels also return `1.0` so callers don't need to special-case them
    /// before applying damage (the `try_penetrate_pixel` path filters air
    /// itself).
    #[must_use]
    pub fn pixel_integrity(&self, px: i64, py: i64) -> f32 {
        if !self.in_bounds(px, py) {
            return 1.0;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        match self.pixel_meta_grid.get(&PixelMetaKey::new(coord.cx, coord.cy, lx, ly)) {
            Some(meta) => meta.integrity,
            None => 1.0,
        }
    }

    /// **M9 § Destructible terrain — 5-tier per-pixel integrity**: derive
    /// the band at `(px, py)`. Untouched pixels report Pristine.
    #[must_use]
    pub fn pixel_band(&self, px: i64, py: i64) -> IntegrityBand {
        IntegrityBand::from_integrity(self.pixel_integrity(px, py))
    }

    /// **M9 § Destructible terrain — per-pixel integrity damage**.
    ///
    /// Apply `impact_energy` damage to a single pixel under the per-material
    /// hardness curve (`damage = impact_energy * (1 - hardness) / hardness`,
    /// clamped to `[0, 1]`). Tracks integrity in `pixel_meta_grid` and
    /// emits a `PenetrationOutcome` describing band crossings, pixel
    /// removal, and any cascade decay applied to direct neighbors when
    /// the pixel reaches Destroyed.
    ///
    /// Caller is responsible for emitting `terrain.material_state_changed`
    /// plus `terrain.pixel_removed` plus `terrain.cascade_triggered` based
    /// on the outcome. cf-control's projectile-vs-terrain handler does this
    /// inside its event loop with the right `parent_event_id` chain.
    ///
    /// Returns `None` when the pixel is out of bounds OR air — air pixels
    /// receive no damage state.
    pub fn try_penetrate_pixel(
        &mut self,
        px: i64,
        py: i64,
        impact_energy: f32,
        cause: DamageKind,
        damage_source: Option<String>,
    ) -> Option<PenetrationOutcome> {
        if !self.in_bounds(px, py) {
            return None;
        }
        let mat = self.material_at(px, py);
        if mat == self.default_material || mat == MATERIAL_AIR {
            return None;
        }
        let aff = self.registry.affordance(mat)?;
        let (coord, lx, ly) = chunk_split(px, py);
        let key = PixelMetaKey::new(coord.cx, coord.cy, lx, ly);
        let integrity_before = self.pixel_meta_grid.get(&key).map(|m| m.integrity).unwrap_or(1.0);
        let band_before = IntegrityBand::from_integrity(integrity_before);
        let hardness = normalized_hardness(mat);
        let integrity_after = apply_damage_formula(integrity_before, impact_energy, hardness);
        let band_after = IntegrityBand::from_integrity(integrity_after);
        let band_crossed = band_before != band_after;
        let destroyed = integrity_after <= 0.0;
        if destroyed {
            // Clear damage state + remove pixel from the world. Cascade
            // decay applies to surviving neighbors only — never to air.
            self.pixel_meta_grid.remove(&key);
            self.set_pixel_internal(px, py, self.default_material);
        } else {
            self.pixel_meta_grid.insert(
                key,
                PixelMeta {
                    integrity: integrity_after,
                    last_damage_tick: self.current_tick,
                    damage_kind: cause,
                    damage_source: damage_source.clone(),
                },
            );
        }
        let cascades = if destroyed {
            self.apply_cascade_decay(px, py, damage_source.as_deref())
        } else {
            Vec::new()
        };
        Some(PenetrationOutcome {
            pos: [px, py],
            material_id: mat,
            material_name: aff.name,
            integrity_before,
            integrity_after,
            band_before,
            band_after,
            band_crossed,
            destroyed,
            cascades,
        })
    }

    /// **M9 § Cascade rule** for digger / blast carves: after a multi-pixel
    /// carve, walk the perimeter of the destroyed bbox and apply cascade
    /// decay to any solid pixel adjacent to a freshly-cleared pixel whose
    /// normalized hardness is at or below the cascade threshold. Returns
    /// one `CascadeEvent` per affected neighbor (caller emits
    /// `terrain.cascade_triggered` with the right `parent_event_id`).
    ///
    /// `bbox_min` / `bbox_max` are inclusive pixel-space bounds (matching
    /// `ChunkedCarveStats`). `source` annotates the cascade source for the
    /// cause chain.
    #[must_use]
    pub fn apply_cascade_to_carve_perimeter(
        &mut self,
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        source: Option<&str>,
    ) -> Vec<CascadeEvent> {
        let mut events: Vec<CascadeEvent> = Vec::new();
        let mut seen: BTreeSet<(i64, i64)> = BTreeSet::new();
        // Walk a 1-pixel halo around the destroyed bbox. Any air pixel
        // inside the carve volume is a candidate "destroyed source"; any
        // solid pixel adjacent to it is a candidate "affected neighbor".
        for py in (bbox_min[1] - 1)..=(bbox_max[1] + 1) {
            for px in (bbox_min[0] - 1)..=(bbox_max[0] + 1) {
                if !self.in_bounds(px, py) {
                    continue;
                }
                let mat = self.material_at(px, py);
                if mat != self.default_material && mat != MATERIAL_AIR {
                    continue;
                }
                // Cleared pixel inside or at the edge of the carve. Check
                // its 4-neighbors for cascade candidates.
                for (dx, dy) in [(-1_i64, 0_i64), (1, 0), (0, -1), (0, 1)] {
                    let nx = px + dx;
                    let ny = py + dy;
                    if !self.in_bounds(nx, ny) {
                        continue;
                    }
                    if !seen.insert((nx, ny)) {
                        continue;
                    }
                    if let Some(ev) = self.cascade_pixel(nx, ny, [px, py], source) {
                        events.push(ev);
                    }
                }
            }
        }
        events
    }

    /// **M9 § Cascade rule**: when a pixel reaches Destroyed, decay direct
    /// 4-neighbors whose normalized hardness is at or below
    /// `cascade_threshold` (default 0.6). Each affected neighbor loses
    /// `DEFAULT_CASCADE_DECAY_PCT` (default 0.1) integrity, clamped to
    /// `[0, 1]`. Neighbors that reach 0 under cascade are removed in turn
    /// (still capped to `cascade_depth=1` — we do NOT recurse).
    ///
    /// Returns the cascade events so the engine can emit
    /// `terrain.cascade_triggered` (one per affected neighbor) and
    /// `terrain.pixel_removed` (when cascade kills the neighbor).
    fn apply_cascade_decay(&mut self, source_x: i64, source_y: i64, source: Option<&str>) -> Vec<CascadeEvent> {
        let mut events = Vec::with_capacity(4);
        for (dx, dy) in [(-1_i64, 0_i64), (1, 0), (0, -1), (0, 1)] {
            let nx = source_x + dx;
            let ny = source_y + dy;
            if let Some(ev) = self.cascade_pixel(nx, ny, [source_x, source_y], source) {
                events.push(ev);
            }
        }
        events
    }

    /// Apply cascade decay to a single candidate neighbor pixel from
    /// `from_pos`. Returns the event when decay landed; `None` when:
    ///   - out of bounds
    ///   - neighbor is air / default material
    ///   - neighbor hardness > cascade threshold
    ///   - neighbor integrity unchanged (already at 0)
    fn cascade_pixel(&mut self, nx: i64, ny: i64, from_pos: [i64; 2], source: Option<&str>) -> Option<CascadeEvent> {
        if !self.in_bounds(nx, ny) {
            return None;
        }
        let nmat = self.material_at(nx, ny);
        if nmat == self.default_material || nmat == MATERIAL_AIR {
            return None;
        }
        let nhardness = normalized_hardness(nmat);
        if nhardness > DEFAULT_CASCADE_THRESHOLD {
            return None;
        }
        let (ncoord, nlx, nly) = chunk_split(nx, ny);
        let nkey = PixelMetaKey::new(ncoord.cx, ncoord.cy, nlx, nly);
        let integrity_before = self.pixel_meta_grid.get(&nkey).map(|m| m.integrity).unwrap_or(1.0);
        let integrity_after = (integrity_before - DEFAULT_CASCADE_DECAY_PCT).clamp(0.0, 1.0);
        if (integrity_after - integrity_before).abs() < f32::EPSILON {
            return None;
        }
        let band_before = IntegrityBand::from_integrity(integrity_before);
        let band_after = IntegrityBand::from_integrity(integrity_after);
        let destroyed_neighbor = integrity_after <= 0.0;
        if destroyed_neighbor {
            self.pixel_meta_grid.remove(&nkey);
            self.set_pixel_internal(nx, ny, self.default_material);
        } else {
            self.pixel_meta_grid.insert(
                nkey,
                PixelMeta {
                    integrity: integrity_after,
                    last_damage_tick: self.current_tick,
                    damage_kind: DamageKind::NeighborDestroyed,
                    damage_source: source.map(str::to_owned),
                },
            );
        }
        let nname = self.registry.affordance(nmat).map(|a| a.name).unwrap_or("unknown");
        Some(CascadeEvent {
            from_pos,
            to_pos: [nx, ny],
            material_id: nmat,
            material_name: nname,
            integrity_before,
            integrity_after,
            from_band: band_before,
            to_band: band_after,
            destroyed_neighbor,
            depth: DEFAULT_CASCADE_DEPTH,
            threshold: DEFAULT_CASCADE_THRESHOLD,
        })
    }

    /// Reverse of [`Self::snapshot`]: rebuild a terrain from a snapshot. Used
    /// by the cf-headless replay verifier so a replayed bundle's per-tick
    /// state matches the live run pixel-for-pixel.
    pub fn from_snapshot(snapshot: &ChunkedTerrainSnapshot) -> Self {
        let mut t = Self::new(snapshot.width_px, snapshot.height_px, snapshot.default_material);
        t.anchor = snapshot.anchor;
        t.carve_count = snapshot.carve_count;
        t.refusal_count = snapshot.refusal_count;
        for c in &snapshot.chunks {
            if c.pixels.len() != CHUNK_PIXELS {
                tracing::warn!(
                    target: "cf::terrain",
                    coord = ?c.coord,
                    expected = CHUNK_PIXELS,
                    actual = c.pixels.len(),
                    "from_snapshot: chunk pixel array has wrong length; skipping"
                );
                continue;
            }
            t.chunks.insert(
                c.coord,
                Chunk {
                    pixels: c.pixels.clone(),
                    active_region: false,
                    last_modified_tick: 0,
                    color_grid: None,
                    dirty_rect: None,
                },
            );
        }
        t
    }

    /// Reset every chunk to the default material. Used by `scenario.reset` so
    /// the engine can rewind the terrain without rebuilding from disk.
    pub fn reset_to_default(&mut self) {
        self.chunks.clear();
        self.dirty_chunks.clear();
        self.carve_count = 0;
        self.refusal_count = 0;
        self.pixel_meta_grid.clear();
    }

    fn aabb_to_pixels(&self, min: [f32; 2], max: [f32; 2]) -> (i64, i64, i64, i64) {
        let x0 = ((min[0] - self.anchor[0]).floor() as i64).max(0);
        let y0 = ((min[1] - self.anchor[1]).floor() as i64).max(0);
        let x1 = ((max[0] - self.anchor[0]).ceil() as i64)
            .max(0)
            .min(self.width_px as i64);
        let y1 = ((max[1] - self.anchor[1]).ceil() as i64)
            .max(0)
            .min(self.height_px as i64);
        (x0, y0, x1.max(x0), y1.max(y0))
    }

    pub(crate) fn in_bounds(&self, px: i64, py: i64) -> bool {
        // Devin BUG_pr-review-job (yellow): compare in i64 space so a
        // pathological caller passing px >= 2^32 doesn't truncate-wrap to a
        // small u32 and falsely report "in bounds". All current call sites
        // derive (px, py) from f32 world coords (max ~16M) or
        // `aabb_to_pixels` which clamps to the terrain extent, so this is
        // defensive — but the contract is now branch-truthful regardless of
        // the caller's coordinate source.
        px >= 0 && py >= 0 && px < (self.width_px as i64) && py < (self.height_px as i64)
    }

    pub(crate) fn set_pixel_internal(&mut self, px: i64, py: i64, mat: MaterialId) -> bool {
        if !self.in_bounds(px, py) {
            return false;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        let entry = self
            .chunks
            .entry(coord)
            .or_insert_with(|| Chunk::uniform(self.default_material));
        // M3 re-audit pass 4 (2026-05-13): route through `set_pixel_at_tick`
        // so the chunk's `last_modified_tick` stamp tracks the engine's
        // current tick. `inspect.terrain.chunk` reads this stamp.
        let changed = entry.set_pixel_at_tick(lx, ly, mat, self.current_tick);
        if changed {
            self.dirty_chunks.insert(coord);
            // Reclaim chunks that fully match the default to keep storage sparse.
            if entry.is_uniform(self.default_material) {
                self.chunks.remove(&coord);
            }
        }
        changed
    }
}

/// Split a pixel coordinate into (chunk coord, local x, local y).
fn chunk_split(px: i64, py: i64) -> (ChunkCoord, u32, u32) {
    let cs = CHUNK_SIZE as i64;
    let cx = px.div_euclid(cs) as i32;
    let cy = py.div_euclid(cs) as i32;
    let lx = px.rem_euclid(cs) as u32;
    let ly = py.rem_euclid(cs) as u32;
    (ChunkCoord::new(cx, cy), lx, ly)
}

