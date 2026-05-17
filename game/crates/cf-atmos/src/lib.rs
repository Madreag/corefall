//! cf-atmos — atmospherics-grade kernel.
//!
//! Scaffolded by M0-001; real implementation lands in M7.5 (DR-036 / T-MAT)
//! + M19 atmospherics-grade kernel.
//!
//! M12B (2026-05-17) introduces the [`room`] submodule — the per-room
//! reverb derivation bridge that joins cf-atmos's room geometry with
//! `cf-audio::ReverbProfile` derivation. Per M12B spec § Crates /
//! modules touched: "MODIFY: Expose `reverb_profile(room_id) ->
//! ReverbProfile` derived from `volume_m3` + wall_material_distribution.".

pub mod room;

pub use room::{reverb_profile, RoomAtmosphere};
