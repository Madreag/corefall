use serde::{Deserialize, Serialize};

use crate::ChassisKind;

/// modifiers stackable on the same weapon. Discriminants are stable so the
/// modifier registry remains deterministic across milestones.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponModifier {
    Homing = 0,
    Explosive = 1,
    Freezing = 2,
    Electric = 3,
    Poisoning = 4,
    Bouncing = 5,
    Piercing = 6,
    Ricochet = 7,
    Bleed = 8,
    Stun = 9,
    FireChain = 10,
    DoubleTap = 11,
    TripleShot = 12,
    FastFire = 13,
    SlowFire = 14,
    SlowMotionOnKill = 15,
    SummonMinion = 16,
    GravityWell = 17,
    Vortex = 18,
    Magnet = 19,
    TimeSlowOnHit = 20,
    HealingBurst = 21,
    LifeSteal = 22,
    ManaBurst = 23,
    ShieldBreak = 24,
    ArmorPiercingRandom = 25,
    Knockback = 26,
    Weighted = 27,
    Magnetic = 28,
    ChainLightning = 29,
    FrostAura = 30,
}

impl WeaponModifier {
    pub fn as_str(self) -> &'static str {
        match self {
            WeaponModifier::Homing => "homing",
            WeaponModifier::Explosive => "explosive",
            WeaponModifier::Freezing => "freezing",
            WeaponModifier::Electric => "electric",
            WeaponModifier::Poisoning => "poisoning",
            WeaponModifier::Bouncing => "bouncing",
            WeaponModifier::Piercing => "piercing",
            WeaponModifier::Ricochet => "ricochet",
            WeaponModifier::Bleed => "bleed",
            WeaponModifier::Stun => "stun",
            WeaponModifier::FireChain => "fire_chain",
            WeaponModifier::DoubleTap => "double_tap",
            WeaponModifier::TripleShot => "triple_shot",
            WeaponModifier::FastFire => "fast_fire",
            WeaponModifier::SlowFire => "slow_fire",
            WeaponModifier::SlowMotionOnKill => "slow_motion_on_kill",
            WeaponModifier::SummonMinion => "summon_minion",
            WeaponModifier::GravityWell => "gravity_well",
            WeaponModifier::Vortex => "vortex",
            WeaponModifier::Magnet => "magnet",
            WeaponModifier::TimeSlowOnHit => "time_slow_on_hit",
            WeaponModifier::HealingBurst => "healing_burst",
            WeaponModifier::LifeSteal => "life_steal",
            WeaponModifier::ManaBurst => "mana_burst",
            WeaponModifier::ShieldBreak => "shield_break",
            WeaponModifier::ArmorPiercingRandom => "armor_piercing_random",
            WeaponModifier::Knockback => "knockback",
            WeaponModifier::Weighted => "weighted",
            WeaponModifier::Magnetic => "magnetic",
            WeaponModifier::ChainLightning => "chain_lightning",
            WeaponModifier::FrostAura => "frost_aura",
        }
    }

    pub fn parse(s: &str) -> Option<WeaponModifier> {
        for m in WeaponModifier::all() {
            if m.as_str() == s {
                return Some(*m);
            }
        }
        None
    }

    pub fn all() -> &'static [WeaponModifier] {
        &[
            WeaponModifier::Homing,
            WeaponModifier::Explosive,
            WeaponModifier::Freezing,
            WeaponModifier::Electric,
            WeaponModifier::Poisoning,
            WeaponModifier::Bouncing,
            WeaponModifier::Piercing,
            WeaponModifier::Ricochet,
            WeaponModifier::Bleed,
            WeaponModifier::Stun,
            WeaponModifier::FireChain,
            WeaponModifier::DoubleTap,
            WeaponModifier::TripleShot,
            WeaponModifier::FastFire,
            WeaponModifier::SlowFire,
            WeaponModifier::SlowMotionOnKill,
            WeaponModifier::SummonMinion,
            WeaponModifier::GravityWell,
            WeaponModifier::Vortex,
            WeaponModifier::Magnet,
            WeaponModifier::TimeSlowOnHit,
            WeaponModifier::HealingBurst,
            WeaponModifier::LifeSteal,
            WeaponModifier::ManaBurst,
            WeaponModifier::ShieldBreak,
            WeaponModifier::ArmorPiercingRandom,
            WeaponModifier::Knockback,
            WeaponModifier::Weighted,
            WeaponModifier::Magnetic,
            WeaponModifier::ChainLightning,
            WeaponModifier::FrostAura,
        ]
    }
}

/// chassis tier's slot count (see `ChassisKind::weapon_modifier_slot_count`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeaponModifierSet {
    pub max_slots: u8,
    pub modifiers: Vec<WeaponModifier>,
}

impl WeaponModifierSet {
    pub fn new(kind: ChassisKind) -> Self {
        Self {
            max_slots: kind.weapon_modifier_slot_count(),
            modifiers: Vec::new(),
        }
    }

    pub fn attach(&mut self, m: WeaponModifier) -> Result<(), &'static str> {
        if self.modifiers.contains(&m) {
            return Err("modifier_already_attached");
        }
        if self.modifiers.len() as u8 >= self.max_slots {
            return Err("modifier_slots_full");
        }
        self.modifiers.push(m);
        Ok(())
    }

    pub fn detach(&mut self, m: WeaponModifier) -> bool {
        if let Some(idx) = self.modifiers.iter().position(|x| *x == m) {
            self.modifiers.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, m: WeaponModifier) -> bool {
        self.modifiers.contains(&m)
    }

    pub fn is_combined(&self) -> bool {
        self.modifiers.len() >= 2
    }
}
