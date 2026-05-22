//! M7-A: per-bot reason-label ring buffer.
//!
//! Every meaningful AI tick emits a structured reason label describing what
//! the thinking stack chose + why. The label is deterministic (byte-identical
//! across re-runs from the same seed) so replay viewers can diff bot
//! decisions across runs.
//!
//! `ReasonLabelRing` holds the last N labels per bot; the engine emits
//! `ai.reason_label_changed` when the latest label differs from the previous
//! (so a bot ticking on the same plan for N ticks emits one event, not N).
//!
//! Spec § Reason label format mandates the field order; `ReasonLabel::format`
//! produces that exact format. Re-runs from the same seed must produce the
//! same string — every numeric component is finite + quantized.

use serde::{Deserialize, Serialize};

/// commandable AI mandates "reason_label_recent" be a ring; M8A locks the
/// depth.
pub const REASON_LABEL_RING_DEPTH: usize = 8;

///
/// Re-runs from the same seed must produce byte-identical `format()` output;
/// the producer guarantees this by quantizing floats + sorting candidate
/// arrays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasonLabel {
    /// Chosen task (e.g. "TriageDownedAlly").
    pub chosen_task: String,
    /// Optional target identifier (actor id, zone id, etc.). `None` ↔ no
    /// target on this tick.
    pub chosen_target: Option<String>,
    /// Final utility score for the chosen task. Quantized to 2 decimal
    /// places before formatting.
    pub score: f32,
    /// Decomposition of the score: base utility, priority multiplier,
    /// tag/order bonus, mood/stress multiplier. All quantized.
    pub score_base: f32,
    pub score_priority_multiplier: f32,
    pub score_tag_bonus: f32,
    /// Top-3 candidate (task, score) pairs, sorted descending by score then
    /// by task name (stable order).
    pub candidates: Vec<(String, f32)>,
    /// Layer 4 HTN goal stack (e.g. "protect_squad/triage_medic_route").
    pub htn_goal_stack: String,
    /// Layer 3 behavior-tree node trail (e.g.
    /// "move_to_cover→approach_ally→treat_loop").
    pub behavior_tree_node: String,
    /// Layer 5 doctrine prior (defaults to "defensive" when LLM mind is off).
    pub doctrine: String,
    /// Archetype name (e.g. "medic").
    pub role: String,
}

impl ReasonLabel {
    /// Build an empty/placeholder label (used by the engine before the first
    /// tick).
    pub fn idle(role: &str) -> Self {
        Self {
            chosen_task: "Idle".to_string(),
            chosen_target: None,
            score: 0.0,
            score_base: 0.0,
            score_priority_multiplier: 1.0,
            score_tag_bonus: 1.0,
            candidates: Vec::new(),
            htn_goal_stack: "idle".to_string(),
            behavior_tree_node: "idle".to_string(),
            doctrine: "defensive".to_string(),
            role: role.to_string(),
        }
    }

    /// Render this label as the canonical deterministic string. Every
    /// numeric field is rounded to 2 decimal places before printing. The
    /// format mirrors the spec § Reason label format example.
    pub fn format(&self) -> String {
        let target = match &self.chosen_target {
            Some(t) => format!("{}({})", self.chosen_task, t),
            None => self.chosen_task.clone(),
        };
        let candidates = self
            .candidates
            .iter()
            .map(|(name, s)| format!("{name}({:.2})", quantize2(*s)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "chosen={}; score={:.2} (base={:.2} × prio={:.2} × tag_bonus={:.2}); candidates=[{}]; htn={}; bt={}; doctrine={}; role={}",
            target,
            quantize2(self.score),
            quantize2(self.score_base),
            quantize2(self.score_priority_multiplier),
            quantize2(self.score_tag_bonus),
            candidates,
            self.htn_goal_stack,
            self.behavior_tree_node,
            self.doctrine,
            self.role
        )
    }

    /// Bytes for the determinism checksum (label-discriminator only — full
    /// strings would bloat the digest).
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let s = self.format();
        s.into_bytes()
    }
}

fn quantize2(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    (value * 100.0).round() / 100.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasonLabelRing {
    entries: Vec<ReasonLabel>,
    /// Index where the next push writes.
    head: usize,
    /// True once the ring has wrapped.
    wrapped: bool,
}

impl ReasonLabelRing {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(REASON_LABEL_RING_DEPTH),
            head: 0,
            wrapped: false,
        }
    }

    pub fn push(&mut self, label: ReasonLabel) -> bool {
        let changed = self
            .latest()
            .map(|prev| prev.format() != label.format())
            .unwrap_or(true);
        if self.entries.len() < REASON_LABEL_RING_DEPTH {
            self.entries.push(label);
            self.head = self.entries.len() % REASON_LABEL_RING_DEPTH;
        } else {
            self.entries[self.head] = label;
            self.head = (self.head + 1) % REASON_LABEL_RING_DEPTH;
            self.wrapped = true;
        }
        changed
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Most recent label (or None if empty).
    pub fn latest(&self) -> Option<&ReasonLabel> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = if self.wrapped {
            (self.head + REASON_LABEL_RING_DEPTH - 1) % REASON_LABEL_RING_DEPTH
        } else {
            self.entries.len() - 1
        };
        self.entries.get(idx)
    }

    /// Iterate oldest → newest (deterministic).
    pub fn iter_chronological(&self) -> impl Iterator<Item = &ReasonLabel> + '_ {
        let len = self.entries.len();
        let start = if self.wrapped { self.head } else { 0 };
        (0..len).map(move |i| &self.entries[(start + i) % len])
    }
}

impl Default for ReasonLabelRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_format_is_deterministic() {
        let mut a = ReasonLabel::idle("medic");
        a.chosen_task = "TriageDownedAlly".to_string();
        a.chosen_target = Some("Mendez".to_string());
        a.score = 0.92345;
        a.score_base = 0.46;
        a.score_priority_multiplier = 1.8;
        a.score_tag_bonus = 1.11;
        a.candidates = vec![("SuppressFire".to_string(), 0.61), ("Advance".to_string(), 0.45)];
        a.htn_goal_stack = "protect_squad/triage_medic_route".to_string();
        a.behavior_tree_node = "move_to_cover→approach_ally→treat_loop".to_string();
        let b = a.clone();
        assert_eq!(a.format(), b.format());
        assert!(a.format().contains("chosen=TriageDownedAlly(Mendez)"));
        assert!(a.format().contains("role=medic"));
    }

    #[test]
    fn ring_push_detects_label_change() {
        let mut r = ReasonLabelRing::new();
        let l1 = ReasonLabel::idle("rifleman");
        assert!(r.push(l1.clone()));
        assert!(!r.push(l1.clone()), "identical label must not be flagged changed");
        let mut l2 = l1.clone();
        l2.chosen_task = "Suppress".to_string();
        assert!(r.push(l2));
    }
}
