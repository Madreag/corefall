//! M17 — actor personal-power network (the actor-side of the M29 grid).
//!
//! Each actor has a personal power network: a worn battery pack, an equipment
//! draw, and a priority chain that sheds low-priority loads when the battery
//! runs low. Humans' bodies are unaffected by battery state (only equipment);
//! robots' bodies are power-survival (empty battery = INERT, recoverable).

use serde::{Deserialize, Serialize};

use crate::origin::Origin;

/// Battery-pack tiers (the 4-tier ladder canonically owned by M29; M17
/// declares the per-origin need + ships the tier shape so the actor model is
/// self-contained).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryTier {
    /// T1 — small lithium-ion.
    SmallLithium,
    /// T2 — standard lithium-ion.
    StandardLithium,
    /// T3 — heavy-duty reactor battery.
    ReactorBattery,
    /// T4 — superconductor capacitor pack.
    Superconductor,
}

impl BatteryTier {
    pub fn as_str(self) -> &'static str {
        match self {
            BatteryTier::SmallLithium => "small_lithium",
            BatteryTier::StandardLithium => "standard_lithium",
            BatteryTier::ReactorBattery => "reactor_battery",
            BatteryTier::Superconductor => "superconductor",
        }
    }

    pub fn capacity_kwh(self) -> f32 {
        match self {
            BatteryTier::SmallLithium => 1.5,
            BatteryTier::StandardLithium => 5.0,
            BatteryTier::ReactorBattery => 20.0,
            BatteryTier::Superconductor => 50.0,
        }
    }

    /// Max sustained discharge in kW (drives the priority-shedding cap).
    pub fn discharge_max_kw(self) -> f32 {
        match self {
            BatteryTier::SmallLithium => 1.5,
            BatteryTier::StandardLithium => 5.0,
            BatteryTier::ReactorBattery => 15.0,
            BatteryTier::Superconductor => 40.0,
        }
    }

    pub fn weight_kg(self) -> f32 {
        match self {
            BatteryTier::SmallLithium => 2.0,
            BatteryTier::StandardLithium => 6.0,
            BatteryTier::ReactorBattery => 18.0,
            BatteryTier::Superconductor => 12.0,
        }
    }

    /// T1-T2 hot-swap in the field; T3+ needs a base.
    pub fn hot_swap_capable(self) -> bool {
        matches!(self, BatteryTier::SmallLithium | BatteryTier::StandardLithium)
    }
}

/// A worn battery pack.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BatteryPack {
    pub tier: BatteryTier,
    pub current_charge_kwh: f32,
    pub health_pct: f32,
}

impl BatteryPack {
    pub fn full(tier: BatteryTier) -> Self {
        Self {
            tier,
            current_charge_kwh: tier.capacity_kwh(),
            health_pct: 100.0,
        }
    }

    pub fn charge_fraction(&self) -> f32 {
        let cap = self.tier.capacity_kwh();
        if cap <= 0.0 {
            0.0
        } else {
            (self.current_charge_kwh / cap).clamp(0.0, 1.0)
        }
    }

    pub fn swap_full(&mut self) {
        self.current_charge_kwh = self.tier.capacity_kwh();
    }
}

/// Equipment power-priority class (lowest priority shed first on low battery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerPriority {
    /// Shed first.
    Utility = 0,
    Important = 1,
    /// Never shed (life / locomotion).
    Critical = 2,
}

impl PowerPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            PowerPriority::Utility => "utility",
            PowerPriority::Important => "important",
            PowerPriority::Critical => "critical",
        }
    }
}

/// The actor's personal power network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorPower {
    pub primary_battery: Option<BatteryPack>,
    pub backup_battery: Option<BatteryPack>,
    /// Robots only: chassis-internal storage (kWh).
    pub internal_storage_kwh: f32,
    /// Current sum of all powered-equipment draw (watts).
    pub equipment_draw_w: f32,
    /// Inbound charge rate (watts) — positive on a charging pad / solar trickle.
    pub charge_rate_inbound_w: f32,
    /// Equipment shed this tick due to low power.
    pub shedded_count: u32,
}

impl Default for ActorPower {
    fn default() -> Self {
        Self {
            primary_battery: None,
            backup_battery: None,
            internal_storage_kwh: 0.0,
            equipment_draw_w: 0.0,
            charge_rate_inbound_w: 0.0,
            shedded_count: 0,
        }
    }
}

impl ActorPower {
    /// Default personal-power network for an origin: robots carry a standard
    /// battery; humans / androids carry a small one for equipment; biomech
    /// none.
    pub fn for_origin(origin: Origin) -> Self {
        let battery = match origin {
            Origin::Robot => Some(BatteryPack::full(BatteryTier::StandardLithium)),
            Origin::Drone | Origin::Crystalline => Some(BatteryPack::full(BatteryTier::SmallLithium)),
            Origin::Human | Origin::Android | Origin::PoweredOrganic => {
                Some(BatteryPack::full(BatteryTier::SmallLithium))
            }
            _ => None,
        };
        let internal = if origin == Origin::Robot { 10.0 } else { 0.0 };
        Self {
            primary_battery: battery,
            internal_storage_kwh: internal,
            ..Self::default()
        }
    }

    /// Total available charge across all sources (kWh).
    pub fn total_charge_kwh(&self) -> f32 {
        self.internal_storage_kwh
            + self.primary_battery.map_or(0.0, |b| b.current_charge_kwh)
            + self.backup_battery.map_or(0.0, |b| b.current_charge_kwh)
    }

    /// Time (minutes) until the battery empties at the current net draw, or
    /// `None` if charging / no draw.
    pub fn time_to_empty_minutes(&self) -> Option<f32> {
        let net_w = self.equipment_draw_w - self.charge_rate_inbound_w;
        if net_w <= 0.0 {
            return None;
        }
        let kwh = self.total_charge_kwh();
        // kWh / kW = hours; convert to minutes.
        Some((kwh / (net_w / 1000.0)) * 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_tiers_scale_capacity() {
        assert!(BatteryTier::SmallLithium.capacity_kwh() < BatteryTier::StandardLithium.capacity_kwh());
        assert!(BatteryTier::ReactorBattery.capacity_kwh() < BatteryTier::Superconductor.capacity_kwh());
        assert!(BatteryTier::SmallLithium.hot_swap_capable());
        assert!(!BatteryTier::ReactorBattery.hot_swap_capable());
    }

    #[test]
    fn robot_carries_standard_battery_plus_internal() {
        let p = ActorPower::for_origin(Origin::Robot);
        assert!(p.primary_battery.is_some());
        assert!(p.internal_storage_kwh > 0.0);
        assert!(p.total_charge_kwh() > 0.0);
    }

    #[test]
    fn biomech_carries_no_battery() {
        let p = ActorPower::for_origin(Origin::HeavyBiomech);
        assert!(p.primary_battery.is_none());
        assert_eq!(p.total_charge_kwh(), 0.0);
    }

    #[test]
    fn time_to_empty_is_none_when_charging() {
        let mut p = ActorPower::for_origin(Origin::Robot);
        p.equipment_draw_w = 100.0;
        p.charge_rate_inbound_w = 200.0;
        assert!(p.time_to_empty_minutes().is_none());
        p.charge_rate_inbound_w = 0.0;
        assert!(p.time_to_empty_minutes().unwrap() > 0.0);
    }
}
