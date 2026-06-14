use bevy::prelude::*;

use cf_actor::ActorObservation;

use crate::hud_model::{
    HudBanner, HudBodySilhouette, HudBreach, HudEnemy, HudMission, HudModuleStrip, HudRifle, HudSettings,
    HudToolValidity,
};
use crate::palette::palette_text;

/// Format the banner line. Severity and an icon glyph are rendered alongside
/// the label so the HUD never communicates state with color alone.
pub fn banner_line(banner: &HudBanner) -> String {
    let icon = match banner.severity.as_str() {
        "critical" => "[!!]",
        "warning" => "[!]",
        _ => "[*]",
    };
    format!(
        "{icon} {sev} {label}",
        icon = icon,
        sev = banner.severity.to_uppercase(),
        label = banner.label
    )
}

/// Format the stance HUD line. The stance label IS the readable signal — color
/// is not used as the only cue.
pub fn stance_line(stance: &str, player: Option<&ActorObservation>) -> String {
    if stance.is_empty() {
        return "STANCE: --".to_string();
    }
    let air_marker = match player {
        Some(p) if !p.on_ground => " (airborne)",
        _ => "",
    };
    let stability_tag = match player {
        Some(p) if p.knockdown_ticks_remaining > 0 => {
            let pct = (p.stability * 100.0).round() as i32;
            format!(" | STABILITY {pct}% KNOCKED_DOWN")
        }
        Some(p) if p.stability < 0.9 => {
            let pct = (p.stability * 100.0).round() as i32;
            let label = if pct >= 60 {
                "SHAKEN"
            } else if pct >= 30 {
                "UNSTABLE"
            } else if pct > 0 {
                "CRITICAL"
            } else {
                "DISRUPTED"
            };
            format!(" | STABILITY {pct}% {label}")
        }
        _ => String::new(),
    };
    format!("STANCE: {}{}{}", stance.to_uppercase(), air_marker, stability_tag)
}

/// Format the stability HUD line.
pub fn stability_line(stability: f32) -> String {
    stability_line_with_knockdown(stability, false)
}

/// Knockdown-aware variant of `stability_line`.
pub fn stability_line_with_knockdown(stability: f32, knocked_down: bool) -> String {
    let pct = (stability * 100.0).round() as i32;
    let label = if knocked_down {
        "KNOCKED_DOWN"
    } else if pct >= 90 {
        "SOLID"
    } else if pct >= 60 {
        "SHAKEN"
    } else if pct >= 30 {
        "UNSTABLE"
    } else if pct > 0 {
        "CRITICAL"
    } else {
        "DISRUPTED"
    };
    format!("STABILITY: {pct}% {label}")
}

/// Format the silhouette HUD line. Renders six per-zone bars as ASCII so the
/// readability does not depend on color.
pub fn silhouette_line(body: &HudBodySilhouette) -> String {
    let placeholder_marker = if body.placeholder { "~" } else { "" };
    format!(
        "BODY{ph}: H{h:>3} T{t:>3} A{al:>3}/{ar:>3} L{ll:>3}/{lr:>3}",
        ph = placeholder_marker,
        h = (body.head_hp_pct * 100.0).round() as i32,
        t = (body.torso_hp_pct * 100.0).round() as i32,
        al = (body.arm_left_hp_pct * 100.0).round() as i32,
        ar = (body.arm_right_hp_pct * 100.0).round() as i32,
        ll = (body.leg_left_hp_pct * 100.0).round() as i32,
        lr = (body.leg_right_hp_pct * 100.0).round() as i32,
    )
}

/// Format the module strip HUD line. Color-independent: each module's state
/// label is text (`nominal` / `degraded` / `warning` / `failed` / `not_present`).
pub fn module_line(modules: &HudModuleStrip) -> String {
    if modules.modules.is_empty() {
        return "MODS: --".to_string();
    }
    let placeholder_marker = if modules.placeholder { "~" } else { "" };
    let mut s = format!("MODS{}:", placeholder_marker);
    for m in &modules.modules {
        s.push(' ');
        if m.state == "not_present" {
            s.push_str(&format!("{}:N/A", compact_module_name(&m.kind)));
        } else if modules.placeholder {
            s.push_str(&m.label.replace('—', "-"));
        } else {
            let state_tag = match m.state.as_str() {
                "nominal" => "OK",
                "degraded" => "DEG",
                "warning" => "WARN",
                "failed" => "FAIL",
                other => other,
            };
            s.push_str(&format!("{}:{}", compact_module_name(&m.kind), state_tag));
        }
    }
    s
}

fn compact_module_name(kind: &str) -> &'static str {
    match kind {
        "weapon_mount" => "WEAPON",
        "jet" => "JET",
        "shield" => "SHIELD",
        "sensor" => "SENSOR",
        "repair_drone" => "REPAIR",
        _ => "MOD",
    }
}

/// Format the tool-validity HUD line.
pub fn tool_line(validity: Option<&HudToolValidity>) -> String {
    let Some(v) = validity else {
        return "TOOL: --".to_string();
    };
    if v.valid {
        match v.last_carve_tick {
            Some(t) => format!("TOOL: VALID (last carve @ {t}t)"),
            None => "TOOL: VALID".to_string(),
        }
    } else {
        let reason = v.last_refusal_reason.as_deref().unwrap_or("unknown");
        match v.last_refusal_target.as_deref() {
            Some(target) => format!("TOOL: REFUSED | {reason} ({target})"),
            None => format!("TOOL: REFUSED | {reason}"),
        }
    }
}

/// Green for >30s remaining; yellow for 10..=30s; red for <10s. Inactive
/// mission OR no time limit returns the default base-palette color.
pub fn mission_timer_color(mission: Option<&HudMission>, tick_rate_hz: u32, high_contrast: bool) -> Color {
    let Some(m) = mission else {
        return palette_text(high_contrast);
    };
    let in_progress = matches!(m.result.as_str(), "in_progress" | "active");
    if !in_progress || m.time_limit_ticks == 0 {
        return palette_text(high_contrast);
    }
    let rate = tick_rate_hz.max(1) as f32;
    let remaining_s = ((m.time_limit_ticks.saturating_sub(m.elapsed_ticks)) as f32 / rate).max(0.0);
    if remaining_s < 10.0 {
        Color::srgb(1.0, 0.25, 0.25)
    } else if remaining_s < 30.0 {
        Color::srgb(1.0, 0.85, 0.2)
    } else {
        Color::srgb(0.4, 1.0, 0.4)
    }
}

/// Format the mission HUD line. Public for unit tests.
pub fn mission_line(mission: Option<&HudMission>, tick_rate_hz: u32) -> String {
    let Some(m) = mission else {
        return "MISSION: --".to_string();
    };
    let rate = tick_rate_hz.max(1) as f32;
    let elapsed_s = m.elapsed_ticks as f32 / rate;
    let in_progress = matches!(m.result.as_str(), "in_progress" | "active");
    let total = if m.time_limit_ticks > 0 {
        if in_progress {
            let remaining_s = (m.time_limit_ticks.saturating_sub(m.elapsed_ticks) as f32 / rate).max(0.0);
            let minutes = (remaining_s as u32) / 60;
            let seconds = (remaining_s as u32) % 60;
            format!(" / {minutes:02}:{seconds:02}")
        } else {
            format!(" / {:.0}s", m.time_limit_ticks as f32 / rate)
        }
    } else {
        String::new()
    };
    let label = match m.result.as_str() {
        "won" => "WON".to_string(),
        "lost" => format!("LOST ({})", m.loss_reason.clone().unwrap_or_else(|| "?".into())),
        _ => "ACTIVE".to_string(),
    };
    format!("MISSION: {label} {elapsed_s:>4.1}s{total}")
}

/// Format the objective line. Public for unit tests.
pub fn objective_line(mission: Option<&HudMission>) -> String {
    let Some(m) = mission else {
        return "OBJECTIVE: --".to_string();
    };
    match &m.active_objective {
        Some(id) => format!("OBJECTIVE: {id}"),
        None => "OBJECTIVE: (none active)".to_string(),
    }
}

/// Format the enemy summary line.
pub fn enemy_line(enemy: Option<&HudEnemy>) -> String {
    let Some(e) = enemy else {
        return "ENEMY: --".to_string();
    };
    format!(
        "ENEMY: {} hp={}/{}, {} ({})",
        e.status.to_uppercase(),
        e.hp as i32,
        e.hp_max as i32,
        e.state.to_uppercase(),
        e.last_tactic
    )
}

/// AI-debug enemy intent label. Returns `None` when the overlay is disabled OR
/// no enemy is available so cf-app can despawn the text node.
pub fn ai_debug_label(enemy: Option<&HudEnemy>, settings: &HudSettings) -> Option<String> {
    if !settings.ai_debug {
        return None;
    }
    let e = enemy?;
    if e.intent_label.is_empty() {
        return None;
    }
    Some(e.intent_label.clone())
}

/// Replay-CTA event id for the mission-resolved modal. Returns `None` when the
/// CTA should be hidden.
pub fn show_replay_cta_event_id(mission: Option<&HudMission>) -> Option<String> {
    let m = mission?;
    if !m.show_replay_cta {
        return None;
    }
    m.show_me_why_event_id.clone()
}

/// Format the breach summary line.
pub fn breach_line(breach: Option<&HudBreach>) -> String {
    let Some(b) = breach else {
        return "BREACH: --".to_string();
    };
    if b.broken {
        return format!("BREACH: {} BROKEN", b.id);
    }
    if let Some(reason) = &b.refusal_reason {
        return format!("BREACH: {} REFUSED ({})", b.id, reason);
    }
    let pct = if b.max_hp > 0.0 { (b.hp / b.max_hp) * 100.0 } else { 0.0 };
    let range_label = if b.in_range { "" } else { " (out of range)" };
    format!(
        "BREACH: {} {}/{} ({:>3.0}%){range_label}",
        b.id, b.hp as i32, b.max_hp as i32, pct
    )
}

/// Build the rifle status line shown in the HUD strip.
///
/// Format: `READY 30/30`, `RELOADING NN% (X/Y)`, `EMPTY (X/Y)`, `COOLDOWN Nt (X/Y)`, or
/// `NO RIFLE` when no rifle is selected.
pub fn rifle_status_line(rifle: Option<&HudRifle>) -> String {
    let Some(rifle) = rifle else {
        return "NO RIFLE".to_string();
    };
    if rifle.reload_remaining_ticks > 0 {
        let total = rifle.reload_total_ticks.max(1) as f32;
        let progress = (1.0 - (rifle.reload_remaining_ticks as f32 / total)) * 100.0;
        return format!("RELOADING {progress:>3.0}% ({}/{})", rifle.ammo, rifle.capacity);
    }
    if rifle.capacity > 0 && rifle.ammo == 0 {
        return format!("EMPTY ({}/{})", rifle.ammo, rifle.capacity);
    }
    if rifle.fire_cooldown_ticks > 0 {
        return format!(
            "COOLDOWN {}t ({}/{})",
            rifle.fire_cooldown_ticks, rifle.ammo, rifle.capacity
        );
    }
    format!("READY {}/{}", rifle.ammo, rifle.capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud_model::{HudModule, HudModuleStrip};

    fn rifle(ammo: u32, capacity: u32, cooldown: u32, remaining: u32, total: u32) -> HudRifle {
        HudRifle {
            ammo,
            capacity,
            fire_cooldown_ticks: cooldown,
            reload_remaining_ticks: remaining,
            reload_total_ticks: total,
        }
    }

    #[test]
    fn rifle_status_line_formats_ready() {
        let s = rifle_status_line(Some(&rifle(30, 30, 0, 0, 90)));
        assert_eq!(s, "READY 30/30");
    }

    #[test]
    fn rifle_status_line_formats_reload() {
        let s = rifle_status_line(Some(&rifle(0, 30, 0, 45, 90)));
        assert!(s.starts_with("RELOADING  50%"), "got `{s}`");
    }

    #[test]
    fn rifle_status_line_formats_empty() {
        let s = rifle_status_line(Some(&rifle(0, 30, 0, 0, 90)));
        assert_eq!(s, "EMPTY (0/30)");
    }

    #[test]
    fn rifle_status_line_formats_cooldown() {
        let s = rifle_status_line(Some(&rifle(15, 30, 5, 0, 90)));
        assert_eq!(s, "COOLDOWN 5t (15/30)");
    }

    #[test]
    fn rifle_status_line_formats_no_rifle() {
        let s = rifle_status_line(None);
        assert_eq!(s, "NO RIFLE");
    }

    #[test]
    fn mission_line_formats_active_with_timer() {
        let m = HudMission {
            result: "active".to_string(),
            loss_reason: None,
            elapsed_ticks: 60,
            time_limit_ticks: 5400,
            ticks_remaining: Some(5340),
            active_objective: Some("breach".to_string()),
            last_event_label: "mission_started".to_string(),
            show_me_why_event_id: None,
            show_replay_cta: false,
        };
        assert_eq!(mission_line(Some(&m), 60), "MISSION: ACTIVE  1.0s / 01:29");
    }

    #[test]
    fn mission_line_formats_won_and_lost() {
        let won = HudMission {
            result: "won".to_string(),
            ..HudMission::default()
        };
        assert!(mission_line(Some(&won), 60).starts_with("MISSION: WON"));
        let lost = HudMission {
            result: "lost".to_string(),
            loss_reason: Some("player_dead".to_string()),
            ..HudMission::default()
        };
        assert!(mission_line(Some(&lost), 60).starts_with("MISSION: LOST (player_dead)"));
    }

    #[test]
    fn breach_line_formats_progress_and_broken_states() {
        let progress = HudBreach {
            id: "outer_wall".to_string(),
            material: "concrete_soft".to_string(),
            hp: 30.0,
            max_hp: 60.0,
            broken: false,
            refusal_reason: None,
            in_range: true,
        };
        assert!(breach_line(Some(&progress)).contains("50%"));
        let broken = HudBreach {
            broken: true,
            id: "outer_wall".to_string(),
            ..HudBreach::default()
        };
        assert_eq!(breach_line(Some(&broken)), "BREACH: outer_wall BROKEN");
        let metal = HudBreach {
            id: "anchor".to_string(),
            refusal_reason: Some("metal_nohook".to_string()),
            ..HudBreach::default()
        };
        assert_eq!(breach_line(Some(&metal)), "BREACH: anchor REFUSED (metal_nohook)");
    }

    #[test]
    fn objective_line_handles_no_mission() {
        assert_eq!(objective_line(None), "OBJECTIVE: --");
        let m = HudMission {
            active_objective: Some("extract".to_string()),
            ..HudMission::default()
        };
        assert_eq!(objective_line(Some(&m)), "OBJECTIVE: extract");
    }

    #[test]
    fn stance_line_uppercases_and_appends_airborne_marker() {
        assert_eq!(stance_line("idle", None), "STANCE: IDLE");
        let player = ActorObservation {
            id: 1,
            team: "blue".into(),
            controllable: true,
            position: [0.0, 10.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            on_ground: false,
            status: "stable".into(),
            hp: 100.0,
            hp_max: 100.0,
            selected_slot: 0,
            selected_item: "rifle".into(),
            inventory: vec!["rifle".into(), "empty".into(), "empty".into(), "empty".into()],
            stance: "airborne".into(),
            body_silhouette: cf_actor::BodySilhouette::default(),
            chassis: None,
            origin_id: "human".into(),
            m17: cf_actor::M17ResourceView::default(),
            stability: 1.0,
            stability_recovery_rate: 0.02,
            mass_kg: 80.0,
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            sharp_aim_progress: 0.0,
            recoil_accumulator: 0.0,
            knockdown_ticks_remaining: 0,
            dying_dwell_ticks_remaining: 0,
            mission_critical: false,
            bloom_factor: 1.0,
            facing: "right".into(),
            stamina: 1.0,
            stamina_max: 1.0,
            sprint_active: false,
            prone_active: false,
            lean_angle_degrees: 0.0,
            lean_direction: "none".into(),
            stealth_meter: 0.0,
            spotted: false,
            cover_side: "none".into(),
            cover_effectiveness: 0.0,
            inventory_weight_kg: 0.0,
            weight_forces_walk: false,
            limb_loss: cf_actor::LimbLossFlags::default(),
            inventory_extended: Vec::new(),
            weapon_state: cf_actor::WeaponStateView::default(),
            is_brain: false,
            hit_reaction_kind: String::new(),
            hit_reaction_ticks_remaining: 0,
            drone_mode: None,
            drone_fuel: None,
            max_carry_kg: cf_equipment::HUMAN_BASELINE_MAX_CARRY_KG,
            max_carry_volume_l: cf_equipment::HUMAN_BASELINE_MAX_CARRY_VOLUME_L,
            total_carried_kg: 0.0,
            total_carried_volume_l: 0.0,
            encumbrance_walk_speed_multiplier: 1.0,
            encumbrance_band: "none".to_string(),
            encumbered: false,
            inventory_grid: None,
        };
        let line = stance_line("airborne", Some(&player));
        assert!(line.contains("AIRBORNE"));
        assert!(line.contains("(airborne)"));
    }

    #[test]
    fn silhouette_line_renders_per_zone_pct_with_placeholder_marker() {
        let body = HudBodySilhouette {
            head_hp_pct: 0.6,
            torso_hp_pct: 0.6,
            arm_left_hp_pct: 0.6,
            arm_right_hp_pct: 0.6,
            leg_left_hp_pct: 0.6,
            leg_right_hp_pct: 0.6,
            placeholder: true,
        };
        let line = silhouette_line(&body);
        assert!(line.starts_with("BODY~:"));
        assert!(line.contains("H 60"));
        assert!(line.contains("T 60"));
        assert!(line.contains("A 60/ 60"));
        assert!(line.contains("L 60/ 60"));
    }

    #[test]
    fn module_line_aggregates_module_labels_with_placeholder_marker() {
        let mods = HudModuleStrip {
            modules: vec![HudModule {
                id: "weapon_mount".into(),
                label: "READY 30/30".into(),
                state: "nominal".into(),
                kind: "weapon_mount".into(),
            }],
            placeholder: true,
        };
        let s = module_line(&mods);
        assert!(s.starts_with("MODS~:"));
        assert!(s.contains("READY 30/30"));
        assert!(s.is_ascii());
    }

    #[test]
    fn tool_line_handles_valid_and_refused_states() {
        let valid = HudToolValidity {
            valid: true,
            last_carve_tick: Some(120),
            ..HudToolValidity::default()
        };
        assert_eq!(tool_line(Some(&valid)), "TOOL: VALID (last carve @ 120t)");
        let refused = HudToolValidity {
            valid: false,
            last_refusal_reason: Some("material_metal_nohook".into()),
            last_refusal_target: Some("anchor_post".into()),
            ..HudToolValidity::default()
        };
        let s = tool_line(Some(&refused));
        assert!(s.contains("REFUSED"));
        assert!(s.contains("material_metal_nohook"));
        assert!(s.contains("anchor_post"));
        assert_eq!(tool_line(None), "TOOL: --");
    }

    #[test]
    fn banner_line_includes_severity_word_and_icon() {
        let critical = HudBanner {
            id: "eject_now".into(),
            severity: "critical".into(),
            label: "EJECT NOW".into(),
            raised_at_tick: 90,
        };
        let s = banner_line(&critical);
        assert!(s.contains("[!!]"));
        assert!(s.contains("CRITICAL"));
        assert!(s.contains("EJECT NOW"));

        let warning = HudBanner {
            id: "ammo_out".into(),
            severity: "warning".into(),
            label: "AMMO OUT".into(),
            raised_at_tick: 200,
        };
        let s = banner_line(&warning);
        assert!(s.contains("[!]"));
        assert!(s.contains("WARNING"));
    }

    #[test]
    fn enemy_line_summarises_state() {
        let e = HudEnemy {
            state: "engaged".to_string(),
            last_tactic: "attack_target".to_string(),
            hp: 50.0,
            hp_max: 80.0,
            status: "stable".to_string(),
            intent_label: String::new(),
            world_position: None,
        };
        let s = enemy_line(Some(&e));
        assert!(s.contains("ENGAGED"));
        assert!(s.contains("attack_target"));
        assert!(s.contains("hp=50/80"));
    }

    #[test]
    fn ai_debug_label_hidden_when_flag_off() {
        let enemy = HudEnemy {
            intent_label: "ENGAGED: ATTACK".to_string(),
            ..Default::default()
        };
        let settings = HudSettings {
            ai_debug: false,
            ..Default::default()
        };
        assert_eq!(ai_debug_label(Some(&enemy), &settings), None);
    }

    #[test]
    fn ai_debug_label_renders_when_flag_on() {
        let enemy = HudEnemy {
            intent_label: "ALERT: SEARCH".to_string(),
            ..Default::default()
        };
        let settings = HudSettings {
            ai_debug: true,
            ..Default::default()
        };
        assert_eq!(
            ai_debug_label(Some(&enemy), &settings),
            Some("ALERT: SEARCH".to_string())
        );
    }

    #[test]
    fn ai_debug_label_hidden_when_no_enemy() {
        let settings = HudSettings {
            ai_debug: true,
            ..Default::default()
        };
        assert_eq!(ai_debug_label(None, &settings), None);
    }

    #[test]
    fn show_replay_cta_hidden_for_won_mission() {
        let mission = HudMission {
            result: "won".to_string(),
            show_replay_cta: false,
            show_me_why_event_id: None,
            ..Default::default()
        };
        assert_eq!(show_replay_cta_event_id(Some(&mission)), None);
    }

    #[test]
    fn show_replay_cta_returns_event_id_for_lost_mission() {
        let mission = HudMission {
            result: "lost".to_string(),
            loss_reason: Some("player_dead".to_string()),
            show_replay_cta: true,
            show_me_why_event_id: Some("event:704:3354".to_string()),
            ..Default::default()
        };
        assert_eq!(
            show_replay_cta_event_id(Some(&mission)),
            Some("event:704:3354".to_string())
        );
    }

    #[test]
    fn show_replay_cta_hidden_when_no_mission() {
        assert_eq!(show_replay_cta_event_id(None), None);
    }
}
