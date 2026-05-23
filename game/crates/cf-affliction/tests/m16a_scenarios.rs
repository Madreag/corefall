//! M16A § Acceptance scenario kernel-level tests.
//!
//! These tests drive the cf-affliction::env kernel through the literal
//! signal profiles described in the M16A spec's Gherkin acceptance
//! scenarios. They do not require the full engine; they validate that
//! the kernel produces the right event sequence + state when given the
//! atmospheric / electric / wet / heavy-weapon / spotlight / razor-wire
//! signals the producers (M19/M28/M9) deliver.

use cf_affliction::env::{
    self, AtmosphericSusceptibility, EnvAfflictionKind, EnvAfflictionRegistry, EnvAfflictionState,
    EnvSeverity, EnvSignal, OriginId,
};

fn human() -> AtmosphericSusceptibility {
    AtmosphericSusceptibility::for_origin(OriginId::Human)
}

fn registry() -> EnvAfflictionRegistry {
    EnvAfflictionRegistry::default_registry()
}

#[test]
fn stuffiness_crowded_room_then_ventilate_clears() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let crowded = EnvSignal {
        humidity_pct: 80.0,
        co2_partial_kpa: 0.5,
        occupant_count: 6,
        ..Default::default()
    };
    for tick in 0..620 {
        env::tick_all(
            &mut state,
            7,
            human(),
            &crowded,
            &reg,
            1.0,
            Some(format!("m16a:{tick}")),
        );
    }
    let mild = state
        .last_threshold(EnvAfflictionKind::Stuffiness);
    assert!(matches!(mild, EnvSeverity::Mild | EnvSeverity::Moderate));
    let ventilated = EnvSignal {
        humidity_pct: 50.0,
        co2_partial_kpa: 0.1,
        occupant_count: 6,
        ..Default::default()
    };
    let mut cleared = false;
    for tick in 0..1000 {
        let out = env::tick_all(
            &mut state,
            7,
            human(),
            &ventilated,
            &reg,
            1.0,
            Some(format!("m16a:{tick}")),
        );
        if out.cleared.iter().any(|e| e.kind == EnvAfflictionKind::Stuffiness) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "stuffiness must clear when CO2 drops below 0.2%");
}

#[test]
fn mimas_uninsulated_room_drives_hypothermia_for_human() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let cold = EnvSignal {
        room_temp_k: 230.0,
        ..Default::default()
    };
    let mut crossed_mild = false;
    for tick in 0..40 {
        let out = env::tick_all(&mut state, 1, human(), &cold, &reg, 1.0, Some(format!("mimas:{tick}")));
        if out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Hypothermia)
        {
            crossed_mild = true;
        }
    }
    assert!(crossed_mild, "expected hypothermia.env_threshold_crossed");
    assert!(
        state.accumulator(EnvAfflictionKind::Hypothermia).kind_value >= 300.0,
        "expected cold_load >= 300 C*s after 40 s in 230 K"
    );
    let airlock = EnvSignal {
        room_temp_k: 293.0,
        ..Default::default()
    };
    let mut cleared = false;
    for tick in 0..400 {
        let out = env::tick_all(
            &mut state,
            1,
            human(),
            &airlock,
            &reg,
            1.0,
            Some(format!("airlock:{tick}")),
        );
        if out
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Hypothermia)
        {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "5 min in airlock must clear hypothermia");
}

#[test]
fn halon_dump_progresses_asphyxiation_through_lethal() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let halon = EnvSignal {
        o2_partial_kpa: 10.0,
        ..Default::default()
    };
    let mut max_band = EnvSeverity::None;
    for tick in 0..120 {
        let out = env::tick_all(&mut state, 1, human(), &halon, &reg, 1.0, Some(format!("halon:{tick}")));
        for ev in &out.threshold_crossed {
            if ev.kind == EnvAfflictionKind::Asphyxiation && ev.severity > max_band {
                max_band = ev.severity;
            }
        }
    }
    assert!(
        max_band >= EnvSeverity::Severe,
        "expected severe+ asphyxiation; got {:?}",
        max_band
    );
    assert!(
        state.accumulator(EnvAfflictionKind::Asphyxiation).kind_value >= 90.0,
        "expected asphyxia_seconds >= 90"
    );
}

#[test]
fn refrigerant_r22_leak_triggers_mild_and_feeds_m16b() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let leak = EnvSignal {
        refrigerant_partial_kpa: 10.0,
        ..Default::default()
    };
    for tick in 0..10 {
        env::tick_all(&mut state, 1, human(), &leak, &reg, 1.0, Some(format!("r22:{tick}")));
    }
    assert!(
        state.accumulator(EnvAfflictionKind::RefrigerantInhalation).kind_value >= 50.0,
        "expected refrigerant accumulator >= 50 kPa*s"
    );
    assert!(state.m16b_sepsis_feed, "M16B sepsis-feed flag must be set");
}

#[test]
fn electric_fence_three_shocks_escalate_to_unconscious() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let shock = EnvSignal {
        electric_shock_event_j: 80.0,
        ..Default::default()
    };
    let mut last_band = EnvSeverity::None;
    for tick in 0..3 {
        let out = env::tick_all(&mut state, 1, human(), &shock, &reg, 1.0 / 30.0, Some(format!("shock:{tick}")));
        for ev in &out.threshold_crossed {
            if ev.kind == EnvAfflictionKind::Electrocution && ev.severity > last_band {
                last_band = ev.severity;
            }
        }
    }
    assert!(
        last_band >= EnvSeverity::Severe,
        "3 shocks should escalate to severe (unconscious); got {:?}",
        last_band
    );
    assert!(
        state.accumulator(EnvAfflictionKind::Electrocution).arc_count >= 3,
        "arc_count must >= 3"
    );
}

#[test]
fn razor_wire_crossings_stack_bleed() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let wire = EnvSignal {
        razor_wire_contact: true,
        ..Default::default()
    };
    let neutral = EnvSignal::default();
    for _ in 0..3 {
        env::tick_all(&mut state, 1, human(), &wire, &reg, 1.0, None);
        env::tick_all(&mut state, 1, human(), &neutral, &reg, 1.0, None);
    }
    assert!(
        state.accumulator(EnvAfflictionKind::Laceration).bleed_stack >= 3,
        "3 crossings → bleed_stack >= 3"
    );
    assert!(state.bleed_stack_total >= 3);
    assert!(state.m16b_sepsis_feed, "laceration feeds M16B sepsis pathway");
}

#[test]
fn trench_foot_2h_wet_then_24h_dry_cycle_clears() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let wet = EnvSignal {
        wet_duckboard_contact: true,
        ..Default::default()
    };
    for _ in 0..7200 {
        env::tick_all(&mut state, 1, human(), &wet, &reg, 1.0, None);
    }
    assert!(
        state.accumulator(EnvAfflictionKind::TrenchFoot).kind_value >= 7200.0,
        "2h on wet duckboard"
    );
    let dry = EnvSignal {
        wet_duckboard_contact: false,
        feet_dry_and_warm: true,
        ..Default::default()
    };
    let mut cleared = false;
    for _ in 0..86400 {
        let out = env::tick_all(&mut state, 1, human(), &dry, &reg, 1.0, None);
        if out.cleared.iter().any(|e| e.kind == EnvAfflictionKind::TrenchFoot) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "24h dry+warm clears trench foot");
}

#[test]
fn spotlight_cone_reveals_then_leaving_clears() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let lit = EnvSignal {
        spotlight_lit: true,
        ..Default::default()
    };
    let out_lit = env::tick_all(&mut state, 1, human(), &lit, &reg, 1.0, None);
    assert!(out_lit.reveal_to_ai);
    let dark = EnvSignal {
        spotlight_lit: false,
        ..Default::default()
    };
    let out_dark = env::tick_all(&mut state, 1, human(), &dark, &reg, 1.0, None);
    assert!(out_dark.cleared.iter().any(|e| e.kind == EnvAfflictionKind::Illuminated));
}

#[test]
fn heavy_weapon_load_then_drop_clears() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let loaded = EnvSignal {
        heavy_weapon_kg: 420.0,
        baseline_carry_kg: 20.0,
        ..Default::default()
    };
    let out_load = env::tick_all(&mut state, 1, human(), &loaded, &reg, 1.0, None);
    assert!(out_load.stamina_drain_multiplier > 1.0);
    let dropped = EnvSignal {
        heavy_weapon_kg: 20.0,
        baseline_carry_kg: 20.0,
        ..Default::default()
    };
    let out_drop = env::tick_all(&mut state, 1, human(), &dropped, &reg, 1.0, None);
    assert!(
        out_drop
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::StaminaMovementCost)
    );
}

#[test]
fn analyzer_alarm_panics_then_stabilize_clears() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let alarm = EnvSignal {
        analyzer_alarm_unaddressed: true,
        ..Default::default()
    };
    let out_alarm = env::tick_all(&mut state, 1, human(), &alarm, &reg, 1.0 / 30.0, None);
    assert!(out_alarm.panic_freeze_ticks > 0);
    let stabilize = EnvSignal {
        stabilize_assist: true,
        ..Default::default()
    };
    let out_stab = env::tick_all(&mut state, 1, human(), &stabilize, &reg, 1.0 / 30.0, None);
    assert!(
        out_stab
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::PanicFreezeEnv)
    );
}

#[test]
fn methane_breather_immune_to_asphyxiation_in_o2_atmosphere() {
    let reg = registry();
    let methane = AtmosphericSusceptibility::for_origin(OriginId::MethaneBreather);
    let mut state = EnvAfflictionState::default();
    let earth_atmo = EnvSignal {
        o2_partial_kpa: 21.0,
        ..Default::default()
    };
    let out = env::tick_all(&mut state, 1, methane, &earth_atmo, &reg, 1.0, None);
    let immune = out
        .origin_immune
        .iter()
        .find(|e| e.kind == EnvAfflictionKind::Asphyxiation);
    assert!(immune.is_some(), "methane breather must emit asphyxiation immune");
    let immune = immune.unwrap();
    assert_eq!(immune.reason, "oxygen_toxic_origin");
    assert_eq!(immune.alt_kind, Some(EnvAfflictionKind::RefrigerantInhalation));
    assert!(
        state.accumulator(EnvAfflictionKind::RefrigerantInhalation).kind_value > 0.0,
        "alt kind (refrigerant_inhalation kernel) must accumulate oxygen-toxicity"
    );
}

#[test]
fn determinism_across_two_runs_with_identical_seed() {
    let reg = registry();
    let signal = EnvSignal {
        room_temp_k: 230.0,
        wet_duckboard_contact: true,
        spotlight_lit: true,
        electric_shock_event_j: 60.0,
        refrigerant_partial_kpa: 5.0,
        razor_wire_contact: true,
        analyzer_alarm_unaddressed: true,
        heavy_weapon_kg: 200.0,
        baseline_carry_kg: 20.0,
        humidity_pct: 85.0,
        co2_partial_kpa: 0.5,
        occupant_count: 6,
        o2_partial_kpa: 10.0,
        ..Default::default()
    };
    let run = || -> (Vec<f32>, EnvAfflictionState) {
        let mut state = EnvAfflictionState::default();
        let mut acc_trace = Vec::new();
        for tick in 0..600u64 {
            env::tick_all(&mut state, 11, human(), &signal, &reg, 1.0 / 30.0, Some(format!("det:{tick}")));
            for k in EnvAfflictionKind::all() {
                acc_trace.push(state.accumulator(*k).kind_value);
                acc_trace.push(state.severity(*k));
            }
        }
        (acc_trace, state)
    };
    let (trace_a, state_a) = run();
    let (trace_b, state_b) = run();
    assert_eq!(trace_a, trace_b, "accumulator + severity traces must match exactly");
    assert_eq!(state_a, state_b, "final env state must match exactly");
}

#[test]
fn auto_triage_hook_severity_surfaces_for_medic_doctrine() {
    let reg = registry();
    let mut state = EnvAfflictionState::default();
    let very_hot = EnvSignal {
        room_temp_k: 380.0,
        ..Default::default()
    };
    for tick in 0..60 {
        env::tick_all(&mut state, 1, human(), &very_hot, &reg, 1.0, Some(format!("heat:{tick}")));
    }
    let heat_sev = state.severity(EnvAfflictionKind::Heatstroke);
    assert!(heat_sev >= 0.8, "expected heatstroke severity >= 0.8, got {heat_sev}");
    let mild_cold = EnvSignal {
        room_temp_k: 250.0,
        ..Default::default()
    };
    let mut state2 = EnvAfflictionState::default();
    for tick in 0..15 {
        env::tick_all(&mut state2, 1, human(), &mild_cold, &reg, 1.0, Some(format!("cold:{tick}")));
    }
    let cold_sev = state2.severity(EnvAfflictionKind::Hypothermia);
    assert!(cold_sev > 0.0 && cold_sev < 0.5, "expected mild hypothermia, got {cold_sev}");
}
