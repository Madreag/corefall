//! M9C: AI-OBS-A-01 doctrine — spotter actor in a watchtower /
//! observation post emits a `spotter_target_marked` event with a 3-second
//! TTL; squad MGs / snipers consume the mark for +50% acquisition rate.
//!
//! Spec § "Notes for the implementer":
//!
//! > **AI-OBS-A-01** = "if I see a target my squad cannot, broadcast a
//! > mark with TTL 3s." Doctrine output is the `spotter_target_marked`
//! > event; consumers handle the bonus.
//!
//! > **Spotter mark** has a TTL (3s after LOS break) + a per-target
//! > unique mark (multiple spotters can stack-mark for + bonus). Don't
//! > let marks pile up infinitely; cap at 1 per target.
//!
//! Spec § Acceptance criteria scenario:
//!
//! > Given a friendly spotter actor in a `watchtower_t2` with LOS to an
//! > enemy AND a friendly MG nest 16 tiles away with LOS-blocked-by-
//! > terrain to the same enemy
//! > When AI-OBS-A-01 doctrine evaluates the spotter
//! > Then spotter_target_marked event fires with target_id + target_pos
//! > And the MG nest's target_acquisition_rate increases by 50%
//! > And the MG nest's UI shows the marked target with a yellow chevron
//! > When the spotter is killed or LOS breaks for > 3s
//! > Then the mark expires and the bonus drops
//!
//! This module owns the pure decision function. The cf-control engine
//! consumes the per-tick [`ObserverDoctrineDecision`] to dispatch the
//! `spotter_target_marked` replay-event emission + per-target mark
//! table update.
//!
//! VAL-M9C-021 / VAL-M9C-022 / VAL-M9C-045 land here.

use serde::{Deserialize, Serialize};

use cf_actor::ActorId;
use cf_fortification::SpotterMark;

/// Doctrine id used in AI archetype RON files + replay event payloads.
pub const DOCTRINE_ID: &str = "AI-OBS-A-01";

/// Spec line: "broadcast a mark with TTL 3s".
pub const SPOTTER_MARK_TTL_SECONDS: f32 = 3.0;

/// Spec § "Notes for the implementer": "Spotter mark has a TTL (3s
/// after LOS break)". The LOS-loss window equals the mark TTL: 3s.
pub const SPOTTER_MARK_LOS_LOSS_TTL_SECONDS: f32 = 3.0;

/// Spec § "Spotter role" + Gherkin "the MG nest's target_acquisition_rate
/// increases by 50%". cf-ai target-selection consumers apply this
/// multiplier (re-exported from cf-fortification).
pub use cf_fortification::SPOTTER_TARGET_MARK_ACQUISITION_BONUS;

/// One spotter's per-tick perception of one candidate target — the
/// information the doctrine needs to decide whether to emit / refresh /
/// expire a mark. cf-control engine pre-filters by faction + LOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverDoctrineInputs {
    pub spotter_actor_id: ActorId,
    pub target_actor_id: ActorId,
    pub target_pos_tiles: (i32, i32),
    /// True when the spotter currently has line-of-sight to the target
    /// (engine resolves visibility per the M22 visibility pipeline).
    pub spotter_has_los: bool,
    /// True when the spotter's squad (excluding the spotter) cannot see
    /// the target. The spec line "see a target my squad cannot" gates
    /// the mark emission — no point in marking what's already known.
    pub squad_has_los: bool,
    /// Existing per-target mark in the mark table (None when no mark
    /// is in flight).
    pub prior_mark: Option<SpotterMark>,
    /// Current simulation tick.
    pub tick_index: u64,
    /// Tick rate (Hz). Used to convert the 3-second TTL constants to a
    /// tick count. Per project AGENTS.md, never hardcode `60`.
    pub tick_rate_hz: u32,
}

/// One observer-doctrine decision. cf-control engine consumes this
/// each tick to update the mark table and emit
/// `spotter_target_marked` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ObserverDoctrineDecision {
    /// No mark this tick (squad already has LOS, or spotter has no LOS
    /// and there is no prior mark).
    Idle,
    /// Emit a fresh `spotter_target_marked` event AND insert the mark
    /// into the per-target mark table. Used when there is no prior mark
    /// for this target.
    EmitMark {
        target_id: ActorId,
        target_pos_tiles: (i32, i32),
        ttl_ticks: u32,
    },
    /// Refresh an existing mark for this target (per spec § "cap at 1
    /// per target"): update `last_los_tick` + bump `target_pos_tiles`
    /// without emitting a fresh event. The mark TTL is extended.
    RefreshMark {
        target_id: ActorId,
        target_pos_tiles: (i32, i32),
    },
    /// Spotter has lost LOS to the target for > 3 s OR the mark itself
    /// is older than 3s: expire the existing mark.
    ExpireMark { target_id: ActorId },
}

impl ObserverDoctrineDecision {
    /// Stable name used for replay event payloads + cfctl HUD overlays.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ObserverDoctrineDecision::Idle => "idle",
            ObserverDoctrineDecision::EmitMark { .. } => "emit_mark",
            ObserverDoctrineDecision::RefreshMark { .. } => "refresh_mark",
            ObserverDoctrineDecision::ExpireMark { .. } => "expire_mark",
        }
    }
}

/// Convert the seconds-valued TTL constant to a tick count given the
/// engine's tick rate. Helper exposed for cf-control + tests.
#[must_use]
pub fn ttl_ticks_for(seconds: f32, tick_rate_hz: u32) -> u32 {
    let raw = (seconds * tick_rate_hz as f32).round();
    // Saturate the cast at u32::MAX so a misconfigured tick_rate
    // never panics in debug builds.
    if raw <= 0.0 {
        0
    } else if raw >= f32::from(u16::MAX) * f32::from(u16::MAX) {
        u32::MAX
    } else {
        raw as u32
    }
}

/// Returns true when the supplied mark has expired given the current
/// tick (TTL OR LOS-loss window).
#[must_use]
pub fn mark_expired(mark: &SpotterMark, tick_index: u64, tick_rate_hz: u32) -> bool {
    let ttl_ticks = u64::from(ttl_ticks_for(SPOTTER_MARK_TTL_SECONDS, tick_rate_hz));
    let los_loss_ticks =
        u64::from(ttl_ticks_for(SPOTTER_MARK_LOS_LOSS_TTL_SECONDS, tick_rate_hz));

    let mark_age = tick_index.saturating_sub(mark.mark_tick);
    let los_age = tick_index.saturating_sub(mark.last_los_tick);

    mark_age >= ttl_ticks || los_age >= los_loss_ticks
}

/// AI-OBS-A-01 per-tick decision function.
///
/// The decision tree (spec § Notes for the implementer):
///
/// 1. Spotter has LOS AND squad does NOT have LOS:
///    - No prior mark → `EmitMark`.
///    - Prior mark exists → `RefreshMark` (cap at 1 per target).
/// 2. Spotter has no LOS AND prior mark exists:
///    - Mark TTL/LOS-loss expired → `ExpireMark`.
///    - Otherwise → `Idle` (mark still alive in TTL window).
/// 3. Otherwise → `Idle`.
#[must_use]
pub fn decide(inputs: &ObserverDoctrineInputs) -> ObserverDoctrineDecision {
    if inputs.spotter_has_los {
        // Spec line gate: only mark targets the squad CANNOT already see.
        if inputs.squad_has_los {
            return ObserverDoctrineDecision::Idle;
        }
        match inputs.prior_mark {
            None => ObserverDoctrineDecision::EmitMark {
                target_id: inputs.target_actor_id,
                target_pos_tiles: inputs.target_pos_tiles,
                ttl_ticks: ttl_ticks_for(SPOTTER_MARK_TTL_SECONDS, inputs.tick_rate_hz),
            },
            Some(_) => ObserverDoctrineDecision::RefreshMark {
                target_id: inputs.target_actor_id,
                target_pos_tiles: inputs.target_pos_tiles,
            },
        }
    } else if let Some(mark) = inputs.prior_mark {
        if mark_expired(&mark, inputs.tick_index, inputs.tick_rate_hz) {
            ObserverDoctrineDecision::ExpireMark {
                target_id: inputs.target_actor_id,
            }
        } else {
            ObserverDoctrineDecision::Idle
        }
    } else {
        ObserverDoctrineDecision::Idle
    }
}

/// Apply a per-tick decision to the supplied mark slot. Returns the
/// post-tick mark state (or `None` when the mark was expired / never
/// existed). Used by tests + cf-control to thread the mark table.
#[must_use]
pub fn apply_decision(
    prior: Option<SpotterMark>,
    decision: ObserverDoctrineDecision,
    spotter_actor_id: ActorId,
    tick_index: u64,
) -> Option<SpotterMark> {
    match decision {
        ObserverDoctrineDecision::Idle => prior,
        ObserverDoctrineDecision::EmitMark {
            target_id,
            target_pos_tiles,
            ..
        } => Some(SpotterMark {
            spotter_actor_id: spotter_actor_id.0,
            target_actor_id: target_id.0,
            target_pos_tiles,
            mark_tick: tick_index,
            last_los_tick: tick_index,
        }),
        ObserverDoctrineDecision::RefreshMark {
            target_pos_tiles, ..
        } => prior.map(|mut m| {
            m.target_pos_tiles = target_pos_tiles;
            m.last_los_tick = tick_index;
            m
        }),
        ObserverDoctrineDecision::ExpireMark { .. } => None,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn inputs(
        prior: Option<SpotterMark>,
        spotter_los: bool,
        squad_los: bool,
        tick: u64,
        tick_rate_hz: u32,
    ) -> ObserverDoctrineInputs {
        ObserverDoctrineInputs {
            spotter_actor_id: ActorId(1),
            target_actor_id: ActorId(99),
            target_pos_tiles: (40, 40),
            spotter_has_los: spotter_los,
            squad_has_los: squad_los,
            prior_mark: prior,
            tick_index: tick,
            tick_rate_hz,
        }
    }

    /// VAL-M9C-045 line 1 + scenario "spotter_target_marked event fires":
    /// spotter has LOS, no prior mark → EmitMark (with the correct TTL).
    #[test]
    fn observer_doctrine_emits_mark() {
        let tick_rate_hz = 60u32;
        let decision = decide(&inputs(None, true, false, 100, tick_rate_hz));
        match decision {
            ObserverDoctrineDecision::EmitMark {
                target_id,
                target_pos_tiles,
                ttl_ticks,
            } => {
                assert_eq!(target_id, ActorId(99));
                assert_eq!(target_pos_tiles, (40, 40));
                assert_eq!(
                    ttl_ticks,
                    (SPOTTER_MARK_TTL_SECONDS * tick_rate_hz as f32) as u32
                );
            }
            other => panic!("expected EmitMark, got {other:?}"),
        }
    }

    /// VAL-M9C-022 + VAL-M9C-045: TTL = 3s. With a 60 Hz tick rate the
    /// mark expires at exactly 180 ticks after `last_los_tick`.
    #[test]
    fn observer_doctrine_mark_ttl_3s() {
        let tick_rate_hz = 60u32;
        let ttl_ticks = ttl_ticks_for(SPOTTER_MARK_TTL_SECONDS, tick_rate_hz);
        assert_eq!(ttl_ticks, 180);

        let mark = SpotterMark {
            spotter_actor_id: 1,
            target_actor_id: 99,
            target_pos_tiles: (40, 40),
            mark_tick: 100,
            last_los_tick: 100,
        };

        // Within TTL → not expired.
        assert!(!mark_expired(&mark, 100 + ttl_ticks as u64 - 1, tick_rate_hz));
        // At TTL boundary → expired.
        assert!(mark_expired(&mark, 100 + ttl_ticks as u64, tick_rate_hz));
        // Beyond TTL → expired.
        assert!(mark_expired(&mark, 100 + ttl_ticks as u64 + 50, tick_rate_hz));

        // Doctrine returns ExpireMark when spotter loses LOS + window
        // closes.
        let decision = decide(&inputs(
            Some(mark),
            false,
            false,
            100 + ttl_ticks as u64,
            tick_rate_hz,
        ));
        assert!(matches!(
            decision,
            ObserverDoctrineDecision::ExpireMark { target_id } if target_id == ActorId(99)
        ));

        // Doctrine returns Idle while still within TTL after LOS loss.
        let decision = decide(&inputs(
            Some(mark),
            false,
            false,
            100 + 50,
            tick_rate_hz,
        ));
        assert_eq!(decision, ObserverDoctrineDecision::Idle);
    }

    /// VAL-M9C-045 line 3 + spec § "cap at 1 per target": a second
    /// emission against the same target returns RefreshMark, not
    /// EmitMark (no stacking-inflation).
    #[test]
    fn observer_doctrine_one_mark_per_target() {
        let tick_rate_hz = 60u32;
        let mut tick: u64 = 100;
        // First tick: EmitMark emits.
        let initial_decision = decide(&inputs(None, true, false, tick, tick_rate_hz));
        assert!(matches!(
            initial_decision,
            ObserverDoctrineDecision::EmitMark { .. }
        ));

        // Apply the decision to derive the prior mark for the next
        // tick.
        let mark_after_emit =
            apply_decision(None, initial_decision, ActorId(1), tick).expect("emit emits a mark");

        // Subsequent ticks while LOS holds → RefreshMark (no new event).
        tick += 1;
        let inputs_tick2 = ObserverDoctrineInputs {
            target_pos_tiles: (41, 40),
            ..inputs(Some(mark_after_emit), true, false, tick, tick_rate_hz)
        };
        let decision2 = decide(&inputs_tick2);
        match decision2 {
            ObserverDoctrineDecision::RefreshMark {
                target_id,
                target_pos_tiles,
            } => {
                assert_eq!(target_id, ActorId(99));
                assert_eq!(target_pos_tiles, (41, 40));
            }
            other => panic!("expected RefreshMark, got {other:?}"),
        }

        // Apply the refresh; mark stays single (NOT duplicated) and
        // last_los_tick advances.
        let refreshed =
            apply_decision(Some(mark_after_emit), decision2, ActorId(1), tick)
                .expect("refresh keeps the mark");
        assert_eq!(refreshed.target_actor_id, 99);
        assert_eq!(refreshed.last_los_tick, tick);
        assert_eq!(refreshed.target_pos_tiles, (41, 40));
        assert_eq!(
            refreshed.mark_tick, mark_after_emit.mark_tick,
            "RefreshMark must NOT bump mark_tick (no stacking inflation)"
        );

        // Refresh runs N more times — mark count stays at 1
        // throughout. We verify the cap by asserting the doctrine
        // returns RefreshMark every tick after the first.
        for advance in 2..10 {
            let inputs_n = ObserverDoctrineInputs {
                ..inputs(Some(refreshed), true, false, tick + advance, tick_rate_hz)
            };
            let dn = decide(&inputs_n);
            assert!(
                matches!(dn, ObserverDoctrineDecision::RefreshMark { .. }),
                "tick {} must be RefreshMark not EmitMark",
                tick + advance
            );
        }
    }

    /// Spec line gate: "see a target my squad cannot" — if the squad
    /// already has LOS, no mark fires (no point).
    #[test]
    fn observer_doctrine_idle_when_squad_already_has_los() {
        let tick_rate_hz = 60u32;
        let decision = decide(&inputs(None, true, true, 100, tick_rate_hz));
        assert_eq!(decision, ObserverDoctrineDecision::Idle);
    }

    /// No prior mark + no LOS → idle (nothing to refresh or expire).
    #[test]
    fn observer_doctrine_idle_when_no_los_no_prior_mark() {
        let tick_rate_hz = 60u32;
        let decision = decide(&inputs(None, false, false, 100, tick_rate_hz));
        assert_eq!(decision, ObserverDoctrineDecision::Idle);
    }

    #[test]
    fn observer_doctrine_expires_on_los_loss_over_3s() {
        let tick_rate_hz = 60u32;
        let mark = SpotterMark {
            spotter_actor_id: 1,
            target_actor_id: 99,
            target_pos_tiles: (40, 40),
            mark_tick: 100,
            // Last LOS at tick 100 — 3.5s × 60 = 210 ticks since LOS
            // loss.
            last_los_tick: 100,
        };

        // 3.5s after the mark (last_los_tick same) → LOS-loss exceeds
        // 3s → expire.
        let decision = decide(&inputs(Some(mark), false, false, 310, tick_rate_hz));
        assert!(matches!(
            decision,
            ObserverDoctrineDecision::ExpireMark { .. }
        ));

        // Apply the expire decision → mark is removed.
        let after = apply_decision(Some(mark), decision, ActorId(1), 310);
        assert!(after.is_none());
    }

    /// Refresh updates last_los_tick + target position; mark_tick stays.
    #[test]
    fn observer_doctrine_refresh_updates_last_los_tick() {
        let mark = SpotterMark {
            spotter_actor_id: 1,
            target_actor_id: 99,
            target_pos_tiles: (40, 40),
            mark_tick: 100,
            last_los_tick: 100,
        };
        let refreshed = apply_decision(
            Some(mark),
            ObserverDoctrineDecision::RefreshMark {
                target_id: ActorId(99),
                target_pos_tiles: (42, 41),
            },
            ActorId(1),
            150,
        )
        .expect("refresh keeps the mark");
        assert_eq!(refreshed.last_los_tick, 150);
        assert_eq!(refreshed.target_pos_tiles, (42, 41));
        assert_eq!(refreshed.mark_tick, 100);
    }

    /// Spec consumer surface: when a mark exists, target_acquisition_rate
    /// for the squad MG goes to 1.5× (spec scenario asserts +50%).
    #[test]
    fn observer_mark_grants_fifty_percent_acquisition_to_squad_mg() {
        // The acquisition multiplier is sourced from cf-fortification
        // (`spotter_acquisition_multiplier`). Re-export check + value
        // assertion lives here so the doctrine module remains the
        // single point of truth for AI-OBS-A-01 consumers.
        use cf_fortification::{spotter_acquisition_multiplier, SpotterAcquisitionInputs};

        let mark = SpotterMark {
            spotter_actor_id: 1,
            target_actor_id: 99,
            target_pos_tiles: (40, 40),
            mark_tick: 100,
            last_los_tick: 100,
        };
        let mult = spotter_acquisition_multiplier(SpotterAcquisitionInputs {
            mark: Some(mark),
            firing_actor_id: 7,
            target_actor_id: 99,
        });
        assert_eq!(mult, SPOTTER_TARGET_MARK_ACQUISITION_BONUS);
        assert_eq!(mult, 1.5);
    }

    #[test]
    fn ttl_ticks_for_handles_zero_and_high_tick_rates() {
        // 0s → 0 ticks.
        assert_eq!(ttl_ticks_for(0.0, 60), 0);
        // 3s @ 120 Hz → 360 ticks.
        assert_eq!(ttl_ticks_for(SPOTTER_MARK_TTL_SECONDS, 120), 360);
        // Non-zero seconds with 0 Hz → 0 ticks (defensive).
        assert_eq!(ttl_ticks_for(3.0, 0), 0);
    }

    #[test]
    fn decision_as_str_round_trips() {
        assert_eq!(ObserverDoctrineDecision::Idle.as_str(), "idle");
        assert_eq!(
            ObserverDoctrineDecision::EmitMark {
                target_id: ActorId(1),
                target_pos_tiles: (0, 0),
                ttl_ticks: 0,
            }
            .as_str(),
            "emit_mark"
        );
        assert_eq!(
            ObserverDoctrineDecision::RefreshMark {
                target_id: ActorId(1),
                target_pos_tiles: (0, 0),
            }
            .as_str(),
            "refresh_mark"
        );
        assert_eq!(
            ObserverDoctrineDecision::ExpireMark {
                target_id: ActorId(1),
            }
            .as_str(),
            "expire_mark"
        );
    }
}
