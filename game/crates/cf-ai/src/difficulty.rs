use serde::{Deserialize, Serialize};

use crate::ReactiveGuardParams;

/// Engines load the registry once at boot and apply a preset to each
/// reactive guard via `apply_to(params)`. Fields are public so cf-mod
/// validation can introspect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyPreset {
    pub id: String,
    pub display_name: String,
    pub hp: f32,
    pub aim_settle_ticks: u32,
    pub miss_chance: f32,
    pub sight_range: f32,
    pub sight_fov_degrees: f32,
    pub hearing_radius: f32,
    pub memory_decay_ticks: u32,
    pub reload_ms: u32,
    pub retreat_hp_pct: f32,
}

impl DifficultyPreset {
    /// Apply this preset to the params struct in place. Fields not
    /// represented in the preset stay at their current value (e.g.
    /// burst_pause_seconds is a tuning detail not surfaced to the player).
    pub fn apply_to(&self, params: &mut ReactiveGuardParams, tick_rate_hz: u32) {
        params.miss_chance = self.miss_chance;
        params.sight_radius = self.sight_range;
        params.sight_cone_degrees = self.sight_fov_degrees;
        params.hearing_radius = self.hearing_radius;
        params.memory_decay_ticks = self.memory_decay_ticks;
        params.retreat_hp_pct = self.retreat_hp_pct;
        params.recover_hp_pct = (self.retreat_hp_pct + 0.05).min(1.0);
        params.aim_settle_seconds = if tick_rate_hz > 0 {
            self.aim_settle_ticks as f32 / tick_rate_hz as f32
        } else {
            self.aim_settle_ticks as f32 / 60.0
        };
        params.reload_seconds = self.reload_ms as f32 / 1000.0;
    }

    /// Built-in preset by id (mirrors the three entries in
    /// `content/ai/difficulty.json`). Returns None for unknown ids.
    /// Used as a fallback when the registry file is missing / not loaded.
    pub fn builtin(id: &str) -> Option<DifficultyPreset> {
        Some(match id {
            "cakewalk" => DifficultyPreset {
                id: "cakewalk".into(),
                display_name: "Cakewalk".into(),
                hp: 60.0,
                aim_settle_ticks: 24,
                miss_chance: 0.3,
                sight_range: 240.0,
                sight_fov_degrees: 90.0,
                hearing_radius: 320.0,
                memory_decay_ticks: 180,
                reload_ms: 2400,
                retreat_hp_pct: 0.5,
            },
            "tough_crowd" => DifficultyPreset {
                id: "tough_crowd".into(),
                display_name: "Tough Crowd".into(),
                hp: 80.0,
                aim_settle_ticks: 12,
                miss_chance: 0.1,
                sight_range: 320.0,
                sight_fov_degrees: 120.0,
                hearing_radius: 480.0,
                memory_decay_ticks: 300,
                reload_ms: 1800,
                retreat_hp_pct: 0.3,
            },
            "veteran" => DifficultyPreset {
                id: "veteran".into(),
                display_name: "Veteran".into(),
                hp: 120.0,
                aim_settle_ticks: 6,
                miss_chance: 0.05,
                sight_range: 480.0,
                sight_fov_degrees: 140.0,
                hearing_radius: 600.0,
                memory_decay_ticks: 600,
                reload_ms: 1200,
                retreat_hp_pct: 0.2,
            },
            // M17 — Hardcore: real-physics TTD (no compound floor) + permadeath.
            // The player had better know what they're doing.
            "hardcore" => DifficultyPreset {
                id: "hardcore".into(),
                display_name: "Hardcore".into(),
                hp: 150.0,
                aim_settle_ticks: 4,
                miss_chance: 0.02,
                sight_range: 560.0,
                sight_fov_degrees: 160.0,
                hearing_radius: 720.0,
                memory_decay_ticks: 900,
                reload_ms: 1000,
                retreat_hp_pct: 0.15,
            },
            _ => return None,
        })
    }
}
