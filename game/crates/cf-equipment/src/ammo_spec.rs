//! **M14C** — ammo-spec content loader for HEAT + APFSDS rounds.
//!
//! Each ammo spec is a `.ron` file under `game/content/ammo/`. The
//! `AmmoSpec` struct mirrors the spec field set per `specs/active/M14C.md`
//! "Notes for the implementer" plus the validation-contract field
//! requirements (`charge_mj`, `jet_velocity_mps`, `cone_half_angle_deg`,
//! `optimum_standoff_m` for HEAT; `rod_mass_kg`, `velocity_mps`,
//! `rod_aspect_ratio`, `sabot_mass_kg`, `material` for APFSDS).
//!
//! M14C ships two reference specs:
//! - `heat_round_v1.ron`   — RPG-grade 10 MJ shaped charge @ 3 km/s
//! - `apfsds_round_v1.ron` — 7 kg DU long-rod @ 1600 m/s, 30:1 aspect

use serde::{Deserialize, Serialize};

use crate::magazine::RoundKind;

/// Top-level ammo spec discriminated by `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AmmoSpec {
    Heat(HeatRoundSpec),
    Apfsds(ApfsdsRoundSpec),
}

impl AmmoSpec {
    /// Stable spec id.
    pub fn id(&self) -> &str {
        match self {
            AmmoSpec::Heat(s) => &s.id,
            AmmoSpec::Apfsds(s) => &s.id,
        }
    }

    /// `RoundKind` discriminator for the magazine.
    pub fn round_kind(&self) -> RoundKind {
        match self {
            AmmoSpec::Heat(_) => RoundKind::Heat,
            AmmoSpec::Apfsds(_) => RoundKind::Apfsds,
        }
    }

    /// Try to interpret this spec as a HEAT round.
    pub fn as_heat(&self) -> Option<&HeatRoundSpec> {
        match self {
            AmmoSpec::Heat(s) => Some(s),
            _ => None,
        }
    }

    /// Try to interpret this spec as an APFSDS round.
    pub fn as_apfsds(&self) -> Option<&ApfsdsRoundSpec> {
        match self {
            AmmoSpec::Apfsds(s) => Some(s),
            _ => None,
        }
    }
}

/// M14C HEAT round (shaped-charge anti-tank). All units SI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatRoundSpec {
    pub id: String,
    pub display_name: String,
    /// Shaped-charge energy (megajoules).
    pub charge_mj: f32,
    /// Charge mass in kilograms (drives the velocity×mass damage model).
    pub charge_mass_kg: f32,
    /// Jet velocity (m/s) — spec § 10 MJ @ ~3 km/s.
    pub jet_velocity_mps: f32,
    /// Penetration cone half-angle in degrees (5° per spec).
    pub cone_half_angle_deg: f32,
    /// Optimum stand-off distance for full jet formation (m).
    pub optimum_standoff_m: f32,
    /// Below this stand-off the jet is under-formed (m).
    pub min_jet_formation_standoff_m: f32,
}

impl HeatRoundSpec {
    /// **VAL-M14C-022**: HEAT damage tracks `charge_mass_kg × jet_velocity_mps`
    /// (velocity × mass product), NOT `0.5 × m × v²`. Multiplied by a
    /// content-tunable scalar to bring the result into the gameplay
    /// damage band.
    pub fn damage_scalar(&self) -> f32 {
        self.charge_mass_kg.max(0.0) * self.jet_velocity_mps.max(0.0)
    }
}

/// M14C APFSDS round (long-rod kinetic penetrator). All units SI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApfsdsRoundSpec {
    pub id: String,
    pub display_name: String,
    /// Rod (penetrator) mass in kilograms — spec § 7 kg DU rod.
    pub rod_mass_kg: f32,
    /// Sabot mass that discards in flight (kg). Required by the contract.
    pub sabot_mass_kg: f32,
    /// Muzzle velocity of the sabot package (m/s). Spec § 1600 m/s.
    pub velocity_mps: f32,
    /// Rod length-to-diameter aspect ratio. Spec § ~30:1.
    pub rod_aspect_ratio: f32,
    /// Rod length in millimeters (drives long-rod penetration math).
    pub rod_length_mm: f32,
    /// Penetrator material id. `depleted_uranium` is the M14C baseline.
    pub material: String,
}

impl ApfsdsRoundSpec {
    /// Derived KE per spec note: `0.5 × m × v² ≈ 9.0 MJ` for the v1 baseline.
    pub fn kinetic_energy_mj(&self) -> f32 {
        let m = self.rod_mass_kg.max(0.0);
        let v = self.velocity_mps.max(0.0);
        0.5 * m * v * v / 1.0e6
    }
}

/// Stable id of the M14C reference HEAT round.
pub const HEAT_ROUND_V1_ID: &str = "heat_round_v1";
/// Stable id of the M14C reference APFSDS round.
pub const APFSDS_ROUND_V1_ID: &str = "apfsds_round_v1";

/// **M14C** § HEAT v1 reference spec (10 MJ @ 3 km/s, 5° cone, 0.6 m optimum).
#[must_use]
pub fn heat_round_v1() -> HeatRoundSpec {
    HeatRoundSpec {
        id: HEAT_ROUND_V1_ID.to_string(),
        display_name: "RPG HEAT v1".to_string(),
        charge_mj: 10.0,
        charge_mass_kg: 1.0,
        jet_velocity_mps: 3000.0,
        cone_half_angle_deg: 5.0,
        optimum_standoff_m: 0.6,
        min_jet_formation_standoff_m: 0.2,
    }
}

/// **M14C** § APFSDS v1 reference spec (7 kg DU @ 1600 m/s, 30:1 aspect).
#[must_use]
pub fn apfsds_round_v1() -> ApfsdsRoundSpec {
    ApfsdsRoundSpec {
        id: APFSDS_ROUND_V1_ID.to_string(),
        display_name: "Tank APFSDS v1".to_string(),
        rod_mass_kg: 7.0,
        sabot_mass_kg: 1.5,
        velocity_mps: 1600.0,
        rod_aspect_ratio: 30.0,
        rod_length_mm: 600.0,
        material: "depleted_uranium".to_string(),
    }
}

/// Errors that may occur loading an [`AmmoSpec`] from text.
#[derive(Debug)]
pub enum AmmoSpecLoadError {
    /// RON parse failed.
    Parse(ron::error::SpannedError),
    /// The deserialized payload doesn't satisfy the M14C spec invariants
    /// (e.g., HEAT round with zero charge, APFSDS without sabot mass).
    Invariant(String),
}

impl std::fmt::Display for AmmoSpecLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmmoSpecLoadError::Parse(e) => write!(f, "ron parse error: {e}"),
            AmmoSpecLoadError::Invariant(s) => write!(f, "invariant violation: {s}"),
        }
    }
}

impl std::error::Error for AmmoSpecLoadError {}

/// Parse an [`AmmoSpec`] from RON text and validate spec invariants.
pub fn parse_ammo_spec(text: &str) -> Result<AmmoSpec, AmmoSpecLoadError> {
    let spec: AmmoSpec = ron::from_str(text).map_err(AmmoSpecLoadError::Parse)?;
    match &spec {
        AmmoSpec::Heat(h) => {
            if h.charge_mj <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "HEAT charge_mj must be positive".to_string(),
                ));
            }
            if h.jet_velocity_mps <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "HEAT jet_velocity_mps must be positive".to_string(),
                ));
            }
            if h.cone_half_angle_deg <= 0.0 || h.cone_half_angle_deg > 45.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "HEAT cone_half_angle_deg must be in (0, 45]".to_string(),
                ));
            }
            if h.optimum_standoff_m <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "HEAT optimum_standoff_m must be positive".to_string(),
                ));
            }
        }
        AmmoSpec::Apfsds(a) => {
            if a.rod_mass_kg <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "APFSDS rod_mass_kg must be positive".to_string(),
                ));
            }
            if a.sabot_mass_kg <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "APFSDS sabot_mass_kg must be positive".to_string(),
                ));
            }
            if a.velocity_mps <= 0.0 {
                return Err(AmmoSpecLoadError::Invariant(
                    "APFSDS velocity_mps must be positive".to_string(),
                ));
            }
            if a.material.is_empty() {
                return Err(AmmoSpecLoadError::Invariant(
                    "APFSDS material must not be empty".to_string(),
                ));
            }
        }
    }
    Ok(spec)
}

/// RON text for the canonical heat_round_v1 spec, used to back the
/// content-on-disk + the in-process registry.
pub const HEAT_ROUND_V1_RON: &str = include_str!("../../../content/ammo/heat_round_v1.ron");
/// RON text for the canonical apfsds_round_v1 spec.
pub const APFSDS_ROUND_V1_RON: &str = include_str!("../../../content/ammo/apfsds_round_v1.ron");

/// Resolve an ammo spec id to its loaded `AmmoSpec`. Returns `None` for
/// unknown ids.
pub fn resolve_ammo_spec(id: &str) -> Option<AmmoSpec> {
    match id {
        HEAT_ROUND_V1_ID => parse_ammo_spec(HEAT_ROUND_V1_RON).ok(),
        APFSDS_ROUND_V1_ID => parse_ammo_spec(APFSDS_ROUND_V1_RON).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_round_v1_loads() {
        let spec = parse_ammo_spec(HEAT_ROUND_V1_RON).expect("heat_round_v1.ron parses");
        let heat = spec.as_heat().expect("kind=heat");
        assert_eq!(heat.id, HEAT_ROUND_V1_ID);
        assert!((heat.charge_mj - 10.0).abs() < 1e-3, "charge_mj=10 MJ");
        assert!((heat.jet_velocity_mps - 3000.0).abs() < 1e-3, "jet ~3 km/s");
        assert!((heat.cone_half_angle_deg - 5.0).abs() < 1e-3, "5° cone");
        assert!((heat.optimum_standoff_m - 0.6).abs() < 1e-3, "0.6 m standoff");
        assert_eq!(spec.round_kind(), RoundKind::Heat);
    }

    #[test]
    fn apfsds_round_v1_loads() {
        let spec = parse_ammo_spec(APFSDS_ROUND_V1_RON).expect("apfsds_round_v1.ron parses");
        let apfsds = spec.as_apfsds().expect("kind=apfsds");
        assert_eq!(apfsds.id, APFSDS_ROUND_V1_ID);
        assert!((apfsds.rod_mass_kg - 7.0).abs() < 1e-3, "7 kg rod");
        assert!((apfsds.velocity_mps - 1600.0).abs() < 1e-3, "1600 m/s");
        assert!((apfsds.rod_aspect_ratio - 30.0).abs() < 1e-3, "30:1 aspect");
        assert!(apfsds.sabot_mass_kg > 0.0, "sabot_mass_kg present");
        assert_eq!(apfsds.material, "depleted_uranium");
        // Derived KE ≈ 0.5 × 7 × 1600² = 8.96 MJ (~9.0 MJ per spec).
        let ke = apfsds.kinetic_energy_mj();
        assert!((ke - 8.96).abs() < 0.1, "derived KE ≈ 9.0 MJ, got {ke}");
        assert_eq!(spec.round_kind(), RoundKind::Apfsds);
    }

    #[test]
    fn heat_damage_uses_velocity_mass_product() {
        // VAL-M14C-022: HEAT damage tracks `velocity × mass`, NOT raw KE.
        let mut a = heat_round_v1();
        let mut b = heat_round_v1();
        // Identical velocity × mass product, different individual values.
        a.charge_mass_kg = 2.0;
        a.jet_velocity_mps = 1500.0;
        b.charge_mass_kg = 1.0;
        b.jet_velocity_mps = 3000.0;
        assert!((a.damage_scalar() - b.damage_scalar()).abs() < 1e-3);
        // Identical raw KE, different velocity×mass — must NOT be equal.
        let mut c = heat_round_v1();
        let mut d = heat_round_v1();
        // 0.5 × 2 × 1000² = 1e6 == 0.5 × 8 × 500² = 1e6
        c.charge_mass_kg = 2.0;
        c.jet_velocity_mps = 1000.0;
        d.charge_mass_kg = 8.0;
        d.jet_velocity_mps = 500.0;
        assert!((c.damage_scalar() - d.damage_scalar()).abs() > 1.0);
    }

    #[test]
    fn parse_rejects_negative_charge() {
        let bad = "(kind: heat, id: \"x\", display_name: \"X\", charge_mj: -1.0, charge_mass_kg: 1.0, jet_velocity_mps: 3000.0, cone_half_angle_deg: 5.0, optimum_standoff_m: 0.6, min_jet_formation_standoff_m: 0.2)";
        assert!(matches!(parse_ammo_spec(bad), Err(AmmoSpecLoadError::Invariant(_))));
    }

    #[test]
    fn resolve_known_ids() {
        assert!(resolve_ammo_spec(HEAT_ROUND_V1_ID).is_some());
        assert!(resolve_ammo_spec(APFSDS_ROUND_V1_ID).is_some());
        assert!(resolve_ammo_spec("does_not_exist").is_none());
    }
}
