use std::path::Path;

use cf_control::Scenario;

use crate::report::ValidationReport;

pub(crate) fn validate_scenario(path: &Path, report: &mut ValidationReport) {
    match Scenario::load_from_file(path) {
        Ok(scenario) => {
            let mut messages: Vec<String> = Vec::new();
            if scenario.schema_version != 1 {
                messages.push(format!(
                    "scenario.schema_version must be 1 (got {})",
                    scenario.schema_version
                ));
            }
            if scenario.id.is_empty() {
                messages.push("scenario.id must be non-empty".to_string());
            }
            if scenario.display_name.trim().is_empty() {
                messages.push("scenario.display_name must be non-empty".to_string());
            }
            if scenario.expected_tests.is_empty() {
                messages.push("scenario.expected_tests must reference at least one acceptance test id".to_string());
            }
            // **M9** § cf-mod validate micro_reactor_defense — extra rules per
            // spec § "When `cargo run -p cf-mod -- validate content/scenarios/`
            // runs Then the validator confirms: 1 reactor with mission_critical=true
            // + hp>0 + AABB defined, 1 player spawn, 1 guard slot, timer in
            // [1800, 10800] ticks (30-180s @60Hz), objectives[] includes
            // defend_reactor".
            if scenario.id == "micro_reactor_defense" {
                if scenario.reactors.len() != 1 {
                    messages.push(format!(
                        "M9 micro_reactor_defense must declare exactly 1 reactor (got {})",
                        scenario.reactors.len()
                    ));
                }
                if let Some(r) = scenario.reactors.first() {
                    if r.hp <= 0.0 {
                        messages.push(format!("M9 reactor.hp must be > 0 (got {})", r.hp));
                    }
                    if r.half_extents.0 <= 0.0 || r.half_extents.1 <= 0.0 {
                        messages.push("M9 reactor.half_extents must define a positive AABB".to_string());
                    }
                }
                let controllable_count = scenario.actors.iter().filter(|a| a.controllable).count();
                if controllable_count != 1 {
                    messages.push(format!(
                        "M9 micro_reactor_defense must declare exactly 1 controllable player spawn (got {controllable_count})"
                    ));
                }
                let guard_count = scenario.actors.iter().filter(|a| a.enemy.is_some()).count();
                if guard_count < 1 {
                    messages
                        .push("M9 micro_reactor_defense must declare at least 1 reactive_guard enemy slot".to_string());
                }
                if let Some(mission) = scenario.mission.as_ref() {
                    if mission.time_limit_ticks < 1800 || mission.time_limit_ticks > 10800 {
                        messages.push(format!(
                            "M9 micro_reactor_defense mission.time_limit_ticks must be in [1800, 10800] (got {})",
                            mission.time_limit_ticks
                        ));
                    }
                } else {
                    messages.push("M9 micro_reactor_defense must declare a mission timer block".to_string());
                }
                let has_defend_reactor = scenario
                    .objectives
                    .iter()
                    .any(|o| matches!(&o.kind, cf_control::ScenarioObjectiveKind::DefendReactor { .. }));
                if !has_defend_reactor {
                    messages.push(
                        "M9 micro_reactor_defense must declare at least one defend_reactor objective".to_string(),
                    );
                }
            }
            if !messages.is_empty() {
                report.add_error(path.to_path_buf(), messages.join("; "));
            } else {
                report.add_pass(path.to_path_buf(), format!("scenario {}", scenario.id));
            }
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("scenario load failed: {err}"));
        }
    }
}
