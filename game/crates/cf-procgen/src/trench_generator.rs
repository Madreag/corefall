//! M9B § "Zigzag pattern procgen generator".
//!
//! WWI-style trench layout: every straight run is capped at 12 tiles
//! before a ±45° kink so a single enfilade ray cannot graze more than
//! 12 contiguous trench-floor tiles. Branches of the
//! `Communication` variant connect the front polyline to a rear
//! polyline, enabling rear→front movement entirely inside trench
//! floor. Endpoints are capped with either a `fire_step` dead-end
//! facing outward, or a `FortificationConnection` to an M9C-owned
//! anchor (the placeholder id is consumed during template instantiation
//! per the spec's forward-compat grammar).
//!
//! The generator is deterministic: the same `world_seed` + start/end
//! polyline produces byte-identical kink + branch sequences across
//! independent runs (VAL-M9B-PROCGEN-003). The RNG is a seeded
//! `Xoshiro256StarStar` threaded through [`ZigzagInput`]; nothing in
//! this module reaches for `thread_rng` (project AGENTS.md sim-crate
//! rule).
//!
//! ## PvE ruin pass
//!
//! [`ruin_procgen`] is the M43-facing entry point: given a biome id
//! and a small input vector, it returns 2..=4 ruin template
//! placements with a `decay_factor` matching the spec's
//! `[0.4, 0.4]` exact-match contract (VAL-M9B-PROCGEN-DECAY-001). The
//! decay factor is exposed as a runtime value so future spec drift to
//! a range is a one-line change.

use std::collections::HashSet;

use blake3::Hasher;
use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

use cf_trench::SegmentVariant;

/// Default min/max straight-run length per spec § "Per-kink offset:
/// ±45° every 8-12 tiles". `MAX_STRAIGHT_RUN_TILES` is the
/// **inclusive** upper bound; the generator never emits a straight run
/// of more contiguous tiles than this (VAL-M9B-PROCGEN-001 /
/// VAL-M9B-PROCGEN-004 both bound a ray cast through trench floor at
/// 12 contiguous tiles). Each kink is recorded as `straight_run_tiles
/// = segment_len`, but the kink point itself is also a trench-floor
/// tile shared with the next segment — so the algorithm internally
/// caps `segment_len` at `MAX_CONTIGUOUS_TILES - 1` to keep the
/// "kink point + segment tiles" sum at ≤ 12.
pub const DEFAULT_MIN_SEGMENT_LENGTH: u32 = 7;
/// See [`DEFAULT_MIN_SEGMENT_LENGTH`]. The hard upper bound of 11
/// keeps `1 (kink point) + 11 (segment tiles) = 12 ≤
/// MAX_CONTIGUOUS_TILES`. The on-disk display value in events /
/// reports calls these "12-tile runs" because the kink-point tile is
/// shared with the previous segment.
pub const DEFAULT_MAX_SEGMENT_LENGTH: u32 = 11;
/// Hard ceiling on contiguous trench-floor tiles in any direction
/// (matches VAL-M9B-PROCGEN-004 wording: "cannot intersect more than
/// 12 contiguous tiles of trench floor"). Inclusive.
pub const MAX_CONTIGUOUS_TILES: u32 = 12;

/// 64-bit deterministic hash of the input polyline, mixed into the RNG
/// seed so two distinct polylines on the same map (same `world_seed`)
/// produce independent kink sequences.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PolylineHash(pub u64);

impl PolylineHash {
    #[must_use]
    pub fn of(start: (i32, i32), end: (i32, i32), extra: &[(i32, i32)]) -> Self {
        let mut h = Hasher::new();
        h.update(&start.0.to_le_bytes());
        h.update(&start.1.to_le_bytes());
        h.update(&end.0.to_le_bytes());
        h.update(&end.1.to_le_bytes());
        for (x, y) in extra {
            h.update(&x.to_le_bytes());
            h.update(&y.to_le_bytes());
        }
        let bytes = h.finalize();
        let raw = bytes.as_bytes();
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&raw[..8]);
        PolylineHash(u64::from_le_bytes(buf))
    }
}

/// Inputs to the zigzag generator. The defaults (set via
/// `ZigzagInput::new`) match the spec; tests + scenarios vary
/// `branch_every`, `rear_offset`, and `target_length_tiles` to exercise
/// the matrix without forcing every caller to supply every knob.
#[derive(Debug, Clone)]
pub struct ZigzagInput {
    pub start: (i32, i32),
    pub end: (i32, i32),
    pub world_seed: u64,
    pub branch_every: u32,
    pub rear_offset: i32,
    pub min_segment_length: u32,
    pub max_segment_length: u32,
    pub target_length_tiles: u32,
    pub fortification_anchor: Option<String>,
}

impl ZigzagInput {
    /// Spec-default inputs: spans 60 tiles between `start` and `end`,
    /// branches every 20 front segments, rear polyline 16 tiles
    /// perpendicular to the front. `world_seed` is the only required
    /// driver of variation.
    #[must_use]
    pub fn new(start: (i32, i32), end: (i32, i32), world_seed: u64) -> Self {
        Self {
            start,
            end,
            world_seed,
            branch_every: 20,
            rear_offset: 8,
            min_segment_length: DEFAULT_MIN_SEGMENT_LENGTH,
            max_segment_length: DEFAULT_MAX_SEGMENT_LENGTH,
            target_length_tiles: 60,
            fortification_anchor: None,
        }
    }
}

/// One ±45° kink in the front polyline.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct Kink {
    pub at_tile: (i32, i32),
    /// 1-based index of the front polyline segment that ENDS at this
    /// kink. The segment that BEGINS at this kink is `segment_index + 1`.
    pub segment_index: u32,
    /// Number of straight tiles between the previous kink (or start)
    /// and this kink. Always within `[min_segment_length, max_segment_length]`.
    pub straight_run_tiles: u32,
    pub angle: KinkAngle,
}

/// Allowed kink angles per VAL-M9B-PROCGEN-002 ("every kink is exactly
/// ±45°").
#[repr(i8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum KinkAngle {
    Plus45 = 1,
    Minus45 = -1,
}

impl KinkAngle {
    #[must_use]
    pub fn degrees(self) -> f32 {
        match self {
            KinkAngle::Plus45 => 45.0,
            KinkAngle::Minus45 => -45.0,
        }
    }

    #[must_use]
    pub fn rotation_step(self) -> i32 {
        match self {
            KinkAngle::Plus45 => 1,
            KinkAngle::Minus45 => -1,
        }
    }
}

/// Cap on either end of a generated trench line. The spec mandates
/// every trench line ends in either a fire_step facing outward, or a
/// connection to an M9C-owned fortification (with placeholder support
/// for the forward-compat grammar).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endpoint {
    FireStepDeadEnd {
        pos: (i32, i32),
        facing: EndpointFacing,
    },
    FortificationConnection {
        pos: (i32, i32),
        fortification_id: String,
    },
}

/// Outward-facing unit vector (each component in `{-1, 0, 1}`) on a
/// fire-step dead-end. Used by HUD + AI doctrine to know which side of
/// the trench mouth the defender's firing arc spans.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct EndpointFacing {
    pub dx: i32,
    pub dy: i32,
}

/// A `Communication` variant branch perpendicular to the front
/// polyline, terminating at the rear polyline. Every step of `path`
/// stays inside trench-floor tiles per VAL-M9B-PROCGEN-005.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunicationBranch {
    pub front_anchor: (i32, i32),
    pub rear_anchor: (i32, i32),
    pub path: Vec<(i32, i32)>,
}

/// Output of [`generate_trench_polyline`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedZigzag {
    pub polyline_hash: PolylineHash,
    pub front_polyline: Vec<(i32, i32)>,
    pub rear_polyline: Vec<(i32, i32)>,
    pub kinks: Vec<Kink>,
    pub branches: Vec<CommunicationBranch>,
    pub endpoints: Vec<Endpoint>,
    pub floor_tiles: Vec<(i32, i32)>,
    pub max_straight_run_tiles: u32,
}

impl ResolvedZigzag {
    pub fn floor_tile_set(&self) -> HashSet<(i32, i32)> {
        self.floor_tiles.iter().copied().collect()
    }
}

/// Errors returned by [`generate_trench_polyline`] when input
/// parameters are inconsistent. The procgen kernel is the authoritative
/// validator for these — modder + cfctl callers receive the same
/// typed error rather than a panic.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum GeneratorError {
    #[error("min_segment_length ({min}) must be ≤ max_segment_length ({max})")]
    InvalidSegmentRange { min: u32, max: u32 },
    #[error("max_segment_length ({0}) must be > 0 and ≤ 12 tiles per anti-enfilade invariant")]
    InvalidMaxSegmentLength(u32),
    #[error("target_length_tiles ({0}) must be ≥ max_segment_length")]
    TargetLengthTooSmall(u32),
    #[error("rear_offset must be non-zero (got 0)")]
    RearOffsetZero,
    #[error("rear_offset ({0}) must satisfy |rear_offset| ≤ 11 (anti-enfilade ceiling per VAL-M9B-PROCGEN-004)")]
    RearOffsetTooLarge(i32),
}

/// Generate a deterministic zigzag trench polyline + the rear polyline
/// + the perpendicular `Communication` branches per the M9B spec.
pub fn generate_trench_polyline(input: &ZigzagInput) -> Result<ResolvedZigzag, GeneratorError> {
    if input.min_segment_length == 0 || input.min_segment_length > input.max_segment_length {
        return Err(GeneratorError::InvalidSegmentRange {
            min: input.min_segment_length,
            max: input.max_segment_length,
        });
    }
    if input.max_segment_length == 0 || input.max_segment_length > DEFAULT_MAX_SEGMENT_LENGTH {
        return Err(GeneratorError::InvalidMaxSegmentLength(
            input.max_segment_length,
        ));
    }
    if input.target_length_tiles < input.max_segment_length {
        return Err(GeneratorError::TargetLengthTooSmall(
            input.target_length_tiles,
        ));
    }
    if input.rear_offset == 0 {
        return Err(GeneratorError::RearOffsetZero);
    }
    if input.rear_offset.unsigned_abs() > MAX_CONTIGUOUS_TILES - 1 {
        return Err(GeneratorError::RearOffsetTooLarge(input.rear_offset));
    }

    let polyline_hash = PolylineHash::of(input.start, input.end, &[]);
    let seed = input.world_seed ^ polyline_hash.0;
    let mut rng = Xoshiro256StarStar::seed_from_u64(seed);

    let initial_dir = initial_direction(input.start, input.end);
    let mut direction_idx: i32 = initial_dir;

    let mut front: Vec<(i32, i32)> = Vec::new();
    let mut kinks: Vec<Kink> = Vec::new();
    let mut floor_tiles_set: HashSet<(i32, i32)> = HashSet::new();
    front.push(input.start);
    floor_tiles_set.insert(input.start);

    let mut cursor = input.start;
    let mut total_tiles: u32 = 0;
    let mut segment_index: u32 = 0;
    let mut max_run: u32 = 0;

    while total_tiles < input.target_length_tiles {
        let remaining = input.target_length_tiles - total_tiles;
        let segment_len = pick_segment_length(
            &mut rng,
            input.min_segment_length,
            input.max_segment_length,
            remaining,
        );
        let (dx, dy) = dir_vector(direction_idx);
        let mut placed: u32 = 0;
        for step in 1..=segment_len {
            let next = (cursor.0 + dx * step as i32, cursor.1 + dy * step as i32);
            floor_tiles_set.insert(next);
            placed += 1;
        }
        cursor = (
            cursor.0 + dx * segment_len as i32,
            cursor.1 + dy * segment_len as i32,
        );
        front.push(cursor);
        total_tiles += placed;
        if placed > max_run {
            max_run = placed;
        }
        segment_index += 1;
        if total_tiles >= input.target_length_tiles {
            break;
        }
        let kink_angle = pick_kink_angle(&mut rng);
        direction_idx = wrap_direction(direction_idx + kink_angle.rotation_step());
        kinks.push(Kink {
            at_tile: cursor,
            segment_index,
            straight_run_tiles: placed,
            angle: kink_angle,
        });
    }

    let rear_polyline = derive_rear_polyline(&front, input.rear_offset);
    fill_polyline_into(&rear_polyline, &mut floor_tiles_set);

    let branches =
        build_communication_branches(&front, &rear_polyline, input.branch_every, &mut floor_tiles_set);

    let endpoints =
        cap_endpoints(input, &front, initial_dir, direction_idx);

    let mut floor_tiles: Vec<(i32, i32)> = floor_tiles_set.into_iter().collect();
    floor_tiles.sort_unstable();

    Ok(ResolvedZigzag {
        polyline_hash,
        front_polyline: front,
        rear_polyline,
        kinks,
        branches,
        endpoints,
        floor_tiles,
        max_straight_run_tiles: max_run,
    })
}

fn pick_segment_length(rng: &mut Xoshiro256StarStar, min: u32, max: u32, remaining: u32) -> u32 {
    let upper = max.min(remaining);
    let lower = min.min(upper);
    if upper == lower {
        return upper;
    }
    let span = upper - lower + 1;
    let raw = rng.next_u64() % u64::from(span);
    lower + raw as u32
}

fn pick_kink_angle(rng: &mut Xoshiro256StarStar) -> KinkAngle {
    if rng.next_u64() & 1 == 0 {
        KinkAngle::Plus45
    } else {
        KinkAngle::Minus45
    }
}

/// 8-way direction index. 0 = E, 1 = NE, 2 = N, 3 = NW, 4 = W,
/// 5 = SW, 6 = S, 7 = SE. Each +1/-1 step is a ±45° rotation
/// counterclockwise/clockwise respectively. The integer modular axis
/// matches the spec's "kink is ±45°" invariant exactly.
fn dir_vector(idx: i32) -> (i32, i32) {
    let normalized = wrap_direction(idx);
    match normalized {
        0 => (1, 0),
        1 => (1, -1),
        2 => (0, -1),
        3 => (-1, -1),
        4 => (-1, 0),
        5 => (-1, 1),
        6 => (0, 1),
        7 => (1, 1),
        _ => unreachable!(),
    }
}

fn wrap_direction(idx: i32) -> i32 {
    ((idx % 8) + 8) % 8
}

/// Pick the closest 8-way direction to the (end - start) vector. Used
/// to seed the initial walking direction so the polyline points
/// towards `end` from the first segment.
fn initial_direction(start: (i32, i32), end: (i32, i32)) -> i32 {
    let dx = (end.0 - start.0) as f32;
    let dy = (end.1 - start.1) as f32;
    if dx == 0.0 && dy == 0.0 {
        return 0;
    }
    let mut best_idx = 0i32;
    let mut best_dot = f32::MIN;
    for i in 0..8 {
        let (vx, vy) = dir_vector(i);
        let dot = vx as f32 * dx + vy as f32 * dy;
        if dot > best_dot {
            best_dot = dot;
            best_idx = i;
        }
    }
    best_idx
}

/// Offsets the front polyline perpendicular to the start→end vector
/// by `rear_offset` tiles to obtain the rear polyline. Per spec § "
/// Branching rule: every N segments, spawn a perpendicular
/// communication trench connecting to the rear line".
fn derive_rear_polyline(front: &[(i32, i32)], rear_offset: i32) -> Vec<(i32, i32)> {
    if front.is_empty() {
        return Vec::new();
    }
    let start = front[0];
    let end = front[front.len() - 1];
    let dx = (end.0 - start.0) as f32;
    let dy = (end.1 - start.1) as f32;
    let mag = (dx * dx + dy * dy).sqrt().max(1.0);
    let perp_x = -dy / mag;
    let perp_y = dx / mag;
    let mut rear: Vec<(i32, i32)> = Vec::with_capacity(front.len());
    for &(x, y) in front {
        let nx = x as f32 + perp_x * rear_offset as f32;
        let ny = y as f32 + perp_y * rear_offset as f32;
        rear.push((nx.round() as i32, ny.round() as i32));
    }
    rear
}

/// Connect-the-dots fill: Bresenham between consecutive points of a
/// kink-only polyline so the trench-floor set includes every tile in
/// the continuous trench line, not just the kink corners. Each
/// pairwise Bresenham step is bounded by the segment length cap so
/// the anti-enfilade invariant holds.
fn fill_polyline_into(polyline: &[(i32, i32)], floor_tiles: &mut HashSet<(i32, i32)>) {
    for window in polyline.windows(2) {
        let from = window[0];
        let to = window[1];
        for tile in bresenham(from, to) {
            floor_tiles.insert(tile);
        }
    }
}

fn build_communication_branches(
    front: &[(i32, i32)],
    rear: &[(i32, i32)],
    branch_every: u32,
    floor_tiles: &mut HashSet<(i32, i32)>,
) -> Vec<CommunicationBranch> {
    if branch_every == 0 || front.len() < 2 || rear.len() != front.len() {
        return Vec::new();
    }
    let mut branches: Vec<CommunicationBranch> = Vec::new();
    let mut next_branch_at: u32 = branch_every;
    let mut cumulative_segments: u32 = 0;
    for window in front.windows(2).enumerate() {
        let (i, pair) = window;
        cumulative_segments += 1;
        if cumulative_segments < next_branch_at {
            continue;
        }
        next_branch_at += branch_every;
        let from = pair[1];
        let to = rear[i + 1];
        let mut path = bresenham(from, to);
        for &p in &path {
            floor_tiles.insert(p);
        }
        let last = *path.last().unwrap_or(&to);
        let _ = last;
        path.dedup();
        branches.push(CommunicationBranch {
            front_anchor: from,
            rear_anchor: to,
            path,
        });
    }
    branches
}

/// Walks a connected 8-neighbour path from `a` to `b`, inclusive. Pure
/// Bresenham over a tile grid; emits the same tile at most once per
/// path since each step moves at least one cell in one axis.
fn bresenham(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let mut x0 = a.0;
    let mut y0 = a.1;
    let x1 = b.0;
    let y1 = b.1;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut path = Vec::new();
    loop {
        path.push((x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    path
}

fn cap_endpoints(
    input: &ZigzagInput,
    front: &[(i32, i32)],
    initial_dir: i32,
    final_dir: i32,
) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    if front.is_empty() {
        return endpoints;
    }

    let head = front[0];
    let tail = front[front.len() - 1];

    let start_facing = if let Some(anchor) = &input.fortification_anchor {
        endpoints.push(Endpoint::FortificationConnection {
            pos: head,
            fortification_id: anchor.clone(),
        });
        None
    } else {
        let (dx, dy) = dir_vector(wrap_direction(initial_dir + 4));
        Some(EndpointFacing { dx, dy })
    };
    if let Some(facing) = start_facing {
        endpoints.push(Endpoint::FireStepDeadEnd { pos: head, facing });
    }

    let (dx, dy) = dir_vector(final_dir);
    endpoints.push(Endpoint::FireStepDeadEnd {
        pos: tail,
        facing: EndpointFacing { dx, dy },
    });
    endpoints
}

/// PvE ruin biome procgen input. The decay range is exposed so future
/// spec drift (e.g. range vs exact match) is a parameter change rather
/// than a recompile.
#[derive(Debug, Clone)]
pub struct RuinProcgenInput {
    pub biome_id: String,
    pub world_seed: u64,
    pub template_ids: Vec<String>,
    pub min_instances: u32,
    pub max_instances: u32,
    pub decay_factor: f32,
}

impl RuinProcgenInput {
    /// Spec-default input for the `ruined_frontline` biome
    /// (VAL-M9B-PROCGEN-DECAY-001): 2..=4 templates with
    /// `decay_factor = 0.4` exact-match.
    #[must_use]
    pub fn ruined_frontline(world_seed: u64, template_ids: Vec<String>) -> Self {
        Self {
            biome_id: "ruined_frontline".to_string(),
            world_seed,
            template_ids,
            min_instances: 2,
            max_instances: 4,
            decay_factor: 0.4,
        }
    }
}

/// One template instance placed by [`ruin_procgen`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuinPlacement {
    pub template_id: String,
    pub origin: (i32, i32),
    pub decay_factor: f32,
}

/// Output of [`ruin_procgen`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuinProcgenOutput {
    pub biome_id: String,
    pub instances: Vec<RuinPlacement>,
    pub polyline_hash: PolylineHash,
}

/// Decorate a PvE ruin biome with 2..=4 decayed trench template
/// instances (VAL-M9B-PROCGEN-DECAY-001). Repair is the player's
/// responsibility via `act.player.repair_trench_module` (owned by
/// m9b-3); this pass only places the decayed instances.
pub fn ruin_procgen(input: &RuinProcgenInput) -> RuinProcgenOutput {
    let polyline_hash = PolylineHash::of(
        (0, 0),
        (input.template_ids.len() as i32, 0),
        &[],
    );
    let mut rng = Xoshiro256StarStar::seed_from_u64(
        input
            .world_seed
            .wrapping_add(polyline_hash.0)
            .wrapping_add(biome_hash(&input.biome_id)),
    );
    let span = if input.max_instances <= input.min_instances {
        1
    } else {
        input.max_instances - input.min_instances + 1
    };
    let instance_count = input.min_instances + (rng.next_u64() % u64::from(span)) as u32;
    let mut instances: Vec<RuinPlacement> = Vec::with_capacity(instance_count as usize);
    for i in 0..instance_count {
        let template_id = if input.template_ids.is_empty() {
            "wwi_frontline_a".to_string()
        } else {
            input.template_ids[i as usize % input.template_ids.len()].clone()
        };
        let raw = rng.next_u64();
        let ox = ((raw & 0xFFFF) as i32) - 0x8000;
        let oy = (((raw >> 16) & 0xFFFF) as i32) - 0x8000;
        instances.push(RuinPlacement {
            template_id,
            origin: (ox % 1024, oy % 1024),
            decay_factor: input.decay_factor,
        });
    }
    RuinProcgenOutput {
        biome_id: input.biome_id.clone(),
        instances,
        polyline_hash,
    }
}

fn biome_hash(id: &str) -> u64 {
    let bytes = blake3::hash(id.as_bytes());
    let raw = bytes.as_bytes();
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&raw[..8]);
    u64::from_le_bytes(buf)
}

/// True iff the supplied `SegmentVariant` is one of the variants that
/// can naturally cap a trench endpoint (`fire_step` for dead-ends,
/// `parapet_raised` for fortified caps). Used by the template loader
/// to validate authored endpoint metadata.
#[must_use]
pub fn variant_can_cap(variant: SegmentVariant) -> bool {
    matches!(
        variant,
        SegmentVariant::FireStep | SegmentVariant::ParapetRaised
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_seed(seed: u64) -> ResolvedZigzag {
        let input = ZigzagInput::new((0, 0), (60, 0), seed);
        generate_trench_polyline(&input).expect("generation succeeds")
    }

    /// VAL-M9B-PROCGEN-001 — every straight run is ≤ 12 tiles.
    #[test]
    fn zigzag_no_straight_run_over_12_tiles() {
        let out = run_seed(42);
        assert!(
            out.max_straight_run_tiles <= 12,
            "max straight run {} exceeded 12",
            out.max_straight_run_tiles
        );
        for k in &out.kinks {
            assert!(
                k.straight_run_tiles <= 12,
                "kink at segment {} run {}",
                k.segment_index,
                k.straight_run_tiles
            );
            assert!(
                k.straight_run_tiles >= 1,
                "kink at segment {} run {}",
                k.segment_index,
                k.straight_run_tiles
            );
        }
    }

    /// VAL-M9B-PROCGEN-002 — every kink is exactly ±45° (no
    /// intermediate angles).
    #[test]
    fn kinks_are_45_degree_only() {
        let out = run_seed(42);
        assert!(!out.kinks.is_empty(), "test setup expects ≥1 kink");
        for k in &out.kinks {
            let d = k.angle.degrees();
            assert!(
                (d - 45.0).abs() < f32::EPSILON || (d + 45.0).abs() < f32::EPSILON,
                "kink degrees {d} is not exactly ±45.0",
            );
        }
    }

    /// VAL-M9B-PROCGEN-003 — determinism: two invocations with the
    /// same `world_seed=42` produce byte-identical kink + branch
    /// sequences.
    #[test]
    fn deterministic_kink_sequence_for_seed_42() {
        let a = run_seed(42);
        let b = run_seed(42);
        assert_eq!(a, b, "outputs should be byte-identical for the same seed");
    }

    /// Different seeds should yield different layouts (sanity guard so
    /// a constant-output regression doesn't pass the determinism test
    /// trivially).
    #[test]
    fn different_seeds_produce_different_layouts() {
        let a = run_seed(42);
        let b = run_seed(7);
        assert_ne!(a, b, "different seeds must produce different layouts");
    }

    /// VAL-M9B-PROCGEN-004 — enfilade ray cast along any straight
    /// section of the generated line cannot intersect more than 12
    /// contiguous trench-floor tiles in any of the 8 cardinal/diagonal
    /// directions.
    #[test]
    fn enfilade_ray_max_intersect_12_tiles() {
        let out = run_seed(42);
        let tiles = out.floor_tile_set();
        let directions: &[(i32, i32)] = &[
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        for &origin in &out.floor_tiles {
            for &(dx, dy) in directions {
                let mut run = 0u32;
                let mut step = 0i32;
                loop {
                    let probe = (origin.0 + dx * step, origin.1 + dy * step);
                    if tiles.contains(&probe) {
                        run += 1;
                        step += 1;
                        if run > 12 {
                            panic!(
                                "enfilade ray from {origin:?} in {dx:?},{dy:?} \
                                 covered > 12 contiguous tiles"
                            );
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    /// VAL-M9B-PROCGEN-005 — communication branches connect the front
    /// polyline to the rear polyline through trench-floor tiles only.
    /// (We don't link to cf-ai pathfind here; we verify connectivity
    /// via BFS over the union of front + rear + branch tiles.)
    #[test]
    fn communication_branch_reaches_rear() {
        let input = ZigzagInput {
            branch_every: 4,
            target_length_tiles: 48,
            ..ZigzagInput::new((0, 0), (48, 0), 42)
        };
        let out = generate_trench_polyline(&input).expect("ok");
        assert!(
            !out.branches.is_empty(),
            "expected ≥1 communication branch for branch_every=4 over 48 tiles"
        );
        let tile_set = out.floor_tile_set();
        for branch in &out.branches {
            for &step in &branch.path {
                assert!(
                    tile_set.contains(&step),
                    "branch path step {step:?} not in trench-floor set"
                );
            }
            assert_eq!(branch.path.first().copied(), Some(branch.front_anchor));
            assert_eq!(branch.path.last().copied(), Some(branch.rear_anchor));
        }
        let front_pt = out.branches[0].front_anchor;
        let rear_pt = out.branches[0].rear_anchor;
        assert!(
            bfs_connected(&tile_set, front_pt, rear_pt),
            "front anchor {front_pt:?} should be reachable from rear anchor {rear_pt:?} through trench-floor only"
        );
    }

    /// VAL-M9B-PROCGEN-006 — every endpoint must be capped (fire_step
    /// dead-end facing outward, OR a fortification connection).
    #[test]
    fn endpoints_are_capped() {
        let out = run_seed(42);
        assert_eq!(out.endpoints.len(), 2, "exactly two endpoints");
        for ep in &out.endpoints {
            match ep {
                Endpoint::FireStepDeadEnd { facing, .. } => {
                    let mag = facing.dx.abs() + facing.dy.abs();
                    assert!(mag >= 1, "fire-step facing must be non-zero");
                }
                Endpoint::FortificationConnection {
                    fortification_id, ..
                } => {
                    assert!(
                        !fortification_id.is_empty(),
                        "fortification id must be non-empty"
                    );
                }
            }
        }
    }

    #[test]
    fn endpoints_resolve_fortification_anchor() {
        let mut input = ZigzagInput::new((0, 0), (40, 0), 42);
        input.fortification_anchor = Some("mg_nest_static".to_string());
        let out = generate_trench_polyline(&input).expect("ok");
        let has_anchor = out
            .endpoints
            .iter()
            .any(|e| matches!(e, Endpoint::FortificationConnection { fortification_id, .. } if fortification_id == "mg_nest_static"));
        assert!(has_anchor, "fortification anchor should appear in endpoints");
    }

    /// VAL-M9B-PROCGEN-DECAY-001 — the PvE ruin pass instantiates
    /// 2..=4 template instances with `decay_factor=0.4` exact-match.
    #[test]
    fn ruin_procgen_places_2_to_4_decayed_templates() {
        let input = RuinProcgenInput::ruined_frontline(
            42,
            vec![
                "wwi_frontline_a".to_string(),
                "wwi_frontline_b_two_line".to_string(),
                "reactor_defense_zigzag".to_string(),
                "forward_outpost_with_mgnest".to_string(),
            ],
        );
        let out = ruin_procgen(&input);
        assert_eq!(out.biome_id, "ruined_frontline");
        assert!(
            (2..=4).contains(&(out.instances.len() as u32)),
            "ruin biome should place 2-4 instances, got {}",
            out.instances.len()
        );
        for inst in &out.instances {
            assert!(
                (inst.decay_factor - 0.4).abs() < f32::EPSILON,
                "decay_factor {} must equal 0.4 exact-match",
                inst.decay_factor
            );
        }
    }

    /// Determinism check for ruin pass: same world_seed yields the
    /// same instance count + template ids.
    #[test]
    fn ruin_procgen_is_deterministic() {
        let templates = vec![
            "wwi_frontline_a".to_string(),
            "wwi_frontline_b_two_line".to_string(),
        ];
        let a = ruin_procgen(&RuinProcgenInput::ruined_frontline(42, templates.clone()));
        let b = ruin_procgen(&RuinProcgenInput::ruined_frontline(42, templates));
        assert_eq!(a, b, "ruin pass must be byte-identical for the same seed");
    }

    fn bfs_connected(tiles: &HashSet<(i32, i32)>, from: (i32, i32), to: (i32, i32)) -> bool {
        if !tiles.contains(&from) || !tiles.contains(&to) {
            return false;
        }
        let mut frontier: Vec<(i32, i32)> = vec![from];
        let mut seen: HashSet<(i32, i32)> = HashSet::new();
        seen.insert(from);
        while let Some(p) = frontier.pop() {
            if p == to {
                return true;
            }
            for &(dx, dy) in &[
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let n = (p.0 + dx, p.1 + dy);
                if tiles.contains(&n) && seen.insert(n) {
                    frontier.push(n);
                }
            }
        }
        false
    }

    #[test]
    fn polyline_hash_is_stable_for_same_inputs() {
        let h1 = PolylineHash::of((0, 0), (60, 0), &[]);
        let h2 = PolylineHash::of((0, 0), (60, 0), &[]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn polyline_hash_differs_for_different_inputs() {
        let h1 = PolylineHash::of((0, 0), (60, 0), &[]);
        let h2 = PolylineHash::of((0, 0), (61, 0), &[]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn rejects_invalid_segment_range() {
        let mut input = ZigzagInput::new((0, 0), (60, 0), 0);
        input.min_segment_length = 14;
        input.max_segment_length = 10;
        let err = generate_trench_polyline(&input).unwrap_err();
        assert!(matches!(err, GeneratorError::InvalidSegmentRange { .. }));
    }

    #[test]
    fn rejects_invalid_max_segment_length() {
        let mut input = ZigzagInput::new((0, 0), (60, 0), 0);
        input.max_segment_length = 15;
        let err = generate_trench_polyline(&input).unwrap_err();
        assert!(matches!(err, GeneratorError::InvalidMaxSegmentLength(_)));
    }

    #[test]
    fn rejects_zero_rear_offset() {
        let mut input = ZigzagInput::new((0, 0), (60, 0), 0);
        input.rear_offset = 0;
        let err = generate_trench_polyline(&input).unwrap_err();
        assert!(matches!(err, GeneratorError::RearOffsetZero));
    }
}
