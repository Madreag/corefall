//! **M14C** — Tank-grade weapon entries (`rpg_launcher_v1` HEAT,
//! `tank_autocannon_t3` APFSDS).
//!
//! Spec § Crates / modules touched:
//!   cf-equipment::weapons MODIFY: add `rpg_launcher_v1` (HEAT) +
//!   `tank_autocannon_t3` (APFSDS) + magazines + reload profiles.
//!
//! Each weapon points at one of the M14C ammo specs (`ammo_spec_id` ==
//! `heat_round_v1` / `apfsds_round_v1`). The validation contract
//! (VAL-M14C-005 + VAL-M14C-006) requires that:
//!   - the weapon's `primary_round` equals the matching `RoundKind`;
//!   - a populated magazine + reload profile is present.

use serde::{Deserialize, Serialize};

use crate::magazine::{Magazine, RoundKind};

/// One M14C tank-grade weapon spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M14cWeaponSpec {
    pub id: String,
    pub display_name: String,
    pub primary_round: RoundKind,
    pub ammo_spec_id: String,
    pub mag_capacity: u32,
    pub reload_seconds: f32,
    pub fire_interval_seconds: f32,
    pub projectile_speed: f32,
    pub damage_per_hit: f32,
}

impl M14cWeaponSpec {
    /// Construct an empty magazine sized for this weapon.
    pub fn magazine(&self) -> Magazine {
        Magazine::new(self.mag_capacity.max(1), 0, self.primary_round)
    }
}

/// Stable id of the M14C reference HEAT launcher.
pub const RPG_LAUNCHER_V1_ID: &str = "rpg_launcher_v1";
/// Stable id of the M14C reference APFSDS autocannon.
pub const TANK_AUTOCANNON_T3_ID: &str = "tank_autocannon_t3";

/// RON text for the canonical rpg_launcher_v1.ron.
pub const RPG_LAUNCHER_V1_RON: &str =
    include_str!("../../../content/equipment/weapons/rpg_launcher_v1.ron");
/// RON text for the canonical tank_autocannon_t3.ron.
pub const TANK_AUTOCANNON_T3_RON: &str =
    include_str!("../../../content/equipment/weapons/tank_autocannon_t3.ron");

/// Errors that may occur loading an [`M14cWeaponSpec`] from text.
#[derive(Debug)]
pub enum M14cWeaponSpecLoadError {
    /// RON parse failed.
    Parse(ron::error::SpannedError),
    /// The deserialized payload violates M14C invariants.
    Invariant(String),
}

impl std::fmt::Display for M14cWeaponSpecLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            M14cWeaponSpecLoadError::Parse(e) => write!(f, "ron parse error: {e}"),
            M14cWeaponSpecLoadError::Invariant(s) => write!(f, "invariant violation: {s}"),
        }
    }
}

impl std::error::Error for M14cWeaponSpecLoadError {}

/// Parse an [`M14cWeaponSpec`] from RON text and validate invariants.
pub fn parse_m14c_weapon(text: &str) -> Result<M14cWeaponSpec, M14cWeaponSpecLoadError> {
    let spec: M14cWeaponSpec = ron::from_str(text).map_err(M14cWeaponSpecLoadError::Parse)?;
    if spec.mag_capacity == 0 {
        return Err(M14cWeaponSpecLoadError::Invariant(
            "M14C weapon mag_capacity must be > 0".to_string(),
        ));
    }
    if spec.reload_seconds <= 0.0 {
        return Err(M14cWeaponSpecLoadError::Invariant(
            "M14C weapon reload_seconds must be > 0".to_string(),
        ));
    }
    if spec.ammo_spec_id.is_empty() {
        return Err(M14cWeaponSpecLoadError::Invariant(
            "M14C weapon ammo_spec_id must not be empty".to_string(),
        ));
    }
    Ok(spec)
}

/// Resolve an M14C weapon id to its loaded [`M14cWeaponSpec`].
pub fn resolve_m14c_weapon(id: &str) -> Option<M14cWeaponSpec> {
    match id {
        RPG_LAUNCHER_V1_ID => parse_m14c_weapon(RPG_LAUNCHER_V1_RON).ok(),
        TANK_AUTOCANNON_T3_ID => parse_m14c_weapon(TANK_AUTOCANNON_T3_RON).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ammo_spec::{APFSDS_ROUND_V1_ID, HEAT_ROUND_V1_ID};

    #[test]
    fn rpg_launcher_v1_loads_as_heat_capable() {
        let spec = parse_m14c_weapon(RPG_LAUNCHER_V1_RON).expect("rpg_launcher_v1.ron parses");
        assert_eq!(spec.id, RPG_LAUNCHER_V1_ID);
        assert_eq!(spec.primary_round, RoundKind::Heat);
        assert_eq!(spec.ammo_spec_id, HEAT_ROUND_V1_ID);
        assert!(spec.mag_capacity >= 1, "magazine populated");
        assert!(spec.reload_seconds > 0.0, "reload profile populated");
    }

    #[test]
    fn tank_autocannon_t3_loads_as_apfsds_capable() {
        let spec = parse_m14c_weapon(TANK_AUTOCANNON_T3_RON).expect("tank_autocannon_t3.ron parses");
        assert_eq!(spec.id, TANK_AUTOCANNON_T3_ID);
        assert_eq!(spec.primary_round, RoundKind::Apfsds);
        assert_eq!(spec.ammo_spec_id, APFSDS_ROUND_V1_ID);
        assert!(spec.mag_capacity >= 1, "magazine populated");
        assert!(spec.reload_seconds > 0.0, "reload profile populated");
    }

    #[test]
    fn rpg_launcher_v1_magazine_pops_heat() {
        let spec = parse_m14c_weapon(RPG_LAUNCHER_V1_RON).unwrap();
        let mut mag = spec.magazine();
        let r = mag.pop_next_round().expect("magazine has rounds");
        assert_eq!(r.round_kind, RoundKind::Heat);
    }

    #[test]
    fn tank_autocannon_t3_magazine_pops_apfsds() {
        let spec = parse_m14c_weapon(TANK_AUTOCANNON_T3_RON).unwrap();
        let mut mag = spec.magazine();
        let r = mag.pop_next_round().expect("magazine has rounds");
        assert_eq!(r.round_kind, RoundKind::Apfsds);
    }

    #[test]
    fn resolve_known_weapon_ids() {
        assert!(resolve_m14c_weapon(RPG_LAUNCHER_V1_ID).is_some());
        assert!(resolve_m14c_weapon(TANK_AUTOCANNON_T3_ID).is_some());
        assert!(resolve_m14c_weapon("does_not_exist").is_none());
    }
}
