use bevy::prelude::*;

use cf_actor::ActorObservation;

use crate::live_render_state::{ActorChassisZoneView, BreachRender};

/// `(zone_name, dx, dy, width, height)` describing a small rect anchored at the
/// actor's center. The layout assembles a 14-pip humanoid silhouette inside the
/// actor's ~16×32 sprite bounds (PoweredArmor scale); LightMech scales 2.25× via
/// the chassis kind multiplier in `chassis_scale_multiplier`. The Backpack pip
/// sits behind the torso at slightly negative z so it renders underneath.
pub(crate) const CHASSIS_ZONE_LAYOUT: &[(&str, f32, f32, f32, f32)] = &[
    // (zone, dx, dy, width, height)
    ("head", 0.0, 12.0, 6.0, 4.0),
    ("torso", 0.0, 4.0, 8.0, 10.0),
    ("arm_left", -6.0, 6.0, 3.0, 5.0),
    ("arm_right", 6.0, 6.0, 3.0, 5.0),
    ("forearm_left", -7.0, 1.0, 3.0, 4.0),
    ("forearm_right", 7.0, 1.0, 3.0, 4.0),
    ("hand_left", -7.0, -3.0, 3.0, 3.0),
    ("hand_right", 7.0, -3.0, 3.0, 3.0),
    ("leg_left", -2.0, -3.0, 3.0, 5.0),
    ("leg_right", 2.0, -3.0, 3.0, 5.0),
    ("shin_left", -2.0, -8.0, 3.0, 4.0),
    ("shin_right", 2.0, -8.0, 3.0, 4.0),
    ("foot_left", -2.0, -12.0, 4.0, 3.0),
    ("foot_right", 2.0, -12.0, 4.0, 3.0),
    ("backpack", 0.0, 5.0, 6.0, 8.0),
];

/// scaled by `ACTOR_SILHOUETTE_BASE_SCALE` so the silhouette is visible at
/// battlefield zoom); LightMech = 2.25× (matches the sim's `attach_chassis`
/// half_extents: LightMech 18×36 vs PoweredArmor 10×20). Infantry default
/// = 0.9× (slightly smaller than PoweredArmor).
pub(crate) fn chassis_scale_multiplier(kind: &str) -> f32 {
    match kind {
        "light_mech" => 2.25,
        "powered_armor" => 1.0,
        _ => 0.9,
    }
}

/// pip (and to chassis-less actors via `infantry_default_silhouette`). The
/// CHASSIS_ZONE_LAYOUT geometry is authored at ~16×28 px bounds (PoweredArmor
/// baseline); scaling by 2.0 produces a ~32×56 px on-screen silhouette that
/// reads clearly at 1280×720 capture resolution. Without this multiplier the
/// limbs are sub-pixel-thin and visual review tools (humans + AI agents
/// reading capture frames) can't verify destroyed-limb / stance / module
/// state at-a-glance — defeating the M5-DC-3 / M5-DC-4 visual closure goal.
pub const ACTOR_SILHOUETTE_BASE_SCALE: f32 = 2.0;

/// 15 pips together so the player can SEE crouch/climb/jet/eject states.
pub(crate) fn stance_offset(stance: &str) -> (f32, f32, f32) {
    match stance {
        "crouching" => (0.0, -4.0, 0.65),
        "climbing" => (3.0, 2.0, 1.0),
        "jetting" => (0.0, 6.0, 1.0),
        "ejecting" => (0.0, 8.0, 0.9),
        "downed" => (0.0, -8.0, 1.0),
        "dead" => (0.0, -10.0, 1.0),
        _ => (0.0, 0.0, 1.0),
    }
}

/// instantly visible: Nominal=untinted; Degraded=slight yellow; Disabled=orange;
/// Wreck=red; Gibbed=very-dark-red.
pub(crate) fn chassis_stage_tint(stage: &str) -> Color {
    match stage {
        "nominal" => Color::srgb(1.0, 1.0, 1.0),
        "degraded" | "module_warning" | "module_failed" | "weapon_jammed" => Color::srgb(1.0, 0.95, 0.55),
        "armor_cracked" | "disabled" | "pilot_injured" => Color::srgb(1.0, 0.65, 0.30),
        "eject" => Color::srgb(0.95, 0.40, 0.20),
        "bail_too_late" | "wreck" => Color::srgb(0.85, 0.20, 0.20),
        "gibbed" => Color::srgb(0.50, 0.10, 0.10),
        _ => Color::srgb(1.0, 1.0, 1.0),
    }
}

/// silhouette to read as a humanoid at-a-glance — head/helmet darker, chest
/// armor plate brighter (Cortex's ChestPlateA overlay pattern), arms/legs at
/// muted shade so they recede behind the torso, hands/feet at the brightest
/// tip of the limb so silhouette reads as a clear armored figure, backpack
/// distinct so jetpack is visible. Returns a per-zone (r_mul, g_mul, b_mul)
/// triple applied on top of the team base color.
fn zone_anatomical_tint(zone: &str) -> (f32, f32, f32) {
    match zone {
        // Head + helmet: darker shade so the silhouette has a recognizable
        // helmet contrast against the brighter torso.
        "head" => (0.55, 0.55, 0.65),
        // Torso = primary chest armor; brightest of the limbs.
        "torso" => (1.05, 1.05, 1.10),
        // Upper arms: thinner muted shade so they recede behind torso.
        "arm_left" | "arm_right" => (0.80, 0.80, 0.85),
        // Forearms: slightly brighter than upper arm so the elbow joint reads.
        "forearm_left" | "forearm_right" => (0.85, 0.85, 0.90),
        // Hands: bright tip; carries weapon visibility for the rifle-hand side.
        "hand_left" | "hand_right" => (0.95, 0.90, 0.75),
        // Thighs: same shade as upper arms.
        "leg_left" | "leg_right" => (0.80, 0.80, 0.85),
        // Shins: slightly brighter than thigh.
        "shin_left" | "shin_right" => (0.85, 0.85, 0.90),
        // Boots: dark contrast so footing is readable at battlefield zoom.
        "foot_left" | "foot_right" => (0.45, 0.45, 0.50),
        // Backpack/jetpack: distinct cool shade so the silhouette has a
        // visible pack behind the torso when the actor faces the camera.
        "backpack" => (0.40, 0.55, 0.75),
        _ => (1.0, 1.0, 1.0),
    }
}

/// Internal → Core → wound) through color. Corefall surfaces simulation
/// depth Cortex/Soldat/Noita don't have: every zone has 3 stacked armor
/// layers + a wound container, and the silhouette must show which deepest
/// layer is still intact at a glance.
///
/// Color progression as armor degrades:
///   External fully intact (>=0.66 hp)   → bright team color + anatomical tint
///   External breached, Internal intact  → mid-shade (60% brightness, cooler)
///   Internal breached, Core intact      → dark + warning tint
///   Core breached, wound bleeding       → red wound color
///   Wound drained → zone destroyed      → transparent (visible limb-loss gap)
///
/// Stage tint multiplies on top so a Disabled chassis renders orange-tinted
/// even if individual zones are healthy.
pub(crate) fn zone_color(base: Color, zone: &ActorChassisZoneView, stage_tint: Color) -> Color {
    if zone.destroyed {
        // Destroyed zone: transparent void where the limb used to be.
        // Matches M5-DC-4 ("limb damage has visible/mechanical consequences:
        // limp, crawl, one-arm handling, dropped gear, disabled grip").
        return Color::srgba(0.0, 0.0, 0.0, 0.0);
    }
    let (br, bg, bb, _) = base.to_srgba().to_f32_array().into();
    let (tr, tg, tb, _) = stage_tint.to_srgba().to_f32_array().into();
    let (ar, ag, ab) = zone_anatomical_tint(zone.zone.as_str());

    // Find the deepest-intact layer; its hp% drives the visible brightness.
    // Below 0.05 we treat the layer as breached and progress to the next.
    let (layer_bright, layer_warm_shift) = if zone.external_integrity > 0.05 {
        // External layer intact: full brightness, no warm shift.
        (zone.external_integrity.clamp(0.0, 1.0).max(0.40), 0.0)
    } else if zone.internal_integrity > 0.05 {
        // External breached, Internal intact: ~60% brightness + slight warm.
        (zone.internal_integrity.clamp(0.0, 1.0).max(0.30) * 0.65, 0.15)
    } else if zone.core_integrity > 0.05 {
        // Internal breached, Core intact: ~40% brightness + strong warm.
        (zone.core_integrity.clamp(0.0, 1.0).max(0.25) * 0.45, 0.35)
    } else {
        // Core breached, wound bleeding: red wound color regardless of base.
        let wound = zone.wound_integrity.clamp(0.0, 1.0);
        return Color::srgb(0.85 * wound.max(0.30), 0.10, 0.10);
    };

    // Apply warm shift: bias red up, blue down as armor degrades.
    let warm_r = 1.0 + layer_warm_shift;
    let warm_b = 1.0 - layer_warm_shift * 0.8;
    Color::srgb(
        (br * tr * ar * layer_bright * warm_r).clamp(0.0, 1.0),
        (bg * tg * ag * layer_bright).clamp(0.0, 1.0),
        (bb * tb * ab * layer_bright * warm_b).clamp(0.0, 1.0),
    )
}

/// overlay sprites on the chassis at their bound zone, so a player watching
/// a wreck_eject scenario can SEE the jet module turn red when the backpack
/// zone takes damage. This is a Corefall-only feature — Cortex actors don't
/// surface module state at all.
pub(crate) fn module_pip_color(state: &str) -> Option<Color> {
    match state {
        "nominal" => Some(Color::srgb(0.30, 0.95, 0.40)),
        "degraded" => Some(Color::srgb(0.95, 0.85, 0.30)),
        "warning" => Some(Color::srgb(0.95, 0.60, 0.20)),
        "failed" => Some(Color::srgb(0.85, 0.20, 0.20)),
        _ => None, // NotPresent or unknown — don't render
    }
}

/// the zone's center where the module pip should render. Multiple modules
/// can be bound to the same zone (e.g., shield + sensor on torso); the
/// caller cycles through positions.
pub(crate) fn module_pip_offset(kind: &str) -> (f32, f32, f32) {
    // (dx, dy, size). Position relative to the bound zone's center.
    match kind {
        "jet" => (0.0, 2.0, 3.0),           // jet pip slightly above backpack center
        "shield" => (0.0, 0.0, 2.0),        // shield emitter in front of torso
        "sensor" => (0.0, 2.0, 2.0),        // sensor antenna above head
        "weapon_mount" => (2.0, 0.0, 2.5),  // weapon mount on hand side
        "repair_drone" => (0.0, -2.0, 2.5), // repair drone below backpack
        _ => (0.0, 0.0, 2.0),
    }
}

/// horizontal velocity sign so the legs visibly alternate during locomotion.
/// Returns (left_leg_dy, right_leg_dy) — opposite phase between legs.
pub(crate) fn walk_cycle_offsets(tick: u64, stance: &str, velocity_x: f32) -> (f32, f32) {
    // Only animate during locomotion stances; static stances hold pose.
    let moving = matches!(stance, "walking" | "running") && velocity_x.abs() > 1.0;
    if !moving {
        return (0.0, 0.0);
    }
    // ~8-tick walk cycle (4 ticks per step at 60Hz = ~133ms cadence; ~6Hz
    // step rate, matches infantry walk feel without looking jittery).
    let phase = (tick % 8) as f32;
    let amplitude = if stance == "running" { 2.5 } else { 1.5 };
    let cycle = ((phase / 8.0) * std::f32::consts::TAU).sin();
    (cycle * amplitude, -cycle * amplitude)
}

/// pixel-art sprite frames under `content/sprites/actor_m1/` are reserved
/// for the asset loader at BP4+; M1 ships the visible-silhouette stance
/// swap so the player no longer renders as a transparent ghost rectangle.
pub(crate) fn stance_tint_for(stance: &str, status: &str) -> Color {
    if status == "dead" {
        return Color::srgb(0.05, 0.05, 0.05);
    }
    if status == "dying" {
        return Color::srgb(0.35, 0.05, 0.05);
    }
    if status == "downed" {
        return Color::srgb(0.30, 0.10, 0.10);
    }
    if status == "inactive" {
        return Color::srgb(0.35, 0.35, 0.40);
    }
    match stance {
        "idle" => Color::srgb(0.55, 0.58, 0.65),
        "walking" => Color::srgb(0.50, 0.65, 0.78),
        "running" => Color::srgb(0.35, 0.65, 0.85),
        "airborne" => Color::srgb(0.80, 0.75, 0.30),
        "knocked_down" => Color::srgb(0.80, 0.25, 0.25),
        "crouching" => Color::srgb(0.45, 0.55, 0.50),
        "climbing" => Color::srgb(0.55, 0.45, 0.65),
        "jetting" => Color::srgb(0.90, 0.55, 0.20),
        "ejecting" => Color::srgb(0.95, 0.90, 0.25),
        _ => Color::srgb(0.50, 0.55, 0.60),
    }
}

pub(crate) fn breach_color(breach: &BreachRender) -> Color {
    if breach.broken {
        Color::srgba(0.40, 0.30, 0.20, 0.25)
    } else if breach.refusal_reason.is_some() {
        Color::srgb(0.55, 0.55, 0.60)
    } else {
        let pct = if breach.max_hp > 0.0 {
            (breach.hp / breach.max_hp).clamp(0.0, 1.0)
        } else {
            1.0
        };
        // Solid concrete tone darkens as the strip is dug down.
        let v = 0.25 + 0.40 * pct;
        Color::srgb(v, v * 0.85, v * 0.70)
    }
}

pub(crate) fn actor_color(actor: &ActorObservation) -> Color {
    let base = match actor.team.as_str() {
        "blue" => Color::srgb(0.30, 0.55, 0.95),
        "red" => Color::srgb(0.85, 0.30, 0.30),
        _ => Color::srgb(0.70, 0.70, 0.70),
    };
    if actor.status == "dead" {
        Color::srgb(0.20, 0.20, 0.20)
    } else if actor.status == "downed" {
        let s = base.to_srgba();
        Color::srgb(s.red * 0.5, s.green * 0.5, s.blue * 0.5)
    } else {
        base
    }
}
