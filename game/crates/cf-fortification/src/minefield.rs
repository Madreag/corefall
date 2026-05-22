//! M9C § "Minefield system (4 mine kinds + minesweeper + bomb
//! disposal)": kernel facade for the four mine kinds, IED chain
//! cascade, minesweeper detection-mask, manual / robot disarm.
//!
//! Per the spec table:
//!
//! | Mine kind | Trigger | Yield | Detection |
//! |---|---|---|---|
//! | `mine_proximity` | hostile within 1.5 tiles | 80 J HE | minesweeper within 3 tiles |
//! | `mine_pressure` | Standing/Crouched over tile | 120 J HE | minesweeper at 2 tiles |
//! | `tripwire_mine` | actor crosses tripwire line | 60 J HE + alarm.tripwire_triggered | tripwire visible at 1-tile LOS |
//! | `ied_chain` | remote OR proximity OR pressure | 200..400 J HE, daisy-fires linked IEDs | minesweeper at 2 tiles |
//!
//! Per the spec § Notes for the implementer:
//!
//! > **IED chain** detonation order is `BFS from trigger origin` over
//! > the wire-link graph; cascade fires 100ms apart for the visual
//! > chain. Deterministic seed = trigger event.
//! >
//! > **Mine detection masking**: hidden mines are not invisible in the
//! > render — they're behind a per-faction "detected" flag. The
//! > minesweeper flips the flag for the sweeping faction; the mine's
//! > enemy faction never sees the marker.
//!
//! VAL-M9C-006 / VAL-M9C-025 / VAL-M9C-026 / VAL-M9C-027 / VAL-M9C-028
//! / VAL-M9C-029 / VAL-M9C-030 / VAL-M9C-031 / VAL-M9C-MINE-ARMED-EMIT
//! / VAL-M9C-MINEFIELD-DEPLOY-BEHAVIOR / VAL-M9C-IED-COOKOFF land here.
//!
//! This file is a facade. The kernel is split across siblings:
//! `minefield_types`, `minefield_trigger`, `minefield_sweep`,
//! `minefield_disarm`, `minefield_chain`, `minefield_template`.
//! Public API at `cf_fortification::minefield::*` is preserved by
//! the re-exports below.

pub use crate::minefield_chain::{
    begin_ied_chain_cascade, ms_to_ticks, IedChainEmission, IedChainOutcome,
};
pub use crate::minefield_disarm::{
    manual_disarm_required_ticks, robot_disarm_required_ticks,
    tick_manual_disarm, DisarmInputs, DisarmTickResult,
};
pub use crate::minefield_sweep::{
    minesweeper_detection_radius_tiles, run_minesweeper_ping,
    MinesweeperPingInputs, MinesweeperPingOutcome,
};
pub use crate::minefield_template::{
    deploy_template, template_inventory_cost, MinefieldDeployOutcome,
    MinefieldPlacement, MinefieldTemplateSpec,
};
pub use crate::minefield_trigger::{
    evaluate_trigger, ActorCandidate, TriggerOutcome,
};
pub use crate::minefield_types::{
    DetectionMask, DisarmFailureCause, DisarmResult, IedCookoffEvent,
    IedCookoffKind, Mine, MineArmedEvent, MineDisarmedEvent, MineKind,
    MineTriggerCause, MineTriggeredEvent, MinesweeperDetectedEvent,
    BOMB_DISPOSAL_ROBOT_ARMOR_REDUCTION_PERCENT,
    BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND, BOMB_DISPOSAL_ROBOT_HP,
    IED_CHAIN_HOP_MILLIS, IED_CHAIN_LINK_RANGE_TILES,
    IED_CHAIN_MAX_WINDOW_MILLIS, MANUAL_DISARM_SECONDS,
    MINESWEEPER_DETECTION_PRESSURE_IED_TILES,
    MINESWEEPER_DETECTION_PROXIMITY_TRIPWIRE_TILES,
    MINESWEEPER_PING_SECONDS, MINE_DISARMED_EXPLOSIVE_RECOVERED,
    MINE_PRESSURE_BLAST_RADIUS_TILES, MINE_PROXIMITY_TRIGGER_DECITILES,
    ROBOT_DISARM_SECONDS,
};
