//! M9C: static-fortification kernel.
//!
//! `cf-fortification` owns the authored-defensive-structure surfaces
//! enumerated in `specs/active/M9C.md`:
//!
//! - [`mg_nest`] — MG nest crewing logic, ammo-box auto-feed, tripod
//!   deploy state machine.
//! - [`watchtower`] — Watchtower tier kernel, spotter role, spotlight
//!   cone, observation_post, radio_repeater.
//! - [`minefield`] — Mine instance state machine, trigger evaluation
//!   (proximity / pressure / tripwire / IED chain), detection masking.
//! - [`wire`] — Wire kernel: per-actor crossing state, cut-with-tool,
//!   electrified-power coupling.
//! - [`anti_tank`] — Anti-tank ditch carve, dragon's teeth + tank trap
//!   collision + per-vehicle damage routing.
//! - [`camo`] — Camo netting concealment overlay + bypass-rule
//!   consumers (thermal, spotlight, proximity, motion-while-firing).
//! - [`sandbag`] — 3-tier sandbag-wall kernel + per-pixel erosion
//!   (top row first) + tier-transition event emission.
//!
//! Downstream crates (`cf-actor`, `cf-control`, `cf-ai`, `cf-render-2d`,
//! `cf-ui`, …) consume these seven modules.
//!
//! M9C-1 ships the **scaffold + sandbag + camo** kernels (per the
//! `m9c-1-fortification-core-sandbag-camo` feature definition).
//! The five remaining kernels (mg_nest / watchtower / minefield / wire
//! / anti_tank) ship as full surfaces in features m9c-2..m9c-5.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::derivable_impls,
    clippy::struct_excessive_bools,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::default_trait_access,
    clippy::if_not_else,
    clippy::similar_names,
    clippy::single_match_else
)]

pub mod anti_tank;
pub mod camo;
pub mod common;
pub mod mg_nest;
pub mod minefield;
pub mod sandbag;
pub mod spec;
pub mod watchtower;
pub mod wire;

pub use anti_tank::AntiTankKind;
pub use camo::{
    camo_concealed, CamoBypassReason, CamoConcealment, CamoConcealmentInputs,
    CamoNetting, BYPASS_RANGE_TILES, CAMO_NETTING_HP, CAMO_NETTING_TILE_FOOTPRINT,
};
pub use common::{
    FortId, FortificationFaction, FortificationId, FortificationKind,
};
pub use mg_nest::{
    fire_binding_for, route_bunker_slit_damage, spotter_scope_acquisition_multiplier,
    AmmoBoxMg, BunkerFiringSlit, BunkerSlitDamageResult, BunkerSlitRoundKind,
    CrewedKind, FireBinding, MgNest, MgNestError, MgNestFireOutcome,
    MgNestUncrewReason, MgNestUncrewedEvent, MgTripod, MgTripodError,
    MgTripodPhase, SpotterScope, AMMO_BOX_MG_MAX_HP, AMMO_BOX_MG_ROUNDS,
    BUNKER_FIRING_SLIT_APERTURE_PX, BUNKER_FIRING_SLIT_HP,
    MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES, MG_DOCTRINE_RETREAT_HP_THRESHOLD,
    MG_DOCTRINE_THREAT_RANGE_TILES, MG_NEST_STATIC_MAX_HP,
    MG_TRIPOD_DEPLOYED_HP, MG_TRIPOD_DEPLOY_SECONDS, SPOTTER_SCOPE_HP,
    SPOTTER_SCOPE_ACQUISITION_MULTIPLIER,
};
pub use minefield::MineKind;
pub use sandbag::{
    apply_damage_to_wall, sandbag_eroded_events, sandbag_pixel_mask,
    sandbag_tier_for_hp, SandbagErodedEvent, SandbagPixelMask, SandbagTier,
    SandbagWall, SandbagWallSpec, SANDBAG_HIGH_MAX_HP, SANDBAG_LOW_MAX_HP,
    SANDBAG_MID_MAX_HP,
};
pub use spec::{FortCoverLevel, FortificationSpec, SandbagCoverByStance};
pub use watchtower::{
    apply_destruction_collapse, collapse_distance_tiles, fall_impulse_damage_for,
    faction_radio_range, observation_post_artillery_multiplier, spotlight_illuminates,
    spotter_acquisition_multiplier, ActorInCollapseRadius, FallImpulseDamageEvent,
    ObservationPost, RadioRepeater, Spotlight, SpotlightConeInputs, SpotlightDazzledEvent,
    SpotterAcquisitionInputs, SpotterMark, Watchtower, WatchtowerDestructionOutcome,
    WatchtowerDestructionPending, WatchtowerDestroyedEvent, WatchtowerTier,
    OBSERVATION_POST_ARTILLERY_ACQUISITION_MULTIPLIER, OBSERVATION_POST_MAX_HP,
    RADIO_REPEATER_MAX_HP, RADIO_REPEATER_RANGE_BONUS_TILES,
    SPOTLIGHT_CONE_RANGE_TILES, SPOTLIGHT_DAZZLE_SECONDS, SPOTLIGHT_HALF_ANGLE_DEGREES,
    SPOTLIGHT_MAX_HP, SPOTLIGHT_POWER_DRAW_KW, SPOTTER_TARGET_MARK_ACQUISITION_BONUS,
    WATCHTOWER_T1_BASE_FALL_DAMAGE, WATCHTOWER_T1_COLLAPSE_RADIUS_TILES,
    WATCHTOWER_T1_MAX_HP, WATCHTOWER_T2_BASE_FALL_DAMAGE,
    WATCHTOWER_T2_COLLAPSE_RADIUS_TILES, WATCHTOWER_T2_MAX_HP,
    WATCHTOWER_T3_BASE_FALL_DAMAGE, WATCHTOWER_T3_COLLAPSE_RADIUS_TILES,
    WATCHTOWER_T3_MAX_HP,
};
pub use wire::WireKind;
