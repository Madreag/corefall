//! M7B: 9 formation kinds + slot solver + local→world transforms.
//!
//! Spec § "9 formation kinds with per-actor slot resolution: `Wedge`,
//! `Diamond`, `Column`, `Line Abreast`, `Echelon-Left`, `Echelon-Right`,
//! `Single-File`, `Stack (door)`, `Defensive Perimeter`. Each formation has
//! slot count, per-slot relative-position vector in commander-facing local
//! space, and per-slot role hint."
//!
//! Slot solver runs at issue + every 2s while moving; collapses gracefully
//! on member loss (Wedge → Diamond → Column → Single-File).

use serde::{Deserialize, Serialize};

pub mod slot_solver;
pub mod transforms;

pub use slot_solver::{SlotAssignment, SlotSolver};
pub use transforms::{rotate_local_to_world, world_anchor_for_slot};

/// collapse chain Wedge → Diamond → Column → Single-File when a slot is
/// orphaned by member loss.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationKind {
    Wedge,
    Diamond,
    Column,
    LineAbreast,
    EchelonLeft,
    EchelonRight,
    SingleFile,
    StackDoor,
    DefensivePerimeter,
}

impl FormationKind {
    pub const ALL: [FormationKind; 9] = [
        FormationKind::Wedge,
        FormationKind::Diamond,
        FormationKind::Column,
        FormationKind::LineAbreast,
        FormationKind::EchelonLeft,
        FormationKind::EchelonRight,
        FormationKind::SingleFile,
        FormationKind::StackDoor,
        FormationKind::DefensivePerimeter,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            FormationKind::Wedge => "wedge",
            FormationKind::Diamond => "diamond",
            FormationKind::Column => "column",
            FormationKind::LineAbreast => "line_abreast",
            FormationKind::EchelonLeft => "echelon_left",
            FormationKind::EchelonRight => "echelon_right",
            FormationKind::SingleFile => "single_file",
            FormationKind::StackDoor => "stack_door",
            FormationKind::DefensivePerimeter => "defensive_perimeter",
        }
    }

    pub fn from_str(value: &str) -> Option<FormationKind> {
        Some(match value {
            "wedge" => FormationKind::Wedge,
            "diamond" => FormationKind::Diamond,
            "column" => FormationKind::Column,
            "line_abreast" => FormationKind::LineAbreast,
            "echelon_left" => FormationKind::EchelonLeft,
            "echelon_right" => FormationKind::EchelonRight,
            "single_file" => FormationKind::SingleFile,
            "stack_door" => FormationKind::StackDoor,
            "defensive_perimeter" => FormationKind::DefensivePerimeter,
            _ => return None,
        })
    }

    /// Spec-mandated collapse step: if `self` cannot be filled by the
    /// surviving member count, fall back through Wedge → Diamond → Column
    /// → Single-File. Other formations collapse to Single-File directly.
    pub fn collapse_step(self) -> Option<FormationKind> {
        match self {
            FormationKind::Wedge => Some(FormationKind::Diamond),
            FormationKind::Diamond => Some(FormationKind::Column),
            FormationKind::Column => Some(FormationKind::SingleFile),
            FormationKind::LineAbreast
            | FormationKind::EchelonLeft
            | FormationKind::EchelonRight
            | FormationKind::StackDoor
            | FormationKind::DefensivePerimeter => Some(FormationKind::SingleFile),
            FormationKind::SingleFile => None,
        }
    }
}

/// the engine reassigns on KIA / brain-hop.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadRoleHint {
    SquadLeader,
    Pointman,
    Rifleman,
    Marksman,
    Heavy,
    Engineer,
    Medic,
}

impl SquadRoleHint {
    pub const ALL: [SquadRoleHint; 7] = [
        SquadRoleHint::SquadLeader,
        SquadRoleHint::Pointman,
        SquadRoleHint::Rifleman,
        SquadRoleHint::Marksman,
        SquadRoleHint::Heavy,
        SquadRoleHint::Engineer,
        SquadRoleHint::Medic,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SquadRoleHint::SquadLeader => "squad_leader",
            SquadRoleHint::Pointman => "pointman",
            SquadRoleHint::Rifleman => "rifleman",
            SquadRoleHint::Marksman => "marksman",
            SquadRoleHint::Heavy => "heavy",
            SquadRoleHint::Engineer => "engineer",
            SquadRoleHint::Medic => "medic",
        }
    }

    pub fn from_str(value: &str) -> Option<SquadRoleHint> {
        Some(match value {
            "squad_leader" => SquadRoleHint::SquadLeader,
            "pointman" => SquadRoleHint::Pointman,
            "rifleman" => SquadRoleHint::Rifleman,
            "marksman" => SquadRoleHint::Marksman,
            "heavy" => SquadRoleHint::Heavy,
            "engineer" => SquadRoleHint::Engineer,
            "medic" => SquadRoleHint::Medic,
            _ => return None,
        })
    }
}

/// space (x forward, y left). Roll-up sectors-of-fire are advisory metadata
/// the BT consumes when expanding Breach / Stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationSlot {
    pub slot_id: u32,
    pub offset: [f32; 2],
    pub role_hint: SquadRoleHint,
    /// Sector-of-fire bearing (degrees, 0=forward, ccw positive).
    pub sector_bearing_degrees: f32,
}

/// `game/content/ai/formations/<kind>.ron` at boot OR via `builtin(kind)`
/// for tests / headless paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationDef {
    pub kind: FormationKind,
    pub slots: Vec<FormationSlot>,
}

impl FormationDef {
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Convert RON content into a `FormationDef`.
    pub fn from_ron(src: &str) -> Result<Self, String> {
        ron::from_str(src).map_err(|e| format!("ron parse failed: {e}"))
    }

    /// Spec-mandated default formation. Coordinates are world units; the
    /// commander is at the origin and faces +x. y is "left" in commander
    /// local space. Each definition packs at least 5 slots so the
    /// spec-mandated 5-member squad fits.
    pub fn builtin(kind: FormationKind) -> FormationDef {
        match kind {
            FormationKind::Wedge => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-6.0, 4.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 30.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-6.0, -4.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -30.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-12.0, 8.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 60.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-12.0, -8.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: -60.0,
                    },
                ],
            },
            FormationKind::Diamond => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [6.0, 0.0],
                        role_hint: SquadRoleHint::Pointman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [0.0, 6.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 90.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [0.0, -6.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -90.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-6.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 180.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-12.0, 0.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 180.0,
                    },
                ],
            },
            FormationKind::Column => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-6.0, 0.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-12.0, 0.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 90.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-18.0, 0.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: -90.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-24.0, 0.0],
                        role_hint: SquadRoleHint::Marksman,
                        sector_bearing_degrees: 180.0,
                    },
                ],
            },
            FormationKind::LineAbreast => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, -8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [0.0, -4.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [0.0, 4.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [0.0, 8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 0.0,
                    },
                ],
            },
            FormationKind::EchelonLeft => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-4.0, 4.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 30.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-8.0, 8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 45.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-12.0, 12.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 60.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-16.0, 16.0],
                        role_hint: SquadRoleHint::Marksman,
                        sector_bearing_degrees: 75.0,
                    },
                ],
            },
            FormationKind::EchelonRight => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-4.0, -4.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -30.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-8.0, -8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -45.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-12.0, -12.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: -60.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-16.0, -16.0],
                        role_hint: SquadRoleHint::Marksman,
                        sector_bearing_degrees: -75.0,
                    },
                ],
            },
            FormationKind::SingleFile => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::Pointman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-3.0, 0.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-6.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-9.0, 0.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-12.0, 0.0],
                        role_hint: SquadRoleHint::Marksman,
                        sector_bearing_degrees: 180.0,
                    },
                ],
            },
            FormationKind::StackDoor => FormationDef {
                kind,
                // Stack-1 ahead of the jamb, Stack-2 left, Stack-3 right,
                // Stack-4 rear; offsets are along the door's local x-axis.
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::Pointman,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-2.0, 1.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 90.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [-2.0, -1.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -90.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [-4.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 180.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [-6.0, 0.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 180.0,
                    },
                ],
            },
            FormationKind::DefensivePerimeter => FormationDef {
                kind,
                slots: vec![
                    FormationSlot {
                        slot_id: 0,
                        offset: [8.0, 0.0],
                        role_hint: SquadRoleHint::SquadLeader,
                        sector_bearing_degrees: 0.0,
                    },
                    FormationSlot {
                        slot_id: 1,
                        offset: [-8.0, 0.0],
                        role_hint: SquadRoleHint::Heavy,
                        sector_bearing_degrees: 180.0,
                    },
                    FormationSlot {
                        slot_id: 2,
                        offset: [0.0, 8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: 90.0,
                    },
                    FormationSlot {
                        slot_id: 3,
                        offset: [0.0, -8.0],
                        role_hint: SquadRoleHint::Rifleman,
                        sector_bearing_degrees: -90.0,
                    },
                    FormationSlot {
                        slot_id: 4,
                        offset: [0.0, 0.0],
                        role_hint: SquadRoleHint::Marksman,
                        sector_bearing_degrees: 0.0,
                    },
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_formation_kinds() {
        assert_eq!(FormationKind::ALL.len(), 9);
    }

    #[test]
    fn builtin_each_kind_has_at_least_5_slots() {
        for kind in FormationKind::ALL {
            let f = FormationDef::builtin(kind);
            assert!(f.slot_count() >= 5, "{kind:?} has only {} slots", f.slot_count());
        }
    }

    #[test]
    fn collapse_chain_terminates_at_single_file() {
        let mut kind = FormationKind::Wedge;
        let mut steps = 0;
        while let Some(next) = kind.collapse_step() {
            kind = next;
            steps += 1;
            assert!(steps < 10, "collapse chain runaway");
        }
        assert_eq!(kind, FormationKind::SingleFile);
        assert!(steps >= 3, "wedge → diamond → column → single-file");
    }

    #[test]
    fn round_trip_str() {
        for kind in FormationKind::ALL {
            assert_eq!(FormationKind::from_str(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn role_hint_round_trip() {
        for r in SquadRoleHint::ALL {
            assert_eq!(SquadRoleHint::from_str(r.as_str()), Some(r));
        }
    }

    #[test]
    fn builtin_round_trips_through_ron() {
        for kind in FormationKind::ALL {
            let def = FormationDef::builtin(kind);
            let src = ron::to_string(&def).expect("serialize");
            let parsed = FormationDef::from_ron(&src).expect("parse");
            assert_eq!(parsed, def);
        }
    }

    #[test]
    fn ron_content_files_parse() {
        // The RON files under game/content/ai/formations/ mirror the
        // builtin defs. Exercising the parser here keeps the content
        // path honest without requiring a runtime filesystem hop.
        for (kind, src) in [
            (FormationKind::Wedge, include_str!("../../../content/ai/formations/wedge.ron")),
            (FormationKind::Diamond, include_str!("../../../content/ai/formations/diamond.ron")),
            (FormationKind::Column, include_str!("../../../content/ai/formations/column.ron")),
            (
                FormationKind::LineAbreast,
                include_str!("../../../content/ai/formations/line_abreast.ron"),
            ),
            (
                FormationKind::EchelonLeft,
                include_str!("../../../content/ai/formations/echelon_left.ron"),
            ),
            (
                FormationKind::EchelonRight,
                include_str!("../../../content/ai/formations/echelon_right.ron"),
            ),
            (
                FormationKind::SingleFile,
                include_str!("../../../content/ai/formations/single_file.ron"),
            ),
            (
                FormationKind::StackDoor,
                include_str!("../../../content/ai/formations/stack_door.ron"),
            ),
            (
                FormationKind::DefensivePerimeter,
                include_str!("../../../content/ai/formations/defensive_perimeter.ron"),
            ),
        ] {
            let def = FormationDef::from_ron(src).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            assert_eq!(def, FormationDef::builtin(kind));
        }
    }
}
