use serde_json::Value;

/// Lookup `key` inside the observe.once result envelope. Supports a couple of
/// shortcuts on top of dotted paths:
///
/// - `mission.result` / `mission.loss_reason` / `mission.active_objective`
/// - `objective.<id>` => `mission.objectives[id==<id>].status`
/// - `breach.<id>.broken` / `breach.<id>.hp` etc.
/// - `enemy.<actor_id>.state` etc.
pub(crate) fn lookup(value: &Value, key: &str) -> Option<Value> {
    // § "Crates / modules touched / cf-e2e":
    //   mission.reactor_alive          (bool — true while first reactor's hp > 0)
    //   mission.reactor_hp_pct         (number — first reactor's hp_percent)
    //   mission.physics_kill_count     (count of combat.projectile_hit events that
    //                                   resulted in actor.actor_status_changed → dead)
    //   terrain.terrain_carved.count   (already handled by event-stream resolver)
    //   mission.timer_remaining_ticks  (already handled below)
    //   mission.reactor_destroyed.count (already handled by event-stream resolver)
    match key {
        "mission.reactor_alive" => {
            // First reactor in the snapshot. `reactors` is the run-bundle's
            // serialized ReactorWorld; absence means no reactor → not alive.
            if let Some(reactors) = value.get("reactors").and_then(|r| r.as_array()) {
                let alive = reactors
                    .iter()
                    .next()
                    .and_then(|r| r.get("hp").and_then(|h| h.as_f64()))
                    .map(|hp| hp > 0.0)
                    .unwrap_or(false);
                return Some(Value::Bool(alive));
            }
            // Fall back to the mission state's `reactors_destroyed` flag.
            if let Some(d) = value
                .get("mission")
                .and_then(|m| m.get("reactors_destroyed"))
                .and_then(|d| d.as_object())
            {
                let any_alive = d.values().any(|v| !v.as_bool().unwrap_or(false));
                return Some(Value::Bool(any_alive));
            }
            None
        }
        "mission.reactor_hp_pct" => value
            .get("reactors")
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.iter().next())
            .and_then(|reactor| {
                let hp = reactor.get("hp").and_then(|h| h.as_f64())?;
                let max_hp = reactor.get("max_hp").and_then(|h| h.as_f64())?;
                if max_hp <= 0.0 {
                    Some(Value::from(0.0))
                } else {
                    Some(Value::from((hp / max_hp).clamp(0.0, 1.0)))
                }
            }),
        "mission.physics_kill_count" => {
            // Count actor.actor_status_changed events where new_status=dead
            // and cause includes projectile_hit (physics damage path).
            let events = value.get("events").and_then(|e| e.as_array())?;
            let count = events
                .iter()
                .filter(|e| {
                    let cat = e.get("category").and_then(|c| c.as_str()).unwrap_or("");
                    let typ = e.get("event_type").and_then(|c| c.as_str()).unwrap_or("");
                    let payload = e.get("payload");
                    let new_status = payload
                        .and_then(|p| p.get("new_status"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    let cause = payload
                        .and_then(|p| p.get("cause"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    cat == "actor"
                        && typ == "actor_status_changed"
                        && (new_status == "dead" || new_status == "Dead" || new_status == "DEAD")
                        && cause.contains("projectile")
                })
                .count();
            Some(Value::from(count as u64))
        }
        // `observe.actor.silhouette.<zone>` / `observe.actor.module_strip.<slot>` /
        // `ux.banner_raised.severity` operators are spec-named convenience aliases
        // that resolve against the live observe-frame projection.
        key if key.starts_with("observe.accessibility.") => {
            let rest = &key["observe.accessibility.".len()..];
            value.get("accessibility").and_then(|a| lookup_inner(a, rest))
        }
        key if key.starts_with("observe.actor.silhouette.") => {
            let rest = &key["observe.actor.silhouette.".len()..];
            value
                .get("actors")
                .and_then(|a| a.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|a| a.get("controllable").and_then(|c| c.as_bool()) == Some(true))
                })
                .and_then(|player| player.get("body_silhouette"))
                .and_then(|sil| lookup_inner(sil, rest))
        }
        key if key.starts_with("observe.actor.module_strip.") => {
            let rest = &key["observe.actor.module_strip.".len()..];
            value
                .get("actors")
                .and_then(|a| a.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|a| a.get("controllable").and_then(|c| c.as_bool()) == Some(true))
                })
                .and_then(|player| player.get("module_strip"))
                .and_then(|strip| {
                    // Slot lookup: <slot_id>=<state>. Iterate modules array.
                    let modules = strip.get("modules")?.as_array()?;
                    modules
                        .iter()
                        .find(|m| m.get("id").and_then(|i| i.as_str()) == Some(rest))
                        .and_then(|m| m.get("state").cloned())
                })
        }
        "ux.banner_raised.severity" => {
            // The most-recent ux.banner_raised event's severity field, if any.
            let events = value.get("events").and_then(|e| e.as_array())?;
            events
                .iter()
                .rev()
                .find(|e| {
                    let cat = e.get("category").and_then(|c| c.as_str()).unwrap_or("");
                    let typ = e.get("event_type").and_then(|c| c.as_str()).unwrap_or("");
                    cat == "ux" && typ == "banner_raised"
                })
                .and_then(|e| e.get("payload"))
                .and_then(|p| p.get("severity"))
                .cloned()
        }
        _ => None,
    }
    .or_else(|| lookup_inner(value, key))
}

/// Inner walker; isolated so the M9 shortcut keys above don't need to call
/// the full event-stream resolver.
fn lookup_inner(value: &Value, key: &str) -> Option<Value> {
    // for cfctl scripts that need to assert "K events of type X with field
    // Y = Z fired during this run." The grammar:
    //
    // - `events.count` ........................ total event count.
    // - `events.<category>.count` ............. count by category.
    // - `events.<category>.<event_type>.count`  count by category+type.
    // - `events.<category>.<event_type>.last.payload.<field>`
    //                                          last matching event's payload field.
    // - `events.where(<f1=v1>,<f2=v2>).count`  count where field=value for
    //                                          all listed fields (and-of).
    // - `events.where(<f1=v1>).last.payload.<field>` analogous.
    //
    // Mission shorthands (deferred to the existing `mission.*` path) remain
    // unchanged.
    if let Some(rest) = key.strip_prefix("events.where(") {
        return lookup_events_where(value, rest);
    }
    if let Some(rest) = key.strip_prefix("events.") {
        return lookup_events_dotted(value, rest);
    }
    if key == "events.count" || key == "events" {
        // `events` alone returns the raw array. `events.count` falls through
        // to the generic walker below (handled there).
        if key == "events" {
            return value.get("events").cloned();
        }
    }
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() >= 2 && parts[0] == "objective" {
        let id = parts[1];
        let arr = value.get("mission")?.get("objectives")?.as_array()?;
        let obj = arr.iter().find(|o| o.get("id").and_then(|i| i.as_str()) == Some(id))?;
        if parts.len() == 2 {
            return obj.get("status").cloned();
        }
        let mut node = obj;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    if parts.len() >= 2 && parts[0] == "breach" {
        let id = parts[1];
        let arr = value.get("breaches")?.as_array()?;
        let strip = arr.iter().find(|s| s.get("id").and_then(|i| i.as_str()) == Some(id))?;
        if parts.len() == 2 {
            return Some(strip.clone());
        }
        let mut node = strip;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    if parts.len() >= 2 && parts[0] == "enemy" {
        let actor: u64 = parts[1].parse().ok()?;
        let arr = value.get("enemies")?.as_array()?;
        let enemy = arr
            .iter()
            .find(|e| e.get("actor").and_then(|i| i.as_u64()) == Some(actor))?;
        if parts.len() == 2 {
            return Some(enemy.clone());
        }
        let mut node = enemy;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    // M5: `actor.<id>.foo.bar` lookup against `actors[]` by id (`actor.player.*` also accepted).
    // either the literal "player" or a parseable u64 id. Otherwise the path
    // looks like `actor.<event_type>.count` (a bare event-stream expectation,
    // per the spec text) and we fall through to the event-stream passthrough
    // below. Same reasoning for `breach.<event_type>` and `enemy.<event_type>`.
    if parts.len() >= 2 && parts[0] == "actor" && (parts[1] == "player" || parts[1].parse::<u64>().is_ok()) {
        let arr = value.get("actors")?.as_array()?;
        let actor_match = if parts[1] == "player" {
            let pid = value.get("player_actor_id").and_then(|i| i.as_u64())?;
            arr.iter().find(|a| a.get("id").and_then(|i| i.as_u64()) == Some(pid))?
        } else {
            let pid: u64 = parts[1].parse().ok()?;
            arr.iter().find(|a| a.get("id").and_then(|i| i.as_u64()) == Some(pid))?
        };
        if parts.len() == 2 {
            return Some(actor_match.clone());
        }
        let mut current: Value = actor_match.clone();
        for seg in &parts[2..] {
            if *seg == "count" {
                if let Some(arr) = current.as_array() {
                    current = Value::from(arr.len() as u64);
                    continue;
                }
                return None;
            }
            current = current.get(*seg)?.clone();
        }
        return Some(current);
    }
    // `ai.state_changed.count>=N`, `terrain.terrain_carved.count>=N`,
    // `mission.objective_completed.count>=N` etc. (no `events.` prefix). If
    // the first segment matches a known event category AND the path's
    // intent is clearly event-stream (last segment ∈ {count, first, last}
    // OR contains a `.last.payload.` / `.first.payload.` drill-down), route
    // through the event-stream resolver. This keeps the spec text honest
    // without colliding with the existing `mission.result` / `mission.loss_reason`
    // /  `mission.objective.<id>.status` / `mission.timer_remaining_ticks`
    // shorthand paths the lookup walker resolves elsewhere.
    const KNOWN_EVENT_CATEGORIES: &[&str] = &[
        "accessibility",
        "actor",
        "ai",
        "chassis",
        "combat",
        "control",
        "determinism",
        "equipment",
        "input",
        "mission",
        "physics",
        "system",
        "terrain",
        "ux",
    ];
    let looks_like_event_stream = parts.last().is_some_and(|seg| {
        matches!(*seg, "count" | "first" | "last")
            || parts
                .windows(2)
                .any(|w| (w[0] == "first" || w[0] == "last") && (w[1] == "payload" || w[1] == "event_id"))
    });
    if parts.len() >= 2 && KNOWN_EVENT_CATEGORIES.contains(&parts[0]) && looks_like_event_stream {
        if let Some(v) = lookup_events_dotted(value, key) {
            return Some(v);
        }
    }
    let mut current: Value = value.clone();
    for seg in &parts {
        if *seg == "count" {
            if let Some(arr) = current.as_array() {
                current = Value::from(arr.len() as u64);
                continue;
            }
            return None;
        }
        let next = current.get(*seg)?.clone();
        current = next;
    }
    Some(current)
}

/// Resolve an `events.<category>[.<event_type>][.last.payload.<field>][.count]`
/// lookup against the observation snapshot's `events` array.
///
/// Grammar:
///   events.<cat>.count                          → matching count
///   events.<cat>.<type>.count                   → cat+type count
///   events.<cat>.<type>.last.payload.<field>    → payload field of last match
///   events.<cat>.<type>.last.event_id           → event_id of last match
fn lookup_events_dotted(observation: &Value, rest: &str) -> Option<Value> {
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let events = observation.get("events")?.as_array()?;
    // Single-segment "events.count" already handled by caller's generic walker
    // because it sits on `value.events`. Here we expect at least a category.
    if parts.len() == 1 && parts[0] == "count" {
        return Some(Value::from(events.len() as u64));
    }
    let category = parts[0];
    // Filter by category first.
    let mut filtered: Vec<&Value> = events
        .iter()
        .filter(|e| e.get("category").and_then(|v| v.as_str()) == Some(category))
        .collect();
    let tail = &parts[1..];
    if tail.is_empty() {
        return Some(Value::from(filtered.len() as u64));
    }
    if tail.len() == 1 && tail[0] == "count" {
        return Some(Value::from(filtered.len() as u64));
    }
    // tail[0] may be an event_type filter; if it doesn't look like a special
    // token (count/last/payload/where), treat it as a type filter.
    let mut tail_iter: &[&str] = tail;
    let reserved = ["count", "last", "payload", "first", "event_id"];
    if !reserved.contains(&tail[0]) {
        let event_type = tail[0];
        filtered.retain(|e| e.get("event_type").and_then(|v| v.as_str()) == Some(event_type));
        tail_iter = &tail[1..];
    }
    resolve_event_subpath(&filtered, tail_iter)
}

/// `events.where(category=actor,event_type=inventory_settled).count` style.
/// `rest` begins **after** `events.where(`.
fn lookup_events_where(observation: &Value, rest: &str) -> Option<Value> {
    let close = rest.find(')')?;
    let filter_expr = &rest[..close];
    let after = rest[close + 1..].trim_start_matches('.');
    let events = observation.get("events")?.as_array()?;
    let filters: Vec<(&str, &str)> = filter_expr
        .split(',')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect();
    if filters.is_empty() {
        return None;
    }
    let filtered: Vec<&Value> = events
        .iter()
        .filter(|e| {
            filters.iter().all(|(k, v)| {
                // The filter key can be a dotted payload path
                // (e.g. payload.zone=head).
                let candidate = if let Some(payload_field) = k.strip_prefix("payload.") {
                    e.get("payload").and_then(|p| p.get(payload_field))
                } else {
                    e.get(*k)
                };
                match candidate {
                    Some(Value::String(s)) => s == v,
                    Some(Value::Bool(b)) => *b == matches!(*v, "true" | "1"),
                    Some(Value::Number(n)) => v.parse::<f64>().is_ok_and(|x| n.as_f64() == Some(x)),
                    _ => false,
                }
            })
        })
        .collect();
    if after.is_empty() {
        return Some(Value::from(filtered.len() as u64));
    }
    let parts: Vec<&str> = after.split('.').collect();
    resolve_event_subpath(&filtered, &parts)
}

fn resolve_event_subpath(filtered: &[&Value], parts: &[&str]) -> Option<Value> {
    if parts.is_empty() || (parts.len() == 1 && parts[0] == "count") {
        return Some(Value::from(filtered.len() as u64));
    }
    if parts[0] == "first" || parts[0] == "last" {
        let target = if parts[0] == "first" {
            filtered.first()
        } else {
            filtered.last()
        };
        let target = target?;
        if parts.len() == 1 {
            return Some((*target).clone());
        }
        if parts[1] == "payload" {
            let payload = target.get("payload")?;
            if parts.len() == 2 {
                return Some(payload.clone());
            }
            let mut cur = payload;
            for seg in &parts[2..] {
                cur = cur.get(*seg)?;
            }
            return Some(cur.clone());
        }
        if parts[1] == "event_id" {
            return target.get("event_id").cloned();
        }
        // Generic dotted walk into the event object.
        let mut cur: &Value = target;
        for seg in &parts[1..] {
            cur = cur.get(*seg)?;
        }
        return Some(cur.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_observation() -> Value {
        serde_json::json!({
            "events": [
                {"category": "equipment", "event_type": "weapon_fired",
                 "event_id": "e1",
                 "payload": {"actor": 1, "loudness_radius": 480.0, "bloom_factor": 0.5}},
                {"category": "equipment", "event_type": "weapon_fired",
                 "event_id": "e2",
                 "payload": {"actor": 1, "loudness_radius": 480.0, "bloom_factor": 0.6}},
                {"category": "combat", "event_type": "projectile_hit",
                 "event_id": "e3",
                 "payload": {"shooter": 1, "target": 2, "zone": "torso"}},
                {"category": "combat", "event_type": "projectile_hit",
                 "event_id": "e4",
                 "payload": {"shooter": 1, "target": 2, "zone": "head"}},
                {"category": "actor", "event_type": "inventory_dropped",
                 "event_id": "e5",
                 "payload": {"actor": 2, "item_label": "rifle"}},
                {"category": "actor", "event_type": "inventory_settled",
                 "event_id": "e6",
                 "payload": {"loose_item_id": 0, "item_label": "rifle"}},
            ]
        })
    }

    #[test]
    fn events_dotted_count_filters_by_category_and_type() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.equipment.weapon_fired.count"),
            Some(Value::from(2u64))
        );
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.count"),
            Some(Value::from(2u64))
        );
        assert_eq!(
            lookup(&obs, "events.actor.inventory_settled.count"),
            Some(Value::from(1u64))
        );
        assert_eq!(lookup(&obs, "events.actor.count"), Some(Value::from(2u64)));
    }

    #[test]
    fn events_dotted_last_payload_returns_last_match_field() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.last.payload.zone"),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "events.equipment.weapon_fired.last.payload.bloom_factor"),
            Some(Value::from(0.6))
        );
        assert_eq!(
            lookup(&obs, "events.actor.inventory_settled.last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }

    #[test]
    fn events_where_count_with_payload_filter() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.where(category=combat,payload.zone=head).count"),
            Some(Value::from(1u64))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=combat,payload.zone=torso).count"),
            Some(Value::from(1u64))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=actor,event_type=inventory_settled).count"),
            Some(Value::from(1u64))
        );
    }

    #[test]
    fn events_where_last_payload_drill_down() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(
                &obs,
                "events.where(category=combat,event_type=projectile_hit).last.payload.zone"
            ),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=actor).last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }

    #[test]
    fn events_first_returns_first_match() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.first.payload.zone"),
            Some(Value::String("torso".into()))
        );
    }

    #[test]
    fn events_count_for_unknown_type_returns_zero() {
        let obs = fixture_observation();
        assert_eq!(lookup(&obs, "events.combat.nonexistent.count"), Some(Value::from(0u64)));
        assert_eq!(
            lookup(&obs, "events.where(category=unknown).count"),
            Some(Value::from(0u64))
        );
    }

    /// `ai.state_changed.count>=N` / `terrain.terrain_carved.count>=N`
    /// syntax resolves without an explicit `events.` prefix.
    #[test]
    fn bare_prefix_routes_to_event_stream_when_count_terminator() {
        let obs = fixture_observation();
        assert_eq!(lookup(&obs, "equipment.weapon_fired.count"), Some(Value::from(2u64)));
        assert_eq!(lookup(&obs, "combat.projectile_hit.count"), Some(Value::from(2u64)));
        assert_eq!(lookup(&obs, "actor.inventory_settled.count"), Some(Value::from(1u64)));
    }

    #[test]
    fn bare_prefix_preserves_actor_by_id_resolver() {
        let obs = serde_json::json!({
            "actors": [{"id": 7, "hp": 80}],
            "player_actor_id": 7,
            "events": [],
        });
        assert_eq!(lookup(&obs, "actor.7.hp"), Some(Value::from(80)));
        assert_eq!(lookup(&obs, "actor.player.hp"), Some(Value::from(80)));
    }

    /// lookup support". The actor-by-id resolver already supports nested chassis
    /// paths; this test pins the contract so a future refactor can't silently
    /// regress the dotted path drill-down used by cf-e2e scripts.
    #[test]
    fn actor_player_chassis_pilot_state_resolves_via_actor_resolver() {
        let obs = serde_json::json!({
            "actors": [{
                "id": 1,
                "chassis": {
                    "spec_id": "powered_armor_v1",
                    "stage": "eject",
                    "pilot_state": "ejecting",
                    "weapon_jammed": false
                }
            }],
            "player_actor_id": 1,
            "events": [],
        });
        assert_eq!(
            lookup(&obs, "actor.player.chassis.pilot_state"),
            Some(Value::String("ejecting".into()))
        );
        assert_eq!(
            lookup(&obs, "actor.1.chassis.pilot_state"),
            Some(Value::String("ejecting".into()))
        );
        assert_eq!(
            lookup(&obs, "actor.player.chassis.stage"),
            Some(Value::String("eject".into()))
        );
    }

    #[test]
    fn bare_prefix_preserves_mission_field_paths() {
        let obs = serde_json::json!({
            "mission": {"result": "won", "loss_reason": null},
            "events": [],
        });
        assert_eq!(lookup(&obs, "mission.result"), Some(Value::String("won".into())));
    }

    #[test]
    fn bare_prefix_last_payload_drill_down() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "combat.projectile_hit.last.payload.zone"),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "actor.inventory_settled.last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }
}
