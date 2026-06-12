//! M16B — per-disease R0 spread + vector-specific transmission.
//!
//! Consumes the `cf-disease` registry + susceptibility matrix and produces
//! [`ExposureRequest`]s the engine turns into `cf_disease::lifecycle::expose`
//! calls (emitting `disease.exposed`). All checks are deterministic: contact
//! spread rolls a seeded value; environmental vectors are threshold gates.

use cf_disease::{
    deterministic_roll,
    registry::DiseaseSpec,
    susceptibility::SusceptibilityMatrix,
    DiseaseKind, OriginId, TransmissionVector, ItemId,
    SEPSIS_AGE_SECONDS_THRESHOLD, SEPSIS_DIRT_PCT_THRESHOLD,
};
use serde::{Deserialize, Serialize};

/// Per-tick contact transmission coefficient. Tuned so a high-R0 airborne
/// disease in a shared room reliably spreads over minutes, not instantly.
pub const CONTACT_TRANSMISSION_COEFF: f32 = 0.0008;

/// Dirt added to a wound when it contacts a rust tile (tetanus vector).
pub const WOUND_RUST_DIRT_INCREASE: f32 = 0.3;

/// One requested exposure — the engine validates + calls
/// `cf_disease::lifecycle::expose_with_dose` (passing `magnitude`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExposureRequest {
    pub actor_id: u64,
    pub disease: DiseaseKind,
    pub vector: TransmissionVector,
    pub source_item_id: Option<ItemId>,
    /// Exposure dose multiplier (1.0 = baseline). Scales the lethality roll
    /// for dose-scaled diseases (radiation sickness).
    pub magnitude: f32,
}

impl ExposureRequest {
    fn new(actor_id: u64, disease: DiseaseKind, vector: TransmissionVector, source: Option<ItemId>) -> Self {
        Self {
            actor_id,
            disease,
            vector,
            source_item_id: source,
            magnitude: 1.0,
        }
    }

    fn with_magnitude(
        actor_id: u64,
        disease: DiseaseKind,
        vector: TransmissionVector,
        source: Option<ItemId>,
        magnitude: f32,
    ) -> Self {
        Self {
            actor_id,
            disease,
            vector,
            source_item_id: source,
            magnitude,
        }
    }
}

/// Epidemiological snapshot of one actor for the contact-spread driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorEpi {
    pub actor_id: u64,
    pub origin: OriginId,
    /// Currently infectious with the disease under evaluation.
    pub infectious_with: bool,
    /// Protective immunity (vaccine record active OR natural immunity).
    pub immune: bool,
    /// Already has an active infection of this disease.
    pub infected: bool,
}

/// Per-tick exposure probability from contact with `infectious_count`
/// infectious actors, scaled by the susceptibility multiplier. Uses the
/// effective R0 of the disease's contagious vector (from its
/// `transmission_vector_table`), so e.g. plague's weaker pneumonic route
/// spreads slower than its primary critter route.
pub fn contact_exposure_probability(
    spec: &DiseaseSpec,
    infectious_count: u32,
    susceptibility_mult: f32,
) -> f32 {
    if infectious_count == 0 || susceptibility_mult <= 0.0 {
        return 0.0;
    }
    let r0 = spec
        .contagious_vector()
        .map_or(spec.r0_per_exposure_event, |v| spec.r0_for_vector(v));
    (r0 * susceptibility_mult * infectious_count as f32 * CONTACT_TRANSMISSION_COEFF).clamp(0.0, 0.95)
}

/// Drive one tick of contact (airborne / close-contact / waterborne-pool)
/// spread for `disease` across a co-located population. Returns the
/// exposures that fire this tick. Deterministic: each susceptible rolls
/// `deterministic_roll(seed, actor, disease, tick)`.
pub fn tick_room_contact_spread(
    disease: DiseaseKind,
    spec: &DiseaseSpec,
    matrix: &SusceptibilityMatrix,
    population: &[ActorEpi],
    tick: u64,
    seed: u64,
) -> Vec<ExposureRequest> {
    if !spec.human_to_human {
        return Vec::new();
    }
    let infectious_count = population.iter().filter(|a| a.infectious_with).count() as u32;
    if infectious_count == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for actor in population {
        if actor.infectious_with || actor.infected || actor.immune {
            continue;
        }
        let mult = matrix.multiplier(actor.origin, disease);
        if mult <= 0.0 {
            continue;
        }
        let prob = contact_exposure_probability(spec, infectious_count, mult);
        let roll = deterministic_roll(seed, actor.actor_id, disease, tick);
        if roll < prob {
            out.push(ExposureRequest::new(actor.actor_id, disease, spec.primary_vector, None));
        }
    }
    out
}

/// Per-actor exposure decision for a single source (used by tests + the
/// engine's single-actor airborne check). Returns `None` when the actor is
/// immune (origin multiplier 0), already infected, or vaccinated.
pub fn check_contact_exposure(
    actor_id: u64,
    origin: OriginId,
    disease: DiseaseKind,
    spec: &DiseaseSpec,
    matrix: &SusceptibilityMatrix,
    vaccine_immune: bool,
    already_infected: bool,
    infectious_count: u32,
    tick: u64,
    seed: u64,
) -> Option<ExposureRequest> {
    if vaccine_immune || already_infected {
        return None;
    }
    let mult = matrix.multiplier(origin, disease);
    if mult <= 0.0 {
        return None;
    }
    let prob = contact_exposure_probability(spec, infectious_count, mult);
    if deterministic_roll(seed, actor_id, disease, tick) < prob {
        Some(ExposureRequest::new(actor_id, disease, spec.primary_vector, None))
    } else {
        None
    }
}

/// Foodborne exposure: every actor that eats a batch with `food_quality`
/// below threshold is exposed to food poisoning (deterministic — point
/// source), unless origin-immune or already infected.
pub fn check_foodborne_exposure(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    dish_item_id: &str,
    food_quality: f32,
    quality_threshold: f32,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected || food_quality >= quality_threshold {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::FoodPoisoning) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(
        actor_id,
        DiseaseKind::FoodPoisoning,
        TransmissionVector::Foodborne,
        Some(dish_item_id.to_string()),
    ))
}

/// Waterborne exposure: contact with a polluted-water tile (M15) exposes the
/// actor to cholera or typhoid (deterministic gate).
pub fn check_waterborne_exposure(
    actor_id: u64,
    origin: OriginId,
    disease: DiseaseKind,
    matrix: &SusceptibilityMatrix,
    polluted_water_contact: bool,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected
        || !polluted_water_contact
        || !matches!(disease, DiseaseKind::Cholera | DiseaseKind::Typhoid)
    {
        return None;
    }
    if matrix.multiplier(origin, disease) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(actor_id, disease, TransmissionVector::Waterborne, None))
}

/// Tetanus from a wound contacting a rust tile (M14G + M15). Returns the
/// exposure plus the dirt-percentage increase to apply to the wound.
pub fn check_wound_contact_tetanus(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    has_open_wound: bool,
    touches_rust_tile: bool,
    already_infected: bool,
) -> Option<(ExposureRequest, f32)> {
    if already_infected || !has_open_wound || !touches_rust_tile {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::Tetanus) <= 0.0 {
        return None;
    }
    Some((
        ExposureRequest::new(actor_id, DiseaseKind::Tetanus, TransmissionVector::WoundContact, None),
        WOUND_RUST_DIRT_INCREASE,
    ))
}

/// Sepsis cascade: a wound with `dirt_pct` above the threshold and age above
/// the age threshold escalates to sepsis (vector = wound_infection).
pub fn check_wound_sepsis(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    wound_dirt_pct: f32,
    wound_age_seconds: f32,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected
        || wound_dirt_pct <= SEPSIS_DIRT_PCT_THRESHOLD
        || wound_age_seconds <= SEPSIS_AGE_SECONDS_THRESHOLD
    {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::Sepsis) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(
        actor_id,
        DiseaseKind::Sepsis,
        TransmissionVector::WoundInfection,
        None,
    ))
}

/// Critter-borne exposure (M19H pet/critter contact): rabies from a bite,
/// bubonic plague from rat-class contact.
pub fn check_critter_exposure(
    actor_id: u64,
    origin: OriginId,
    disease: DiseaseKind,
    matrix: &SusceptibilityMatrix,
    critter_contact: bool,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected
        || !critter_contact
        || !matches!(disease, DiseaseKind::Rabies | DiseaseKind::BubonicPlague)
    {
        return None;
    }
    if matrix.multiplier(origin, disease) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(actor_id, disease, TransmissionVector::VectorBorne, None))
}

/// Anthrax from spore exposure (cutaneous / inhalation).
pub fn check_spore_exposure(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    spore_contact: bool,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected || !spore_contact {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::Anthrax) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(
        actor_id,
        DiseaseKind::Anthrax,
        TransmissionVector::SporeExposure,
        None,
    ))
}

/// Radiation sickness once cumulative dose (M17) crosses the threshold.
pub fn check_radiation_exposure(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    cumulative_dose: f32,
    dose_threshold: f32,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected || cumulative_dose < dose_threshold {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::RadiationSickness) <= 0.0 {
        return None;
    }
    // Lethality "scales with dose": magnitude = dose / threshold, capped.
    let magnitude = (cumulative_dose / dose_threshold.max(f32::EPSILON)).clamp(1.0, 5.0);
    Some(ExposureRequest::with_magnitude(
        actor_id,
        DiseaseKind::RadiationSickness,
        TransmissionVector::RadiationDose,
        None,
        magnitude,
    ))
}

/// Cancer from cumulative radiation (M17) or toxin accumulation (M19I).
pub fn check_cancer_exposure(
    actor_id: u64,
    origin: OriginId,
    matrix: &SusceptibilityMatrix,
    accumulation: f32,
    accumulation_threshold: f32,
    already_infected: bool,
) -> Option<ExposureRequest> {
    if already_infected || accumulation < accumulation_threshold {
        return None;
    }
    if matrix.multiplier(origin, DiseaseKind::Cancer) <= 0.0 {
        return None;
    }
    Some(ExposureRequest::new(
        actor_id,
        DiseaseKind::Cancer,
        TransmissionVector::ToxinAccumulation,
        None,
    ))
}

/// (infected, total) tally across a population for the pandemic ratio. An
/// actor is "infected" when it carries any infectious disease.
pub fn population_infected_ratio(infectious_flags: &[bool]) -> (u32, u32) {
    let total = infectious_flags.len() as u32;
    let infected = infectious_flags.iter().filter(|f| **f).count() as u32;
    (infected, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_disease::DiseaseRegistry;

    fn matrix() -> SusceptibilityMatrix {
        SusceptibilityMatrix::default_matrix()
    }

    #[test]
    fn methane_breather_not_exposed_to_slimelung() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Slimelung);
        let got = check_contact_exposure(
            1,
            OriginId::MethaneBreather,
            DiseaseKind::Slimelung,
            spec,
            &matrix(),
            false,
            false,
            5,
            0,
            42,
        );
        assert!(got.is_none(), "methane breather is immune to slimelung (mult=0)");
    }

    #[test]
    fn vaccinated_actor_not_exposed() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Flu);
        let got = check_contact_exposure(
            1,
            OriginId::Human,
            DiseaseKind::Flu,
            spec,
            &matrix(),
            true, // vaccine immune
            false,
            10,
            0,
            42,
        );
        assert!(got.is_none(), "vaccinated actor must not be exposed");
    }

    #[test]
    fn foodborne_exposes_all_eaters_with_source() {
        let m = matrix();
        for actor in [10u64, 11, 12] {
            let got = check_foodborne_exposure(actor, OriginId::Human, &m, "mystery_stew", 0.2, 0.5, false)
                .expect("low-quality dish exposes eater");
            assert_eq!(got.vector, TransmissionVector::Foodborne);
            assert_eq!(got.source_item_id.as_deref(), Some("mystery_stew"));
            assert_eq!(got.disease, DiseaseKind::FoodPoisoning);
        }
        // Good food → no exposure.
        assert!(check_foodborne_exposure(10, OriginId::Human, &m, "fresh_meal", 0.9, 0.5, false).is_none());
    }

    #[test]
    fn tetanus_from_wound_plus_rust_raises_dirt() {
        let m = matrix();
        let (req, dirt) =
            check_wound_contact_tetanus(7, OriginId::Human, &m, true, true, false).expect("tetanus exposure");
        assert_eq!(req.disease, DiseaseKind::Tetanus);
        assert_eq!(req.vector, TransmissionVector::WoundContact);
        assert!((dirt - 0.3).abs() < 1e-6);
        // No rust → no tetanus.
        assert!(check_wound_contact_tetanus(7, OriginId::Human, &m, true, false, false).is_none());
    }

    #[test]
    fn sepsis_from_dirty_old_wound() {
        let m = matrix();
        let got = check_wound_sepsis(7, OriginId::Human, &m, 0.7, 30.0 * 3600.0, false).expect("sepsis");
        assert_eq!(got.disease, DiseaseKind::Sepsis);
        assert_eq!(got.vector, TransmissionVector::WoundInfection);
        // Clean wound → no sepsis.
        assert!(check_wound_sepsis(7, OriginId::Human, &m, 0.2, 30.0 * 3600.0, false).is_none());
        // Fresh wound → no sepsis.
        assert!(check_wound_sepsis(7, OriginId::Human, &m, 0.7, 1.0 * 3600.0, false).is_none());
    }

    #[test]
    fn airborne_room_spread_is_deterministic() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::InfluenzaPandemic);
        let m = matrix();
        let pop: Vec<ActorEpi> = (0..20u64)
            .map(|id| ActorEpi {
                actor_id: id,
                origin: OriginId::Human,
                infectious_with: id < 3,
                immune: false,
                infected: id < 3,
            })
            .collect();
        let mut a = Vec::new();
        let mut b = Vec::new();
        for tick in 0..600u64 {
            a.extend(tick_room_contact_spread(DiseaseKind::InfluenzaPandemic, spec, &m, &pop, tick, 99));
            b.extend(tick_room_contact_spread(DiseaseKind::InfluenzaPandemic, spec, &m, &pop, tick, 99));
        }
        assert_eq!(a, b, "identical seed must reproduce identical exposures");
        assert!(!a.is_empty(), "high-R0 pandemic must spread in a shared room");
    }

    #[test]
    fn radiation_exposure_carries_dose_magnitude() {
        let m = matrix();
        // Dose at threshold → magnitude 1.0.
        let at = check_radiation_exposure(7, OriginId::Human, &m, 100.0, 100.0, false).unwrap();
        assert!((at.magnitude - 1.0).abs() < 1e-6);
        // 3x threshold → magnitude 3.0.
        let high = check_radiation_exposure(7, OriginId::Human, &m, 300.0, 100.0, false).unwrap();
        assert!((high.magnitude - 3.0).abs() < 1e-6);
        // Cap at 5x.
        let capped = check_radiation_exposure(7, OriginId::Human, &m, 9000.0, 100.0, false).unwrap();
        assert!((capped.magnitude - 5.0).abs() < 1e-6);
    }

    #[test]
    fn contact_spread_uses_vector_table_r0() {
        // Plague's pneumonic (airborne) route is weaker than its primary
        // critter route, so contact spread uses the airborne relative_r0.
        let reg = DiseaseRegistry::default_registry();
        let plague = reg.lookup(DiseaseKind::BubonicPlague);
        let p = contact_exposure_probability(plague, 3, 1.0);
        // Recompute with the full base r0 (no table weighting) for comparison.
        let full = (plague.r0_per_exposure_event * 1.0 * 3.0 * CONTACT_TRANSMISSION_COEFF).clamp(0.0, 0.95);
        assert!(p < full, "plague contact spread must use the weaker pneumonic r0");
        assert!(p > 0.0);
    }

    #[test]
    fn non_contagious_disease_does_not_spread() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Tetanus);
        let m = matrix();
        let pop = vec![
            ActorEpi { actor_id: 0, origin: OriginId::Human, infectious_with: true, immune: false, infected: true },
            ActorEpi { actor_id: 1, origin: OriginId::Human, infectious_with: false, immune: false, infected: false },
        ];
        let out = tick_room_contact_spread(DiseaseKind::Tetanus, spec, &m, &pop, 5, 1);
        assert!(out.is_empty(), "tetanus is not human-to-human");
    }
}
