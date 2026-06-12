//! M16B § Disease dashboard HUD widget.
//!
//! Per-actor disease state: lifecycle stage, severity, cure-progress bar,
//! and isolation/quarantine status — plus the base-wide pandemic lockdown
//! banner. The Medical Scanner feeds this panel (scenario: "the actor's
//! disease_dashboard surfaces full lifecycle stage + cure recipe").

use bevy::prelude::*;
use cf_disease::{
    lifecycle::{ActorDisease, ActorDiseases, DiseaseStage},
    registry::DiseaseSpec,
    IsolationClass, TreatmentKind,
};

/// One row in the disease dashboard.
#[derive(Debug, Clone, PartialEq)]
pub struct DiseaseDashboardEntry {
    pub actor_id: u64,
    pub disease_id: String,
    pub stage: DiseaseStage,
    pub severity: f32,
    /// Cure-course completion in [0,1] (0 when no treatment is underway).
    pub cure_progress: f32,
    pub treatment_kind: TreatmentKind,
    pub cure_item: Option<String>,
    pub doses_taken: u8,
    pub doses_required: u8,
    pub isolation_class: IsolationClass,
    pub quarantined: bool,
    pub resistant_strain: bool,
}

impl DiseaseDashboardEntry {
    /// Build a row from an active infection + its spec.
    pub fn from_disease(
        actor_id: u64,
        disease: &ActorDisease,
        spec: &DiseaseSpec,
        quarantined: bool,
    ) -> Self {
        let (cure_progress, doses_taken) = disease
            .treatment
            .as_ref()
            .map(|t| (t.fraction(), t.doses_taken))
            .unwrap_or((0.0, 0));
        Self {
            actor_id,
            disease_id: disease.kind.as_str().to_string(),
            stage: disease.stage,
            severity: disease.severity,
            cure_progress,
            treatment_kind: spec.cure.treatment_kind,
            cure_item: spec.cure.item_required.clone(),
            doses_taken,
            doses_required: spec.cure.dose_count,
            isolation_class: spec.isolation_class,
            quarantined,
            resistant_strain: disease.resistant_strain,
        }
    }

    /// Single-line accessibility string, e.g.
    /// `[pneumonia] manifest 60% — antibiotic 11/14 — isolation B`.
    pub fn formatted_line(&self) -> String {
        let cure = if self.doses_required > 0 {
            format!(
                " — {} {}/{}",
                self.treatment_kind.as_str(),
                self.doses_taken,
                self.doses_required
            )
        } else {
            format!(" — {}", self.treatment_kind.as_str())
        };
        let iso = match self.isolation_class {
            IsolationClass::NotApplicable => String::new(),
            other => format!(" — isolation {}", other.as_str()),
        };
        let quar = if self.quarantined { " [QUARANTINED]" } else { "" };
        format!(
            "[{}] {} {}%{}{}{}",
            self.disease_id,
            self.stage.as_str(),
            (self.severity * 100.0).round() as i32,
            cure,
            iso,
            quar
        )
    }
}

/// Resource projection of the disease dashboard.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct DiseaseDashboardState {
    pub entries: Vec<DiseaseDashboardEntry>,
    /// True while a base-wide pandemic lockdown is active.
    pub pandemic_active: bool,
    /// Lockdown banner text (e.g. "PANDEMIC: locking down quarters").
    pub pandemic_banner: Option<String>,
}

impl DiseaseDashboardState {
    /// Refresh the dashboard from one actor's disease state + the registry.
    /// Terminal (recovered/dead) infections are dropped from the view.
    pub fn refresh_actor(
        &mut self,
        actor_id: u64,
        state: &ActorDiseases,
        registry: &cf_disease::DiseaseRegistry,
    ) {
        self.entries.retain(|e| e.actor_id != actor_id);
        for disease in &state.active {
            if disease.stage.is_terminal() {
                continue;
            }
            if let Some(spec) = registry.get(disease.kind) {
                self.entries.push(DiseaseDashboardEntry::from_disease(
                    actor_id,
                    disease,
                    spec,
                    state.quarantined,
                ));
            }
        }
    }

    pub fn set_pandemic(&mut self, active: bool, banner: Option<String>) {
        self.pandemic_active = active;
        self.pandemic_banner = banner;
    }

    pub fn entries_for(&self, actor_id: u64) -> Vec<&DiseaseDashboardEntry> {
        self.entries.iter().filter(|e| e.actor_id == actor_id).collect()
    }

    #[must_use]
    pub fn formatted_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.entries.iter().map(|e| e.formatted_line()).collect();
        if self.pandemic_active {
            if let Some(banner) = &self.pandemic_banner {
                lines.insert(0, banner.clone());
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_disease::{lifecycle::expose, DiseaseKind, DiseaseRegistry, OriginId, TransmissionVector};

    #[test]
    fn dashboard_surfaces_stage_and_cure_recipe() {
        let reg = DiseaseRegistry::default_registry();
        let spec = reg.lookup(DiseaseKind::Pneumonia).clone();
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        {
            let d = state.find_mut(DiseaseKind::Pneumonia).unwrap();
            d.stage = DiseaseStage::Manifest;
            d.severity = 0.6;
        }
        cf_disease::lifecycle::start_treatment(&mut state, DiseaseKind::Pneumonia, &spec, 0);
        for _ in 0..11 {
            cf_disease::lifecycle::administer_dose(&mut state, DiseaseKind::Pneumonia, 1);
        }
        let mut dash = DiseaseDashboardState::default();
        dash.refresh_actor(7, &state, &reg);
        let entry = &dash.entries_for(7)[0];
        assert_eq!(entry.stage, DiseaseStage::Manifest);
        assert_eq!(entry.treatment_kind, TreatmentKind::Antibiotic);
        assert_eq!(entry.doses_taken, 11);
        assert_eq!(entry.doses_required, 14);
        assert_eq!(entry.isolation_class, IsolationClass::ClassB);
        let line = entry.formatted_line();
        assert!(line.contains("pneumonia"));
        assert!(line.contains("manifest"));
        assert!(line.contains("11/14"));
        assert!(line.contains("isolation B"));
    }

    #[test]
    fn terminal_infections_are_hidden() {
        let reg = DiseaseRegistry::default_registry();
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 7, DiseaseKind::Flu, TransmissionVector::Airborne, None, 0);
        state.find_mut(DiseaseKind::Flu).unwrap().stage = DiseaseStage::Recovered;
        let mut dash = DiseaseDashboardState::default();
        dash.refresh_actor(7, &state, &reg);
        assert!(dash.entries_for(7).is_empty());
    }

    #[test]
    fn pandemic_banner_leads_the_lines() {
        let mut dash = DiseaseDashboardState::default();
        dash.set_pandemic(true, Some("PANDEMIC: locking down quarters".to_string()));
        let lines = dash.formatted_lines();
        assert_eq!(lines[0], "PANDEMIC: locking down quarters");
    }
}
