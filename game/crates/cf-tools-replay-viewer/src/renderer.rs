//! M10 — Plain-language renderer for replay events.
//!
//! Every event-category surface the player or AI Self-Test grader reads
//! routes through this renderer. The renderer looks up `event_type` in a
//! template table and substitutes payload fields into a sentence. If no
//! template is registered, the fallback is the deterministic, JSON-free
//! string `event <category>.<event_type> at tick <N>` — never a raw payload
//! dump. This is the contract that satisfies the M10 spec's
//! `docs/plan/spec/death-recap-ux-contract.md` clause:
//!
//! > Plain-language rendering, never raw JSON to players.
//!
//! Templates are scoped to the M4+M7+M9 event-family surface currently in
//! the engine emission set; M13+/M16+/M19 placeholders use the same shape
//! so future producer events render cleanly when they ship.
//!
//! The renderer is a pure function of `(Event, payload-extractors)`; it
//! never reads bundle state, never panics on missing fields, and produces
//! byte-identical output across re-runs of the same input (deterministic
//! float quantization + sorted field order).

use cf_replay::Event;
use serde_json::Value;

/// Maximum length of a single rendered sentence. Anything longer is
/// truncated with `…` at a UTF-8 char boundary. Keeps debrief markdown
/// + death-recap modals scannable.
pub const MAX_SENTENCE_LEN: usize = 320;

/// Render an event as a plain-language sentence. Always prefixed by
/// `Tick {tick}: ` for chronological clarity. The renderer never returns
/// raw JSON; on unknown event types it falls back to a short identifier
/// line that surfaces the category + type so the AI Self-Test grader can
/// still grep for missing templates.
pub fn render_event_plain(event: &Event) -> String {
    let body = render_event_body(event);
    let prefix = format!("Tick {}: ", event.tick);
    truncate_sentence(&format!("{prefix}{body}"))
}

/// Same as `render_event_plain` but without the `Tick {N}: ` prefix. Used
/// by callers that want to embed the body inside their own structure.
pub fn render_event_body(event: &Event) -> String {
    let p = &event.payload;
    let etype = event.event_type.as_str();
    let cat = event.category.as_str();
    let rendered = match etype {
        // --- input + control surface ---------------------------------------
        "intent_received" => format!(
            "{} pressed {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "an actor".into()),
            field_str(p, "action").unwrap_or_else(|| "action".into())
        ),
        // **M14 audit fix** (pre-existing M3B bug): the plain-language
        // renderer must NOT surface raw payload tokens. The previous
        // rendering inlined the method literal (e.g. "act.player.fire")
        // which broke the cause-chain plain-language contract and tripped
        // `render_markdown_emits_event_id_and_chain_arrows`. Translate the
        // method into a verb phrase and drop the raw token.
        "command_accepted" => {
            let method = field_str(p, "method").unwrap_or_else(|| "command".into());
            let phrase = match method.as_str() {
                "act.player.fire" => "fire command".to_string(),
                "act.player.move" => "move command".to_string(),
                "act.player.aim" => "aim command".to_string(),
                "act.player.jump" => "jump command".to_string(),
                "act.player.reload" => "reload command".to_string(),
                "act.player.dig" => "dig command".to_string(),
                "act.player.crouch" => "crouch command".to_string(),
                "act.player.climb" => "climb command".to_string(),
                "act.player.eject" => "eject command".to_string(),
                "act.player.board" => "board command".to_string(),
                "act.player.disembark" => "disembark command".to_string(),
                other if other.starts_with("act.player.") => format!("{} command", &other[11..]),
                other if other.starts_with("act.") => "control command".to_string(),
                _ => "command".to_string(),
            };
            format!("control accepted {phrase}")
        }
        // --- equipment / weapon surface ------------------------------------
        "weapon_fired" => format!(
            "{}'s {} fired ({} round{})",
            actor_label(p, "shooter_actor_id")
                .or_else(|| actor_label(p, "shooter"))
                .unwrap_or_else(|| "an actor".into()),
            field_str(p, "weapon_name")
                .or_else(|| field_str(p, "weapon"))
                .unwrap_or_else(|| "weapon".into()),
            field_u64(p, "rounds_count").unwrap_or(1),
            if field_u64(p, "rounds_count").unwrap_or(1) == 1 {
                ""
            } else {
                "s"
            }
        ),
        "magazine_changed" => format!(
            "{}'s magazine: {} → {} round{}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_u64(p, "from").unwrap_or(0),
            field_u64(p, "to").unwrap_or(0),
            if field_u64(p, "to").unwrap_or(0) == 1 { "" } else { "s" }
        ),
        "tool_broken" => format!(
            "{}'s {} broke (durability hit 0)",
            actor_label(p, "actor_id").unwrap_or_else(|| "an actor".into()),
            field_str(p, "tool_name")
                .or_else(|| field_str(p, "tool"))
                .unwrap_or_else(|| "tool".into())
        ),
        "tool_refused" => format!(
            "{} refused on {} ({})",
            field_str(p, "tool_name")
                .or_else(|| field_str(p, "tool"))
                .unwrap_or_else(|| "tool".into()),
            field_str(p, "material_name")
                .or_else(|| field_str(p, "material"))
                .unwrap_or_else(|| "the target".into()),
            field_str(p, "reason").unwrap_or_else(|| "rejected".into())
        ),
        // --- projectile + hit surface --------------------------------------
        "projectile_spawned" => format!(
            "projectile spawned at {} (velocity {})",
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            point_label(p, "velocity").unwrap_or_else(|| "—".into())
        ),
        "projectile_hit" | "projectile_hit_mo" => format!(
            "{}'s shot hit {}'s {} for {} damage",
            actor_label(p, "shooter_actor_id")
                .or_else(|| actor_label(p, "shooter"))
                .unwrap_or_else(|| "an actor".into()),
            actor_label(p, "target_actor_id")
                .or_else(|| actor_label(p, "target"))
                .unwrap_or_else(|| "a target".into()),
            field_str(p, "body_zone")
                .or_else(|| field_str(p, "surface_kind"))
                .unwrap_or_else(|| "body".into()),
            field_f64(p, "damage").map(short_num).unwrap_or_else(|| "?".into())
        ),
        "wound_added" => format!(
            "{} wounded ({} severity)",
            actor_label(p, "target_actor_id")
                .or_else(|| actor_label(p, "actor_id"))
                .unwrap_or_else(|| "actor".into()),
            field_str(p, "severity").unwrap_or_else(|| "unknown".into())
        ),
        // --- actor + status surface ----------------------------------------
        "actor_status_changed" => format!(
            "{} status: {} → {} ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "from")
                .or_else(|| field_str(p, "from_status"))
                .unwrap_or_else(|| "?".into()),
            field_str(p, "to")
                .or_else(|| field_str(p, "to_status"))
                .unwrap_or_else(|| "?".into()),
            field_str(p, "cause").unwrap_or_else(|| "?".into())
        ),
        "actor_died" => format!(
            "{} died ({})",
            actor_label(p, "actor_id")
                .or_else(|| actor_label(p, "actor"))
                .unwrap_or_else(|| "actor".into()),
            field_str(p, "cause").unwrap_or_else(|| "cause unknown".into())
        ),
        "actor_stance_changed" => format!(
            "{} stance: {} → {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "from").unwrap_or_else(|| "?".into()),
            field_str(p, "to").unwrap_or_else(|| "?".into())
        ),
        "actor_facing_changed" => format!(
            "{} now facing {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "to")
                .or_else(|| field_str(p, "facing"))
                .unwrap_or_else(|| "?".into())
        ),
        "inventory_dropped" => format!(
            "{} dropped {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "item_name")
                .or_else(|| field_str(p, "item"))
                .unwrap_or_else(|| "an item".into())
        ),
        // --- terrain surface (M3+M9) ---------------------------------------
        "terrain_carved" => format!(
            "{} carved {} pixels of {}",
            field_str(p, "tool_name")
                .or_else(|| field_str(p, "tool"))
                .unwrap_or_else(|| "tool".into()),
            field_u64(p, "count")
                .or_else(|| field_u64(p, "pixel_count"))
                .unwrap_or(0),
            field_str(p, "material_name")
                .or_else(|| field_str(p, "material"))
                .unwrap_or_else(|| "terrain".into())
        ),
        "material_state_changed" => format!(
            "{} at {} degraded {} → {} ({}% integrity)",
            field_str(p, "material").unwrap_or_else(|| "material".into()),
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            field_str(p, "from_band").unwrap_or_else(|| "?".into()),
            field_str(p, "to_band").unwrap_or_else(|| "?".into()),
            field_f64(p, "integrity_pct")
                .map(short_num)
                .unwrap_or_else(|| "?".into())
        ),
        "pixel_removed" => format!(
            "{} at {} destroyed ({})",
            field_str(p, "material").unwrap_or_else(|| "pixel".into()),
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            field_str(p, "cause").unwrap_or_else(|| "removed".into())
        ),
        "cascade_triggered" => format!(
            "Damage cascaded at {} ({})",
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            field_str(p, "reason").unwrap_or_else(|| "neighbor decay".into())
        ),
        // --- mission surface -----------------------------------------------
        "mission_started" => format!(
            "Mission started ({})",
            field_str(p, "scenario_id").unwrap_or_else(|| "scenario".into())
        ),
        "mission_resolved" => match (
            field_str(p, "result"),
            field_str(p, "loss_reason").or_else(|| field_str(p, "reason")),
        ) {
            (Some(r), Some(reason)) => format!("Mission ended — {} ({})", r, reason),
            (Some(r), None) => format!("Mission ended — {}", r),
            (None, _) => "Mission ended".to_string(),
        },
        "objective_started" => format!(
            "Objective started — {}",
            field_str(p, "objective")
                .or_else(|| field_str(p, "name"))
                .unwrap_or_else(|| "(unnamed)".into())
        ),
        "objective_completed" => format!(
            "Objective `{}` completed",
            field_str(p, "objective")
                .or_else(|| field_str(p, "name"))
                .unwrap_or_else(|| "(unnamed)".into())
        ),
        "objective_failed" => format!(
            "Objective `{}` failed ({})",
            field_str(p, "objective")
                .or_else(|| field_str(p, "name"))
                .unwrap_or_else(|| "(unnamed)".into()),
            field_str(p, "reason").unwrap_or_else(|| "unknown".into())
        ),
        "reactor_destroyed" | "mission_reactor_destroyed" => format!(
            "Reactor destroyed{}",
            field_str(p, "source_name")
                .or_else(|| field_str(p, "shooter"))
                .map(|s| format!(" by {s}"))
                .unwrap_or_default()
        ),
        "reactor_hp_changed" | "mission_reactor_hp_changed" => format!(
            "Reactor HP: {} → {} ({})",
            field_f64(p, "hp_before")
                .or_else(|| field_f64(p, "from"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
            field_f64(p, "hp_after")
                .or_else(|| field_f64(p, "to"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
            field_str(p, "cause").unwrap_or_else(|| "damage".into())
        ),
        // --- AI surface (M7) -----------------------------------------------
        "state_changed" if cat == "ai" => format!(
            "{} AI: {} → {} ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "from_state").unwrap_or_else(|| "?".into()),
            field_str(p, "to_state").unwrap_or_else(|| "?".into()),
            field_str(p, "reason").unwrap_or_else(|| "—".into())
        ),
        "target_acquired" => format!(
            "{} acquired target {} ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "AI".into()),
            actor_label(p, "target_actor_id")
                .or_else(|| actor_label(p, "target"))
                .unwrap_or_else(|| "target".into()),
            field_str(p, "reason").unwrap_or_else(|| "visible".into())
        ),
        "missed_shot_reason" => format!(
            "{} missed ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "AI".into()),
            field_str(p, "reason").unwrap_or_else(|| "no reason".into())
        ),
        "reason_label_changed" => format!(
            "{} chose `{}` (score {:.2})",
            actor_label(p, "actor_id").unwrap_or_else(|| "AI".into()),
            field_str(p, "chosen_task").unwrap_or_else(|| "—".into()),
            field_f64(p, "score").unwrap_or(0.0),
        ),
        "thinking_layer_invoked" => format!(
            "{} thinking layers fired: {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "AI".into()),
            layers_label(p)
        ),
        // --- chassis (M13+) ------------------------------------------------
        "stage_changed" if cat == "chassis" => format!(
            "{}'s chassis: {} → {} ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "from_stage").unwrap_or_else(|| "?".into()),
            field_str(p, "to_stage").unwrap_or_else(|| "?".into()),
            field_str(p, "reason").unwrap_or_else(|| "—".into())
        ),
        "pilot_ejected" => format!(
            "{}'s pilot ejected",
            actor_label(p, "actor_id").unwrap_or_else(|| "an actor".into())
        ),
        // --- armor (M9) ----------------------------------------------------
        "armor_layer_hp_changed" | "layer_hp_changed" if cat == "armor" => format!(
            "{}'s {} {} HP: {} → {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "zone").unwrap_or_else(|| "armor".into()),
            field_str(p, "layer").unwrap_or_else(|| "layer".into()),
            field_f64(p, "from").map(short_num).unwrap_or_else(|| "?".into()),
            field_f64(p, "to").map(short_num).unwrap_or_else(|| "?".into()),
        ),
        "armor_layer_destroyed" | "layer_destroyed" if cat == "armor" => format!(
            "{}'s {} {} armor destroyed ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "zone").unwrap_or_else(|| "armor".into()),
            field_str(p, "layer").unwrap_or_else(|| "layer".into()),
            field_str(p, "breach_kind").unwrap_or_else(|| "breached".into())
        ),
        // --- hazard (M9) ---------------------------------------------------
        "spawned" if cat == "hazard" => format!(
            "{} hazard spawned at {} (intensity {})",
            field_str(p, "kind").unwrap_or_else(|| "hazard".into()),
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            field_f64(p, "intensity").map(short_num).unwrap_or_else(|| "?".into())
        ),
        "actor_contact" if cat == "hazard" => format!(
            "{} contacted {} hazard",
            actor_label(p, "actor_id").unwrap_or_else(|| "an actor".into()),
            field_str(p, "kind").unwrap_or_else(|| "hazard".into())
        ),
        "dissipated" if cat == "hazard" => format!(
            "{} hazard at {} dissipated ({})",
            field_str(p, "kind").unwrap_or_else(|| "hazard".into()),
            point_label(p, "position").unwrap_or_else(|| "—".into()),
            field_str(p, "reason").unwrap_or_else(|| "decay".into())
        ),
        // --- affliction (M9 / M16+) ----------------------------------------
        "applied" if cat == "affliction" => format!(
            "{} afflicted with {} (severity {})",
            actor_label(p, "actor_id").unwrap_or_else(|| "an actor".into()),
            field_str(p, "kind").unwrap_or_else(|| "affliction".into()),
            field_f64(p, "severity").map(short_num).unwrap_or_else(|| "?".into())
        ),
        "escalated" if cat == "affliction" => format!(
            "{}'s {} escalated severity {} → {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "kind").unwrap_or_else(|| "affliction".into()),
            field_f64(p, "from_severity")
                .or_else(|| field_f64(p, "from"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
            field_f64(p, "to_severity")
                .or_else(|| field_f64(p, "to"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
        ),
        "cleared" if cat == "affliction" => format!(
            "{}'s {} cleared ({})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "kind").unwrap_or_else(|| "affliction".into()),
            field_str(p, "reason").unwrap_or_else(|| "naturally".into())
        ),
        // --- internal / concussion / fluid (M9 deep damage) ----------------
        "organ_damaged" => format!(
            "{}'s {} took {} damage (HP {} → {})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "organ_kind")
                .or_else(|| field_str(p, "organ"))
                .unwrap_or_else(|| "organ".into()),
            field_f64(p, "damage").map(short_num).unwrap_or_else(|| "?".into()),
            field_f64(p, "from_hp")
                .or_else(|| field_f64(p, "from"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
            field_f64(p, "to_hp")
                .or_else(|| field_f64(p, "to"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
        ),
        "organ_destroyed" => format!(
            "{}'s {} destroyed",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "organ_kind")
                .or_else(|| field_str(p, "organ"))
                .unwrap_or_else(|| "organ".into())
        ),
        "circuit_damaged" => format!(
            "{}'s {} circuit damaged (HP {} → {})",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "circuit_kind")
                .or_else(|| field_str(p, "circuit"))
                .unwrap_or_else(|| "circuit".into()),
            field_f64(p, "from_hp")
                .or_else(|| field_f64(p, "from"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
            field_f64(p, "to_hp")
                .or_else(|| field_f64(p, "to"))
                .map(short_num)
                .unwrap_or_else(|| "?".into()),
        ),
        "concussion_band_changed" | "band_changed" if cat == "concussion" || cat == "internal_shock" => format!(
            "{} concussion band: {} → {}",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "from").unwrap_or_else(|| "?".into()),
            field_str(p, "to").unwrap_or_else(|| "?".into())
        ),
        "fluid_leak_started" | "leak_started" if cat == "fluid" => format!(
            "{} {} leak started ({} L/s)",
            actor_label(p, "actor_id").unwrap_or_else(|| "actor".into()),
            field_str(p, "fluid_kind")
                .or_else(|| field_str(p, "fluid"))
                .unwrap_or_else(|| "fluid".into()),
            field_f64(p, "leak_rate")
                .or_else(|| field_f64(p, "rate"))
                .map(short_num)
                .unwrap_or_else(|| "?".into())
        ),
        // --- system bookkeeping --------------------------------------------
        "run_started" => "run started".to_string(),
        "run_finished" => "run finished".to_string(),
        "category_baseline" => "category baseline registered".to_string(),
        _ => return fallback(event),
    };
    rendered
}

/// Fallback rendering when no template is registered. Surfaces the
/// category + type so the AI Self-Test grader can grep for missing
/// templates and never leaks a raw payload to the player.
fn fallback(event: &Event) -> String {
    format!("event {}.{}", event.category, event.event_type)
}

/// Extract a `String` payload field, defaulting to None on absent/non-string.
fn field_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract an unsigned-integer payload field.
fn field_u64(p: &Value, key: &str) -> Option<u64> {
    p.get(key).and_then(|v| v.as_u64())
}

/// Extract a numeric (f64) payload field; quantizes Inf/NaN to None.
fn field_f64(p: &Value, key: &str) -> Option<f64> {
    p.get(key).and_then(|v| v.as_f64()).filter(|f| f.is_finite())
}

/// Render an `actor_id`-style field as a labelled identifier. Prefers
/// `actor_name`/`name` siblings when present (engine attaches these for
/// debug-friendly output); otherwise falls back to `actor #N`. Engine
/// emission sites that drop the integer entirely produce None so callers
/// can supply their own label.
fn actor_label(p: &Value, key: &str) -> Option<String> {
    if let Some(name) = p.get(format!("{key}_name")).and_then(|v| v.as_str()) {
        return Some(name.to_string());
    }
    if key == "actor_id" {
        if let Some(n) = p.get("actor_name").and_then(|v| v.as_str()) {
            return Some(n.to_string());
        }
    }
    let n = p.get(key).and_then(|v| v.as_u64())?;
    Some(format!("actor #{n}"))
}

/// Render a `[x, y]` array payload field as `(x, y)`.
fn point_label(p: &Value, key: &str) -> Option<String> {
    let arr = p.get(key)?.as_array()?;
    let x = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
    Some(format!("({}, {})", short_num(x), short_num(y)))
}

/// Render `layers` payload (string array) as comma-joined sorted list.
fn layers_label(p: &Value) -> String {
    let Some(arr) = p.get("layers").and_then(|v| v.as_array()) else {
        return "—".into();
    };
    let mut s: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    s.sort();
    if s.is_empty() {
        "—".into()
    } else {
        s.join(", ")
    }
}

/// Quantize a float to a short, deterministic string.
fn short_num(f: f64) -> String {
    if !f.is_finite() {
        return "?".into();
    }
    if (f.fract().abs()) < 1e-6 {
        format!("{:.0}", f)
    } else {
        format!("{:.2}", f)
    }
}

/// Truncate a sentence at MAX_SENTENCE_LEN respecting char boundaries.
fn truncate_sentence(s: &str) -> String {
    if s.len() <= MAX_SENTENCE_LEN {
        return s.to_string();
    }
    let mut idx = MAX_SENTENCE_LEN;
    while !s.is_char_boundary(idx) && idx > 0 {
        idx -= 1;
    }
    let mut out: String = s[..idx].to_string();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_replay::Event;
    use serde_json::json;

    fn make_event(category: &str, event_type: &str, tick: u64, payload: serde_json::Value) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "test".into(),
            tick,
            sim_time_ms: tick as f64 * 16.6,
            event_id: format!("test:{tick}:0"),
            category: category.into(),
            event_type: event_type.into(),
            payload,
            parent_event_id: None,
            actor_id: None,
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: None,
            cosmetic: None,
            asset_ref: None,
        }
    }

    #[test]
    fn renders_weapon_fired_as_sentence_not_json() {
        let e = make_event(
            "equipment",
            "weapon_fired",
            42,
            json!({"shooter_actor_id": 7, "weapon_name": "rifle_m1", "rounds_count": 1}),
        );
        let s = render_event_plain(&e);
        assert_eq!(s, "Tick 42: actor #7's rifle_m1 fired (1 round)");
        assert!(!s.contains("{"), "must not contain JSON");
    }

    #[test]
    fn renders_projectile_hit_with_damage_and_zone() {
        let e = make_event(
            "combat",
            "projectile_hit",
            120,
            json!({"shooter_actor_id": 1, "target_actor_id": 2, "body_zone": "torso", "damage": 15.0}),
        );
        let s = render_event_plain(&e);
        assert_eq!(s, "Tick 120: actor #1's shot hit actor #2's torso for 15 damage");
    }

    #[test]
    fn renders_actor_died_with_cause() {
        let e = make_event(
            "actor",
            "actor_died",
            521,
            json!({"actor_id": 2, "cause": "projectile"}),
        );
        assert_eq!(render_event_plain(&e), "Tick 521: actor #2 died (projectile)");
    }

    #[test]
    fn renders_terrain_carved_with_count() {
        let e = make_event(
            "terrain",
            "terrain_carved",
            33,
            json!({"tool_name": "shovel", "count": 42, "material_name": "dirt"}),
        );
        assert_eq!(render_event_plain(&e), "Tick 33: shovel carved 42 pixels of dirt");
    }

    #[test]
    fn renders_mission_resolved_with_loss_reason() {
        let e = make_event(
            "mission",
            "mission_resolved",
            4521,
            json!({"result": "lost", "loss_reason": "reactor_destroyed"}),
        );
        assert_eq!(
            render_event_plain(&e),
            "Tick 4521: Mission ended — lost (reactor_destroyed)"
        );
    }

    #[test]
    fn renders_unknown_event_via_fallback_never_raw_json() {
        let e = make_event(
            "totally_made_up_category",
            "totally_made_up_type",
            999,
            json!({"raw": {"deep": "value"}}),
        );
        let s = render_event_plain(&e);
        assert!(s.starts_with("Tick 999: event totally_made_up_category.totally_made_up_type"));
        assert!(!s.contains("deep"), "fallback must NOT leak raw payload");
        assert!(!s.contains("{"), "fallback must NOT contain JSON braces");
    }

    #[test]
    fn renders_ai_reason_label_changed_with_score() {
        let e = make_event(
            "ai",
            "reason_label_changed",
            4271,
            json!({"actor_id": 5, "chosen_task": "TriageDownedAlly", "score": 0.92}),
        );
        assert_eq!(
            render_event_plain(&e),
            "Tick 4271: actor #5 chose `TriageDownedAlly` (score 0.92)"
        );
    }

    #[test]
    fn renders_ai_thinking_layers_sorted_deterministically() {
        let e = make_event(
            "ai",
            "thinking_layer_invoked",
            4271,
            json!({"actor_id": 5, "layers": ["Utility", "BehaviorTree", "Reactive"]}),
        );
        let s = render_event_plain(&e);
        // Sorted alphabetically for determinism.
        assert_eq!(
            s,
            "Tick 4271: actor #5 thinking layers fired: BehaviorTree, Reactive, Utility"
        );
    }

    #[test]
    fn truncates_overlong_sentence_at_char_boundary() {
        let long = "x".repeat(MAX_SENTENCE_LEN * 2);
        let truncated = truncate_sentence(&long);
        assert!(truncated.ends_with('…'));
        // Truncated must remain valid UTF-8 (every byte is on a char boundary).
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn actor_label_prefers_named_field_when_present() {
        let p = json!({"actor_id": 7, "actor_id_name": "Jenkins"});
        assert_eq!(actor_label(&p, "actor_id").as_deref(), Some("Jenkins"));
    }

    #[test]
    fn short_num_quantizes_integer_floats_without_decimals() {
        assert_eq!(short_num(15.0), "15");
        assert_eq!(short_num(15.234), "15.23");
        assert_eq!(short_num(f64::INFINITY), "?");
        assert_eq!(short_num(f64::NAN), "?");
    }
}
