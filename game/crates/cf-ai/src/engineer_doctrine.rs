//! M9C: AI-ENG-A-03 doctrine — engineer behavior: lay mines forward
//! of the perimeter, repair breaches (re-mine + restring wire),
//! disarm enemy minefields with the minesweeper tool.
//!
//! Spec § "Notes for the implementer":
//!
//! > **AI-ENG-A-03** = "if I have wire/mines + a defensive perimeter
//! > is being built, lay wire/mines forward of the fortification line;
//! > if a friendly defensive line has a breach, repair (re-mine,
//! > restring wire); if an enemy minefield is in my squad's path,
//! > disarm with minesweeper."
//!
//! Spec § Acceptance Gherkin (AI doctrine):
//!
//! > And AI-ENG-A-03: lay-mines, repair-breach, disarm tests all PASS.
//!
//! This module owns the pure decision function. The cf-control engine
//! consumes per-tick [`EngineerDoctrineDecision`] to dispatch the
//! corresponding `act.player.deploy_minefield_template`,
//! `act.player.repair_fortification`, or `act.player.disarm_mine`
//! action.
//!
//! VAL-M9C-046 lands here (sub-tests
//! `engineer_doctrine_lay_mines` + `engineer_doctrine_repair_breach`
//! + `engineer_doctrine_disarm_enemy_minefield`).

use serde::{Deserialize, Serialize};

use cf_actor::ActorId;
use cf_fortification::FortificationId;

/// Doctrine id used in AI archetype RON files + replay event payloads.
pub const DOCTRINE_ID: &str = "AI-ENG-A-03";

/// Spec line: lay mines forward of the perimeter while a defensive
/// line is being built. The doctrine fires when the engineer is within
/// this many tiles of a friendly perimeter under construction.
pub const ENGINEER_DOCTRINE_LAY_MINE_FORWARD_TILES: u32 = 8;

/// Spec line: "if a friendly defensive line has a breach, repair".
/// The doctrine considers any fortification at < this HP threshold
/// as a breach candidate.
pub const ENGINEER_DOCTRINE_BREACH_HP_THRESHOLD: u32 = 200;

/// Inputs to the per-tick engineer doctrine evaluation. The engine
/// pre-filters by faction, LOS, and squad assignment.
#[derive(Debug, Clone)]
pub struct EngineerDoctrineInputs {
    pub engineer_actor_id: ActorId,
    /// Tile position of the engineer.
    pub engineer_pos_tiles: (i32, i32),
    /// True when the engineer is carrying ≥ 1 pooled mine (proximity /
    /// pressure / tripwire / IED) in inventory.
    pub has_mines_in_inventory: bool,
    /// True when the engineer is carrying wire (barbed / razor / etc.)
    /// for restringing.
    pub has_wire_in_inventory: bool,
    /// True when the engineer is carrying a minesweeper tool.
    pub has_minesweeper: bool,
    /// Friendly perimeter sites the engineer could fortify forward of
    /// (sorted by distance ascending).
    pub friendly_perimeter_sites: Vec<PerimeterSite>,
    /// Damaged friendly fortifications the engineer should repair
    /// ("breach" rule: any fortification HP < 200).
    pub breached_fortifications: Vec<BreachedFortification>,
    /// Enemy mines in the squad's path that should be disarmed.
    pub enemy_mines_in_squad_path: Vec<EnemyMineObservation>,
}

/// One friendly perimeter site observed by the engineer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerimeterSite {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    /// True when the site is still under construction (the doctrine's
    /// trigger condition).
    pub under_construction: bool,
}

/// One breached friendly fortification observed by the engineer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreachedFortification {
    pub id: FortificationId,
    pub pos_tiles: (i32, i32),
    pub hp: u32,
}

/// One enemy mine observed by the engineer (already detected by the
/// engineer's squad's minesweeper).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyMineObservation {
    pub mine_id: FortificationId,
    pub pos_tiles: (i32, i32),
    /// True when the mine sits on the engineer's squad path. The
    /// engine pre-filters by path overlap; the doctrine only consumes
    /// the boolean.
    pub on_squad_path: bool,
}

/// One engineer-doctrine decision the engine consumes each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EngineerDoctrineDecision {
    /// No engineer-specific action this tick.
    Idle,
    /// Lay a mine forward of a friendly perimeter site under
    /// construction.
    LayMineForwardOfPerimeter {
        perimeter_id: FortificationId,
        /// Engineer-suggested mine drop position (forward of the
        /// perimeter site). Engine routes through
        /// `act.player.deploy_minefield_template` once a template id
        /// is picked.
        drop_pos_tiles: (i32, i32),
    },
    /// Repair a breached friendly fortification (re-mine / restring
    /// wire / patch the wall).
    RepairBreach {
        fortification_id: FortificationId,
    },
    /// Use the minesweeper to disarm an enemy mine in the squad's
    /// path.
    DisarmEnemyMine {
        mine_id: FortificationId,
    },
}

impl EngineerDoctrineDecision {
    /// Stable name used for replay event payloads + cfctl HUD overlays.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            EngineerDoctrineDecision::Idle => "idle",
            EngineerDoctrineDecision::LayMineForwardOfPerimeter { .. } => {
                "lay_mine_forward_of_perimeter"
            }
            EngineerDoctrineDecision::RepairBreach { .. } => "repair_breach",
            EngineerDoctrineDecision::DisarmEnemyMine { .. } => "disarm_enemy_mine",
        }
    }
}

/// Integer Chebyshev-distance helper used for the "forward of
/// perimeter" radius check.
#[must_use]
fn chebyshev_distance(a: (i32, i32), b: (i32, i32)) -> u32 {
    let dx = (a.0 - b.0).unsigned_abs();
    let dy = (a.1 - b.1).unsigned_abs();
    dx.max(dy)
}

/// AI-ENG-A-03 per-tick decision function.
///
/// Decision priority (spec § Notes for the implementer):
///
/// 1. **Disarm enemy minefield in squad path** — survival first
///    (a stepped-on mine kills the squad).
/// 2. **Repair perimeter breach** — patch broken defenses before they
///    cascade.
/// 3. **Lay mines forward of perimeter under construction** — proactive
///    fortification.
/// 4. **Idle** — nothing to do.
#[must_use]
pub fn decide(inputs: &EngineerDoctrineInputs) -> EngineerDoctrineDecision {
    // 1) Disarm any enemy mine on the squad's path (with minesweeper).
    if inputs.has_minesweeper {
        let mut best: Option<(u32, FortificationId)> = None;
        for obs in &inputs.enemy_mines_in_squad_path {
            if !obs.on_squad_path {
                continue;
            }
            let dist = chebyshev_distance(inputs.engineer_pos_tiles, obs.pos_tiles);
            match best {
                None => best = Some((dist, obs.mine_id)),
                Some((bd, bid)) => {
                    if dist < bd || (dist == bd && obs.mine_id.0 < bid.0) {
                        best = Some((dist, obs.mine_id));
                    }
                }
            }
        }
        if let Some((_, mine_id)) = best {
            return EngineerDoctrineDecision::DisarmEnemyMine { mine_id };
        }
    }

    // 2) Repair perimeter breach (closest breached fortification with
    //    HP < threshold).
    let mut best_breach: Option<(u32, FortificationId)> = None;
    for b in &inputs.breached_fortifications {
        if b.hp >= ENGINEER_DOCTRINE_BREACH_HP_THRESHOLD {
            continue;
        }
        let dist = chebyshev_distance(inputs.engineer_pos_tiles, b.pos_tiles);
        match best_breach {
            None => best_breach = Some((dist, b.id)),
            Some((bd, bid)) => {
                if dist < bd || (dist == bd && b.id.0 < bid.0) {
                    best_breach = Some((dist, b.id));
                }
            }
        }
    }
    if let Some((_, fortification_id)) = best_breach {
        return EngineerDoctrineDecision::RepairBreach { fortification_id };
    }

    // 3) Lay mines forward of perimeter under construction (mines OR
    //    wire in inventory both qualify per spec line "lay wire/mines
    //    forward of the fortification line").
    if inputs.has_mines_in_inventory || inputs.has_wire_in_inventory {
        let mut best_site: Option<(u32, &PerimeterSite)> = None;
        for s in &inputs.friendly_perimeter_sites {
            if !s.under_construction {
                continue;
            }
            let dist = chebyshev_distance(inputs.engineer_pos_tiles, s.pos_tiles);
            if dist > ENGINEER_DOCTRINE_LAY_MINE_FORWARD_TILES {
                continue;
            }
            match best_site {
                None => best_site = Some((dist, s)),
                Some((bd, bs)) => {
                    if dist < bd || (dist == bd && s.id.0 < bs.id.0) {
                        best_site = Some((dist, s));
                    }
                }
            }
        }
        if let Some((_, site)) = best_site {
            // Forward of perimeter: one tile past the perimeter site
            // away from the engineer. Deterministic + integer arithmetic.
            let dx_sign = (site.pos_tiles.0 - inputs.engineer_pos_tiles.0).signum();
            let dy_sign = (site.pos_tiles.1 - inputs.engineer_pos_tiles.1).signum();
            let drop_pos_tiles = (
                site.pos_tiles.0 + dx_sign,
                site.pos_tiles.1 + dy_sign,
            );
            return EngineerDoctrineDecision::LayMineForwardOfPerimeter {
                perimeter_id: site.id,
                drop_pos_tiles,
            };
        }
    }

    EngineerDoctrineDecision::Idle
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engineer(pos: (i32, i32)) -> EngineerDoctrineInputs {
        EngineerDoctrineInputs {
            engineer_actor_id: ActorId(1),
            engineer_pos_tiles: pos,
            has_mines_in_inventory: false,
            has_wire_in_inventory: false,
            has_minesweeper: false,
            friendly_perimeter_sites: vec![],
            breached_fortifications: vec![],
            enemy_mines_in_squad_path: vec![],
        }
    }

    /// VAL-M9C-046 line (a): engineer with mines + perimeter under
    /// construction lays mines forward of the perimeter.
    #[test]
    fn engineer_doctrine_lay_mines() {
        let mut e = engineer((0, 0));
        e.has_mines_in_inventory = true;
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(42),
            pos_tiles: (4, 0),
            under_construction: true,
        }];
        let d = decide(&e);
        match d {
            EngineerDoctrineDecision::LayMineForwardOfPerimeter {
                perimeter_id,
                drop_pos_tiles,
            } => {
                assert_eq!(perimeter_id, FortificationId(42));
                // Drop is one tile forward of the perimeter site away
                // from the engineer.
                assert_eq!(drop_pos_tiles, (5, 0));
            }
            other => panic!("expected LayMine, got {other:?}"),
        }

        // Sites NOT under construction are ignored (no proactive lay).
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(43),
            pos_tiles: (4, 0),
            under_construction: false,
        }];
        assert_eq!(decide(&e), EngineerDoctrineDecision::Idle);

        // Without mines / wire the doctrine cannot lay anything.
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(42),
            pos_tiles: (4, 0),
            under_construction: true,
        }];
        e.has_mines_in_inventory = false;
        e.has_wire_in_inventory = false;
        assert_eq!(decide(&e), EngineerDoctrineDecision::Idle);

        // Wire-in-inventory ALSO triggers the lay-forward branch per
        // spec line "lay wire/mines forward of the fortification line".
        e.has_wire_in_inventory = true;
        match decide(&e) {
            EngineerDoctrineDecision::LayMineForwardOfPerimeter { .. } => {}
            other => panic!("wire-in-inventory must trigger lay-forward, got {other:?}"),
        }

        // Sites > 8 tiles away are ignored (out of the forward radius).
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(44),
            pos_tiles: (100, 100),
            under_construction: true,
        }];
        assert_eq!(decide(&e), EngineerDoctrineDecision::Idle);
    }

    /// VAL-M9C-046 line (b): engineer-doctrine repairs a breach
    /// (fortification HP < 200) before laying new mines.
    #[test]
    fn engineer_doctrine_repair_breach() {
        let mut e = engineer((0, 0));
        e.has_mines_in_inventory = true;
        e.has_wire_in_inventory = true;
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(10),
            pos_tiles: (4, 0),
            under_construction: true,
        }];
        e.breached_fortifications = vec![BreachedFortification {
            id: FortificationId(99),
            pos_tiles: (2, 0),
            hp: 150, // < 200
        }];
        // Repair wins over lay-forward (priority 2 over priority 3).
        let d = decide(&e);
        assert_eq!(
            d,
            EngineerDoctrineDecision::RepairBreach {
                fortification_id: FortificationId(99)
            }
        );

        // Breach above threshold is ignored.
        e.breached_fortifications = vec![BreachedFortification {
            id: FortificationId(99),
            pos_tiles: (2, 0),
            hp: 300, // >= 200
        }];
        match decide(&e) {
            EngineerDoctrineDecision::LayMineForwardOfPerimeter { .. } => {}
            other => panic!("expected lay-forward when no breach below threshold, got {other:?}"),
        }
    }

    /// VAL-M9C-046 line (c): engineer with minesweeper disarms an
    /// enemy minefield in the squad's path.
    #[test]
    fn engineer_doctrine_disarm_enemy_minefield() {
        let mut e = engineer((0, 0));
        e.has_minesweeper = true;
        e.has_mines_in_inventory = true;
        e.friendly_perimeter_sites = vec![PerimeterSite {
            id: FortificationId(10),
            pos_tiles: (4, 0),
            under_construction: true,
        }];
        e.breached_fortifications = vec![BreachedFortification {
            id: FortificationId(99),
            pos_tiles: (2, 0),
            hp: 150,
        }];
        e.enemy_mines_in_squad_path = vec![
            EnemyMineObservation {
                mine_id: FortificationId(7),
                pos_tiles: (3, 0),
                on_squad_path: true,
            },
            EnemyMineObservation {
                mine_id: FortificationId(8),
                pos_tiles: (10, 0),
                on_squad_path: false, // ignored
            },
        ];
        // Disarm wins over repair (priority 1 over priority 2).
        let d = decide(&e);
        assert_eq!(
            d,
            EngineerDoctrineDecision::DisarmEnemyMine {
                mine_id: FortificationId(7)
            }
        );

        // Without minesweeper, the disarm branch falls through to the
        // repair branch.
        e.has_minesweeper = false;
        let d = decide(&e);
        assert_eq!(
            d,
            EngineerDoctrineDecision::RepairBreach {
                fortification_id: FortificationId(99)
            }
        );
    }

    /// Idle when nothing is actionable.
    #[test]
    fn engineer_doctrine_idle_when_nothing_actionable() {
        let e = engineer((0, 0));
        assert_eq!(decide(&e), EngineerDoctrineDecision::Idle);
    }

    /// Decision `as_str` round-trips to the documented stable id.
    #[test]
    fn engineer_decision_as_str_matches_stable_id() {
        assert_eq!(EngineerDoctrineDecision::Idle.as_str(), "idle");
        assert_eq!(
            EngineerDoctrineDecision::LayMineForwardOfPerimeter {
                perimeter_id: FortificationId(1),
                drop_pos_tiles: (0, 0),
            }
            .as_str(),
            "lay_mine_forward_of_perimeter"
        );
        assert_eq!(
            EngineerDoctrineDecision::RepairBreach {
                fortification_id: FortificationId(1),
            }
            .as_str(),
            "repair_breach"
        );
        assert_eq!(
            EngineerDoctrineDecision::DisarmEnemyMine {
                mine_id: FortificationId(1),
            }
            .as_str(),
            "disarm_enemy_mine"
        );
    }
}
