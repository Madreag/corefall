//! M6C-7: Placed-mine trigger evaluation.
//!
//! Gherkin scenario M6C-7:
//! ```text
//! Scenario: M6C-7 Proximity mine triggers
//!   Given player places proximity_mine
//!   When hostile enters 4-tile radius:
//!     Then mine.detonated fires
//! ```
//!
//! The trigger radius is per-preset (proximity = 4 tiles, pressure = 1,
//! tripwire = 6, bouncing_betty = 3). World-tile size is configured at
//! engine setup; this module operates on tile distance directly so the
//! actor coordinate system does not leak into cf-equipment.

use serde::{Deserialize, Serialize};

/// Default proximity mine trigger radius (tiles) per M6C-7.
pub const PROXIMITY_TRIGGER_RADIUS_TILES: u8 = 4;

/// Per-actor relationship to the mine owner.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Owner of the mine — never triggers.
    Owner = 0,
    /// Same faction as owner — never triggers.
    Friendly = 1,
    /// Hostile to owner — triggers.
    Hostile = 2,
    /// Neutral / unknown — does not trigger.
    Neutral = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateActor {
    pub distance_tiles: f32,
    pub relation: Relation,
    /// True when this actor's footprint touched the trigger plate
    /// (pressure mines).
    pub touched_plate: bool,
    /// True when this actor crossed the tripwire this tick.
    pub crossed_tripwire: bool,
}

/// Outcome of [`evaluate_mine_trigger`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MineTriggerOutcome {
    NotArmed,
    NoTrigger,
    Triggered { reason: String },
}

impl MineTriggerOutcome {
    pub fn fired(&self) -> bool {
        matches!(self, MineTriggerOutcome::Triggered { .. })
    }
}

/// Kind of trigger geometry the mine uses.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MineTriggerKind {
    Proximity = 0,
    Pressure = 1,
    Tripwire = 2,
    BouncingBetty = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MineDescriptor {
    pub trigger_kind: MineTriggerKind,
    pub trigger_radius_tiles: u8,
    pub armed: bool,
}

/// Evaluate whether any candidate actor in the input set should trigger the
/// mine this tick.
pub fn evaluate_mine_trigger(
    mine: MineDescriptor,
    candidates: &[CandidateActor],
) -> MineTriggerOutcome {
    if !mine.armed {
        return MineTriggerOutcome::NotArmed;
    }
    let radius = f32::from(mine.trigger_radius_tiles);
    for c in candidates {
        if c.relation != Relation::Hostile {
            continue;
        }
        match mine.trigger_kind {
            MineTriggerKind::Proximity => {
                if c.distance_tiles <= radius {
                    return MineTriggerOutcome::Triggered {
                        reason: "hostile_proximity".to_string(),
                    };
                }
            }
            MineTriggerKind::Pressure => {
                if c.touched_plate {
                    return MineTriggerOutcome::Triggered {
                        reason: "actor_weight_plate".to_string(),
                    };
                }
            }
            MineTriggerKind::Tripwire => {
                if c.crossed_tripwire || c.distance_tiles <= radius {
                    return MineTriggerOutcome::Triggered {
                        reason: "tripwire_crossed".to_string(),
                    };
                }
            }
            MineTriggerKind::BouncingBetty => {
                if c.distance_tiles <= radius {
                    return MineTriggerOutcome::Triggered {
                        reason: "air_burst_arm".to_string(),
                    };
                }
            }
        }
    }
    MineTriggerOutcome::NoTrigger
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proximity_mine_triggers_on_hostile_inside_radius() {
        // M6C-7 Scenario:
        //   Given player places proximity_mine
        //   When hostile enters 4-tile radius:
        //     Then mine.detonated fires
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Proximity,
            trigger_radius_tiles: PROXIMITY_TRIGGER_RADIUS_TILES,
            armed: true,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 3.5,
            relation: Relation::Hostile,
            touched_plate: false,
            crossed_tripwire: false,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert!(out.fired());
    }

    #[test]
    fn proximity_mine_does_not_trigger_on_friendly() {
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Proximity,
            trigger_radius_tiles: PROXIMITY_TRIGGER_RADIUS_TILES,
            armed: true,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 2.0,
            relation: Relation::Friendly,
            touched_plate: false,
            crossed_tripwire: false,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert!(!out.fired());
    }

    #[test]
    fn proximity_mine_does_not_trigger_outside_radius() {
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Proximity,
            trigger_radius_tiles: 4,
            armed: true,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 4.5,
            relation: Relation::Hostile,
            touched_plate: false,
            crossed_tripwire: false,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert!(!out.fired());
    }

    #[test]
    fn pressure_mine_triggers_on_plate_touch() {
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Pressure,
            trigger_radius_tiles: 1,
            armed: true,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 0.5,
            relation: Relation::Hostile,
            touched_plate: true,
            crossed_tripwire: false,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert!(out.fired());
    }

    #[test]
    fn tripwire_triggers_on_crossing() {
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Tripwire,
            trigger_radius_tiles: 6,
            armed: true,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 10.0,
            relation: Relation::Hostile,
            touched_plate: false,
            crossed_tripwire: true,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert!(out.fired());
    }

    #[test]
    fn disarmed_mine_never_triggers() {
        let mine = MineDescriptor {
            trigger_kind: MineTriggerKind::Proximity,
            trigger_radius_tiles: 4,
            armed: false,
        };
        let candidates = vec![CandidateActor {
            distance_tiles: 1.0,
            relation: Relation::Hostile,
            touched_plate: false,
            crossed_tripwire: false,
        }];
        let out = evaluate_mine_trigger(mine, &candidates);
        assert_eq!(out, MineTriggerOutcome::NotArmed);
    }
}
