//! M16B — Class A/B/C quarantine room classification + quarantine entry.
//!
//! Reuses `cf_disease::IsolationClass` as the room grade. A room graded
//! Class A satisfies any lower isolation requirement (A ⊇ B ⊇ C).

use cf_disease::{
    lifecycle::DiseaseQuarantineEnteredEvent, DiseaseKind, IsolationClass,
};
use serde::{Deserialize, Serialize};

/// HEPA filter throughput required for a Class A (airborne) isolation room.
pub const CLASS_A_FILTER_THROUGHPUT: f32 = 0.9997;

/// Physical features of a room used to grade its isolation capability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoomFeatures {
    /// Hermetically sealed (no atmospheric leak).
    pub sealed: bool,
    /// Atmospheric analyzer present (M19E detection feed).
    pub has_atmospheric_analyzer: bool,
    /// Air filter present; `filter_throughput` is its efficiency [0,1].
    pub has_filter: bool,
    pub filter_throughput: f32,
    /// Airlock present (M28D damper controllable).
    pub has_airlock: bool,
    /// Surface sterilization (UV / chemical) for bodily-fluid containment.
    pub has_surface_sterilization: bool,
    /// Isolation cot (low-class containment).
    pub has_isolation_cot: bool,
    /// Dedicated mealware (prevents foodborne cross-contamination).
    pub has_dedicated_mealware: bool,
}

impl Default for RoomFeatures {
    fn default() -> Self {
        Self {
            sealed: false,
            has_atmospheric_analyzer: false,
            has_filter: false,
            filter_throughput: 0.0,
            has_airlock: false,
            has_surface_sterilization: false,
            has_isolation_cot: false,
            has_dedicated_mealware: false,
        }
    }
}

impl RoomFeatures {
    /// A fully-equipped Class A isolation room.
    pub fn class_a() -> Self {
        Self {
            sealed: true,
            has_atmospheric_analyzer: true,
            has_filter: true,
            filter_throughput: CLASS_A_FILTER_THROUGHPUT,
            has_airlock: true,
            has_surface_sterilization: true,
            has_isolation_cot: true,
            has_dedicated_mealware: true,
        }
    }

    pub fn class_b() -> Self {
        Self {
            has_surface_sterilization: true,
            has_isolation_cot: true,
            has_dedicated_mealware: true,
            ..Self::default()
        }
    }

    pub fn class_c() -> Self {
        Self {
            has_isolation_cot: true,
            has_dedicated_mealware: true,
            ..Self::default()
        }
    }
}

/// Grade a room by its features. Returns the highest isolation class the
/// room satisfies (`NotApplicable` if it isn't an isolation room at all).
pub fn classify_room(features: &RoomFeatures) -> IsolationClass {
    let class_a = features.sealed
        && features.has_atmospheric_analyzer
        && features.has_filter
        && features.has_airlock
        && features.filter_throughput >= CLASS_A_FILTER_THROUGHPUT;
    if class_a {
        return IsolationClass::ClassA;
    }
    let class_b = features.has_surface_sterilization && features.has_isolation_cot;
    if class_b {
        return IsolationClass::ClassB;
    }
    let class_c = features.has_isolation_cot && features.has_dedicated_mealware;
    if class_c {
        return IsolationClass::ClassC;
    }
    IsolationClass::NotApplicable
}

/// Outcome of an `enter_quarantine` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuarantineOutcome {
    pub event: DiseaseQuarantineEnteredEvent,
    /// True when the M28D airlock damper must close to seal the room
    /// (Class A airborne containment).
    pub close_airlock: bool,
}

/// Move an actor into quarantine for `disease`. Succeeds only when the room
/// grade satisfies the disease's isolation requirement. Returns the
/// `disease.quarantine_entered` event + whether the airlock must seal.
pub fn enter_quarantine(
    actor_id: u64,
    disease: DiseaseKind,
    required: IsolationClass,
    room_class: IsolationClass,
    tick: u64,
) -> Option<QuarantineOutcome> {
    if required == IsolationClass::NotApplicable {
        return None;
    }
    if !required.satisfied_by(room_class) {
        return None;
    }
    Some(QuarantineOutcome {
        event: DiseaseQuarantineEnteredEvent {
            actor_id,
            tick,
            pathogen: disease,
            room_class,
        },
        close_airlock: room_class == IsolationClass::ClassA,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_a_requires_full_sealed_filtered_room() {
        assert_eq!(classify_room(&RoomFeatures::class_a()), IsolationClass::ClassA);
        let mut leaky = RoomFeatures::class_a();
        leaky.sealed = false;
        assert_ne!(classify_room(&leaky), IsolationClass::ClassA);
        let mut weak_filter = RoomFeatures::class_a();
        weak_filter.filter_throughput = 0.5;
        assert_ne!(classify_room(&weak_filter), IsolationClass::ClassA);
    }

    #[test]
    fn class_b_and_c_grading() {
        assert_eq!(classify_room(&RoomFeatures::class_b()), IsolationClass::ClassB);
        assert_eq!(classify_room(&RoomFeatures::class_c()), IsolationClass::ClassC);
        assert_eq!(classify_room(&RoomFeatures::default()), IsolationClass::NotApplicable);
    }

    #[test]
    fn tb_quarantine_in_class_a_room_closes_airlock() {
        let outcome = enter_quarantine(
            7,
            DiseaseKind::Tuberculosis,
            IsolationClass::ClassA,
            IsolationClass::ClassA,
            500,
        )
        .expect("class A room satisfies class A requirement");
        assert_eq!(outcome.event.room_class, IsolationClass::ClassA);
        assert_eq!(outcome.event.pathogen, DiseaseKind::Tuberculosis);
        assert!(outcome.close_airlock, "class A quarantine seals the airlock (M28D damper)");
    }

    #[test]
    fn class_c_room_cannot_quarantine_class_a_disease() {
        let outcome = enter_quarantine(
            7,
            DiseaseKind::Tuberculosis,
            IsolationClass::ClassA,
            IsolationClass::ClassC,
            500,
        );
        assert!(outcome.is_none(), "class C room cannot contain a class A airborne disease");
    }

    #[test]
    fn class_a_room_can_quarantine_lower_class_disease() {
        let outcome = enter_quarantine(
            7,
            DiseaseKind::FoodPoisoning,
            IsolationClass::ClassC,
            IsolationClass::ClassA,
            10,
        )
        .expect("class A room subsumes class C requirement");
        assert_eq!(outcome.event.room_class, IsolationClass::ClassA);
    }

    #[test]
    fn non_isolation_disease_is_not_quarantined() {
        assert!(enter_quarantine(7, DiseaseKind::Rabies, IsolationClass::NotApplicable, IsolationClass::ClassA, 1).is_none());
    }
}
