//! **M14A** § "Sim overlay — material overlay" — per-material walking
//! modulators (friction, speed, thermal contact, hazard contact).
//!
//! Pure functions over `MaterialId`. Callers (cf-actor::material_contact)
//! read the modulator + per-tile hazard at planted foot position to drive
//! foot slip, walk-speed scaling, thermal contact, and hazard.actor_contact
//! emission.

use serde::{Deserialize, Serialize};

use crate::chunked::MaterialId;

/// **M14A** § "Per-material walk modulator" — friction + speed scaling.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WalkModulator {
    /// Friction multiplier (1.0 = dry concrete; 0.2 = oil; 0.4 = wet).
    pub friction_mult: f32,
    /// Walk-speed multiplier (1.0 = baseline; 0.6 = snow; 0.4 = mud).
    pub speed_mult: f32,
    /// True when stepping on this tile should emit a hazard.actor_contact
    /// event (lava, acid, electric, fire).
    pub emit_hazard: bool,
    /// Per-tick foot damage in HP units (lava=5.0; acid=2.0; default=0).
    pub foot_damage_hp_per_tick: f32,
    /// Hazard kind string for hazard.actor_contact ("fire" / "acid" /
    /// "electric" / "wet" / "ice" / "snow" / "sand" / "mud" / "oil").
    pub hazard_kind: &'static str,
}

impl Default for WalkModulator {
    fn default() -> Self {
        Self {
            friction_mult: 1.0,
            speed_mult: 1.0,
            emit_hazard: false,
            foot_damage_hp_per_tick: 0.0,
            hazard_kind: "",
        }
    }
}

/// **M14A** § "material_walk_modulator" — lookup per-material walk modifier.
///
/// Material id mapping mirrors the DR-007 launch set (`air`/`dirt`/`concrete`
/// /`metal_nohook`/`hazard`/`loose_fill`/`repair_fill`/`anchor`). M15
/// extends with `lava`/`acid`/`oil`/`ice`/`snow`/`mud`/`water` via the
/// active material kernel; M14A reads those when the kernel ships and
/// falls back to "default" for unknown ids.
pub fn material_walk_modulator(id: MaterialId) -> WalkModulator {
    let key = id;
    match key {
        // air / open: shouldn't be planted on; pass-through baseline.
        0 => WalkModulator::default(),
        // dirt: slightly soft.
        1 => WalkModulator {
            friction_mult: 0.95,
            speed_mult: 0.95,
            ..WalkModulator::default()
        },
        // concrete: baseline.
        2 => WalkModulator::default(),
        // metal_nohook: slick.
        3 => WalkModulator {
            friction_mult: 0.85,
            speed_mult: 1.0,
            ..WalkModulator::default()
        },
        // hazard: emit hazard contact.
        4 => WalkModulator {
            friction_mult: 0.7,
            speed_mult: 0.7,
            emit_hazard: true,
            foot_damage_hp_per_tick: 2.0,
            hazard_kind: "hazard",
        },
        // loose_fill: rubble pile, slow + sticky.
        5 => WalkModulator {
            friction_mult: 0.7,
            speed_mult: 0.7,
            ..WalkModulator::default()
        },
        // repair_fill: very slick.
        6 => WalkModulator {
            friction_mult: 0.9,
            speed_mult: 1.0,
            ..WalkModulator::default()
        },
        // anchor: baseline.
        7 => WalkModulator::default(),
        // M15 forward-compat: lava (12) burns; acid (13) etches; ice (14)
        // slides; snow (15) slows; oil (16) slides hard; mud (17) sucks;
        // water (18) wets.
        12 => WalkModulator {
            friction_mult: 0.6,
            speed_mult: 0.5,
            emit_hazard: true,
            foot_damage_hp_per_tick: 5.0,
            hazard_kind: "fire",
        },
        13 => WalkModulator {
            friction_mult: 0.7,
            speed_mult: 0.7,
            emit_hazard: true,
            foot_damage_hp_per_tick: 2.0,
            hazard_kind: "acid",
        },
        14 => WalkModulator {
            friction_mult: 0.2,
            speed_mult: 0.7,
            ..WalkModulator::default()
        },
        15 => WalkModulator {
            friction_mult: 0.9,
            speed_mult: 0.6,
            ..WalkModulator::default()
        },
        16 => WalkModulator {
            friction_mult: 0.2,
            speed_mult: 0.6,
            ..WalkModulator::default()
        },
        17 => WalkModulator {
            friction_mult: 0.5,
            speed_mult: 0.4,
            ..WalkModulator::default()
        },
        18 => WalkModulator {
            friction_mult: 0.4,
            speed_mult: 0.85,
            ..WalkModulator::default()
        },
        // Unknown material → baseline.
        _ => WalkModulator::default(),
    }
}

/// **M14A** § "material_thermal_contact" — heat-transfer params per material.
pub fn material_thermal_contact(id: MaterialId) -> ThermalContact {
    let key = id;
    match key {
        // metal_nohook: high conductivity.
        3 => ThermalContact {
            conductivity: 0.9,
            ambient_temp_c: 20.0,
        },
        // lava (12): very hot.
        12 => ThermalContact {
            conductivity: 0.95,
            ambient_temp_c: 1100.0,
        },
        // ice (14): cold.
        14 => ThermalContact {
            conductivity: 0.7,
            ambient_temp_c: -10.0,
        },
        _ => ThermalContact {
            conductivity: 0.3,
            ambient_temp_c: 20.0,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermalContact {
    pub conductivity: f32,
    pub ambient_temp_c: f32,
}

impl Default for ThermalContact {
    fn default() -> Self {
        Self {
            conductivity: 0.3,
            ambient_temp_c: 20.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_emits_fire_hazard_with_damage() {
        let m = material_walk_modulator(12);
        assert!(m.emit_hazard);
        assert_eq!(m.hazard_kind, "fire");
        assert!((m.foot_damage_hp_per_tick - 5.0).abs() < 1e-6);
    }

    #[test]
    fn oil_drops_friction_to_one_fifth() {
        let m = material_walk_modulator(16);
        assert!((m.friction_mult - 0.2).abs() < 1e-6);
    }

    #[test]
    fn concrete_is_baseline() {
        let m = material_walk_modulator(2);
        assert!((m.friction_mult - 1.0).abs() < 1e-6);
        assert!((m.speed_mult - 1.0).abs() < 1e-6);
        assert!(!m.emit_hazard);
    }
}
