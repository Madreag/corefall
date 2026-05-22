//! M9C minefield kernel — IED chain BFS cascade. Walks the wired-
//! link graph in BFS order, scheduling each hop `IED_CHAIN_HOP_MILLIS`
//! apart; adjacent hops are bridged by an M14J cookoff event.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::common::FortificationId;
use crate::minefield_types::{
    IedCookoffEvent, IedCookoffKind, Mine, MineKind, MineTriggerCause,
    MineTriggeredEvent, IED_CHAIN_HOP_MILLIS,
};

/// Convert milliseconds to ticks at the supplied tick rate.
#[must_use]
pub fn ms_to_ticks(ms: u32, tick_rate_hz: u32) -> u32 {
    if tick_rate_hz == 0 {
        return 0;
    }
    let raw = u64::from(ms) * u64::from(tick_rate_hz) / 1000;
    raw.try_into().unwrap_or(u32::MAX)
}

/// One emitted record from the IED chain cascade BFS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IedChainEmission {
    Trigger(MineTriggeredEvent),
    Cookoff(IedCookoffEvent),
}

/// Outcome of an IED-chain cascade. Events are listed in the order
/// the recorder must emit them (BFS over the wire-link graph; each
/// adjacent hop is bridged by an [`IedCookoffEvent::ChargeInitiated`]
/// record so VAL-M9C-IED-COOKOFF is satisfied).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IedChainOutcome {
    pub emissions: Vec<IedChainEmission>,
    /// Mine ids consumed by the cascade (caller marks them inactive).
    pub detonated_ids: Vec<FortificationId>,
    /// Total cascade window in ticks (start of trigger → final hop).
    pub window_ticks: u32,
}

/// Trigger an IED chain cascade from the supplied origin mine and
/// produce the per-hop event sequence. The cascade walks the wired-
/// link graph (`mine.wired_links`) in BFS order, scheduling each hop
/// `IED_CHAIN_HOP_MILLIS` apart. Each adjacent pair is bridged by an
/// `cookoff.charge_initiated` event so the replay stream between
/// `mine_triggered` records carries the M14J cookoff intermediary.
///
/// `mines` is the world mine list. The function does NOT mutate
/// `armed`; the caller (cf-control) does that after consuming the
/// outcome, so the BFS sees the original wire graph.
#[must_use]
pub fn begin_ied_chain_cascade(
    origin_id: FortificationId,
    mines: &[Mine],
    initial_cause: MineTriggerCause,
    tick_index: u64,
    tick_rate_hz: u32,
) -> IedChainOutcome {
    let mut emissions = Vec::new();
    let mut detonated_ids = Vec::new();

    let mut by_id: BTreeMap<FortificationId, &Mine> = BTreeMap::new();
    for m in mines {
        by_id.insert(m.id, m);
    }
    let origin = match by_id.get(&origin_id) {
        Some(m) if m.armed && m.kind == MineKind::IedChain => *m,
        _ => return IedChainOutcome::default(),
    };

    let hop_ticks = ms_to_ticks(IED_CHAIN_HOP_MILLIS, tick_rate_hz);
    let mut seen: BTreeSet<FortificationId> = BTreeSet::new();
    let mut queue: VecDeque<(FortificationId, u32, MineTriggerCause)> = VecDeque::new();
    seen.insert(origin.id);
    queue.push_back((origin.id, 0, initial_cause));

    let mut chain_index = 0usize;
    let mut max_window_ticks = 0u32;

    while let Some((mid, hop_offset_ticks, cause)) = queue.pop_front() {
        let mine = match by_id.get(&mid) {
            Some(m) => *m,
            None => continue,
        };
        if !mine.armed || mine.kind != MineKind::IedChain {
            continue;
        }
        let fire_tick = tick_index.saturating_add(u64::from(hop_offset_ticks));
        emissions.push(IedChainEmission::Trigger(MineTriggeredEvent {
            mine_id: mine.id,
            trigger_kind: if chain_index == 0 {
                cause
            } else {
                MineTriggerCause::IedChain
            },
            yield_joules: mine.yield_joules,
            blast_radius_tiles: mine.blast_radius_tiles,
            tick_index: fire_tick,
        }));
        detonated_ids.push(mine.id);
        chain_index += 1;
        max_window_ticks = max_window_ticks.max(hop_offset_ticks);

        // Sort neighbors so the BFS is deterministic.
        let mut neighbors: Vec<FortificationId> = mine.wired_links.clone();
        neighbors.sort();
        let next_hop_offset_ticks = hop_offset_ticks.saturating_add(hop_ticks);
        let next_fire_tick =
            tick_index.saturating_add(u64::from(next_hop_offset_ticks));
        for n_id in neighbors {
            if seen.contains(&n_id) {
                continue;
            }
            let neighbor = match by_id.get(&n_id) {
                Some(m) => *m,
                None => continue,
            };
            if !neighbor.armed || neighbor.kind != MineKind::IedChain {
                continue;
            }
            // event between this hop and the next. The recorder writes
            // it between adjacent `mine_triggered` records.
            emissions.push(IedChainEmission::Cookoff(IedCookoffEvent {
                bridging_mine_id: mine.id,
                next_mine_id: neighbor.id,
                kind: IedCookoffKind::ChargeInitiated,
                tick_index: next_fire_tick,
            }));
            seen.insert(neighbor.id);
            queue.push_back((
                neighbor.id,
                next_hop_offset_ticks,
                MineTriggerCause::IedChain,
            ));
        }
    }
    IedChainOutcome {
        emissions,
        detonated_ids,
        window_ticks: max_window_ticks,
    }
}
