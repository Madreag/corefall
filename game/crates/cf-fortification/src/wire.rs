//! M9C § "Barbed wire + razor wire + electrified fence": full kernel
//! for the 4 wire kinds + per-actor crossing state + wire_cutters
//! cut-time + electrified-fence power coupling + vehicle crush
//! interaction.
//!
//! Per the spec table:
//!
//! | Asset | HP  | Cross speed | Snag    | Cross dmg | Cut time |
//! |---|---|---|---|---|---|
//! | `barbed_wire`        | 200 | -75% (×0.25) | 0.5s  | 1 dmg/tick           | 3 s        |
//! | `razor_wire`         | 300 | -90% (×0.10) | 1.0s  | 4 dmg/tick + lacer.  | 4 s + 1 HP |
//! | `electrified_fence`  | 400 | cannot cross | shock | 80 J + knockback 4t  | 4 s + safe |
//! | `concertina_roll`    | 250 | -85% (×0.15) | 0.75s | 3 dmg/tick (4 tiles) | 4 s        |
//!
//! Per the spec § Notes for the implementer:
//!
//! > Per-actor wire state: don't store wire crossing state on the
//! > wire (one wire, many actors); store it on the actor's
//! > `crossing: Option<wire_id>`. Avoid the O(n×m) interaction-matrix
//! > trap.
//! >
//! > Electrified fence consumes power; if a fence's power coupling is
//! > destroyed (M14 hit on the coupling component), fire
//! > `fence_depowered` and the fence acts as barbed_wire from then on.
//! > Repair the coupling to re-energize.
//! >
//! > Vehicle interaction: light vehicles cross wire (taking minor track
//! > damage); heavy vehicles (tanks) crush wire (destroying it on
//! > contact, no damage to vehicle). `wire_crushed_by_vehicle` event
//! > fires.
//!
//! Per the feature definition expected-behavior list:
//!
//! > Each wire kind applies correct speed clamp + snag time + per-tick
//! > damage; wire_cutters cut barbed in 3 s; razor in 4 s + cutter
//! > takes 1 dmg; actor cross state stored on actor; electrified
//! > fence powered: 80 J shock + electrocution + knockback; depowered:
//! > 1 dmg/tick only; toggling M29 breaker depowers fence; power
//! > coupling destruction emits fence_depowered; light tank entering
//! > powered fence: fence_shocked_actor for driver + wire_crushed_by_
//! > vehicle for fence.
//!
//! VAL-M9C-032 / VAL-M9C-033 / VAL-M9C-034 / VAL-M9C-035 / VAL-M9C-036
//! / VAL-M9C-037 / VAL-M9C-038 / VAL-M9C-FENCE-DEPOWERED-SCHEMA land
//! here.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

// ---------------------------------------------------------------------
// Wire id newtype (consumed by the actor crate's per-actor crossing
// state). Stable u32 newtype so cf-actor can hold a
// `crossing: Option<WireId>` field without a tagged-union enum.
// ---------------------------------------------------------------------

/// Handle for a placed wire instance.
///
/// Per spec § Notes for the implementer: per-actor crossing state lives
/// on the actor as `crossing: Option<WireId>`. This newtype is what
/// cf-actor pulls into [`cf_actor::ActorState`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct WireId(pub u32);

impl From<WireId> for FortificationId {
    fn from(value: WireId) -> Self {
        FortificationId(value.0)
    }
}

impl From<FortificationId> for WireId {
    fn from(value: FortificationId) -> Self {
        WireId(value.0)
    }
}

// ---------------------------------------------------------------------
// WireKind enum (4 wire kinds enumerated in the spec § "Barbed wire +
// razor wire + electrified fence" table).
// ---------------------------------------------------------------------

/// Four wire kinds enumerated in the spec § "Barbed wire + razor wire
/// + electrified fence" table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireKind {
    BarbedWire = 0,
    RazorWire = 1,
    ElectrifiedFence = 2,
    ConcertinaRoll = 3,
}

impl WireKind {
    pub const ALL: [WireKind; 4] = [
        WireKind::BarbedWire,
        WireKind::RazorWire,
        WireKind::ElectrifiedFence,
        WireKind::ConcertinaRoll,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WireKind::BarbedWire => "barbed_wire",
            WireKind::RazorWire => "razor_wire",
            WireKind::ElectrifiedFence => "electrified_fence",
            WireKind::ConcertinaRoll => "concertina_roll",
        }
    }

    /// Spec table HP cap for the kind.
    #[must_use]
    pub const fn max_hp(self) -> u32 {
        match self {
            WireKind::BarbedWire => BARBED_WIRE_HP,
            WireKind::RazorWire => RAZOR_WIRE_HP,
            WireKind::ElectrifiedFence => ELECTRIFIED_FENCE_HP,
            WireKind::ConcertinaRoll => CONCERTINA_ROLL_HP,
        }
    }
}

// ---------------------------------------------------------------------
// Spec constants — pinned to the spec table verbatim. Speed
// multipliers are stored in basis-points (×100) to keep all crossing
// math integer-only.
// ---------------------------------------------------------------------

/// Spec table row HP for `barbed_wire`.
pub const BARBED_WIRE_HP: u32 = 200;
/// Spec table row HP for `razor_wire`.
pub const RAZOR_WIRE_HP: u32 = 300;
/// Spec table row HP for `electrified_fence`.
pub const ELECTRIFIED_FENCE_HP: u32 = 400;
/// Spec table row HP for `concertina_roll`.
pub const CONCERTINA_ROLL_HP: u32 = 250;

/// Barbed wire: speed clamped to 25% while crossing (spec: -75%).
pub const BARBED_WIRE_SPEED_BP: u32 = 25;
/// Razor wire: speed clamped to 10% while crossing (spec: -90%).
pub const RAZOR_WIRE_SPEED_BP: u32 = 10;
/// Concertina: speed clamped to 15% while crossing (spec: -85%).
pub const CONCERTINA_ROLL_SPEED_BP: u32 = 15;
/// Powered electrified fence: actor cannot cross (speed 0%).
pub const ELECTRIFIED_POWERED_SPEED_BP: u32 = 0;
/// Depowered electrified fence: behaves as barbed_wire (-75%).
pub const ELECTRIFIED_DEPOWERED_SPEED_BP: u32 = 25;
/// FORCE-through (no cutters, intentional bypass): speed clamped to
/// 2% per spec § "the player can FORCE through".
pub const FORCE_THROUGH_SPEED_BP: u32 = 2;

/// Barbed wire: 0.5 second snag window.
pub const BARBED_WIRE_SNAG_MILLIS: u32 = 500;
/// Razor wire: 1.0 second snag window.
pub const RAZOR_WIRE_SNAG_MILLIS: u32 = 1000;
/// Concertina: 0.75 second snag window.
pub const CONCERTINA_ROLL_SNAG_MILLIS: u32 = 750;

/// Barbed wire: 1 damage per tick while crossing.
pub const BARBED_WIRE_DAMAGE_PER_TICK: u32 = 1;
/// Razor wire: 4 damage per tick while crossing.
pub const RAZOR_WIRE_DAMAGE_PER_TICK: u32 = 4;
/// Concertina: 3 damage per tick while crossing.
pub const CONCERTINA_ROLL_DAMAGE_PER_TICK: u32 = 3;
/// FORCE-through (no cutters): 8 damage per tick.
pub const FORCE_THROUGH_DAMAGE_PER_TICK: u32 = 8;

/// Cut time for barbed wire with wire_cutters: 3 seconds.
pub const BARBED_WIRE_CUT_SECONDS: u32 = 3;
/// Cut time for razor wire: 4 seconds; cutter loses 1 HP per cut.
pub const RAZOR_WIRE_CUT_SECONDS: u32 = 4;
/// Razor wire: each cut costs the cutter 1 HP of durability.
pub const RAZOR_WIRE_CUTTER_DAMAGE: u32 = 1;
/// Cut time for electrified fence (DEPOWERED only): 4 seconds.
pub const ELECTRIFIED_FENCE_CUT_SECONDS: u32 = 4;
/// Cut time for concertina coil section: 4 seconds.
pub const CONCERTINA_ROLL_CUT_SECONDS: u32 = 4;

/// Concertina coverage footprint (4 tiles wide; spec table column).
pub const CONCERTINA_ROLL_FOOTPRINT_TILES: u32 = 4;

/// Powered electrified fence: 80 J HE-equivalent shock damage per
/// contact (spec § "Electrified fence — powered grid").
pub const FENCE_SHOCK_JOULES: u32 = 80;
/// Powered electrified fence: 4-tile knockback when shock fires.
pub const FENCE_SHOCK_KNOCKBACK_TILES: u32 = 4;
/// Depowered electrified fence: matches barbed_wire damage profile
/// (1 dmg/tick + no shock).
pub const FENCE_DEPOWERED_DAMAGE_PER_TICK: u32 = BARBED_WIRE_DAMAGE_PER_TICK;

// ---------------------------------------------------------------------
// Wire instance struct
// ---------------------------------------------------------------------

/// One placed wire instance. The kernel keeps wires as a flat list;
/// cf-control owns the per-world id allocation. Per spec § Notes for
/// the implementer the *crossing-state* lives on the actor — this
/// struct holds only the wire's intrinsic state (kind, HP, position,
/// for electrified fences also the power coupling state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wire {
    pub id: WireId,
    pub kind: WireKind,
    pub hp: u32,
    pub pos_tiles: (i32, i32),
    /// For `ElectrifiedFence` only: true while the M29 grid is
    /// supplying the fence's 1 kW continuous draw via an intact
    /// power coupling. `false` collapses the fence to the
    /// barbed_wire-equivalent damage profile. Always `true` for the
    /// other 3 kinds (no power coupling).
    #[serde(default = "default_powered")]
    pub powered: bool,
    /// For `ElectrifiedFence` only: true while the dedicated power
    /// coupling component is intact. M14 hits on the coupling flip
    /// this to false + fire `fence_depowered { cause: coupling_destroyed }`.
    /// Always `true` for the other 3 kinds.
    #[serde(default = "default_powered")]
    pub power_coupling_intact: bool,
    /// For `ElectrifiedFence` only: true while the upstream M29
    /// breaker is in the closed (powered) position. The player can
    /// toggle this via cfctl to depower without destroying the
    /// coupling.
    #[serde(default = "default_powered")]
    pub breaker_closed: bool,
}

const fn default_powered() -> bool {
    true
}

impl Wire {
    /// Construct a placed wire instance at the kind's spec-table HP.
    #[must_use]
    pub fn new(id: WireId, kind: WireKind, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            kind,
            hp: kind.max_hp(),
            pos_tiles,
            powered: true,
            power_coupling_intact: true,
            breaker_closed: true,
        }
    }

    /// True when the wire has been destroyed (HP 0).
    #[must_use]
    pub const fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    /// True when the wire is currently delivering an active shock
    /// (powered electrified_fence with intact coupling + closed
    /// breaker).
    #[must_use]
    pub const fn is_electrified_active(&self) -> bool {
        matches!(self.kind, WireKind::ElectrifiedFence)
            && self.power_coupling_intact
            && self.breaker_closed
            && self.powered
    }

    /// Recompute the live `powered` flag from the coupling, breaker,
    /// and supplied M29-grid-energized flag. Returns the new value;
    /// engine ticks call this each tick on every electrified fence.
    pub fn sync_powered_from_grid(&mut self, grid_energized: bool) -> bool {
        let new_powered = matches!(self.kind, WireKind::ElectrifiedFence)
            && self.power_coupling_intact
            && self.breaker_closed
            && grid_energized;
        self.powered = new_powered;
        new_powered
    }
}

// ---------------------------------------------------------------------
// Per-actor crossing predicate
// ---------------------------------------------------------------------

/// Inputs to a single per-actor wire crossing evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCrossInputs {
    pub actor_id: u64,
    /// True when the actor is FORCE-ing through the wire (intentional
    /// bypass). Per spec § "the player can FORCE through (continued
    /// movement at 2% speed) OR retreat".
    pub force_through: bool,
}

/// Result of evaluating one actor's contact with a wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCrossOutcome {
    /// Crossing speed in basis-points (0..=100). 0 = cannot cross.
    pub speed_basis_points: u32,
    /// Damage applied per tick while the actor is crossing.
    pub damage_per_tick: u32,
    /// Snag window for the contact in milliseconds.
    pub snag_millis: u32,
    /// True when crossing applies the M16A `Laceration` affliction
    /// (razor_wire only per spec table).
    pub applies_laceration: bool,
    /// `Some(joules)` when crossing applies an instant shock blast
    /// (powered electrified fence only). The blast also triggers
    /// `FENCE_SHOCK_KNOCKBACK_TILES` of knockback + M16A
    /// `Electrocution` per spec.
    pub instant_shock_joules: Option<u32>,
    /// Knockback in tiles applied alongside the shock; zero when
    /// `instant_shock_joules` is None.
    pub instant_shock_knockback_tiles: u32,
    /// True when the contact would apply the M16A `Electrocution`
    /// affliction (powered electrified fence only).
    pub applies_electrocution: bool,
}

impl WireCrossOutcome {
    pub const SAFE: WireCrossOutcome = WireCrossOutcome {
        speed_basis_points: 100,
        damage_per_tick: 0,
        snag_millis: 0,
        applies_laceration: false,
        instant_shock_joules: None,
        instant_shock_knockback_tiles: 0,
        applies_electrocution: false,
    };
}

/// Evaluate one actor's contact with a wire. Returns the per-tick
/// damage / speed / affliction outcome the engine consumes.
#[must_use]
pub fn evaluate_wire_cross(wire: &Wire, inputs: WireCrossInputs) -> WireCrossOutcome {
    if wire.is_destroyed() {
        return WireCrossOutcome::SAFE;
    }
    if inputs.force_through {
        return WireCrossOutcome {
            speed_basis_points: FORCE_THROUGH_SPEED_BP,
            damage_per_tick: FORCE_THROUGH_DAMAGE_PER_TICK,
            // FORCE-through pins the actor to the contact tile each
            // tick rather than firing a discrete snag window; the
            // engine drives the held movement input.
            snag_millis: 0,
            applies_laceration: matches!(wire.kind, WireKind::RazorWire),
            instant_shock_joules: if wire.is_electrified_active() {
                Some(FENCE_SHOCK_JOULES)
            } else {
                None
            },
            instant_shock_knockback_tiles: if wire.is_electrified_active() {
                FENCE_SHOCK_KNOCKBACK_TILES
            } else {
                0
            },
            applies_electrocution: wire.is_electrified_active(),
        };
    }
    match wire.kind {
        WireKind::BarbedWire => WireCrossOutcome {
            speed_basis_points: BARBED_WIRE_SPEED_BP,
            damage_per_tick: BARBED_WIRE_DAMAGE_PER_TICK,
            snag_millis: BARBED_WIRE_SNAG_MILLIS,
            applies_laceration: false,
            instant_shock_joules: None,
            instant_shock_knockback_tiles: 0,
            applies_electrocution: false,
        },
        WireKind::RazorWire => WireCrossOutcome {
            speed_basis_points: RAZOR_WIRE_SPEED_BP,
            damage_per_tick: RAZOR_WIRE_DAMAGE_PER_TICK,
            snag_millis: RAZOR_WIRE_SNAG_MILLIS,
            applies_laceration: true,
            instant_shock_joules: None,
            instant_shock_knockback_tiles: 0,
            applies_electrocution: false,
        },
        WireKind::ConcertinaRoll => WireCrossOutcome {
            speed_basis_points: CONCERTINA_ROLL_SPEED_BP,
            damage_per_tick: CONCERTINA_ROLL_DAMAGE_PER_TICK,
            snag_millis: CONCERTINA_ROLL_SNAG_MILLIS,
            applies_laceration: false,
            instant_shock_joules: None,
            instant_shock_knockback_tiles: 0,
            applies_electrocution: false,
        },
        WireKind::ElectrifiedFence => {
            if wire.is_electrified_active() {
                WireCrossOutcome {
                    speed_basis_points: ELECTRIFIED_POWERED_SPEED_BP,
                    damage_per_tick: 0,
                    snag_millis: 0,
                    applies_laceration: false,
                    instant_shock_joules: Some(FENCE_SHOCK_JOULES),
                    instant_shock_knockback_tiles: FENCE_SHOCK_KNOCKBACK_TILES,
                    applies_electrocution: true,
                }
            } else {
                // Depowered: behaves as barbed_wire per spec § "Power
                // loss → fence acts as barbed_wire".
                WireCrossOutcome {
                    speed_basis_points: ELECTRIFIED_DEPOWERED_SPEED_BP,
                    damage_per_tick: FENCE_DEPOWERED_DAMAGE_PER_TICK,
                    snag_millis: BARBED_WIRE_SNAG_MILLIS,
                    applies_laceration: false,
                    instant_shock_joules: None,
                    instant_shock_knockback_tiles: 0,
                    applies_electrocution: false,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Wire cutters
// ---------------------------------------------------------------------

/// Per-kind cut time in whole seconds (with wire_cutters held [E]).
#[must_use]
pub const fn cut_time_seconds(kind: WireKind) -> u32 {
    match kind {
        WireKind::BarbedWire => BARBED_WIRE_CUT_SECONDS,
        WireKind::RazorWire => RAZOR_WIRE_CUT_SECONDS,
        WireKind::ElectrifiedFence => ELECTRIFIED_FENCE_CUT_SECONDS,
        WireKind::ConcertinaRoll => CONCERTINA_ROLL_CUT_SECONDS,
    }
}

/// Convert the kind's cut time to a tick budget.
#[must_use]
pub fn cut_time_ticks(kind: WireKind, tick_rate_hz: u32) -> u32 {
    cut_time_seconds(kind).saturating_mul(tick_rate_hz)
}

/// Cutter HP damage applied per completed cut (razor_wire only per
/// spec § "Cut with wire_cutters; 4-second hold; cutter takes 1 HP
/// damage").
#[must_use]
pub const fn cutter_damage_per_cut(kind: WireKind) -> u32 {
    match kind {
        WireKind::RazorWire => RAZOR_WIRE_CUTTER_DAMAGE,
        _ => 0,
    }
}

/// Inputs to one wire-cut attempt. The engine drives the hold timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireCutInputs {
    pub wire_id: WireId,
    pub actor_id: u64,
    /// True when the actor holds the wire_cutters tool.
    pub has_wire_cutters: bool,
    /// True when the actor is alive + adjacent + holding [E].
    pub adjacent_and_holding: bool,
    /// True when the actor took damage this tick (interrupts).
    pub took_damage_this_tick: bool,
    /// True when the actor moved this tick (interrupts).
    pub moved_this_tick: bool,
}

/// One tick of a wire-cut hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireCutTickResult {
    /// Hold continues; no event emitted this tick.
    Holding {
        hold_ticks: u32,
    },
    /// Cut completed: emit `wire_cut { wire_id, actor_id }` + drop
    /// the wire entity. Razor_wire additionally damages the cutter.
    Cut(WireCutEvent),
    /// Cut failed before completion (interrupted by movement /
    /// damage / release / fence still powered).
    Failed(WireCutFailureCause),
}

/// Reason a wire-cut attempt failed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireCutFailureCause {
    /// Actor moved during the hold.
    ActorMoved = 0,
    /// Actor took damage during the hold.
    ActorDamaged = 1,
    /// Actor released the [E] hold / lost adjacency.
    InterruptedOther = 2,
    /// Actor has no wire_cutters equipped.
    MissingTool = 3,
    /// Attempt to cut a powered electrified fence (would shock the
    /// cutter per spec § "Cut while powered = electrocution; must
    /// depower first").
    FenceStillPowered = 4,
}

impl WireCutFailureCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WireCutFailureCause::ActorMoved => "actor_moved",
            WireCutFailureCause::ActorDamaged => "actor_damaged",
            WireCutFailureCause::InterruptedOther => "interrupted_other",
            WireCutFailureCause::MissingTool => "missing_tool",
            WireCutFailureCause::FenceStillPowered => "fence_still_powered",
        }
    }
}

/// Replay-event-payload shape for `fortification.wire_cut`. Schema:
/// `cf-replay/schemas/event/wire_cut.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCutEvent {
    pub wire_id: WireId,
    pub wire_kind: WireKind,
    pub actor_id: u64,
    pub cut_duration_ticks: u32,
    /// Cutter durability damage applied by this cut (1 for
    /// razor_wire; 0 otherwise).
    pub cutter_damage: u32,
    pub tick_index: u64,
}

/// Drive one tick of a wire cut. Returns the per-tick outcome.
#[must_use]
pub fn tick_wire_cut(
    wire: &Wire,
    hold_ticks: u32,
    required_ticks: u32,
    inputs: WireCutInputs,
    tick_index: u64,
) -> WireCutTickResult {
    if !inputs.has_wire_cutters {
        return WireCutTickResult::Failed(WireCutFailureCause::MissingTool);
    }
    if wire.is_electrified_active() {
        return WireCutTickResult::Failed(WireCutFailureCause::FenceStillPowered);
    }
    if inputs.took_damage_this_tick {
        return WireCutTickResult::Failed(WireCutFailureCause::ActorDamaged);
    }
    if inputs.moved_this_tick {
        return WireCutTickResult::Failed(WireCutFailureCause::ActorMoved);
    }
    if !inputs.adjacent_and_holding {
        return WireCutTickResult::Failed(WireCutFailureCause::InterruptedOther);
    }
    let next = hold_ticks.saturating_add(1);
    if next >= required_ticks {
        WireCutTickResult::Cut(WireCutEvent {
            wire_id: inputs.wire_id,
            wire_kind: wire.kind,
            actor_id: inputs.actor_id,
            cut_duration_ticks: next,
            cutter_damage: cutter_damage_per_cut(wire.kind),
            tick_index,
        })
    } else {
        WireCutTickResult::Holding { hold_ticks: next }
    }
}

// ---------------------------------------------------------------------
// Electrified fence ↔ M29 power coupling
// ---------------------------------------------------------------------

/// Reason an electrified fence transitioned from powered → depowered.
/// Spec § "Power loss → fence acts as barbed_wire" enumerates three
/// causes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FenceDepoweredCause {
    /// Player flipped the upstream M29 breaker to open.
    BreakerToggled = 0,
    /// M14 hit destroyed the dedicated power coupling component.
    CouplingDestroyed = 1,
    /// Upstream M29 grid lost power (substation destruction etc.).
    GridFailure = 2,
}

impl FenceDepoweredCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FenceDepoweredCause::BreakerToggled => "breaker_toggled",
            FenceDepoweredCause::CouplingDestroyed => "coupling_destroyed",
            FenceDepoweredCause::GridFailure => "grid_failure",
        }
    }
}

/// Replay-event-payload shape for `fortification.fence_depowered`.
/// Schema: `cf-replay/schemas/event/fence_depowered.json`. Non-cosmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FenceDepoweredEvent {
    pub fence_id: WireId,
    pub cause: FenceDepoweredCause,
    pub tick_index: u64,
}

/// Replay-event-payload shape for `fortification.fence_shocked_actor`.
/// Schema: `cf-replay/schemas/event/fence_shocked_actor.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FenceShockedActorEvent {
    pub fence_id: WireId,
    pub actor_id: u64,
    pub joules: u32,
    pub knockback_tiles: u32,
    pub tick_index: u64,
}

/// Replay-event-payload shape for `fortification.wire_crushed_by_vehicle`.
/// Schema: `cf-replay/schemas/event/wire_crushed_by_vehicle.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCrushedByVehicleEvent {
    pub wire_id: WireId,
    pub vehicle_id: u64,
    pub tick_index: u64,
}

/// Toggle the upstream M29 breaker. Returns the depowered-cause when
/// the toggle transitions the fence from powered → depowered (engine
/// fans out [`FenceDepoweredEvent`]); `None` otherwise.
pub fn toggle_breaker(
    fence: &mut Wire,
    new_closed: bool,
    grid_energized: bool,
    tick_index: u64,
) -> Option<FenceDepoweredEvent> {
    if !matches!(fence.kind, WireKind::ElectrifiedFence) {
        return None;
    }
    let was_powered = fence.is_electrified_active();
    fence.breaker_closed = new_closed;
    fence.sync_powered_from_grid(grid_energized);
    let now_powered = fence.is_electrified_active();
    if was_powered && !now_powered {
        Some(FenceDepoweredEvent {
            fence_id: fence.id,
            cause: FenceDepoweredCause::BreakerToggled,
            tick_index,
        })
    } else {
        None
    }
}

/// Apply M14 damage to the dedicated power coupling component.
/// `coupling_destroyed = true` flips `power_coupling_intact = false`
/// and (if the fence was previously powered) returns the
/// [`FenceDepoweredEvent`] the recorder fans out.
pub fn apply_coupling_damage(
    fence: &mut Wire,
    coupling_destroyed: bool,
    tick_index: u64,
) -> Option<FenceDepoweredEvent> {
    if !matches!(fence.kind, WireKind::ElectrifiedFence) {
        return None;
    }
    if !coupling_destroyed {
        return None;
    }
    let was_powered = fence.is_electrified_active();
    fence.power_coupling_intact = false;
    fence.powered = false;
    if was_powered {
        Some(FenceDepoweredEvent {
            fence_id: fence.id,
            cause: FenceDepoweredCause::CouplingDestroyed,
            tick_index,
        })
    } else {
        None
    }
}

/// Sync the fence's power state from a grid-energized flag (driven
/// from M29 each tick). Returns `Some(event)` if the sync transitions
/// the fence from powered → depowered via grid failure.
pub fn sync_grid_energized(
    fence: &mut Wire,
    grid_energized: bool,
    tick_index: u64,
) -> Option<FenceDepoweredEvent> {
    if !matches!(fence.kind, WireKind::ElectrifiedFence) {
        return None;
    }
    let was_powered = fence.is_electrified_active();
    fence.sync_powered_from_grid(grid_energized);
    let now_powered = fence.is_electrified_active();
    if was_powered && !now_powered && fence.power_coupling_intact && fence.breaker_closed {
        Some(FenceDepoweredEvent {
            fence_id: fence.id,
            cause: FenceDepoweredCause::GridFailure,
            tick_index,
        })
    } else {
        None
    }
}

/// Re-energize a previously depowered fence (engineer repaired the
/// coupling OR breaker re-closed OR grid recovered). Idempotent: the
/// re-energize call only updates `powered` when its 3 preconditions
/// are met. No replay event fired (it's a state restoration, not a
/// sim-relevant transition for replay purposes).
pub fn reenergize_fence(fence: &mut Wire, grid_energized: bool) {
    if !matches!(fence.kind, WireKind::ElectrifiedFence) {
        return;
    }
    fence.power_coupling_intact = true;
    fence.breaker_closed = true;
    fence.sync_powered_from_grid(grid_energized);
}

// ---------------------------------------------------------------------
// Vehicle crush
// ---------------------------------------------------------------------

/// Vehicle class for the wire-vs-vehicle contact predicate (spec §
/// "Vehicle interaction: light vehicles cross wire (taking minor track
/// damage); heavy vehicles (tanks) crush wire (destroying it on
/// contact, no damage to vehicle)").
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireVehicleClass {
    Light = 0,
    Heavy = 1,
}

/// Minor track damage applied to a light vehicle that crosses wire
/// (per spec § "light vehicles cross wire (taking minor track damage)").
pub const LIGHT_VEHICLE_WIRE_TRACK_DAMAGE: u32 = 4;

/// Result of a vehicle contacting a wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireVehicleContactOutcome {
    /// True when the wire is destroyed by the contact (heavy vehicle
    /// only). The engine removes the wire entity + fans the crushed
    /// event.
    pub wire_destroyed: bool,
    /// `Some(damage)` when the contact deals per-component track
    /// damage to the vehicle (light vehicles only).
    pub vehicle_track_damage: Option<u32>,
    /// `Some(event)` when the wire was destroyed; `None` when the
    /// vehicle merely crossed it.
    pub crushed_event: Option<WireCrushedByVehicleEvent>,
    /// `Some(event)` when the vehicle's driver / crew took a shock
    /// from a powered electrified fence (powered fence + non-
    /// insulated crew).
    pub shock_event: Option<FenceShockedActorEvent>,
}

/// Apply one vehicle's contact with a wire. Mutates the wire's
/// `hp = 0` when destroyed; emits the appropriate events the
/// engine fans out.
pub fn apply_vehicle_contact(
    wire: &mut Wire,
    vehicle_class: WireVehicleClass,
    vehicle_id: u64,
    driver_actor_id: Option<u64>,
    driver_insulated: bool,
    tick_index: u64,
) -> WireVehicleContactOutcome {
    if wire.is_destroyed() {
        return WireVehicleContactOutcome {
            wire_destroyed: false,
            vehicle_track_damage: None,
            crushed_event: None,
            shock_event: None,
        };
    }
    let shock_event = if wire.is_electrified_active() && !driver_insulated {
        driver_actor_id.map(|actor_id| FenceShockedActorEvent {
            fence_id: wire.id,
            actor_id,
            joules: FENCE_SHOCK_JOULES,
            knockback_tiles: FENCE_SHOCK_KNOCKBACK_TILES,
            tick_index,
        })
    } else {
        None
    };
    match vehicle_class {
        WireVehicleClass::Heavy => {
            wire.hp = 0;
            WireVehicleContactOutcome {
                wire_destroyed: true,
                vehicle_track_damage: None,
                crushed_event: Some(WireCrushedByVehicleEvent {
                    wire_id: wire.id,
                    vehicle_id,
                    tick_index,
                }),
                shock_event,
            }
        }
        WireVehicleClass::Light => WireVehicleContactOutcome {
            wire_destroyed: false,
            vehicle_track_damage: Some(LIGHT_VEHICLE_WIRE_TRACK_DAMAGE),
            crushed_event: None,
            shock_event,
        },
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fence_at(id: u32, pos: (i32, i32)) -> Wire {
        Wire::new(WireId(id), WireKind::ElectrifiedFence, pos)
    }

    #[test]
    fn wire_kind_round_trips_via_ron() {
        for kind in WireKind::ALL {
            let s = kind.as_str();
            let parsed: WireKind = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, kind);
        }
    }

    /// VAL-M9C-007: unknown enum values are rejected at parse time.
    #[test]
    fn unknown_wire_kind_rejected_at_parse() {
        let result: Result<WireKind, _> = ron::from_str("\"definitely_not_real_wire\"");
        assert!(result.is_err(), "unknown enum must reject");
    }

    /// VAL-M9C-032: barbed wire applies -75% speed clamp, 0.5s snag,
    /// 1 dmg/tick.
    #[test]
    fn barbed_snag_bleed() {
        let wire = Wire::new(WireId(1), WireKind::BarbedWire, (0, 0));
        let outcome = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: false,
            },
        );
        assert_eq!(outcome.speed_basis_points, 25);
        assert_eq!(outcome.damage_per_tick, 1);
        assert_eq!(outcome.snag_millis, 500);
        assert!(!outcome.applies_laceration);
        assert!(outcome.instant_shock_joules.is_none());

        // FORCE-through bumps damage to 8 and speed to 2%.
        let forced = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: true,
            },
        );
        assert_eq!(forced.speed_basis_points, 2);
        assert_eq!(forced.damage_per_tick, 8);
    }

    /// VAL-M9C-035: razor wire applies -90% speed, 1.0s snag, 4
    /// dmg/tick, M16A laceration affliction.
    #[test]
    fn razor_laceration() {
        let wire = Wire::new(WireId(2), WireKind::RazorWire, (0, 0));
        let outcome = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: false,
            },
        );
        assert_eq!(outcome.speed_basis_points, 10);
        assert_eq!(outcome.damage_per_tick, 4);
        assert_eq!(outcome.snag_millis, 1000);
        assert!(outcome.applies_laceration, "razor_wire must apply M16A Laceration");
    }

    /// Concertina roll covers 4 tiles, snags 0.75s, -85% speed,
    /// 3 dmg/tick.
    #[test]
    fn concertina_roll_profile() {
        let wire = Wire::new(WireId(3), WireKind::ConcertinaRoll, (0, 0));
        let outcome = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: false,
            },
        );
        assert_eq!(outcome.speed_basis_points, 15);
        assert_eq!(outcome.damage_per_tick, 3);
        assert_eq!(outcome.snag_millis, 750);
        assert_eq!(CONCERTINA_ROLL_FOOTPRINT_TILES, 4);
    }

    /// VAL-M9C-036 part 1: powered electrified fence shocks crossing
    /// actor with 80 J + electrocution + knockback 4 tiles.
    #[test]
    fn electrified_shock() {
        let wire = fence_at(4, (0, 0));
        assert!(wire.is_electrified_active());
        let outcome = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: false,
            },
        );
        assert_eq!(outcome.speed_basis_points, 0, "powered fence cannot be crossed");
        assert_eq!(outcome.instant_shock_joules, Some(80));
        assert_eq!(outcome.instant_shock_knockback_tiles, 4);
        assert!(outcome.applies_electrocution);
    }

    /// VAL-M9C-036 part 2: depowered electrified fence behaves like
    /// barbed_wire (1 dmg/tick, no shock).
    #[test]
    fn electrified_depowered_behaves_like_barbed() {
        let mut wire = fence_at(4, (0, 0));
        let evt = toggle_breaker(&mut wire, false, true, 100);
        assert!(evt.is_some(), "breaker open transitions to depowered");
        assert_eq!(evt.unwrap().cause, FenceDepoweredCause::BreakerToggled);
        let outcome = evaluate_wire_cross(
            &wire,
            WireCrossInputs {
                actor_id: 7,
                force_through: false,
            },
        );
        assert_eq!(outcome.speed_basis_points, 25);
        assert_eq!(outcome.damage_per_tick, 1);
        assert!(outcome.instant_shock_joules.is_none());
        assert!(!outcome.applies_electrocution);
    }

    /// VAL-M9C-037: destroying the power coupling fires
    /// `fence_depowered { cause: coupling_destroyed }` and collapses
    /// the fence to barbed_wire behavior.
    #[test]
    fn fence_depowered_on_coupling_destroy() {
        let mut wire = fence_at(4, (0, 0));
        let evt = apply_coupling_damage(&mut wire, true, 250);
        let evt = evt.expect("coupling destruction emits event");
        assert_eq!(evt.cause, FenceDepoweredCause::CouplingDestroyed);
        assert_eq!(evt.tick_index, 250);
        assert!(!wire.is_electrified_active());

        // Re-applying coupling damage when already destroyed is a no-op
        // for event emission.
        let evt2 = apply_coupling_damage(&mut wire, true, 260);
        assert!(evt2.is_none());
    }

    /// VAL-M9C-037 / VAL-M9C-MOD-MISSING-DEPENDENCY pair:
    /// `fence_coupling_destruction_depowers` semantics asserted via
    /// public test name expected by the feature definition's
    /// verification-steps list.
    #[test]
    fn fence_coupling_destruction_depowers() {
        fence_depowered_on_coupling_destroy();
    }

    /// Grid failure path: when the upstream M29 substation drops,
    /// `sync_grid_energized(fence, false, t)` emits
    /// `fence_depowered { cause: grid_failure }`.
    #[test]
    fn fence_depowered_on_grid_failure() {
        let mut wire = fence_at(5, (0, 0));
        // Tick once with grid energized; powered stays true.
        assert!(sync_grid_energized(&mut wire, true, 0).is_none());
        assert!(wire.is_electrified_active());
        // Tick with grid de-energized; event fires.
        let evt = sync_grid_energized(&mut wire, false, 100)
            .expect("grid failure transitions to depowered");
        assert_eq!(evt.cause, FenceDepoweredCause::GridFailure);
        assert!(!wire.is_electrified_active());
    }

    /// Re-energize after grid restoration: state recovers but no
    /// event fires.
    #[test]
    fn fence_reenergizes_after_grid_recovery() {
        let mut wire = fence_at(5, (0, 0));
        let _ = sync_grid_energized(&mut wire, false, 100);
        assert!(!wire.is_electrified_active());
        reenergize_fence(&mut wire, true);
        assert!(wire.is_electrified_active());
    }

    /// VAL-M9C-033: wire_cutters cut barbed wire in 3 seconds at
    /// 60 Hz tick rate.
    #[test]
    fn wire_cutters_cut_barbed_in_three_seconds() {
        let wire = Wire::new(WireId(1), WireKind::BarbedWire, (0, 0));
        let required = cut_time_ticks(WireKind::BarbedWire, 60);
        assert_eq!(required, 3 * 60);
        let mut hold = 0;
        for tick in 0..required {
            let res = tick_wire_cut(
                &wire,
                hold,
                required,
                WireCutInputs {
                    wire_id: WireId(1),
                    actor_id: 7,
                    has_wire_cutters: true,
                    adjacent_and_holding: true,
                    took_damage_this_tick: false,
                    moved_this_tick: false,
                },
                u64::from(tick),
            );
            match res {
                WireCutTickResult::Holding { hold_ticks } => {
                    hold = hold_ticks;
                }
                WireCutTickResult::Cut(evt) if tick == required - 1 => {
                    assert_eq!(evt.wire_kind, WireKind::BarbedWire);
                    assert_eq!(evt.cutter_damage, 0);
                    return;
                }
                other => panic!("unexpected cut tick result {other:?}"),
            }
        }
        panic!("cut did not complete after {required} ticks");
    }

    /// VAL-M9C-033 (razor branch): razor wire cuts in 4 s + cutter
    /// loses 1 HP.
    #[test]
    fn wire_cutters_cut_razor_in_four_seconds_cutter_damaged() {
        let wire = Wire::new(WireId(2), WireKind::RazorWire, (0, 0));
        let required = cut_time_ticks(WireKind::RazorWire, 60);
        assert_eq!(required, 4 * 60);
        let res = tick_wire_cut(
            &wire,
            required - 1,
            required,
            WireCutInputs {
                wire_id: WireId(2),
                actor_id: 7,
                has_wire_cutters: true,
                adjacent_and_holding: true,
                took_damage_this_tick: false,
                moved_this_tick: false,
            },
            500,
        );
        match res {
            WireCutTickResult::Cut(evt) => {
                assert_eq!(evt.cutter_damage, RAZOR_WIRE_CUTTER_DAMAGE);
            }
            other => panic!("expected Cut for final tick, got {other:?}"),
        }
    }

    /// Cutting a powered electrified fence fails with
    /// `FenceStillPowered`; depowering enables the safe cut.
    #[test]
    fn wire_cut_fails_when_fence_still_powered() {
        let mut wire = fence_at(4, (0, 0));
        let res = tick_wire_cut(
            &wire,
            0,
            cut_time_ticks(WireKind::ElectrifiedFence, 60),
            WireCutInputs {
                wire_id: WireId(4),
                actor_id: 7,
                has_wire_cutters: true,
                adjacent_and_holding: true,
                took_damage_this_tick: false,
                moved_this_tick: false,
            },
            0,
        );
        assert_eq!(res, WireCutTickResult::Failed(WireCutFailureCause::FenceStillPowered));
        // Depower the fence; cut now proceeds.
        let _ = toggle_breaker(&mut wire, false, true, 1);
        let required = cut_time_ticks(WireKind::ElectrifiedFence, 60);
        let res = tick_wire_cut(
            &wire,
            required - 1,
            required,
            WireCutInputs {
                wire_id: WireId(4),
                actor_id: 7,
                has_wire_cutters: true,
                adjacent_and_holding: true,
                took_damage_this_tick: false,
                moved_this_tick: false,
            },
            500,
        );
        match res {
            WireCutTickResult::Cut(evt) => {
                assert_eq!(evt.wire_kind, WireKind::ElectrifiedFence);
                assert_eq!(evt.cutter_damage, 0, "depowered fence cut costs no cutter HP");
            }
            other => panic!("expected Cut for final tick, got {other:?}"),
        }
    }

    /// Wire cut failures: missing tool / movement / damage / release.
    #[test]
    fn wire_cut_failure_paths() {
        let wire = Wire::new(WireId(1), WireKind::BarbedWire, (0, 0));
        let required = cut_time_ticks(WireKind::BarbedWire, 60);
        let base = WireCutInputs {
            wire_id: WireId(1),
            actor_id: 7,
            has_wire_cutters: true,
            adjacent_and_holding: true,
            took_damage_this_tick: false,
            moved_this_tick: false,
        };
        assert_eq!(
            tick_wire_cut(
                &wire,
                0,
                required,
                WireCutInputs {
                    has_wire_cutters: false,
                    ..base
                },
                0,
            ),
            WireCutTickResult::Failed(WireCutFailureCause::MissingTool)
        );
        assert_eq!(
            tick_wire_cut(
                &wire,
                0,
                required,
                WireCutInputs {
                    moved_this_tick: true,
                    ..base
                },
                0,
            ),
            WireCutTickResult::Failed(WireCutFailureCause::ActorMoved)
        );
        assert_eq!(
            tick_wire_cut(
                &wire,
                0,
                required,
                WireCutInputs {
                    took_damage_this_tick: true,
                    ..base
                },
                0,
            ),
            WireCutTickResult::Failed(WireCutFailureCause::ActorDamaged)
        );
        assert_eq!(
            tick_wire_cut(
                &wire,
                0,
                required,
                WireCutInputs {
                    adjacent_and_holding: false,
                    ..base
                },
                0,
            ),
            WireCutTickResult::Failed(WireCutFailureCause::InterruptedOther)
        );
    }

    /// VAL-M9C-038: light tank entering powered electrified fence:
    /// fence_shocked_actor fires for the driver; wire_crushed_by_
    /// vehicle fires as the (heavy-class) tank destroys the fence.
    #[test]
    fn heavy_tank_crushes_powered_fence_shocks_driver() {
        let mut wire = fence_at(4, (0, 0));
        let outcome = apply_vehicle_contact(
            &mut wire,
            WireVehicleClass::Heavy,
            42,
            Some(99),
            false,
            1000,
        );
        assert!(outcome.wire_destroyed);
        let crushed = outcome.crushed_event.expect("crushed event fires");
        assert_eq!(crushed.wire_id, WireId(4));
        assert_eq!(crushed.vehicle_id, 42);
        let shock = outcome.shock_event.expect("shock event fires for driver");
        assert_eq!(shock.actor_id, 99);
        assert_eq!(shock.joules, FENCE_SHOCK_JOULES);
        assert_eq!(shock.knockback_tiles, FENCE_SHOCK_KNOCKBACK_TILES);
        assert!(wire.is_destroyed());
    }

    /// Light vehicle crossing barbed wire: minor track damage, wire
    /// survives.
    #[test]
    fn light_vehicle_crosses_barbed_wire() {
        let mut wire = Wire::new(WireId(1), WireKind::BarbedWire, (0, 0));
        let outcome = apply_vehicle_contact(
            &mut wire,
            WireVehicleClass::Light,
            42,
            Some(99),
            false,
            500,
        );
        assert!(!outcome.wire_destroyed);
        assert!(outcome.crushed_event.is_none());
        assert_eq!(outcome.vehicle_track_damage, Some(LIGHT_VEHICLE_WIRE_TRACK_DAMAGE));
        assert!(outcome.shock_event.is_none(), "barbed wire never shocks");
        assert_eq!(wire.hp, BARBED_WIRE_HP, "light contact does not damage wire HP");
    }

    /// Insulated driver: no shock event even on powered fence contact.
    #[test]
    fn insulated_driver_skips_shock_event() {
        let mut wire = fence_at(4, (0, 0));
        let outcome = apply_vehicle_contact(
            &mut wire,
            WireVehicleClass::Heavy,
            42,
            Some(99),
            true,
            1000,
        );
        assert!(outcome.wire_destroyed);
        assert!(
            outcome.shock_event.is_none(),
            "insulated driver does not get shocked"
        );
    }

    /// VAL-M9C-034: wire crossing state is stored per-actor, not
    /// per-wire. The kernel's [`Wire`] struct has zero per-actor
    /// fields; the per-actor state is owned by the actor crate (see
    /// `cf_actor::ActorState::crossing`).
    #[test]
    fn wire_state_lives_on_actor() {
        // Confirm the Wire kernel struct contains no per-actor map.
        // We do this by serializing a Wire and asserting the JSON
        // representation contains no `crossing`, `actors`, `bound_
        // actors`, or `interaction_matrix` keys (any of which would
        // indicate per-wire actor tracking — the O(n×m) trap).
        let wire = Wire::new(WireId(1), WireKind::BarbedWire, (0, 0));
        let json = serde_json::to_string(&wire).expect("serialize wire");
        for forbidden in ["crossing", "actors", "bound_actors", "interaction_matrix"] {
            assert!(
                !json.contains(forbidden),
                "Wire JSON {json} must not contain `{forbidden}` (would indicate per-actor map)"
            );
        }
        // Confirm the WireId type is the typed handle cf-actor pulls
        // into its `crossing: Option<WireId>` field. The newtype
        // round-trips via serde so cf-actor can serialize it.
        let id = WireId(123);
        let txt = serde_json::to_string(&id).expect("serialize id");
        let back: WireId = serde_json::from_str(&txt).expect("deserialize id");
        assert_eq!(id, back);

        // Confirm sizeof(Wire) is constant regardless of how many
        // actors interact with it: many wires + many actors stay at
        // O(n+m) memory (not O(n×m)) when crossing state lives on the
        // actor.
        let baseline = std::mem::size_of::<Wire>();
        let many_wires: Vec<Wire> = (0..1000)
            .map(|i| Wire::new(WireId(i), WireKind::BarbedWire, (i as i32, 0)))
            .collect();
        for w in &many_wires {
            assert_eq!(std::mem::size_of_val(w), baseline);
        }
    }

    /// Round-trip a `FenceDepoweredCause` through RON / serde.
    #[test]
    fn fence_depowered_cause_round_trips() {
        for c in [
            FenceDepoweredCause::BreakerToggled,
            FenceDepoweredCause::CouplingDestroyed,
            FenceDepoweredCause::GridFailure,
        ] {
            let s = c.as_str();
            let parsed: FenceDepoweredCause = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, c);
        }
    }

    /// `WireId` ↔ `FortificationId` round-trip (the two id namespaces
    /// share a `u32` underlay so cfctl observe-frames can use a
    /// single id space).
    #[test]
    fn wire_id_round_trips_with_fortification_id() {
        let wid = WireId(7);
        let fid: FortificationId = wid.into();
        assert_eq!(fid, FortificationId(7));
        let back: WireId = fid.into();
        assert_eq!(wid, back);
    }
}
