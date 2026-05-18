//! cf-killcam — M8 killcam stub. Spec § Killcam: 3-second slow-motion
//! replay from the killing enemy's perspective on player death; toggleable
//! per accessibility (`killcam_enabled`). Slow-mo kill cam: 1.5s
//! cinematic camera angle on mission boss final blow.
//!
//! M8 ships the local state machine + transition events; network forward-
//! compat for M36+ multiplayer. **M14C** adds `heat_penetration` +
//! `apfsds_through_module` variant payloads so M41's polished kill cam
//! has live death-type variants per VAL-M14C-013.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod playback;
pub mod recorder;
pub mod variant;

pub use playback::{tick, KillcamPhase, KillcamState, KILLCAM_DURATION_MS, SLOW_MO_KILL_CAM_DURATION_MS};
pub use recorder::{start, start_slow_mo_kill_cam, KillcamTrigger};
pub use variant::{
    dispatch_pair_contact_variant, dispatch_variant, ApfsdsThroughModulePayload, HeatPenetrationPayload,
    KillcamVariant, KillcamVariantId, KillcamVariantTrigger, ProjectilePairContactPayload,
    APFSDS_THROUGH_MODULE_VARIANT_ID, DEFAULT_REPLAY_INTERCEPTS, HEAT_PENETRATION_VARIANT_ID,
    PROJECTILE_PAIR_CONTACT_VARIANT_ID,
};
