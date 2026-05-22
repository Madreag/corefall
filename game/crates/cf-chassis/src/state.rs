use serde::{Deserialize, Serialize};

use crate::{
    outcomes::stage_from_integrity, AbilityRejectReason, AbilitySlotState, AbilityTickOutcome, ArmorLayer,
    ArmorLayerKind, ArmorMountAngles, BodyGraph, BodyZone, CameraAnchor, ChassisAbility, ChassisAbilitySlots,
    ChassisKind, ChassisModule, ChassisSpec, ChassisStage, CriticalModuleEvent, CriticalModuleOutcome,
    EjectAccepted, EjectProgress, EjectWindow, FailureCascade, LayerDamage, LayerGlance, ModuleKind,
    ModuleStateKind, ModuleTransition, PilotState, RepairOutcome, SalvageOutcome, SpallingFragmentOutcome,
    TransitionCompleted, WeaponModifier, WeaponModifierSet, ZoneDamageOutcome, ZoneState,
};

/// Runtime mutable chassis state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisState {
    pub spec_id: String,
    pub kind: ChassisKind,
    pub stage: ChassisStage,
    pub pilot_state: PilotState,
    pub body_graph: BodyGraph,
    pub zones: Vec<ZoneState>,
    pub modules: Vec<ChassisModule>,
    pub eject_window: EjectWindow,
    /// Tick rate this chassis was instantiated with. Used to scale eject ticks
    /// from seconds — same chassis at 60 Hz vs 120 Hz produce identical real-time
    /// eject windows.
    pub tick_rate_hz: u32,
    /// Tutorial safety: lethal damage capped at Disabled / PilotInjured.
    pub tutorial_safety: bool,
    /// Mass of the chassis in kilograms (drives M5.5 impulse-to-damage routing).
    pub mass_kg: f32,
    /// True iff the weapon mounted at SOCKET_HAND_RIGHT is currently jammed.
    /// Distinct from module Failed because the rifle ITSELF jams (mechanism fault)
    /// rather than the chassis weapon-mount module failing.
    pub weapon_jammed: bool,
    /// Last reason for stage transition (for the next event emit). Cleared by
    /// the engine on read.
    pub last_stage_reason: String,
    /// Modules salvaged after wreck (populated by [`ChassisState::salvage`]).
    pub salvaged_modules: Vec<ChassisModule>,
    #[serde(default)]
    pub armor_angles: ArmorMountAngles,
    #[serde(default)]
    pub abilities: ChassisAbilitySlots,
    #[serde(default)]
    pub weapon_modifiers: WeaponModifierSet,
    #[serde(default)]
    pub camera_anchor: CameraAnchor,
    /// the 1500ms boarding transition (0 = idle).
    #[serde(default)]
    pub boarding_ticks_remaining: u32,
    /// the 1500ms disembarking transition.
    #[serde(default)]
    pub disembarking_ticks_remaining: u32,
    /// budget in ticks at the chassis tick rate (1500ms).
    #[serde(default)]
    pub transition_ticks_total: u32,
}

impl ChassisState {
    /// Build a runtime state from a spec at the given tick rate.
    pub fn from_spec(spec: &ChassisSpec, tick_rate_hz: u32, tutorial_safety: bool) -> Self {
        let tick_rate = tick_rate_hz.max(1);
        let eject_ticks = ((spec.eject_window_seconds.max(0.1)) * tick_rate as f32).round() as u32;
        // 1500ms transition window per spec § "Boarding / disembarking transitions".
        let transition_ticks_total = ((1.5_f32) * tick_rate as f32).round() as u32;
        Self {
            spec_id: spec.id.clone(),
            kind: spec.kind,
            stage: ChassisStage::Nominal,
            pilot_state: PilotState::Bound,
            body_graph: spec.body_graph.clone(),
            zones: spec.zones.clone(),
            modules: spec.modules.clone(),
            eject_window: EjectWindow {
                ticks_remaining: 0,
                ticks_total: eject_ticks.max(1),
                triggered_at_tick: 0,
            },
            tick_rate_hz: tick_rate,
            tutorial_safety,
            mass_kg: spec.mass_kg,
            weapon_jammed: false,
            last_stage_reason: String::new(),
            salvaged_modules: Vec::new(),
            armor_angles: spec.armor_angles,
            abilities: ChassisAbilitySlots::new(spec.kind, tick_rate),
            weapon_modifiers: WeaponModifierSet::new(spec.kind),
            camera_anchor: CameraAnchor::Default,
            boarding_ticks_remaining: 0,
            disembarking_ticks_remaining: 0,
            transition_ticks_total: transition_ticks_total.max(1),
        }
    }

    /// Returns `Ok(prev_anchor)` on success or a typed reason on rejection.
    pub fn set_camera_anchor(&mut self, anchor: CameraAnchor) -> Result<CameraAnchor, &'static str> {
        if anchor == CameraAnchor::Cockpit && !self.kind.supports_cockpit_anchor() {
            return Err("camera_anchor_not_supported_by_chassis_class");
        }
        let prev = self.camera_anchor;
        self.camera_anchor = anchor;
        Ok(prev)
    }

    /// 1500ms boarding transition. Returns `true` when accepted (was idle).
    pub fn begin_boarding(&mut self) -> bool {
        if self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0 {
            return false;
        }
        self.boarding_ticks_remaining = self.transition_ticks_total;
        true
    }

    /// disembarking transition.
    pub fn begin_disembarking(&mut self) -> bool {
        if self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0 {
            return false;
        }
        self.disembarking_ticks_remaining = self.transition_ticks_total;
        true
    }

    /// is mid-transition (input rejected during).
    pub fn is_in_transition(&self) -> bool {
        self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0
    }

    /// timers. Returns the side that just completed (if any).
    pub fn tick_transitions(&mut self) -> Option<TransitionCompleted> {
        if self.boarding_ticks_remaining > 0 {
            self.boarding_ticks_remaining -= 1;
            if self.boarding_ticks_remaining == 0 {
                return Some(TransitionCompleted::Boarded);
            }
        } else if self.disembarking_ticks_remaining > 0 {
            self.disembarking_ticks_remaining -= 1;
            if self.disembarking_ticks_remaining == 0 {
                return Some(TransitionCompleted::Disembarked);
            }
        }
        None
    }

    pub fn activate_ability(&mut self, ability: ChassisAbility) -> Result<AbilitySlotState, AbilityRejectReason> {
        self.abilities.activate(ability)
    }

    pub fn tick_abilities(&mut self) -> AbilityTickOutcome {
        self.abilities.tick()
    }

    pub fn attach_weapon_modifier(&mut self, m: WeaponModifier) -> Result<bool, &'static str> {
        let before_len = self.weapon_modifiers.modifiers.len();
        self.weapon_modifiers.attach(m)?;
        Ok(self.weapon_modifiers.modifiers.len() > before_len)
    }

    pub fn detach_weapon_modifier(&mut self, m: WeaponModifier) -> bool {
        self.weapon_modifiers.detach(m)
    }

    pub fn zone(&self, zone: BodyZone) -> Option<&ZoneState> {
        self.zones.iter().find(|z| z.zone == zone)
    }

    pub fn zone_mut(&mut self, zone: BodyZone) -> Option<&mut ZoneState> {
        self.zones.iter_mut().find(|z| z.zone == zone)
    }

    pub fn module(&self, id: &str) -> Option<&ChassisModule> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn module_mut(&mut self, id: &str) -> Option<&mut ChassisModule> {
        self.modules.iter_mut().find(|m| m.id == id)
    }

    pub fn module_by_kind(&self, kind: ModuleKind) -> Option<&ChassisModule> {
        self.modules.iter().find(|m| m.kind == kind)
    }

    pub fn destroyed_zones(&self) -> Vec<BodyZone> {
        self.zones.iter().filter(|z| z.destroyed).map(|z| z.zone).collect()
    }

    /// Composite chassis integrity — averages every zone's zone_integrity. Drives
    /// HUD silhouette + stage transitions ("ArmorCracked" when avg drops below 0.6).
    pub fn integrity(&self) -> f32 {
        if self.zones.is_empty() {
            return 1.0;
        }
        let sum: f32 = self.zones.iter().map(ZoneState::zone_integrity).sum();
        sum / self.zones.len() as f32
    }

    /// Reset state to spec defaults (used by scenario.reset).
    pub fn reset(&mut self) {
        for zone in &mut self.zones {
            zone.reset();
        }
        for module in &mut self.modules {
            module.reset();
        }
        self.stage = ChassisStage::Nominal;
        self.pilot_state = PilotState::Bound;
        self.eject_window.ticks_remaining = 0;
        self.eject_window.triggered_at_tick = 0;
        self.weapon_jammed = false;
        self.last_stage_reason.clear();
        self.salvaged_modules.clear();
    }

    /// Apply damage to a specific zone with a typed cause label. Returns a
    /// [`ZoneDamageOutcome`] describing every layer/module transition the engine
    /// must emit as events.
    pub fn apply_zone_damage(&mut self, zone: BodyZone, damage: f32, cause: &str) -> ZoneDamageOutcome {
        let mut outcome = ZoneDamageOutcome::default();
        if damage <= 0.0 || !damage.is_finite() {
            return outcome;
        }
        outcome.zone = Some(zone);
        outcome.cause = cause.to_string();

        let mut remaining = damage;
        let mut layers_breached: Vec<(ArmorLayerKind, f32)> = Vec::new();
        let mut zone_destroyed = false;
        let mut wound_damage_taken = 0.0_f32;
        let mut wound_destroyed = false;

        if let Some(zs) = self.zone_mut(zone) {
            // Drain layers in canonical order.
            for kind in [ArmorLayerKind::External, ArmorLayerKind::Internal, ArmorLayerKind::Core] {
                if remaining <= 0.0 {
                    break;
                }
                let Some(layer) = zs.layers.iter_mut().find(|l| l.kind == kind) else {
                    continue;
                };
                if layer.hp <= 0.0 {
                    continue;
                }
                let effective = (remaining - layer.hardness).max(0.0);
                if effective <= 0.0 {
                    // Hardness absorbed the hit; record a glance event.
                    outcome.glances.push(LayerGlance {
                        layer: kind,
                        absorbed: remaining,
                    });
                    remaining = 0.0;
                    break;
                }
                let taken = effective.min(layer.hp);
                layer.hp -= taken;
                outcome.layer_damage.push(LayerDamage {
                    layer: kind,
                    damage: taken,
                    hp_after: layer.hp,
                    breached: layer.is_breached(),
                });
                if layer.is_breached() {
                    layers_breached.push((kind, layer.hp_max));
                }
                remaining -= taken;
            }
            // Spill into wound HP if all layers breached.
            if remaining > 0.0 {
                let wound_take = remaining.min(zs.wound_hp);
                zs.wound_hp -= wound_take;
                wound_damage_taken = wound_take;
                remaining -= wound_take;
                if zs.wound_hp <= 0.0 && !zs.destroyed {
                    zs.destroyed = true;
                    zone_destroyed = true;
                    wound_destroyed = true;
                }
            }
        }
        outcome.layers_breached = layers_breached;
        outcome.wound_damage = wound_damage_taken;
        outcome.zone_destroyed = zone_destroyed;
        let _ = wound_destroyed; // tracked for future routing into actor HP coefficient
        outcome.actor_hp_damage = remaining.max(0.0);
        // INSTANT DEATH per CCCP decapitation rule. Tutorial-safety overrides
        // (`tutorial_safety=true` caps damage at PilotInjured) suppress lethal.
        if zone_destroyed && !self.tutorial_safety && matches!(zone, BodyZone::Head | BodyZone::Torso) {
            outcome.lethal = true;
        }

        // Propagate to module health bound to this zone.
        if zone_destroyed {
            let modules_to_update: Vec<(String, ModuleStateKind, String)> = self
                .modules
                .iter_mut()
                .filter(|m| m.bound_zone == zone && m.state != ModuleStateKind::NotPresent)
                .map(|m| {
                    m.hp = 0.0;
                    m.state = ModuleStateKind::Failed;
                    m.last_reason = "bound_zone_destroyed".to_string();
                    (m.id.clone(), ModuleStateKind::Failed, m.last_reason.clone())
                })
                .collect();
            outcome.module_transitions.extend(
                modules_to_update
                    .into_iter()
                    .map(|(id, state, reason)| ModuleTransition { id, state, reason }),
            );
            // Sever joints connected to this zone.
            for joint in &mut self.body_graph.joints {
                if (joint.parent == zone || joint.child == zone) && joint.intact {
                    joint.intact = false;
                    outcome.joints_severed.push(joint.id.clone());
                }
            }
        } else {
            // Non-destroying damage to a zone with low integrity also degrades modules
            // bound to it (e.g., torso cracked → jet warning).
            let integrity = self.zone(zone).map_or(1.0, ZoneState::zone_integrity);
            let new_state = stage_from_integrity(integrity);
            if new_state != ModuleStateKind::Nominal && new_state != ModuleStateKind::NotPresent {
                let updates: Vec<(String, ModuleStateKind, String)> = self
                    .modules
                    .iter_mut()
                    .filter(|m| m.bound_zone == zone && m.state.is_present())
                    .filter_map(|m| {
                        if (new_state as u8) > (m.state as u8) {
                            m.state = new_state;
                            m.last_reason = "bound_zone_damaged".to_string();
                            // Drain module HP proportional to its bound zone.
                            m.hp = (m.hp_max * integrity).clamp(0.0, m.hp_max);
                            Some((m.id.clone(), new_state, m.last_reason.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                outcome
                    .module_transitions
                    .extend(
                        updates
                            .into_iter()
                            .map(|(id, state, reason)| ModuleTransition { id, state, reason }),
                    );
            }
        }

        outcome
    }

    /// Apply damage directly to a module (e.g., direct hit on the jet module).
    pub fn apply_module_damage(&mut self, module_id: &str, damage: f32, cause: &str) -> Option<ModuleTransition> {
        if damage <= 0.0 || !damage.is_finite() {
            return None;
        }
        let module = self.module_mut(module_id)?;
        if !module.state.is_present() {
            return None;
        }
        module.hp = (module.hp - damage).max(0.0);
        let new_state = stage_from_integrity(module.integrity());
        if new_state == module.state && new_state != ModuleStateKind::Nominal {
            return None;
        }
        if (new_state as u8) > (module.state as u8) {
            module.state = new_state;
            module.last_reason = cause.to_string();
            Some(ModuleTransition {
                id: module.id.clone(),
                state: new_state,
                reason: module.last_reason.clone(),
            })
        } else {
            None
        }
    }

    /// damage to a module and surface its cascade outcome (ammo cookoff,
    /// engine fire, optics blind, etc.). The engine wires this into
    /// `module.ammo_rack_cooking` / `module.ammo_rack_detonated` /
    /// `module.spalling_damage` event emitters.
    pub fn apply_critical_module_damage(
        &mut self,
        module_id: &str,
        damage: f32,
        cause: &str,
    ) -> Option<CriticalModuleOutcome> {
        let transition = self.apply_module_damage(module_id, damage, cause);
        let module = self.module(module_id)?;
        let mut cascade_events: Vec<CriticalModuleEvent> = Vec::new();
        let module_id = module.id.clone();
        let module_kind = module.kind;
        let module_state = module.state;
        let cascade = module.failure_cascade;
        let ammo_remaining = module.ammo_quantity_remaining;
        // ONLY when the module's `state` has advanced past its previous
        // `last_cascade_emitted_state` — otherwise multiple zone hits in a
        // single tick (or any other rapid succession of calls while
        // already inside Warning) would re-cook ammo, re-leak oil,
        // re-advance pressure, etc. once per call. PilotDirectHit is
        // intentionally per-hit (damage-amount-bearing event) and gates
        // on damage > 0 instead.
        let last_emitted = module
            .last_cascade_emitted_state;
        let tier_advanced = (module_state as u8) > (last_emitted as u8);
        // 1/3 of remaining ammo; severe-hit (Failed state) detonates the rack.
        if cascade == FailureCascade::AmmoCookoff && tier_advanced {
            match module_state {
                ModuleStateKind::Warning if ammo_remaining > 0 => {
                    let cook = ammo_remaining / 3;
                    // borrow again to mutate counters
                    if let Some(m) = self.module_mut(&module_id) {
                        m.rounds_cooked_off = m.rounds_cooked_off.saturating_add(cook);
                        m.ammo_quantity_remaining = m.ammo_quantity_remaining.saturating_sub(cook);
                    }
                    cascade_events.push(CriticalModuleEvent::AmmoCooking { rounds_cooked: cook });
                }
                ModuleStateKind::Failed => {
                    let detonated = ammo_remaining;
                    if let Some(m) = self.module_mut(&module_id) {
                        m.rounds_cooked_off = m.rounds_cooked_off.saturating_add(detonated);
                        m.ammo_quantity_remaining = 0;
                    }
                    cascade_events.push(CriticalModuleEvent::AmmoDetonated {
                        rounds_detonated: detonated,
                    });
                    // Catastrophic — flag chassis as gibbed unless tutorial-safe.
                    if !self.tutorial_safety {
                        self.stage = ChassisStage::Gibbed;
                        self.pilot_state = PilotState::Lost;
                        self.last_stage_reason = "ammo_rack_detonated".to_string();
                    }
                }
                _ => {}
            }
        }
        // oil; destroyed engine cascades fire. Tier-gated.
        if cascade == FailureCascade::EngineFire && tier_advanced {
            if matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed) {
                if let Some(m) = self.module_mut(&module_id) {
                    m.oil_level = (m.oil_level - 0.5).max(0.0);
                }
                cascade_events.push(CriticalModuleEvent::EngineOilLeak);
            }
            if module_state == ModuleStateKind::Failed {
                cascade_events.push(CriticalModuleEvent::EngineFire);
            }
        }
        // Reactor pressure has its own internal crossed flag below (it only
        // emits when `pressure` > prior `pressure_state`); leave that as the
        // authoritative dedupe for reactors.
        if cascade == FailureCascade::ReactorOverpressure {
            let pressure = match module_state {
                ModuleStateKind::Degraded => 1,
                ModuleStateKind::Warning => 2,
                ModuleStateKind::Failed => 4,
                ModuleStateKind::Nominal | ModuleStateKind::NotPresent => 0,
            };
            let mut crossed = false;
            if let Some(m) = self.module_mut(&module_id) {
                if pressure > m.pressure_state {
                    m.pressure_state = pressure;
                    crossed = true;
                }
            }
            if crossed {
                cascade_events.push(CriticalModuleEvent::ReactorPressureAdvanced { tier: pressure });
            }
        }
        // Per-hit cascade (carries damage payload); gates on damage>0 instead
        // of tier_advanced so multiple hits all surface their damage.
        if cascade == FailureCascade::PilotDirectDamage
            && damage > 0.0
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::PilotDirectHit { damage });
            // Promote pilot to Injured when cockpit takes damage.
            if matches!(self.pilot_state, PilotState::Bound) {
                self.pilot_state = PilotState::Injured;
            }
        }
        if cascade == FailureCascade::SightImpairment
            && tier_advanced
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::OpticsImpaired {
                blind: module_state == ModuleStateKind::Failed,
            });
        }
        if cascade == FailureCascade::MobilityLoss
            && tier_advanced
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::MobilityReduced {
                immobile: module_state == ModuleStateKind::Failed,
            });
        }
        // subsequent same-state calls don't refire the tier-gated cascades.
        if tier_advanced {
            if let Some(m) = self.module_mut(&module_id) {
                m.last_cascade_emitted_state = module_state;
            }
        }
        if transition.is_none() && cascade_events.is_empty() {
            return None;
        }
        Some(CriticalModuleOutcome {
            module_id,
            module_kind,
            transition,
            cascade_events,
        })
    }

    /// impact point in chassis-local space, fire 1-3 deterministic spalling
    /// fragments into the chassis and report each fragment's module hit.
    /// `seed` is the caller-supplied deterministic PRNG seed (NO thread_rng).
    pub fn spawn_spalling_fragments(
        &mut self,
        impact_local: (f32, f32),
        fragment_count: u32,
        original_damage: f32,
        seed: u64,
    ) -> Vec<SpallingFragmentOutcome> {
        let mut outcomes: Vec<SpallingFragmentOutcome> = Vec::new();
        let count = fragment_count.clamp(1, 3);
        for i in 0..count {
            // Deterministic fragment direction within ±30° cone (per spec).
            let frag_seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i as u64);
            let angle_norm = ((frag_seed % 1024) as f32 / 1024.0) - 0.5; // -0.5..+0.5
            let angle_rad = angle_norm * std::f32::consts::PI * (60.0 / 180.0);
            let dx = angle_rad.cos();
            let dy = angle_rad.sin();
            // Per spec: per-fragment damage = 20-50% of original.
            let damage_frac = 0.2 + ((frag_seed >> 10) % 30) as f32 / 100.0;
            let damage = original_damage * damage_frac;
            // Pick the first module whose local_aabb is on the ray. Stub
            // walks the module list and returns the first positioned hit.
            let target_id: Option<String> = self
                .modules
                .iter()
                .find(|m| m.state.is_present() && m.local_aabb.is_positioned())
                .map(|m| m.id.clone());
            let _ = (dx, dy, impact_local);
            if let Some(id) = target_id {
                let transition = self.apply_module_damage(&id, damage, "spalling_fragment");
                outcomes.push(SpallingFragmentOutcome {
                    fragment_id: format!("frag_{i}"),
                    module_id: id,
                    damage,
                    transition,
                });
            }
        }
        outcomes
    }

    /// Stage transition pass — call once per tick (or right after damage application).
    /// Updates `self.stage` based on aggregate damage, module health, and pilot state.
    /// Returns `Some(new_stage)` iff the stage advanced.
    pub fn recompute_stage(&mut self) -> Option<ChassisStage> {
        let prev = self.stage;
        let mut next = prev;

        // Composite cues.
        let core_integrity_min = self.zones.iter().map(ZoneState::core_integrity).fold(1.0_f32, f32::min);
        let any_zone_destroyed = self.zones.iter().any(|z| z.destroyed);
        let any_module_failed = self
            .modules
            .iter()
            .any(|m| m.state == ModuleStateKind::Failed && m.kind != ModuleKind::WeaponMount);
        let any_module_warning = self.modules.iter().any(|m| m.state == ModuleStateKind::Warning);
        let weapon_mount_failed = self
            .module_by_kind(ModuleKind::WeaponMount)
            .is_some_and(|m| m.state == ModuleStateKind::Failed);
        let armor_cracked = self.integrity() <= 0.5;
        let disabled = core_integrity_min <= 0.0 && any_zone_destroyed;
        let pilot_injured = matches!(self.pilot_state, PilotState::Injured);
        let pilot_ejected = matches!(self.pilot_state, PilotState::Ejected | PilotState::Extracted);
        let pilot_lost = self.pilot_state.is_lost();
        let chassis_wrecked = self.zone(BodyZone::Torso).is_some_and(|z| z.destroyed) && disabled;

        // Advance stage by precedence (last-wins for "more severe"). Never step
        // backwards except via explicit repair.
        if prev <= ChassisStage::Nominal && self.integrity() < 1.0 {
            next = ChassisStage::Degraded;
        }
        if any_module_warning && next < ChassisStage::ModuleWarning {
            next = ChassisStage::ModuleWarning;
        }
        if any_module_failed && next < ChassisStage::ModuleFailed {
            next = ChassisStage::ModuleFailed;
        }
        if (self.weapon_jammed || weapon_mount_failed) && next < ChassisStage::WeaponJammed {
            next = ChassisStage::WeaponJammed;
        }
        if armor_cracked && next < ChassisStage::ArmorCracked {
            next = ChassisStage::ArmorCracked;
        }
        if disabled && next < ChassisStage::Disabled {
            next = ChassisStage::Disabled;
        }
        if pilot_injured && next < ChassisStage::PilotInjured {
            next = ChassisStage::PilotInjured;
        }
        if matches!(self.pilot_state, PilotState::Ejecting) && next < ChassisStage::Eject {
            next = ChassisStage::Eject;
        }
        if matches!(self.pilot_state, PilotState::BailedTooLate) && next < ChassisStage::BailTooLate {
            next = ChassisStage::BailTooLate;
        }
        // Wreck stage requires either disable + ejected_or_lost OR torso destroyed.
        if ((disabled && (pilot_ejected || pilot_lost)) || chassis_wrecked) && next < ChassisStage::Wreck {
            next = ChassisStage::Wreck;
        }
        // Gibbed is reserved for explicit catastrophic damage flagged via
        // [`ChassisState::mark_gibbed`].

        // Tutorial-safety floor: never advance beyond PilotInjured.
        if self.tutorial_safety && next > ChassisStage::PilotInjured {
            next = ChassisStage::PilotInjured;
        }
        if next != prev {
            self.stage = next;
            Some(next)
        } else {
            None
        }
    }

    /// Mark this chassis as gibbed (catastrophic explosion). Used by M5.6+ reactions.
    pub fn mark_gibbed(&mut self, reason: &str) {
        if !self.tutorial_safety {
            self.stage = ChassisStage::Gibbed;
            self.pilot_state = PilotState::Lost;
            self.last_stage_reason = reason.to_string();
        }
    }

    /// Trigger an eject sequence. Returns `Some(EjectAccepted { ticks_total })` if
    /// the chassis accepted the eject; `None` if the pilot is already ejected/lost
    /// or the chassis stage forbids it.
    pub fn attempt_eject(&mut self, tick: u64) -> Option<EjectAccepted> {
        // Cannot eject if already out of the chassis.
        if !self.pilot_state.is_in_chassis() {
            return None;
        }
        // Tutorial safety blocks "real" eject; it returns a no-op extracted instead.
        if self.tutorial_safety {
            self.pilot_state = PilotState::Extracted;
            self.eject_window.triggered_at_tick = tick;
            self.eject_window.ticks_remaining = 0;
            return Some(EjectAccepted {
                ticks_total: 0,
                tutorial_extract: true,
            });
        }
        self.pilot_state = PilotState::Ejecting;
        self.eject_window.triggered_at_tick = tick;
        self.eject_window.ticks_remaining = self.eject_window.ticks_total;
        self.last_stage_reason = "pilot_ejected".to_string();
        Some(EjectAccepted {
            ticks_total: self.eject_window.ticks_total,
            tutorial_extract: false,
        })
    }

    /// Tick the eject sequence. Returns `Some(EjectProgress)` when the sequence
    /// transitions (started→ejected, ejected→bail-too-late) so the engine emits
    /// events.
    pub fn tick_eject(&mut self) -> Option<EjectProgress> {
        if !matches!(self.pilot_state, PilotState::Ejecting) {
            return None;
        }
        if self.eject_window.ticks_remaining > 0 {
            self.eject_window.ticks_remaining -= 1;
        }
        if self.eject_window.ticks_remaining == 0 {
            // If the chassis is already wrecked / gibbed before the sequence
            // completed, the pilot bailed too late.
            if matches!(self.stage, ChassisStage::Wreck | ChassisStage::Gibbed) {
                self.pilot_state = PilotState::BailedTooLate;
                return Some(EjectProgress::BailedTooLate);
            }
            self.pilot_state = PilotState::Ejected;
            return Some(EjectProgress::Ejected);
        }
        None
    }

    /// Mark the pilot as extracted (reached safety zone).
    pub fn mark_pilot_extracted(&mut self) -> bool {
        if matches!(self.pilot_state, PilotState::Ejected) {
            self.pilot_state = PilotState::Extracted;
            true
        } else {
            false
        }
    }

    /// Mark the pilot as lost (chassis exploded with pilot inside).
    pub fn mark_pilot_lost(&mut self, reason: &str) -> bool {
        if !self.pilot_state.is_lost() {
            self.pilot_state = PilotState::Lost;
            self.last_stage_reason = reason.to_string();
            true
        } else {
            false
        }
    }

    /// Repair a zone (heal all its layers + wound back to spec). Stage may step
    /// back at most one level.
    pub fn repair_zone(&mut self, zone: BodyZone, reason: &str) -> Option<RepairOutcome> {
        let zs = self.zone_mut(zone)?;
        let was_destroyed = zs.destroyed;
        zs.reset();
        // Resurrect modules whose bound zone is the repaired one — they go back to
        // Nominal with full HP.
        let restored: Vec<String> = self
            .modules
            .iter_mut()
            .filter(|m| m.bound_zone == zone && m.state.is_present())
            .filter_map(|m| {
                let prev = m.state;
                m.hp = m.hp_max;
                m.state = ModuleStateKind::Nominal;
                // emission high-water mark so re-damage re-fires its
                // tier-crossing cascades.
                m.last_cascade_emitted_state = ModuleStateKind::Nominal;
                m.pressure_state = 0;
                m.oil_level = 1.0;
                m.coolant_level = 1.0;
                m.last_reason = format!("repaired_via:{reason}");
                if prev != ModuleStateKind::Nominal {
                    Some(m.id.clone())
                } else {
                    None
                }
            })
            .collect();
        // Joint mend.
        for joint in &mut self.body_graph.joints {
            if joint.parent == zone || joint.child == zone {
                joint.intact = true;
            }
        }
        // Step stage back one slot if any progress.
        let prev_stage = self.stage;
        self.stage = match self.stage {
            ChassisStage::Degraded => ChassisStage::Nominal,
            ChassisStage::ModuleWarning => ChassisStage::Degraded,
            ChassisStage::ModuleFailed => ChassisStage::ModuleWarning,
            ChassisStage::WeaponJammed => ChassisStage::ModuleFailed,
            ChassisStage::ArmorCracked => ChassisStage::WeaponJammed,
            ChassisStage::Disabled => ChassisStage::ArmorCracked,
            ChassisStage::PilotInjured => ChassisStage::Disabled,
            other => other,
        };
        Some(RepairOutcome {
            zone,
            was_destroyed,
            modules_restored: restored,
            prev_stage,
            new_stage: self.stage,
            reason: reason.to_string(),
        })
    }

    /// Repair a specific module (e.g., field-deployed repair drone).
    pub fn repair_module(&mut self, module_id: &str, reason: &str) -> Option<ModuleTransition> {
        let module = self.module_mut(module_id)?;
        if !module.state.is_present() {
            return None;
        }
        let prev = module.state;
        module.hp = module.hp_max;
        module.state = ModuleStateKind::Nominal;
        // high-water mark on repair so future damage that re-crosses a
        // tier emits its cascade event again.
        module.last_cascade_emitted_state = ModuleStateKind::Nominal;
        // Repair also restores reactor pressure_state + oil/coolant so
        // the cascade pipeline resumes at full nominal reserves.
        module.pressure_state = 0;
        module.oil_level = 1.0;
        module.coolant_level = 1.0;
        module.last_reason = format!("repaired:{reason}");
        if prev != ModuleStateKind::Nominal {
            Some(ModuleTransition {
                id: module.id.clone(),
                state: ModuleStateKind::Nominal,
                reason: module.last_reason.clone(),
            })
        } else {
            None
        }
    }

    /// `ScenarioChassis::initial_stage` so a scenario can spawn a chassis
    /// already in `Wreck` / `Disabled` for salvage proof. Does NOT recompute
    /// from zone/module integrity — callers that need integrity-driven stage
    /// should use [`Self::recompute_stage`] instead.
    pub fn force_stage(&mut self, stage: ChassisStage) {
        self.stage = stage;
        self.last_stage_reason = format!("scenario_force:{stage:?}");
    }

    /// Salvage a wrecked chassis: pull every non-Failed module into
    /// `salvaged_modules` and emit a [`SalvageOutcome`]. Returns `None` if the
    /// chassis is not wreck-stage.
    pub fn salvage(&mut self, reason: &str) -> Option<SalvageOutcome> {
        if !matches!(
            self.stage,
            ChassisStage::Wreck | ChassisStage::Disabled | ChassisStage::Gibbed
        ) {
            return None;
        }
        let mut salvaged_ids: Vec<String> = Vec::new();
        for module in &mut self.modules {
            if !module.state.is_present() {
                continue;
            }
            // Modules below 25% integrity are too broken to salvage.
            if module.integrity() < 0.25 {
                continue;
            }
            module.last_reason = format!("salvaged:{reason}");
            self.salvaged_modules.push(module.clone());
            salvaged_ids.push(module.id.clone());
        }
        // Move the chassis into Wreck if it wasn't already.
        if self.stage != ChassisStage::Gibbed {
            self.stage = ChassisStage::Wreck;
        }
        Some(SalvageOutcome {
            salvaged_module_ids: salvaged_ids,
            reason: reason.to_string(),
        })
    }

    /// Mark the rifle as jammed. Distinct from the weapon-mount module's `Failed`
    /// state (which is structural). A jam clears on `clear_jam`.
    pub fn jam_weapon(&mut self, reason: &str) -> bool {
        if self.weapon_jammed {
            return false;
        }
        self.weapon_jammed = true;
        self.last_stage_reason = format!("weapon_jammed:{reason}");
        true
    }

    pub fn clear_jam(&mut self) -> bool {
        if !self.weapon_jammed {
            return false;
        }
        self.weapon_jammed = false;
        true
    }

    /// Hash bytes for the deterministic checksum extension. Layout-stable.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.push(self.kind as u8);
        out.push(self.stage as u8);
        out.push(self.pilot_state as u8);
        out.push(u8::from(self.weapon_jammed));
        out.extend_from_slice(&self.eject_window.ticks_remaining.to_le_bytes());
        for zone in BodyZone::all() {
            let z = self
                .zones
                .iter()
                .find(|z| z.zone == *zone)
                .cloned()
                .unwrap_or_else(|| ZoneState::new(*zone, Vec::new(), 0.0));
            for layer in [ArmorLayerKind::External, ArmorLayerKind::Internal, ArmorLayerKind::Core] {
                let l = z
                    .layers
                    .iter()
                    .find(|l| l.kind == layer)
                    .cloned()
                    .unwrap_or(ArmorLayer {
                        kind: layer,
                        hp: 0.0,
                        hp_max: 0.0,
                        hardness: 0.0,
                    });
                out.extend_from_slice(&(l.hp * 1024.0).round().to_bits().to_le_bytes());
            }
            out.extend_from_slice(&(z.wound_hp * 1024.0).round().to_bits().to_le_bytes());
            out.push(u8::from(z.destroyed));
        }
        let mut module_ids: Vec<&ChassisModule> = self.modules.iter().collect();
        module_ids.sort_by(|a, b| a.id.cmp(&b.id));
        out.extend_from_slice(&(module_ids.len() as u32).to_le_bytes());
        for m in module_ids {
            out.push(m.state as u8);
            out.extend_from_slice(&(m.hp * 1024.0).round().to_bits().to_le_bytes());
        }
        out
    }
}
