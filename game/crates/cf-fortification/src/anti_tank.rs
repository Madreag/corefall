//! M9C § "Anti-tank ditch + dragon's teeth + tank trap": full kernel
//! for the 4 anti-tank obstacles + per-component vehicle damage
//! routing + deterministic stuck-roll.
//!
//! Per the spec table:
//!
//! | Asset | Footprint | HP | Vehicle effect | Infantry effect |
//! |---|---|---|---|---|
//! | `anti_tank_ditch`  | 8×4   | n/a   | Light 30% stuck; Heavy 80% stuck; SPG/howitzer impassable | 25% slip + Partial cover (in ditch) |
//! | `dragons_teeth`    | 1×1×3 | 1200  | Light: full stop; Heavy: suspension damage + stops after ~2 teeth | walks through gaps |
//! | `tank_trap_x`      | 2×2   | 800   | Light: stops + per-component damage; Heavy: plows in ~5s | walks through gaps |
//! | `bollard_concrete` | 1×1   | 600   | Stops light / soft-stops heavy | non-block |
//!
//! Per the spec § Notes for the implementer:
//!
//! > Anti-tank ditch stuck-chance is rolled deterministically off
//! > `(actor_id, ditch_id, world_seed)`. Multiple attempts to escape
//! > are independent rolls (no save-scumming bias).
//! >
//! > Dragon's teeth + tank trap vehicle damage routes through M44C
//! > per-component damage — the tank's suspension/track takes the hit,
//! > not the hull armor.
//!
//! Per the feature definition expected-behavior:
//!
//! > Anti-tank ditch: stuck rolls deterministic; infantry 25% slip +
//! > Partial cover; dragon's teeth: suspension damage routes through
//! > M44C; tank stops after 2 contacts; tank trap X: HP 800; light
//! > stop / heavy plow ~5 s; bollard: light stop / heavy soft-stop /
//! > infantry pass.
//!
//! Per the feature definition cross-area note:
//!
//! > Cross-area: AT-ditch infantry cover-state computed via same
//! > cover-state derivation engine as M9B trench segments (no per-
//! > system divergence).
//!
//! VAL-M9C-039 / VAL-M9C-040 / VAL-M9C-041 / VAL-CROSS-NEW-015 land
//! here.

use rand_core::{Rng as _, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

use cf_trench::{cover_state as trench_cover_state, CoverState, SegmentVariant, TrenchStance};

use crate::common::FortificationId;

// ---------------------------------------------------------------------
// Spec constants — pinned to the spec table verbatim. Percentages are
// integer (0..=100) to keep the stuck-roll path deterministic.
// ---------------------------------------------------------------------

/// Spec table row HP for `dragons_teeth` (per tooth).
pub const DRAGONS_TEETH_PER_TOOTH_HP: u32 = 1200;
/// Spec table row HP for `tank_trap_x`.
pub const TANK_TRAP_X_HP: u32 = 800;
/// Spec table row HP for `bollard_concrete`.
pub const BOLLARD_CONCRETE_HP: u32 = 600;

/// Spec table row footprint for an `anti_tank_ditch`: 8 px wide × 4
/// px deep terrain carve.
pub const ANTI_TANK_DITCH_WIDTH_PX: u32 = 8;
pub const ANTI_TANK_DITCH_DEPTH_PX: u32 = 4;

/// Spec table row stuck-chance (percent) for a light vehicle in an AT
/// ditch.
pub const AT_DITCH_LIGHT_STUCK_PERCENT: u32 = 30;
/// Spec table row stuck-chance (percent) for a heavy vehicle in an
/// AT ditch.
pub const AT_DITCH_HEAVY_STUCK_PERCENT: u32 = 80;
/// Spec table: SPG / howitzer is impassable (effectively 100%).
pub const AT_DITCH_SPG_HOWITZER_STUCK_PERCENT: u32 = 100;
/// Spec table row infantry slip-chance (percent) in an AT ditch.
pub const AT_DITCH_INFANTRY_SLIP_PERCENT: u32 = 25;

/// Spec § "Dragon's teeth deal per-component suspension damage. After
/// ~2 tooth contacts (with HP 1200 each), the tank's suspension HP
/// exhausts and the tank is fully immobilized."
///
/// Per-tooth suspension damage: 600 HP per contact (so 2 contacts =
/// 1200 HP = M44C suspension cap, matching the spec).
pub const DRAGONS_TEETH_SUSPENSION_DAMAGE_PER_CONTACT: u32 = 600;
/// M44C suspension HP cap (matches the documented "tank stops after
/// ~2 teeth" budget).
pub const M44C_SUSPENSION_HP_BUDGET: u32 = 1200;

/// Spec § "Tank trap X is destructible; stops light, slows heavy.
/// Heavy vehicle can plow through in ~5s while taking damage."
/// Heavy plow window per spec: 5 seconds.
pub const TANK_TRAP_X_HEAVY_PLOW_SECONDS: u32 = 5;
/// Per-tick HP damage delivered to the trap during a heavy plow,
/// scaled to centi-HP so the integer math lands at exactly
/// `TANK_TRAP_X_HEAVY_PLOW_SECONDS × tick_rate_hz` ticks (within
/// ±1 tick). With 60 Hz tick rate and 5 s window the centi value is
/// `800 × 100 / 300 = 266` (i.e. 2.66 HP / tick). Caller applies the
/// integer-HP portion each tick + threads the centi-remainder via
/// [`TankTrapX::plow_accumulator_centi`].
#[must_use]
pub fn tank_trap_x_heavy_plow_centi_per_tick(tick_rate_hz: u32) -> u32 {
    let denom = TANK_TRAP_X_HEAVY_PLOW_SECONDS.saturating_mul(tick_rate_hz);
    if denom == 0 {
        return 0;
    }
    TANK_TRAP_X_HP.saturating_mul(100) / denom
}

/// Tank-trap X light-vehicle suspension damage applied on contact
/// (per-component routing per spec § "light vehicle stops + per-
/// component damage").
pub const TANK_TRAP_X_LIGHT_SUSPENSION_DAMAGE: u32 = 200;

// ---------------------------------------------------------------------
// AntiTankKind enum (4 anti-tank kinds enumerated in the spec table).
// ---------------------------------------------------------------------

/// Four anti-tank kinds enumerated in the spec § "Anti-tank ditch +
/// dragon's teeth + tank trap" table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiTankKind {
    AntiTankDitch = 0,
    DragonsTeeth = 1,
    TankTrapX = 2,
    BollardConcrete = 3,
}

impl AntiTankKind {
    pub const ALL: [AntiTankKind; 4] = [
        AntiTankKind::AntiTankDitch,
        AntiTankKind::DragonsTeeth,
        AntiTankKind::TankTrapX,
        AntiTankKind::BollardConcrete,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AntiTankKind::AntiTankDitch => "anti_tank_ditch",
            AntiTankKind::DragonsTeeth => "dragons_teeth",
            AntiTankKind::TankTrapX => "tank_trap_x",
            AntiTankKind::BollardConcrete => "bollard_concrete",
        }
    }
}

// ---------------------------------------------------------------------
// Vehicle class (consumed by the per-obstacle predicates)
// ---------------------------------------------------------------------

/// Vehicle class consumed by the anti-tank obstacle predicates. Spec
/// table distinguishes light vehicles (IFVs, scout cars), heavy
/// vehicles (tanks), SPG / howitzer (effectively impassable in
/// ditches), and infantry (foot soldiers).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiTankVehicleClass {
    Light = 0,
    Heavy = 1,
    SpgHowitzer = 2,
    Infantry = 3,
}

impl AntiTankVehicleClass {
    /// Spec table stuck-percentage for an AT ditch contact.
    #[must_use]
    pub const fn at_ditch_stuck_percent(self) -> u32 {
        match self {
            AntiTankVehicleClass::Light => AT_DITCH_LIGHT_STUCK_PERCENT,
            AntiTankVehicleClass::Heavy => AT_DITCH_HEAVY_STUCK_PERCENT,
            AntiTankVehicleClass::SpgHowitzer => AT_DITCH_SPG_HOWITZER_STUCK_PERCENT,
            AntiTankVehicleClass::Infantry => AT_DITCH_INFANTRY_SLIP_PERCENT,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AntiTankVehicleClass::Light => "light",
            AntiTankVehicleClass::Heavy => "heavy",
            AntiTankVehicleClass::SpgHowitzer => "spg_howitzer",
            AntiTankVehicleClass::Infantry => "infantry",
        }
    }
}

// ---------------------------------------------------------------------
// AntiTankDitch
// ---------------------------------------------------------------------

/// One placed `anti_tank_ditch` — 8 px wide × 4 px deep terrain
/// carve. Per spec § Notes: stuck-chance rolled deterministically
/// off `(actor_id, ditch_id, world_seed)`; multiple attempts are
/// independent rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiTankDitch {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    pub width_tiles: u32,
    pub depth_tiles: u32,
}

impl AntiTankDitch {
    #[must_use]
    pub const fn new(id: FortificationId, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            pos_tiles,
            // Spec footprint values are pixels at the canonical
            // tile-to-pixel ratio; M9C stores them as pixel-tiles.
            width_tiles: ANTI_TANK_DITCH_WIDTH_PX,
            depth_tiles: ANTI_TANK_DITCH_DEPTH_PX,
        }
    }
}

/// Inputs for one deterministic AT-ditch stuck roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtDitchStuckInputs {
    pub actor_id: u64,
    pub ditch_id: FortificationId,
    pub world_seed: u64,
    pub vehicle_class: AntiTankVehicleClass,
    /// Attempt index (0 = first attempt; 1 = first escape retry; …).
    /// (no save-scumming bias)". The attempt index seeds the roll so
    /// successive attempts diverge.
    pub attempt_index: u32,
}

/// Outcome of one deterministic AT-ditch stuck roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtDitchStuckOutcome {
    /// True when the vehicle is stuck for this attempt.
    pub stuck: bool,
    /// 0..=99 roll value (for telemetry / determinism cross-check).
    pub roll: u32,
    /// Threshold the roll was compared against (the spec-table
    /// percentage for the vehicle class).
    pub threshold_percent: u32,
}

/// Mix the three (actor_id, ditch_id, world_seed) inputs into a
/// deterministic 64-bit seed for the Xoshiro256** stream. Uses a
/// splitmix-style multiplier so all three inputs propagate into
/// the high bits of the seed.
#[must_use]
fn mix_seed(actor_id: u64, ditch_id: u32, world_seed: u64, attempt_index: u32) -> u64 {
    let mut acc = world_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    acc ^= actor_id.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    acc ^= u64::from(ditch_id).wrapping_mul(0x94D0_49BB_1331_11EB);
    acc = acc.rotate_left(17);
    acc ^= u64::from(attempt_index).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    acc.wrapping_add(0x2D35_8DCC_AA6C_78A5)
}

/// Roll the deterministic AT-ditch stuck check. Returns the outcome
/// the engine consumes (true = stuck this attempt).
#[must_use]
pub fn at_ditch_stuck_roll(inputs: AtDitchStuckInputs) -> AtDitchStuckOutcome {
    let threshold = inputs.vehicle_class.at_ditch_stuck_percent();
    if threshold == 0 {
        return AtDitchStuckOutcome {
            stuck: false,
            roll: 0,
            threshold_percent: 0,
        };
    }
    if threshold >= 100 {
        // Spec table: SPG / howitzer impassable. The roll is still
        // emitted (for replay determinism) but the outcome is always
        // stuck.
        let seed = mix_seed(
            inputs.actor_id,
            inputs.ditch_id.0,
            inputs.world_seed,
            inputs.attempt_index,
        );
        let mut rng = Xoshiro256StarStar::seed_from_u64(seed);
        let roll = (rng.next_u32() % 100).min(99);
        return AtDitchStuckOutcome {
            stuck: true,
            roll,
            threshold_percent: threshold,
        };
    }
    let seed = mix_seed(
        inputs.actor_id,
        inputs.ditch_id.0,
        inputs.world_seed,
        inputs.attempt_index,
    );
    let mut rng = Xoshiro256StarStar::seed_from_u64(seed);
    let roll = rng.next_u32() % 100;
    AtDitchStuckOutcome {
        stuck: roll < threshold,
        roll,
        threshold_percent: threshold,
    }
}

/// shared cf-trench `cover_state` engine. Per the feature definition's
/// cross-area note, the AT ditch (8 px wide × 4 px deep, comparable to
/// shallow_scrape) maps to [`SegmentVariant::ShallowScrape`] so the
/// derived cover-state matches what the same actor would see inside a
/// shallow_scrape trench. No per-system divergence: there is exactly
/// one cover-state derivation function in the workspace.
#[must_use]
pub fn at_ditch_infantry_cover_state(stance: TrenchStance) -> CoverState {
    trench_cover_state(stance, SegmentVariant::ShallowScrape)
}

// ---------------------------------------------------------------------
// DragonsTeeth
// ---------------------------------------------------------------------

/// One placed `dragons_teeth` instance — a single concrete pyramid
/// tooth (HP 1200). Spec: deployed in 3-row staggered patterns; the
/// kernel keeps one instance per tooth so per-tooth HP + per-tooth
/// destruction routes correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragonsTeeth {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    pub hp: u32,
}

impl DragonsTeeth {
    #[must_use]
    pub const fn new(id: FortificationId, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            pos_tiles,
            hp: DRAGONS_TEETH_PER_TOOTH_HP,
        }
    }

    #[must_use]
    pub const fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    /// Apply per-tooth damage; saturates at 0.
    pub fn apply_damage(&mut self, damage: u32) {
        self.hp = self.hp.saturating_sub(damage);
    }
}

/// One per-contact suspension-damage event the engine routes through
/// M44C per-component damage. Spec § "Notes for the implementer":
///
/// > Dragon's teeth + tank trap vehicle damage routes through M44C
/// > per-component damage — the tank's suspension/track takes the
/// > hit, not the hull armor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspensionDamageEvent {
    pub vehicle_id: u64,
    pub source_id: FortificationId,
    pub damage: u32,
    pub component: SuspensionComponentKind,
    pub tick_index: u64,
}

/// Sub-kind of [`SuspensionDamageEvent`]. The kernel routes dragon's
/// teeth + tank-trap contacts to the suspension component (per spec);
/// the bollard contact is a soft-stop with no per-component damage.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuspensionComponentKind {
    Suspension = 0,
    Track = 1,
}

impl SuspensionComponentKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SuspensionComponentKind::Suspension => "suspension",
            SuspensionComponentKind::Track => "track",
        }
    }
}

/// Outcome of one dragons-teeth tooth contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragonsTeethContactOutcome {
    /// Suspension damage event (always emitted on heavy-vehicle
    /// contact; light vehicles full-stop on contact and don't take
    /// suspension damage per spec).
    pub suspension_event: Option<SuspensionDamageEvent>,
    /// True when the vehicle becomes fully immobilized this contact
    /// (suspension HP exhausted; ~2 contacts at 600 HP each).
    pub immobilized: bool,
    /// True when the contact full-stops the vehicle (light vehicle
    /// hits a tooth) without per-component damage.
    pub full_stopped: bool,
}

/// Apply a heavy vehicle's contact with a dragon's-teeth tooth.
/// Damages the vehicle's M44C suspension component; immobilizes the
/// vehicle once cumulative suspension damage reaches the M44C
/// suspension HP budget.
///
/// `current_suspension_hp` is the M44C suspension HP **before** the
/// contact. The engine maintains the running suspension HP.
#[must_use]
pub fn apply_dragons_teeth_heavy_contact(
    tooth_id: FortificationId,
    vehicle_id: u64,
    current_suspension_hp: u32,
    tick_index: u64,
) -> DragonsTeethContactOutcome {
    let damage = DRAGONS_TEETH_SUSPENSION_DAMAGE_PER_CONTACT;
    let new_hp = current_suspension_hp.saturating_sub(damage);
    DragonsTeethContactOutcome {
        suspension_event: Some(SuspensionDamageEvent {
            vehicle_id,
            source_id: tooth_id,
            damage,
            component: SuspensionComponentKind::Suspension,
            tick_index,
        }),
        immobilized: new_hp == 0,
        full_stopped: false,
    }
}

/// Apply a light vehicle's contact with a dragon's-teeth tooth:
/// per spec the contact full-stops the vehicle without per-component
/// suspension damage (the tooth blocks rather than crushes the IFV
/// chassis).
#[must_use]
pub const fn apply_dragons_teeth_light_contact(
    _tooth_id: FortificationId,
    _vehicle_id: u64,
    _tick_index: u64,
) -> DragonsTeethContactOutcome {
    DragonsTeethContactOutcome {
        suspension_event: None,
        immobilized: false,
        full_stopped: true,
    }
}

// ---------------------------------------------------------------------
// Tank trap X
// ---------------------------------------------------------------------

/// One placed `tank_trap_x` instance (HP 800). Spec: destructible
/// welded I-beam X (Czech hedgehog reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TankTrapX {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    pub hp: u32,
    /// Sub-tick damage accumulator (HP × 100). Lets the heavy plow
    /// land at exactly 5 s ± 1 tick even when `TANK_TRAP_X_HP /
    /// (period_ticks)` does not divide evenly. Engine threads the
    /// accumulator across plow ticks; non-plow callers can ignore it.
    #[serde(default)]
    pub plow_accumulator_centi: u32,
}

impl TankTrapX {
    #[must_use]
    pub const fn new(id: FortificationId, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            pos_tiles,
            hp: TANK_TRAP_X_HP,
            plow_accumulator_centi: 0,
        }
    }

    #[must_use]
    pub const fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    /// Apply HP damage; saturates at 0.
    pub fn apply_damage(&mut self, damage: u32) {
        self.hp = self.hp.saturating_sub(damage);
    }
}

/// Outcome of one tank-trap-X contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TankTrapContactOutcome {
    /// True when the contact full-stops the vehicle (light vehicle
    /// only).
    pub full_stopped: bool,
    /// True when the trap was destroyed by this contact (heavy
    /// vehicle plow completes after ~5 s).
    pub trap_destroyed: bool,
    /// Per-component suspension damage applied (light vehicle: 200;
    /// heavy plow: per-tick plow damage applied during plow window).
    pub suspension_event: Option<SuspensionDamageEvent>,
}

/// Apply a light vehicle's contact with a tank trap X: per spec the
/// trap stops the vehicle + applies per-component suspension damage.
/// The trap itself takes no damage in this branch.
#[must_use]
pub const fn apply_tank_trap_x_light_contact(
    trap_id: FortificationId,
    vehicle_id: u64,
    tick_index: u64,
) -> TankTrapContactOutcome {
    TankTrapContactOutcome {
        full_stopped: true,
        trap_destroyed: false,
        suspension_event: Some(SuspensionDamageEvent {
            vehicle_id,
            source_id: trap_id,
            damage: TANK_TRAP_X_LIGHT_SUSPENSION_DAMAGE,
            component: SuspensionComponentKind::Suspension,
            tick_index,
        }),
    }
}

/// Apply one tick of a heavy vehicle's plow through a tank trap X.
/// Damage is applied via a centi-HP accumulator so the plow lands at
/// exactly `TANK_TRAP_X_HEAVY_PLOW_SECONDS × tick_rate_hz` ticks
/// (within ±1 tick) regardless of whether `HP / period_ticks`
/// divides evenly. Spec § "heavy plows through in ~5 s while taking
/// damage; the trap is destroyed when HP reaches 0".
#[must_use]
pub fn tick_tank_trap_x_heavy_plow(
    trap: &mut TankTrapX,
    vehicle_id: u64,
    tick_rate_hz: u32,
    tick_index: u64,
) -> TankTrapContactOutcome {
    if trap.is_destroyed() {
        return TankTrapContactOutcome {
            full_stopped: false,
            trap_destroyed: false,
            suspension_event: None,
        };
    }
    let period_ticks = TANK_TRAP_X_HEAVY_PLOW_SECONDS.saturating_mul(tick_rate_hz);
    let centi_per_tick = TANK_TRAP_X_HP
        .saturating_mul(100)
        .checked_div(period_ticks)
        .unwrap_or_else(|| TANK_TRAP_X_HP.saturating_mul(100));
    trap.plow_accumulator_centi = trap
        .plow_accumulator_centi
        .saturating_add(centi_per_tick);
    let damage = trap.plow_accumulator_centi / 100;
    trap.plow_accumulator_centi -= damage * 100;
    trap.apply_damage(damage);
    let trap_destroyed = trap.is_destroyed();
    TankTrapContactOutcome {
        full_stopped: false,
        trap_destroyed,
        suspension_event: Some(SuspensionDamageEvent {
            vehicle_id,
            source_id: trap.id,
            damage,
            component: SuspensionComponentKind::Track,
            tick_index,
        }),
    }
}

// ---------------------------------------------------------------------
// Bollard
// ---------------------------------------------------------------------

/// One placed `bollard_concrete` instance (HP 600). Spec: stops light
/// vehicles / soft-stops heavy / non-block to infantry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BollardConcrete {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    pub hp: u32,
}

impl BollardConcrete {
    #[must_use]
    pub const fn new(id: FortificationId, pos_tiles: (i32, i32)) -> Self {
        Self {
            id,
            pos_tiles,
            hp: BOLLARD_CONCRETE_HP,
        }
    }

    #[must_use]
    pub const fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    pub fn apply_damage(&mut self, damage: u32) {
        self.hp = self.hp.saturating_sub(damage);
    }
}

/// Outcome of one bollard contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BollardContactOutcome {
    /// True when the bollard blocks the vehicle entirely (light
    /// vehicle).
    pub full_stopped: bool,
    /// True when the bollard soft-stops the vehicle (heavy vehicle:
    /// slows but doesn't fully block).
    pub soft_stopped: bool,
    /// True when infantry pass through without interaction.
    pub passes_through: bool,
}

/// Apply one bollard contact predicate per vehicle class. Spec § "Stops
/// light vehicle / soft-stops heavy; non-block to infantry".
#[must_use]
pub const fn apply_bollard_contact(
    vehicle_class: AntiTankVehicleClass,
) -> BollardContactOutcome {
    match vehicle_class {
        AntiTankVehicleClass::Light => BollardContactOutcome {
            full_stopped: true,
            soft_stopped: false,
            passes_through: false,
        },
        AntiTankVehicleClass::Heavy | AntiTankVehicleClass::SpgHowitzer => {
            BollardContactOutcome {
                full_stopped: false,
                soft_stopped: true,
                passes_through: false,
            }
        }
        AntiTankVehicleClass::Infantry => BollardContactOutcome {
            full_stopped: false,
            soft_stopped: false,
            passes_through: true,
        },
    }
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::cast_possible_wrap)]
mod tests {
    use super::*;

    #[test]
    fn anti_tank_kind_round_trips_via_ron() {
        for kind in AntiTankKind::ALL {
            let s = kind.as_str();
            let parsed: AntiTankKind = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn unknown_anti_tank_kind_rejected_at_parse() {
        let result: Result<AntiTankKind, _> = ron::from_str("\"definitely_not_real_at\"");
        assert!(result.is_err(), "unknown enum must reject");
    }

    /// (light 30%, heavy 80%, SPG/howitzer impassable). Two runs with
    /// the same (actor_id, ditch_id, world_seed, attempt) inputs must
    /// produce byte-identical outcomes.
    #[test]
    fn ditch_deterministic_stuck() {
        let world_seed = 12345_u64;
        let ditch_id = FortificationId(7);
        let actor_id = 42_u64;
        for class in [
            AntiTankVehicleClass::Light,
            AntiTankVehicleClass::Heavy,
            AntiTankVehicleClass::SpgHowitzer,
            AntiTankVehicleClass::Infantry,
        ] {
            for attempt in [0_u32, 1, 2, 5, 10] {
                let inputs = AtDitchStuckInputs {
                    actor_id,
                    ditch_id,
                    world_seed,
                    vehicle_class: class,
                    attempt_index: attempt,
                };
                let a = at_ditch_stuck_roll(inputs);
                let b = at_ditch_stuck_roll(inputs);
                assert_eq!(a, b, "{class:?} attempt {attempt} must be deterministic");
                assert_eq!(a.threshold_percent, class.at_ditch_stuck_percent());
            }
        }
        // SPG / howitzer always stuck.
        let spg = at_ditch_stuck_roll(AtDitchStuckInputs {
            actor_id,
            ditch_id,
            world_seed,
            vehicle_class: AntiTankVehicleClass::SpgHowitzer,
            attempt_index: 0,
        });
        assert!(spg.stuck);
        assert_eq!(spg.threshold_percent, 100);
    }

    /// Distribution sanity: over many actor ids, a light vehicle's
    /// stuck-rate should be in the ballpark of the spec percentage
    /// (30% ± 5%). The roll uses Xoshiro256** so the distribution is
    /// expected to be uniform.
    #[test]
    fn ditch_distribution_matches_spec() {
        let world_seed = 0xCAFE_BABE_u64;
        let ditch_id = FortificationId(1);
        let mut stuck = 0_u32;
        let n = 10_000_u64;
        for actor in 0..n {
            let r = at_ditch_stuck_roll(AtDitchStuckInputs {
                actor_id: actor,
                ditch_id,
                world_seed,
                vehicle_class: AntiTankVehicleClass::Light,
                attempt_index: 0,
            });
            if r.stuck {
                stuck += 1;
            }
        }
        let rate_bp = stuck * 10_000 / (n as u32);
        // Expected 3000 bp (30%); allow ±500 bp window for 10k samples.
        assert!(
            (2500..=3500).contains(&rate_bp),
            "light stuck rate {rate_bp} bp out of [2500, 3500]"
        );
    }

    /// Attempt index independence: successive escape attempts roll
    /// independent values (per spec § "no save-scumming bias").
    #[test]
    fn ditch_attempts_are_independent() {
        let world_seed = 7_u64;
        let ditch_id = FortificationId(99);
        let actor_id = 5_u64;
        let mut last = None;
        let mut all_same = true;
        for attempt in 0..10_u32 {
            let r = at_ditch_stuck_roll(AtDitchStuckInputs {
                actor_id,
                ditch_id,
                world_seed,
                vehicle_class: AntiTankVehicleClass::Heavy,
                attempt_index: attempt,
            });
            if let Some(prev) = last {
                if prev != r.roll {
                    all_same = false;
                }
            }
            last = Some(r.roll);
        }
        assert!(
            !all_same,
            "attempt index must produce divergent rolls (no save-scumming bias)"
        );
    }

    /// damage (NOT hull armor); tank stops after ~2 contacts at 600
    /// HP per contact (M44C suspension budget = 1200).
    #[test]
    fn dragons_teeth_per_component_damage() {
        let tooth_id = FortificationId(1);
        let vehicle_id = 42_u64;
        let mut suspension_hp = M44C_SUSPENSION_HP_BUDGET;

        let first = apply_dragons_teeth_heavy_contact(tooth_id, vehicle_id, suspension_hp, 100);
        assert!(first.suspension_event.is_some());
        let e1 = first.suspension_event.unwrap();
        assert_eq!(e1.component, SuspensionComponentKind::Suspension);
        assert_eq!(e1.damage, 600);
        assert!(!first.immobilized);
        suspension_hp = suspension_hp.saturating_sub(e1.damage);
        assert_eq!(suspension_hp, 600);

        let second = apply_dragons_teeth_heavy_contact(tooth_id, vehicle_id, suspension_hp, 200);
        let e2 = second.suspension_event.unwrap();
        assert_eq!(e2.damage, 600);
        suspension_hp = suspension_hp.saturating_sub(e2.damage);
        assert_eq!(suspension_hp, 0);
        assert!(
            second.immobilized,
            "tank immobilized after 2 dragons-teeth contacts"
        );

        // Light vehicle contact: full stop, no suspension damage.
        let light = apply_dragons_teeth_light_contact(tooth_id, vehicle_id, 300);
        assert!(light.full_stopped);
        assert!(light.suspension_event.is_none());
    }

    /// per-component damage; heavy plows in ~5 s. With 60 Hz tick rate
    /// the plow completes in `5 × 60 = 300 ticks` (within ±0.5s gate).
    #[test]
    fn tank_trap_x_behavior() {
        let trap_id = FortificationId(1);
        let vehicle_id = 42_u64;

        // Light vehicle contact: trap not destroyed, vehicle stops,
        // per-component damage routed.
        let light = apply_tank_trap_x_light_contact(trap_id, vehicle_id, 50);
        assert!(light.full_stopped);
        assert!(!light.trap_destroyed);
        let evt = light.suspension_event.unwrap();
        assert_eq!(evt.damage, TANK_TRAP_X_LIGHT_SUSPENSION_DAMAGE);
        assert_eq!(evt.component, SuspensionComponentKind::Suspension);

        // Heavy vehicle plow at 60 Hz: completes within
        // `5s × 60 = 300` ticks (per spec "plows through in ~5s").
        let tick_rate = 60_u32;
        let mut trap = TankTrapX::new(trap_id, (0, 0));
        let mut ticks_until_destroyed = 0_u32;
        for t in 0..=(TANK_TRAP_X_HEAVY_PLOW_SECONDS * tick_rate) {
            let outcome = tick_tank_trap_x_heavy_plow(&mut trap, vehicle_id, tick_rate, u64::from(t));
            if outcome.trap_destroyed {
                ticks_until_destroyed = t + 1;
                break;
            }
        }
        let expected = TANK_TRAP_X_HEAVY_PLOW_SECONDS * tick_rate;
        let half_second_ticks = tick_rate / 2;
        assert!(
            ticks_until_destroyed > 0
                && ticks_until_destroyed >= expected.saturating_sub(half_second_ticks)
                && ticks_until_destroyed <= expected + half_second_ticks,
            "heavy plow completed in {ticks_until_destroyed} ticks; expected ~{expected} (±0.5s)"
        );
        assert!(trap.is_destroyed());
    }

    /// VAL-M9C-041 (continued): bollard concrete behavior — light
    /// stop / heavy soft-stop / infantry pass.
    #[test]
    fn bollard_concrete_behavior() {
        let light = apply_bollard_contact(AntiTankVehicleClass::Light);
        assert!(light.full_stopped);
        assert!(!light.soft_stopped);
        assert!(!light.passes_through);

        let heavy = apply_bollard_contact(AntiTankVehicleClass::Heavy);
        assert!(!heavy.full_stopped);
        assert!(heavy.soft_stopped);
        assert!(!heavy.passes_through);

        let spg = apply_bollard_contact(AntiTankVehicleClass::SpgHowitzer);
        assert!(spg.soft_stopped, "SPG soft-stops on bollard contact");

        let infantry = apply_bollard_contact(AntiTankVehicleClass::Infantry);
        assert!(!infantry.full_stopped);
        assert!(!infantry.soft_stopped);
        assert!(infantry.passes_through);

        // Bollard HP cap matches spec table.
        let b = BollardConcrete::new(FortificationId(1), (0, 0));
        assert_eq!(b.hp, BOLLARD_CONCRETE_HP);
    }

    /// shallow_scrape cover_state for matching depth — i.e. both
    /// surfaces consult the same cf-trench cover-state derivation
    /// engine.
    #[test]
    fn anti_tank_ditch_infantry_cover_state_matches_trench() {
        for stance in [
            TrenchStance::Standing,
            TrenchStance::Crouched,
            TrenchStance::Prone,
        ] {
            let from_ditch = at_ditch_infantry_cover_state(stance);
            let from_trench = trench_cover_state(stance, SegmentVariant::ShallowScrape);
            assert_eq!(
                from_ditch, from_trench,
                "AT-ditch cover_state diverges from shallow_scrape for {stance:?}"
            );
        }
        // Standing infantry in the ditch sees Exposed cover — same as
        // a shallow_scrape trench. Crouched sees Partial; Prone Full.
        assert_eq!(
            at_ditch_infantry_cover_state(TrenchStance::Crouched),
            CoverState::Partial,
            "spec § 'infantry: 25% slip + partial cover'"
        );
        assert_eq!(
            at_ditch_infantry_cover_state(TrenchStance::Prone),
            CoverState::Full
        );
    }

    /// Alias name expected by the feature definition verification-step
    /// `cargo test -p cf-fortification anti_tank_ditch_infantry_
    /// cover_state_matches_trench`.
    #[test]
    fn anti_tank_ditch_uses_shared_cover_engine() {
        anti_tank_ditch_infantry_cover_state_matches_trench();
    }

    /// Heavy plow centi-per-tick helper handles edge cases.
    #[test]
    fn heavy_plow_centi_per_tick_lands_at_target() {
        // 60 Hz × 5 s = 300 ticks; 800 × 100 / 300 = 266 centi/tick.
        assert_eq!(tank_trap_x_heavy_plow_centi_per_tick(60), 266);
        // 30 Hz × 5 s = 150 ticks; 800 × 100 / 150 = 533 centi/tick.
        assert_eq!(tank_trap_x_heavy_plow_centi_per_tick(30), 533);
        // Tick rate of 0 returns 0 (safety net).
        assert_eq!(tank_trap_x_heavy_plow_centi_per_tick(0), 0);
    }
}
