//! M7B: data-driven verb registry — 50+ verb defs.
//!
//! Each entry carries `verb_id`, family, display label, argument schema, and
//! per-doctrine compatibility row (held separately in `doctrine_compat.rs`).
//! The registry is the single source of truth: the M25 wheel + Tab overlay
//! + M23B commander-doctrine all read from `builtin_registry()`.
//!
//! The spec mandates the registry be data-driven so re-enumeration does not
//! require a Rust rebuild; the matching `game/content/ai/verbs/registry.ron`
//! mirrors these entries and `VerbRegistry::from_ron` parses it. The Rust
//! constant remains canonical at startup (tests + headless builds work
//! without a filesystem); the RON file is the content-author surface.

use serde::{Deserialize, Serialize};

/// **M7B**: verb family — drives wheel grouping + UI sectioning.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbFamily {
    Movement,
    Engagement,
    MovementToContact,
    RoleSpecific,
    Logistics,
}

impl VerbFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            VerbFamily::Movement => "movement",
            VerbFamily::Engagement => "engagement",
            VerbFamily::MovementToContact => "movement_to_contact",
            VerbFamily::RoleSpecific => "role_specific",
            VerbFamily::Logistics => "logistics",
        }
    }

    pub fn from_str(value: &str) -> Option<VerbFamily> {
        Some(match value {
            "movement" => VerbFamily::Movement,
            "engagement" => VerbFamily::Engagement,
            "movement_to_contact" => VerbFamily::MovementToContact,
            "role_specific" => VerbFamily::RoleSpecific,
            "logistics" => VerbFamily::Logistics,
            _ => return None,
        })
    }
}

pub fn verb_family_label(family: VerbFamily) -> &'static str {
    match family {
        VerbFamily::Movement => "Movement",
        VerbFamily::Engagement => "Engagement",
        VerbFamily::MovementToContact => "Movement-to-Contact",
        VerbFamily::RoleSpecific => "Role-Specific",
        VerbFamily::Logistics => "Logistics",
    }
}

/// **M7B**: argument kind for the registry's per-verb arg schema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerbArgKind {
    Waypoint,
    Actor,
    Door,
    Side,
    Sector,
    Window,
    Label,
    Index,
}

impl VerbArgKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VerbArgKind::Waypoint => "waypoint",
            VerbArgKind::Actor => "actor",
            VerbArgKind::Door => "door",
            VerbArgKind::Side => "side",
            VerbArgKind::Sector => "sector",
            VerbArgKind::Window => "window",
            VerbArgKind::Label => "label",
            VerbArgKind::Index => "index",
        }
    }
}

/// **M7B**: one argument slot in a verb's schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerbArgSpec {
    pub name: String,
    pub kind: VerbArgKind,
    pub required: bool,
}

/// **M7B**: a single verb definition. Held as `&'static str` slices where
/// possible; the RON path uses owned String + Vec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerbDef {
    pub verb_id: String,
    pub display_name: String,
    pub family: VerbFamily,
    pub args: Vec<VerbArgSpec>,
    /// Coarse target-shape predicate label (`none`, `door`, `actor`,
    /// `window`, `area`). Drives valid-target highlighting in the wheel
    /// + Tab overlay; the engine performs the precise gating.
    pub valid_target: String,
}

impl VerbDef {
    pub fn required_args(&self) -> usize {
        self.args.iter().filter(|a| a.required).count()
    }
}

/// **M7B**: verb registry. Ordered insertion is preserved; UI surfaces walk
/// `iter()` for stable wheel layout.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerbRegistry {
    pub verbs: Vec<VerbDef>,
}

impl<'a> IntoIterator for &'a VerbRegistry {
    type Item = &'a VerbDef;
    type IntoIter = std::slice::Iter<'a, VerbDef>;

    fn into_iter(self) -> Self::IntoIter {
        self.verbs.iter()
    }
}

impl VerbRegistry {
    pub fn new() -> Self {
        Self { verbs: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.verbs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, VerbDef> {
        self.verbs.iter()
    }

    pub fn into_iter_owned(self) -> std::vec::IntoIter<VerbDef> {
        self.verbs.into_iter()
    }

    pub fn find(&self, verb_id: &str) -> Option<&VerbDef> {
        self.verbs.iter().find(|d| d.verb_id == verb_id)
    }

    pub fn by_family(&self, family: VerbFamily) -> impl Iterator<Item = &VerbDef> {
        self.verbs.iter().filter(move |d| d.family == family)
    }

    /// Construct a registry from a RON document (the canonical content path
    /// at `game/content/ai/verbs/registry.ron`).
    pub fn from_ron(src: &str) -> Result<Self, String> {
        let value: VerbRegistry = ron::from_str(src).map_err(|e| format!("ron parse failed: {e}"))?;
        Ok(value)
    }
}

fn arg(name: &str, kind: VerbArgKind, required: bool) -> VerbArgSpec {
    VerbArgSpec {
        name: name.to_string(),
        kind,
        required,
    }
}

fn verb(
    verb_id: &str,
    display_name: &str,
    family: VerbFamily,
    args: Vec<VerbArgSpec>,
    valid_target: &str,
) -> VerbDef {
    VerbDef {
        verb_id: verb_id.to_string(),
        display_name: display_name.to_string(),
        family,
        args,
        valid_target: valid_target.to_string(),
    }
}

/// **M7B**: the 50+ canonical verbs. Held as a builder so tests can assert
/// uniqueness + size without touching the filesystem.
pub fn builtin_registry() -> VerbRegistry {
    let mut r = VerbRegistry::new();

    // ============================================================
    // Movement (11)
    // ============================================================
    r.verbs.push(verb(
        "move_to",
        "Move To",
        VerbFamily::Movement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb("stop", "Stop", VerbFamily::Movement, vec![], "none"));
    r.verbs.push(verb("halt", "Halt", VerbFamily::Movement, vec![], "none"));
    r.verbs.push(verb(
        "hold_position",
        "Hold Position",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "bound_alt",
        "Bound (alt)",
        VerbFamily::Movement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "bound_succ",
        "Bound (succ)",
        VerbFamily::Movement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "fall_back",
        "Fall Back",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "withdraw",
        "Withdraw",
        VerbFamily::Movement,
        vec![arg("rally", VerbArgKind::Waypoint, false)],
        "area",
    ));
    r.verbs.push(verb(
        "retreat_in_order",
        "Retreat In Order",
        VerbFamily::Movement,
        vec![arg("rally", VerbArgKind::Waypoint, false)],
        "area",
    ));
    r.verbs.push(verb(
        "rally_on_me",
        "Rally On Me",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb("regroup", "Regroup", VerbFamily::Movement, vec![], "none"));

    // ============================================================
    // Engagement (14)
    // ============================================================
    r.verbs.push(verb(
        "advance",
        "Advance",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "press_attack",
        "Press Attack",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "hold_fire",
        "Hold Fire",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "fire_at_will",
        "Fire At Will",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "engage_priority_target",
        "Engage Priority Target",
        VerbFamily::Engagement,
        vec![arg("target", VerbArgKind::Actor, true)],
        "actor",
    ));
    r.verbs.push(verb(
        "disengage",
        "Disengage",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "overwatch_sector",
        "Overwatch (sector)",
        VerbFamily::Engagement,
        vec![arg("sector", VerbArgKind::Sector, true)],
        "area",
    ));
    r.verbs.push(verb(
        "suppress_target",
        "Suppress (target)",
        VerbFamily::Engagement,
        vec![arg("target", VerbArgKind::Actor, true)],
        "actor",
    ));
    r.verbs.push(verb(
        "suppress_window",
        "Suppress (window)",
        VerbFamily::Engagement,
        vec![arg("window", VerbArgKind::Window, true)],
        "window",
    ));
    r.verbs.push(verb(
        "cover_me",
        "Cover Me",
        VerbFamily::Engagement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "cover_that_wall",
        "Cover That Wall",
        VerbFamily::Engagement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "frag_out",
        "Frag-Out",
        VerbFamily::Engagement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "smoke",
        "Smoke",
        VerbFamily::Engagement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "flash",
        "Flash",
        VerbFamily::Engagement,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));

    // ============================================================
    // Movement-to-contact (10)
    // ============================================================
    r.verbs.push(verb(
        "breach_door",
        "Breach (door)",
        VerbFamily::MovementToContact,
        vec![arg("door", VerbArgKind::Door, true)],
        "door",
    ));
    r.verbs.push(verb(
        "stack_door",
        "Stack (door, side)",
        VerbFamily::MovementToContact,
        vec![
            arg("door", VerbArgKind::Door, true),
            arg("side", VerbArgKind::Side, true),
        ],
        "door",
    ));
    r.verbs.push(verb(
        "single_file",
        "Single-File",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "wedge",
        "Wedge",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "echelon_left",
        "Echelon-Left",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "echelon_right",
        "Echelon-Right",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "line_abreast",
        "Line Abreast",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "column",
        "Column",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "diamond",
        "Diamond",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "form_defensive_perimeter",
        "Form Defensive Perimeter",
        VerbFamily::MovementToContact,
        vec![arg("center", VerbArgKind::Waypoint, false)],
        "area",
    ));

    // ============================================================
    // Role-specific (6)
    // ============================================================
    r.verbs.push(verb(
        "sniper_cover",
        "Sniper-Cover",
        VerbFamily::RoleSpecific,
        vec![arg("sector", VerbArgKind::Sector, false)],
        "area",
    ));
    r.verbs.push(verb(
        "heavy_forward",
        "Heavy-Forward",
        VerbFamily::RoleSpecific,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "engineer_up",
        "Engineer-Up",
        VerbFamily::RoleSpecific,
        vec![arg("target", VerbArgKind::Actor, false)],
        "actor",
    ));
    r.verbs.push(verb(
        "medic_up",
        "Medic-Up",
        VerbFamily::RoleSpecific,
        vec![arg("target", VerbArgKind::Actor, false)],
        "actor",
    ));
    r.verbs.push(verb(
        "reinforce",
        "Reinforce",
        VerbFamily::RoleSpecific,
        vec![arg("waypoint", VerbArgKind::Waypoint, true)],
        "area",
    ));
    r.verbs.push(verb(
        "drag_to_cover",
        "Drag To Cover",
        VerbFamily::RoleSpecific,
        vec![arg("target", VerbArgKind::Actor, true)],
        "actor",
    ));

    // ============================================================
    // Logistics (5)
    // ============================================================
    r.verbs.push(verb(
        "pick_up",
        "Pick Up",
        VerbFamily::Logistics,
        vec![arg("item", VerbArgKind::Actor, true)],
        "actor",
    ));
    r.verbs.push(verb(
        "drop",
        "Drop",
        VerbFamily::Logistics,
        vec![arg("item", VerbArgKind::Label, true)],
        "none",
    ));
    r.verbs.push(verb(
        "hand_off",
        "Hand Off",
        VerbFamily::Logistics,
        vec![
            arg("item", VerbArgKind::Label, true),
            arg("recipient", VerbArgKind::Actor, true),
        ],
        "actor",
    ));
    r.verbs.push(verb(
        "reload_up",
        "Reload Up",
        VerbFamily::Logistics,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "top_off_mags",
        "Top Off Mags",
        VerbFamily::Logistics,
        vec![],
        "none",
    ));

    // ============================================================
    // Extension verbs needed to clear the 50+ threshold + cover
    // additional spec hooks (mark, take cover, spread out, form up,
    // storm building, crouch, prone).
    // ============================================================
    r.verbs.push(verb(
        "mark_threat",
        "Mark Threat",
        VerbFamily::Engagement,
        vec![arg("target", VerbArgKind::Actor, true)],
        "actor",
    ));
    r.verbs.push(verb(
        "take_cover",
        "Take Cover",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "spread_out",
        "Spread Out",
        VerbFamily::MovementToContact,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "form_up",
        "Form Up",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "storm_building",
        "Storm Building",
        VerbFamily::MovementToContact,
        vec![arg("door", VerbArgKind::Door, false)],
        "door",
    ));
    r.verbs.push(verb(
        "crouch",
        "Crouch",
        VerbFamily::Movement,
        vec![],
        "none",
    ));
    r.verbs.push(verb(
        "prone",
        "Prone",
        VerbFamily::Movement,
        vec![],
        "none",
    ));

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_size_meets_floor() {
        let r = builtin_registry();
        assert!(
            r.len() >= 50,
            "verb registry contains {} entries; spec floor is 50",
            r.len()
        );
    }

    #[test]
    fn each_family_present() {
        let r = builtin_registry();
        for family in [
            VerbFamily::Movement,
            VerbFamily::Engagement,
            VerbFamily::MovementToContact,
            VerbFamily::RoleSpecific,
            VerbFamily::Logistics,
        ] {
            assert!(
                r.by_family(family).next().is_some(),
                "no verbs in family {:?}",
                family
            );
        }
    }

    #[test]
    fn breach_chain_verbs_present() {
        let r = builtin_registry();
        for id in ["stack_door", "breach_door", "frag_out", "advance"] {
            assert!(r.find(id).is_some(), "missing breach-chain verb {id}");
        }
    }

    #[test]
    fn ron_registry_parses_and_matches_builtin() {
        let src = include_str!("../../../../content/ai/verbs/registry.ron");
        let parsed = VerbRegistry::from_ron(src).expect("registry RON must parse");
        let builtin = builtin_registry();
        assert_eq!(
            parsed.len(),
            builtin.len(),
            "RON entry count {} should match builtin {}",
            parsed.len(),
            builtin.len()
        );
        for def in builtin.iter() {
            assert!(
                parsed.find(&def.verb_id).is_some(),
                "RON registry missing verb {}",
                def.verb_id
            );
        }
    }
}
