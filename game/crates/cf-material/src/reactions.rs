//! **M14G** § Material reaction → typed wound producer.
//!
//! Maps a per-tick "actor in contact with hazardous material X" sample to
//! a typed [`cf_wound::WoundKind`] record. Pure / deterministic.
//!
//! - Acid contact → `AcidBurn` (VAL-M14G-029).
//! - Refrigerant contact → `ChemicalBurn` (VAL-M14G-029).
//!
//! Other reactive materials (chlorine vapor, ammonia, etc.) reuse
//! `ChemicalBurn` until M16B ships a wider chemistry-affliction surface.

use cf_wound::registry::ZoneId;
use cf_wound::WoundKind;

/// **M14G** material-reaction typed wound emit candidate. Severity is
/// derived from the contact intensity (per-tick dwell × material reactivity).
#[derive(Debug, Clone, PartialEq)]
pub struct ReactionWoundEmit {
    pub kind: WoundKind,
    pub severity: f32,
    pub zone: ZoneId,
}

/// Map a material name (canonical lowercase) and contact intensity scalar
/// (0..1 over a single tick) to a typed wound. Returns `None` for materials
/// that do not feed a wound producer at M14G.
pub fn classify_reaction(material_name: &str, zone: ZoneId, intensity: f32) -> Option<ReactionWoundEmit> {
    let severity = intensity.clamp(0.0, 1.0);
    let kind = match material_name.to_ascii_lowercase().as_str() {
        "acid" => WoundKind::AcidBurn,
        "refrigerant" | "coolant" | "ammonia" | "chlorine" => WoundKind::ChemicalBurn,
        _ => return None,
    };
    Some(ReactionWoundEmit {
        kind,
        severity: severity.max(0.05),
        zone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14G-029: acid → AcidBurn, refrigerant → ChemicalBurn.
    #[test]
    fn acid_and_refrigerant_wound_kinds() {
        let acid = classify_reaction("acid", ZoneId::from("hand_left"), 0.6).unwrap();
        assert_eq!(acid.kind, WoundKind::AcidBurn);
        let refrigerant = classify_reaction("refrigerant", ZoneId::from("hand_right"), 0.3).unwrap();
        assert_eq!(refrigerant.kind, WoundKind::ChemicalBurn);
    }

    #[test]
    fn unmatched_material_returns_none() {
        assert!(classify_reaction("water", ZoneId::from("foot_left"), 1.0).is_none());
        assert!(classify_reaction("dirt", ZoneId::from("foot_left"), 1.0).is_none());
    }
}
