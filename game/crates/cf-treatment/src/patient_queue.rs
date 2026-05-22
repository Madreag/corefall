//! **M14H** § Triage UX — Patient Queue + Patient Detail panels.
//!
//! Patient Queue exposes one row per patient with:
//! - name
//! - compound_TTD (one integer)
//! - top wound label
//! - top affliction label
//! - status (triage band)
//!
//! Sortable by `compound_TTD` ascending (Gherkin scenario 4).
//!
//! Clicking a row opens the Patient Detail panel ([`PatientDetail`])
//! surfacing the full wound list + per-wound Treat button labels.

use serde::{Deserialize, Serialize};

use crate::producers::TreatmentKind;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatientStatus {
    /// No immediate risk; routine treatment.
    Stable,
    /// Compound TTD > 30s; treatable.
    Urgent,
    /// Compound TTD ≤ 30s; needs immediate attention.
    Critical,
    /// Cardiac arrest or other life-threatening event.
    Arresting,
    /// Already deceased.
    Deceased,
}

impl PatientStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PatientStatus::Stable => "stable",
            PatientStatus::Urgent => "urgent",
            PatientStatus::Critical => "critical",
            PatientStatus::Arresting => "arresting",
            PatientStatus::Deceased => "deceased",
        }
    }

    pub fn from_signals(ttd_s: f32, cardiac_arrest: bool, deceased: bool) -> Self {
        if deceased {
            PatientStatus::Deceased
        } else if cardiac_arrest {
            PatientStatus::Arresting
        } else if ttd_s <= 30.0 {
            PatientStatus::Critical
        } else if ttd_s <= 90.0 {
            PatientStatus::Urgent
        } else {
            PatientStatus::Stable
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatientRow {
    pub actor_id: u64,
    pub name: String,
    pub compound_ttd_seconds: f32,
    pub top_wound_label: String,
    pub top_affliction_label: String,
    pub status: PatientStatus,
}

///
/// Each entry surfaces enough state for the HUD to render a row with a
/// per-wound Treat button. The `recommended_treatment` is computed by
/// the field-medic decision tree's wound-priority resolver.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatientDetailWound {
    pub wound_id: u64,
    pub kind_label: String,
    pub zone: String,
    pub severity: f32,
    pub bandaged: bool,
    pub sutured: bool,
    pub dirt_pct: f32,
    pub bleed_ml_per_s: f32,
    /// Suggested treatment producer for this wound (from the field-medic
    /// decision tree). The HUD renders this as the per-wound Treat button.
    pub recommended_treatment: Option<TreatmentKind>,
}

/// Patient Queue row).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatientDetail {
    pub actor_id: u64,
    pub name: String,
    pub compound_ttd_seconds: f32,
    pub status: PatientStatus,
    pub wounds: Vec<PatientDetailWound>,
    pub pain_total: f32,
    pub active_buffs: Vec<String>,
    pub cardiac_arrest: bool,
}

impl PatientDetail {
    /// One-line label per wound for the per-wound Treat button.
    pub fn treat_button_label(wound: &PatientDetailWound) -> String {
        match wound.recommended_treatment {
            Some(kind) => format!("Treat: {}", kind.as_str()),
            None => "Treat: (no recommendation)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PatientQueue {
    pub rows: Vec<PatientRow>,
    /// Currently-selected actor id (None = no patient detail open).
    pub selected: Option<u64>,
    /// Patient Detail data for the selected actor (None when no row is
    /// selected). Updated on every `select()` call.
    pub detail: Option<PatientDetail>,
}

impl PatientQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, row: PatientRow) {
        if let Some(existing) = self.rows.iter_mut().find(|r| r.actor_id == row.actor_id) {
            *existing = row;
        } else {
            self.rows.push(row);
        }
        self.sort_by_compound_ttd();
    }

    pub fn remove(&mut self, actor_id: u64) {
        self.rows.retain(|r| r.actor_id != actor_id);
        if self.selected == Some(actor_id) {
            self.selected = None;
        }
    }

    /// **Gherkin scenario 4**: sortable by TTD ascending.
    pub fn sort_by_compound_ttd(&mut self) {
        self.rows.sort_by(|a, b| {
            a.compound_ttd_seconds
                .partial_cmp(&b.compound_ttd_seconds)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.actor_id.cmp(&b.actor_id))
        });
    }

    pub fn select(&mut self, actor_id: u64) {
        if self.rows.iter().any(|r| r.actor_id == actor_id) {
            self.selected = Some(actor_id);
        }
    }

    /// Install a [`PatientDetail`] snapshot for the currently-selected
    /// patient. cf-app's bridge calls this every frame from the engine's
    /// per-actor wound list + cardiac component.
    pub fn set_detail(&mut self, detail: PatientDetail) {
        if self.selected == Some(detail.actor_id) {
            self.detail = Some(detail);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
        self.detail = None;
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(actor_id: u64, name: &str, ttd: f32) -> PatientRow {
        PatientRow {
            actor_id,
            name: name.to_string(),
            compound_ttd_seconds: ttd,
            top_wound_label: "LacerationLight Light".to_string(),
            top_affliction_label: "bleed_2w".to_string(),
            status: PatientStatus::from_signals(ttd, false, false),
        }
    }

    /// Given 4 squadmates in various states of injury, when the player
    /// opens the Patient Queue, then the panel lists 4 rows sorted by
    /// compound_TTD ascending.
    #[test]
    fn patient_queue_sorted_by_compound_ttd_ascending() {
        let mut q = PatientQueue::new();
        q.upsert(row(1, "Alpha", 120.0));
        q.upsert(row(2, "Bravo", 18.0));
        q.upsert(row(3, "Charlie", 45.0));
        q.upsert(row(4, "Delta", 200.0));
        assert_eq!(q.len(), 4);
        let order: Vec<u64> = q.rows.iter().map(|r| r.actor_id).collect();
        assert_eq!(order, vec![2, 3, 1, 4]);
    }

    /// Status mapping mirrors compound TTD bands.
    #[test]
    fn patient_status_band_thresholds() {
        assert_eq!(
            PatientStatus::from_signals(5.0, false, false),
            PatientStatus::Critical
        );
        assert_eq!(
            PatientStatus::from_signals(60.0, false, false),
            PatientStatus::Urgent
        );
        assert_eq!(
            PatientStatus::from_signals(200.0, false, false),
            PatientStatus::Stable
        );
        assert_eq!(
            PatientStatus::from_signals(100.0, true, false),
            PatientStatus::Arresting
        );
        assert_eq!(
            PatientStatus::from_signals(0.0, false, true),
            PatientStatus::Deceased
        );
    }

    /// Selection + removal interact correctly.
    #[test]
    fn select_and_remove() {
        let mut q = PatientQueue::new();
        q.upsert(row(7, "Echo", 30.0));
        q.select(7);
        assert_eq!(q.selected, Some(7));
        q.remove(7);
        assert!(q.selected.is_none());
    }

    /// list + per-wound Treat button)" — PatientDetail surfaces all
    /// wound rows with per-wound treat button labels.
    #[test]
    fn patient_detail_per_wound_treat_button_labels() {
        let mut q = PatientQueue::new();
        q.upsert(row(7, "Echo", 30.0));
        q.select(7);
        let detail = PatientDetail {
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
                    dirt_pct: 0.1,
                    bleed_ml_per_s: 1.0,
                    recommended_treatment: Some(TreatmentKind::SurgeryKitV1),
                },
            ],
            pain_total: 0.45,
            active_buffs: vec![],
            cardiac_arrest: false,
        };
        q.set_detail(detail);
        let d = q.detail.as_ref().expect("detail set");
        assert_eq!(d.wounds.len(), 2);
        let label0 = PatientDetail::treat_button_label(&d.wounds[0]);
        let label1 = PatientDetail::treat_button_label(&d.wounds[1]);
        assert!(label0.contains("field_bandage_v1"));
        assert!(label1.contains("surgery_kit_v1"));
    }

    #[test]
    fn detail_cleared_on_clear_selection() {
        let mut q = PatientQueue::new();
        q.upsert(row(7, "Echo", 30.0));
        q.select(7);
        q.set_detail(PatientDetail {
            actor_id: 7,
            name: "Echo".to_string(),
            compound_ttd_seconds: 30.0,
            status: PatientStatus::Critical,
            wounds: vec![],
            pain_total: 0.0,
            active_buffs: vec![],
            cardiac_arrest: false,
        });
        assert!(q.detail.is_some());
        q.clear_selection();
        assert!(q.detail.is_none());
    }
}
