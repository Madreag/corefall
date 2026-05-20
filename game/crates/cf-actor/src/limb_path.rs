//! **M14A** § "LimbPath data type — Cortex Command's foot-trajectory primitive".
//!
//! Mirrors CCCP `Entities/LimbPath.h:1-739` *behaviorally*. The data shape is
//! original Rust — a list of waypoints + per-segment timing + speed tier +
//! push-force-escalation timer. The critical algorithm we replicate is
//! `LimbPath::GetPushForce()` (CCCP inline definition): effective push force
//! doubles every 500 ms a foot is stuck on a single segment.
//!
//! Determinism: all timer state lives on the path; no clock reads, no RNG.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::move_state::MoveState;

/// CCCP `LimbPath::Speed` enum (`SLOW=0`, `NORMAL=1`, `FAST=2`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimbPathSpeed {
    Slow = 0,
    Normal = 1,
    Fast = 2,
}

impl Default for LimbPathSpeed {
    fn default() -> Self {
        LimbPathSpeed::Normal
    }
}

impl LimbPathSpeed {
    pub fn as_str(self) -> &'static str {
        match self {
            LimbPathSpeed::Slow => "slow",
            LimbPathSpeed::Normal => "normal",
            LimbPathSpeed::Fast => "fast",
        }
    }
}

/// **M14A** § "LimbPath data type".
///
/// One foot's stride trajectory: a list of waypoints (relative to the owning
/// AtomGroup's local origin), per-tier travel speeds, and a push-force
/// escalation timer used by CCCP's `PushAsLimb` body-response physics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimbPath {
    /// First waypoint (CCCP `m_Start`).
    pub start: [f32; 2],
    /// Subsequent waypoints (CCCP `m_Segments`).
    pub segments: Vec<[f32; 2]>,
    /// Per-tier travel speed (slow/normal/fast). CCCP `m_TravelSpeed[3]`.
    pub travel_speed: [f32; 3],
    /// Multiplier applied to the chosen tier speed. Used by crouch + heavy
    /// mass to scale walk speed without touching the per-tier array.
    pub travel_speed_multiplier: f32,
    /// Base push force (N) the foot pushes the chassis with on each tick.
    /// CCCP `m_PushForce`.
    pub push_force: f32,
    /// Per-segment index where foot-vs-foot collision is disabled (CCCP
    /// `m_FootCollisionsDisabledSegment`). `-1` = always enabled.
    pub foot_collisions_disabled_segment: i32,
    /// Horizontal flip flag (mirrors waypoints across X). CCCP `m_HFlipped`.
    pub h_flipped: bool,
    /// Active speed tier.
    pub speed: LimbPathSpeed,
    /// Currently-active segment index (0 = `start → segments[0]`).
    pub current_segment: u32,
    /// Per-tick progress along the current segment in [0, 1].
    pub seg_progress: f32,
    /// Milliseconds the foot has been stuck on this segment. Drives
    /// [`LimbPath::effective_push_force`].
    pub seg_timer_ms: u32,
    /// `true` after the last segment completed (foot reached its end).
    pub path_ended: bool,
    /// `true` when no progress has been made on this stride yet.
    pub path_at_start: bool,
}

impl Default for LimbPath {
    fn default() -> Self {
        Self {
            start: [0.0, 0.0],
            segments: Vec::new(),
            travel_speed: [0.6, 1.0, 1.5],
            travel_speed_multiplier: 1.0,
            push_force: 80.0,
            foot_collisions_disabled_segment: -1,
            h_flipped: false,
            speed: LimbPathSpeed::Normal,
            current_segment: 0,
            seg_progress: 0.0,
            seg_timer_ms: 0,
            path_ended: false,
            path_at_start: true,
        }
    }
}

impl LimbPath {
    /// Build an `n`-segment path from a list of waypoints. The first waypoint
    /// becomes `start`; the rest become segment endpoints.
    pub fn from_waypoints(waypoints: &[[f32; 2]]) -> Self {
        let mut p = Self::default();
        if let Some((first, rest)) = waypoints.split_first() {
            p.start = *first;
            p.segments = rest.to_vec();
        }
        p
    }

    /// CCCP `LimbPath::GetPushForce()` — effective push force doubles every
    /// 500 ms a foot is stuck on a single segment.
    pub fn effective_push_force(&self) -> f32 {
        self.push_force * (1.0 + self.seg_timer_ms as f32 / 500.0)
    }

    /// CCCP `LimbPath::GetSpeed()` — the per-tier speed × multiplier.
    pub fn effective_speed(&self) -> f32 {
        let base = match self.speed {
            LimbPathSpeed::Slow => self.travel_speed[0],
            LimbPathSpeed::Normal => self.travel_speed[1],
            LimbPathSpeed::Fast => self.travel_speed[2],
        };
        (base * self.travel_speed_multiplier).max(0.0)
    }

    /// CCCP `LimbPath::IsAtStart()` — true at stride start (no progress yet).
    pub fn at_start(&self) -> bool {
        self.path_at_start
    }

    /// CCCP `LimbPath::Ended()` — true after the last segment completed.
    pub fn ended(&self) -> bool {
        self.path_ended
    }

    /// CCCP `LimbPath::GetRegularProgress()` — fraction in [0, 1] across the
    /// whole stride (current_segment + seg_progress) / total_segments.
    pub fn regular_progress(&self) -> f32 {
        let total = (self.segments.len() as f32).max(1.0);
        let walked = self.current_segment as f32 + self.seg_progress;
        (walked / total).clamp(0.0, 1.0)
    }

    /// CCCP `LimbPath::ReportProgress` — push the segment timer forward by
    /// `dt_ms` then advance segment when at end. Pure mutation; deterministic.
    pub fn report_progress(&mut self, fraction_delta: f32, dt_ms: u32) {
        self.path_at_start = false;
        self.seg_timer_ms = self.seg_timer_ms.saturating_add(dt_ms);
        self.seg_progress = (self.seg_progress + fraction_delta).clamp(0.0, 2.0);
        while self.seg_progress >= 1.0 {
            self.seg_progress -= 1.0;
            self.current_segment += 1;
            self.seg_timer_ms = 0;
            if self.current_segment as usize >= self.segments.len() {
                self.path_ended = true;
                self.seg_progress = 0.0;
                break;
            }
        }
    }

    /// CCCP `LimbPath::RestartFree` — reset to start of path, clear timers.
    /// Returns `true` if restart is permitted (always true for free reset).
    pub fn restart_free(&mut self) -> bool {
        self.current_segment = 0;
        self.seg_progress = 0.0;
        self.seg_timer_ms = 0;
        self.path_ended = false;
        self.path_at_start = true;
        true
    }

    /// CCCP `LimbPath::Terminate` — mark the path complete without advancing
    /// further. Used when the foot has strayed off path beyond chassis radius.
    pub fn terminate(&mut self) {
        self.path_ended = true;
        self.seg_progress = 0.0;
        self.seg_timer_ms = 0;
    }

    /// CCCP `LimbPath::OverrideSpeed` — push the per-tier speeds wholesale.
    pub fn override_speed(&mut self, speeds: [f32; 3]) {
        self.travel_speed = speeds;
    }

    /// CCCP `LimbPath::SetSpeed` — set the active speed tier.
    pub fn set_speed(&mut self, speed: LimbPathSpeed) {
        self.speed = speed;
    }

    /// Compute the current segment endpoint in path-local coords.
    pub fn current_endpoint(&self) -> [f32; 2] {
        if self.segments.is_empty() {
            return self.start;
        }
        let idx = (self.current_segment as usize).min(self.segments.len() - 1);
        self.segments[idx]
    }
}

/// **M14A** § "Per-stance limb-path registry" — one path per
/// `(MoveState, side)`. Side = `fg` / `bg` matches CCCP's two-leg model;
/// quadrupeds use FL/FR/RL/RR by routing through the FG/BG slots in a
/// 2-pair gait.
///
/// **M14J** extends the registry with per-stroke swim paths (keyed by
/// [`crate::move_state::SwimKind`]) + parkour-specific vault + wall_jump
/// paths. Existing serde bundles forward-compat via `#[serde(default)]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LimbPathRegistry {
    /// Foreground (right-side) per-state paths.
    pub fg: BTreeMap<MoveState, LimbPath>,
    /// Background (left-side) per-state paths.
    pub bg: BTreeMap<MoveState, LimbPath>,
    /// **M14J** § per-stroke swim limb paths (one per `SwimKind` variant).
    /// Defaults to empty for legacy bundles.
    #[serde(default)]
    pub swim: BTreeMap<crate::move_state::SwimKind, LimbPath>,
    /// **M14J** § parkour vault cinematic path (used during the 200 ms
    /// `Stance::Vault` transition).
    #[serde(default)]
    pub parkour_vault: Option<LimbPath>,
    /// **M14J** § parkour wall-jump cinematic path.
    #[serde(default)]
    pub parkour_wall_jump: Option<LimbPath>,
}

impl LimbPathRegistry {
    /// Insert a path for the given (state, side). `side="fg"` or `"bg"`.
    pub fn insert(&mut self, state: MoveState, side: PathSide, path: LimbPath) {
        match side {
            PathSide::Fg => {
                self.fg.insert(state, path);
            }
            PathSide::Bg => {
                self.bg.insert(state, path);
            }
        }
    }

    pub fn get(&self, state: MoveState, side: PathSide) -> Option<&LimbPath> {
        match side {
            PathSide::Fg => self.fg.get(&state),
            PathSide::Bg => self.bg.get(&state),
        }
    }

    pub fn get_mut(&mut self, state: MoveState, side: PathSide) -> Option<&mut LimbPath> {
        match side {
            PathSide::Fg => self.fg.get_mut(&state),
            PathSide::Bg => self.bg.get_mut(&state),
        }
    }

    /// **M14J** § insert or replace a swim limb path keyed by `SwimKind`.
    pub fn insert_swim(&mut self, kind: crate::move_state::SwimKind, path: LimbPath) {
        self.swim.insert(kind, path);
    }

    /// **M14J** § fetch a swim limb path by `SwimKind` (immutable).
    pub fn get_swim(&self, kind: crate::move_state::SwimKind) -> Option<&LimbPath> {
        self.swim.get(&kind)
    }

    /// **M14J** § fetch a swim limb path by `SwimKind` (mutable).
    pub fn get_swim_mut(&mut self, kind: crate::move_state::SwimKind) -> Option<&mut LimbPath> {
        self.swim.get_mut(&kind)
    }
}

/// Which side of the chassis a path belongs to.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathSide {
    Fg = 0,
    Bg = 1,
}

impl PathSide {
    pub fn flip(self) -> Self {
        match self {
            PathSide::Fg => PathSide::Bg,
            PathSide::Bg => PathSide::Fg,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PathSide::Fg => "fg",
            PathSide::Bg => "bg",
        }
    }
}

/// Default infantry walk path (FG side). Mirrors CCCP infantry stride: ~6
/// segments, push_force 80 N, normal speed 1.0 px/ms. Used as a fallback
/// when a chassis's RON limb_paths file is missing the WALK entry.
pub fn default_infantry_walk_fg() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-2.0, 16.0],
        [4.0, -2.0],
        [4.0, 0.0],
        [4.0, 2.0],
        [-4.0, 2.0],
        [-4.0, 0.0],
        [-4.0, -2.0],
    ]);
    p.push_force = 80.0;
    p.travel_speed = [0.6, 1.0, 1.5];
    p
}

/// Default infantry walk path (BG side). Phase-shifted from FG so the feet
/// alternate.
pub fn default_infantry_walk_bg() -> LimbPath {
    let mut p = default_infantry_walk_fg();
    // Phase-shift: BG starts halfway through the FG cycle.
    p.current_segment = (p.segments.len() / 2) as u32;
    p
}

/// Default stand path — tiny push toward chassis center to keep feet planted.
pub fn default_infantry_stand() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[[0.0, 16.0], [0.0, 16.5], [0.0, 16.0]]);
    p.push_force = 40.0;
    p.travel_speed = [0.2, 0.4, 0.6];
    p
}

/// Default crouch path — shortened stride, deeper plant.
pub fn default_infantry_crouch() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-1.0, 12.0],
        [2.0, -1.0],
        [2.0, 1.0],
        [-2.0, 1.0],
        [-2.0, -1.0],
    ]);
    p.push_force = 70.0;
    p.travel_speed = [0.3, 0.5, 0.75];
    p
}

/// Default crawl path — short crawl strokes.
pub fn default_infantry_crawl() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-2.0, 8.0],
        [3.0, 0.0],
        [-3.0, 0.0],
    ]);
    p.push_force = 60.0;
    p.travel_speed = [0.2, 0.4, 0.6];
    p
}

/// Default arm-crawl path — both arms drag the body when legs are gone.
pub fn default_infantry_arm_crawl() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-3.0, 4.0],
        [4.0, 0.0],
        [-4.0, 0.0],
    ]);
    p.push_force = 50.0;
    p.travel_speed = [0.15, 0.25, 0.4];
    p
}

/// Default climb path.
pub fn default_infantry_climb() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-2.0, 12.0],
        [-2.0, 0.0],
        [-2.0, -8.0],
        [2.0, -8.0],
        [2.0, 0.0],
        [2.0, 12.0],
    ]);
    p.push_force = 90.0;
    p.travel_speed = [0.3, 0.5, 0.75];
    p
}

/// Default jump path — short launch push from planted foot.
pub fn default_infantry_jump() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[[0.0, 16.0], [0.0, 14.0], [0.0, 12.0]]);
    p.push_force = 120.0;
    p.travel_speed = [0.4, 0.8, 1.2];
    p
}

/// Default dislodge path — used when foot is stuck.
pub fn default_infantry_dislodge() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[[0.0, 14.0], [0.0, 10.0], [0.0, 14.0]]);
    p.push_force = 150.0;
    p.travel_speed = [0.5, 1.0, 1.5];
    p
}

/// **M14J** § "vault.path — 200 ms `Vault` stance whose limb path lifts the
/// body over the obstacle". Mirrors `game/content/actors/paths/vault.path`.
pub fn default_infantry_vault() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-4.0, 16.0],
        [0.0, -8.0],
        [4.0, -16.0],
        [8.0, -8.0],
        [8.0, 4.0],
        [4.0, 16.0],
    ]);
    p.push_force = 150.0;
    p.travel_speed = [1.0, 1.5, 2.0];
    p.foot_collisions_disabled_segment = 0;
    p
}

/// **M14J** § "wall_jump.path — perpendicular kick off a vertical surface".
pub fn default_infantry_wall_jump() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [4.0, 0.0],
        [2.0, -4.0],
        [-2.0, -8.0],
        [-4.0, -10.0],
    ]);
    p.push_force = 180.0;
    p.travel_speed = [1.0, 1.5, 2.0];
    p
}

/// **M14J** § "swim_breast.path (4-stroke cycle) — surface breast stroke".
pub fn default_infantry_swim_breast() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-4.0, 6.0],
        [-2.0, 4.0],
        [2.0, 4.0],
        [4.0, 6.0],
        [0.0, 8.0],
    ]);
    p.push_force = 60.0;
    p.travel_speed = [0.4, 0.7, 1.0];
    p
}

/// **M14J** § "swim_freestyle.path — surface horizontal burst".
pub fn default_infantry_swim_freestyle() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [-3.0, 6.0],
        [3.0, 5.0],
        [-3.0, 5.0],
        [3.0, 6.0],
    ]);
    p.push_force = 80.0;
    p.travel_speed = [0.5, 0.9, 1.4];
    p
}

/// **M14J** § "swim_dive.path — vertical-down submerged dive".
pub fn default_infantry_swim_dive() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [0.0, 4.0],
        [-1.0, 0.0],
        [1.0, -4.0],
        [-1.0, -8.0],
    ]);
    p.push_force = 70.0;
    p.travel_speed = [0.3, 0.6, 1.0];
    p
}

/// **M14J** § "swim_tread.path — idle keep-head-above stroke".
pub fn default_infantry_swim_tread() -> LimbPath {
    let mut p = LimbPath::from_waypoints(&[
        [0.0, 4.0],
        [-2.0, 4.0],
        [0.0, 4.0],
        [2.0, 4.0],
        [0.0, 4.0],
    ]);
    p.push_force = 40.0;
    p.travel_speed = [0.2, 0.3, 0.4];
    p
}

/// **M14A** § "Per-actor limb-path registry — RON-loadable" — load a single
/// limb path from a RON string.
pub fn load_path_from_ron(ron_str: &str) -> Result<LimbPathSpec, String> {
    ron::from_str::<LimbPathSpec>(ron_str).map_err(|e| format!("limb_path RON parse failed: {e}"))
}

/// Spec-locked shape that mirrors the on-disk RON.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LimbPathSpec {
    pub schema_version: u32,
    pub chassis_archetype: String,
    pub move_state: String,
    pub side: String,
    pub start: (f32, f32),
    pub segments: Vec<(f32, f32)>,
    pub travel_speed: Vec<f32>,
    pub travel_speed_multiplier: f32,
    pub push_force: f32,
    pub foot_collisions_disabled_segment: i32,
}

impl LimbPathSpec {
    pub fn to_limb_path(&self) -> LimbPath {
        let speeds = if self.travel_speed.len() == 3 {
            [self.travel_speed[0], self.travel_speed[1], self.travel_speed[2]]
        } else {
            [0.6, 1.0, 1.5]
        };
        LimbPath {
            start: [self.start.0, self.start.1],
            segments: self.segments.iter().map(|(x, y)| [*x, *y]).collect(),
            travel_speed: speeds,
            travel_speed_multiplier: self.travel_speed_multiplier,
            push_force: self.push_force,
            foot_collisions_disabled_segment: self.foot_collisions_disabled_segment,
            ..LimbPath::default()
        }
    }
}

/// Default infantry limb-path registry covering every MoveState. Used by
/// `ActorState` when no chassis-specific RON file is loaded.
///
/// **M14J** extends the default registry with vault + wall_jump + 4 swim
/// paths so M14J cinematics can dispatch off the actor's owned registry
/// without external content lookups.
pub fn default_infantry_registry() -> LimbPathRegistry {
    let mut reg = LimbPathRegistry::default();
    reg.insert(MoveState::Stand, PathSide::Fg, default_infantry_stand());
    reg.insert(MoveState::Stand, PathSide::Bg, default_infantry_stand());
    reg.insert(MoveState::Walk, PathSide::Fg, default_infantry_walk_fg());
    reg.insert(MoveState::Walk, PathSide::Bg, default_infantry_walk_bg());
    reg.insert(MoveState::Crouch, PathSide::Fg, default_infantry_crouch());
    reg.insert(MoveState::Crouch, PathSide::Bg, default_infantry_crouch());
    reg.insert(MoveState::Crawl, PathSide::Fg, default_infantry_crawl());
    reg.insert(MoveState::Crawl, PathSide::Bg, default_infantry_crawl());
    reg.insert(MoveState::ArmCrawl, PathSide::Fg, default_infantry_arm_crawl());
    reg.insert(MoveState::ArmCrawl, PathSide::Bg, default_infantry_arm_crawl());
    reg.insert(MoveState::Climb, PathSide::Fg, default_infantry_climb());
    reg.insert(MoveState::Climb, PathSide::Bg, default_infantry_climb());
    reg.insert(MoveState::Jump, PathSide::Fg, default_infantry_jump());
    reg.insert(MoveState::Dislodge, PathSide::Fg, default_infantry_dislodge());
    // **M14J** parkour + swim defaults.
    reg.parkour_vault = Some(default_infantry_vault());
    reg.parkour_wall_jump = Some(default_infantry_wall_jump());
    reg.insert_swim(crate::move_state::SwimKind::SurfaceBreast, default_infantry_swim_breast());
    reg.insert_swim(crate::move_state::SwimKind::SurfaceFreestyle, default_infantry_swim_freestyle());
    reg.insert_swim(crate::move_state::SwimKind::Dive, default_infantry_swim_dive());
    reg.insert_swim(crate::move_state::SwimKind::Tread, default_infantry_swim_tread());
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_push_force_doubles_in_500ms() {
        let mut p = LimbPath::default();
        p.push_force = 80.0;
        assert!((p.effective_push_force() - 80.0).abs() < 1e-6);
        p.seg_timer_ms = 500;
        assert!((p.effective_push_force() - 160.0).abs() < 1e-6);
        p.seg_timer_ms = 1000;
        assert!((p.effective_push_force() - 240.0).abs() < 1e-6);
    }

    #[test]
    fn from_waypoints_splits_start_and_segments() {
        let p = LimbPath::from_waypoints(&[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);
        assert_eq!(p.start, [1.0, 2.0]);
        assert_eq!(p.segments, vec![[3.0, 4.0], [5.0, 6.0]]);
    }

    #[test]
    fn report_progress_advances_segments_and_resets_timer() {
        let mut p = LimbPath::from_waypoints(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]);
        p.report_progress(0.5, 100);
        assert_eq!(p.current_segment, 0);
        assert!((p.seg_progress - 0.5).abs() < 1e-6);
        assert_eq!(p.seg_timer_ms, 100);
        // Full segment crossed → advances to next, timer resets.
        p.report_progress(0.6, 100);
        assert_eq!(p.current_segment, 1);
        assert_eq!(p.seg_timer_ms, 0);
        // Walking past the last segment terminates the path.
        p.report_progress(1.5, 50);
        assert!(p.path_ended);
    }

    #[test]
    fn restart_free_resets_everything() {
        let mut p = LimbPath::from_waypoints(&[[0.0, 0.0], [1.0, 0.0]]);
        p.current_segment = 1;
        p.path_ended = true;
        assert!(p.restart_free());
        assert_eq!(p.current_segment, 0);
        assert!(!p.path_ended);
        assert!(p.path_at_start);
    }

    #[test]
    fn registry_round_trip() {
        let mut reg = LimbPathRegistry::default();
        reg.insert(MoveState::Walk, PathSide::Fg, default_infantry_walk_fg());
        assert!(reg.get(MoveState::Walk, PathSide::Fg).is_some());
        assert!(reg.get(MoveState::Walk, PathSide::Bg).is_none());
    }

    #[test]
    fn default_registry_covers_every_state() {
        let reg = default_infantry_registry();
        for s in [
            MoveState::Stand,
            MoveState::Walk,
            MoveState::Crouch,
            MoveState::Crawl,
            MoveState::ArmCrawl,
            MoveState::Climb,
            MoveState::Jump,
            MoveState::Dislodge,
        ] {
            assert!(reg.get(s, PathSide::Fg).is_some(), "missing FG path for {:?}", s);
        }
    }
}
