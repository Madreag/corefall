//! M9C minefield kernel — minesweeper detection ping + per-faction
//! detection-bit flip.

use crate::common::FortificationFaction;
use crate::minefield_types::{
    Mine, MineKind, MinesweeperDetectedEvent,
    MINESWEEPER_DETECTION_PRESSURE_IED_TILES,
    MINESWEEPER_DETECTION_PROXIMITY_TRIPWIRE_TILES,
};

/// Inputs to a single minesweeper detection ping.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_field_names)]
pub struct MinesweeperPingInputs {
    pub sweeper_actor_id: u64,
    pub sweeper_faction: FortificationFaction,
    pub sweeper_pos_tiles: (i32, i32),
}

/// Result of running a minesweeper detection ping over a list of mines.
/// The kernel flips the per-faction detection bit on each newly
/// revealed mine and emits one [`MinesweeperDetectedEvent`] per
/// transition (so the recorder doesn't re-emit the event each ping).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinesweeperPingOutcome {
    pub events: Vec<MinesweeperDetectedEvent>,
}

/// Detection-radius lookup. Spec § "Minesweeper" table:
/// proximity / tripwire → 3-tile radius; pressure / IED → 2-tile radius.
#[must_use]
pub const fn minesweeper_detection_radius_tiles(kind: MineKind) -> u32 {
    match kind {
        MineKind::MineProximity | MineKind::TripwireMine => {
            MINESWEEPER_DETECTION_PROXIMITY_TRIPWIRE_TILES
        }
        MineKind::MinePressure | MineKind::IedChain => {
            MINESWEEPER_DETECTION_PRESSURE_IED_TILES
        }
    }
}

/// Run a minesweeper ping over the supplied mines + flip the
/// per-faction detection bit on each newly revealed mine.
pub fn run_minesweeper_ping(
    inputs: MinesweeperPingInputs,
    mines: &mut [Mine],
    tick_index: u64,
) -> MinesweeperPingOutcome {
    let mut events = Vec::new();
    for mine in mines.iter_mut() {
        if mine.is_inactive() {
            continue;
        }
        if mine.detection.visible_to(inputs.sweeper_faction) {
            continue;
        }
        let radius = i64::from(minesweeper_detection_radius_tiles(mine.kind));
        let dx = i64::from(mine.pos_tiles.0 - inputs.sweeper_pos_tiles.0);
        let dy = i64::from(mine.pos_tiles.1 - inputs.sweeper_pos_tiles.1);
        if dx * dx + dy * dy > radius * radius {
            continue;
        }
        if mine.detection.set(inputs.sweeper_faction) {
            events.push(MinesweeperDetectedEvent {
                mine_id: mine.id,
                sweeper_id: inputs.sweeper_actor_id,
                sweeper_faction: inputs.sweeper_faction,
                tick_index,
            });
        }
    }
    MinesweeperPingOutcome { events }
}
