//! **M14H** § Triage UX — Patient Queue + Patient Detail panels.
//!
//! M11 extension: a new HUD widget that surfaces the per-patient
//! compound_TTD, top wound label, top affliction label, and triage
//! status. Sortable by `compound_TTD` ascending (Gherkin scenario 4).
//! Clicking a row opens the Patient Detail panel surfacing the full
//! wound list + per-wound Treat button.

use bevy::prelude::*;

pub use cf_treatment::{
    PatientDetail, PatientDetailWound, PatientQueue, PatientRow, PatientStatus,
};

/// Bevy resource projection of the M14H Patient Queue panel.
///
/// cf-app's bridge writes this per frame from the engine's
/// `state.triage_queue` or per-tick simulation snapshot. The widget
/// reads it to render the Patient Queue HUD widget.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TriagePanelState {
    pub queue: PatientQueue,
    /// True if the player has the panel currently open. Toggled via
    /// `act.player.triage_select` with a non-empty target (or via the
    /// HUD widget's open affordance).
    pub open: bool,
}

impl TriagePanelState {
    /// **M14H** spec § "Click patient → opens Patient Detail (full wound
    /// list + per-wound Treat button)". Returns the per-wound Treat
    /// button label list for the currently-selected patient (empty when
    /// none selected).
    #[must_use]
    pub fn patient_detail_treat_buttons(&self) -> Vec<String> {
        let Some(d) = self.queue.detail.as_ref() else {
            return Vec::new();
        };
        d.wounds
            .iter()
            .map(PatientDetail::treat_button_label)
            .collect()
    }
}

impl TriagePanelState {
    /// Headline string per spec — "TRIAGE: N patients (M critical)".
    #[must_use]
    pub fn headline(&self) -> String {
        if !self.open || self.queue.is_empty() {
            return String::new();
        }
        let critical = self
            .queue
            .rows
            .iter()
            .filter(|r| matches!(r.status, PatientStatus::Critical | PatientStatus::Arresting))
            .count();
        format!(
            "TRIAGE: {} patients ({} critical)",
            self.queue.rows.len(),
            critical
        )
    }

    /// Render-ready row labels for the Patient Queue.
    #[must_use]
    pub fn row_labels(&self) -> Vec<String> {
        self.queue
            .rows
            .iter()
            .map(|r| {
                format!(
                    "[{}] {} TTD={:.0}s {} / {}",
                    r.status.as_str(),
                    r.name,
                    r.compound_ttd_seconds,
                    r.top_wound_label,
                    r.top_affliction_label
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(actor_id: u64, ttd: f32) -> PatientRow {
        PatientRow {
            actor_id,
            name: format!("Actor{actor_id}"),
            compound_ttd_seconds: ttd,
            top_wound_label: "LacerationLight Light".to_string(),
            top_affliction_label: "bleed_2w".to_string(),
            status: PatientStatus::from_signals(ttd, false, false),
        }
    }

    #[test]
    fn no_rows_hidden() {
        let s = TriagePanelState::default();
        assert_eq!(s.headline(), "");
        assert!(s.row_labels().is_empty());
    }

    /// **M14H** Gherkin scenario 4: 4 squadmates listed sorted by TTD asc.
    #[test]
    fn four_squadmates_sorted_by_ttd() {
        let mut s = TriagePanelState::default();
        s.open = true;
        s.queue.upsert(row(1, 120.0));
        s.queue.upsert(row(2, 18.0));
        s.queue.upsert(row(3, 45.0));
        s.queue.upsert(row(4, 200.0));
        assert_eq!(s.queue.rows.len(), 4);
        let labels = s.row_labels();
        assert_eq!(labels.len(), 4);
        // First row should be the most urgent (TTD=18, actor_id=2).
        assert!(labels[0].contains("Actor2"));
        assert!(labels[0].contains("TTD=18s"));
        assert!(s.headline().contains("4 patients"));
    }

    #[test]
    fn arresting_patient_counts_as_critical() {
        let mut s = TriagePanelState::default();
        s.open = true;
        s.queue.upsert(PatientRow {
            actor_id: 7,
            name: "Arrest".to_string(),
            compound_ttd_seconds: 60.0,
            top_wound_label: "Concussion".into(),
            top_affliction_label: "cardiac_arrest".into(),
            status: PatientStatus::Arresting,
        });
        s.queue.upsert(row(8, 30.0));
        let h = s.headline();
        assert!(h.contains("2 patients"));
        assert!(h.contains("(2 critical)"));
    }

    /// **M14H** § "Patient Detail (full wound list + per-wound Treat
    /// button)" surfaces the per-wound treat button labels.
    #[test]
    fn patient_detail_panel_renders_treat_buttons() {
        use cf_treatment::TreatmentKind;
        let mut s = TriagePanelState::default();
        s.open = true;
        s.queue.upsert(row(7, 30.0));
        s.queue.select(7);
        s.queue.set_detail(PatientDetail {
            actor_id: 7,
            name: "Echo".to_string(),
            compound_ttd_seconds: 30.0,
            status: PatientStatus::Critical,
            wounds: vec![
                PatientDetailWound {
                    wound_id: 1,
                    kind_label: "LacerationModerate".to_string(),
                    zone: "torso_front".to_string(),
                    severity: 0.4,
                    bandaged: false,
                    sutured: false,
                    dirt_pct: 0.0,
                    bleed_ml_per_s: 2.0,
                    recommended_treatment: Some(TreatmentKind::FieldBandageV1),
                },
                PatientDetailWound {
                    wound_id: 2,
                    kind_label: "ShrapnelEmbedded".to_string(),
                    zone: "leg_left".to_string(),
                    severity: 0.5,
                    bandaged: false,
                    sutured: false,
                    dirt_pct: 0.0,
                    bleed_ml_per_s: 1.0,
                    recommended_treatment: Some(TreatmentKind::SurgeryKitV1),
                },
            ],
            pain_total: 0.5,
            active_buffs: vec![],
            cardiac_arrest: false,
        });
        let buttons = s.patient_detail_treat_buttons();
        assert_eq!(buttons.len(), 2);
        assert!(buttons[0].contains("field_bandage_v1"));
        assert!(buttons[1].contains("surgery_kit_v1"));
    }
}
