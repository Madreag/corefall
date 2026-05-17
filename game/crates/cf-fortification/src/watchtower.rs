//! M9C § "Watchtower (3 height tiers) + spotlight + observation post":
//! watchtower-tier ladder, M14F lateral-collapse on destruction,
//! spotlight cone-of-light + flashbang-dazzle, observation post SPG
//! acquisition bonus, radio repeater range extension.
//!
//! Spec § "Watchtower (3 height tiers) + spotlight + observation post":
//!
//! | Asset | HP | Provides | Build time |
//! |---|---|---|---|
//! | `watchtower_t1` | 600 | +1 LOS tile; cover at top platform | 90s |
//! | `watchtower_t2` | 1200 | +3 LOS tiles; rooftop mount slot | 150s |
//! | `watchtower_t3` | 2400 | +6 LOS tiles; integrated radio repeater | 240s |
//! | `spotlight` | 100 | Cone-of-light 24-tile range; reveals concealed actors | 12s |
//! | `observation_post` | 400 | +75% acquisition bonus to faction-wide artillery | 60s |
//! | `radio_repeater` | 200 | Extends squad-radio range by 100 tiles | 20s |
//!
//! Per spec § "Notes for the implementer":
//!
//! > Watchtower destruction uses M14F lateral wall collapse — falling
//! > debris causes M14 fall_impulse damage to anything in the radius.
//! > T3 tower at 48 px is the biggest collapse in M9C; use the same
//! > animation as M14F.
//!
//! Per spec § Gherkin "Watchtower destruction triggers lateral collapse":
//!
//! > Given a built `watchtower_t3` (HP 2400, 48 px tall)
//! > When the player fires HEAT rounds (M14C) at the tower base until HP=0
//! > Then watchtower_destroyed event fires
//! > And M14F lateral wall collapse triggers a 5-tile-radius debris drop
//! > And actors within the radius take fall_impulse damage
//! > And the radio_repeater module is destroyed
//! > And faction squad-radio range drops by 100 tiles immediately
//!
//! VAL-M9C-019 / VAL-M9C-020 / VAL-M9C-021 / VAL-M9C-022 / VAL-M9C-023
//! / VAL-M9C-024 land here. AI-OBS-A-01 doctrine lives in cf-ai but
//! consumes the [`SpotterMarkInputs`] / [`spotter_acquisition_multiplier`]
//! surfaces declared here.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// Spec table row HP per watchtower tier.
pub const WATCHTOWER_T1_MAX_HP: u32 = 600;
pub const WATCHTOWER_T2_MAX_HP: u32 = 1200;
pub const WATCHTOWER_T3_MAX_HP: u32 = 2400;
/// Spec table row HP for a `spotlight` mounted on a watchtower.
pub const SPOTLIGHT_MAX_HP: u32 = 100;
/// Spec table row HP for an `observation_post`.
pub const OBSERVATION_POST_MAX_HP: u32 = 400;
/// Spec table row HP for a `radio_repeater`.
pub const RADIO_REPEATER_MAX_HP: u32 = 200;

/// Spec § "Notes for the implementer": T3 tower at 48 px = the
/// biggest collapse in M9C; 5-tile radius lateral drop.
pub const WATCHTOWER_T3_COLLAPSE_RADIUS_TILES: u32 = 5;
/// T2 lateral collapse radius — 3 tiles (32 px / ~8 px per tile floor).
pub const WATCHTOWER_T2_COLLAPSE_RADIUS_TILES: u32 = 3;
/// T1 lateral collapse radius — 2 tiles (16 px tower → minimal drop).
pub const WATCHTOWER_T1_COLLAPSE_RADIUS_TILES: u32 = 2;

/// Per-tier baseline fall-impulse damage applied to an actor at the
/// collapse epicenter (distance 0). Damage scales linearly with
/// distance until the radius edge where damage hits 0. T3 = 48 px
/// debris drop → 480 base damage; T2 = 32 px → 320; T1 = 16 px → 160.
pub const WATCHTOWER_T1_BASE_FALL_DAMAGE: u32 = 160;
pub const WATCHTOWER_T2_BASE_FALL_DAMAGE: u32 = 320;
pub const WATCHTOWER_T3_BASE_FALL_DAMAGE: u32 = 480;

/// Spec table row: 24-tile range of the spotlight cone-of-light.
pub const SPOTLIGHT_CONE_RANGE_TILES: u32 = 24;
/// Spec § "Spotlight": 1 kW grid power draw (continuous).
pub const SPOTLIGHT_POWER_DRAW_KW: u32 = 1;
/// Spec § "Spotlight": flashbang dazzle keeps the spotlight offline
/// for exactly 12 seconds.
pub const SPOTLIGHT_DAZZLE_SECONDS: u32 = 12;
/// Spec § "Spotlight cone-of-light": cone half-angle in degrees. The
/// spec table doesn't pin the angle so we use a generous 30° (60° full
/// cone) per typical floodlight optics.
pub const SPOTLIGHT_HALF_ANGLE_DEGREES: f32 = 30.0;

/// Spec § "Observation post" table row: +75% acquisition multiplier
/// to faction-wide artillery for targets the observer has LOS to.
pub const OBSERVATION_POST_ARTILLERY_ACQUISITION_MULTIPLIER: f32 = 1.75;

/// Spec § "Radio repeater": +100 tile range extension to the owning
/// faction's squad radio. Loss of the repeater drops the bonus by 100
/// immediately.
pub const RADIO_REPEATER_RANGE_BONUS_TILES: u32 = 100;

/// One of the three watchtower height tiers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchtowerTier {
    T1 = 1,
    T2 = 2,
    T3 = 3,
}

impl WatchtowerTier {
    pub const ALL: [WatchtowerTier; 3] = [
        WatchtowerTier::T1,
        WatchtowerTier::T2,
        WatchtowerTier::T3,
    ];

    #[must_use]
    pub const fn max_hp(self) -> u32 {
        match self {
            WatchtowerTier::T1 => WATCHTOWER_T1_MAX_HP,
            WatchtowerTier::T2 => WATCHTOWER_T2_MAX_HP,
            WatchtowerTier::T3 => WATCHTOWER_T3_MAX_HP,
        }
    }

    /// Lateral-collapse radius in tiles when this tier is destroyed.
    #[must_use]
    pub const fn collapse_radius_tiles(self) -> u32 {
        match self {
            WatchtowerTier::T1 => WATCHTOWER_T1_COLLAPSE_RADIUS_TILES,
            WatchtowerTier::T2 => WATCHTOWER_T2_COLLAPSE_RADIUS_TILES,
            WatchtowerTier::T3 => WATCHTOWER_T3_COLLAPSE_RADIUS_TILES,
        }
    }

    /// Base fall-impulse damage applied to an actor at the collapse
    /// epicenter. Damage decays linearly with distance until 0 at the
    /// radius edge.
    #[must_use]
    pub const fn base_fall_damage(self) -> u32 {
        match self {
            WatchtowerTier::T1 => WATCHTOWER_T1_BASE_FALL_DAMAGE,
            WatchtowerTier::T2 => WATCHTOWER_T2_BASE_FALL_DAMAGE,
            WatchtowerTier::T3 => WATCHTOWER_T3_BASE_FALL_DAMAGE,
        }
    }

    /// Stable string id used on `watchtower_destroyed` replay events.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WatchtowerTier::T1 => "t1",
            WatchtowerTier::T2 => "t2",
            WatchtowerTier::T3 => "t3",
        }
    }

    /// True when this tier ships with an integrated radio repeater
    /// (spec table: only T3 has the integrated module).
    #[must_use]
    pub const fn has_integrated_repeater(self) -> bool {
        matches!(self, WatchtowerTier::T3)
    }
}

/// Placed watchtower instance — owns hp, tier, tile-position, and
/// optionally an embedded radio repeater. Spec table: only T3 ships
/// with an integrated repeater.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchtower {
    pub id: FortificationId,
    pub tier: WatchtowerTier,
    pub hp: u32,
    pub pos_tiles: (i32, i32),
    /// Embedded radio repeater for T3 (None for T1/T2).
    pub integrated_repeater: Option<FortificationId>,
}

impl Watchtower {
    /// Construct a freshly built watchtower at the supplied tier; HP
    /// is pinned to the tier max and T3 gets an integrated repeater
    /// id from the supplied factory closure (None for T1/T2).
    #[must_use]
    pub fn new_built(
        id: FortificationId,
        tier: WatchtowerTier,
        pos_tiles: (i32, i32),
        integrated_repeater: Option<FortificationId>,
    ) -> Self {
        let actual_repeater = if tier.has_integrated_repeater() {
            integrated_repeater
        } else {
            None
        };
        Self {
            id,
            tier,
            hp: tier.max_hp(),
            pos_tiles,
            integrated_repeater: actual_repeater,
        }
    }

    /// True when the tower has been destroyed (HP 0).
    #[must_use]
    pub const fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    /// Apply `damage` HP to the tower; returns the destruction outcome
    /// when the damage drives HP to 0. The outcome carries the
    /// watchtower-destroyed event payload + the per-actor fall-impulse
    /// damage events. Callers (cf-control) thread the actor list +
    /// engine state in via [`apply_destruction_collapse`].
    pub fn apply_damage(&mut self, damage: u32) -> Option<WatchtowerDestructionPending> {
        if self.is_destroyed() {
            return None;
        }
        self.hp = self.hp.saturating_sub(damage);
        if self.hp == 0 {
            return Some(WatchtowerDestructionPending {
                tower_id: self.id,
                tier: self.tier,
                pos_tiles: self.pos_tiles,
                collapse_radius_tiles: self.tier.collapse_radius_tiles(),
                integrated_repeater: self.integrated_repeater.take(),
            });
        }
        None
    }
}

/// Pending destruction outcome — caller (cf-control) consumes this to
/// emit the `watchtower_destroyed` event, generate fall-impulse damage
/// events against actors in radius, and tear down any embedded
/// repeater. Spec § "Notes for the implementer" describes the engine
/// thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchtowerDestructionPending {
    pub tower_id: FortificationId,
    pub tier: WatchtowerTier,
    pub pos_tiles: (i32, i32),
    pub collapse_radius_tiles: u32,
    pub integrated_repeater: Option<FortificationId>,
}

/// Replay-event-payload shape for `fortification.watchtower_destroyed`.
/// The recorder converts this into the JSON envelope declared in
/// `watchtower_destroyed.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchtowerDestroyedEvent {
    pub tower_id: FortificationId,
    pub tier: WatchtowerTier,
    pub collapse_radius_tiles: u32,
    pub tick_index: u64,
}

/// One per-actor fall-impulse damage event emitted by M14F lateral
/// collapse. cf-control routes each through the M14 damage pipeline
/// (the cf-physics::fall_impulse_chain consumer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallImpulseDamageEvent {
    pub actor_id: u64,
    pub tower_id: FortificationId,
    pub distance_tiles: u32,
    pub impulse_damage: u32,
}

/// One observed actor inside the post-destruction lateral-collapse
/// radius. The engine builds these from its actor index just before
/// calling [`apply_destruction_collapse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorInCollapseRadius {
    pub actor_id: u64,
    pub pos_tiles: (i32, i32),
}

/// Aggregated destruction outcome — the per-tick payload the engine
/// emits to the replay log + applies to the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchtowerDestructionOutcome {
    pub destroyed: WatchtowerDestroyedEvent,
    pub fall_impulse_events: Vec<FallImpulseDamageEvent>,
    pub destroyed_repeater: Option<FortificationId>,
}

/// Chebyshev (king-move) tile distance helper. Spec § "Notes for the
/// implementer" doesn't pin a distance metric; we use Chebyshev so the
/// collapse "5-tile radius" is a square region (mirrors the M14F
/// debris drop animation).
#[must_use]
pub fn collapse_distance_tiles(a: (i32, i32), b: (i32, i32)) -> u32 {
    let dx = (a.0 - b.0).unsigned_abs();
    let dy = (a.1 - b.1).unsigned_abs();
    dx.max(dy)
}

/// Compute the per-actor fall-impulse damage for a watchtower
/// destruction event. Damage scales linearly with distance: an actor
/// at the epicenter takes [`WatchtowerTier::base_fall_damage`] and an
/// actor at the radius edge takes 0. Distances beyond the radius
/// return 0 (no damage).
#[must_use]
pub fn fall_impulse_damage_for(
    tier: WatchtowerTier,
    distance_tiles: u32,
) -> u32 {
    let radius = tier.collapse_radius_tiles();
    if distance_tiles > radius {
        return 0;
    }
    let base = tier.base_fall_damage();
    if radius == 0 {
        return base;
    }
    // Linear decay: damage = base * (radius - distance) / radius.
    let remaining = radius - distance_tiles;
    base * remaining / radius
}

/// Drive the M14F lateral-collapse for a destroyed watchtower:
/// generate the `watchtower_destroyed` event + per-actor
/// `fall_impulse_damage` events for actors inside the radius +
/// surface the embedded repeater id (so callers can fire
/// `fortification_destroyed` against it).
///
/// Caller passes the destruction-pending payload from
/// [`Watchtower::apply_damage`], the list of actors-in-world (the
/// engine pre-filters by tile distance), and the current tick.
#[must_use]
pub fn apply_destruction_collapse(
    pending: WatchtowerDestructionPending,
    actors_in_world: &[ActorInCollapseRadius],
    tick_index: u64,
) -> WatchtowerDestructionOutcome {
    let mut fall_impulse_events = Vec::new();
    for actor in actors_in_world {
        let distance = collapse_distance_tiles(pending.pos_tiles, actor.pos_tiles);
        if distance > pending.collapse_radius_tiles {
            continue;
        }
        let damage = fall_impulse_damage_for(pending.tier, distance);
        if damage == 0 {
            continue;
        }
        fall_impulse_events.push(FallImpulseDamageEvent {
            actor_id: actor.actor_id,
            tower_id: pending.tower_id,
            distance_tiles: distance,
            impulse_damage: damage,
        });
    }
    WatchtowerDestructionOutcome {
        destroyed: WatchtowerDestroyedEvent {
            tower_id: pending.tower_id,
            tier: pending.tier,
            collapse_radius_tiles: pending.collapse_radius_tiles,
            tick_index,
        },
        fall_impulse_events,
        destroyed_repeater: pending.integrated_repeater,
    }
}

// ---------------------------------------------------------------------
// Spotlight
// ---------------------------------------------------------------------

/// One placed `spotlight` mounted on a watchtower (typically T2 /
/// T3). 1 kW grid power; 24-tile cone-of-light; reveals concealed
/// actors. Flashbang dazzle keeps it offline for exactly 12 seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Spotlight {
    pub id: FortificationId,
    pub hp: u32,
    /// Watchtower id the spotlight is mounted on (None for free-
    /// standing test fixtures).
    pub mounted_on: Option<FortificationId>,
    /// Tile origin of the watchtower the spotlight is mounted on. The
    /// cone-of-light projects from this position.
    pub pos_tiles: (i32, i32),
    /// Cone facing direction in radians (0 = +x, π/2 = +y).
    pub facing_radians: f32,
    /// True when the M29 grid is supplying the spotlight's 1 kW
    /// continuous draw.
    pub powered: bool,
    /// True when the spotlight is on (operator-flipped switch, plus
    /// powered, plus not dazzled). The `online()` helper computes the
    /// effective state.
    pub switched_on: bool,
    /// Ticks remaining in the flashbang dazzle window. 0 when not
    /// dazzled.
    pub dazzle_ticks_remaining: u32,
}

impl Spotlight {
    /// Build a freshly mounted spotlight: HP = max, switched on,
    /// powered (the engine flips `powered` to false when the M29 grid
    /// loses the 1 kW supply).
    #[must_use]
    pub fn new(
        id: FortificationId,
        mounted_on: Option<FortificationId>,
        pos_tiles: (i32, i32),
        facing_radians: f32,
    ) -> Self {
        Self {
            id,
            hp: SPOTLIGHT_MAX_HP,
            mounted_on,
            pos_tiles,
            facing_radians,
            powered: true,
            switched_on: true,
            dazzle_ticks_remaining: 0,
        }
    }

    /// True when the spotlight has been destroyed (HP 0).
    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        self.hp == 0
    }

    /// True when the spotlight is currently emitting its cone-of-light
    /// (alive + powered + switched on + not dazzled).
    #[must_use]
    pub const fn online(self) -> bool {
        !self.is_destroyed()
            && self.powered
            && self.switched_on
            && self.dazzle_ticks_remaining == 0
    }

    /// Trigger a flashbang dazzle. Returns the per-event payload the
    /// recorder writes to the replay log. The dazzle holds the
    /// spotlight offline for [`SPOTLIGHT_DAZZLE_SECONDS`] seconds; the
    /// engine drives the ticker via [`Self::tick_dazzle`].
    pub fn dazzle(&mut self, tick_rate_hz: u32, tick_index: u64) -> SpotlightDazzledEvent {
        let dazzle_ticks = SPOTLIGHT_DAZZLE_SECONDS.saturating_mul(tick_rate_hz);
        self.dazzle_ticks_remaining = dazzle_ticks;
        SpotlightDazzledEvent {
            spotlight_id: self.id,
            duration_ticks: dazzle_ticks,
            tick_index,
        }
    }

    /// Decrement the dazzle timer by one tick. Returns true on the
    /// tick the spotlight returns online (transition edge for the
    /// engine to fire `spotlight_online_resumed` if it ever ships).
    pub fn tick_dazzle(&mut self) -> bool {
        if self.dazzle_ticks_remaining == 0 {
            return false;
        }
        self.dazzle_ticks_remaining -= 1;
        self.dazzle_ticks_remaining == 0
    }
}

/// Replay-event-payload shape for `fortification.spotlight_dazzled`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpotlightDazzledEvent {
    pub spotlight_id: FortificationId,
    pub duration_ticks: u32,
    pub tick_index: u64,
}

/// Inputs to [`spotlight_illuminates`]. Field names match the spec
/// scenario verbatim so cf-control / cf-ai callers can wire them
/// directly.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpotlightConeInputs {
    pub spotlight_pos_tiles: (i32, i32),
    pub facing_radians: f32,
    /// Cone half-angle in degrees. The spotlight illuminates targets
    /// inside a `2 * half_angle` cone.
    pub half_angle_degrees: f32,
    pub target_pos_tiles: (i32, i32),
}

/// Returns true when `target` falls inside the spotlight's
/// cone-of-light (within 24 tiles of the spotlight position AND
/// within the cone half-angle of the spotlight's facing).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn spotlight_illuminates(spotlight: Spotlight, inputs: SpotlightConeInputs) -> bool {
    if !spotlight.online() {
        return false;
    }
    let dx = (inputs.target_pos_tiles.0 - inputs.spotlight_pos_tiles.0) as f32;
    let dy = (inputs.target_pos_tiles.1 - inputs.spotlight_pos_tiles.1) as f32;
    let range_sq = dx * dx + dy * dy;
    let max_range = SPOTLIGHT_CONE_RANGE_TILES as f32;
    if range_sq > max_range * max_range {
        return false;
    }
    if range_sq < f32::EPSILON {
        return true;
    }
    let target_angle = dy.atan2(dx);
    let half_angle_rad = inputs.half_angle_degrees.to_radians();
    let delta = (target_angle - inputs.facing_radians).abs();
    let normalized = if delta > std::f32::consts::PI {
        2.0 * std::f32::consts::PI - delta
    } else {
        delta
    };
    normalized <= half_angle_rad
}

// ---------------------------------------------------------------------
// Observation post
// ---------------------------------------------------------------------

/// One placed `observation_post` — hardened spotter station providing
/// +75% acquisition bonus to faction-wide artillery (M44D SPG) for
/// targets the observer has LOS to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationPost {
    pub id: FortificationId,
    pub hp: u32,
    pub pos_tiles: (i32, i32),
    /// Actor id of the observer occupying the station. `None` when
    /// empty (no bonus is granted).
    pub observer_actor_id: Option<u64>,
}

impl ObservationPost {
    /// Build a freshly placed observation post (HP cap, no observer).
    #[must_use]
    pub const fn new(id: FortificationId, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            hp: OBSERVATION_POST_MAX_HP,
            pos_tiles,
            observer_actor_id: None,
        }
    }

    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        self.hp == 0
    }

    /// Bind the supplied actor as the observer. Returns the prior
    /// observer (if any).
    pub fn occupy(&mut self, actor_id: u64) -> Option<u64> {
        let prior = self.observer_actor_id;
        self.observer_actor_id = Some(actor_id);
        prior
    }

    /// Release the observer (the actor leaves the post). Returns the
    /// prior observer id.
    pub fn vacate(&mut self) -> Option<u64> {
        self.observer_actor_id.take()
    }

    /// True when an observer is bound + the post is intact.
    #[must_use]
    pub const fn has_active_observer(self) -> bool {
        self.observer_actor_id.is_some() && !self.is_destroyed()
    }
}

/// Resolve the faction-wide artillery acquisition multiplier for a
/// friendly SPG firing at a target the observer has LOS to.
///
/// Returns 1.75 when the post has an active observer AND the observer
/// has LOS to the target; 1.0 otherwise.
#[must_use]
pub fn observation_post_artillery_multiplier(
    post: Option<&ObservationPost>,
    observer_has_los_to_target: bool,
) -> f32 {
    match post {
        Some(p) if p.has_active_observer() && observer_has_los_to_target => {
            OBSERVATION_POST_ARTILLERY_ACQUISITION_MULTIPLIER
        }
        _ => 1.0,
    }
}

// ---------------------------------------------------------------------
// Radio repeater
// ---------------------------------------------------------------------

/// One placed `radio_repeater` — extends the owning faction's
/// squad-radio range by 100 tiles. May be **embedded** on a T3
/// watchtower (destroyed together with the tower per spec) or **stand-
/// alone** (a 1×1 footprint placed independently).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadioRepeater {
    pub id: FortificationId,
    pub hp: u32,
    pub pos_tiles: (i32, i32),
    /// Watchtower id the repeater is embedded on. `None` for
    /// standalone repeaters.
    pub embedded_on: Option<FortificationId>,
}

impl RadioRepeater {
    /// Build a freshly placed repeater.
    #[must_use]
    pub const fn new(
        id: FortificationId,
        pos_tiles: (i32, i32),
        embedded_on: Option<FortificationId>,
    ) -> Self {
        Self {
            id,
            hp: RADIO_REPEATER_MAX_HP,
            pos_tiles,
            embedded_on,
        }
    }

    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        self.hp == 0
    }

    /// True when this repeater is embedded on the supplied watchtower
    /// id. Used by the destruction-cascade helper to find the matching
    /// repeater when the host tower is destroyed.
    #[must_use]
    pub fn is_embedded_on(self, tower: FortificationId) -> bool {
        self.embedded_on == Some(tower)
    }
}

/// Compute the faction's effective squad-radio range given the base
/// range + the set of intact repeaters. Each alive repeater adds 100
/// tiles. Destroyed repeaters contribute nothing — per spec, "Squad-
/// radio range drops by 100 tiles immediately on repeater destruction".
#[must_use]
pub fn faction_radio_range(base_range_tiles: u32, repeaters: &[RadioRepeater]) -> u32 {
    let bonus: u32 = repeaters
        .iter()
        .filter(|r| !r.is_destroyed())
        .map(|_| RADIO_REPEATER_RANGE_BONUS_TILES)
        .sum();
    base_range_tiles.saturating_add(bonus)
}

// ---------------------------------------------------------------------
// Spotter mark (AI-OBS-A-01 doctrine consumers)
// ---------------------------------------------------------------------

/// Per-spec § "Spotter role" + spec Gherkin "Spotter in watchtower
/// marks target": +50% acquisition multiplier applied to a squad MG /
/// sniper firing at a marked target.
pub const SPOTTER_TARGET_MARK_ACQUISITION_BONUS: f32 = 1.5;

/// One inflight spotter-target mark. The spotter actor in a watchtower
/// (or observation post) emits one of these via AI-OBS-A-01; squad
/// consumers query the per-target table to apply the +50% acquisition
/// bonus. Marks expire 3 s after LOS is lost (driven from the
/// cf-ai::observer_doctrine module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpotterMark {
    pub spotter_actor_id: u64,
    pub target_actor_id: u64,
    pub target_pos_tiles: (i32, i32),
    /// Tick the mark was first emitted (or last refreshed).
    pub mark_tick: u64,
    /// Tick the spotter most recently had LOS to the target. The mark
    /// expires when `current_tick - last_los_tick >= 3 * tick_rate`.
    pub last_los_tick: u64,
}

/// Inputs to [`spotter_acquisition_multiplier`]. cf-ai target-selection
/// passes these in when resolving the per-target acquisition rate for
/// a squad MG / sniper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpotterAcquisitionInputs {
    /// Active spotter mark for the target (None when no mark is in
    /// flight).
    pub mark: Option<SpotterMark>,
    /// Squad id of the firing actor (used in the future to gate the
    /// bonus to the marking faction; m9c-3 ships with the simpler
    /// "any friendly mark grants the bonus" semantics).
    pub firing_actor_id: u64,
    pub target_actor_id: u64,
}

/// Returns 1.5 when the supplied mark exists AND targets the same
/// actor; 1.0 otherwise. Used by cf-ai target-selection to scale the
/// per-tick acquisition rate of a squad MG/sniper at a marked target.
#[must_use]
pub fn spotter_acquisition_multiplier(inputs: SpotterAcquisitionInputs) -> f32 {
    match inputs.mark {
        Some(m) if m.target_actor_id == inputs.target_actor_id => {
            SPOTTER_TARGET_MARK_ACQUISITION_BONUS
        }
        _ => 1.0,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn fresh_t3() -> Watchtower {
        Watchtower::new_built(
            FortificationId(1),
            WatchtowerTier::T3,
            (10, 10),
            Some(FortificationId(2)),
        )
    }

    fn fresh_t2() -> Watchtower {
        Watchtower::new_built(
            FortificationId(3),
            WatchtowerTier::T2,
            (20, 20),
            None,
        )
    }

    fn fresh_t1() -> Watchtower {
        Watchtower::new_built(
            FortificationId(4),
            WatchtowerTier::T1,
            (30, 30),
            None,
        )
    }

    /// VAL-M9C-019 part 1: every tier's HP cap matches the spec table.
    #[test]
    fn watchtower_tier_hp_matches_spec_table() {
        assert_eq!(WatchtowerTier::T1.max_hp(), 600);
        assert_eq!(WatchtowerTier::T2.max_hp(), 1200);
        assert_eq!(WatchtowerTier::T3.max_hp(), 2400);

        let t3 = fresh_t3();
        assert_eq!(t3.hp, WATCHTOWER_T3_MAX_HP);
        assert_eq!(t3.tier, WatchtowerTier::T3);

        let t2 = fresh_t2();
        assert_eq!(t2.hp, WATCHTOWER_T2_MAX_HP);

        let t1 = fresh_t1();
        assert_eq!(t1.hp, WATCHTOWER_T1_MAX_HP);
    }

    /// Spec table: only T3 ships with an integrated radio repeater.
    /// T1/T2 ignore the supplied repeater id.
    #[test]
    fn only_t3_has_integrated_repeater() {
        assert!(WatchtowerTier::T3.has_integrated_repeater());
        assert!(!WatchtowerTier::T2.has_integrated_repeater());
        assert!(!WatchtowerTier::T1.has_integrated_repeater());

        let t3 = fresh_t3();
        assert_eq!(t3.integrated_repeater, Some(FortificationId(2)));

        // T2 ignores the supplied repeater id (spec: only T3 has one).
        let t2 = Watchtower::new_built(
            FortificationId(99),
            WatchtowerTier::T2,
            (0, 0),
            Some(FortificationId(123)),
        );
        assert_eq!(t2.integrated_repeater, None);
    }

    /// VAL-M9C-019: T3 destruction emits the watchtower_destroyed
    /// event + fall_impulse damage to actors within the 5-tile radius
    /// + drops the integrated radio repeater.
    #[test]
    fn watchtower_t3_destruction_triggers_lateral_collapse() {
        let mut tower = fresh_t3();
        // Apply enough damage to drive HP to 0.
        let pending = tower.apply_damage(WATCHTOWER_T3_MAX_HP + 100);
        let pending = pending.expect("T3 destruction must emit pending payload");
        assert_eq!(pending.tower_id, FortificationId(1));
        assert_eq!(pending.tier, WatchtowerTier::T3);
        assert_eq!(
            pending.collapse_radius_tiles,
            WATCHTOWER_T3_COLLAPSE_RADIUS_TILES
        );
        assert_eq!(pending.integrated_repeater, Some(FortificationId(2)));

        // Actors: one inside radius (distance 2) + one at edge
        // (distance 5) + one outside (distance 6).
        let actors = vec![
            ActorInCollapseRadius {
                actor_id: 7,
                pos_tiles: (12, 10),
            }, // distance 2
            ActorInCollapseRadius {
                actor_id: 8,
                pos_tiles: (15, 10),
            }, // distance 5 (radius edge: damage = 0, skipped)
            ActorInCollapseRadius {
                actor_id: 9,
                pos_tiles: (16, 10),
            }, // distance 6 (outside)
            ActorInCollapseRadius {
                actor_id: 10,
                pos_tiles: (10, 10),
            }, // distance 0 (epicenter)
        ];
        let outcome = apply_destruction_collapse(pending, &actors, 1000);

        // Watchtower destroyed event payload.
        assert_eq!(outcome.destroyed.tower_id, FortificationId(1));
        assert_eq!(outcome.destroyed.tier, WatchtowerTier::T3);
        assert_eq!(
            outcome.destroyed.collapse_radius_tiles,
            WATCHTOWER_T3_COLLAPSE_RADIUS_TILES
        );
        assert_eq!(outcome.destroyed.tick_index, 1000);

        // Integrated repeater is surfaced for removal.
        assert_eq!(outcome.destroyed_repeater, Some(FortificationId(2)));

        // At least one fall_impulse event per spec gherkin:
        // "And actors within the radius take fall_impulse damage".
        assert!(
            !outcome.fall_impulse_events.is_empty(),
            "expected ≥1 fall_impulse damage event within 5-tile radius"
        );

        // Epicenter actor (10) takes max damage.
        let epicenter = outcome
            .fall_impulse_events
            .iter()
            .find(|e| e.actor_id == 10)
            .expect("epicenter actor must be in event list");
        assert_eq!(
            epicenter.impulse_damage,
            WATCHTOWER_T3_BASE_FALL_DAMAGE,
            "epicenter actor takes full fall-impulse base damage"
        );
        assert_eq!(epicenter.distance_tiles, 0);

        // Inside-radius actor (7) takes partial damage.
        let inside = outcome
            .fall_impulse_events
            .iter()
            .find(|e| e.actor_id == 7)
            .expect("inside-radius actor must be in event list");
        assert!(inside.impulse_damage > 0);
        assert!(inside.impulse_damage < WATCHTOWER_T3_BASE_FALL_DAMAGE);
        assert_eq!(inside.distance_tiles, 2);

        // Outside-radius actor (9) is NOT in the event list.
        assert!(
            outcome
                .fall_impulse_events
                .iter()
                .all(|e| e.actor_id != 9),
            "actor outside the 5-tile radius must NOT receive fall_impulse damage"
        );

        // Edge-of-radius actor (8) takes 0 damage (boundary case) and
        // is also not in the event list.
        assert!(
            outcome
                .fall_impulse_events
                .iter()
                .all(|e| e.actor_id != 8),
            "edge-of-radius actor takes 0 damage and is dropped from event list"
        );

        // Tower's integrated_repeater is now taken (None).
        assert_eq!(tower.integrated_repeater, None);
    }

    /// Lateral collapse radius scales with tier per spec § "Height
    /// tradeoff: taller tower = bigger collapse".
    #[test]
    fn watchtower_collapse_radius_scales_with_tier() {
        assert_eq!(
            WatchtowerTier::T1.collapse_radius_tiles(),
            WATCHTOWER_T1_COLLAPSE_RADIUS_TILES
        );
        assert_eq!(
            WatchtowerTier::T2.collapse_radius_tiles(),
            WATCHTOWER_T2_COLLAPSE_RADIUS_TILES
        );
        assert_eq!(
            WatchtowerTier::T3.collapse_radius_tiles(),
            WATCHTOWER_T3_COLLAPSE_RADIUS_TILES
        );
        assert!(
            WatchtowerTier::T3.collapse_radius_tiles()
                > WatchtowerTier::T2.collapse_radius_tiles()
        );
        assert!(
            WatchtowerTier::T2.collapse_radius_tiles()
                > WatchtowerTier::T1.collapse_radius_tiles()
        );
    }

    /// Damage decays linearly with distance: epicenter actor takes
    /// `base_fall_damage`; radius-edge actor takes 0.
    #[test]
    fn watchtower_fall_impulse_decays_linearly() {
        assert_eq!(
            fall_impulse_damage_for(WatchtowerTier::T3, 0),
            WATCHTOWER_T3_BASE_FALL_DAMAGE
        );
        // Half radius → half damage.
        let half = WATCHTOWER_T3_COLLAPSE_RADIUS_TILES / 2;
        let expected =
            WATCHTOWER_T3_BASE_FALL_DAMAGE * (WATCHTOWER_T3_COLLAPSE_RADIUS_TILES - half)
                / WATCHTOWER_T3_COLLAPSE_RADIUS_TILES;
        assert_eq!(
            fall_impulse_damage_for(WatchtowerTier::T3, half),
            expected
        );
        // Radius edge → 0 damage.
        assert_eq!(
            fall_impulse_damage_for(
                WatchtowerTier::T3,
                WATCHTOWER_T3_COLLAPSE_RADIUS_TILES
            ),
            0
        );
        // Outside radius → 0 damage.
        assert_eq!(
            fall_impulse_damage_for(WatchtowerTier::T3, 100),
            0
        );
    }

    /// VAL-M9C-020: faction squad-radio range drops by 100 tiles
    /// immediately on repeater destruction (both embedded + standalone).
    #[test]
    fn radio_range_drops_by_100_on_repeater_destruction_embedded() {
        let mut embedded = RadioRepeater::new(FortificationId(2), (10, 10), Some(FortificationId(1)));
        let base_range = 200u32;

        // Pre-destruction: base + 100 = 300.
        let pre = faction_radio_range(base_range, &[embedded]);
        assert_eq!(pre, base_range + RADIO_REPEATER_RANGE_BONUS_TILES);

        // Destroy the repeater (HP → 0).
        embedded.hp = 0;
        assert!(embedded.is_destroyed());

        // Post-destruction: drops back to base_range (100 less than pre).
        let post = faction_radio_range(base_range, &[embedded]);
        assert_eq!(post, base_range);
        assert_eq!(pre - post, RADIO_REPEATER_RANGE_BONUS_TILES);
    }

    #[test]
    fn radio_range_drops_by_100_on_repeater_destruction_standalone() {
        let mut standalone = RadioRepeater::new(FortificationId(5), (50, 50), None);
        let base_range = 150u32;

        let pre = faction_radio_range(base_range, &[standalone]);
        assert_eq!(pre, base_range + RADIO_REPEATER_RANGE_BONUS_TILES);

        standalone.hp = 0;
        let post = faction_radio_range(base_range, &[standalone]);
        assert_eq!(pre - post, RADIO_REPEATER_RANGE_BONUS_TILES);
    }

    /// Multiple repeaters stack: 2 alive = +200, 1 alive + 1 destroyed = +100.
    #[test]
    fn radio_range_multiple_repeaters_stack_correctly() {
        let r1 = RadioRepeater::new(FortificationId(1), (0, 0), None);
        let r2 = RadioRepeater::new(FortificationId(2), (10, 10), None);
        let mut r3 = RadioRepeater::new(FortificationId(3), (20, 20), Some(FortificationId(99)));
        let base_range = 200u32;

        // 3 alive → +300.
        assert_eq!(
            faction_radio_range(base_range, &[r1, r2, r3]),
            base_range + 3 * RADIO_REPEATER_RANGE_BONUS_TILES
        );

        // Destroy one → +200.
        r3.hp = 0;
        assert_eq!(
            faction_radio_range(base_range, &[r1, r2, r3]),
            base_range + 2 * RADIO_REPEATER_RANGE_BONUS_TILES
        );
    }

    // ---- Spotlight tests ----

    fn online_spotlight() -> Spotlight {
        Spotlight::new(FortificationId(10), Some(FortificationId(1)), (50, 50), 0.0)
    }

    /// VAL-M9C-023: spotlight illuminates a target in cone.
    #[test]
    fn spotlight_cone_reveals_concealed_actor() {
        let spotlight = online_spotlight();
        assert!(spotlight.online());

        // Target straight ahead at distance 16 → in cone.
        let inputs = SpotlightConeInputs {
            spotlight_pos_tiles: (50, 50),
            facing_radians: 0.0, // +x direction
            half_angle_degrees: SPOTLIGHT_HALF_ANGLE_DEGREES,
            target_pos_tiles: (66, 50),
        };
        assert!(
            spotlight_illuminates(spotlight, inputs),
            "target straight ahead at 16 tiles must be illuminated"
        );

        // Target straight behind → outside cone.
        let behind = SpotlightConeInputs {
            target_pos_tiles: (34, 50),
            ..inputs
        };
        assert!(
            !spotlight_illuminates(spotlight, behind),
            "target straight behind must not be illuminated"
        );

        // Target inside cone but beyond 24-tile range → not illuminated.
        let too_far = SpotlightConeInputs {
            target_pos_tiles: (80, 50), // 30 tiles away
            ..inputs
        };
        assert!(
            !spotlight_illuminates(spotlight, too_far),
            "target beyond 24-tile cone range must not be illuminated"
        );

        // Target inside cone, just inside range edge → illuminated.
        let in_range_edge = SpotlightConeInputs {
            target_pos_tiles: (50 + SPOTLIGHT_CONE_RANGE_TILES as i32, 50),
            ..inputs
        };
        assert!(spotlight_illuminates(spotlight, in_range_edge));
    }

    /// VAL-M9C-024: flashbang dazzles spotlight for exactly 12 s ±1 tick.
    #[test]
    fn spotlight_flashbang_12s_dazzle() {
        let tick_rate_hz = 60u32;
        let mut spotlight = online_spotlight();
        assert!(spotlight.online());

        let event = spotlight.dazzle(tick_rate_hz, 500);
        assert_eq!(event.spotlight_id, spotlight.id);
        assert_eq!(event.tick_index, 500);
        assert_eq!(
            event.duration_ticks,
            SPOTLIGHT_DAZZLE_SECONDS * tick_rate_hz
        );
        assert!(!spotlight.online(), "spotlight must be offline while dazzled");

        // Tick forward `12 * tick_rate - 1` ticks: still offline.
        let expected_dazzle_ticks = SPOTLIGHT_DAZZLE_SECONDS * tick_rate_hz;
        for _ in 0..expected_dazzle_ticks - 1 {
            let resumed = spotlight.tick_dazzle();
            assert!(!resumed, "spotlight stays offline mid-dazzle");
            assert!(!spotlight.online());
        }

        // Final tick: spotlight comes back online.
        let resumed = spotlight.tick_dazzle();
        assert!(resumed, "spotlight returns online on last dazzle tick");
        assert!(spotlight.online());
    }

    #[test]
    fn spotlight_does_not_illuminate_when_dazzled() {
        let mut spotlight = online_spotlight();
        spotlight.dazzle(60, 0);
        let inputs = SpotlightConeInputs {
            spotlight_pos_tiles: (50, 50),
            facing_radians: 0.0,
            half_angle_degrees: SPOTLIGHT_HALF_ANGLE_DEGREES,
            target_pos_tiles: (60, 50),
        };
        assert!(
            !spotlight_illuminates(spotlight, inputs),
            "dazzled spotlight must not illuminate even targets in cone"
        );
    }

    #[test]
    fn spotlight_does_not_illuminate_when_destroyed_or_unpowered() {
        let mut spotlight = online_spotlight();
        let inputs = SpotlightConeInputs {
            spotlight_pos_tiles: (50, 50),
            facing_radians: 0.0,
            half_angle_degrees: SPOTLIGHT_HALF_ANGLE_DEGREES,
            target_pos_tiles: (60, 50),
        };

        // Destroyed → no light.
        spotlight.hp = 0;
        assert!(!spotlight_illuminates(spotlight, inputs));

        // Restore HP but cut power → still no light.
        spotlight.hp = SPOTLIGHT_MAX_HP;
        spotlight.powered = false;
        assert!(!spotlight_illuminates(spotlight, inputs));

        // Re-power but switched off → still no light.
        spotlight.powered = true;
        spotlight.switched_on = false;
        assert!(!spotlight_illuminates(spotlight, inputs));

        // All systems online → light.
        spotlight.switched_on = true;
        assert!(spotlight_illuminates(spotlight, inputs));
    }

    // ---- Observation post tests ----

    #[test]
    fn observation_post_artillery_bonus_only_with_observer_and_los() {
        let mut post = ObservationPost::new(FortificationId(7), (10, 10));
        // No observer → no bonus.
        assert_eq!(
            observation_post_artillery_multiplier(Some(&post), true),
            1.0
        );

        // Bind observer; observer has LOS → +75% (1.75x).
        post.occupy(42);
        assert_eq!(
            observation_post_artillery_multiplier(Some(&post), true),
            OBSERVATION_POST_ARTILLERY_ACQUISITION_MULTIPLIER
        );

        // Observer present but no LOS → no bonus.
        assert_eq!(
            observation_post_artillery_multiplier(Some(&post), false),
            1.0
        );

        // Observer leaves → bonus drops.
        let prior = post.vacate();
        assert_eq!(prior, Some(42));
        assert!(!post.has_active_observer());
        assert_eq!(
            observation_post_artillery_multiplier(Some(&post), true),
            1.0
        );

        // Destroyed post → no bonus even with observer field set.
        post.observer_actor_id = Some(99);
        post.hp = 0;
        assert!(post.is_destroyed());
        assert_eq!(
            observation_post_artillery_multiplier(Some(&post), true),
            1.0
        );

        // None passed → no bonus.
        assert_eq!(observation_post_artillery_multiplier(None, true), 1.0);
    }

    // ---- Spotter mark / AI-OBS-A-01 consumer surface ----

    #[test]
    fn spotter_acquisition_multiplier_only_applies_to_marked_target() {
        let mark = SpotterMark {
            spotter_actor_id: 1,
            target_actor_id: 99,
            target_pos_tiles: (20, 20),
            mark_tick: 100,
            last_los_tick: 100,
        };

        // Mark targets actor 99; firing actor at actor 99 → +50%.
        let mult = spotter_acquisition_multiplier(SpotterAcquisitionInputs {
            mark: Some(mark),
            firing_actor_id: 5,
            target_actor_id: 99,
        });
        assert_eq!(mult, SPOTTER_TARGET_MARK_ACQUISITION_BONUS);

        // Different target → no bonus.
        let mult = spotter_acquisition_multiplier(SpotterAcquisitionInputs {
            mark: Some(mark),
            firing_actor_id: 5,
            target_actor_id: 100,
        });
        assert_eq!(mult, 1.0);

        // No mark at all → no bonus.
        let mult = spotter_acquisition_multiplier(SpotterAcquisitionInputs {
            mark: None,
            firing_actor_id: 5,
            target_actor_id: 99,
        });
        assert_eq!(mult, 1.0);
    }

    /// Chebyshev distance helper is correct.
    #[test]
    fn collapse_distance_is_chebyshev() {
        assert_eq!(collapse_distance_tiles((0, 0), (3, 4)), 4);
        assert_eq!(collapse_distance_tiles((0, 0), (-2, 1)), 2);
        assert_eq!(collapse_distance_tiles((10, 10), (10, 10)), 0);
        assert_eq!(collapse_distance_tiles((0, 0), (0, 5)), 5);
        assert_eq!(collapse_distance_tiles((0, 0), (5, 0)), 5);
    }

    /// Pre-existing test from the m9c-1 placeholder — keep so the
    /// rebuilt surface still satisfies VAL-M9C-002.
    #[test]
    fn watchtower_t3_built_pins_max_hp() {
        let tower = Watchtower::new_built(
            FortificationId(1),
            WatchtowerTier::T3,
            (0, 0),
            None,
        );
        assert_eq!(tower.hp, WATCHTOWER_T3_MAX_HP);
    }

    /// Stable string ids round-trip for the tier enum (replay event
    /// payload compatibility).
    #[test]
    fn watchtower_tier_as_str_round_trips() {
        for t in WatchtowerTier::ALL {
            let parsed: WatchtowerTier = ron::from_str(t.as_str()).expect("ron round-trip");
            assert_eq!(parsed, t);
        }
    }
}
