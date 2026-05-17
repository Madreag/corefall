//! M9B integration tests for the cf-equipment tool surfaces.
//!
//! Lives under `tests/tools.rs` so it ships as the `--test tools`
//! integration binary referenced by the M9B validation contract evidence
//! string `cargo test -p cf-equipment --test tools entrenching_tool_registered`.

use cf_equipment::{
    find_entrenching_tool, m9b_entrenching_tools, EntrenchingToolSpec, ENTRENCHING_TOOL_ID,
};

/// VAL-M9B-EQUIP-001: the entrenching tool is a registered T0 equipment
/// item with cost `5 dirt + 1 wood` and dig-time `5s shallow_scrape /
/// 12s standard`. Test name encodes both `tools` and
/// `entrenching_tool_registered` so the feature-spec and
/// validation-contract filters both match.
#[test]
fn tools_entrenching_tool_registered() {
    let catalog = m9b_entrenching_tools();
    let entry: &EntrenchingToolSpec = catalog
        .iter()
        .find(|t| t.id == ENTRENCHING_TOOL_ID)
        .expect("entrenching_tool present in M9B catalog");

    assert_eq!(entry.tier, 0, "entrenching_tool is a T0 entry-level tool");
    assert_eq!(entry.id, "entrenching_tool");

    // Cost: 5 dirt + 1 wood.
    assert_eq!(entry.material_cost.len(), 2);
    assert_eq!(entry.material_cost.get("dirt"), Some(&5));
    assert_eq!(entry.material_cost.get("wood"), Some(&1));

    // Dig-time: 5s shallow_scrape / 12s standard.
    assert_eq!(entry.dig_time_for_variant("shallow_scrape"), Some(5));
    assert_eq!(entry.dig_time_for_variant("standard"), Some(12));

    // Find-by-id is the cfctl-shaped lookup; the m9b-3 cfctl handler
    // dispatches through `find_entrenching_tool` to surface the right spec.
    let looked_up = find_entrenching_tool(ENTRENCHING_TOOL_ID)
        .expect("find_entrenching_tool returns the registered spec");
    assert_eq!(looked_up.id, entry.id);
    assert_eq!(looked_up.dig_time_seconds, entry.dig_time_seconds);
}

#[test]
fn entrenching_tool_registered() {
    // Alias of the M9B test under the validation-contract name so the
    // contract's `--test tools entrenching_tool_registered` filter
    // resolves even when run without the longer `tools_` prefix.
    tools_entrenching_tool_registered();
}

/// Unknown id surfaces as `None` rather than panicking; m9b-3 cfctl uses
/// this signal to return a structured `tool_not_supported` error.
#[test]
fn tools_find_entrenching_tool_unknown_returns_none() {
    assert!(find_entrenching_tool("not_a_tool").is_none());
}
