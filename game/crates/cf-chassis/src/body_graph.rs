use serde::{Deserialize, Serialize};

use crate::BodyZone;

/// One joint in the body graph. Joints connect zones and propagate physical
/// disruption (e.g., destroying the elbow joint between `arm_right` and the hand
/// means the hand can no longer grip). M5 ships a fixed set for the launch
/// chassis; M5.5 collision filters reference joint names for parent-linked
/// limb-collision events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub id: String,
    pub parent: BodyZone,
    pub child: BodyZone,
    /// True iff this joint is intact. Destroying the parent zone severs the
    /// joint; the runtime updates this on `apply_zone_damage`.
    pub intact: bool,
}

/// Equipment socket on the body graph. Sockets are mount points where role-record
/// items are attached. Each socket is bound to a zone so dropping the zone drops
/// the gear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentSocket {
    pub id: String,
    pub zone: BodyZone,
    /// True iff a piece of equipment is currently mounted at the socket.
    pub occupied: bool,
    /// Role-record id of the mounted equipment, if any. M5 reuses
    /// `RIFLE_M1_DEFAULT_ID` for the canonical rifle socket; mods can add new.
    pub mounted_role: Option<String>,
}

/// Per-limb movement-contribution flags. When the zone is destroyed the flags say
/// what the actor loses (jet, jump, climb, two-handed grip, aim stability).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementContribution {
    pub zone: BodyZone,
    /// Multiplicative factor on movement speed if zone destroyed (1.0 = no impact).
    pub move_speed_factor_when_destroyed: f32,
    /// Multiplicative factor on jump impulse if zone destroyed.
    pub jump_impulse_factor_when_destroyed: f32,
    /// Whether destroying this zone disables the rifle (e.g., right arm gone).
    pub disables_rifle_when_destroyed: bool,
    /// Whether destroying this zone forces a crawl state.
    pub forces_crawl_when_destroyed: bool,
    /// Whether destroying this zone drops carried gear.
    pub drops_gear_when_destroyed: bool,
    /// Whether destroying this zone disables jet/jump (e.g., backpack housing the
    /// jet module, or legs gone).
    pub disables_jet_when_destroyed: bool,
}

impl MovementContribution {
    pub fn neutral(zone: BodyZone) -> Self {
        Self {
            zone,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        }
    }
}

/// Body graph for a chassis. Lists every zone, joint, socket, and movement
/// contribution. The runtime walks this graph to resolve animation events and
/// damage consequences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyGraph {
    pub zones: Vec<BodyZone>,
    pub joints: Vec<Joint>,
    pub sockets: Vec<EquipmentSocket>,
    pub movement_contributions: Vec<MovementContribution>,
}

impl BodyGraph {
    pub fn movement_factor(&self, destroyed_zones: &[BodyZone]) -> (f32, f32, bool, bool, bool, bool) {
        // Returns (move_speed_factor, jump_factor, disable_rifle, force_crawl, drop_gear, disable_jet).
        let mut move_factor: f32 = 1.0;
        let mut jump_factor: f32 = 1.0;
        let mut disable_rifle = false;
        let mut force_crawl = false;
        let mut drop_gear = false;
        let mut disable_jet = false;
        for zone in destroyed_zones {
            if let Some(c) = self.movement_contributions.iter().find(|c| c.zone == *zone) {
                move_factor = move_factor.min(c.move_speed_factor_when_destroyed);
                jump_factor = jump_factor.min(c.jump_impulse_factor_when_destroyed);
                disable_rifle = disable_rifle || c.disables_rifle_when_destroyed;
                force_crawl = force_crawl || c.forces_crawl_when_destroyed;
                drop_gear = drop_gear || c.drops_gear_when_destroyed;
                disable_jet = disable_jet || c.disables_jet_when_destroyed;
            }
        }
        (
            move_factor,
            jump_factor,
            disable_rifle,
            force_crawl,
            drop_gear,
            disable_jet,
        )
    }
}

/// Eject window state. Eject is a multi-tick sequence: triggered at tick `T`, the
/// chassis spends `eject_ticks` blowing the canopy and clearing the pilot, then
/// transitions Ejected → Extracted when the pilot reaches a safe spot (engine
/// drives the extraction check).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EjectWindow {
    /// Ticks remaining in the active eject sequence. `0` = not ejecting OR completed.
    pub ticks_remaining: u32,
    /// Ticks the full eject takes once triggered. Defaults to 1 second at 60 Hz.
    pub ticks_total: u32,
    /// Tick at which the eject was triggered. Used by replay for event ordering.
    pub triggered_at_tick: u64,
}

impl Default for EjectWindow {
    fn default() -> Self {
        Self {
            ticks_remaining: 0,
            ticks_total: 60,
            triggered_at_tick: 0,
        }
    }
}
