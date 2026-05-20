//! **M14I** § per-actor long-term-consequence aggregate.
//!
//! Stored on [`crate::ActorState`] as a single struct, serialized once
//! per save tick. Owns:
//! - scar timeline (cf-scar)
//! - biological age clock (cf-aging)
//! - installed prosthetics (cf-prosthetic)
//! - per-actor trait set (cf-actor::traits)
//! - aggregated functional debuffs (pre-resolved on scar acquisition)
//! - concussion-count tracker for the memory_loss thresholds
//! - phantom-limb tracking + per-week panic timer
//! - chronic-pain baseline
//! - long-term radiation dose accumulator (M16B handoff)
//!
//! Determinism: all RNG flows through cf-aging's seeded
//! `Xoshiro256StarStar` resolver — never `thread_rng()`.

use std::collections::BTreeMap;

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

use cf_aging::BiologicalAge;
use cf_prosthetic::ProstheticInstance;
use cf_scar::ScarTimeline;
use cf_wound::registry::ZoneId;

use crate::traits::{ids as trait_ids, TraitSet};

/// Concussion threshold (KO count) for MemoryLossMinor. Spec § Tunables.
pub const MEMORY_LOSS_MINOR_THRESHOLD: u32 = 5;
/// Concussion threshold (KO count) for MemoryLossMajor. Spec § Tunables.
pub const MEMORY_LOSS_MAJOR_THRESHOLD: u32 = 10;
/// Spec § "Phantom limb panic-roll 1× / week". The unit is one in-game
/// week expressed in sim seconds (`cf_aging::SECONDS_PER_IN_GAME_WEEK`,
/// duplicated here to keep this module standalone). 1 in-game year = 3600
/// sim seconds → 1 in-game week ≈ 69.23 sim seconds.
pub const PHANTOM_LIMB_PANIC_INTERVAL_SECONDS: f32 = 3600.0 / 52.0;
/// Base probability for the phantom-limb panic-roll once an actor's
/// severed limb has registered as `phantom_limb`. Multiplied × 0.25 when
/// a prosthetic is installed at the same zone (spec § "phantom_limb
/// panic-roll multiplier × 0.25"). The base chance is moderate (0.10)
/// so a 100-in-game-year campaign produces a recognizable number of
/// panic events without saturating.
pub const PHANTOM_LIMB_PANIC_BASE_CHANCE: f32 = 0.10;
/// Spec § "Refuse non-essential order" — chronic_depression rolls this
/// chance per AI tick when evaluating a non-essential order.
pub const CHRONIC_DEPRESSION_REFUSE_ORDER_CHANCE: f32 = 0.10;
/// Cumulative radiation dose threshold for cancer.lifecycle handoff.
/// In `dose_units`; matches M17 cumulative-dose granularity (1.0 = 1 Sv).
pub const RADIATION_CANCER_THRESHOLD: f32 = 6.0;

/// **M14I** § per-zone severed-limb record. Captures whether an actor's
/// limb was severed via attachable.detached so the post-survival pass
/// can promote it to `phantom_limb` once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeveredLimbRecord {
    pub zone: ZoneId,
    pub tick_severed: u64,
    /// True once the phantom-limb trait has been registered.
    pub phantom_limb_registered: bool,
    /// Cumulative seconds since the last per-week panic-roll. Reset to 0
    /// after each fired panic.
    pub seconds_since_last_panic: f32,
    /// Total number of panic rolls that fired.
    pub panic_rolls_fired: u32,
}

/// **M14I** § per-zone long-term state — derived from
/// [`LongTermState::severed_limbs`] + [`LongTermState::prosthetics`].
/// Consumers (cf-actor sim, cf-ui silhouette, AI doctrine) read this
/// instead of poking the raw maps.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneLongTermState {
    /// Default — the zone has not been severed and has no prosthetic.
    Intact = 0,
    /// The zone was severed via M14 attachable.detached and no prosthetic
    /// has been installed yet.
    Severed = 1,
    /// A prosthetic is installed on this zone. Replaces a Severed state.
    Prosthetic = 2,
}

impl ZoneLongTermState {
    pub fn as_str(self) -> &'static str {
        match self {
            ZoneLongTermState::Intact => "intact",
            ZoneLongTermState::Severed => "severed",
            ZoneLongTermState::Prosthetic => "prosthetic",
        }
    }
}

impl SeveredLimbRecord {
    pub fn new(zone: ZoneId, tick: u64) -> Self {
        Self {
            zone,
            tick_severed: tick,
            phantom_limb_registered: false,
            seconds_since_last_panic: 0.0,
            panic_rolls_fired: 0,
        }
    }
}

/// **M14I** § per-actor long-term-consequence aggregate.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LongTermState {
    pub scar_timeline: ScarTimeline,
    pub biological_age: Option<BiologicalAge>,
    pub prosthetics: Vec<ProstheticInstance>,
    pub traits: TraitSet,
    pub severed_limbs: BTreeMap<ZoneId, SeveredLimbRecord>,
    /// Number of concussions that crossed the KO threshold. Spec §
    /// "Each Concussion that exceeds the KO threshold increments
    /// `concussion_count`".
    pub concussion_count: u32,
    /// Cumulative pain baseline from scars + chronic conditions.
    pub chronic_pain_baseline: f32,
    /// Cumulative radiation dose (additive). M16B consumer.
    pub cumulative_radiation_dose: f32,
    /// True once `cumulative_radiation_dose >= RADIATION_CANCER_THRESHOLD`
    /// so the cancer hand-off fires once.
    pub cancer_handoff_fired: bool,
    /// Aggregated functional-debuff impact (pre-resolved on each scar
    /// acquisition + prosthetic install). Used by the passive pass.
    pub aggregate: FunctionalAggregate,
    /// Chassis wear pct for mechanical origins. Increments alongside the
    /// biological_age clock for robot / crystalline actors.
    pub chassis_wear_pct: f32,
    /// True when the actor is flagged for retirement (UI offers Retire).
    pub retirement_offered: bool,
    /// True after the actor opted into retirement.
    pub retired: bool,
    /// Tick the retire action committed.
    pub retired_tick: u64,
}

impl LongTermState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this actor accumulates any persistent long-term state.
    pub fn is_empty(&self) -> bool {
        self.scar_timeline.is_empty()
            && self.biological_age.is_none()
            && self.prosthetics.is_empty()
            && self.traits.is_empty()
            && self.severed_limbs.is_empty()
            && self.concussion_count == 0
            && self.chronic_pain_baseline == 0.0
            && self.cumulative_radiation_dose == 0.0
            && !self.cancer_handoff_fired
            && self.aggregate.is_zero()
            && self.chassis_wear_pct == 0.0
            && !self.retirement_offered
            && !self.retired
    }

    /// **M14I** § resolve the long-term state of a single zone.
    /// Consumers (sim, UI, AI) should treat the per-zone return as the
    /// canonical answer to "is this limb intact / severed / fitted with a
    /// prosthetic?".
    pub fn zone_state(&self, zone: &ZoneId) -> ZoneLongTermState {
        if self.prosthetics.iter().any(|p| &p.zone == zone) {
            ZoneLongTermState::Prosthetic
        } else if self.severed_limbs.contains_key(zone) {
            ZoneLongTermState::Severed
        } else {
            ZoneLongTermState::Intact
        }
    }

    /// **M14I** § list every (zone, state) pair the actor currently
    /// reports. Skips Intact entries so the caller only sees mutations.
    pub fn zone_states(&self) -> Vec<(ZoneId, ZoneLongTermState)> {
        let mut out: BTreeMap<ZoneId, ZoneLongTermState> = BTreeMap::new();
        for zone in self.severed_limbs.keys() {
            out.insert(zone.clone(), ZoneLongTermState::Severed);
        }
        for p in &self.prosthetics {
            out.insert(p.zone.clone(), ZoneLongTermState::Prosthetic);
        }
        out.into_iter().collect()
    }

    /// **M14I** § scenario 9 — chronic_depression "Refuse non-essential
    /// order" roll. Returns true ~10% of the time once the actor has the
    /// `chronic_depression` trait; always false otherwise.
    ///
    /// The roll uses a seeded `Xoshiro256StarStar` so the AI tick can
    /// reproduce the answer across saves. Callers MUST pass a seed
    /// derived from the engine seed (e.g. `engine_seed ⊕ tick ⊕
    /// actor_id`).
    pub fn chronic_depression_refuse_roll(&self, seed: u64) -> bool {
        if !self.traits.has(trait_ids::CHRONIC_DEPRESSION) {
            return false;
        }
        let mut rng = Xoshiro256StarStar::seed_from_u64(seed);
        let roll = (rng.next_u64() % 1000) as f32 / 1000.0;
        roll < crate::long_term::CHRONIC_DEPRESSION_REFUSE_ORDER_CHANCE
    }

    /// **M14I** § true when this actor currently carries any
    /// `chronic_<condition>` trait.
    pub fn has_chronic_condition(&self) -> bool {
        self.traits.has_chronic()
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.scar_timeline.checksum_bytes());
        if let Some(age) = &self.biological_age {
            out.push(1);
            out.extend_from_slice(&age.age_in_game_years.to_le_bytes());
            out.push(age.origin as u8);
            out.extend_from_slice(&age.caloric_max_decay.to_le_bytes());
            out.extend_from_slice(&age.max_speed_decay.to_le_bytes());
            out.extend_from_slice(&age.heal_rate_decay.to_le_bytes());
            out.push(if age.retirement_offered { 1 } else { 0 });
            out.push(if age.terminal_age_reached { 1 } else { 0 });
            out.push(if age.died_of_old_age { 1 } else { 0 });
            out.extend_from_slice(&age.terminal_rolls_fired.to_le_bytes());
            out.extend_from_slice(&age.seconds_into_current_year.to_le_bytes());
            out.extend_from_slice(&age.seconds_into_current_week.to_le_bytes());
        } else {
            out.push(0);
        }
        out.extend_from_slice(&(self.prosthetics.len() as u64).to_le_bytes());
        for p in &self.prosthetics {
            out.push(p.kind as u8);
            out.push(p.tier as u8);
            out.extend_from_slice(p.zone.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&p.wear_pct.to_le_bytes());
            out.push(if p.malfunctioning { 1 } else { 0 });
            out.extend_from_slice(&p.installed_tick.to_le_bytes());
            out.extend_from_slice(&p.last_maintained_tick.to_le_bytes());
        }
        out.extend_from_slice(&self.traits.checksum_bytes());
        out.extend_from_slice(&(self.severed_limbs.len() as u64).to_le_bytes());
        for (z, rec) in &self.severed_limbs {
            out.extend_from_slice(z.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&rec.tick_severed.to_le_bytes());
            out.push(if rec.phantom_limb_registered { 1 } else { 0 });
            out.extend_from_slice(&rec.seconds_since_last_panic.to_le_bytes());
            out.extend_from_slice(&rec.panic_rolls_fired.to_le_bytes());
        }
        out.extend_from_slice(&self.concussion_count.to_le_bytes());
        out.extend_from_slice(&self.chronic_pain_baseline.to_le_bytes());
        out.extend_from_slice(&self.cumulative_radiation_dose.to_le_bytes());
        out.push(if self.cancer_handoff_fired { 1 } else { 0 });
        out.extend_from_slice(&self.aggregate.checksum_bytes());
        out.extend_from_slice(&self.chassis_wear_pct.to_le_bytes());
        out.push(if self.retirement_offered { 1 } else { 0 });
        out.push(if self.retired { 1 } else { 0 });
        out.extend_from_slice(&self.retired_tick.to_le_bytes());
        out
    }
}

/// **M14I** § pre-resolved aggregate of every functional debuff on an
/// actor. Refreshed on scar acquisition + prosthetic install/maintain.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FunctionalAggregate {
    /// Cumulative `ml_lost` (ReducedMaxBlood).
    pub max_blood_ml_lost: f32,
    /// Per-zone cumulative pct strength loss (ReducedZoneStrength).
    pub zone_strength_loss: BTreeMap<ZoneId, f32>,
    /// Cumulative aim accuracy penalty `[0, 1]`.
    pub aim_accuracy_loss: f32,
    /// Cumulative move speed penalty `[0, 1]`.
    pub move_speed_loss: f32,
    /// Per-sense cumulative loss `[0, 1]`.
    pub sensory_loss: BTreeMap<String, f32>,
    /// True if any source contributes a Limp.
    pub limp: bool,
    /// Cumulative phantom-limb panic-chance per fire.
    pub phantom_panic_chance: f32,
}

impl FunctionalAggregate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_zero(&self) -> bool {
        self.max_blood_ml_lost == 0.0
            && self.zone_strength_loss.is_empty()
            && self.aim_accuracy_loss == 0.0
            && self.move_speed_loss == 0.0
            && self.sensory_loss.is_empty()
            && !self.limp
            && self.phantom_panic_chance == 0.0
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.max_blood_ml_lost.to_le_bytes());
        out.extend_from_slice(&(self.zone_strength_loss.len() as u64).to_le_bytes());
        for (z, pct) in &self.zone_strength_loss {
            out.extend_from_slice(z.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(&pct.to_le_bytes());
        }
        out.extend_from_slice(&self.aim_accuracy_loss.to_le_bytes());
        out.extend_from_slice(&self.move_speed_loss.to_le_bytes());
        out.extend_from_slice(&(self.sensory_loss.len() as u64).to_le_bytes());
        for (s, pct) in &self.sensory_loss {
            out.extend_from_slice(s.as_bytes());
            out.push(0);
            out.extend_from_slice(&pct.to_le_bytes());
        }
        out.push(if self.limp { 1 } else { 0 });
        out.extend_from_slice(&self.phantom_panic_chance.to_le_bytes());
        out
    }

    /// Apply one [`cf_scar::FunctionalDebuff`] onto the aggregate.
    #[allow(clippy::enum_glob_use)]
    pub fn add_debuff(&mut self, debuff: &cf_scar::FunctionalDebuff) {
        use cf_scar::FunctionalDebuff::*;
        match debuff {
            None => {}
            ReducedMaxBlood { ml_lost } => {
                self.max_blood_ml_lost += *ml_lost;
            }
            ReducedZoneStrength { zone, pct } => {
                let entry = self.zone_strength_loss.entry(zone.clone()).or_insert(0.0);
                *entry = (*entry + pct).clamp(0.0, 1.0);
            }
            ReducedAimAccuracy { pct } => {
                self.aim_accuracy_loss = (self.aim_accuracy_loss + pct).clamp(0.0, 1.0);
            }
            ReducedMoveSpeed { pct } => {
                self.move_speed_loss = (self.move_speed_loss + pct).clamp(0.0, 1.0);
            }
            SensoryLoss { sense, pct } => {
                let key = sense.as_str().to_string();
                let entry = self.sensory_loss.entry(key).or_insert(0.0);
                *entry = (*entry + pct).clamp(0.0, 1.0);
            }
            ChronicPainBaseline { .. } => {
                // Chronic pain feeds `LongTermState.chronic_pain_baseline`
                // directly so the move-speed multiplier matches the spec
                // (move 0.9× for chronic_depression, etc.).
            }
            Limp => {
                self.limp = true;
            }
            PhantomLimbRisk { chance_per_panic } => {
                self.phantom_panic_chance = (self.phantom_panic_chance + chance_per_panic).clamp(0.0, 1.0);
            }
        }
    }

    /// Apply a prosthetic install — credits back functional restoration
    /// against the zone's loss. Spec § "max_speed restored to 70%".
    pub fn apply_prosthetic(&mut self, inst: &ProstheticInstance) {
        let restoration = inst.tier.functional_restoration();
        if let Some(loss) = self.zone_strength_loss.get_mut(&inst.zone) {
            let restored = (*loss * restoration).min(*loss);
            *loss = (*loss - restored).max(0.0);
        }
        // Move-speed credit when a leg prosthetic is installed.
        match inst.kind {
            cf_prosthetic::ProstheticKind::ProstheticLegT1
            | cf_prosthetic::ProstheticKind::CyberneticLegT2 => {
                self.move_speed_loss =
                    (self.move_speed_loss * (1.0 - restoration)).clamp(0.0, 1.0);
                self.limp = false;
            }
            cf_prosthetic::ProstheticKind::ProstheticArmT1
            | cf_prosthetic::ProstheticKind::CyberneticArmT2 => {
                self.aim_accuracy_loss =
                    (self.aim_accuracy_loss * (1.0 - restoration)).clamp(0.0, 1.0);
            }
            cf_prosthetic::ProstheticKind::ProstheticEyeT1
            | cf_prosthetic::ProstheticKind::CyberneticEyeT2Thermal => {
                if let Some(loss) = self.sensory_loss.get_mut("sight") {
                    *loss = (*loss * (1.0 - restoration)).clamp(0.0, 1.0);
                }
            }
            cf_prosthetic::ProstheticKind::ProstheticEarT1 => {
                if let Some(loss) = self.sensory_loss.get_mut("hearing") {
                    *loss = (*loss * (1.0 - restoration)).clamp(0.0, 1.0);
                }
            }
            _ => {}
        }
    }

    /// Resolve the actor's effective move-speed multiplier given the
    /// aggregate + chronic-condition traits.
    pub fn move_speed_multiplier(&self, has_chronic_depression: bool) -> f32 {
        let mut mult = (1.0 - self.move_speed_loss).clamp(0.0, 1.0);
        if self.limp {
            mult *= 0.75;
        }
        if has_chronic_depression {
            mult *= 0.9;
        }
        mult.clamp(0.0, 1.0)
    }

    /// Resolve the actor's effective aim-accuracy multiplier.
    pub fn aim_accuracy_multiplier(&self) -> f32 {
        (1.0 - self.aim_accuracy_loss).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_scar::FunctionalDebuff;
    use cf_wound::registry::ZoneId;

    #[test]
    fn aggregate_stacks_zone_strength() {
        let mut a = FunctionalAggregate::new();
        a.add_debuff(&FunctionalDebuff::ReducedZoneStrength {
            zone: ZoneId::from("arm_left"),
            pct: 0.05,
        });
        a.add_debuff(&FunctionalDebuff::ReducedZoneStrength {
            zone: ZoneId::from("arm_left"),
            pct: 0.05,
        });
        assert!((a.zone_strength_loss[&ZoneId::from("arm_left")] - 0.10).abs() < 1e-6);
    }

    #[test]
    fn limp_flag_set_by_fracture() {
        let mut a = FunctionalAggregate::new();
        a.add_debuff(&FunctionalDebuff::Limp);
        assert!(a.limp);
        let m = a.move_speed_multiplier(false);
        assert!(m < 1.0);
    }

    #[test]
    fn chronic_depression_slows_to_0_9() {
        let a = FunctionalAggregate::new();
        let m = a.move_speed_multiplier(true);
        assert!((m - 0.9).abs() < 1e-6);
    }

    #[test]
    fn prosthetic_install_restores_speed() {
        let mut a = FunctionalAggregate::new();
        a.add_debuff(&FunctionalDebuff::Limp);
        a.move_speed_loss = 0.3;
        let inst = ProstheticInstance::new(
            cf_prosthetic::ProstheticKind::ProstheticLegT1,
            ZoneId::from("leg_right"),
            0,
        );
        a.apply_prosthetic(&inst);
        assert!(!a.limp);
        // T1 restores 70%: 0.3 × (1 - 0.7) = 0.09
        assert!((a.move_speed_loss - 0.09).abs() < 1e-5);
    }

    #[test]
    fn zone_state_transitions_severed_to_prosthetic() {
        let mut lt = LongTermState::new();
        let zone = ZoneId::from("leg_right");
        assert_eq!(lt.zone_state(&zone), ZoneLongTermState::Intact);
        lt.severed_limbs.insert(
            zone.clone(),
            SeveredLimbRecord::new(zone.clone(), 10),
        );
        assert_eq!(lt.zone_state(&zone), ZoneLongTermState::Severed);
        lt.prosthetics.push(ProstheticInstance::new(
            cf_prosthetic::ProstheticKind::ProstheticLegT1,
            zone.clone(),
            20,
        ));
        assert_eq!(lt.zone_state(&zone), ZoneLongTermState::Prosthetic);
    }

    #[test]
    fn chronic_depression_refuse_roll_only_with_trait() {
        let mut lt = LongTermState::new();
        // No trait — always false.
        for seed in 0..256 {
            assert!(!lt.chronic_depression_refuse_roll(seed));
        }
        lt.traits.insert(trait_ids::CHRONIC_DEPRESSION);
        // ~10% chance. Sweep 1000 seeds + check the empirical rate sits
        // between 5% and 15% (loose bound but flushes obvious bugs).
        let mut hits = 0u32;
        for seed in 0..1000 {
            if lt.chronic_depression_refuse_roll(seed) {
                hits += 1;
            }
        }
        assert!(
            hits >= 50 && hits <= 150,
            "expected ~10% hit rate, got {}/1000",
            hits
        );
    }

    #[test]
    fn chronic_depression_refuse_roll_deterministic() {
        let mut lt = LongTermState::new();
        lt.traits.insert(trait_ids::CHRONIC_DEPRESSION);
        for seed in 0..100 {
            let a = lt.chronic_depression_refuse_roll(seed);
            let b = lt.chronic_depression_refuse_roll(seed);
            assert_eq!(a, b, "roll must be deterministic for seed={seed}");
        }
    }

    #[test]
    fn has_chronic_condition_matches_prefix() {
        let mut lt = LongTermState::new();
        assert!(!lt.has_chronic_condition());
        lt.traits.insert(trait_ids::PHANTOM_LIMB);
        assert!(!lt.has_chronic_condition());
        lt.traits.insert("chronic_insomnia");
        assert!(lt.has_chronic_condition());
    }
}
