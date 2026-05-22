//! **M12B** § Per-room reverb derivation bridge between cf-atmos's room
//! kernel + cf-audio's [`cf_audio::ReverbProfile`].
//!
//! Per spec § Crates / modules touched:
//!
//! > `cf-atmos::room` MODIFY: Expose `reverb_profile(room_id) ->
//! > ReverbProfile` derived from `volume_m3` + wall_material_distribution.
//!
//! M19 ships the room kernel (full pressure / temperature / aperture
//! state). M12B's contribution is the acoustic-derivation API: take the
//! room's `volume_m3` + wall surface area distribution (keyed by
//! `cf-material` material id) + open-aperture fraction → derive a
//! [`cf_audio::ReverbProfile`] deterministically.
//!
//! The room kernel keeps the geometry; cf-audio owns the math. This
//! module is the thin adapter between them so cf-control's per-tick
//! spatial-resolve pass can call `reverb_profile(room_id)` once per
//! audio cue without pulling cf-material into cf-audio.

use cf_audio::{derive_reverb_profile, fraction_of_walls_open, DecayBand, ReverbProfile, WallComposition};
use cf_material::{MaterialId, MaterialRegistry};

/// owns the full room (pressure, temperature, gas mix, etc.); this is
/// the M12B audio-relevant projection — enough fields to derive a
/// [`ReverbProfile`].
#[derive(Debug, Clone, PartialEq)]
pub struct RoomAtmosphere {
    /// Stable room id (referenced by `audio.reverb_applied`).
    pub id: u64,
    /// Room volume in m³ (drives reverb tail seconds).
    pub volume_m3: f32,
    /// Total wall surface area in m² (drives `fraction_of_walls_open`).
    pub total_wall_m2: f32,
    /// Per-material wall surface area `(material_id, area_m2)`. Sums to
    /// `total_wall_m2` minus any open-aperture area.
    pub walls: Vec<(MaterialId, f32)>,
    /// Open-aperture area in m² (open doors, breaches, windows). Drives
    /// the `wet_dry_mix → dry` term + `aperture_attenuation_db`.
    pub open_aperture_m2: f32,
}

impl RoomAtmosphere {
    /// canonical material registry.
    ///
    /// The wall composition is built by joining each `(material_id,
    /// area)` row against `registry.acoustic_for(material_id)`. Open
    /// apertures contribute zero to the weighted echo coefficient.
    /// Identical inputs → identical output.
    #[must_use]
    pub fn reverb_profile(&self, registry: &MaterialRegistry) -> ReverbProfile {
        let mut comp: Vec<WallComposition> = Vec::with_capacity(self.walls.len());
        for (id, area) in &self.walls {
            let acoustics = registry.acoustic_for(*id);
            comp.push(WallComposition {
                echo_coefficient: acoustics.echo_coefficient,
                decay_band: DecayBand::from_str(acoustics.decay_band),
                surface_area_m2: *area,
            });
        }
        let frac_open = fraction_of_walls_open(self.open_aperture_m2, self.total_wall_m2);
        derive_reverb_profile(self.volume_m3, &comp, frac_open)
    }
}

/// shape. `reverb_profile(room_id) -> ReverbProfile`. Returns the
/// open-outdoor profile when `rooms` doesn't contain `room_id`.
#[must_use]
pub fn reverb_profile(rooms: &[RoomAtmosphere], registry: &MaterialRegistry, room_id: u64) -> ReverbProfile {
    match rooms.iter().find(|r| r.id == room_id) {
        Some(r) => r.reverb_profile(registry),
        None => ReverbProfile::open_outdoor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_audio::DecayBand;
    use cf_material::{load_registry_from_file, MaterialRegistry};
    use std::path::PathBuf;

    fn registry() -> MaterialRegistry {
        // The cargo test harness runs from `game/crates/cf-atmos`, so the
        // registry sits at `../../content/materials/material_registry.json`.
        let cwd = std::env::current_dir().expect("cwd");
        let candidate_a = cwd.join("../../content/materials/material_registry.json");
        let candidate_b = PathBuf::from("game/content/materials/material_registry.json");
        let path = if candidate_a.exists() {
            candidate_a
        } else {
            candidate_b
        };
        let (r, _report) = load_registry_from_file(path).expect("load registry");
        r
    }

    #[test]
    fn steel_bunker_yields_short_bright_ringing_tail() {
        // 4×4×3 m steel bunker (V=48 m³, walls 100% metal_nohook id=3 → steel).
        let registry = registry();
        let room = RoomAtmosphere {
            id: 1,
            volume_m3: 48.0,
            total_wall_m2: 80.0,
            walls: vec![(3, 80.0)], // metal_nohook = steel acoustic row.
            open_aperture_m2: 0.0,
        };
        let p = room.reverb_profile(&registry);
        assert!(p.tail_seconds < 0.25);
        assert_eq!(p.decay_band, DecayBand::BrightRinging);
        assert!(p.decay_coefficient >= 0.9);
    }

    #[test]
    fn concrete_warehouse_yields_long_bright_tail() {
        // 30×20×4 m concrete warehouse (V=2400 m³).
        let registry = registry();
        let room = RoomAtmosphere {
            id: 2,
            volume_m3: 2400.0,
            total_wall_m2: 2000.0,
            walls: vec![(2, 2000.0)], // concrete
            open_aperture_m2: 0.0,
        };
        let p = room.reverb_profile(&registry);
        assert!(p.tail_seconds >= 2.0);
        assert_eq!(p.decay_band, DecayBand::Bright);
    }

    #[test]
    fn fabric_lined_room_dampens_to_near_anechoic() {
        // 80% cloth + 20% wood per acceptance scenario.
        let registry = registry();
        let room = RoomAtmosphere {
            id: 3,
            volume_m3: 300.0,
            total_wall_m2: 100.0,
            walls: vec![(9, 80.0), (8, 20.0)], // cloth + wood
            open_aperture_m2: 0.0,
        };
        let p = room.reverb_profile(&registry);
        assert!(p.decay_coefficient <= 0.20);
        assert_eq!(p.decay_band, DecayBand::Dampened);
        assert!(p.wet_dry_mix <= 0.25);
    }

    #[test]
    fn reverb_profile_function_returns_open_outdoor_for_unknown_room() {
        let registry = registry();
        let p = reverb_profile(&[], &registry, 42);
        assert!(p.is_mostly_dry());
        assert!((p.tail_seconds - 0.0).abs() < 1e-6);
    }

    #[test]
    fn reverb_profile_function_delegates_to_room() {
        let registry = registry();
        let room = RoomAtmosphere {
            id: 7,
            volume_m3: 48.0,
            total_wall_m2: 80.0,
            walls: vec![(3, 80.0)],
            open_aperture_m2: 0.0,
        };
        let rooms = vec![room];
        let p = reverb_profile(&rooms, &registry, 7);
        assert_eq!(p.decay_band, DecayBand::BrightRinging);
    }

    #[test]
    fn open_aperture_drops_wet_send() {
        let registry = registry();
        let mut room = RoomAtmosphere {
            id: 4,
            volume_m3: 2400.0,
            total_wall_m2: 2000.0,
            walls: vec![(2, 2000.0)],
            open_aperture_m2: 0.0,
        };
        let closed = room.reverb_profile(&registry);
        room.open_aperture_m2 = 1500.0;
        let mostly_open = room.reverb_profile(&registry);
        assert!(closed.wet_dry_mix > mostly_open.wet_dry_mix);
    }

    #[test]
    fn reverb_profile_is_deterministic() {
        let registry = registry();
        let room = RoomAtmosphere {
            id: 5,
            volume_m3: 300.0,
            total_wall_m2: 100.0,
            walls: vec![(9, 80.0), (8, 20.0)],
            open_aperture_m2: 0.0,
        };
        let a = room.reverb_profile(&registry);
        let b = room.reverb_profile(&registry);
        assert_eq!(a, b);
    }
}
