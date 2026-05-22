//! M9C minefield kernel — shared types: kinds, constants, detection
//! mask, mine instance, replay-event payloads.

use serde::{Deserialize, Serialize};

use crate::common::{FortificationFaction, FortificationId};

/// Four mine kinds enumerated in the spec § Minefield system table.
/// The RON loader rejects unknown enum values up-front via the
/// standard `serde` `#[serde(rename_all = "snake_case")]` shape.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MineKind {
    MineProximity = 0,
    MinePressure = 1,
    TripwireMine = 2,
    IedChain = 3,
}

impl MineKind {
    pub const ALL: [MineKind; 4] = [
        MineKind::MineProximity,
        MineKind::MinePressure,
        MineKind::TripwireMine,
        MineKind::IedChain,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MineKind::MineProximity => "mine_proximity",
            MineKind::MinePressure => "mine_pressure",
            MineKind::TripwireMine => "tripwire_mine",
            MineKind::IedChain => "ied_chain",
        }
    }

    /// Spec table HE yield in joules for the kind. IEDs configure per-
    /// mine yield separately (200..400 J range); the table value is
    /// the kind's documented baseline.
    #[must_use]
    pub const fn baseline_yield_joules(self) -> u32 {
        match self {
            MineKind::MineProximity => 80,
            MineKind::MinePressure => 120,
            MineKind::TripwireMine => 60,
            MineKind::IedChain => 200,
        }
    }

    /// Spec table blast radius in tiles for the kind.
    #[must_use]
    pub const fn baseline_blast_radius_tiles(self) -> u32 {
        match self {
            MineKind::MineProximity => 2,
            MineKind::MinePressure => 1,
            MineKind::TripwireMine => 1,
            MineKind::IedChain => 3,
        }
    }
}

// ---------------------------------------------------------------------
// Spec constants — pinned to the spec table verbatim.
// ---------------------------------------------------------------------

/// Spec § "Mine kinds": proximity mine triggers at 1.5-tile radius.
/// Stored in fixed-point tenths-of-tiles so the trigger predicate
/// stays integer-deterministic.
pub const MINE_PROXIMITY_TRIGGER_DECITILES: u32 = 15;
/// Spec § "Pressure mine" triggers directly over (1-tile blast radius;
/// the trigger itself is the actor-over-mine-tile predicate).
pub const MINE_PRESSURE_BLAST_RADIUS_TILES: u32 = 1;
/// Spec § "Minesweeper": 3-tile radius for proximity / tripwire.
pub const MINESWEEPER_DETECTION_PROXIMITY_TRIPWIRE_TILES: u32 = 3;
/// Spec § "Minesweeper": 2-tile radius for pressure / IED.
pub const MINESWEEPER_DETECTION_PRESSURE_IED_TILES: u32 = 2;
/// Spec § "Minesweeper": 2-second detection ping interval.
pub const MINESWEEPER_PING_SECONDS: u32 = 2;
/// Spec § "Manual disarm": 6 seconds crouched + adjacent + [E].
pub const MANUAL_DISARM_SECONDS: u32 = 6;
/// Spec § "Bomb-disposal robot": mechanical arm disarms in 4 seconds.
pub const ROBOT_DISARM_SECONDS: u32 = 4;
/// Spec § "Bomb-disposal robot": HP 1200 with reactive armor.
pub const BOMB_DISPOSAL_ROBOT_HP: u32 = 1200;
/// Spec § "Bomb-disposal robot": reactive armor absorbs 80% of HE.
pub const BOMB_DISPOSAL_ROBOT_ARMOR_REDUCTION_PERCENT: u32 = 80;
/// Spec § "Bomb-disposal robot": 40 px/s drive speed.
pub const BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND: u32 = 40;
/// Spec § "IED chain": cascade fires 100ms apart for the visual chain.
pub const IED_CHAIN_HOP_MILLIS: u32 = 100;
/// Spec § "IED chain": cascade completes within 0.5s of the trigger
/// event (per VAL-M9C-030). A linear chain of 5 IEDs at 100ms per hop
/// completes in 400ms (well inside the gate).
pub const IED_CHAIN_MAX_WINDOW_MILLIS: u32 = 500;
/// Spec § "IED chain": "BFS from trigger origin over the wire-link
/// graph". The DAG link distance for cascade propagation is 4 tiles
/// per the spec § "IED chain" table.
pub const IED_CHAIN_LINK_RANGE_TILES: u32 = 4;
/// Spec § "Manual disarm" recovered components: "the player gains 1
/// explosive component". Bound here so the cfctl handler can return
/// it as part of the disarm response.
pub const MINE_DISARMED_EXPLOSIVE_RECOVERED: u32 = 1;

/// Bitmask of which factions have detected the mine (driven by
/// minesweeper pings or by template author for player-owned mines).
/// The mine's enemy faction never sees the marker per spec § Notes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionMask {
    pub player: bool,
    pub enemy: bool,
    pub neutral: bool,
}

impl DetectionMask {
    /// True when the supplied faction has the detection bit set.
    #[must_use]
    pub const fn visible_to(self, faction: FortificationFaction) -> bool {
        match faction {
            FortificationFaction::Player => self.player,
            FortificationFaction::Enemy => self.enemy,
            FortificationFaction::Neutral => self.neutral,
        }
    }

    /// Set the supplied faction's detection bit; returns true if the
    /// bit actually flipped (was previously false).
    pub fn set(&mut self, faction: FortificationFaction) -> bool {
        let prev = self.visible_to(faction);
        match faction {
            FortificationFaction::Player => self.player = true,
            FortificationFaction::Enemy => self.enemy = true,
            FortificationFaction::Neutral => self.neutral = true,
        }
        !prev
    }

    /// Detection mask seeded so the owning faction can always see
    /// their own mines (template authors register the bit on placement).
    #[must_use]
    pub fn owner_only(owner: FortificationFaction) -> Self {
        let mut m = Self::default();
        m.set(owner);
        m
    }
}

/// Trigger source for a [`Mine`] instance. Used by
/// [`crate::minefield::evaluate_trigger`] to route the per-kind
/// predicate.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MineTriggerCause {
    /// Proximity radius check (1.5 tiles for mine_proximity, link
    /// range for IED in proximity mode).
    Proximity = 0,
    /// Stance:Standing or Crouched directly over the mine tile.
    Pressure = 1,
    /// Visible-line crossing (tripwire).
    Tripwire = 2,
    /// IED chain BFS cascade hop.
    IedChain = 3,
    /// Manual remote detonator (IED only).
    Manual = 4,
}

impl MineTriggerCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MineTriggerCause::Proximity => "proximity",
            MineTriggerCause::Pressure => "pressure",
            MineTriggerCause::Tripwire => "tripwire",
            MineTriggerCause::IedChain => "ied_chain",
            MineTriggerCause::Manual => "manual",
        }
    }
}

/// One placed mine instance. The kernel keeps mines as a flat list;
/// cf-control owns the per-world id allocation. Stored per spec §
/// Notes:
///
/// - Detection mask is per-faction (minesweeper flips the sweeping
///   faction bit; enemy faction never sees the marker).
/// - IED chain link graph is encoded as `wired_links` (the BFS
///   cascade walks this graph to schedule daisy-fires).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mine {
    pub id: FortificationId,
    pub kind: MineKind,
    pub pos_tiles: (i32, i32),
    /// Owning faction (registers detection bit on placement).
    pub owner: FortificationFaction,
    pub armed: bool,
    pub detection: DetectionMask,
    /// Configured HE yield (joules). Defaults to the kind's table
    /// baseline; IEDs can author 200..400 J per spec.
    pub yield_joules: u32,
    /// Configured blast radius (tiles). Defaults to the kind's table
    /// baseline.
    pub blast_radius_tiles: u32,
    /// Tripwire line endpoint for `TripwireMine`. The kernel uses this
    /// for the line-crossing trigger predicate. Both endpoints are in
    /// tile coordinates.
    #[serde(default)]
    pub tripwire_endpoints: Option<((i32, i32), (i32, i32))>,
    /// IED chain wired-link graph: list of mine ids physically wired
    /// to this IED. Always empty for non-IED kinds. The kernel does
    /// not validate that the linked ids actually exist; the caller
    /// (cf-control) is expected to register both endpoints of each
    /// link.
    #[serde(default)]
    pub wired_links: Vec<FortificationId>,
}

impl Mine {
    /// Build a fresh mine instance with the kind's table-baseline
    /// yield / radius.
    #[must_use]
    pub fn new(
        id: FortificationId,
        kind: MineKind,
        pos_tiles: (i32, i32),
        owner: FortificationFaction,
    ) -> Self {
        Self {
            id,
            kind,
            pos_tiles,
            owner,
            armed: true,
            detection: DetectionMask::owner_only(owner),
            yield_joules: kind.baseline_yield_joules(),
            blast_radius_tiles: kind.baseline_blast_radius_tiles(),
            tripwire_endpoints: None,
            wired_links: Vec::new(),
        }
    }

    /// True when the mine has been disarmed / detonated and is no
    /// longer triggerable.
    #[must_use]
    pub const fn is_inactive(&self) -> bool {
        !self.armed
    }

    /// Mark the mine as no longer armed (post-detonation OR
    /// post-disarm). Callers (cf-control) remove the mine from the
    /// world after this.
    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

/// Replay-event-payload shape for `fortification.mine_armed`. cf-control
/// fans out one per mine placed by `act.player.deploy_minefield_template`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MineArmedEvent {
    pub mine_id: FortificationId,
    pub mine_kind: MineKind,
    pub pos: (i32, i32),
    pub tick_index: u64,
}

/// Replay-event-payload shape for `fortification.mine_triggered`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MineTriggeredEvent {
    pub mine_id: FortificationId,
    pub trigger_kind: MineTriggerCause,
    pub yield_joules: u32,
    pub blast_radius_tiles: u32,
    pub tick_index: u64,
}

/// Replay-event-payload shape for the M14J cookoff intermediary that
/// the IED chain emits between successive `mine_triggered` records.
/// Per VAL-M9C-IED-COOKOFF: "the event stream between successive
/// mine_triggered events of trigger_kind=ied_chain contains M14J
/// cookoff-emitted intermediary events (e.g. cookoff.charge_initiated)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IedCookoffEvent {
    pub bridging_mine_id: FortificationId,
    pub next_mine_id: FortificationId,
    pub kind: IedCookoffKind,
    pub tick_index: u64,
}

/// Sub-kind of [`IedCookoffEvent`]. Mirrors the M14J event taxonomy
/// referenced in VAL-M9C-IED-COOKOFF.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IedCookoffKind {
    /// `cookoff.charge_initiated`: the bridging IED's cookoff window
    /// begins (always emitted between adjacent hops).
    ChargeInitiated = 0,
    /// `cookoff.fragmentation_emitted`: the bridging IED throws
    /// fragmentation that contributes to the chain timing.
    FragmentationEmitted = 1,
}

impl IedCookoffKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            IedCookoffKind::ChargeInitiated => "cookoff.charge_initiated",
            IedCookoffKind::FragmentationEmitted => "cookoff.fragmentation_emitted",
        }
    }
}

/// Replay-event-payload shape for `fortification.minesweeper_detected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinesweeperDetectedEvent {
    pub mine_id: FortificationId,
    pub sweeper_id: u64,
    pub sweeper_faction: FortificationFaction,
    pub tick_index: u64,
}

/// Replay-event-payload shape for `fortification.mine_disarmed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MineDisarmedEvent {
    pub mine_id: FortificationId,
    pub actor_id: Option<u64>,
    pub result: DisarmResult,
    pub failure_cause: Option<DisarmFailureCause>,
    pub explosive_recovered: u32,
    pub tick_index: u64,
}

/// Result enum mirroring the `mine_disarmed.json` schema:
/// `ok | failed`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisarmResult {
    Ok = 0,
    Failed = 1,
}

impl DisarmResult {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DisarmResult::Ok => "ok",
            DisarmResult::Failed => "failed",
        }
    }
}

/// Failure-cause enum mirroring the `mine_disarmed.json` schema:
/// `actor_moved | actor_damaged | actor_released_e | interrupted_other`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisarmFailureCause {
    ActorMoved = 0,
    ActorDamaged = 1,
    ActorReleasedE = 2,
    InterruptedOther = 3,
}

impl DisarmFailureCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DisarmFailureCause::ActorMoved => "actor_moved",
            DisarmFailureCause::ActorDamaged => "actor_damaged",
            DisarmFailureCause::ActorReleasedE => "actor_released_e",
            DisarmFailureCause::InterruptedOther => "interrupted_other",
        }
    }
}
