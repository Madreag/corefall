//! M10B cause-chain sidebar overlay.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Cause-chain side-panel during export.** Optional
//! > `--overlay cause_chain` renders the M10 cause-chain walker output
//! > as a per-event sidebar (auto-scrolling) so the viewer can read
//! > "input → fire → projectile → wound → death" as the moment plays.
//!
//! VAL-M10B-030: "every line is plain-language (no `{}`
//! JSON-looking braces, no raw `event_type:` prefixes); rendering an
//! export with a non-en-US locale (`--locale es-ES`) produces sidebar
//! text in Spanish, proving the text is wrapped through
//! `cf-localization`."
//!
//! VAL-CROSS-011 + VAL-CROSS-012 (cross-area M9C):
//!
//! > MUST include cause-chain lines for M9C `fence_shocked_actor`
//! > (`electrified_fence` + `shock` substring) and `mine_triggered`
//! > (`mine_` + numeric yield substring).
//!
//! The renderer routes every line through `cf-localization` via the
//! `cause_chain.<event_type>` key. Missing keys fall through to a
//! deterministic `cause_chain.fallback` template so a partially
//! translated locale still produces a complete sidebar (no blank lines).

use cf_localization::LocalizationTable;
use cf_replay::Event;

use crate::cause_chain_walker::{trace, CauseChain, DEFAULT_MAX_DEPTH};
use crate::overlay_graph::{CAUSE_CHAIN_OVERLAY_NAME, CAUSE_CHAIN_Z_ORDER};

/// Sidebar AOI at 1920×1080 — the right edge of the frame, full
/// height. Other resolutions scale proportionally.
pub const CAUSE_CHAIN_AOI_X: u32 = 1920 - 16 - 360;
pub const CAUSE_CHAIN_AOI_Y: u32 = 280;
pub const CAUSE_CHAIN_AOI_WIDTH: u32 = 360;
pub const CAUSE_CHAIN_AOI_HEIGHT: u32 = 600;

/// Auto-scroll dwell — how long each chain link stays at the top of
/// the visible region before scrolling. At 60 fps + 90 ticks/link
/// dwell, a 6-link chain takes ~9 seconds to display.
pub const CAUSE_CHAIN_LINK_DWELL_TICKS: u64 = 90;

/// l10n key prefix used by every cause-chain template.
pub const CAUSE_CHAIN_LOCALIZATION_PREFIX: &str = "cause_chain.";

/// Fallback localization key, used when an event_type has no
/// dedicated template in the loaded localization bundle.
pub const CAUSE_CHAIN_FALLBACK_KEY: &str = "cause_chain.fallback";

/// One rendered cause-chain line. The renderer produces one
/// `RenderedLine` per `ChainLink`; the sidebar pixel layout stacks
/// them top-to-bottom in the AOI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLine {
    /// 0 = trigger event, n = nth parent. The renderer indents deeper
    /// links per the M10 cause-chain markdown convention.
    pub depth: usize,
    /// Plain-language text rendered through `cf-localization`. Never
    /// contains raw payload braces or `event_type:` prefixes per
    /// VAL-M10B-030.
    pub text: String,
    /// Source event id; the offline rasterizer uses this to align the
    /// line's first-visible frame with the event's source tick.
    pub source_event_id: String,
    /// Source event type (e.g. `fence_shocked_actor`,
    /// `mine_triggered`). Used by cross-area test assertions —
    /// VAL-CROSS-011 / VAL-CROSS-012 search for these literals in the
    /// rendered output.
    pub source_event_type: String,
}

/// The cause-chain overlay descriptor + render facade.
#[derive(Debug, Clone)]
pub struct CauseChainOverlay {
    pub aoi_x: u32,
    pub aoi_y: u32,
    pub aoi_width: u32,
    pub aoi_height: u32,
    pub z_order: u32,
    pub focus_event_id: String,
    pub locale: LocalizationTable,
}

impl CauseChainOverlay {
    #[must_use]
    pub const fn name() -> &'static str {
        CAUSE_CHAIN_OVERLAY_NAME
    }

    /// Construct an overlay with the default AOI for the focused
    /// event id + locale bundle.
    #[must_use]
    pub fn new(focus_event_id: String, locale: LocalizationTable) -> Self {
        Self {
            aoi_x: CAUSE_CHAIN_AOI_X,
            aoi_y: CAUSE_CHAIN_AOI_Y,
            aoi_width: CAUSE_CHAIN_AOI_WIDTH,
            aoi_height: CAUSE_CHAIN_AOI_HEIGHT,
            z_order: CAUSE_CHAIN_Z_ORDER,
            focus_event_id,
            locale,
        }
    }

    /// Walk the cause chain from `focus_event_id` against `events` and
    /// render each link through the loaded localization bundle.
    ///
    /// Returns `None` when `focus_event_id` is not found in the
    /// events slice (the export CLI surfaces a typed error in that
    /// case; m10b-4 handles the user-facing message).
    #[must_use]
    pub fn render<'a>(&self, events: &'a [Event]) -> Option<Vec<RenderedLine>> {
        let trigger = events.iter().find(|e| e.event_id == self.focus_event_id)?;
        let chain = trace(events, trigger, DEFAULT_MAX_DEPTH);
        Some(render_chain(&chain, &self.locale))
    }
}

/// Render every link of the chain through the locale bundle. Public
/// so unit tests + cross-area assertions can drive it directly
/// without constructing a full overlay struct.
#[must_use]
pub fn render_chain(chain: &CauseChain<'_>, locale: &LocalizationTable) -> Vec<RenderedLine> {
    chain
        .links
        .iter()
        .map(|link| RenderedLine {
            depth: link.depth,
            text: render_event_plain(link.event, locale),
            source_event_id: link.event.event_id.clone(),
            source_event_type: link.event.event_type.clone(),
        })
        .collect()
}

/// Render a single event as plain-language text via the locale bundle.
///
/// Lookup order:
/// 1. `cause_chain.<event_type>.<status>` — for multi-status events
///    like `actor_status_changed` where the payload carries a `status`
///    field. Lets the renderer surface a different sentence for
///    `killed` vs `wounded` vs `reviving`.
/// 2. `cause_chain.<event_type>` — the bare event-type key.
/// 3. `cause_chain.fallback` — generic catch-all that interpolates
///    the event_type + tick.
///
/// All keys are looked up against the loaded `cf-localization` bundle;
/// the locale-switch test (VAL-M10B-030) exercises this lookup path.
#[must_use]
pub fn render_event_plain(event: &Event, locale: &LocalizationTable) -> String {
    let event_type_str = event.event_type.clone();
    let tick_str = event.tick.to_string();
    let pairs = payload_args(event);
    let mut base_args: Vec<(String, String)> = pairs.clone();
    base_args.push(("tick".into(), tick_str.clone()));
    if let Some(team) = event.team.as_deref() {
        base_args.push(("team".into(), team.to_owned()));
    }

    let mut args: Vec<(&str, &str)> = base_args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    let status = pairs.iter().find(|(k, _)| k == "status").map(|(_, v)| v.clone());
    if let Some(status) = status {
        let key = format!("{CAUSE_CHAIN_LOCALIZATION_PREFIX}{event_type_str}.{status}");
        if locale.lookup(&key).is_some() {
            return locale.format(&key, &args);
        }
    }
    let key = format!("{CAUSE_CHAIN_LOCALIZATION_PREFIX}{event_type_str}");
    if locale.lookup(&key).is_some() {
        return locale.format(&key, &args);
    }
    args.push(("event_type", event_type_str.as_str()));
    locale.format(CAUSE_CHAIN_FALLBACK_KEY, &args)
}

fn payload_args(event: &Event) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(obj) = event.payload.as_object() {
        for (k, v) in obj {
            let val = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                other => other.to_string(),
            };
            out.push((k.clone(), val));
        }
    }
    if let Some(actor_id) = event.actor_id {
        out.push(("actor_id_envelope".into(), actor_id.to_string()));
        if !out.iter().any(|(k, _)| k == "actor_name") {
            out.push(("actor_name".into(), format!("actor {actor_id}")));
        }
    }
    if let Some(source_id) = event.source_id {
        out.push(("source_id_envelope".into(), source_id.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cause_chain_walker;
    use std::path::PathBuf;

    fn synth_event(event_id: &str, event_type: &str, parent: Option<&str>, payload: serde_json::Value) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "cause_chain_overlay_test".into(),
            tick: 0,
            sim_time_ms: 0.0,
            event_id: event_id.into(),
            category: "test".into(),
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

    fn en_locale() -> LocalizationTable {
        LocalizationTable::english_baseline().expect("english baseline must load")
    }

    fn es_locale() -> LocalizationTable {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .join("content/localization/es-ES.json");
        let txt = std::fs::read_to_string(&path).expect("es-ES.json must load");
        LocalizationTable::load_from_json(&txt).expect("es-ES table must parse")
    }

    #[test]
    fn rendered_lines_are_plain_language_without_payload_braces() {
        let mut e = synth_event(
            "trigger",
            "actor_status_changed",
            None,
            serde_json::json!({"status": "killed"}),
        );
        e.actor_id = Some(42);
        let locale = en_locale();
        let line = render_event_plain(&e, &locale);
        assert!(!line.is_empty());
        // Plain-language rejection clauses from VAL-M10B-030.
        assert!(
            !line.contains("event_type:"),
            "must not contain raw event_type prefix: {line}"
        );
        assert!(!line.contains("\"status\":"), "must not contain raw payload: {line}");
    }

    #[test]
    fn locale_switch_translates_killed_line() {
        let mut e = synth_event(
            "k",
            "actor_status_changed",
            None,
            serde_json::json!({"status": "killed"}),
        );
        e.actor_id = Some(99);
        let en = render_event_plain(&e, &en_locale());
        let es = render_event_plain(&e, &es_locale());
        assert!(en.contains("killed") || en.contains("died"));
        assert!(
            es.contains("matado") || es.contains("muri"),
            "es-ES line should translate: en={en} es={es}"
        );
        assert_ne!(en, es, "locale switch must alter rendered text");
    }

    #[test]
    fn cause_chain_renders_fence_shocked_actor_with_substrings() {
        // Synthesize a death chain ending in a fence_shocked_actor →
        // status.electrocuted → actor_status_changed=killed pattern
        // per VAL-CROSS-011.
        let events = vec![
            synth_event("a", "run_started", None, serde_json::json!({})),
            synth_event(
                "b",
                "fence_shocked_actor",
                Some("a"),
                serde_json::json!({
                    "fence_id": 7,
                    "actor_id": 42,
                    "shock_dose_j": 80,
                }),
            ),
            synth_event(
                "c",
                "actor_status_changed",
                Some("b"),
                serde_json::json!({"status": "killed"}),
            ),
        ];
        let trigger = cause_chain_walker::find_event(&events, "c").unwrap();
        let chain = cause_chain_walker::trace(&events, trigger, DEFAULT_MAX_DEPTH);
        let lines = render_chain(&chain, &en_locale());
        let combined = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            combined.contains("electrified_fence") || combined.contains("Electrified fence"),
            "cause-chain must include electrified_fence substring: {combined}"
        );
        assert!(
            combined.to_ascii_lowercase().contains("shock"),
            "cause-chain must include `shock` substring: {combined}"
        );
    }

    #[test]
    fn cause_chain_renders_mine_triggered_with_mine_prefix_and_yield() {
        let events = vec![
            synth_event("a", "run_started", None, serde_json::json!({})),
            synth_event(
                "b",
                "mine_triggered",
                Some("a"),
                serde_json::json!({
                    "mine_id": "mine_proximity_007",
                    "trigger_kind": "pressure",
                    "yield_joules": 120,
                }),
            ),
            synth_event(
                "c",
                "actor_status_changed",
                Some("b"),
                serde_json::json!({"status": "killed"}),
            ),
        ];
        let trigger = cause_chain_walker::find_event(&events, "c").unwrap();
        let chain = cause_chain_walker::trace(&events, trigger, DEFAULT_MAX_DEPTH);
        let lines = render_chain(&chain, &en_locale());
        let combined = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            combined.contains("mine_") || combined.contains("Mine "),
            "cause-chain must include mine_ substring: {combined}"
        );
        // Numeric yield substring (e.g. "120") must appear so the
        // viewer can read the J value.
        assert!(
            combined.contains("120"),
            "cause-chain must include numeric yield: {combined}"
        );
    }

    #[test]
    fn fallback_template_used_when_event_type_not_localised() {
        let event = synth_event("u", "unknown_event_type", None, serde_json::json!({}));
        let line = render_event_plain(&event, &en_locale());
        assert!(line.contains("unknown_event_type"));
        assert!(line.contains("tick 0") || line.contains("Event"));
    }
}
