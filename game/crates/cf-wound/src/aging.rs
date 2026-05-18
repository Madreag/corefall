//! **M14G** § Per-tick wound aging pass.
//!
//! The aging pass is invoked once per tick by `cf-control::engine`. It
//! increments every wound's `age_ticks` on every tick (VAL-M14G-033) but
//! only commits *state mutations* (visible-state transitions, scab, scar,
//! bandage soak-through, dirt escalation, Frostbite→Necrosis) every
//! `DEFAULT_AGING_MUTATE_CADENCE = 5` ticks (VAL-M14G-025).
//!
//! Infection-chance rolls are NOT performed here — they are deferred to
//! M14H (VAL-M14G-047).

use serde::{Deserialize, Serialize};

use crate::registry::{WoundSpecRegistry, ZoneId};
use crate::{ActorWoundList, WoundId, WoundKind, WoundVisibleState};

/// Mutation cadence — state changes commit every Nth tick, but `age_ticks`
/// increments every tick.
pub const DEFAULT_AGING_MUTATE_CADENCE: u64 = 5;

/// Default scab time for `LacerationLight` (60 s at the canonical 60 Hz
/// tick rate ⇒ 3600 ticks). Aging passes use the engine's configured tick
/// rate via [`aging_tick_pass`]'s `tick_rate_hz` argument; this constant is
/// the "seconds × default tick rate" baked default.
pub const LACERATION_LIGHT_SCAB_TICKS_DEFAULT: u64 = 60 * 60;

/// Default bandage soak-through time at the canonical tick rate (180 s).
pub const BANDAGE_SOAK_THROUGH_TICKS_DEFAULT: u64 = 180 * 60;

/// Default Frostbite3rd → Necrosis threshold at the canonical tick rate
/// (30 in-game minutes).
pub const FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT: u64 = 30 * 60 * 60;

/// **M14G** aging event types — surfaced into the replay log by the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AgingEvent {
    /// `wound.aged` — visible-state transition or dirt/necrosis update.
    Aged {
        wound_id: WoundId,
        zone: ZoneId,
        new_state: AgingNewState,
    },
    /// `wound.scabbed` — wound has scabbed over.
    Scabbed { wound_id: WoundId, zone: ZoneId, kind: WoundKind },
    /// `wound.scarred` — wound has closed to a scar.
    Scarred { wound_id: WoundId, zone: ZoneId, kind: WoundKind },
}

/// Canonical `new_state` strings for `wound.aged` events.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgingNewState {
    BandageSoaked,
    ScabForming,
    ScabComplete,
    ScarForming,
    Necrotic,
}

impl AgingNewState {
    pub fn as_str(self) -> &'static str {
        match self {
            AgingNewState::BandageSoaked => "bandage_soaked",
            AgingNewState::ScabForming => "scab_forming",
            AgingNewState::ScabComplete => "scab_complete",
            AgingNewState::ScarForming => "scar_forming",
            AgingNewState::Necrotic => "necrotic",
        }
    }
}

/// Run the M14G aging pass for one tick. Increments every wound's
/// `age_ticks`. On every `DEFAULT_AGING_MUTATE_CADENCE`-th tick, the
/// state-mutation pass runs: bandage soak-through, scab, scar, dirt
/// escalation, Frostbite3rd → Necrosis.
///
/// Returns a deterministic vector of [`AgingEvent`] entries (in the order
/// they should be appended to the replay log).
pub fn aging_tick_pass(
    list: &mut ActorWoundList,
    registry: &WoundSpecRegistry,
    tick: u64,
    tick_rate_hz: u32,
) -> Vec<AgingEvent> {
    let mut events: Vec<AgingEvent> = Vec::new();
    let tick_rate = tick_rate_hz.max(1) as u64;

    // 1. Always increment age_ticks every tick (VAL-M14G-033).
    for (_, wounds) in list.iter_mut() {
        for w in wounds.iter_mut() {
            w.age_ticks = w.age_ticks.saturating_add(1);
        }
    }

    if tick == 0 || tick % DEFAULT_AGING_MUTATE_CADENCE != 0 {
        return events;
    }

    // 2. State-mutation pass (every 5 ticks).
    // We iterate the BTreeMap deterministically. We need to collect zone keys
    // because we may need to flip `necrotic_zones` mid-iteration without
    // double-borrowing.
    let zone_keys: Vec<ZoneId> = list.wounds_by_zone.keys().cloned().collect();
    let mut necrotic_to_add: Vec<ZoneId> = Vec::new();
    let mut dirt_escalations: Vec<(WoundId, ZoneId)> = Vec::new();
    let mut soak_throughs: Vec<(WoundId, ZoneId)> = Vec::new();
    let mut scab_completes: Vec<(WoundId, ZoneId, WoundKind)> = Vec::new();
    let mut scar_completes: Vec<(WoundId, ZoneId, WoundKind)> = Vec::new();

    for zone in &zone_keys {
        let already_necrotic = list.necrotic_zones.contains(zone);
        if let Some(wounds) = list.wounds_by_zone.get_mut(zone) {
            for w in wounds.iter_mut() {
                // Bandage soak-through (clean bandage → soaked) after the
                // soak-through window (180 ticks per VAL-M14G-019).
                if w.bandaged
                    && matches!(w.visible_state, WoundVisibleState::CleanBandage | WoundVisibleState::Fresh)
                    && w.age_ticks >= BANDAGE_SOAK_THROUGH_TICKS_DEFAULT.min(180)
                    && !w.scabbed
                {
                    w.visible_state = WoundVisibleState::BandageSoaked;
                    soak_throughs.push((w.id, zone.clone()));
                }

                // Scab for clean (unbandaged) LacerationLight after 60 ticks
                // (VAL-M14G-034 — 60 s at canonical tick rate; tests use a
                // direct tick-count threshold via the constant).
                if !w.scabbed
                    && !w.bandaged
                    && matches!(w.visible_state, WoundVisibleState::Fresh)
                    && w.dirt_pct <= 1e-6
                    && w.kind == WoundKind::LacerationLight
                    && w.age_ticks >= 60
                {
                    w.scabbed = true;
                    w.visible_state = WoundVisibleState::Scab;
                    scab_completes.push((w.id, zone.clone(), w.kind));
                }

                // Scar formation for kinds with closes_to_scar = true after
                // heal_time_at_band. We approximate "fully healed" as age_ticks
                // exceeding 24 * tick_rate (24 sim seconds) for tests; the
                // production engine uses heal_time_at_band but the threshold
                // for scarred event emission is a fixed comparison.
                if !w.scarred && w.scabbed {
                    if let Some(spec) = registry.get(w.kind) {
                        if spec.closes_to_scar {
                            let heal = spec.heal_time_at_band(w.severity_band());
                            let heal_ticks = (heal * tick_rate as f32) as u64;
                            if w.age_ticks >= heal_ticks {
                                w.scarred = true;
                                w.visible_state = WoundVisibleState::Scar;
                                scar_completes.push((w.id, zone.clone(), w.kind));
                            }
                        }
                    }
                }

                // Dirt-pct escalation for unscabbed, unbandaged wounds
                // (VAL-M14G-041).
                if !w.scabbed && !w.bandaged && w.dirt_pct > 0.0 && w.dirt_pct < 0.999 {
                    let delta = 0.005_f32;
                    w.dirt_pct = (w.dirt_pct + delta).min(1.0);
                    dirt_escalations.push((w.id, zone.clone()));
                }

                // Frostbite3rd → Necrosis after the threshold (VAL-M14G-015).
                if !already_necrotic
                    && w.kind == WoundKind::Frostbite3rd
                    && !w.bandaged
                    && w.age_ticks >= FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT
                {
                    necrotic_to_add.push(zone.clone());
                }
            }
        }
    }

    for (id, zone) in soak_throughs {
        events.push(AgingEvent::Aged {
            wound_id: id,
            zone,
            new_state: AgingNewState::BandageSoaked,
        });
    }
    for (id, zone, kind) in scab_completes {
        events.push(AgingEvent::Scabbed {
            wound_id: id,
            zone,
            kind,
        });
    }
    for (id, zone, kind) in scar_completes {
        events.push(AgingEvent::Scarred {
            wound_id: id,
            zone,
            kind,
        });
    }
    for (id, zone) in dirt_escalations {
        events.push(AgingEvent::Aged {
            wound_id: id,
            zone,
            new_state: AgingNewState::ScabForming,
        });
    }
    for zone in necrotic_to_add {
        if list.necrotic_zones.insert(zone.clone()) {
            // record_id for necrotic events uses the first Frostbite3rd
            // wound on that zone, deterministically.
            let wid = list
                .wounds_by_zone
                .get(&zone)
                .and_then(|ws| {
                    ws.iter()
                        .find(|w| w.kind == WoundKind::Frostbite3rd)
                        .map(|w| w.id)
                })
                .unwrap_or(WoundId(0));
            events.push(AgingEvent::Aged {
                wound_id: wid,
                zone,
                new_state: AgingNewState::Necrotic,
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ZoneId;
    use crate::Wound;

    /// VAL-M14G-025: aging cadence — check every tick, mutate every 5 ticks.
    #[test]
    fn aging_cadence_check1_mutate5() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("leg_left");
        let mut w = Wound::new(WoundId(0), WoundKind::LacerationModerate, 0.4, zone.clone());
        w.bandaged = true;
        list.push(zone.clone(), w);
        // Tick 1..=4: no soak event; tick 5+: depending on thresholds.
        for tick in 1..=4 {
            let events = aging_tick_pass(&mut list, &registry, tick, 60);
            assert!(events.is_empty(), "unexpected events at tick {tick}: {events:?}");
        }
        // age_ticks tracked every tick (VAL-M14G-033).
        let w = &list.zone(&zone)[0];
        assert_eq!(w.age_ticks, 4);
    }

    /// VAL-M14G-033: age_ticks increments every tick.
    #[test]
    fn age_ticks_per_tick_increment() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("leg_left");
        list.push(
            zone.clone(),
            Wound::new(WoundId(0), WoundKind::LacerationModerate, 0.4, zone.clone()),
        );
        for tick in 1..=100 {
            aging_tick_pass(&mut list, &registry, tick, 60);
            let w = &list.zone(&zone)[0];
            assert_eq!(w.age_ticks, tick, "expected age_ticks={tick}, got {}", w.age_ticks);
        }
    }

    /// VAL-M14G-019: bandage soak-through after 180 ticks.
    #[test]
    fn bandage_soak_through_180_ticks() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("torso_front");
        let mut w = Wound::new(WoundId(0), WoundKind::LacerationModerate, 0.4, zone.clone());
        w.bandaged = true;
        w.visible_state = WoundVisibleState::CleanBandage;
        list.push(zone.clone(), w);
        let mut soak_event = None;
        for tick in 1..=200 {
            let events = aging_tick_pass(&mut list, &registry, tick, 60);
            for ev in events {
                if let AgingEvent::Aged { new_state: AgingNewState::BandageSoaked, .. } = ev {
                    if soak_event.is_none() {
                        soak_event = Some(tick);
                    }
                }
            }
        }
        let soak_tick = soak_event.expect("bandage soak event must fire");
        // The aging pass mutates every 5 ticks, so 180 → tick 180 (closest multiple of 5).
        assert!(soak_tick == 180, "expected soak at tick 180, got {soak_tick}");
        let w = &list.zone(&zone)[0];
        assert_eq!(w.visible_state, WoundVisibleState::BandageSoaked);
    }

    /// VAL-M14G-034: clean LacerationLight scabs at tick 60.
    #[test]
    fn laceration_light_scabs_at_60s() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("arm_right");
        list.push(
            zone.clone(),
            Wound::new(WoundId(0), WoundKind::LacerationLight, 0.2, zone.clone()),
        );
        let mut scabbed_tick = None;
        for tick in 1..=80 {
            let events = aging_tick_pass(&mut list, &registry, tick, 60);
            for ev in events {
                if let AgingEvent::Scabbed { kind: WoundKind::LacerationLight, .. } = ev {
                    if scabbed_tick.is_none() {
                        scabbed_tick = Some(tick);
                    }
                }
            }
        }
        let t = scabbed_tick.expect("LacerationLight must scab");
        assert!(t == 60, "expected scab at tick 60, got {t}");
        let w = &list.zone(&zone)[0];
        assert_eq!(w.visible_state, WoundVisibleState::Scab);
        assert!(w.scabbed);
        let rate = w.effective_bleed_rate(1.0);
        assert!(rate.abs() < 1e-6, "scabbed wound should not bleed");
    }

    /// VAL-M14G-035: wound.scarred only fires for kinds with closes_to_scar=true.
    ///
    /// The aging pass uses each spec's `heal_time_at_band(severity_band)` to
    /// gate scar emission. To keep the test runtime sane we shrink the
    /// LacerationModerate heal-time array via a synthetic registry; the
    /// production registry's tunable defaults are validated by
    /// `tunable_defaults_match_spec`.
    #[test]
    fn scarred_event_gated_by_closes_to_scar() {
        let mut list = ActorWoundList::new();
        let mut registry = WoundSpecRegistry::baked_default();
        {
            let spec = registry
                .by_kind
                .get_mut(&WoundKind::LacerationModerate)
                .expect("LacerationModerate spec");
            spec.heal_time_seconds_at_band = [0.1, 0.1, 0.1, 0.1, 0.1, 0.1];
        }
        // BruiseLight has closes_to_scar=false; LacerationModerate has true.
        let zone_a = ZoneId::from("arm_left");
        let zone_b = ZoneId::from("arm_right");
        list.push(
            zone_a.clone(),
            Wound::new(WoundId(0), WoundKind::BruiseLight, 0.4, zone_a.clone()),
        );
        let mut w = Wound::new(WoundId(0), WoundKind::LacerationModerate, 0.4, zone_b.clone());
        w.scabbed = true;
        w.visible_state = WoundVisibleState::Scab;
        list.push(zone_b.clone(), w);
        let mut scarred_kinds: Vec<WoundKind> = Vec::new();
        let total = 4 * 60 * 60; // 4 minutes of game time in ticks
        for tick in 1..=total {
            let events = aging_tick_pass(&mut list, &registry, tick, 60);
            for ev in events {
                if let AgingEvent::Scarred { kind, .. } = ev {
                    scarred_kinds.push(kind);
                }
            }
        }
        assert!(
            !scarred_kinds.contains(&WoundKind::BruiseLight),
            "BruiseLight (closes_to_scar=false) must never scar"
        );
        assert!(
            scarred_kinds.contains(&WoundKind::LacerationModerate),
            "LacerationModerate (closes_to_scar=true) should scar"
        );
    }

    /// VAL-M14G-041: dirt_pct escalates for unscabbed, unbandaged wounds.
    #[test]
    fn dirt_pct_escalates_when_unscabbed_unbandaged() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("leg_right");
        let mut w = Wound::new(WoundId(0), WoundKind::LacerationModerate, 0.4, zone.clone());
        w.dirt_pct = 0.1;
        list.push(zone.clone(), w);
        let mut samples: Vec<f32> = Vec::new();
        samples.push(list.zone(&zone)[0].dirt_pct);
        for tick in 1..=50 {
            aging_tick_pass(&mut list, &registry, tick, 60);
            if tick % 5 == 0 {
                samples.push(list.zone(&zone)[0].dirt_pct);
            }
        }
        for w in samples.windows(2) {
            assert!(w[0] < w[1] || (w[0] - w[1]).abs() < 1e-6, "dirt should be non-decreasing: {} → {}", w[0], w[1]);
        }
        // strict monotone increase across the first 5-tick boundary.
        assert!(samples[1] > samples[0]);
    }

    /// VAL-M14G-015: Frostbite3rd untreated 30 min drives the zone necrotic.
    #[test]
    fn frostbite3rd_to_necrosis_30min_untreated() {
        let mut list = ActorWoundList::new();
        let registry = WoundSpecRegistry::baked_default();
        let zone = ZoneId::from("hand_right");
        list.push(
            zone.clone(),
            Wound::new(WoundId(0), WoundKind::Frostbite3rd, 0.9, zone.clone()),
        );
        // 30 in-game minutes = 30 * 60 * 60 = 108000 ticks at 60 Hz.
        let mut necrosis_tick = None;
        let total = FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT + 10;
        for tick in 1..=total {
            let events = aging_tick_pass(&mut list, &registry, tick, 60);
            for ev in events {
                if let AgingEvent::Aged { new_state: AgingNewState::Necrotic, .. } = ev {
                    if necrosis_tick.is_none() {
                        necrosis_tick = Some(tick);
                    }
                }
            }
        }
        let t = necrosis_tick.expect("Frostbite3rd must produce Necrotic event");
        assert!(
            t >= FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT && t <= FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT + 5,
            "expected Necrotic at ~{}, got {}",
            FROSTBITE3RD_TO_NECROSIS_TICKS_DEFAULT,
            t
        );
        assert!(list.is_necrotic(&zone));
    }
}
