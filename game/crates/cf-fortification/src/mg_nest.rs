//! M9C § "MG nest module + ammo box + tripod variant": placeholder for
//! the full surface that lands in feature m9c-2.
//!
//! This module is a deliberate scaffold for m9c-1: it owns the
//! type-shape contract (so the cf-fortification public API can be
//! frozen at workspace-registration time) without implementing the
//! crewing state machine. The state machine, ammo-box auto-feed, and
//! tripod deploy logic ship in feature `m9c-2-mg-nest-suite`.
//!
//! VAL-M9C-002 is the only contract this file satisfies: the
//! submodule MUST exist and MUST be reachable through `lib.rs`.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// Spec table row HP for a built `mg_nest_static`.
pub const MG_NEST_STATIC_MAX_HP: u32 = 800;
/// Spec table row HP for an `ammo_box_mg`.
pub const AMMO_BOX_MG_MAX_HP: u32 = 200;
/// Spec table row HP for a packed `mg_tripod_portable` once deployed.
pub const MG_TRIPOD_DEPLOYED_HP: u32 = 400;
/// Spec table: 800-round belt cache for the canonical ammo box.
pub const AMMO_BOX_MG_ROUNDS: u32 = 800;

/// MG-nest crew-slot placeholder. The full state machine (Crewing
/// stance binding, weapon rebind, ammo-feed) lands in m9c-2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgNest {
    pub id: FortificationId,
    pub hp: u32,
    pub rounds_remaining: u32,
    pub crewed_by: Option<u32>,
}

impl MgNest {
    #[must_use]
    pub fn new_built(id: FortificationId) -> Self {
        Self {
            id,
            hp: MG_NEST_STATIC_MAX_HP,
            rounds_remaining: AMMO_BOX_MG_ROUNDS,
            crewed_by: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mg_nest_static_construction_pins_spec_table_row() {
        let nest = MgNest::new_built(FortificationId(42));
        assert_eq!(nest.hp, MG_NEST_STATIC_MAX_HP);
        assert_eq!(nest.rounds_remaining, AMMO_BOX_MG_ROUNDS);
        assert!(nest.crewed_by.is_none());
    }
}
