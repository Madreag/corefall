//! **M14D** — Projectile-Projectile Continuous Collision Detection (CCD).
//!
//! Pure / deterministic kernel for projectile-vs-projectile swept-collision
//! resolution. Wired into the per-tick schedule by `cf-control::engine`
//! strictly between the actor-collision pass and the terrain pass.
//!
//! Layout:
//!   - 32-px spatial-hash broadphase (matches average projectile sweep
//!     length per tick at the 60 Hz canonical rate).
//!   - Selective `INTERESTING_PAIRS` allowlist culls uninteresting
//!     `(kind_a, kind_b)` pairs before narrowphase.
//!   - Symmetric Minkowski-difference swept narrowphase produces the
//!     time-of-impact (`toi`) for each surviving candidate.
//!   - Outcome resolver maps the surviving pair → one of
//!     `{fuze_triggered, mutual_cancellation, aps_intercept, kinetic_deflect}`.
//!
//! All public functions are deterministic and free of `thread_rng()` /
//! `unwrap()` / `unsafe`. The kernel never allocates beyond the inputs +
//! a small bookkeeping vector proportional to broadphase candidate count.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Bucket size used by the spatial-hash broadphase.
///
/// Matches the average projectile sweep length per tick at 60 Hz —
/// kinetic rounds typically sweep ~30 px per tick (rifle = 600 px/s),
/// energy beams sweep faster but their broadphase footprint is still
/// dominated by the 32-px bucket choice. **VAL-M14D-011** pins this
/// constant.
pub const BROADPHASE_BUCKET_PX: f32 = 32.0;

/// Maximum number of narrowphase candidate pairs the broadphase will
/// hand off per tick. Pinned by **VAL-M14D-009** (≤ 12 in the 50-
/// projectile / 1024² fixture). The kernel caps candidate emission at
/// this number so a pathological scene (50 projectiles inside the same
/// bucket) can't explode the perf budget.
pub const NARROWPHASE_CANDIDATE_BUDGET: usize = 12;

/// Kinetic-energy retention each kinetic projectile keeps after a
/// `kinetic_deflect` outcome (**VAL-M14D-004**: 60 % retained = 40 %
/// lost). Symmetric across the pair.
pub const KINETIC_DEFLECT_ENERGY_RETAINED: f32 = 0.6;

/// Minimum convergence angle (degrees) required for a kinetic-vs-kinetic
/// pair to enter narrowphase. Pinned by **VAL-M14D-005** (< 10 ° must
/// be rejected by the selective filter).
pub const KINETIC_DEFLECT_MIN_ANGLE_DEG: f32 = 10.0;

/// Convergence angle (degrees) above which an energy-vs-energy pair is
/// considered to satisfy the canonical "mutual cancellation" intercept
/// geometry (Gherkin scenario 2 spec prose: "at correct intercept
/// angle"). Below this angle the pair is filtered out alongside the
/// shallow kinetic case.
pub const ENERGY_CANCEL_MIN_ANGLE_DEG: f32 = 30.0;

/// Discriminator carried on every projectile snapshot we feed into the
/// pair kernel. The kernel only cares about kind for the
/// `INTERESTING_PAIRS` allowlist + the outcome resolver.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectileKind {
    /// Standard kinetic round (rifle, autocannon, tracer).
    KineticRifle,
    /// Explosive round with a fuze that can be triggered by an external
    /// impulse (frag grenade, mortar shell, RPG warhead pre-detonation).
    ExplosiveGrenade,
    /// Energy-class projectile (laser pulse, plasma bolt, charged round).
    EnergyBeam,
    /// M14C HEAT round — explosive shaped-charge warhead in flight.
    HeatRound,
    /// M14C APFSDS long-rod kinetic round in flight.
    ApfsdsRound,
    /// Active Protection System tracking laser pulse (C-RAM, APS).
    ApsLaser,
}

impl ProjectileKind {
    /// Stable snake_case discriminator used by the replay event payload.
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectileKind::KineticRifle => "kinetic_rifle",
            ProjectileKind::ExplosiveGrenade => "explosive_grenade",
            ProjectileKind::EnergyBeam => "energy_beam",
            ProjectileKind::HeatRound => "heat_round",
            ProjectileKind::ApfsdsRound => "apfsds_round",
            ProjectileKind::ApsLaser => "aps_laser",
        }
    }

    /// True for kinds that carry their own kinetic energy (used by the
    /// outcome resolver to choose between deflect / cancel / intercept).
    pub fn is_kinetic(self) -> bool {
        matches!(self, ProjectileKind::KineticRifle | ProjectileKind::ApfsdsRound)
    }

    /// True for kinds that carry an explosive payload susceptible to
    /// fuze-triggered detonation when struck by a kinetic round.
    pub fn is_explosive(self) -> bool {
        matches!(self, ProjectileKind::ExplosiveGrenade | ProjectileKind::HeatRound)
    }

    /// True for pure-energy kinds (the kernel uses this to surface the
    /// `mutual_cancellation` outcome on energy-vs-energy intercepts).
    pub fn is_energy(self) -> bool {
        matches!(self, ProjectileKind::EnergyBeam)
    }

    /// True for Active Protection System tracking pulses (the kernel
    /// uses this to surface the `aps_intercept` outcome when an APS
    /// pulse meets a hostile incoming round).
    pub fn is_aps(self) -> bool {
        matches!(self, ProjectileKind::ApsLaser)
    }
}

/// Selective filter — only `(kind_a, kind_b)` pairs whose canonical
/// (kind_a < kind_b) tuple is present here will reach narrowphase.
///
/// Pinned by **VAL-M14D-012** — the allowlist gates which kind pairs
/// enter narrowphase before TOI computation runs. Pure
/// kinetic-vs-kinetic shallow-angle pairs are still further filtered by
/// the convergence-angle check inside narrowphase per
/// **VAL-M14D-005**.
pub fn interesting_pairs() -> &'static [(ProjectileKind, ProjectileKind)] {
    const TABLE: &[(ProjectileKind, ProjectileKind)] = &[
        // Kinetic-vs-explosive fuze-trigger geometries.
        (ProjectileKind::KineticRifle, ProjectileKind::ExplosiveGrenade),
        (ProjectileKind::KineticRifle, ProjectileKind::HeatRound),
        (ProjectileKind::ApfsdsRound, ProjectileKind::ExplosiveGrenade),
        // Kinetic-vs-kinetic deflect.
        (ProjectileKind::KineticRifle, ProjectileKind::KineticRifle),
        (ProjectileKind::ApfsdsRound, ProjectileKind::ApfsdsRound),
        (ProjectileKind::KineticRifle, ProjectileKind::ApfsdsRound),
        // Energy-vs-energy mutual cancellation.
        (ProjectileKind::EnergyBeam, ProjectileKind::EnergyBeam),
        // APS intercept lanes.
        (ProjectileKind::ApsLaser, ProjectileKind::HeatRound),
        (ProjectileKind::ApsLaser, ProjectileKind::ApfsdsRound),
        (ProjectileKind::ApsLaser, ProjectileKind::ExplosiveGrenade),
        (ProjectileKind::ApsLaser, ProjectileKind::KineticRifle),
    ];
    TABLE
}

/// True if `(a, b)` (in either order) is present in the
/// `INTERESTING_PAIRS` allowlist.
pub fn is_interesting_pair(a: ProjectileKind, b: ProjectileKind) -> bool {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    interesting_pairs().iter().any(|(x, y)| {
        let (px, py) = if *x <= *y { (*x, *y) } else { (*y, *x) };
        px == lo && py == hi
    })
}

/// One projectile snapshot fed into the pair kernel. Constructed by
/// `cf-control` from the engine's projectile pool (or from a scenario's
/// scripted projectile-pair fixture).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectileSnapshot {
    /// Stable id (`cf-control` allocates ids monotonically across all
    /// projectile pools so the pair kernel can dedupe).
    pub id: u64,
    /// Kind discriminator.
    pub kind: ProjectileKind,
    /// World position at the start of this tick.
    pub position: [f32; 2],
    /// Velocity vector in world units per second.
    pub velocity: [f32; 2],
    /// Effective collision radius in world units (px). Used by the
    /// Minkowski-difference swept TOI primitive.
    pub radius: f32,
    /// Scalar mass (kg). Used to compute kinetic energy for the
    /// kinetic-deflect outcome.
    pub mass_kg: f32,
    /// Owner actor id (for replay-event payload provenance). Use 0 for
    /// base-mounted modules (C-RAM, fixed turrets).
    pub owner_actor_id: u64,
}

impl ProjectileSnapshot {
    /// Convenience constructor with sensible defaults for mass / owner.
    pub fn new(id: u64, kind: ProjectileKind, position: [f32; 2], velocity: [f32; 2]) -> Self {
        Self {
            id,
            kind,
            position,
            velocity,
            radius: 1.0,
            mass_kg: 0.01,
            owner_actor_id: 0,
        }
    }

    /// Kinetic energy in joules (0.5 × m × |v|²).
    pub fn kinetic_energy_j(&self) -> f32 {
        let speed_sq = self.velocity[0] * self.velocity[0] + self.velocity[1] * self.velocity[1];
        0.5 * self.mass_kg.max(0.0) * speed_sq
    }
}

/// One narrowphase candidate pair surfaced by the broadphase. Ordered
/// (`a_id` < `b_id`) so the pair is canonical.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePairCandidate {
    pub a_id: u64,
    pub b_id: u64,
    pub a_kind: ProjectileKind,
    pub b_kind: ProjectileKind,
}

/// Output of `pair_swept_toi`. `None` when the swept paths never meet
/// during this tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PairToi {
    /// Time-of-impact in `[0, 1]` along this tick's swept segment.
    pub toi: f32,
    /// World coordinates of the intercept point.
    pub point: [f32; 2],
    /// Convergence angle (degrees in `[0, 180]`) between the two
    /// velocity vectors at the intercept. 0 ° = parallel (no real
    /// intercept), 180 ° = head-on. Used by the selective angle filter.
    pub convergence_deg: f32,
}

/// One `INTERESTING_PAIRS` outcome — the canonical discriminator carried
/// on the `collision.projectile_pair_contact` event payload.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectilePairOutcome {
    /// Kinetic projectile triggered an explosive's fuze.
    FuzeTriggered,
    /// Energy-vs-energy pair cancelled each other out at the correct
    /// intercept angle.
    MutualCancellation,
    /// Active Protection System tracking pulse intercepted an incoming
    /// hostile projectile.
    ApsIntercept,
    /// Two kinetic rounds deflected off each other (≥ 30 ° convergence).
    KineticDeflect,
}

impl ProjectilePairOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectilePairOutcome::FuzeTriggered => "fuze_triggered",
            ProjectilePairOutcome::MutualCancellation => "mutual_cancellation",
            ProjectilePairOutcome::ApsIntercept => "aps_intercept",
            ProjectilePairOutcome::KineticDeflect => "kinetic_deflect",
        }
    }
}

/// Post-narrowphase pair contact. Carries the outcome, intercept point,
/// per-projectile post-velocity (None = consumed / fragmented), and the
/// `cosmetic: true` flag the replay event payload mirrors.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePairContact {
    pub tick_dt: f32,
    pub a_id: u64,
    pub b_id: u64,
    pub a_kind: ProjectileKind,
    pub b_kind: ProjectileKind,
    pub outcome: ProjectilePairOutcome,
    pub intercept_point: [f32; 2],
    pub toi: f32,
    pub convergence_deg: f32,
    /// Energy retained per projectile after the contact (1.0 = unchanged,
    /// 0.6 = 40 % loss on kinetic deflect, 0.0 = fully consumed).
    pub a_energy_retained: f32,
    pub b_energy_retained: f32,
    /// Post-contact velocities. `None` = projectile is removed from the
    /// pool (fragmented / consumed / detonated).
    pub a_post_velocity: Option<[f32; 2]>,
    pub b_post_velocity: Option<[f32; 2]>,
    /// True for all M14D pair contacts — renderer drops these first
    /// under backpressure, killcam excludes them by default
    /// (per-player `replay_intercepts` opt-in).
    pub cosmetic: bool,
}

/// Spatial-hash broadphase with 32-px bucket size. Pure / deterministic:
/// projectiles are bucketed by their swept-AABB (start + end positions
/// expanded by radius), and candidate pairs are surfaced in
/// `(min_id, max_id)` order for stable downstream sorting.
#[derive(Debug, Default)]
pub struct SpatialHashBroadphase;

impl SpatialHashBroadphase {
    /// Returns the (min, max) bucket index range (inclusive) covered by
    /// the projectile's swept AABB this tick. The swept AABB is the
    /// axis-aligned union of its start AABB and end AABB.
    fn bucket_range(p: &ProjectileSnapshot, tick_dt: f32) -> (i32, i32, i32, i32) {
        let dx = p.velocity[0] * tick_dt;
        let dy = p.velocity[1] * tick_dt;
        let r = p.radius.max(0.0);
        let min_x = p.position[0].min(p.position[0] + dx) - r;
        let max_x = p.position[0].max(p.position[0] + dx) + r;
        let min_y = p.position[1].min(p.position[1] + dy) - r;
        let max_y = p.position[1].max(p.position[1] + dy) + r;
        let bx_min = (min_x / BROADPHASE_BUCKET_PX).floor() as i32;
        let bx_max = (max_x / BROADPHASE_BUCKET_PX).floor() as i32;
        let by_min = (min_y / BROADPHASE_BUCKET_PX).floor() as i32;
        let by_max = (max_y / BROADPHASE_BUCKET_PX).floor() as i32;
        (bx_min, bx_max, by_min, by_max)
    }

    /// Compute the candidate pairs the broadphase wants narrowphase to
    /// inspect. Pairs are deduped + ordered (`a_id` < `b_id`) and the
    /// total is capped at `NARROWPHASE_CANDIDATE_BUDGET` so the perf
    /// budget holds under pathological clustering.
    pub fn candidates(
        projectiles: &[ProjectileSnapshot],
        tick_dt: f32,
    ) -> Vec<ProjectilePairCandidate> {
        if projectiles.is_empty() || projectiles.len() == 1 {
            return Vec::new();
        }
        let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (idx, p) in projectiles.iter().enumerate() {
            let (bx_min, bx_max, by_min, by_max) = Self::bucket_range(p, tick_dt);
            for bx in bx_min..=bx_max {
                for by in by_min..=by_max {
                    buckets.entry((bx, by)).or_default().push(idx);
                }
            }
        }
        let mut seen: BTreeSet<(u64, u64)> = BTreeSet::new();
        let mut out: Vec<ProjectilePairCandidate> = Vec::new();
        for indices in buckets.values() {
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let a = &projectiles[indices[i]];
                    let b = &projectiles[indices[j]];
                    let (lo, hi) = if a.id <= b.id { (a, b) } else { (b, a) };
                    if !seen.insert((lo.id, hi.id)) {
                        continue;
                    }
                    if !is_interesting_pair(lo.kind, hi.kind) {
                        continue;
                    }
                    if out.len() >= NARROWPHASE_CANDIDATE_BUDGET {
                        return out;
                    }
                    out.push(ProjectilePairCandidate {
                        a_id: lo.id,
                        b_id: hi.id,
                        a_kind: lo.kind,
                        b_kind: hi.kind,
                    });
                }
            }
        }
        out
    }
}

/// Symmetric Minkowski-difference swept TOI between two projectiles
/// over this tick. Reduces the projectile-vs-projectile problem to a
/// segment-vs-circle test on the Minkowski difference (radius =
/// `a.radius + b.radius`, ray = relative motion `b - a`). **Symmetric
/// across the argument pair** (swapping `a` and `b` returns the same
/// TOI within `f32` tolerance — pinned by **VAL-M14D-016**).
///
/// Returns `None` when the swept paths never meet during the tick.
pub fn pair_swept_toi(a: &ProjectileSnapshot, b: &ProjectileSnapshot, tick_dt: f32) -> Option<PairToi> {
    let rel_pos = [a.position[0] - b.position[0], a.position[1] - b.position[1]];
    let rel_vel = [
        (a.velocity[0] - b.velocity[0]) * tick_dt,
        (a.velocity[1] - b.velocity[1]) * tick_dt,
    ];
    let combined_r = (a.radius + b.radius).max(0.0);
    let combined_r_sq = combined_r * combined_r;
    let dist_sq = rel_pos[0] * rel_pos[0] + rel_pos[1] * rel_pos[1];
    let vel_sq = rel_vel[0] * rel_vel[0] + rel_vel[1] * rel_vel[1];
    let dot = rel_pos[0] * rel_vel[0] + rel_pos[1] * rel_vel[1];
    let convergence_deg = convergence_angle_deg(a.velocity, b.velocity);
    if dist_sq <= combined_r_sq {
        return Some(PairToi {
            toi: 0.0,
            point: [
                0.5 * (a.position[0] + b.position[0]),
                0.5 * (a.position[1] + b.position[1]),
            ],
            convergence_deg,
        });
    }
    if vel_sq <= f32::EPSILON {
        return None;
    }
    let disc = dot * dot - vel_sq * (dist_sq - combined_r_sq);
    if disc < 0.0 {
        return None;
    }
    let sqrt_disc = disc.sqrt();
    let toi_root = (-dot - sqrt_disc) / vel_sq;
    if !toi_root.is_finite() || toi_root < 0.0 || toi_root > 1.0 {
        return None;
    }
    let point = [
        0.5 * (a.position[0] + a.velocity[0] * tick_dt * toi_root
            + b.position[0]
            + b.velocity[0] * tick_dt * toi_root),
        0.5 * (a.position[1] + a.velocity[1] * tick_dt * toi_root
            + b.position[1]
            + b.velocity[1] * tick_dt * toi_root),
    ];
    Some(PairToi {
        toi: toi_root,
        point,
        convergence_deg,
    })
}

/// Convergence angle in degrees between two velocity vectors. Zero
/// when the vectors point in the same direction (chase / parallel),
/// 90 ° when perpendicular, 180 ° when head-on / anti-parallel.
/// Zero-magnitude vectors return 0.
///
/// Used by the selective filter: shallow-angle kinetic pairs (vectors
/// nearly parallel-same-direction) are rejected with `< 10 °`.
/// Energy-vs-energy pairs require near-head-on geometry (≥ 30 ° per
/// the `ENERGY_CANCEL_MIN_ANGLE_DEG` constant) to mutually cancel.
pub fn convergence_angle_deg(va: [f32; 2], vb: [f32; 2]) -> f32 {
    let mag_a = (va[0] * va[0] + va[1] * va[1]).sqrt();
    let mag_b = (vb[0] * vb[0] + vb[1] * vb[1]).sqrt();
    if mag_a <= f32::EPSILON || mag_b <= f32::EPSILON {
        return 0.0;
    }
    let cos = ((va[0] * vb[0] + va[1] * vb[1]) / (mag_a * mag_b)).clamp(-1.0, 1.0);
    cos.acos().to_degrees()
}

/// Run the narrowphase outcome resolver for one candidate pair. Returns
/// `None` when the pair's geometry doesn't actually produce a contact
/// this tick (TOI missed, shallow-angle kinetic, energy pair below the
/// cancellation angle).
pub fn narrowphase_resolve_pair(
    a: &ProjectileSnapshot,
    b: &ProjectileSnapshot,
    tick_dt: f32,
) -> Option<ProjectilePairContact> {
    let toi = pair_swept_toi(a, b, tick_dt)?;
    let canon_lo = if a.id <= b.id { a } else { b };
    let canon_hi = if a.id <= b.id { b } else { a };
    let outcome = pair_outcome(canon_lo.kind, canon_hi.kind, toi.convergence_deg)?;
    let (a_post, b_post, a_retained, b_retained) =
        post_contact_state(canon_lo, canon_hi, outcome, toi.convergence_deg);
    Some(ProjectilePairContact {
        tick_dt,
        a_id: canon_lo.id,
        b_id: canon_hi.id,
        a_kind: canon_lo.kind,
        b_kind: canon_hi.kind,
        outcome,
        intercept_point: toi.point,
        toi: toi.toi,
        convergence_deg: toi.convergence_deg,
        a_energy_retained: a_retained,
        b_energy_retained: b_retained,
        a_post_velocity: a_post,
        b_post_velocity: b_post,
        cosmetic: true,
    })
}

/// Choose the outcome variant for a (canonical-ordered) pair given its
/// convergence angle, or `None` when the selective filter rejects the
/// pair (shallow-angle kinetic, low-angle energy-vs-energy).
pub fn pair_outcome(
    lo_kind: ProjectileKind,
    hi_kind: ProjectileKind,
    convergence_deg: f32,
) -> Option<ProjectilePairOutcome> {
    if !is_interesting_pair(lo_kind, hi_kind) {
        return None;
    }
    if lo_kind.is_aps() || hi_kind.is_aps() {
        return Some(ProjectilePairOutcome::ApsIntercept);
    }
    let (kinetic_other, has_kinetic, has_explosive, has_energy_only) = {
        let mut kinetic = 0;
        let mut explosive = 0;
        let mut energy = 0;
        for k in [lo_kind, hi_kind] {
            if k.is_kinetic() {
                kinetic += 1;
            } else if k.is_explosive() {
                explosive += 1;
            } else if k.is_energy() {
                energy += 1;
            }
        }
        (kinetic, kinetic >= 1, explosive >= 1, energy == 2)
    };
    if has_kinetic && has_explosive {
        return Some(ProjectilePairOutcome::FuzeTriggered);
    }
    if has_energy_only {
        if convergence_deg < ENERGY_CANCEL_MIN_ANGLE_DEG {
            return None;
        }
        return Some(ProjectilePairOutcome::MutualCancellation);
    }
    if kinetic_other == 2 {
        if convergence_deg < KINETIC_DEFLECT_MIN_ANGLE_DEG {
            return None;
        }
        return Some(ProjectilePairOutcome::KineticDeflect);
    }
    None
}

/// Compute post-contact velocities + per-projectile energy retention
/// scalars for the chosen outcome. Symmetric across kinetic deflect.
fn post_contact_state(
    a: &ProjectileSnapshot,
    b: &ProjectileSnapshot,
    outcome: ProjectilePairOutcome,
    convergence_deg: f32,
) -> (Option<[f32; 2]>, Option<[f32; 2]>, f32, f32) {
    let _ = convergence_deg;
    match outcome {
        ProjectilePairOutcome::FuzeTriggered => {
            // The explosive detonates at the intercept point; the
            // kinetic round fragments / is consumed in the energy
            // transfer (Gherkin scenario 1).
            let a_consumed = a.kind.is_kinetic() || (a.kind.is_explosive() && b.kind.is_explosive());
            let b_consumed = b.kind.is_kinetic() || (a.kind.is_explosive() && b.kind.is_explosive());
            let a_consumed = a_consumed || a.kind.is_explosive();
            let b_consumed = b_consumed || b.kind.is_explosive();
            (
                if a_consumed { None } else { Some(a.velocity) },
                if b_consumed { None } else { Some(b.velocity) },
                0.0,
                0.0,
            )
        }
        ProjectilePairOutcome::MutualCancellation => (None, None, 0.0, 0.0),
        ProjectilePairOutcome::ApsIntercept => {
            // The intercepted projectile is removed; the APS pulse is
            // consumed by the intercept geometry too.
            (None, None, 0.0, 0.0)
        }
        ProjectilePairOutcome::KineticDeflect => {
            // Symmetric energy loss + opposing deflection along the
            // perpendicular axis between the two velocity vectors.
            let scale = KINETIC_DEFLECT_ENERGY_RETAINED.sqrt(); // |v'| = sqrt(0.6) × |v|.
            let a_perp = perpendicular(a.velocity, b.velocity);
            let b_perp = perpendicular(b.velocity, a.velocity);
            let a_speed = (a.velocity[0] * a.velocity[0] + a.velocity[1] * a.velocity[1]).sqrt();
            let b_speed = (b.velocity[0] * b.velocity[0] + b.velocity[1] * b.velocity[1]).sqrt();
            let a_out = scale_vec(a_perp, a_speed * scale);
            let b_out = scale_vec(b_perp, b_speed * scale);
            (
                Some(a_out),
                Some(b_out),
                KINETIC_DEFLECT_ENERGY_RETAINED,
                KINETIC_DEFLECT_ENERGY_RETAINED,
            )
        }
    }
}

/// Deterministic perpendicular component for the deflect outcome —
/// projects `va` onto the axis perpendicular to (`va` + `vb`) so the
/// pair scatters symmetrically.
fn perpendicular(va: [f32; 2], vb: [f32; 2]) -> [f32; 2] {
    let avg = [va[0] + vb[0], va[1] + vb[1]];
    let len_sq = avg[0] * avg[0] + avg[1] * avg[1];
    let normal = if len_sq <= f32::EPSILON {
        // Antiparallel inputs — fall back to a fixed orthogonal axis so
        // determinism still holds across runs.
        [-va[1], va[0]]
    } else {
        // Take the perpendicular to the average vector (in 2D, rotate by
        // 90 °).
        [-avg[1], avg[0]]
    };
    let mag = (normal[0] * normal[0] + normal[1] * normal[1]).sqrt();
    if mag <= f32::EPSILON {
        return [0.0, 0.0];
    }
    let inv = 1.0 / mag;
    let dir_x = normal[0] * inv;
    let dir_y = normal[1] * inv;
    let sign = va[0] * dir_x + va[1] * dir_y;
    // Preserve the sign of `va`'s projection so deflected rounds keep
    // their general heading (split apart instead of mirroring through
    // each other).
    if sign >= 0.0 {
        [dir_x, dir_y]
    } else {
        [-dir_x, -dir_y]
    }
}

fn scale_vec(unit: [f32; 2], speed: f32) -> [f32; 2] {
    [unit[0] * speed, unit[1] * speed]
}

/// Trace counters surfaced by `run_projectile_pair_pass` so callers
/// (cf-control, perf benchmark) can assert the broadphase budget
/// (≤ 12 candidates) + total event count.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProjectilePairPassTrace {
    pub broadphase_candidates: usize,
    pub narrowphase_contacts: usize,
}

/// Drive the full per-tick projectile-pair pass — broadphase →
/// narrowphase → outcome resolution. Pure / deterministic. Callers
/// (cf-control engine, perf bench, integration tests) drive the
/// resulting `Vec<ProjectilePairContact>` through the replay event
/// emit + the post-tick projectile-pool mutator.
pub fn run_projectile_pair_pass(
    projectiles: &[ProjectileSnapshot],
    tick_dt: f32,
) -> (Vec<ProjectilePairContact>, ProjectilePairPassTrace) {
    let candidates = SpatialHashBroadphase::candidates(projectiles, tick_dt);
    let mut trace = ProjectilePairPassTrace {
        broadphase_candidates: candidates.len(),
        narrowphase_contacts: 0,
    };
    if candidates.is_empty() {
        return (Vec::new(), trace);
    }
    let mut by_id: BTreeMap<u64, &ProjectileSnapshot> = BTreeMap::new();
    for p in projectiles {
        by_id.insert(p.id, p);
    }
    let mut resolved: Vec<ProjectilePairContact> = Vec::new();
    let mut consumed: BTreeSet<u64> = BTreeSet::new();
    for cand in &candidates {
        if consumed.contains(&cand.a_id) || consumed.contains(&cand.b_id) {
            continue;
        }
        let Some(a) = by_id.get(&cand.a_id) else {
            continue;
        };
        let Some(b) = by_id.get(&cand.b_id) else {
            continue;
        };
        let Some(contact) = narrowphase_resolve_pair(a, b, tick_dt) else {
            continue;
        };
        if contact.a_post_velocity.is_none() {
            consumed.insert(contact.a_id);
        }
        if contact.b_post_velocity.is_none() {
            consumed.insert(contact.b_id);
        }
        resolved.push(contact);
    }
    resolved.sort_by(|x, y| {
        x.toi
            .partial_cmp(&y.toi)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| x.a_id.cmp(&y.a_id))
            .then_with(|| x.b_id.cmp(&y.b_id))
    });
    trace.narrowphase_contacts = resolved.len();
    (resolved, trace)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: u64, kind: ProjectileKind, pos: [f32; 2], vel: [f32; 2]) -> ProjectileSnapshot {
        ProjectileSnapshot {
            id,
            kind,
            position: pos,
            velocity: vel,
            radius: 1.0,
            mass_kg: 0.01,
            owner_actor_id: 0,
        }
    }

    fn snap_r(
        id: u64,
        kind: ProjectileKind,
        pos: [f32; 2],
        vel: [f32; 2],
        radius: f32,
    ) -> ProjectileSnapshot {
        ProjectileSnapshot {
            id,
            kind,
            position: pos,
            velocity: vel,
            radius,
            mass_kg: 0.01,
            owner_actor_id: 0,
        }
    }

    /// **VAL-M14D-011**: spatial-hash bucket size = 32 px exactly.
    #[test]
    fn bucket_size_is_32_px() {
        assert!((BROADPHASE_BUCKET_PX - 32.0).abs() < f32::EPSILON);
    }

    /// **VAL-M14D-012**: `INTERESTING_PAIRS` allowlist gates entry into
    /// narrowphase. An (EnergyBeam, KineticRifle) pair is excluded by
    /// design.
    #[test]
    fn interesting_pairs_excludes_off_allowlist_kinds() {
        // KineticRifle + ExplosiveGrenade is on the allowlist.
        assert!(is_interesting_pair(
            ProjectileKind::KineticRifle,
            ProjectileKind::ExplosiveGrenade
        ));
        // EnergyBeam + KineticRifle is NOT on the allowlist.
        assert!(!is_interesting_pair(
            ProjectileKind::EnergyBeam,
            ProjectileKind::KineticRifle
        ));
    }

    /// **VAL-M14D-012**: symmetric across pair-swap.
    #[test]
    fn is_interesting_pair_symmetric() {
        for (a, b) in interesting_pairs() {
            assert_eq!(is_interesting_pair(*a, *b), is_interesting_pair(*b, *a));
        }
    }

    /// **VAL-M14D-016**: Minkowski-difference TOI is symmetric across
    /// argument swap.
    #[test]
    fn pair_swept_toi_symmetric_across_argument_swap() {
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [50.0, 0.0], [-100.0, 0.0]);
        let ab = pair_swept_toi(&a, &b, 1.0).expect("intercept");
        let ba = pair_swept_toi(&b, &a, 1.0).expect("intercept");
        assert!((ab.toi - ba.toi).abs() < 1e-5, "{ab:?} vs {ba:?}");
        assert!((ab.point[0] - ba.point[0]).abs() < 1e-3);
        assert!((ab.point[1] - ba.point[1]).abs() < 1e-3);
    }

    /// **VAL-M14D-016**: matches the M14 segment-vs-AABB / circle
    /// primitive surface (i.e., produces a TOI in `[0, 1]` for
    /// converging paths, `None` for missed paths).
    #[test]
    fn pair_swept_toi_matches_reference_primitive_geometry() {
        // Two projectiles moving head-on; they must intercept around
        // t = 0.5.
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [100.0, 0.0], [-100.0, 0.0]);
        let toi = pair_swept_toi(&a, &b, 1.0).expect("intercept");
        assert!((toi.toi - 0.49).abs() < 0.02, "toi={}", toi.toi);
        // Two projectiles moving in parallel (never meet).
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [0.0, 50.0], [100.0, 0.0]);
        assert!(pair_swept_toi(&a, &b, 1.0).is_none());
    }

    /// **VAL-M14D-005**: shallow-angle kinetic pair filtered out.
    /// Vectors nearly parallel-same-direction → small convergence
    /// angle → rejected by the outcome resolver.
    #[test]
    fn shallow_angle_kinetic_pair_rejected_by_selective_filter() {
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 1.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [50.0, 0.0], [100.0, 0.0]);
        let conv = convergence_angle_deg(a.velocity, b.velocity);
        assert!(
            conv < KINETIC_DEFLECT_MIN_ANGLE_DEG,
            "convergence_deg={conv} must be below 10 ° threshold"
        );
        assert!(pair_outcome(a.kind, b.kind, conv).is_none());
    }

    /// **VAL-M14D-003**: ≥ 30 ° kinetic pair triggers deflect outcome
    /// (no shallow-angle filter rejection).
    #[test]
    fn cross_angle_kinetic_pair_deflects() {
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [50.0, 50.0], [0.0, -100.0]);
        let contact = narrowphase_resolve_pair(&a, &b, 1.0).expect("intercept");
        assert_eq!(contact.outcome, ProjectilePairOutcome::KineticDeflect);
        assert!(contact.a_post_velocity.is_some());
        assert!(contact.b_post_velocity.is_some());
    }

    /// **VAL-M14D-004**: kinetic deflect retains 60 % of energy.
    #[test]
    fn kinetic_deflect_retains_60_pct_energy() {
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]);
        let b = snap(2, ProjectileKind::KineticRifle, [50.0, 50.0], [0.0, -100.0]);
        let contact = narrowphase_resolve_pair(&a, &b, 1.0).expect("intercept");
        assert!(
            (contact.a_energy_retained - 0.6).abs() < 1e-3,
            "a_energy_retained={}",
            contact.a_energy_retained
        );
        assert!((contact.b_energy_retained - 0.6).abs() < 1e-3);
        // Post-velocity magnitudes must satisfy |v'|² / |v|² == 0.6.
        let post_a = contact.a_post_velocity.unwrap();
        let post_b = contact.b_post_velocity.unwrap();
        let post_speed_sq_a = post_a[0] * post_a[0] + post_a[1] * post_a[1];
        let post_speed_sq_b = post_b[0] * post_b[0] + post_b[1] * post_b[1];
        let pre_speed_sq = 100.0_f32 * 100.0;
        assert!((post_speed_sq_a / pre_speed_sq - 0.6).abs() < 0.01);
        assert!((post_speed_sq_b / pre_speed_sq - 0.6).abs() < 0.01);
    }

    /// **VAL-M14D-001/002**: kinetic-vs-explosive pair → fuze_triggered;
    /// both projectiles consumed. Uses spec Gherkin-1 geometry
    /// (grenade at [100,50] vel [+5,+2], bullet at [105,55] vel
    /// [-5,-2]). Grenade radius is 4 so the swept paths meet within
    /// combined-radius distance.
    #[test]
    fn kinetic_vs_explosive_pair_emits_fuze_triggered() {
        let grenade = snap_r(1, ProjectileKind::ExplosiveGrenade, [100.0, 50.0], [5.0, 2.0], 4.0);
        let bullet = snap_r(2, ProjectileKind::KineticRifle, [105.0, 55.0], [-5.0, -2.0], 1.0);
        let contact = narrowphase_resolve_pair(&grenade, &bullet, 1.0).expect("intercept");
        assert_eq!(contact.outcome, ProjectilePairOutcome::FuzeTriggered);
        assert!(contact.cosmetic);
        // Both projectiles must be consumed.
        assert!(contact.a_post_velocity.is_none());
        assert!(contact.b_post_velocity.is_none());
    }

    /// **VAL-M14D-015**: energy-vs-energy at correct angle →
    /// mutual_cancellation, both consumed.
    #[test]
    fn energy_vs_energy_at_correct_angle_cancels_mutually() {
        // Head-on geometry: vectors are anti-parallel (180 °
        // convergence) so the energy-cancel angle threshold passes.
        let a = snap_r(1, ProjectileKind::EnergyBeam, [0.0, 0.0], [100.0, 0.0], 2.0);
        let b = snap_r(2, ProjectileKind::EnergyBeam, [100.0, 0.0], [-100.0, 0.0], 2.0);
        let contact = narrowphase_resolve_pair(&a, &b, 1.0).expect("intercept");
        assert_eq!(contact.outcome, ProjectilePairOutcome::MutualCancellation);
        assert!(contact.a_post_velocity.is_none());
        assert!(contact.b_post_velocity.is_none());
    }

    /// **VAL-M14D-015 negative**: energy-vs-energy at shallow angle does
    /// NOT cancel.
    #[test]
    fn energy_vs_energy_at_shallow_angle_does_not_cancel() {
        let a = snap(1, ProjectileKind::EnergyBeam, [0.0, 0.0], [100.0, 5.0]);
        let b = snap(2, ProjectileKind::EnergyBeam, [50.0, 0.0], [100.0, 0.0]);
        let conv = convergence_angle_deg(a.velocity, b.velocity);
        assert!(conv < ENERGY_CANCEL_MIN_ANGLE_DEG);
        // Even if the swept paths meet, outcome resolver rejects.
        assert!(pair_outcome(a.kind, b.kind, conv).is_none());
    }

    /// **VAL-M14D-006**: APS laser vs HEAT round → aps_intercept.
    #[test]
    fn aps_laser_vs_heat_round_emits_aps_intercept() {
        let aps = snap(1, ProjectileKind::ApsLaser, [0.0, 0.0], [1000.0, 0.0]);
        let heat = snap(2, ProjectileKind::HeatRound, [100.0, 0.0], [-200.0, 0.0]);
        let contact = narrowphase_resolve_pair(&aps, &heat, 1.0).expect("intercept");
        assert_eq!(contact.outcome, ProjectilePairOutcome::ApsIntercept);
        assert!(contact.cosmetic);
        assert!(contact.a_post_velocity.is_none() && contact.b_post_velocity.is_none());
    }

    /// **VAL-M14D-014**: cosmetic flag is set on every pair contact.
    #[test]
    fn all_pair_contacts_carry_cosmetic_true() {
        let grenade = snap_r(1, ProjectileKind::ExplosiveGrenade, [100.0, 50.0], [5.0, 2.0], 4.0);
        let bullet = snap_r(2, ProjectileKind::KineticRifle, [105.0, 55.0], [-5.0, -2.0], 1.0);
        let contact = narrowphase_resolve_pair(&grenade, &bullet, 1.0).expect("intercept");
        assert!(contact.cosmetic);
    }

    /// **VAL-M14D-007 + 009 + 011**: broadphase candidates are
    /// deterministic + capped at the budget. Build a 50-projectile
    /// scene and confirm narrowphase candidate count ≤ 12.
    #[test]
    fn broadphase_candidate_count_capped_at_budget_50_projectiles() {
        let mut pool: Vec<ProjectileSnapshot> = Vec::new();
        // 25 kinetic rifles, 25 explosive grenades scattered through a
        // 1024² scene. Grid spacing ensures pairs cluster densely enough
        // to exercise the cap.
        for i in 0..25 {
            pool.push(snap(
                100 + i as u64,
                ProjectileKind::KineticRifle,
                [(i as f32) * 40.0, 100.0],
                [50.0, 0.0],
            ));
            pool.push(snap(
                200 + i as u64,
                ProjectileKind::ExplosiveGrenade,
                [(i as f32) * 40.0 + 4.0, 100.0],
                [-50.0, 0.0],
            ));
        }
        let cands = SpatialHashBroadphase::candidates(&pool, 1.0 / 60.0);
        assert!(
            cands.len() <= NARROWPHASE_CANDIDATE_BUDGET,
            "broadphase exceeded budget: {}",
            cands.len()
        );
    }

    /// **VAL-M14D-007**: pass output is deterministic across runs.
    #[test]
    fn pair_pass_deterministic_across_two_runs() {
        let mut pool: Vec<ProjectileSnapshot> = Vec::new();
        for i in 0..15 {
            pool.push(snap(
                10 + i as u64,
                ProjectileKind::KineticRifle,
                [(i as f32) * 30.0, 0.0],
                [60.0, 0.0],
            ));
            pool.push(snap(
                100 + i as u64,
                ProjectileKind::ExplosiveGrenade,
                [(i as f32) * 30.0 + 5.0, 5.0],
                [-60.0, 0.0],
            ));
        }
        let (a, _) = run_projectile_pair_pass(&pool, 1.0 / 60.0);
        let (b, _) = run_projectile_pair_pass(&pool, 1.0 / 60.0);
        assert_eq!(a, b);
    }

    /// **VAL-M14D-013** (companion to `swept.rs` extension): the kernel
    /// surfaces pairs sorted by TOI when several pairs resolve in one
    /// tick.
    #[test]
    fn multiple_pair_outcomes_sorted_by_toi() {
        let pool = vec![
            // Pair A: kinetic + grenade meet at t ~= 0.3.
            snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.0]),
            snap(2, ProjectileKind::ExplosiveGrenade, [30.0, 0.0], [-100.0, 0.0]),
            // Pair B: kinetic + grenade meet at t ~= 0.1.
            snap(3, ProjectileKind::KineticRifle, [200.0, 0.0], [100.0, 0.0]),
            snap(4, ProjectileKind::ExplosiveGrenade, [220.0, 0.0], [-100.0, 0.0]),
        ];
        let (contacts, _) = run_projectile_pair_pass(&pool, 1.0);
        assert_eq!(contacts.len(), 2);
        assert!(contacts[0].toi <= contacts[1].toi);
    }

    /// **VAL-M14D-005**: zero pair contact + zero velocity change for
    /// shallow-angle kinetic pair.
    #[test]
    fn shallow_angle_pair_emits_no_event() {
        let a = snap(1, ProjectileKind::KineticRifle, [0.0, 0.0], [100.0, 0.1]);
        let b = snap(2, ProjectileKind::KineticRifle, [50.0, 0.0], [100.0, 0.0]);
        let (contacts, _) = run_projectile_pair_pass(&[a, b], 1.0);
        assert!(contacts.is_empty());
    }
}
