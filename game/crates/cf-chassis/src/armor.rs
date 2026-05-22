use serde::{Deserialize, Serialize};

use crate::BodyZone;

/// Layer ordering within a zone. Damage strips `External` first, `Internal` next,
/// then breaches into `Core`. Once `Core.hp == 0` the zone is considered breached
/// and routes damage to the actor HP via [`crate::ZoneDamageOutcome::actor_hp_damage`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorLayerKind {
    External = 0,
    Internal = 1,
    Core = 2,
}

impl ArmorLayerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ArmorLayerKind::External => "external",
            ArmorLayerKind::Internal => "internal",
            ArmorLayerKind::Core => "core",
        }
    }
}

/// One layer of armor on a zone. `hardness` reduces incoming damage; `integrity` is
/// a 0..1 derived field surfaced for HUD + AI ("75% external integrity left").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorLayer {
    pub kind: ArmorLayerKind,
    pub hp: f32,
    pub hp_max: f32,
    /// Flat damage reduction subtracted from incoming damage before HP is touched.
    /// Clamped at 0 so a Hardness > damage produces a no-op (ricochet).
    pub hardness: f32,
}

impl ArmorLayer {
    pub fn new(kind: ArmorLayerKind, hp_max: f32, hardness: f32) -> Self {
        Self {
            kind,
            hp: hp_max.max(0.0),
            hp_max: hp_max.max(0.0),
            hardness: hardness.max(0.0),
        }
    }

    pub fn integrity(&self) -> f32 {
        if self.hp_max <= 0.0 {
            0.0
        } else {
            (self.hp / self.hp_max).clamp(0.0, 1.0)
        }
    }

    pub fn is_breached(&self) -> bool {
        self.hp <= 0.0
    }

    pub fn reset(&mut self) {
        self.hp = self.hp_max;
    }
}

/// Per-zone armor + wound container. `wound_hp` accumulates AFTER all three armor
/// layers are breached; when `wound_hp` hits zero the zone is considered destroyed
/// and emits `armor_zone_destroyed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneState {
    pub zone: BodyZone,
    pub layers: Vec<ArmorLayer>,
    pub wound_hp: f32,
    pub wound_hp_max: f32,
    /// damage on this zone (< 1.0 = tougher). Default 1.0.
    #[serde(default = "default_damage_multiplier")]
    pub damage_multiplier: f32,
    /// below which the zone cannot be gibbed off. Default 800 N·s; heavy
    /// chassis raises this to 1600..3200 N·s.
    #[serde(default = "default_gib_impulse_limit")]
    pub gib_impulse_limit: f32,
    /// duration + knockdown probability (0.2 = heavy; 1.0 = baseline).
    #[serde(default = "default_stagger_factor")]
    pub stagger_factor: f32,
    /// `true` once `wound_hp <= 0`; the zone is destroyed and emits
    /// `armor_zone_destroyed`. Limb destruction has mechanical consequences listed
    /// in [`crate::BodyGraph::movement_contributions`].
    pub destroyed: bool,
}

pub(crate) fn default_damage_multiplier() -> f32 {
    1.0
}

pub(crate) fn default_gib_impulse_limit() -> f32 {
    800.0
}

pub(crate) fn default_stagger_factor() -> f32 {
    1.0
}

impl ZoneState {
    pub fn new(zone: BodyZone, layers: Vec<ArmorLayer>, wound_hp: f32) -> Self {
        Self {
            zone,
            layers,
            wound_hp: wound_hp.max(0.0),
            wound_hp_max: wound_hp.max(0.0),
            damage_multiplier: default_damage_multiplier(),
            gib_impulse_limit: default_gib_impulse_limit(),
            stagger_factor: default_stagger_factor(),
            destroyed: false,
        }
    }

    #[must_use]
    pub fn with_damage_multiplier(mut self, mult: f32) -> Self {
        self.damage_multiplier = mult;
        self
    }

    #[must_use]
    pub fn with_gib_impulse_limit(mut self, limit: f32) -> Self {
        self.gib_impulse_limit = limit;
        self
    }

    #[must_use]
    pub fn with_stagger_factor(mut self, factor: f32) -> Self {
        self.stagger_factor = factor;
        self
    }

    pub fn reset(&mut self) {
        for layer in &mut self.layers {
            layer.reset();
        }
        self.wound_hp = self.wound_hp_max;
        self.destroyed = false;
    }

    pub fn external_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::External)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn internal_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::Internal)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn core_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::Core)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn wound_integrity(&self) -> f32 {
        if self.wound_hp_max <= 0.0 {
            0.0
        } else {
            (self.wound_hp / self.wound_hp_max).clamp(0.0, 1.0)
        }
    }

    /// Composite "how OK is this zone" — averages external/internal/core/wound. Used
    /// for HUD silhouette tinting + AI utility scoring.
    pub fn zone_integrity(&self) -> f32 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for layer in &self.layers {
            sum += layer.integrity();
            n += 1.0;
        }
        sum += self.wound_integrity();
        n += 1.0;
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    }
}

/// angles drive the M9 angled-armor math: incoming projectiles that strike
/// at a glancing angle effectively thicken the armor.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ArmorMountAngles {
    /// Forward-facing armor angle (degrees from vertical).
    pub front_degrees: f32,
    /// Lateral / side armor angle (degrees).
    pub side_degrees: f32,
    /// Rear armor angle (degrees).
    pub back_degrees: f32,
}

impl ArmorMountAngles {
    pub const fn new(front: f32, side: f32, back: f32) -> Self {
        Self {
            front_degrees: front,
            side_degrees: side,
            back_degrees: back,
        }
    }
}
