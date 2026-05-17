//! M10B cause-chain walker (minimal, self-contained).
//!
//! Spec § "Notes for the implementer" + § "Player-facing behavior":
//!
//! > **Cause-chain side-panel during export.** Optional
//! > `--overlay cause_chain` renders the M10 cause-chain walker output
//! > as a per-event sidebar (auto-scrolling) so the viewer can read
//! > "input → fire → projectile → wound → death" as the moment plays.
//!
//! Preconditions to this feature (per the orchestrator's feature
//! definition): "M10 cause-chain walker is available + exposes a query
//! API for a given death event ID." The M10 walker proper lives in
//! `cf-tools-replay-viewer::cause_chain`. We do NOT depend on that
//! crate here (it would invert the dep graph; m10b-4 plans to wire
//! cf-tools-replay-viewer → cf-replay-export, not the other way
//! around). Instead this module re-implements the same `parent_event_id`
//! walk against the `cf-replay::Event` envelope. The walk semantics are
//! identical to the M10 walker; the renderer (m10b-3 `overlay_cause_chain`)
//! routes the output through `cf-localization` for plain-language +
//! locale-switched display.

use std::collections::HashSet;

use cf_replay::Event;

/// Default max chain depth — mirrors the M10 walker's
/// `cf_tools_replay_viewer::cause_chain::DEFAULT_MAX_DEPTH = 50`
/// constant so the two implementations stay aligned.
pub const DEFAULT_MAX_DEPTH: usize = 50;

/// One link in the chain.
#[derive(Debug, Clone)]
pub struct ChainLink<'a> {
    pub depth: usize,
    pub event: &'a Event,
}

/// Result of walking the chain. Mirrors the M10 walker's shape so the
/// renderer (`overlay_cause_chain`) can be ported back to consume the
/// M10 walker output when the dep graph permits (post-m10b-4).
#[derive(Debug, Clone)]
pub struct CauseChain<'a> {
    pub trigger: &'a Event,
    pub links: Vec<ChainLink<'a>>,
    pub termination: ChainTermination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTermination {
    RootReached,
    ParentMissingFromBundle,
    MaxDepthReached,
    CycleDetected,
}

/// Walk the parent chain of `trigger` backwards against an in-memory
/// `events` slice. Returns the chain plus the termination reason.
pub fn trace<'a>(events: &'a [Event], trigger: &'a Event, max_depth: usize) -> CauseChain<'a> {
    let by_id = index_by_event_id(events);
    let mut links: Vec<ChainLink<'a>> = vec![ChainLink {
        depth: 0,
        event: trigger,
    }];
    let mut visited: HashSet<&str> = HashSet::new();
    visited.insert(trigger.event_id.as_str());
    let mut current = trigger;
    let mut termination = ChainTermination::RootReached;

    while let Some(parent_id) = current.parent_event_id.as_deref() {
        if links.len() >= max_depth {
            termination = ChainTermination::MaxDepthReached;
            break;
        }
        if visited.contains(parent_id) {
            termination = ChainTermination::CycleDetected;
            break;
        }
        match by_id.get(parent_id) {
            Some(parent) => {
                visited.insert(parent.event_id.as_str());
                links.push(ChainLink {
                    depth: links.len(),
                    event: parent,
                });
                current = parent;
            }
            None => {
                termination = ChainTermination::ParentMissingFromBundle;
                break;
            }
        }
    }
    CauseChain {
        trigger,
        links,
        termination,
    }
}

/// Look up a specific event by id within `events`. Returns `None`
/// when not found.
pub fn find_event<'a>(events: &'a [Event], event_id: &str) -> Option<&'a Event> {
    events.iter().find(|e| e.event_id == event_id)
}

fn index_by_event_id(events: &[Event]) -> std::collections::HashMap<&str, &Event> {
    let mut map = std::collections::HashMap::with_capacity(events.len());
    for e in events {
        map.insert(e.event_id.as_str(), e);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_event(event_id: &str, event_type: &str, parent: Option<&str>, payload: serde_json::Value) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "cause_chain_walker_test".into(),
            tick: 0,
            sim_time_ms: 0.0,
            event_id: event_id.into(),
            category: "cause_chain_walker_test".into(),
            event_type: event_type.into(),
            payload,
            parent_event_id: parent.map(str::to_owned),
            actor_id: None,
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: None,
            cosmetic: None,
            asset_ref: None,
            prev_event_hash: None,
            chained_hash_hex: None,
        }
    }

    #[test]
    fn linear_chain_walks_to_root() {
        let events = vec![
            synth_event("a", "run_started", None, serde_json::json!({})),
            synth_event("b", "weapon_fired", Some("a"), serde_json::json!({})),
            synth_event("c", "projectile_hit", Some("b"), serde_json::json!({})),
            synth_event("d", "actor_died", Some("c"), serde_json::json!({})),
        ];
        let trigger = find_event(&events, "d").unwrap();
        let chain = trace(&events, trigger, DEFAULT_MAX_DEPTH);
        assert_eq!(chain.termination, ChainTermination::RootReached);
        assert_eq!(chain.links.len(), 4);
        assert_eq!(chain.links[0].event.event_type, "actor_died");
        assert_eq!(chain.links[3].event.event_type, "run_started");
    }

    #[test]
    fn missing_parent_terminates_early() {
        let events = vec![synth_event(
            "c",
            "actor_died",
            Some("missing"),
            serde_json::json!({}),
        )];
        let trigger = find_event(&events, "c").unwrap();
        let chain = trace(&events, trigger, DEFAULT_MAX_DEPTH);
        assert_eq!(chain.termination, ChainTermination::ParentMissingFromBundle);
        assert_eq!(chain.links.len(), 1);
    }

    #[test]
    fn max_depth_truncates_chain() {
        let events = vec![
            synth_event("a", "root", None, serde_json::json!({})),
            synth_event("b", "b", Some("a"), serde_json::json!({})),
            synth_event("c", "c", Some("b"), serde_json::json!({})),
            synth_event("d", "trigger", Some("c"), serde_json::json!({})),
        ];
        let chain = trace(&events, find_event(&events, "d").unwrap(), 2);
        assert_eq!(chain.termination, ChainTermination::MaxDepthReached);
        assert_eq!(chain.links.len(), 2);
    }
}
