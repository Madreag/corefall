//! M16B acceptance — cross-crate scenarios combining `cf-environment`
//! germ-spread + quarantine with the `cf-disease` lifecycle FSM.

use cf_disease::{
    lifecycle::{expose, tick_actor, ActorDiseases, DiseaseStage},
    DiseaseKind, DiseaseRegistry, IsolationClass, OriginId, SusceptibilityMatrix, TransmissionVector,
};
use cf_environment::germ_spread::check_foodborne_exposure;
use cf_environment::{
    classify_room, enter_quarantine, tick_room_contact_spread, ActorEpi, RoomFeatures,
};

/// Deterministically simulate `days` of pandemic spread over a fixed
/// population + seed. Returns (ever_infected, recovered, deaths).
fn run_pandemic(seed: u64, days: u64) -> (u32, u32, u32) {
    let reg = DiseaseRegistry::default_registry();
    let matrix = SusceptibilityMatrix::default_matrix();
    let strain = DiseaseKind::InfluenzaPandemic;
    let spec = reg.lookup(strain).clone();
    let n = 40usize;
    let tick_rate = 60u32;
    let step = 600u64; // 10 in-game seconds per simulation step
    let horizon = days * 86_400 * tick_rate as u64;

    let mut states: Vec<ActorDiseases> = (0..n).map(|_| ActorDiseases::with_origin(OriginId::Human)).collect();
    let mut ever_infected = vec![false; n];
    let mut dead = vec![false; n];
    // Identical initial infected set: actors 0..4.
    for i in 0..4 {
        expose(&mut states[i], i as u64, strain, TransmissionVector::Airborne, None, 0);
        ever_infected[i] = true;
    }
    let mut recovered = 0u32;
    let mut deaths = 0u32;
    let mut tick = 0u64;
    while tick <= horizon {
        let epi: Vec<ActorEpi> = states
            .iter()
            .enumerate()
            .map(|(i, s)| ActorEpi {
                actor_id: i as u64,
                origin: OriginId::Human,
                infectious_with: !dead[i] && s.find(strain).map(|d| d.stage.is_infectious()).unwrap_or(false),
                immune: s.is_immune(strain, tick),
                infected: dead[i] || s.has_active(strain),
            })
            .collect();
        for req in tick_room_contact_spread(strain, &spec, &matrix, &epi, tick, seed) {
            let i = req.actor_id as usize;
            if !dead[i]
                && expose(&mut states[i], req.actor_id, req.disease, req.vector, req.source_item_id, tick).is_some()
            {
                ever_infected[i] = true;
            }
        }
        for i in 0..n {
            if dead[i] {
                continue;
            }
            let out = tick_actor(&mut states[i], i as u64, &reg, tick, tick_rate, seed);
            recovered += out.recovered.len() as u32;
            if !out.died.is_empty() {
                deaths += out.died.len() as u32;
                dead[i] = true;
            }
        }
        tick += step;
    }
    (ever_infected.iter().filter(|x| **x).count() as u32, recovered, deaths)
}

#[test]
fn determinism_across_pandemic_spread() {
    let a = run_pandemic(0xC0FFEE, 12);
    let b = run_pandemic(0xC0FFEE, 12);
    assert_eq!(a, b, "identical seed must reproduce identical (infected, recovered, deaths)");
    // The outbreak must actually spread beyond the initial 4 and resolve outcomes.
    assert!(a.0 > 4, "pandemic must spread to new actors; got {} infected", a.0);
    assert!(a.1 + a.2 > 0, "the initial cohort must reach outcomes within 12 days");
}

#[test]
fn different_seeds_are_independently_deterministic() {
    let a1 = run_pandemic(1, 10);
    let a2 = run_pandemic(1, 10);
    let b1 = run_pandemic(2, 10);
    let b2 = run_pandemic(2, 10);
    assert_eq!(a1, a2);
    assert_eq!(b1, b2);
}

#[test]
fn foodborne_outbreak_traces_to_dish_for_three_actors() {
    let reg = DiseaseRegistry::default_registry();
    let matrix = SusceptibilityMatrix::default_matrix();
    let mut states: Vec<ActorDiseases> = (0..3).map(|_| ActorDiseases::with_origin(OriginId::Human)).collect();

    // 3 actors eat from a below-threshold batch.
    for (i, state) in states.iter_mut().enumerate() {
        let req = check_foodborne_exposure(i as u64, OriginId::Human, &matrix, "mystery_stew", 0.2, 0.5, false)
            .expect("low-quality dish exposes the eater");
        assert_eq!(req.vector, TransmissionVector::Foodborne);
        assert_eq!(req.source_item_id.as_deref(), Some("mystery_stew"));
        let ev = expose(state, i as u64, req.disease, req.vector, req.source_item_id, 0).unwrap();
        assert_eq!(ev.pathogen, DiseaseKind::FoodPoisoning);
    }

    // First lifecycle tick: Exposed -> Incubating for all 3.
    for (i, state) in states.iter_mut().enumerate() {
        let out = tick_actor(state, i as u64, &reg, 1, 60, 7);
        assert!(out
            .stage_changed
            .iter()
            .any(|e| e.from == DiseaseStage::Exposed && e.to == DiseaseStage::Incubating));
    }

    // Advance past incubation (1h) → Incubating -> Manifest.
    let incub_ticks = (3600.0 * 60.0) as u64 + 5;
    let mut manifested = [false; 3];
    for tick in 2..=incub_ticks {
        for (i, state) in states.iter_mut().enumerate() {
            let out = tick_actor(state, i as u64, &reg, tick, 60, 7);
            if out.stage_changed.iter().any(|e| e.to == DiseaseStage::Manifest) {
                manifested[i] = true;
            }
        }
    }
    assert!(manifested.iter().all(|m| *m), "all 3 eaters must reach Manifest");
}

#[test]
fn class_a_quarantine_for_tb_seals_room() {
    // A fully-equipped isolation room grades Class A.
    let room = classify_room(&RoomFeatures::class_a());
    assert_eq!(room, IsolationClass::ClassA);
    let outcome = enter_quarantine(7, DiseaseKind::Tuberculosis, IsolationClass::ClassA, room, 999)
        .expect("class A room quarantines a TB patient");
    assert_eq!(outcome.event.room_class, IsolationClass::ClassA);
    assert!(outcome.close_airlock, "class A quarantine closes the airlock (M28D damper)");
}
