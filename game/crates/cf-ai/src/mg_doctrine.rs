//! M9C: AI-MG-A-02 doctrine — crew nearest empty MG, auto-swap depleted
//! ammo, uncrew + retreat when the nest is critically damaged.
//!
//! Spec §"Notes for the implementer":
//!
//! > **AI-MG-A-02** = "crew nearest empty MG within 8 tiles when threat
//! > detected within 24 tiles." Burst-and-duck behavior reuses M9B
//! > AI-TRENCH-A-01 pattern (crewed MG = effectively in fire-step
//! > position).
//!
//! Spec § Acceptance criteria scenario:
//!
//! > Given m9c_mg_nest_crewed_defense scenario: 3 empty MG nests + 4 AI
//! > defenders
//! > When the scenario starts
//! > Then AI-MG-A-02 doctrine has 3 AI move to + crew the 3 nests
//! > And the 4th AI takes overwatch position
//! > And AI ammo-feed swap is automatic when ammo_box_depleted fires
//! > And AI uncrew + retreat when nest HP < 200
//!
//! This module owns the pure decision function. The cf-control engine
//! consumes the per-AI [`MgDoctrineDecision`] each tick to dispatch
//! the corresponding `act.player.crew_fortification`,
//! `swap_ammo_box`, or `act.player.uncrew_fortification` action.
//!
//! VAL-M9C-044 lands here.

use serde::{Deserialize, Serialize};

use cf_actor::ActorId;
use cf_fortification::{
    FortificationId, MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES,
    MG_DOCTRINE_RETREAT_HP_THRESHOLD,
};

/// Re-export the spec's 24-tile threat-range constant so cf-control /
/// scenario authors can refer to it through the doctrine module.
pub use cf_fortification::MG_DOCTRINE_THREAT_RANGE_TILES;

/// Doctrine id used in AI archetype RON files + replay event payloads.
pub const DOCTRINE_ID: &str = "AI-MG-A-02";

/// One AI defender's perception of the world at this tick — the
/// information the doctrine needs to choose an action.
#[derive(Debug, Clone)]
pub struct MgDoctrineInputs {
    pub actor_id: ActorId,
    /// Tile position of the AI defender.
    pub actor_pos_tiles: (i32, i32),
    /// True when the AI is currently crewing some MG nest.
    pub is_crewing: bool,
    /// Fortification id the AI is currently crewing (Some when
    /// `is_crewing == true`).
    pub crewing_id: Option<FortificationId>,
    /// HP of the crewed nest (relevant when `is_crewing == true`).
    pub crewing_nest_hp: u32,
    /// True when the crewed nest's ammo box has emitted
    /// `ammo_box_depleted` this tick (or any earlier tick + no swap
    /// since).
    pub crewing_ammo_depleted: bool,
    /// True when at least one threat lies within
    /// [`MG_DOCTRINE_THREAT_RANGE_TILES`] (24 tiles).
    pub threat_in_range: bool,
    /// Visible empty MG nests within the AI's search horizon. The
    /// engine pre-filters by line-of-sight and faction.
    pub empty_nests_in_range: Vec<MgNestObservation>,
    /// True when the AI has a fresh `ammo_box_mg` in its inventory
    /// (drives the auto-swap branch when the crewed nest depletes).
    pub has_spare_ammo_box: bool,
}

/// One empty MG nest seen by the AI this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MgNestObservation {
    pub id: FortificationId,
    /// Tile position of the nest's crew-entry tile.
    pub pos_tiles: (i32, i32),
    /// HP of the nest (used to filter out wrecks).
    pub hp: u32,
}

/// Per-AI doctrine output. Engine consumes this each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MgDoctrineDecision {
    /// AI stays in place / falls through to other doctrines (no
    /// MG-specific action).
    Idle,
    /// AI should issue `act.player.crew_fortification { nest_id }`
    /// for the nearest empty MG within
    /// [`MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES`] tiles.
    CrewNearestEmpty { nest_id: FortificationId },
    /// AI is already crewing; the bound ammo box just depleted +
    /// it has a spare in inventory → dispatch the swap.
    SwapAmmoBox { nest_id: FortificationId },
    /// AI is already crewing; the crewed nest's HP dropped below 200
    /// → dispatch uncrew + retreat away.
    UncrewAndRetreat { nest_id: FortificationId },
    /// AI has taken overwatch position — fewer empty nests than AI
    /// defenders, so this one stays out of the crewing rotation.
    Overwatch,
}

impl MgDoctrineDecision {
    /// Stable name used for replay event payloads + cfctl HUD overlays.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MgDoctrineDecision::Idle => "idle",
            MgDoctrineDecision::CrewNearestEmpty { .. } => "crew_nearest_empty",
            MgDoctrineDecision::SwapAmmoBox { .. } => "swap_ammo_box",
            MgDoctrineDecision::UncrewAndRetreat { .. } => "uncrew_and_retreat",
            MgDoctrineDecision::Overwatch => "overwatch",
        }
    }
}

/// Squared-distance helper that stays integer-only so the doctrine
/// remains deterministic regardless of FP rounding.
#[must_use]
fn squared_distance_tiles(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = i64::from(a.0 - b.0);
    let dy = i64::from(a.1 - b.1);
    dx * dx + dy * dy
}

/// AI-MG-A-02 per-tick decision function.
///
/// The decision tree (spec § Notes):
///
/// 1. If currently crewing AND nest HP < 200 → UncrewAndRetreat.
/// 2. If currently crewing AND ammo depleted AND has spare ammo →
///    SwapAmmoBox.
/// 3. If not crewing AND threat within 24 tiles AND empty nest within
///    8 tiles → CrewNearestEmpty (closest of the in-range nests).
/// 4. Else → Idle (or Overwatch if explicitly assigned the overwatch
///    role — covered by a higher-level squad layer).
#[must_use]
pub fn decide(inputs: &MgDoctrineInputs) -> MgDoctrineDecision {
    if inputs.is_crewing {
        if let Some(nest_id) = inputs.crewing_id {
            // Spec line: "uncrew + retreat when nest HP < 200".
            if inputs.crewing_nest_hp < MG_DOCTRINE_RETREAT_HP_THRESHOLD {
                return MgDoctrineDecision::UncrewAndRetreat { nest_id };
            }
            // Spec line: "AI auto-swaps depleted ammo boxes".
            if inputs.crewing_ammo_depleted && inputs.has_spare_ammo_box {
                return MgDoctrineDecision::SwapAmmoBox { nest_id };
            }
        }
        return MgDoctrineDecision::Idle;
    }

    if !inputs.threat_in_range {
        return MgDoctrineDecision::Idle;
    }

    // Find the nearest empty nest within the 8-tile search radius
    // (deterministic tie-break by `nest_id` so two engines see
    // identical decisions).
    let radius_tiles = i64::from(MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES);
    let max_dsq = radius_tiles * radius_tiles;
    let mut best: Option<(i64, FortificationId)> = None;
    for obs in &inputs.empty_nests_in_range {
        if obs.hp == 0 {
            continue;
        }
        let dsq = squared_distance_tiles(inputs.actor_pos_tiles, obs.pos_tiles);
        if dsq > max_dsq {
            continue;
        }
        match best {
            None => best = Some((dsq, obs.id)),
            Some((bdsq, bid)) => {
                if dsq < bdsq || (dsq == bdsq && obs.id.0 < bid.0) {
                    best = Some((dsq, obs.id));
                }
            }
        }
    }
    match best {
        Some((_, nest_id)) => MgDoctrineDecision::CrewNearestEmpty { nest_id },
        None => MgDoctrineDecision::Idle,
    }
}

/// **VAL-M9C-044** convenience: given a set of empty nests + a set of
/// AI defenders within the threat window, return the per-AI decision
/// table that crews 3-nests-with-3-AI (and the 4th AI takes overwatch).
///
/// Returns `decisions[i]` for the corresponding AI in `defenders`.
/// AI without an empty nest in range fall through to `Overwatch` (the
/// spec scenario: "the 4th AI takes overwatch position").
#[must_use]
pub fn assign_crews(defenders: &[MgDoctrineInputs]) -> Vec<MgDoctrineDecision> {
    // Greedy: for each defender in input order, claim the nearest
    // still-unclaimed empty nest within the 8-tile search radius.
    let mut claimed: std::collections::BTreeSet<FortificationId> =
        std::collections::BTreeSet::new();
    let radius_tiles = i64::from(MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES);
    let max_dsq = radius_tiles * radius_tiles;
    let mut decisions = Vec::with_capacity(defenders.len());
    for ai in defenders {
        if ai.is_crewing {
            decisions.push(decide(ai));
            continue;
        }
        if !ai.threat_in_range {
            decisions.push(MgDoctrineDecision::Idle);
            continue;
        }
        let mut best: Option<(i64, FortificationId)> = None;
        for obs in &ai.empty_nests_in_range {
            if obs.hp == 0 || claimed.contains(&obs.id) {
                continue;
            }
            let dsq = squared_distance_tiles(ai.actor_pos_tiles, obs.pos_tiles);
            if dsq > max_dsq {
                continue;
            }
            match best {
                None => best = Some((dsq, obs.id)),
                Some((bdsq, bid)) => {
                    if dsq < bdsq || (dsq == bdsq && obs.id.0 < bid.0) {
                        best = Some((dsq, obs.id));
                    }
                }
            }
        }
        match best {
            Some((_, nest_id)) => {
                claimed.insert(nest_id);
                decisions.push(MgDoctrineDecision::CrewNearestEmpty { nest_id });
            }
            None => {
                // No empty nest claimable in range → overwatch.
                decisions.push(MgDoctrineDecision::Overwatch);
            }
        }
    }
    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_nest(id: u32, pos: (i32, i32)) -> MgNestObservation {
        MgNestObservation {
            id: FortificationId(id),
            pos_tiles: pos,
            hp: 800,
        }
    }

    fn defender(actor_id: u64, pos: (i32, i32), nests: Vec<MgNestObservation>) -> MgDoctrineInputs {
        MgDoctrineInputs {
            actor_id: ActorId(actor_id),
            actor_pos_tiles: pos,
            is_crewing: false,
            crewing_id: None,
            crewing_nest_hp: 0,
            crewing_ammo_depleted: false,
            threat_in_range: true,
            empty_nests_in_range: nests,
            has_spare_ammo_box: false,
        }
    }

    /// 3 nests. The 4th AI (extra defender) falls through to
    /// `Overwatch`.
    #[test]
    fn mg_doctrine_crew_nearest_empty() {
        let nests_visible = vec![
            empty_nest(10, (0, 0)),
            empty_nest(11, (4, 0)),
            empty_nest(12, (8, 0)),
        ];

        let defenders = vec![
            // AI1 close to nest 10.
            defender(1, (1, 0), nests_visible.clone()),
            // AI2 close to nest 11.
            defender(2, (3, 0), nests_visible.clone()),
            // AI3 close to nest 12.
            defender(3, (7, 0), nests_visible.clone()),
            // AI4 far from any nest within range (out of 8-tile
            // radius for all 3) → Overwatch.
            defender(4, (50, 50), nests_visible.clone()),
        ];

        let decisions = assign_crews(&defenders);
        assert_eq!(decisions.len(), 4);

        assert_eq!(
            decisions[0],
            MgDoctrineDecision::CrewNearestEmpty {
                nest_id: FortificationId(10)
            }
        );
        assert_eq!(
            decisions[1],
            MgDoctrineDecision::CrewNearestEmpty {
                nest_id: FortificationId(11)
            }
        );
        assert_eq!(
            decisions[2],
            MgDoctrineDecision::CrewNearestEmpty {
                nest_id: FortificationId(12)
            }
        );
        assert_eq!(decisions[3], MgDoctrineDecision::Overwatch);

        // Spot-check: each crewed nest is claimed exactly once.
        let crewed_ids: Vec<_> = decisions
            .iter()
            .filter_map(|d| match d {
                MgDoctrineDecision::CrewNearestEmpty { nest_id } => Some(*nest_id),
                _ => None,
            })
            .collect();
        assert_eq!(crewed_ids.len(), 3);
        assert!(crewed_ids.contains(&FortificationId(10)));
        assert!(crewed_ids.contains(&FortificationId(11)));
        assert!(crewed_ids.contains(&FortificationId(12)));
    }

    #[test]
    fn mg_doctrine_idles_when_no_threat() {
        let nests = vec![empty_nest(10, (0, 0))];
        let mut ai = defender(1, (1, 0), nests);
        ai.threat_in_range = false;
        assert_eq!(decide(&ai), MgDoctrineDecision::Idle);
    }

    /// Spec line: "AI uncrew + retreat when nest HP < 200".
    #[test]
    fn mg_doctrine_retreats_when_nest_hp_below_threshold() {
        let ai = MgDoctrineInputs {
            actor_id: ActorId(1),
            actor_pos_tiles: (0, 0),
            is_crewing: true,
            crewing_id: Some(FortificationId(10)),
            crewing_nest_hp: 150, // < 200
            crewing_ammo_depleted: false,
            threat_in_range: true,
            empty_nests_in_range: vec![],
            has_spare_ammo_box: false,
        };
        assert_eq!(
            decide(&ai),
            MgDoctrineDecision::UncrewAndRetreat {
                nest_id: FortificationId(10)
            }
        );
    }

    /// Spec line: "AI auto-swaps depleted ammo boxes".
    #[test]
    fn mg_doctrine_swaps_ammo_when_depleted_with_spare() {
        let ai = MgDoctrineInputs {
            actor_id: ActorId(1),
            actor_pos_tiles: (0, 0),
            is_crewing: true,
            crewing_id: Some(FortificationId(10)),
            crewing_nest_hp: 600,
            crewing_ammo_depleted: true,
            threat_in_range: true,
            empty_nests_in_range: vec![],
            has_spare_ammo_box: true,
        };
        assert_eq!(
            decide(&ai),
            MgDoctrineDecision::SwapAmmoBox {
                nest_id: FortificationId(10)
            }
        );
    }

    #[test]
    fn mg_doctrine_no_swap_without_spare_ammo() {
        let ai = MgDoctrineInputs {
            actor_id: ActorId(1),
            actor_pos_tiles: (0, 0),
            is_crewing: true,
            crewing_id: Some(FortificationId(10)),
            crewing_nest_hp: 600,
            crewing_ammo_depleted: true,
            threat_in_range: true,
            empty_nests_in_range: vec![],
            has_spare_ammo_box: false,
        };
        assert_eq!(decide(&ai), MgDoctrineDecision::Idle);
    }

    /// Nests outside the 8-tile search radius are ignored — even when
    /// they're the only empty nests.
    #[test]
    fn mg_doctrine_skips_nests_beyond_search_radius() {
        let nests = vec![
            empty_nest(10, (100, 100)), // far away
        ];
        let ai = defender(1, (0, 0), nests);
        assert_eq!(decide(&ai), MgDoctrineDecision::Idle);
    }

    /// VAL-M9C-044 + spec § "AI-MG-A-02": the threat range gate is at
    /// 24 tiles. The crew-search radius is at 8 tiles. Both are
    /// honored by the doctrine constants.
    #[test]
    fn mg_doctrine_constants_match_spec_thresholds() {
        assert_eq!(super::MG_DOCTRINE_THREAT_RANGE_TILES, 24);
        assert_eq!(MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES, 8);
        assert_eq!(MG_DOCTRINE_RETREAT_HP_THRESHOLD, 200);
    }
}
