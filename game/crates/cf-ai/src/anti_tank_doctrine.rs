//! M9C: AI-AT-A-04 doctrine — anti-tank obstacle approach decisions
//! for AI-driven vehicles. Spec § "Notes for the implementer":
//!
//! > **AI-AT-A-04** = "if I'm driving a vehicle approaching an AT
//! > ditch / dragon's teeth, evaluate the obstacle: detour > 30 tiles
//! > unless suspension HP > 70% (then plow); never plow if dragon's
//! > teeth (always detour)."
//!
//! Spec § Acceptance criteria (AT-doctrine):
//!
//! > AI-AT-A-04: detour-or-plow ditch test PASS; dragon's teeth
//! > always-detour PASS.
//!
//! Pure decision function. cf-control consumes the per-tick
//! [`AntiTankDoctrineDecision`] to drive vehicle path replanning
//! (detour) or to acknowledge the plow-through choice (which surfaces
//! as the engine applying stuck-rolls + dragon's-teeth contacts to
//! the M44C chassis).
//!
//! VAL-M9C-047 lands here.

use serde::{Deserialize, Serialize};

use cf_actor::ActorId;
use cf_fortification::{AntiTankKind, FortificationId};

/// Doctrine id used in AI archetype RON files + replay event payloads.
pub const DOCTRINE_ID: &str = "AI-AT-A-04";

/// Spec line: detour distance threshold for an AT ditch. AI prefers
/// the detour route unless the detour exceeds 30 tiles AND the
/// vehicle's suspension is healthy.
pub const ANTI_TANK_DOCTRINE_DETOUR_THRESHOLD_TILES: u32 = 30;

/// Spec line: suspension HP threshold (as a percent of max) at or
/// above which the AI is willing to plow an AT ditch. Stored in
/// integer percent (0..=100) so the predicate stays integer-only.
pub const ANTI_TANK_DOCTRINE_PLOW_SUSPENSION_THRESHOLD_PERCENT: u32 = 70;

/// One observed anti-tank obstacle on the vehicle's intended path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedObstacle {
    pub id: FortificationId,
    pub kind: AntiTankKind,
    /// Distance from the vehicle to the obstacle along its current
    /// path, in tiles.
    pub distance_tiles: u32,
    /// Detour cost in tiles — the extra tiles the AI would walk
    /// around this obstacle instead of plowing through it. The engine
    /// computes this from its M22 pathfinder.
    pub detour_extra_tiles: u32,
}

/// Inputs to one per-tick AI-AT-A-04 evaluation. The engine pre-
/// filters by faction + LOS + reachability; the doctrine consumes
/// the scalar suspension HP percent + the observed obstacles list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiTankDoctrineInputs {
    pub actor_id: ActorId,
    /// Current M44C suspension HP as percent of max (0..=100).
    pub suspension_hp_percent: u32,
    /// Anti-tank obstacles observed on the vehicle's current path.
    pub obstacles: Vec<ObservedObstacle>,
}

/// One AI-AT-A-04 decision per evaluated tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AntiTankDoctrineDecision {
    /// No obstacle observed; vehicle continues per its main plan.
    Continue,
    /// Detour around the obstacle (engine replans path).
    Detour {
        obstacle_id: FortificationId,
        obstacle_kind: AntiTankKind,
        detour_extra_tiles: u32,
        reason: DetourReason,
    },
    /// Plow through the obstacle (only legal for `anti_tank_ditch`
    /// when suspension HP > 70%).
    Plow {
        obstacle_id: FortificationId,
        obstacle_kind: AntiTankKind,
        suspension_hp_percent: u32,
    },
}

/// Reason an AI chose to detour around an obstacle.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetourReason {
    /// Spec § "never plow dragon's teeth" — always detour regardless
    /// of suspension HP.
    DragonsTeethAlwaysDetour = 0,
    /// AT ditch with suspension HP ≤ 70% — plow would beach the
    /// vehicle; safer to detour.
    DitchSuspensionTooLow = 1,
    /// AT ditch detour is the cheaper option — short detour beats a
    /// stuck-roll gamble.
    DitchShortDetour = 2,
    /// Tank trap X / bollard / unknown obstacle — defer to the
    /// generic detour-prefer behavior.
    UnknownObstacle = 3,
}

impl DetourReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DetourReason::DragonsTeethAlwaysDetour => "dragons_teeth_always_detour",
            DetourReason::DitchSuspensionTooLow => "ditch_suspension_too_low",
            DetourReason::DitchShortDetour => "ditch_short_detour",
            DetourReason::UnknownObstacle => "unknown_obstacle",
        }
    }
}

impl AntiTankDoctrineDecision {
    /// Stable name for replay event payloads + cfctl HUD overlays.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AntiTankDoctrineDecision::Continue => "continue",
            AntiTankDoctrineDecision::Detour { .. } => "detour",
            AntiTankDoctrineDecision::Plow { .. } => "plow",
        }
    }
}

/// AI-AT-A-04 per-tick decision function.
///
/// Priority (spec § Notes for the implementer):
///
/// 1. **Dragon's teeth always detour** — never plow regardless of
///    suspension HP.
/// 2. **AT ditch**:
///    - Suspension HP > 70% AND detour > 30 tiles → plow.
///    - Otherwise → detour.
/// 3. **Tank trap X / bollard / unknown** — defer to detour-preferred
///    fallback (engine's path planner picks the cheaper option).
/// 4. **No obstacles** → continue.
///
/// The doctrine evaluates the **closest** obstacle on the path — the
/// engine pre-sorts. When multiple closest obstacles tie, the doctrine
/// picks the lowest `obstacle_id` deterministically.
#[must_use]
pub fn decide(inputs: &AntiTankDoctrineInputs) -> AntiTankDoctrineDecision {
    if inputs.obstacles.is_empty() {
        return AntiTankDoctrineDecision::Continue;
    }
    let mut best: Option<&ObservedObstacle> = None;
    for obs in &inputs.obstacles {
        match best {
            None => best = Some(obs),
            Some(b) => {
                if obs.distance_tiles < b.distance_tiles
                    || (obs.distance_tiles == b.distance_tiles && obs.id.0 < b.id.0)
                {
                    best = Some(obs);
                }
            }
        }
    }
    let obstacle = match best {
        Some(o) => o,
        None => return AntiTankDoctrineDecision::Continue,
    };
    match obstacle.kind {
        AntiTankKind::DragonsTeeth => AntiTankDoctrineDecision::Detour {
            obstacle_id: obstacle.id,
            obstacle_kind: obstacle.kind,
            detour_extra_tiles: obstacle.detour_extra_tiles,
            reason: DetourReason::DragonsTeethAlwaysDetour,
        },
        AntiTankKind::AntiTankDitch => {
            // Spec line: "detour > 30 tiles unless suspension HP >
            // 70% (then plow)". The detour-threshold predicate fires
            // BEFORE the suspension check — if detour is ≤ 30 tiles,
            // detour anyway (the short detour beats the plow gamble).
            if obstacle.detour_extra_tiles <= ANTI_TANK_DOCTRINE_DETOUR_THRESHOLD_TILES {
                return AntiTankDoctrineDecision::Detour {
                    obstacle_id: obstacle.id,
                    obstacle_kind: obstacle.kind,
                    detour_extra_tiles: obstacle.detour_extra_tiles,
                    reason: DetourReason::DitchShortDetour,
                };
            }
            if inputs.suspension_hp_percent
                > ANTI_TANK_DOCTRINE_PLOW_SUSPENSION_THRESHOLD_PERCENT
            {
                AntiTankDoctrineDecision::Plow {
                    obstacle_id: obstacle.id,
                    obstacle_kind: obstacle.kind,
                    suspension_hp_percent: inputs.suspension_hp_percent,
                }
            } else {
                AntiTankDoctrineDecision::Detour {
                    obstacle_id: obstacle.id,
                    obstacle_kind: obstacle.kind,
                    detour_extra_tiles: obstacle.detour_extra_tiles,
                    reason: DetourReason::DitchSuspensionTooLow,
                }
            }
        }
        AntiTankKind::TankTrapX | AntiTankKind::BollardConcrete => {
            AntiTankDoctrineDecision::Detour {
                obstacle_id: obstacle.id,
                obstacle_kind: obstacle.kind,
                detour_extra_tiles: obstacle.detour_extra_tiles,
                reason: DetourReason::UnknownObstacle,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(
        actor_id: u64,
        suspension_hp_percent: u32,
        obstacles: Vec<ObservedObstacle>,
    ) -> AntiTankDoctrineInputs {
        AntiTankDoctrineInputs {
            actor_id: ActorId(actor_id),
            suspension_hp_percent,
            obstacles,
        }
    }

    /// VAL-M9C-047 part 1: detour > 30 tiles around AT ditch when
    /// suspension HP ≤ 70%; plow when suspension HP > 70%.
    #[test]
    fn anti_tank_doctrine_detour_or_plow_ditch() {
        // Long detour (> 30 tiles) + high suspension HP (> 70%) → plow.
        let plow_inputs = inputs(
            1,
            71,
            vec![ObservedObstacle {
                id: FortificationId(10),
                kind: AntiTankKind::AntiTankDitch,
                distance_tiles: 5,
                detour_extra_tiles: 40,
            }],
        );
        match decide(&plow_inputs) {
            AntiTankDoctrineDecision::Plow {
                obstacle_id,
                obstacle_kind,
                suspension_hp_percent,
            } => {
                assert_eq!(obstacle_id, FortificationId(10));
                assert_eq!(obstacle_kind, AntiTankKind::AntiTankDitch);
                assert_eq!(suspension_hp_percent, 71);
            }
            other => panic!("expected Plow at suspension 71%, got {other:?}"),
        }

        // Long detour (> 30 tiles) + low suspension HP (≤ 70%) → detour
        // with reason DitchSuspensionTooLow.
        let detour_low_sus = inputs(
            1,
            65,
            vec![ObservedObstacle {
                id: FortificationId(10),
                kind: AntiTankKind::AntiTankDitch,
                distance_tiles: 5,
                detour_extra_tiles: 40,
            }],
        );
        match decide(&detour_low_sus) {
            AntiTankDoctrineDecision::Detour { reason, .. } => {
                assert_eq!(reason, DetourReason::DitchSuspensionTooLow);
            }
            other => panic!("expected Detour at suspension 65%, got {other:?}"),
        }

        // Short detour (≤ 30 tiles): detour regardless of suspension HP
        // (cheaper than gambling on the stuck roll).
        let detour_short = inputs(
            1,
            100,
            vec![ObservedObstacle {
                id: FortificationId(10),
                kind: AntiTankKind::AntiTankDitch,
                distance_tiles: 5,
                detour_extra_tiles: 20,
            }],
        );
        match decide(&detour_short) {
            AntiTankDoctrineDecision::Detour { reason, .. } => {
                assert_eq!(reason, DetourReason::DitchShortDetour);
            }
            other => panic!("expected Detour at short detour, got {other:?}"),
        }

        // Boundary: suspension HP exactly at 70% → detour (predicate
        // is strictly `> 70`).
        let boundary = inputs(
            1,
            70,
            vec![ObservedObstacle {
                id: FortificationId(10),
                kind: AntiTankKind::AntiTankDitch,
                distance_tiles: 5,
                detour_extra_tiles: 40,
            }],
        );
        match decide(&boundary) {
            AntiTankDoctrineDecision::Detour { reason, .. } => {
                assert_eq!(reason, DetourReason::DitchSuspensionTooLow);
            }
            other => panic!("expected Detour at exactly 70% suspension, got {other:?}"),
        }
    }

    /// VAL-M9C-047 part 2: dragon's teeth always detour (regardless of
    /// suspension HP).
    #[test]
    fn anti_tank_doctrine_dragons_teeth_always_detour() {
        for sus_hp in [100_u32, 90, 71, 50, 1] {
            let inputs_dt = inputs(
                1,
                sus_hp,
                vec![ObservedObstacle {
                    id: FortificationId(20),
                    kind: AntiTankKind::DragonsTeeth,
                    distance_tiles: 5,
                    detour_extra_tiles: 100,
                }],
            );
            match decide(&inputs_dt) {
                AntiTankDoctrineDecision::Detour {
                    obstacle_kind,
                    reason,
                    ..
                } => {
                    assert_eq!(obstacle_kind, AntiTankKind::DragonsTeeth);
                    assert_eq!(reason, DetourReason::DragonsTeethAlwaysDetour);
                }
                other => panic!(
                    "expected Detour around dragon's teeth at suspension {sus_hp}%, got {other:?}"
                ),
            }
        }
    }

    /// No obstacles observed → `Continue` decision.
    #[test]
    fn anti_tank_doctrine_idle_when_no_obstacles() {
        assert_eq!(decide(&inputs(1, 100, vec![])), AntiTankDoctrineDecision::Continue);
    }

    /// Tank trap X / bollard → fall through to generic detour-prefer
    /// behavior with `UnknownObstacle` reason.
    #[test]
    fn anti_tank_doctrine_tank_trap_falls_through() {
        let result = decide(&inputs(
            1,
            100,
            vec![ObservedObstacle {
                id: FortificationId(30),
                kind: AntiTankKind::TankTrapX,
                distance_tiles: 3,
                detour_extra_tiles: 15,
            }],
        ));
        match result {
            AntiTankDoctrineDecision::Detour {
                obstacle_kind,
                reason,
                ..
            } => {
                assert_eq!(obstacle_kind, AntiTankKind::TankTrapX);
                assert_eq!(reason, DetourReason::UnknownObstacle);
            }
            other => panic!("expected Detour for tank trap X, got {other:?}"),
        }
    }

    /// Closest-obstacle tie-break: lower `obstacle_id` wins
    /// deterministically.
    #[test]
    fn anti_tank_doctrine_picks_closest_obstacle_deterministically() {
        let result = decide(&inputs(
            1,
            100,
            vec![
                ObservedObstacle {
                    id: FortificationId(20),
                    kind: AntiTankKind::AntiTankDitch,
                    distance_tiles: 5,
                    detour_extra_tiles: 40,
                },
                ObservedObstacle {
                    id: FortificationId(10),
                    kind: AntiTankKind::DragonsTeeth,
                    distance_tiles: 5,
                    detour_extra_tiles: 50,
                },
            ],
        ));
        match result {
            AntiTankDoctrineDecision::Detour {
                obstacle_id,
                reason,
                ..
            } => {
                // Both at distance 5; tie-break by id (10 < 20) →
                // dragon's teeth wins → always-detour.
                assert_eq!(obstacle_id, FortificationId(10));
                assert_eq!(reason, DetourReason::DragonsTeethAlwaysDetour);
            }
            other => panic!("expected Detour, got {other:?}"),
        }
    }

    /// Decision `as_str` round-trips to the documented stable id.
    #[test]
    fn anti_tank_decision_as_str_matches_stable_id() {
        assert_eq!(AntiTankDoctrineDecision::Continue.as_str(), "continue");
        assert_eq!(
            AntiTankDoctrineDecision::Detour {
                obstacle_id: FortificationId(1),
                obstacle_kind: AntiTankKind::AntiTankDitch,
                detour_extra_tiles: 0,
                reason: DetourReason::DitchShortDetour,
            }
            .as_str(),
            "detour"
        );
        assert_eq!(
            AntiTankDoctrineDecision::Plow {
                obstacle_id: FortificationId(1),
                obstacle_kind: AntiTankKind::AntiTankDitch,
                suspension_hp_percent: 100,
            }
            .as_str(),
            "plow"
        );
    }

    /// DetourReason round-trips through ron via the snake_case
    /// representation used in replay-event payloads.
    #[test]
    fn detour_reason_round_trips_via_ron() {
        for r in [
            DetourReason::DragonsTeethAlwaysDetour,
            DetourReason::DitchSuspensionTooLow,
            DetourReason::DitchShortDetour,
            DetourReason::UnknownObstacle,
        ] {
            let s = r.as_str();
            let parsed: DetourReason = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, r);
        }
    }
}
