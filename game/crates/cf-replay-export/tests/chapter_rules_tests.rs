//! M10B chapter rules round-trip tests (VAL-M10B-006 + cross-area M9B
//! + M9C coverage).
//!
//! Loads `game/content/replay_export/chapter_rules/default.ron` + asserts
//! every required event-type key is present.

use std::path::PathBuf;

use cf_replay_export::chapter_markers::{
    ChapterRuleSet, COMMANDER_EVENT_PREFIX, REQUIRED_M4_EVENT_KINDS, REQUIRED_M9B_EVENT_KINDS, REQUIRED_M9C_EVENT_KINDS,
};

fn default_chapter_rules_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate manifest dir must be game/crates/cf-replay-export/")
        .join("content/replay_export/chapter_rules/default.ron")
}

#[test]
fn chapter_rules_default_ron_exists_and_parses() {
    let path = default_chapter_rules_path();
    assert!(path.is_file(), "{} must exist", path.display());
    let rules = ChapterRuleSet::load(&path).expect("default.ron must parse");
    assert!(!rules.is_empty(), "default chapter rule set must have ≥1 rule");
}

#[test]
fn chapter_rules_covers_all_required_m4_event_kinds() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    for &kind in &REQUIRED_M4_EVENT_KINDS {
        assert!(
            rules.has_event_type(kind),
            "default chapter rules missing M4 event kind `{kind}`"
        );
    }
}

#[test]
fn chapter_rules_routes_actor_status_changed_killed_filter() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    assert!(
        rules.has_event_type_with_status("actor_status_changed", "killed"),
        "default chapter rules must include actor_status_changed with status_filter=killed"
    );
}

#[test]
fn chapter_rules_has_at_least_one_commander_beat() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    assert!(
        rules.has_commander_beat(),
        "default chapter rules must include ≥1 mission.commander_* rule"
    );
    // Sanity-check the prefix constant.
    assert_eq!(COMMANDER_EVENT_PREFIX, "mission.commander_");
}

#[test]
fn chapter_rules_covers_all_required_m9b_event_kinds() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    for &kind in &REQUIRED_M9B_EVENT_KINDS {
        assert!(
            rules.has_event_type(kind),
            "default chapter rules missing M9B trench event `{kind}`"
        );
    }
}

#[test]
fn chapter_rules_covers_all_required_m9c_event_kinds() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    for &kind in &REQUIRED_M9C_EVENT_KINDS {
        assert!(
            rules.has_event_type(kind),
            "default chapter rules missing M9C fortification event `{kind}`"
        );
    }
}

#[test]
fn chapter_rules_assert_required_keys_passes_for_default() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    rules
        .assert_required_keys()
        .expect("default.ron must satisfy required keys per VAL-M10B-006 + cross-area");
}

#[test]
fn chapter_rules_templates_are_plain_text_no_embedded_code() {
    let rules = ChapterRuleSet::load(&default_chapter_rules_path()).expect("parse");
    for rule in &rules.rules {
        assert!(
            !rule.template.is_empty(),
            "rule for {} must have non-empty template",
            rule.event_type
        );
        // Spec § Notes: "Chapter rules in `chapter_rules/default.ron`
        // are data, not code. ... the rule engine is just `(event_type,
        // chapter_template_string)` pairs." Reject obvious code-injection
        // looking glyphs that would imply an embedded evaluator.
        assert!(
            !rule.template.contains("println!"),
            "rule {} template must be plain text (no embedded code)",
            rule.event_type
        );
        assert!(
            !rule.template.contains("eval("),
            "rule {} template must be plain text (no embedded eval)",
            rule.event_type
        );
    }
}
