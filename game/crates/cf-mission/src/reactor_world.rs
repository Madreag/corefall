//! `Reactor` + `ReactorWorld` — damageable static actors with the M9
//! 3-layer armor cascade and pressure-state ladder. Split out of `lib.rs`
//! for the 2k-LOC ceiling. Public API is re-exported at the crate root.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::reactor;

/// One reactor entry the engine tracks as a damageable static actor. The engine
/// projects current hp + destroyed flag into [`MissionTickInputs::reactors`] so
/// `defend_reactor` objectives can detect destruction.
///
/// pressure-state ladder, 3-layer armor cascade (External / Internal / Core),
/// mission_critical flag, role tag, heat signature, and a set of
/// `serde(default)` forward-compat fields for M13+ chassis modules / M25+
/// command-core power grid + shields + repair pads + doors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reactor {
    pub id: String,
    pub position: [f32; 2],
    pub half_extents: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    /// True once `hp <= 0.0`. Latched: a reactor cannot un-destroy itself.
    #[serde(default)]
    pub destroyed: bool,
    /// Pressure-state ladder. Advances with HP-percent thresholds per M9 spec
    /// (Nominal > 75%, Stressed 50-75%, Critical 25-50%, Venting 0-25%,
    /// Destroyed = 0). Defaults to Nominal at scenario load.
    #[serde(default)]
    pub pressure_state: reactor::PressureState,
    /// Per CCCP `m_MissionCritical`: when true the reactor cannot be gibbed
    /// instantly (chassis_gibbed never fires). Damage routes through the
    /// 3-layer armor cascade. Default true so legacy scenarios that don't
    /// declare the flag inherit the M9 protection.
    #[serde(default = "default_mission_critical")]
    pub mission_critical: bool,
    /// Identifies the reactor's downstream role. M9 ships
    /// `"command_core_predecessor"`; M25+ flips to `"command_core"` per
    #[serde(default = "default_reactor_role")]
    pub role: String,
    /// Forward-compat thermal field for the M16/M19 thermal kernel.
    #[serde(default)]
    pub heat_signature_k: f32,
    /// 3-layer armor cascade per M9 spec § Layered reactor armor. M9 ships
    /// the External (60% of total HP) / Internal (30%) / Core (10%) split.
    /// Default empty so legacy `.ron` scenarios round-trip cleanly; the engine
    /// invokes [`Reactor::ensure_armor_layers`] at scenario load to populate.
    #[serde(default)]
    pub armor_layers: Vec<reactor::LayerState>,
    // without bumping the schema. Audit Pass 7 verifies the placeholder
    // shape matches the spec's declared "Forward-compat (None / empty at M9)"
    // contract.
    #[serde(default)]
    pub power_grid: Option<reactor::PowerGridPlaceholder>,
    #[serde(default)]
    pub shields: Vec<reactor::ShieldModulePlaceholder>,
    #[serde(default)]
    pub modules: Vec<reactor::ChassisModulePlaceholder>,
    #[serde(default)]
    pub uprooted_avatar: Option<u64>,
    #[serde(default)]
    pub repair_pads: Vec<reactor::RepairPadPlaceholder>,
    #[serde(default)]
    pub doors: Vec<reactor::DoorPlaceholder>,
    #[serde(default)]
    pub affliction_overlay: Vec<reactor::AfflictionOverlayPlaceholder>,
    #[serde(default)]
    pub environment_signal: Option<reactor::EnvironmentSignalPlaceholder>,
}

pub(crate) fn default_mission_critical() -> bool {
    true
}

pub(crate) fn default_reactor_role() -> String {
    "command_core_predecessor".to_string()
}

impl Default for Reactor {
    fn default() -> Self {
        Self {
            id: String::new(),
            position: [0.0, 0.0],
            half_extents: [0.0, 0.0],
            hp: 0.0,
            max_hp: 0.0,
            destroyed: false,
            pressure_state: reactor::PressureState::Nominal,
            mission_critical: true,
            role: default_reactor_role(),
            heat_signature_k: 0.0,
            armor_layers: Vec::new(),
            power_grid: None,
            shields: Vec::new(),
            modules: Vec::new(),
            uprooted_avatar: None,
            repair_pads: Vec::new(),
            doors: Vec::new(),
            affliction_overlay: Vec::new(),
            environment_signal: None,
        }
    }
}

impl Reactor {
    pub fn is_destroyed(&self) -> bool {
        self.destroyed || self.hp <= 0.0
    }

    /// True if `(x, y)` is inside the reactor's AABB.
    pub fn aabb_contains(&self, x: f32, y: f32) -> bool {
        let min_x = self.position[0] - self.half_extents[0];
        let max_x = self.position[0] + self.half_extents[0];
        let min_y = self.position[1] - self.half_extents[1];
        let max_y = self.position[1] + self.half_extents[1];
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    pub fn hp_percent(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }

    /// Core armor cascade. Idempotent; safe to call at scenario load (when the
    /// `.ron` may have omitted `armor_layers: []`).
    pub fn ensure_armor_layers(&mut self) {
        if !self.armor_layers.is_empty() {
            return;
        }
        let total = self.max_hp.max(1.0);
        self.armor_layers = vec![
            reactor::LayerState::new(reactor::LayerKind::External, total * 0.6, 0.9),
            reactor::LayerState::new(reactor::LayerKind::Internal, total * 0.3, 0.7),
            reactor::LayerState::new(reactor::LayerKind::Core, total * 0.1, 0.5),
        ];
    }

    /// Apply `damage` to this reactor's hp; returns the post-damage view.
    /// Damage is clamped at zero; `destroyed` flips true when hp hits zero.
    ///
    /// the pressure-state ladder. The legacy single-HP signature is preserved
    /// for callers that only need to mutate hp; the richer cascade output
    /// lives in [`Reactor::apply_damage_cascade`].
    pub fn apply_damage(&mut self, damage: f32) {
        let _ = self.apply_damage_cascade(damage);
    }

    /// layers and advance the pressure-state ladder. Returns a structured
    /// report the engine reads to fire `armor.layer_hp_changed` /
    /// `armor.layer_destroyed` / `mission.reactor_hp_changed` /
    /// `mission.reactor_pressure_state_changed` / `mission.reactor_destroyed`.
    pub fn apply_damage_cascade(&mut self, damage: f32) -> reactor::ReactorDamageReport {
        self.apply_damage_cascade_with_safety(damage, false)
    }

    /// Internal → Core armor layers and advance the pressure-state ladder,
    /// honoring the scenario's `tutorial_safety` flag. When
    /// `tutorial_safety = true` AND the cascade would drop the reactor's
    /// HP to 0, the cascade caps HP at 1.0 and forces `pressure_state =
    /// Critical` so the mission can be recovered instead of resolving as
    /// loss. The matching cfctl script `m2.5_tutorial_safety.cfctl.json`
    /// exercises this branch.
    pub fn apply_damage_cascade_with_safety(
        &mut self,
        damage: f32,
        tutorial_safety: bool,
    ) -> reactor::ReactorDamageReport {
        let hp_before = self.hp;
        let pressure_before = self.pressure_state;
        if self.is_destroyed() {
            return reactor::ReactorDamageReport {
                hp_before,
                hp_after: self.hp,
                hp_percent_after: self.hp_percent(),
                damage_applied: 0.0,
                layer_events: Vec::new(),
                pressure_state_change: None,
                now_destroyed: true,
                triggered_destruction: false,
            };
        }
        self.ensure_armor_layers();
        let mut remaining = damage.max(0.0);
        let mut events: Vec<reactor::ArmorLayerHpEvent> = Vec::new();
        for layer in self.armor_layers.iter_mut() {
            if remaining <= 0.0 {
                break;
            }
            if layer.is_destroyed() {
                continue;
            }
            let from = layer.hp;
            let absorbed = remaining.min(layer.hp);
            layer.hp = (layer.hp - absorbed).max(0.0);
            remaining -= absorbed;
            let to = layer.hp;
            let now_destroyed = layer.is_destroyed();
            let critical = !now_destroyed && layer.hp_percent() <= 0.25;
            events.push(reactor::ArmorLayerHpEvent {
                layer: layer.kind,
                from,
                to,
                destroyed: now_destroyed,
                critical,
            });
        }
        let absorbed_total = damage.max(0.0) - remaining;
        self.hp = (self.hp - absorbed_total).max(0.0);
        // cascade would otherwise destroy the reactor, hold HP at 1.0
        // and stamp `pressure_state = Critical` instead. Restore the
        // Core layer to at least 1 HP so subsequent damage applications
        // don't fall through the destroyed-layer fast path.
        let mut tutorial_safety_engaged = false;
        if tutorial_safety && self.hp <= 0.0 {
            self.hp = 1.0;
            if let Some(core) = self
                .armor_layers
                .iter_mut()
                .find(|l| l.kind == reactor::LayerKind::Core)
            {
                if core.hp <= 0.0 {
                    core.hp = 1.0;
                }
            }
            tutorial_safety_engaged = true;
        }
        let now_destroyed = self.hp <= 0.0;
        let triggered_destruction = now_destroyed && !self.destroyed;
        if now_destroyed {
            self.destroyed = true;
        }
        let pressure_after = if tutorial_safety_engaged {
            reactor::PressureState::Critical
        } else {
            reactor::pressure_state_for_hp_percent(self.hp_percent())
        };
        self.pressure_state = pressure_after;
        let pressure_state_change = if pressure_before == pressure_after {
            None
        } else {
            Some((pressure_before, pressure_after))
        };
        reactor::ReactorDamageReport {
            hp_before,
            hp_after: self.hp,
            hp_percent_after: self.hp_percent(),
            damage_applied: absorbed_total,
            layer_events: events,
            pressure_state_change,
            now_destroyed,
            triggered_destruction,
        }
    }

    pub fn reset(&mut self) {
        self.hp = self.max_hp;
        self.destroyed = false;
        self.pressure_state = reactor::PressureState::Nominal;
        if !self.armor_layers.is_empty() {
            let total = self.max_hp.max(1.0);
            for layer in self.armor_layers.iter_mut() {
                layer.max_hp = match layer.kind {
                    reactor::LayerKind::External => total * 0.6,
                    reactor::LayerKind::Internal => total * 0.3,
                    reactor::LayerKind::Core => total * 0.1,
                };
                layer.hp = layer.max_hp;
            }
        }
    }

    /// Layout-stable bytes for the determinism checksum.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(48 + self.id.len());
        v.extend_from_slice(&(self.id.len() as u32).to_le_bytes());
        v.extend_from_slice(self.id.as_bytes());
        v.extend_from_slice(&quantize(self.position[0]).to_le_bytes());
        v.extend_from_slice(&quantize(self.position[1]).to_le_bytes());
        v.extend_from_slice(&quantize(self.half_extents[0]).to_le_bytes());
        v.extend_from_slice(&quantize(self.half_extents[1]).to_le_bytes());
        v.extend_from_slice(&quantize(self.hp).to_le_bytes());
        v.extend_from_slice(&quantize(self.max_hp).to_le_bytes());
        v.push(u8::from(self.destroyed));
        // per-layer hp so per-tick checksum byte-matches across host
        // implementations + re-runs.
        v.push(self.pressure_state as u8);
        v.push(u8::from(self.mission_critical));
        v.extend_from_slice(&quantize(self.heat_signature_k).to_le_bytes());
        v.extend_from_slice(&(self.armor_layers.len() as u32).to_le_bytes());
        for layer in &self.armor_layers {
            v.push(layer.kind as u8);
            v.extend_from_slice(&quantize(layer.hp).to_le_bytes());
            v.extend_from_slice(&quantize(layer.max_hp).to_le_bytes());
            v.extend_from_slice(&quantize(layer.hardness).to_le_bytes());
        }
        v
    }
}

pub(crate) fn quantize(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}

/// World container of every reactor the engine knows about.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReactorWorld {
    pub reactors: BTreeMap<String, Reactor>,
}

impl ReactorWorld {
    pub fn new(reactors: Vec<Reactor>) -> Self {
        let mut map = BTreeMap::new();
        for r in reactors {
            map.insert(r.id.clone(), r);
        }
        Self { reactors: map }
    }

    pub fn get(&self, id: &str) -> Option<&Reactor> {
        self.reactors.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Reactor> {
        self.reactors.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Reactor> {
        self.reactors.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Reactor> {
        self.reactors.values_mut()
    }

    pub fn is_destroyed(&self, id: &str) -> bool {
        self.get(id).is_some_and(Reactor::is_destroyed)
    }

    pub fn destroyed_map(&self) -> BTreeMap<String, bool> {
        self.reactors
            .iter()
            .map(|(k, v)| (k.clone(), v.is_destroyed()))
            .collect()
    }

    pub fn reset(&mut self) {
        for r in self.reactors.values_mut() {
            r.reset();
        }
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.reactors.len() * 32 + 8);
        out.extend_from_slice(&(self.reactors.len() as u32).to_le_bytes());
        for r in self.reactors.values() {
            out.extend_from_slice(&r.checksum_bytes());
        }
        out
    }
}
