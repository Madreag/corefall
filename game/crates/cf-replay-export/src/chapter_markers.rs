//! M10B chapter rules.
//!
//! Loads + validates `game/content/replay_export/chapter_rules/default.ron`.
//!
//! The rule engine is "just `(event_type, chapter_template_string)`
//! pairs" per the spec's § Notes. Each [`ChapterRule`] declares the M4
//! `event_type` it consumes, the template string that becomes the MP4
//! chapter title, and an optional `status_filter` to disambiguate
//! `actor_status_changed` (only `status == "killed"` produces a chapter
//! marker, per the spec's Player-facing-behavior bullet).
//!
//! Per VAL-M10B-006 the rule set MUST contain entries for:
//!
//! - `mission.objective_started`, `mission.objective_completed`,
//!   `mission.objective_failed`
//! - `actor_status_changed` with status filter `killed`
//! - `reactor.armor_layer_destroyed`
//! - `atmos.breach_detected`
//! - at least one `mission.commander_*` beat
//!
//! Mission AGENTS.md additionally requires (cross-area):
//!
//! - M9B trench events: `trench_breastwork_breached`,
//!   `trench_segment_collapsed`, `trench_template_dropped`
//! - M9C fortification events: `mine_triggered`, `mg_nest_fired_burst`,
//!   `watchtower_destroyed`, `fence_shocked_actor`
//!
//! All thirteen required keys are enumerated by [`REQUIRED_M4_EVENT_KINDS`]
//! + [`REQUIRED_M9B_EVENT_KINDS`] + [`REQUIRED_M9C_EVENT_KINDS`] and
//! exercised by the `chapter_rules` round-trip test.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Workspace-relative path to the default chapter rules RON. Used by
/// loaders that resolve content paths off the crate manifest dir.
pub const DEFAULT_CHAPTER_RULES_RON: &str = "content/replay_export/chapter_rules/default.ron";

/// All `mission.commander_*` event-type names start with this prefix.
/// The `chapter_rules` test asserts at least one rule's `event_type`
/// begins with this string.
pub const COMMANDER_EVENT_PREFIX: &str = "mission.commander_";

/// M4 event-type keys the chapter rule set MUST cover per VAL-M10B-006.
/// `actor_status_changed` is paired with the `killed` status filter at
/// the rule layer; the prefix-match for `mission.commander_*` is
/// asserted separately via [`COMMANDER_EVENT_PREFIX`].
pub const REQUIRED_M4_EVENT_KINDS: [&str; 6] = [
    "mission.objective_started",
    "mission.objective_completed",
    "mission.objective_failed",
    "actor_status_changed",
    "reactor.armor_layer_destroyed",
    "atmos.breach_detected",
];

/// M9B trench event-type keys the chapter rule set MUST cover per the
/// cross-area validation contract.
pub const REQUIRED_M9B_EVENT_KINDS: [&str; 3] = [
    "trench_breastwork_breached",
    "trench_segment_collapsed",
    "trench_template_dropped",
];

/// M9C fortification event-type keys the chapter rule set MUST cover.
pub const REQUIRED_M9C_EVENT_KINDS: [&str; 4] = [
    "mine_triggered",
    "mg_nest_fired_burst",
    "watchtower_destroyed",
    "fence_shocked_actor",
];

/// One declarative `(event_type, template)` rule. `status_filter`
/// disambiguates the multi-status event taxonomy (e.g.
/// `actor_status_changed` fires for many transitions; only
/// `killed` produces a chapter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterRule {
    /// M4 event type the rule consumes (matches the event envelope's
    /// `event_type` field verbatim).
    pub event_type: String,
    /// Template string that becomes the MP4 chapter title. Curly-brace
    /// placeholders are interpolated against the event payload at
    /// chapter-derivation time (m10b-3). Plain-text placeholders are
    /// allowed; embedded code is NOT (the rule engine intentionally
    /// stays declarative per the spec § Notes).
    pub template: String,
    /// Optional `status` filter used by `actor_status_changed` and the
    /// `mission.objective_*` family when one event type carries multiple
    /// outcomes in its payload.
    #[serde(default)]
    pub status_filter: Option<String>,
    /// Optional category tag for downstream overlay grouping
    /// (chapter timeline + cause-chain sidebar). Free-form string.
    #[serde(default)]
    pub category: Option<String>,
}

/// Loaded chapter rule set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterRuleSet {
    pub rules: Vec<ChapterRule>,
}

impl ChapterRuleSet {
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<ChapterRuleSet>(text)
    }

    pub fn load(path: &Path) -> Result<Self, ChapterRulesError> {
        let text = fs::read_to_string(path).map_err(|err| ChapterRulesError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        Self::from_ron_str(&text).map_err(|err| ChapterRulesError::Parse {
            path: path.to_path_buf(),
            source: err,
        })
    }

    /// Total rule count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// `true` when the rule set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// `true` when at least one rule covers `event_type`.
    #[must_use]
    pub fn has_event_type(&self, event_type: &str) -> bool {
        self.rules.iter().any(|r| r.event_type == event_type)
    }

    /// `true` when at least one rule covers `event_type` AND its
    /// `status_filter` equals `status`.
    #[must_use]
    pub fn has_event_type_with_status(&self, event_type: &str, status: &str) -> bool {
        self.rules
            .iter()
            .any(|r| r.event_type == event_type && r.status_filter.as_deref() == Some(status))
    }

    /// `true` when at least one rule's `event_type` starts with
    /// `mission.commander_` (VAL-M10B-006 requires ≥1 commander beat).
    #[must_use]
    pub fn has_commander_beat(&self) -> bool {
        self.rules
            .iter()
            .any(|r| r.event_type.starts_with(COMMANDER_EVENT_PREFIX))
    }

    /// Assert every required event-type key per VAL-M10B-006 +
    /// cross-area M9B + M9C coverage is present. Returns the first
    /// missing key as a typed error so the chapter_rules test points
    /// at the offending key directly.
    pub fn assert_required_keys(&self) -> Result<(), ChapterRulesError> {
        for &kind in &REQUIRED_M4_EVENT_KINDS {
            if !self.has_event_type(kind) {
                return Err(ChapterRulesError::MissingEventKind {
                    event_type: kind.to_owned(),
                });
            }
        }
        if !self.has_event_type_with_status("actor_status_changed", "killed") {
            return Err(ChapterRulesError::MissingStatusFilter {
                event_type: "actor_status_changed".into(),
                status: "killed".into(),
            });
        }
        if !self.has_commander_beat() {
            return Err(ChapterRulesError::MissingCommanderBeat);
        }
        for &kind in &REQUIRED_M9B_EVENT_KINDS {
            if !self.has_event_type(kind) {
                return Err(ChapterRulesError::MissingEventKind {
                    event_type: kind.to_owned(),
                });
            }
        }
        for &kind in &REQUIRED_M9C_EVENT_KINDS {
            if !self.has_event_type(kind) {
                return Err(ChapterRulesError::MissingEventKind {
                    event_type: kind.to_owned(),
                });
            }
        }
        Ok(())
    }
}

/// Typed errors surfaced by the chapter-rules loader + validator.
#[derive(Debug, Error)]
pub enum ChapterRulesError {
    #[error("read chapter rules RON at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse chapter rules RON at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("chapter rule set missing required event kind `{event_type}`")]
    MissingEventKind { event_type: String },
    #[error("chapter rule set missing entry for `{event_type}` with status filter `{status}`")]
    MissingStatusFilter { event_type: String, status: String },
    #[error("chapter rule set missing at least one `mission.commander_*` beat")]
    MissingCommanderBeat,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rule(event_type: &str, template: &str) -> ChapterRule {
        ChapterRule {
            event_type: event_type.into(),
            template: template.into(),
            status_filter: None,
            category: None,
        }
    }

    #[test]
    fn round_trip_preserves_rule_fields() {
        let src = ChapterRuleSet {
            rules: vec![
                sample_rule("mission.objective_started", "Objective started: {name}"),
                ChapterRule {
                    event_type: "actor_status_changed".into(),
                    template: "{actor_name} killed".into(),
                    status_filter: Some("killed".into()),
                    category: Some("death".into()),
                },
            ],
        };
        let text = ron::ser::to_string(&src).expect("serialise");
        let parsed = ChapterRuleSet::from_ron_str(&text).expect("round-trip parse");
        assert_eq!(parsed, src);
    }

    #[test]
    fn assert_required_keys_flags_missing_objective_started() {
        let rules = ChapterRuleSet { rules: vec![] };
        let err = rules.assert_required_keys().expect_err("empty set should fail");
        assert!(matches!(
            err,
            ChapterRulesError::MissingEventKind { event_type } if event_type == "mission.objective_started"
        ));
    }

    #[test]
    fn assert_required_keys_flags_missing_status_filter() {
        let mut rules = ChapterRuleSet { rules: vec![] };
        for &k in &REQUIRED_M4_EVENT_KINDS {
            rules.rules.push(sample_rule(k, &format!("{k} chapter")));
        }
        rules.rules.push(sample_rule(
            "mission.commander_objective_assigned",
            "Commander: assigned",
        ));
        for &k in &REQUIRED_M9B_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        for &k in &REQUIRED_M9C_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        // Strip the killed-status entry: the bare actor_status_changed rule is present
        // but the killed filter is missing.
        let err = rules
            .assert_required_keys()
            .expect_err("missing killed-status filter should fail");
        assert!(matches!(
            err,
            ChapterRulesError::MissingStatusFilter { event_type, status }
                if event_type == "actor_status_changed" && status == "killed"
        ));
    }

    #[test]
    fn assert_required_keys_flags_missing_commander_beat() {
        let mut rules = ChapterRuleSet { rules: vec![] };
        for &k in &REQUIRED_M4_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        rules.rules.push(ChapterRule {
            event_type: "actor_status_changed".into(),
            template: "{actor_name} killed".into(),
            status_filter: Some("killed".into()),
            category: None,
        });
        for &k in &REQUIRED_M9B_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        for &k in &REQUIRED_M9C_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        let err = rules
            .assert_required_keys()
            .expect_err("missing commander beat should fail");
        assert!(matches!(err, ChapterRulesError::MissingCommanderBeat));
    }

    #[test]
    fn assert_required_keys_passes_when_all_kinds_present() {
        let mut rules = ChapterRuleSet { rules: vec![] };
        for &k in &REQUIRED_M4_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        rules.rules.push(ChapterRule {
            event_type: "actor_status_changed".into(),
            template: "{actor_name} killed".into(),
            status_filter: Some("killed".into()),
            category: None,
        });
        rules.rules.push(sample_rule(
            "mission.commander_objective_assigned",
            "Commander: assigned",
        ));
        for &k in &REQUIRED_M9B_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        for &k in &REQUIRED_M9C_EVENT_KINDS {
            rules.rules.push(sample_rule(k, k));
        }
        rules.assert_required_keys().expect("complete set should pass");
    }

    #[test]
    fn commander_prefix_starts_with_mission_dot_commander() {
        assert!(COMMANDER_EVENT_PREFIX.starts_with("mission.commander_"));
    }
}
