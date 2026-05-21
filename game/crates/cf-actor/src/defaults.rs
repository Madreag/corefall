//! Shared `serde(default = ...)` helper functions for [`crate::ActorState`]
//! and [`crate::ActorObservation`].

pub(crate) fn default_swim_breath_seconds() -> f32 {
    30.0
}

pub(crate) fn default_swim_drain_multiplier() -> f32 {
    1.0
}

pub(crate) fn default_mass_dirty() -> bool {
    true
}

pub(crate) fn default_bipod_equipped() -> cf_equipment::Bipod {
    cf_equipment::Bipod::equipped_default()
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn default_grenade_kind() -> Option<cf_equipment::GrenadeKind> {
    Some(cf_equipment::GrenadeKind::Frag)
}

pub(crate) fn default_origin_id() -> String {
    "human".to_string()
}

pub(crate) fn default_stability() -> f32 {
    1.0
}

pub(crate) fn default_stability_recovery_rate() -> f32 {
    0.02
}

pub(crate) fn default_speed_factor() -> f32 {
    1.0
}

pub(crate) fn default_mass_kg() -> f32 {
    80.0
}

pub(crate) fn default_dying_dwell_ticks() -> u32 {
    60
}

pub(crate) fn default_sharp_aim_build_ticks() -> u32 {
    30
}

pub(crate) fn default_recoil_decay_rate() -> f32 {
    0.05
}

pub(crate) fn default_walk_threshold() -> f32 {
    1.5
}

pub(crate) fn default_bloom_factor() -> f32 {
    1.0
}
