//! M16C § Per-actor mental-health dashboard (HUD model).
//!
//! Builds a render-ready [`PsychDashboardModel`] from an actor's
//! `cf_mental_health::ActorMentalHealth` state: one row per active condition
//! with its lifecycle stage, a 0..1 treatment-progress fraction (therapy
//! sessions blended with medication onset), and the comorbid partner. The
//! bevy render layer consumes the model; the model itself is pure + testable.

use cf_mental_health::{
    ActorCondition, ActorMentalHealth, ConditionKind, ConditionRegistry, ConditionStage,
};

/// Human-readable condition label for the dashboard.
pub fn condition_display_name(kind: ConditionKind) -> &'static str {
    match kind {
        ConditionKind::Ptsd => "PTSD",
        ConditionKind::AnxietyDisorder => "Anxiety Disorder",
        ConditionKind::Depression => "Depression",
        ConditionKind::Addiction => "Addiction",
        ConditionKind::Withdrawal => "Withdrawal",
        ConditionKind::Insomnia => "Insomnia",
        ConditionKind::PanicDisorder => "Panic Disorder",
        ConditionKind::AcuteStressReaction => "Acute Stress Reaction",
    }
}

/// Human-readable lifecycle-stage label for the dashboard.
pub fn stage_display_name(stage: ConditionStage) -> &'static str {
    match stage {
        ConditionStage::Triggered => "Triggered",
        ConditionStage::Acute => "Acute",
        ConditionStage::Subacute => "Subacute",
        ConditionStage::Chronic => "Chronic",
        ConditionStage::Remission => "In Remission",
        ConditionStage::Refractory => "Refractory",
    }
}

/// Treatment-progress fraction [0,1] for one condition: therapy sessions
/// blended with medication onset. Terminal stages (remission / refractory)
/// read as a full bar — the course is complete.
pub fn treatment_progress(
    condition: &ActorCondition,
    registry: &ConditionRegistry,
    tick: u64,
    tick_rate_hz: u32,
) -> f32 {
    if condition.stage.is_terminal() {
        return 1.0;
    }
    let Some(spec) = registry.get(condition.kind) else {
        return 0.0;
    };
    let plan = &condition.treatment;
    let therapy_frac = if spec.therapy_sessions_required == 0 {
        1.0
    } else {
        (plan.therapy_sessions_completed as f32 / spec.therapy_sessions_required as f32).min(1.0)
    };
    let med_frac = match spec.medication {
        None => 1.0,
        Some(_) => {
            if plan.medication_started() && spec.medication_onset_seconds > 0.0 {
                (plan.medication_active_for(tick, tick_rate_hz) / spec.medication_onset_seconds).min(1.0)
            } else if plan.medication_started() {
                1.0
            } else {
                0.0
            }
        }
    };
    ((therapy_frac + med_frac) * 0.5).clamp(0.0, 1.0)
}

/// One dashboard row for an active condition.
#[derive(Debug, Clone, PartialEq)]
pub struct PsychConditionRow {
    pub condition: ConditionKind,
    pub stage: ConditionStage,
    /// e.g. `"PTSD — Acute"`.
    pub label: String,
    /// Treatment-progress bar fraction [0,1].
    pub treatment_progress: f32,
    /// Whether the actor is currently symptomatic for this condition.
    pub symptomatic: bool,
    /// The comorbid partner condition, when this arose as a comorbidity.
    pub comorbid_with: Option<ConditionKind>,
}

/// The per-actor mental-health dashboard model.
#[derive(Debug, Clone, PartialEq)]
pub struct PsychDashboardModel {
    pub actor_id: u64,
    pub rows: Vec<PsychConditionRow>,
    pub any_symptomatic: bool,
}

impl PsychDashboardModel {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Build the dashboard model for one actor's mental-health state. Rows are
/// ordered by the condition's stable enum order (deterministic).
pub fn build_dashboard(
    actor_id: u64,
    mh: &ActorMentalHealth,
    registry: &ConditionRegistry,
    tick: u64,
    tick_rate_hz: u32,
) -> PsychDashboardModel {
    let mut rows: Vec<PsychConditionRow> = mh
        .active
        .iter()
        .map(|c| PsychConditionRow {
            condition: c.kind,
            stage: c.stage,
            label: format!("{} — {}", condition_display_name(c.kind), stage_display_name(c.stage)),
            treatment_progress: treatment_progress(c, registry, tick, tick_rate_hz),
            symptomatic: c.stage.is_symptomatic(),
            comorbid_with: c.comorbid_with,
        })
        .collect();
    rows.sort_by_key(|r| r.condition);
    let any_symptomatic = rows.iter().any(|r| r.symptomatic);
    PsychDashboardModel {
        actor_id,
        rows,
        any_symptomatic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_mental_health::{OriginId, PsychMedClass, TriggerReason};

    #[test]
    fn empty_state_is_empty_dashboard() {
        let mh = ActorMentalHealth::with_origin(OriginId::Human);
        let reg = ConditionRegistry::default_registry();
        let model = build_dashboard(7, &mh, &reg, 0, 60);
        assert!(model.is_empty());
        assert!(!model.any_symptomatic);
    }

    #[test]
    fn ptsd_row_labels_and_progress() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        // Untreated → progress 0 (no therapy, no med started).
        let model = build_dashboard(7, &mh, &reg, 10, 60);
        let row = &model.rows[0];
        assert_eq!(row.label, "PTSD — Acute");
        assert!(row.symptomatic);
        assert!((row.treatment_progress - 0.0).abs() < 1e-6);
        assert!(model.any_symptomatic);
    }

    #[test]
    fn progress_blends_therapy_and_medication() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        // 10/10 therapy sessions → therapy_frac 1.0.
        for _ in 0..10 {
            mh.record_therapy_session(7, ConditionKind::Ptsd, &reg, 0, 0);
        }
        // Med started but 0 onset elapsed → med_frac 0.0 → progress 0.5.
        mh.start_medication(7, ConditionKind::Ptsd, PsychMedClass::Ssri, 0);
        let model = build_dashboard(7, &mh, &reg, 0, 60);
        assert!((model.rows[0].treatment_progress - 0.5).abs() < 1e-6);
    }

    #[test]
    fn remission_reads_as_full_bar() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(7, ConditionKind::Insomnia, TriggerReason::SleepDeprivation, 0);
        mh.find_mut(ConditionKind::Insomnia).unwrap().stage = ConditionStage::Remission;
        let model = build_dashboard(7, &mh, &reg, 0, 60);
        assert!((model.rows[0].treatment_progress - 1.0).abs() < 1e-6);
        // Remission is not symptomatic.
        assert!(!model.rows[0].symptomatic);
    }
}
