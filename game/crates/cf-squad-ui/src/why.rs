//! "Why?" key — surfaces the bot's `reason_label_recent` ringbuffer (from
//! M7's `ai.reason_label_changed` feed) as a HUD popup.

use serde::{Deserialize, Serialize};

/// Read-only view of a bot's recent reason labels for the "Why?" key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyView {
    /// Bot whose reason labels are surfaced.
    pub actor_id: u64,
    /// Most recent reason label (for the popup headline).
    pub most_recent_label: Option<String>,
    /// Per-tick label history (oldest → newest); cap is whatever the
    /// engine snapshotted (M7-A defaults to 8 entries per spec § Layer 5
    /// reason ringbuffer).
    pub recent_labels: Vec<String>,
    /// Tick at which the most recent label was emitted.
    pub at_tick: Option<u64>,
}

impl WhyView {
    /// Build an empty view for an actor with no recorded labels.
    pub fn empty(actor_id: u64) -> Self {
        Self {
            actor_id,
            most_recent_label: None,
            recent_labels: Vec::new(),
            at_tick: None,
        }
    }

    /// Build a populated view from a label history (oldest → newest).
    pub fn from_history(actor_id: u64, labels: Vec<String>, at_tick: Option<u64>) -> Self {
        let most_recent_label = labels.last().cloned();
        Self {
            actor_id,
            most_recent_label,
            recent_labels: labels,
            at_tick,
        }
    }

    /// Player-facing one-line caption.
    pub fn caption(&self) -> String {
        match &self.most_recent_label {
            Some(label) => format!("WHY? {label}"),
            None => "WHY? (no recent reason)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_view_has_no_label() {
        let v = WhyView::empty(1);
        assert_eq!(v.actor_id, 1);
        assert!(v.most_recent_label.is_none());
        assert!(v.recent_labels.is_empty());
        assert!(v.at_tick.is_none());
    }

    #[test]
    fn from_history_captures_most_recent() {
        let v = WhyView::from_history(7, vec!["a".into(), "b".into(), "c".into()], Some(42));
        assert_eq!(v.most_recent_label, Some("c".into()));
        assert_eq!(v.recent_labels.len(), 3);
        assert_eq!(v.at_tick, Some(42));
    }

    #[test]
    fn caption_with_label() {
        let v = WhyView::from_history(7, vec!["triage_medic_route".into()], Some(99));
        assert_eq!(v.caption(), "WHY? triage_medic_route");
    }

    #[test]
    fn caption_without_label() {
        let v = WhyView::empty(7);
        assert_eq!(v.caption(), "WHY? (no recent reason)");
    }
}
