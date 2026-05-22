//! **M6C**: per-actor body armor + helmet + gloves + boots + knee/elbow
//! pad slots, separate from the M13 chassis armor surface.
//!
//! Per M6C spec § "Crates / modules touched":
//! > `cf-actor::body_armor_slot` | NEW | Per-actor helmet + body armor +
//! > gloves + boots + knee/elbow slots (separate from M13 chassis armor)
//!
//! Each slot carries a [`cf_equipment::PpePreset`] reference (by id) +
//! current durability. The damage routing for M6C-2 is implemented via
//! [`apply_kinetic_hit_through_body_armor`] which delegates to the
//! `cf_equipment::ppe::armor_calc` helper for the actual reduction
//! formula; this module simply owns the persistent slot state.

use cf_equipment::{
    apply_kinetic_hit, ppe_preset, DamageReductionResult, PpeKind, PpePreset,
};
use serde::{Deserialize, Serialize};

/// One body-armor slot. `item_id` is the canonical
/// [`cf_equipment::ItemSpec`] id; the durability is tracked per-actor so
/// hits degrade the equipped item independently of the global registry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArmorSlotState {
    pub item_id: String,
    pub durability_current: f32,
    pub durability_max: f32,
}

impl ArmorSlotState {
    pub fn equipped(&self) -> bool {
        !self.item_id.is_empty() && self.durability_max > 0.0
    }
}

/// Per-actor body armor coverage. Each slot is independently
/// equippable; the spec § "Player-facing behavior > PPE" enumerates the
/// canonical slot list (helmet + body + gloves + boots + knee + elbow).
/// Sealed suits (EVA / hardsuit / hazmat / radiation) occupy the body
/// slot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BodyArmorSlot {
    pub helmet: ArmorSlotState,
    pub body: ArmorSlotState,
    pub gloves: ArmorSlotState,
    pub boots: ArmorSlotState,
    pub knee_pads: ArmorSlotState,
    pub elbow_pads: ArmorSlotState,
}

/// Where a hit landed. Drives which slot's PPE participates in damage
/// reduction.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitZone {
    Head = 0,
    Torso = 1,
    Arm = 2,
    Leg = 3,
    Knee = 4,
    Elbow = 5,
}

impl HitZone {
    pub fn as_str(self) -> &'static str {
        match self {
            HitZone::Head => "head",
            HitZone::Torso => "torso",
            HitZone::Arm => "arm",
            HitZone::Leg => "leg",
            HitZone::Knee => "knee",
            HitZone::Elbow => "elbow",
        }
    }
}

/// Reason returned by [`BodyArmorSlot::equip`] when an item can't be
/// equipped.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipReject {
    /// The item id is not a registered PPE preset.
    UnknownItem,
    /// The PPE kind does not match the slot category.
    WrongSlot,
}

impl BodyArmorSlot {
    /// Construct an empty slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Equip an item id into the slot the PPE preset declares. Returns
    /// the previously-equipped item id (if any) so the caller can return
    /// it to the actor's inventory.
    pub fn equip(&mut self, item_id: &str) -> Result<Option<String>, EquipReject> {
        let Some(preset) = ppe_preset(item_id) else {
            return Err(EquipReject::UnknownItem);
        };
        let slot = self.slot_mut_for(preset.kind);
        let prev = if slot.equipped() { Some(slot.item_id.clone()) } else { None };
        *slot = ArmorSlotState {
            item_id: preset.id.clone(),
            durability_current: preset.durability_hp,
            durability_max: preset.durability_hp,
        };
        Ok(prev)
    }

    /// Remove whatever is in the slot. Returns the removed item id.
    pub fn unequip(&mut self, kind: PpeKind) -> Option<String> {
        let slot = self.slot_mut_for(kind);
        if slot.equipped() {
            let id = std::mem::take(&mut slot.item_id);
            *slot = ArmorSlotState::default();
            Some(id)
        } else {
            None
        }
    }

    fn slot_mut_for(&mut self, kind: PpeKind) -> &mut ArmorSlotState {
        match kind {
            PpeKind::Helmet => &mut self.helmet,
            PpeKind::Gloves => &mut self.gloves,
            PpeKind::Boots => &mut self.boots,
            PpeKind::KneePads => &mut self.knee_pads,
            PpeKind::ElbowPads => &mut self.elbow_pads,
            PpeKind::BodyArmor
            | PpeKind::Hardsuit
            | PpeKind::EvaSuit
            | PpeKind::RadiationSuit
            | PpeKind::HazmatSuit
            | PpeKind::InsulatedSuit
            | PpeKind::ModularPlateCarrier => &mut self.body,
        }
    }

    /// True when the body slot AND helmet slot are sealed (M6C-6).
    pub fn is_fully_sealed(&self) -> bool {
        let body_sealed = ppe_preset(&self.body.item_id).map_or(false, |p| p.sealed);
        let helmet_sealed = ppe_preset(&self.helmet.item_id).map_or(false, |p| p.sealed);
        body_sealed && helmet_sealed
    }

    /// sealed helmet (per M19C/M6C PPE seal flag). Used by the M14J swim
    /// integration to suppress drowning while submerged.
    pub fn helmet_seal_active(&self) -> bool {
        ppe_preset(&self.helmet.item_id).map_or(false, |p| p.sealed)
    }

    /// a dive-suit / hardsuit / EVA suit (per M19C/M6C PPE). Used by the
    /// M14J swim integration to suppress drowning.
    pub fn dive_suit_equipped(&self) -> bool {
        match ppe_preset(&self.body.item_id) {
            Some(p) => matches!(
                p.kind,
                cf_equipment::PpeKind::Hardsuit
                    | cf_equipment::PpeKind::EvaSuit
                    | cf_equipment::PpeKind::HazmatSuit
            ) || p.sealed,
            None => false,
        }
    }

    /// Apply a kinetic hit to the slot associated with `zone`. Returns
    /// the damage that reaches the actor's HP pool + a flag the engine
    /// can use to emit `body_armor.degraded`.
    pub fn apply_kinetic_hit(&mut self, zone: HitZone, raw_damage: f32) -> ArmorHitOutcome {
        let (kind, slot) = match zone {
            HitZone::Head => (PpeKind::Helmet, &mut self.helmet),
            HitZone::Torso => (PpeKind::BodyArmor, &mut self.body),
            HitZone::Arm => (PpeKind::Gloves, &mut self.gloves),
            HitZone::Leg => (PpeKind::Boots, &mut self.boots),
            HitZone::Knee => (PpeKind::KneePads, &mut self.knee_pads),
            HitZone::Elbow => (PpeKind::ElbowPads, &mut self.elbow_pads),
        };
        if !slot.equipped() {
            return ArmorHitOutcome {
                slot_kind: kind,
                slot_item_id: String::new(),
                hp_damage: raw_damage.max(0.0),
                reduction: DamageReductionResult {
                    damage_after_reduction: raw_damage.max(0.0),
                    ..Default::default()
                },
            };
        }
        // Look up the preset's kinetic reduction value from the canonical
        // registry. Falls back to 0.0 (no protection) if the id was
        // somehow dropped from the registry between equip + hit.
        let kinetic_reduction = ppe_preset(&slot.item_id).map_or(0.0, |p| p.kinetic_damage_reduction);
        let result = apply_kinetic_hit(raw_damage, kinetic_reduction, slot.durability_current, slot.durability_max);
        slot.durability_current = result.durability_after;
        ArmorHitOutcome {
            slot_kind: kind,
            slot_item_id: slot.item_id.clone(),
            hp_damage: result.damage_after_reduction,
            reduction: result,
        }
    }

    /// Return the equipped preset (if any) for a given slot kind.
    pub fn equipped_preset(&self, kind: PpeKind) -> Option<PpePreset> {
        let slot = match kind {
            PpeKind::Helmet => &self.helmet,
            PpeKind::Gloves => &self.gloves,
            PpeKind::Boots => &self.boots,
            PpeKind::KneePads => &self.knee_pads,
            PpeKind::ElbowPads => &self.elbow_pads,
            PpeKind::BodyArmor
            | PpeKind::Hardsuit
            | PpeKind::EvaSuit
            | PpeKind::RadiationSuit
            | PpeKind::HazmatSuit
            | PpeKind::InsulatedSuit
            | PpeKind::ModularPlateCarrier => &self.body,
        };
        if slot.equipped() {
            ppe_preset(&slot.item_id)
        } else {
            None
        }
    }
}

/// Outcome of one body-armor damage hit. The engine surfaces this on
/// `body_armor.degraded` / `body_armor.hit` event channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorHitOutcome {
    pub slot_kind: PpeKind,
    pub slot_item_id: String,
    pub hp_damage: f32,
    pub reduction: DamageReductionResult,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equip_armor_kevlar_light_into_body_slot() {
        let mut s = BodyArmorSlot::new();
        let prev = s.equip("armor_kevlar_light").unwrap();
        assert!(prev.is_none());
        assert_eq!(s.body.item_id, "armor_kevlar_light");
        assert!(s.body.durability_current > 0.0);
    }

    #[test]
    fn equip_helmet_into_helmet_slot() {
        let mut s = BodyArmorSlot::new();
        s.equip("helmet_light_kevlar").unwrap();
        assert_eq!(s.helmet.item_id, "helmet_light_kevlar");
    }

    #[test]
    fn equip_unknown_item_rejected() {
        let mut s = BodyArmorSlot::new();
        let err = s.equip("does_not_exist").unwrap_err();
        assert_eq!(err, EquipReject::UnknownItem);
    }

    #[test]
    fn rifle_round_reduced_by_kevlar_light_armor() {
        // M6C-2 Scenario:
        //   Given infantry actor wearing armor_kevlar_light
        //   When hit by rifle round (kinetic 50)
        //   Then damage reduced by 20%
        let mut s = BodyArmorSlot::new();
        s.equip("armor_kevlar_light").unwrap();
        let outcome = s.apply_kinetic_hit(HitZone::Torso, 50.0);
        assert!((outcome.hp_damage - 40.0).abs() < 1e-3);
        assert!(s.body.durability_current < s.body.durability_max);
    }

    #[test]
    fn body_armor_degraded_fires_on_durability_tick_under_fifty_percent() {
        // M6C-2 Scenario continued:
        //   And body_armor.degraded fires on durability tick
        // Heavy armor has reduction = 0.80, so a 250-damage hit absorbs 200
        // and drops a fresh 1800-durability armor by 200 — still well above
        // 50%. Stack hits until the crossing happens.
        let mut s = BodyArmorSlot::new();
        s.equip("armor_heavy_plate").unwrap();
        let mut saw_degraded = false;
        for _ in 0..20 {
            let out = s.apply_kinetic_hit(HitZone::Torso, 200.0);
            if out.reduction.crossed_degraded_threshold {
                saw_degraded = true;
                break;
            }
        }
        assert!(saw_degraded);
    }

    #[test]
    fn unequipped_slot_passes_full_damage() {
        let mut s = BodyArmorSlot::new();
        let out = s.apply_kinetic_hit(HitZone::Torso, 50.0);
        assert!((out.hp_damage - 50.0).abs() < 1e-3);
    }

    #[test]
    fn eva_suit_plus_helmet_fully_sealed() {
        // M6C-6 Scenario:
        //   Given player wearing eva_suit + sealed helmet
        //   Then is_fully_sealed = true (M6C audit: helmet_heavy_titanium
        //   carries sealed=true so the M6C-6 acceptance can be reproduced
        //   from cataloged SKUs without inventing a new helmet).
        let mut s = BodyArmorSlot::new();
        s.equip("eva_suit").unwrap();
        s.equip("helmet_heavy_titanium").unwrap();
        assert!(ppe_preset(&s.body.item_id).unwrap().sealed);
        assert!(ppe_preset(&s.helmet.item_id).unwrap().sealed);
        assert!(s.is_fully_sealed());
    }

    #[test]
    fn body_unsealed_armor_breaks_fully_sealed_state() {
        let mut s = BodyArmorSlot::new();
        s.equip("armor_kevlar_light").unwrap();
        s.equip("helmet_heavy_titanium").unwrap();
        // Body armor (light kevlar) is NOT sealed, so the seal fails.
        assert!(!s.is_fully_sealed());
    }

    #[test]
    fn unequip_removes_state() {
        let mut s = BodyArmorSlot::new();
        s.equip("armor_kevlar_light").unwrap();
        let removed = s.unequip(PpeKind::BodyArmor).unwrap();
        assert_eq!(removed, "armor_kevlar_light");
        assert!(!s.body.equipped());
    }
}
