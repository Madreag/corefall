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

/// One pixel's material id. The DR-007 launch set ships 8 ids; the runtime stays
/// `u8` so future expansion (M5.6 active material kernel) can fit additional ids
/// without changing the storage layout.
pub type MaterialId = u8;

pub const MATERIAL_AIR: MaterialId = 0;
pub const MATERIAL_DIRT: MaterialId = 1;
pub const MATERIAL_CONCRETE: MaterialId = 2;
pub const MATERIAL_METAL_NOHOOK: MaterialId = 3;
pub const MATERIAL_HAZARD: MaterialId = 4;
pub const MATERIAL_LOOSE_FILL: MaterialId = 5;
pub const MATERIAL_REPAIR_FILL: MaterialId = 6;
pub const MATERIAL_ANCHOR: MaterialId = 7;

/// 256x256 chunk size — matches the canonical roadmap M2 scope ("256×256
/// chunks; per-pixel material id; sparse storage"). Stored as `u32` so chunk
/// math doesn't sign-extend through `usize` casts.
pub const CHUNK_SIZE: u32 = 256;
const CHUNK_PIXELS: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);

/// Per-material affordance the renderer + AI + physics + tool dispatcher read.
/// The canonical roadmap (M2 scope) names this as: hardness, anchorability,
/// hazard flags, path-cost contribution, plus a tool-validity refusal reason
/// for the (intentionally non-diggable) `metal_nohook` and `anchor` materials.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialAffordance {
    pub id: MaterialId,
    pub name: &'static str,
    /// True if the material blocks actor / projectile motion.
    pub solid: bool,
    /// True if a digger tool can carve through it.
    pub diggable: bool,
    /// HP per pixel for diggable materials. Engine deducts `tool_strength`
    /// from this per dig call against pixels in the carve mask.
    pub hardness: f32,
    /// True if the material can support an anchor / climbing tool.
    pub anchorable: bool,
    /// True if the material damages actors that touch / occupy it.
    pub hazard: bool,
    /// AI path-cost contribution (1.0 = nominal floor; >1.0 = expensive).
    pub path_cost: f32,
    /// Material-overlay color (sRGB, alpha 0xFF). Pure black `[0,0,0,0]` for
    /// `air` so the overlay shows nothing on empty space.
    pub overlay_rgba: [u8; 4],
    /// Stable refusal reason emitted on `terrain.tool_refused` when the dig
    /// targets this material. `None` for diggable materials.
    pub refusal_reason: Option<&'static str>,
}

impl MaterialAffordance {
    pub fn is_solid(&self) -> bool {
        self.solid
    }
}

const MATERIAL_TABLE: [MaterialAffordance; 8] = [
    MaterialAffordance {
        id: MATERIAL_AIR,
        name: "air",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        path_cost: 1.0,
        overlay_rgba: [0, 0, 0, 0],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_DIRT,
        name: "dirt",
        solid: true,
        diggable: true,
        hardness: 8.0,
        anchorable: true,
        hazard: false,
        path_cost: 2.0,
        overlay_rgba: [120, 80, 50, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_CONCRETE,
        name: "concrete",
        solid: true,
        diggable: true,
        hardness: 32.0,
        anchorable: true,
        hazard: false,
        path_cost: 4.0,
        overlay_rgba: [180, 180, 180, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_METAL_NOHOOK,
        name: "metal_nohook",
        solid: true,
        diggable: false,
        hardness: f32::INFINITY,
        anchorable: false,
        hazard: false,
        path_cost: 16.0,
        overlay_rgba: [80, 100, 140, 0xFF],
        refusal_reason: Some("material_metal_nohook"),
    },
    MaterialAffordance {
        id: MATERIAL_HAZARD,
        name: "hazard",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        path_cost: 32.0,
        overlay_rgba: [200, 60, 60, 0xFF],
        refusal_reason: Some("material_hazard"),
    },
    MaterialAffordance {
        id: MATERIAL_LOOSE_FILL,
        name: "loose_fill",
        solid: true,
        diggable: true,
        hardness: 4.0,
        anchorable: false,
        hazard: false,
        path_cost: 3.0,
        overlay_rgba: [200, 170, 90, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_REPAIR_FILL,
        name: "repair_fill",
        solid: true,
        diggable: true,
        hardness: 6.0,
        anchorable: true,
        hazard: false,
        path_cost: 2.0,
        overlay_rgba: [120, 200, 140, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_ANCHOR,
        name: "anchor",
        solid: true,
        diggable: false,
        hardness: f32::INFINITY,
        anchorable: true,
        hazard: false,
        path_cost: 8.0,
        overlay_rgba: [60, 60, 200, 0xFF],
        refusal_reason: Some("material_anchor"),
    },
];

/// Look up a material affordance by id. `None` if the id is outside the launch
/// set; callers should treat unknown ids as `air` and emit a structured warning.
#[must_use]
pub fn material_affordance(id: MaterialId) -> Option<&'static MaterialAffordance> {
    MATERIAL_TABLE.iter().find(|m| m.id == id)
}

/// Resolve a material name (case-sensitive) from a scenario manifest. Names
/// match the DR-007 launch set verbatim. `concrete_soft` is a deprecated M1.5
/// alias of `concrete` retained for backward compat with `micro_breach.ron`.
#[must_use]
pub fn material_id_from_name(name: &str) -> Option<MaterialId> {
    match name {
        "air" => Some(MATERIAL_AIR),
        "dirt" => Some(MATERIAL_DIRT),
        "concrete" | "concrete_soft" => Some(MATERIAL_CONCRETE),
        "metal_nohook" => Some(MATERIAL_METAL_NOHOOK),
        "hazard" => Some(MATERIAL_HAZARD),
        "loose_fill" => Some(MATERIAL_LOOSE_FILL),
        "repair_fill" => Some(MATERIAL_REPAIR_FILL),
        "anchor" => Some(MATERIAL_ANCHOR),
        _ => None,
    }
}

/// Material schema version stamped into `run_manifest.json.material_schema_version`
/// so future schema migrations can identify legacy bundles.
pub const MATERIAL_SCHEMA_VERSION: &str = "cf-terrain-launch-v1";

/// One chunk in the terrain. Stored densely as a row-major array of `MaterialId`s.
/// Sparse: chunks with all pixels equal to the terrain's `default_material` are
/// NOT stored at all — `ChunkedTerrain::material_at` returns the default in
/// that case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pixels: Vec<MaterialId>,
}

impl Chunk {
    fn uniform(material: MaterialId) -> Self {
        Self {
            pixels: vec![material; CHUNK_PIXELS],
        }
    }

    /// Material at `(lx, ly)` inside this chunk; `lx`/`ly` are local 0..CHUNK_SIZE.
    pub fn pixel(&self, lx: u32, ly: u32) -> MaterialId {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        self.pixels[(ly as usize) * (CHUNK_SIZE as usize) + (lx as usize)]
    }

    /// Set the material at `(lx, ly)`; returns true if the cell changed.
    pub fn set_pixel(&mut self, lx: u32, ly: u32, mat: MaterialId) -> bool {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        let idx = (ly as usize) * (CHUNK_SIZE as usize) + (lx as usize);
        let prev = self.pixels[idx];
        if prev == mat {
            return false;
        }
        self.pixels[idx] = mat;
        true
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
        }
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
        let mut written: u64 = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - cx;
                let dy = (py as f32 + 0.5) - cy;
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

        // First pass: probe for refusal-reason materials. A refusal short-circuits
        // the whole carve so the player gets a clean "this won't work" event.
        let mut probe = [origin[0].round() as i64, origin[1].round() as i64];
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - origin[0];
                let dy = (py as f32 + 0.5) - origin[1];
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
                let dx = (px as f32 + 0.5) - origin[0];
                let dy = (py as f32 + 0.5) - origin[1];
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

        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        let mut hardest_blocked: Option<(MaterialId, &'static str)> = None;
        let air = self.default_material;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - origin[0];
                let dy = (py as f32 + 0.5) - origin[1];
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
                    probe_at: [origin[0].round() as i64, origin[1].round() as i64],
                    material: mat,
                });
            }
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp {
                probe_at: [origin[0].round() as i64, origin[1].round() as i64],
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

    /// True if the terrain has any allocated chunks; cheap to check before
    /// emitting an empty checksum.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Number of allocated chunks (sparse storage proof).
    pub fn allocated_chunk_count(&self) -> usize {
        self.chunks.len()
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
        out.extend_from_slice(b"cf-terrain-chunked-v1");
        out.extend_from_slice(&self.width_px.to_le_bytes());
        out.extend_from_slice(&self.height_px.to_le_bytes());
        out.push(self.default_material);
        out.extend_from_slice(&self.carve_count.to_le_bytes());
        out.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
        // Hash each chunk's pixel array via blake3 so the output stays bounded
        // regardless of terrain size. Chunks iterate in BTreeMap order — that's
        // deterministic.
        let mut chunk_hasher = blake3::Hasher::new();
        for (coord, chunk) in &self.chunks {
            chunk_hasher.update(&coord.cx.to_le_bytes());
            chunk_hasher.update(&coord.cy.to_le_bytes());
            chunk_hasher.update(&chunk.pixels);
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

    fn in_bounds(&self, px: i64, py: i64) -> bool {
        px >= 0 && py >= 0 && (px as u32) < self.width_px && (py as u32) < self.height_px
    }

    fn set_pixel_internal(&mut self, px: i64, py: i64, mat: MaterialId) -> bool {
        if !self.in_bounds(px, py) {
            return false;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        let entry = self
            .chunks
            .entry(coord)
            .or_insert_with(|| Chunk::uniform(self.default_material));
        let changed = entry.set_pixel(lx, ly, mat);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn small_world() -> ChunkedTerrain {
        let mut t = ChunkedTerrain::new(512, 256, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [512.0, 16.0], MATERIAL_CONCRETE);
        t.fill_aabb([200.0, 16.0], [400.0, 96.0], MATERIAL_DIRT);
        t.fill_aabb([460.0, 16.0], [492.0, 96.0], MATERIAL_METAL_NOHOOK);
        t
    }

    #[test]
    fn material_id_from_name_covers_launch_set() {
        for name in [
            "air",
            "dirt",
            "concrete",
            "concrete_soft",
            "metal_nohook",
            "hazard",
            "loose_fill",
            "repair_fill",
            "anchor",
        ] {
            assert!(material_id_from_name(name).is_some(), "{name}");
        }
        assert!(material_id_from_name("granite_unknown").is_none());
    }

    #[test]
    fn fill_aabb_writes_dense_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_aabb([0.0, 0.0], [10.0, 10.0], MATERIAL_DIRT);
        assert_eq!(written, 100);
        assert_eq!(t.material_at(0, 0), MATERIAL_DIRT);
        assert_eq!(t.material_at(9, 9), MATERIAL_DIRT);
        assert_eq!(t.material_at(10, 10), MATERIAL_AIR);
    }

    #[test]
    fn fill_circle_writes_radius_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_circle([5.0, 5.0], 3.0, MATERIAL_DIRT);
        assert!(written > 0 && written < 64);
        // (5,5) is the centre — must be filled.
        assert_eq!(t.material_at(5, 5), MATERIAL_DIRT);
        // (15,15) is well outside — still air.
        assert_eq!(t.material_at(15, 15), MATERIAL_AIR);
    }

    #[test]
    fn try_carve_into_dirt_succeeds() {
        let mut t = small_world();
        let outcome = t.try_carve([300.0, 60.0], 8.0);
        match outcome {
            ChunkedCarveOutcome::Carved(stats) => {
                assert!(stats.count > 0);
                assert_eq!(stats.dominant_material, MATERIAL_DIRT);
                assert!(!stats.dirty_chunks.is_empty());
            }
            other => panic!("expected Carved, got {other:?}"),
        }
        assert_eq!(t.carve_count, 1);
    }

    #[test]
    fn try_carve_into_metal_refuses() {
        let mut t = small_world();
        let outcome = t.try_carve([476.0, 60.0], 8.0);
        match outcome {
            ChunkedCarveOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, "material_metal_nohook");
                assert_eq!(refusal.material, MATERIAL_METAL_NOHOOK);
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        assert_eq!(t.refusal_count, 1);
    }

    #[test]
    fn try_carve_against_air_is_noop() {
        let mut t = small_world();
        let outcome = t.try_carve([100.0, 200.0], 4.0);
        assert!(matches!(outcome, ChunkedCarveOutcome::NoOp(_)));
        assert_eq!(t.carve_count, 0);
    }

    #[test]
    fn try_carve_increments_carve_count_only_on_success() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let _ = t.try_carve([320.0, 60.0], 8.0);
        let _ = t.try_carve([476.0, 60.0], 8.0); // refused
        assert_eq!(t.carve_count, 2);
        assert_eq!(t.refusal_count, 1);
    }

    #[test]
    fn aabb_overlaps_solid_detects_floor() {
        let t = small_world();
        assert!(t.aabb_overlaps_solid([0.0, 0.0], [10.0, 16.0]));
        assert!(!t.aabb_overlaps_solid([0.0, 100.0], [10.0, 110.0]));
    }

    #[test]
    fn column_top_solid_y_finds_floor_height() {
        let t = small_world();
        let top = t.column_top_solid_y(50, 51, 200).unwrap();
        // Floor occupies [0..16) so the top solid pixel y is 15.
        assert_eq!(top, 15);
    }

    #[test]
    fn try_carve_breaks_through_full_width_with_repeated_calls() {
        let mut t = small_world();
        let mut carved_total: u32 = 0;
        for _ in 0..6 {
            if let ChunkedCarveOutcome::Carved(stats) = t.try_carve([300.0, 60.0], 8.0) {
                carved_total += stats.count;
            }
        }
        assert!(carved_total > 0);
        // Eventually the carve circle hits all-air and turns into a no-op.
        let _ = t.try_carve([300.0, 60.0], 8.0);
    }

    #[test]
    fn dirty_chunks_track_carves_until_cleared() {
        let mut t = small_world();
        t.clear_dirty();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        assert!(t.dirty_chunk_count() > 0);
        t.clear_dirty();
        assert_eq!(t.dirty_chunk_count(), 0);
    }

    #[test]
    fn checksum_changes_when_terrain_changes() {
        let mut t = small_world();
        let before = t.checksum_bytes();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let after = t.checksum_bytes();
        assert_ne!(before, after);
    }

    #[test]
    fn snapshot_round_trip_preserves_pixel_values() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let snap = t.snapshot();
        let restored = ChunkedTerrain::from_snapshot(&snap);
        assert_eq!(restored.checksum_bytes(), t.checksum_bytes());
        assert_eq!(restored.material_at(300, 60), MATERIAL_AIR);
    }

    #[test]
    fn reset_clears_chunks_and_counters() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        t.reset_to_default();
        assert_eq!(t.allocated_chunk_count(), 0);
        assert_eq!(t.carve_count, 0);
        assert_eq!(t.refusal_count, 0);
        assert_eq!(t.material_at(300, 60), MATERIAL_AIR);
    }

    #[test]
    fn material_counts_balances_total_pixels() {
        let t = small_world();
        let counts = t.material_counts();
        let total: u64 = counts.values().sum();
        assert_eq!(total, 512 * 256);
    }

    #[test]
    fn try_blast_clears_circle_through_diggable() {
        let mut t = small_world();
        let outcome = t.try_blast([300.0, 60.0], 16.0, 100.0);
        match outcome {
            ChunkedCarveOutcome::Carved(stats) => {
                assert!(stats.count > 0);
            }
            other => panic!("expected Carved, got {other:?}"),
        }
    }

    #[test]
    fn try_blast_refuses_when_force_below_metal_hardness() {
        let mut t = small_world();
        let outcome = t.try_blast([476.0, 60.0], 16.0, 1.0);
        assert!(matches!(outcome, ChunkedCarveOutcome::Refused(_)));
    }

    #[test]
    fn fill_aabb_clamped_to_terrain_extent() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_aabb([-10.0, -10.0], [200.0, 200.0], MATERIAL_DIRT);
        assert_eq!(written, 64 * 64);
    }

    #[test]
    fn chunk_uniformity_compresses_storage() {
        let mut t = ChunkedTerrain::new(512, 512, MATERIAL_AIR);
        // Fill an entire chunk with dirt, then carve it back to air.
        t.fill_aabb([0.0, 0.0], [256.0, 256.0], MATERIAL_DIRT);
        assert_eq!(t.allocated_chunk_count(), 1);
        // Carving reverts each pixel back to air; once the chunk is uniform
        // again, storage is reclaimed.
        for y in 0..256 {
            for x in 0..256 {
                t.set_pixel_internal(x, y, MATERIAL_AIR);
            }
        }
        assert_eq!(t.allocated_chunk_count(), 0);
    }
}
