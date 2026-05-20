//! **M14H** — Field Medic Workflow + Surgery + Defibrillator + Triage +
//! Treatment Producer Catalog.
//!
//! Canonical owner of:
//! - 22 [`TreatmentKind`] producers + per-producer [`TreatmentSpec`] catalog.
//! - [`TreatmentApply`] state machine (apply / progress / complete / fail).
//! - [`SurgerySession`] 5-phase surgery minigame FSM (Open / Diagnose /
//!   Operate / Close / Recover) with per-step deterministic skill checks.
//! - [`CardiacState`] cardiac-arrest + CPR/defib loop.
//! - [`FieldMedicDecisionTree`] Assess → Triage → Stabilize → Treat → Monitor.
//! - [`PatientQueue`] M11-extension panel rows sorted by compound TTD.
//! - [`MedicalScanner`] diagnostic scan output (wound + Pain + psych state).
//!
//! All RNG flows through a single seeded `rand_xoshiro::Xoshiro256StarStar`
//! so the M14H surgery + defib loops replay deterministically per the
//! M14H spec Gherkin scenario "Determinism — same seed reproduces surgery
//! outcome".

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::manual_range_contains,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::option_if_let_else,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_const_for_fn,
    clippy::wildcard_imports,
    clippy::manual_assert,
    clippy::redundant_closure_for_method_calls,
    clippy::enum_glob_use,
    clippy::uninlined_format_args,
    clippy::should_implement_trait,
    clippy::unnecessary_debug_formatting,
    clippy::single_match,
    clippy::unused_self,
    clippy::doc_lazy_continuation,
    clippy::explicit_iter_loop
)]

pub mod cardiac;
pub mod decision_tree;
pub mod effects;
pub mod patient_queue;
pub mod producers;
pub mod scanner;
pub mod state_machine;
pub mod surgery;
pub mod treatment_trait;

pub use cardiac::{
    CardiacEvent, CardiacOutcome, CardiacState, CardiacTrigger, CPR_ROUND_DURATION_SECONDS,
    CARDIAC_ARREST_GRACE_SECONDS, DEFIB_BASE_SUCCESS, DEFIB_CHARGES_DEFAULT,
    DEFIB_CPR_BOOST_PER_ROUND, DEFIB_RECHARGE_SECONDS, CPR_BRUISE_THRESHOLD_ROUNDS,
};
pub use decision_tree::{
    DecisionStep, FieldMedicDecisionTree, MedicAction, PatientSnapshot, WoundPriority,
};
pub use effects::{effect_for, TreatmentEffect};
pub use treatment_trait::{Treatment, TreatmentRegistry};
pub use patient_queue::{
    PatientDetail, PatientDetailWound, PatientQueue, PatientRow, PatientStatus,
};
pub use producers::{
    treatment_catalog, treatment_spec, RiskKind, ToolRequirement, TreatmentKind, TreatmentSpec,
    TreatmentSpecError, TreatmentSpecRegistry, SkillRequirement, BANDAGE_SOAK_THROUGH_SECONDS,
    TOURNIQUET_NECROSIS_THRESHOLD_SECONDS, SURGEON_T1_SKILL_PASS_RATE_X1000,
    MEDIC_T1_SKILL_PASS_RATE_X1000,
};
pub use scanner::{ScanReport, MedicalScanner, SCAN_DURATION_SECONDS_DEFAULT};
pub use state_machine::{
    bandage_soaked_through, TreatmentApply, TreatmentApplyError, TreatmentContext, TreatmentEvent,
    TreatmentFailureReason, TreatmentOutcome, TreatmentPhase,
};
pub use surgery::{
    SurgeryEvent, SurgeryFailureReason, SurgeryOutcome, SurgeryPhase, SurgerySession,
    SurgeryStepResult, SURGERY_PHASE_CLOSE_SECONDS, SURGERY_PHASE_DIAGNOSE_SECONDS,
    SURGERY_PHASE_OPEN_SECONDS, SURGERY_PHASE_OPERATE_SECONDS_PER_STEP,
    SURGERY_PHASE_RECOVER_SECONDS,
};
