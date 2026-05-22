use serde::{Deserialize, Serialize};

use crate::ChassisKind;

/// abilities". Eight launch abilities + per-chassis slot count.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisAbility {
    TimeStop = 0,
    TimeSlow = 1,
    ShieldBurst = 2,
    Overdrive = 3,
    RepairPulse = 4,
    Cloak = 5,
    EmpPulse = 6,
    GravityWell = 7,
}

impl ChassisAbility {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisAbility::TimeStop => "time_stop",
            ChassisAbility::TimeSlow => "time_slow",
            ChassisAbility::ShieldBurst => "shield_burst",
            ChassisAbility::Overdrive => "overdrive",
            ChassisAbility::RepairPulse => "repair_pulse",
            ChassisAbility::Cloak => "cloak",
            ChassisAbility::EmpPulse => "EMP_pulse",
            ChassisAbility::GravityWell => "gravity_well",
        }
    }

    pub fn parse(s: &str) -> Option<ChassisAbility> {
        match s {
            "time_stop" => Some(ChassisAbility::TimeStop),
            "time_slow" => Some(ChassisAbility::TimeSlow),
            "shield_burst" => Some(ChassisAbility::ShieldBurst),
            "overdrive" => Some(ChassisAbility::Overdrive),
            "repair_pulse" => Some(ChassisAbility::RepairPulse),
            "cloak" => Some(ChassisAbility::Cloak),
            "EMP_pulse" | "emp_pulse" => Some(ChassisAbility::EmpPulse),
            "gravity_well" => Some(ChassisAbility::GravityWell),
            _ => None,
        }
    }

    pub fn defaults(self) -> (f32, f32) {
        match self {
            ChassisAbility::TimeStop => (1.5, 30.0),
            ChassisAbility::TimeSlow => (5.0, 25.0),
            ChassisAbility::ShieldBurst => (8.0, 20.0),
            ChassisAbility::Overdrive => (6.0, 30.0),
            ChassisAbility::RepairPulse => (0.1, 45.0),
            ChassisAbility::Cloak => (5.0, 60.0),
            ChassisAbility::EmpPulse => (4.0, 40.0),
            ChassisAbility::GravityWell => (4.0, 50.0),
        }
    }

    /// Canonical iteration order for events + checksums.
    pub fn all() -> &'static [ChassisAbility] {
        &[
            ChassisAbility::TimeStop,
            ChassisAbility::TimeSlow,
            ChassisAbility::ShieldBurst,
            ChassisAbility::Overdrive,
            ChassisAbility::RepairPulse,
            ChassisAbility::Cloak,
            ChassisAbility::EmpPulse,
            ChassisAbility::GravityWell,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbilitySlotState {
    pub ability: ChassisAbility,
    /// Ticks remaining on the cooldown (0 = ready).
    pub cooldown_remaining_ticks: u32,
    /// Cooldown duration in ticks at the chassis tick rate.
    pub cooldown_total_ticks: u32,
    /// Ticks remaining in the active effect window (0 = effect ended).
    pub effect_remaining_ticks: u32,
    /// Effect duration in ticks at the chassis tick rate.
    pub effect_total_ticks: u32,
}

impl AbilitySlotState {
    pub fn new(ability: ChassisAbility, tick_rate_hz: u32) -> Self {
        let (effect_s, cooldown_s) = ability.defaults();
        let tr = tick_rate_hz.max(1) as f32;
        Self {
            ability,
            cooldown_remaining_ticks: 0,
            cooldown_total_ticks: (cooldown_s * tr).round() as u32,
            effect_remaining_ticks: 0,
            effect_total_ticks: (effect_s * tr).round() as u32,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.cooldown_remaining_ticks == 0 && self.effect_remaining_ticks == 0
    }

    pub fn is_active(&self) -> bool {
        self.effect_remaining_ticks > 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChassisAbilitySlots {
    /// Maximum ability slot count for this chassis (derived from kind).
    pub max_slots: u8,
    /// Active slot roster (length ≤ max_slots).
    pub slots: Vec<AbilitySlotState>,
}

impl ChassisAbilitySlots {
    pub fn new(kind: ChassisKind, tick_rate_hz: u32) -> Self {
        let max_slots = kind.ability_slot_count();
        // Default loadout per chassis kind. Light mech = 3 most-versatile slots;
        // powered armor = 2; infantry = 1; drone = 1; crab = 2.
        let default_loadout: &[ChassisAbility] = match kind {
            ChassisKind::Infantry => &[ChassisAbility::ShieldBurst],
            ChassisKind::PoweredArmor => &[ChassisAbility::Overdrive, ChassisAbility::ShieldBurst],
            ChassisKind::LightMech => &[ChassisAbility::TimeSlow, ChassisAbility::Overdrive, ChassisAbility::ShieldBurst],
            ChassisKind::CrabQuadruped => &[ChassisAbility::ShieldBurst, ChassisAbility::EmpPulse],
            ChassisKind::Drone => &[ChassisAbility::Cloak],
            ChassisKind::HeavyTrooper => &[
                ChassisAbility::ShieldBurst,
                ChassisAbility::Overdrive,
                ChassisAbility::EmpPulse,
            ],
        };
        let slots: Vec<AbilitySlotState> = default_loadout
            .iter()
            .take(max_slots as usize)
            .map(|a| AbilitySlotState::new(*a, tick_rate_hz))
            .collect();
        Self { max_slots, slots }
    }

    /// Find a slot by ability kind.
    pub fn find(&self, ability: ChassisAbility) -> Option<&AbilitySlotState> {
        self.slots.iter().find(|s| s.ability == ability)
    }

    pub fn find_mut(&mut self, ability: ChassisAbility) -> Option<&mut AbilitySlotState> {
        self.slots.iter_mut().find(|s| s.ability == ability)
    }

    /// Tick every slot's cooldown + effect timers. Returns the abilities whose
    /// effect window ended this tick (for `ability.effect_ended` events) and
    /// the abilities whose cooldown ended this tick (for `ability.cooldown_expired`).
    pub fn tick(&mut self) -> AbilityTickOutcome {
        let mut outcome = AbilityTickOutcome::default();
        for slot in &mut self.slots {
            if slot.effect_remaining_ticks > 0 {
                slot.effect_remaining_ticks -= 1;
                if slot.effect_remaining_ticks == 0 {
                    outcome.effects_ended.push(slot.ability);
                    // Effect-ended starts the cooldown.
                    slot.cooldown_remaining_ticks = slot.cooldown_total_ticks;
                }
            } else if slot.cooldown_remaining_ticks > 0 {
                slot.cooldown_remaining_ticks -= 1;
                if slot.cooldown_remaining_ticks == 0 {
                    outcome.cooldowns_expired.push(slot.ability);
                }
            }
        }
        outcome
    }

    /// Attempt to activate `ability`. Returns `Ok(slot_state)` on success or
    /// the typed reason on rejection.
    pub fn activate(&mut self, ability: ChassisAbility) -> Result<AbilitySlotState, AbilityRejectReason> {
        let slot = self.find_mut(ability).ok_or(AbilityRejectReason::NotEquipped)?;
        if slot.cooldown_remaining_ticks > 0 {
            return Err(AbilityRejectReason::OnCooldown);
        }
        if slot.effect_remaining_ticks > 0 {
            return Err(AbilityRejectReason::AlreadyActive);
        }
        slot.effect_remaining_ticks = slot.effect_total_ticks.max(1);
        Ok(*slot)
    }
}

/// Per-tick outcome of [`ChassisAbilitySlots::tick`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbilityTickOutcome {
    pub effects_ended: Vec<ChassisAbility>,
    pub cooldowns_expired: Vec<ChassisAbility>,
}

/// Typed rejection reasons surfaced by [`ChassisAbilitySlots::activate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityRejectReason {
    NotEquipped,
    OnCooldown,
    AlreadyActive,
}

impl AbilityRejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            AbilityRejectReason::NotEquipped => "ability_not_equipped",
            AbilityRejectReason::OnCooldown => "ability_on_cooldown",
            AbilityRejectReason::AlreadyActive => "ability_already_active",
        }
    }
}
