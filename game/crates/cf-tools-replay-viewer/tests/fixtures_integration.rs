//! Integration tests against the committed fixture bundles under
//! `tests/fixtures/`. These differ from the unit tests in `src/*.rs`
//! because they use *committed-to-repo* JSON files instead of building
//! synthetic bundles in temp dirs. They prove that:
//!
//! - The viewer can load a hand-crafted fixture without running a sim.
//! - The strict bundle validator accepts a well-formed fixture.
//! - The cause-chain walks the documented shape (M3B-002 evidence for
//!   `actor_died` + `mission_resolved` against a synthetic fixture, since
//!   real BP2 bundles don't contain `actor_died`).

use std::path::PathBuf;

use cf_tools_replay_viewer::{
    bundle::Bundle,
    cause_chain::{render_markdown, trace, ChainTermination, DEFAULT_MAX_DEPTH},
    debrief,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn actor_died_fixture_loads_and_validates() {
    let bundle = Bundle::load(&fixture("m3b_actor_died_chain")).expect("fixture must load");
    assert_eq!(bundle.manifest.run_id, "m3b_fixture_actor_died_chain");
    assert_eq!(bundle.events.len(), 13);
    assert!(bundle.first_event_of_type("actor_died").is_some());
    assert!(bundle.first_event_of_type("mission_resolved").is_some());
}

#[test]
fn actor_died_cause_chain_walks_back_through_projectile_to_run_started() {
    let bundle = Bundle::load(&fixture("m3b_actor_died_chain")).expect("fixture must load");
    let trigger = bundle
        .first_event_of_type("actor_died")
        .expect("fixture has actor_died");
    let chain = trace(&bundle, trigger, DEFAULT_MAX_DEPTH);
    assert_eq!(chain.terminated_reason, ChainTermination::RootReached);
    let event_types: Vec<&str> = chain.links.iter().map(|l| l.event.event_type.as_str()).collect();
    assert_eq!(
        event_types,
        vec![
            "actor_died",
            "projectile_hit",
            "projectile_spawned",
            "weapon_fired",
            "command_accepted",
            "run_started",
        ]
    );
    let md = render_markdown(&chain);
    assert!(md.contains("actor_died"));
    assert!(md.contains("projectile_hit"));
    assert!(md.contains("weapon_fired"));
    assert!(md.contains("command_accepted"));
    assert!(md.contains("run_started"));
    assert!(md.contains("root reached"));
}

#[test]
fn mission_resolved_walks_back_through_actor_died_in_fixture() {
    // Real BP2 bundles emit mission_resolved without a parent (tick-driven
    // checks). The fixture intentionally chains mission_resolved back to
    // actor_died so M3B-D02 has an end-to-end cause chain for the
    // "death → mission outcome" path. This is the canonical shape future
    // engine code should produce when a mission ends because of a
    // specific event.
    let bundle = Bundle::load(&fixture("m3b_actor_died_chain")).expect("fixture must load");
    let trigger = bundle
        .first_event_of_type("mission_resolved")
        .expect("fixture has mission_resolved");
    let chain = trace(&bundle, trigger, DEFAULT_MAX_DEPTH);
    assert_eq!(chain.terminated_reason, ChainTermination::RootReached);
    let event_types: Vec<&str> = chain.links.iter().map(|l| l.event.event_type.as_str()).collect();
    assert_eq!(
        event_types,
        vec![
            "mission_resolved",
            "actor_died",
            "projectile_hit",
            "projectile_spawned",
            "weapon_fired",
            "command_accepted",
            "run_started",
        ]
    );
}

#[test]
fn debrief_reports_won_mission_and_one_actor_death() {
    let bundle = Bundle::load(&fixture("m3b_actor_died_chain")).expect("fixture must load");
    let d = debrief::compose(&bundle);
    assert_eq!(d.outcome.result.as_deref(), Some("won"));
    assert_eq!(d.outcome.reason.as_deref(), Some("all_red_actors_defeated"));
    assert_eq!(d.damage.actor_deaths, 1);
    assert_eq!(d.damage.projectile_hits, 1);
    assert_eq!(d.damage.total_projectile_damage as i64, 100);
    assert_eq!(
        d.checksum.final_checksum.as_deref(),
        Some("abcdef0123456789fedcba9876543210abcdef0123456789fedcba9876543210")
    );
    let md = debrief::render_markdown(&d);
    assert!(md.contains("Result: `won`"));
    assert!(md.contains("Actor deaths: 1"));
}
