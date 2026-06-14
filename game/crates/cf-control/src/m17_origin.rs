//! M17 § Origin reaction + per-origin resource model — engine integration.
//!
//! [`M0Engine::tick_m17_origin`] runs once per advanced tick (after the M16C
//! psych pass so afflictions + pain are settled) and drives the per-origin
//! survival simulation:
//!   - **lazy resource seeding** from the origin profile (blood / oil / power /
//!     caloric / bio_fluid / oxygen) on first sight,
//!   - **resource drain** (blood from bleeding, oil from leaks, caloric +
//!     power sustain, overclock heat/power), with `resource.changed /
//!     critical / depleted` events at the 30 % / 10 % / 0 % bands,
//!   - **oxygen / vacuum / helmet-breach** drain (3× on breach; HP drain +
//!     hypoxia at empty),
//!   - **concussion decay + recovery** (per-origin decay; `concussion.recovered`
//!     at return-to-Clear) and **internal-shock decay** (robots),
//!   - **overclock / downclock** (action-speed + heat band → thermal throttle),
//!   - **death triggers**: power-survival origins go INERT (recoverable) at
//!     power 0; organics bleed toward DYING at blood 0.
//!
//! The control surface (`m17_*`) mirrors the `m16*` engine API: each acquires
//! the state lock, mutates per-actor state, emits the matching replay event,
//! and returns a value for cfctl + the acceptance tests.

use serde_json::json;

use cf_actor::concussion::{ConcussionBand};
use cf_actor::origin::{Origin, OriginProfile};
use cf_actor::overclock::{self, ThermalBand};
use cf_actor::oxygen::{tick_oxygen, OxygenTickInput};
use cf_actor::{ActorId, Status};
use cf_affliction::M16AfflictionKind;
use cf_replay::Recorder;
use cf_sim_core::Tick;

use crate::engine::{EngineMutable, M0Engine};

// --- drain rates (spec § "Resource depletion mechanics"; per second) ---
/// Bleed-rate scale: an affliction severity of 1.0 ≈ a femoral bleed (10 mL/s).
const BLEED_RATE_ML_PER_S_AT_FULL: f32 = 10.0;
/// Oil-leak rate scale: a severity-1.0 leak ≈ a severed line (5 mL/s).
const OIL_LEAK_RATE_ML_PER_S_AT_FULL: f32 = 5.0;
/// Caloric base sustain drain (~5 hours per full 100 stack at rest).
const CALORIC_SUSTAIN_PER_S: f32 = 100.0 / (5.0 * 3600.0);
/// Power base sustain draw (kWh/s) for a power-survival body.
const POWER_SUSTAIN_KWH_PER_S: f32 = 0.005;
/// Vacuum pressure ceiling (kPa) below which an oxygen-requiring origin is
/// exposed (mirrors `cf_atmos::SUIT_PRESSURE_MIN_KPA`).
const VACUUM_PRESSURE_KPA: f32 = 11.0;
/// Reserve-mode power fraction: below this a power-survival origin can't fire.
const POWER_RESERVE_FRACTION: f32 = 0.10;
/// HP/s bled out once blood is below the critical floor.
const BLEEDOUT_HP_PER_S: f32 = 3.0;
/// Ticks at the critical thermal band before a module is damaged.
const THERMAL_CRITICAL_MODULE_TICKS: u32 = 1;

/// Resource identity (drives the event `kind` field + the critical-band dedupe
/// key). Discriminants are stable so the dedupe map survives reorders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResKind {
    Blood = 0,
    Oil = 1,
    Power = 2,
    Caloric = 3,
    BioFluid = 4,
}

impl ResKind {
    fn as_str(self) -> &'static str {
        match self {
            ResKind::Blood => "blood",
            ResKind::Oil => "oil",
            ResKind::Power => "power",
            ResKind::Caloric => "caloric",
            ResKind::BioFluid => "bio_fluid",
        }
    }
    fn id(self) -> u8 {
        self as u8
    }
}

/// 0 = nominal, 1 = ≤30 %, 2 = ≤10 %, 3 = empty.
fn critical_band(fraction: f32) -> u8 {
    if fraction <= 0.0 {
        3
    } else if fraction <= 0.10 {
        2
    } else if fraction <= 0.30 {
        1
    } else {
        0
    }
}

impl M0Engine {
    /// Per-tick origin/resource pass. See module docs.
    pub(crate) fn tick_m17_origin(&self, tick: Tick, sim_time_ms: f64) {
        let tick_rate_hz = self.config.tick_rate_hz.max(1);
        let dt = 1.0 / tick_rate_hz as f32;
        let recorder = &self.recorder;
        let mut state = self.state.write().expect("engine state poisoned for m17 origin tick");
        if state.actor_state.is_none() {
            return;
        }
        let EngineMutable {
            actor_state,
            m16_affliction_by_actor: aff_map,
            m16_affliction_registry: aff_registry,
            m17_origin_registry: registry,
            m17_seeded_actors: seeded,
            m17_internal_shock_dose: is_dose_map,
            m17_internal_shock_band: is_band_map,
            m17_helmet_breached: breached,
            m17_resource_critical_band: crit_band,
            m17_power_by_actor: power_map,
            m17_oxygen_tank_by_actor: tank_map,
            m17_thermal_band: thermal_map,
            m17_death_cause: death_cause,
            m9_concussion_dose: conc_dose_map,
            m9_concussion_band: conc_band_map,
            ..
        } = &mut *state;
        let world = actor_state.as_mut().expect("checked actor_state is_some");

        let ids: Vec<ActorId> = world.world.actors.keys().copied().collect();
        for aid in ids {
            let origin = {
                let Some(actor) = world.world.actors.get(&aid) else { continue };
                actor.origin()
            };
            let profile = *registry.profile(origin);

            // ----- 1) Lazy resource seeding -----
            if !seeded.contains(&aid) {
                seeded.insert(aid);
                if let Some(actor) = world.world.actors.get_mut(&aid) {
                    // Only seed pools the origin uses + only if unseeded (0).
                    let seed = profile.seed_resources();
                    if profile.has_blood() && actor.resources.blood <= 0.0 {
                        actor.resources.blood = seed.blood;
                    }
                    if profile.has_bio_fluid() && actor.resources.bio_fluid <= 0.0 {
                        actor.resources.bio_fluid = seed.bio_fluid;
                    }
                    if profile.has_oil() && actor.resources.oil <= 0.0 {
                        actor.resources.oil = seed.oil;
                    }
                    if profile.has_power() && actor.resources.power <= 0.0 {
                        actor.resources.power = seed.power;
                        actor.resources.battery_charge = 100.0;
                    }
                    if profile.has_caloric() && actor.resources.caloric_energy <= 0.0 {
                        actor.resources.caloric_energy = seed.caloric_energy;
                    }
                    if profile.oxygen_required && actor.resources.oxygen_supply <= 0.0 {
                        actor.resources.oxygen_supply = seed.oxygen_supply;
                    }
                }
                power_map
                    .entry(aid)
                    .or_insert_with(|| cf_actor::power::ActorPower::for_origin(origin));
                if profile.oxygen_required {
                    tank_map.entry(aid).or_insert_with(|| {
                        cf_actor::oxygen::OxygenTank::full(cf_actor::oxygen::OxygenTankTier::Compressed)
                    });
                }
            }

            // Skip the rest for dead actors (but INERT robots still tick so they
            // can be revived + so their power reads are stable).
            let status = world.world.actors.get(&aid).map(|a| a.status).unwrap_or(Status::Dead);
            if matches!(status, Status::Dead) {
                continue;
            }

            // Read the afflictions feeding the drains.
            let (bleed_sev, oil_leak_sev) = aff_map.get(&aid).map_or((0.0, 0.0), |a| {
                (
                    a.severity_of(M16AfflictionKind::Bleeding),
                    a.severity_of(M16AfflictionKind::OilLeaking)
                        + a.severity_of(M16AfflictionKind::CoolantLeaking),
                )
            });

            // ----- 2) Resource drain -----
            let mut events: Vec<ResourceDelta> = Vec::new();
            if let Some(actor) = world.world.actors.get_mut(&aid) {
                // Blood / bio-fluid bleed-out (minus natural clot).
                if profile.has_blood() {
                    let rate = (bleed_sev * BLEED_RATE_ML_PER_S_AT_FULL - profile.clot_rate_ml_per_s).max(0.0);
                    drain_resource(&mut actor.resources.blood, rate * dt, &mut events, ResKind::Blood);
                }
                if profile.has_bio_fluid() {
                    let rate = (bleed_sev * BLEED_RATE_ML_PER_S_AT_FULL - profile.clot_rate_ml_per_s).max(0.0);
                    drain_resource(&mut actor.resources.bio_fluid, rate * dt, &mut events, ResKind::BioFluid);
                }
                // Oil / coolant leak (no clot).
                if profile.has_oil() {
                    let rate = oil_leak_sev * OIL_LEAK_RATE_ML_PER_S_AT_FULL;
                    drain_resource(&mut actor.resources.oil, rate * dt, &mut events, ResKind::Oil);
                }
                // Caloric sustain.
                if profile.has_caloric() {
                    let sprint = if actor.sprint_active { 0.5 } else { 0.0 };
                    drain_resource(
                        &mut actor.resources.caloric_energy,
                        (CALORIC_SUSTAIN_PER_S + sprint) * dt,
                        &mut events,
                        ResKind::Caloric,
                    );
                }
                // Power sustain + overclock drain.
                if profile.has_power() {
                    let oc = overclock::overclock_power_drain_per_s(actor.overclock.tier);
                    drain_resource(
                        &mut actor.resources.power,
                        (POWER_SUSTAIN_KWH_PER_S + oc) * dt,
                        &mut events,
                        ResKind::Power,
                    );
                }
            }

            // ----- 3) Oxygen / vacuum / helmet breach -----
            let mut o2_hp_drain = 0.0f32;
            if profile.oxygen_required {
                let (pressure_kpa, helmet_sealed, o2_now) = {
                    let actor = world.world.actors.get(&aid).unwrap();
                    (
                        actor.atmosphere_sample.pressure_kpa,
                        actor.body_armor.helmet_seal_active() || actor.body_armor.dive_suit_equipped(),
                        actor.resources.oxygen_supply,
                    )
                };
                let vacuum_exposed = pressure_kpa < VACUUM_PRESSURE_KPA;
                let is_breached = breached.contains(&aid);
                if vacuum_exposed {
                    let res = tick_oxygen(
                        o2_now,
                        OxygenTickInput {
                            oxygen_required: true,
                            helmet_sealed,
                            helmet_breached: is_breached,
                            vacuum_exposed: true,
                            consumption_modifier: 1.0,
                            dt_seconds: dt,
                        },
                    );
                    if (res.to_seconds - o2_now).abs() > f32::EPSILON {
                        if let Some(actor) = world.world.actors.get_mut(&aid) {
                            actor.resources.oxygen_supply = res.to_seconds;
                        }
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "origin",
                            "oxygen_supply_changed",
                            json!({
                                "actor_id": aid.0,
                                "from_s": res.from_seconds,
                                "to_s": res.to_seconds,
                                "source": if is_breached { "helmet_breach" } else { "exhaled" },
                            }),
                            None,
                        );
                    }
                    o2_hp_drain = res.hp_drain;
                    // Vacuum without an effective seal (or empty reserve) →
                    // hypoxia stacks (spec scenarios 8 + 11).
                    if !helmet_sealed || is_breached || res.to_seconds <= 0.0 {
                        let actor_aff = aff_map.entry(aid).or_default();
                        let _ = cf_affliction::apply_affliction(
                            actor_aff,
                            aid.0,
                            M16AfflictionKind::Hypoxic,
                            0.1,
                            aff_registry,
                            tick.0,
                            tick_rate_hz,
                            format!("vacuum_hypoxia:{}", tick.0),
                        );
                    }
                }
            }

            // ----- 4) Concussion decay + recovery (organic / g-load origins) -----
            if profile.accumulates_g_load {
                let dose = conc_dose_map.get(&aid).copied().unwrap_or(0.0);
                if dose > 0.0 {
                    let new_dose = (dose - profile.dose_decay_per_s * dt).max(0.0);
                    conc_dose_map.insert(aid, new_dose);
                    let prev_band = conc_band_map.get(&aid).copied().unwrap_or("Clear");
                    let new_band = ConcussionBand::from_dose(new_dose).as_str();
                    if let Some(actor) = world.world.actors.get_mut(&aid) {
                        actor.resources.concussion_dose = new_dose;
                    }
                    if prev_band != new_band {
                        conc_band_map.insert(aid, new_band);
                        if new_band == "Clear" {
                            let _ = recorder.record(
                                tick,
                                sim_time_ms,
                                "concussion",
                                "recovered",
                                json!({ "actor_id": aid.0, "recovery_reason": "time" }),
                                None,
                            );
                        }
                    }
                }
            }

            // ----- 5) Internal-shock decay (robots / synthetic) -----
            if profile.uses_internal_shock {
                let dose = is_dose_map.get(&aid).copied().unwrap_or(0.0);
                if dose > 0.0 {
                    let new_dose = cf_actor::internal_shock::decay_dose(dose, profile.dose_decay_per_s, dt);
                    is_dose_map.insert(aid, new_dose);
                    if let Some(actor) = world.world.actors.get_mut(&aid) {
                        actor.resources.internal_shock_dose = new_dose;
                    }
                    let _ = is_band_map;
                }
            }

            // ----- 6) Overclock / downclock + thermal band -----
            if profile.uses_internal_shock {
                let (heat, oc_tier) = {
                    let actor = world.world.actors.get_mut(&aid).unwrap();
                    // Heat: overclock adds, passive dissipation removes.
                    let in_vacuum = actor.atmosphere_sample.pressure_kpa < VACUUM_PRESSURE_KPA;
                    let add = overclock::overclock_heat_per_s(actor.overclock.tier);
                    let diss = overclock::heat_dissipation_per_s(in_vacuum, false);
                    actor.resources.heat = (actor.resources.heat + (add - diss) * dt).clamp(0.0, 1.2);
                    (actor.resources.heat, actor.overclock.tier)
                };
                let band = ThermalBand::from_heat(heat);
                let prev_band = thermal_map.get(&aid).copied().unwrap_or(ThermalBand::Nominal);
                if band != prev_band {
                    thermal_map.insert(aid, band);
                    // Entering the throttle band (or skipping straight past it)
                    // from nominal starts the involuntary downclock.
                    if prev_band == ThermalBand::Nominal && band != ThermalBand::Nominal {
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "chassis",
                            "thermal_throttle_started",
                            json!({
                                "actor_id": aid.0,
                                "heat": heat,
                                "action_speed_factor": overclock::DOWNCLOCK_ACTION_SPEED,
                            }),
                            None,
                        );
                    }
                }
                // Apply throttle state + sustained-heat module damage.
                if let Some(actor) = world.world.actors.get_mut(&aid) {
                    actor.overclock.throttled = heat >= overclock::THROTTLE_BAND;
                    if actor.overclock.tier > 0 {
                        actor.overclock.sustained_ticks = actor.overclock.sustained_ticks.saturating_add(1);
                    } else {
                        actor.overclock.sustained_ticks = 0;
                    }
                }
                if band == ThermalBand::Critical || band == ThermalBand::Meltdown {
                    let sustained = world
                        .world
                        .actors
                        .get(&aid)
                        .map(|a| a.overclock.sustained_ticks)
                        .unwrap_or(0);
                    if sustained >= THERMAL_CRITICAL_MODULE_TICKS && tick.0 % tick_rate_hz as u64 == 0 {
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "internal_shock",
                            "module_damaged",
                            json!({
                                "actor_id": aid.0,
                                "module_id": "coolant_pump",
                                "damage_amount": 5.0,
                                "hit_zone": "torso",
                                "source_hit_event_id": format!("overheat:{}", tick.0),
                            }),
                            None,
                        );
                    }
                }
            }

            // ----- 7) Resource critical / depleted events (band crossings) -----
            for d in &events {
                let frac_to = d.fraction_to(&profile);
                let prev_band = crit_band.get(&(aid, d.kind.id())).copied().unwrap_or(0);
                let new_band = critical_band(frac_to);
                if new_band > prev_band {
                    crit_band.insert((aid, d.kind.id()), new_band);
                    let _ = recorder.record(
                        tick,
                        sim_time_ms,
                        "resource",
                        "changed",
                        json!({
                            "actor_id": aid.0,
                            "kind": d.kind.as_str(),
                            "from": d.from,
                            "to": d.to,
                        }),
                        None,
                    );
                    if new_band >= 3 {
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "resource",
                            "depleted",
                            json!({ "actor_id": aid.0, "kind": d.kind.as_str() }),
                            None,
                        );
                    } else {
                        let threshold_pct = if new_band == 2 { 10.0 } else { 30.0 };
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "resource",
                            "critical",
                            json!({
                                "actor_id": aid.0,
                                "kind": d.kind.as_str(),
                                "threshold_pct": threshold_pct,
                                "current": d.to,
                            }),
                            None,
                        );
                    }
                } else if new_band < prev_band {
                    crit_band.insert((aid, d.kind.id()), new_band);
                }
            }

            // ----- 8) Depletion consequences + death triggers -----
            self.apply_m17_death_and_consequences(
                aid,
                &profile,
                o2_hp_drain,
                dt,
                world,
                breached,
                death_cause,
                recorder,
                tick,
                sim_time_ms,
            );
        }
    }

    /// Apply per-origin depletion consequences: oxygen-empty HP drain +
    /// hypoxia, robot INERT at power 0, organic bleed-out at blood 0, and the
    /// reserve-mode fire lock + resource → move-speed coupling.
    #[allow(clippy::too_many_arguments)]
    fn apply_m17_death_and_consequences(
        &self,
        aid: ActorId,
        profile: &OriginProfile,
        o2_hp_drain: f32,
        dt: f32,
        world: &mut cf_actor::sim::ActorSimState,
        breached: &mut std::collections::BTreeSet<ActorId>,
        death_cause: &mut std::collections::BTreeMap<ActorId, String>,
        recorder: &Recorder,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        let Some(actor) = world.world.actors.get_mut(&aid) else {
            return;
        };
        // Oxygen-empty suffocation HP drain.
        if o2_hp_drain > 0.0 {
            let _ = actor.apply_damage(o2_hp_drain);
            death_cause
                .entry(aid)
                .or_insert_with(|| "Suffocated: oxygen supply exhausted in vacuum.".to_string());
        }

        // Power-survival origin: INERT at power 0 (recoverable), reserve-mode
        // fire lock below the reserve fraction.
        if profile.has_power() && profile.origin.is_power_survival() {
            let frac = if profile.power_max_kwh > 0.0 {
                actor.resources.power / profile.power_max_kwh
            } else {
                0.0
            };
            if actor.resources.power <= 0.0 {
                if !matches!(actor.status, Status::Inert | Status::Dead) {
                    actor.status = Status::Inert;
                    death_cause.insert(
                        aid,
                        "Went offline: power depleted (0 kWh). Recoverable via repair tool + battery.".to_string(),
                    );
                    let _ = recorder.record(
                        tick,
                        sim_time_ms,
                        "resource",
                        "cascade_offline",
                        json!({
                            "actor_id": aid.0,
                            "kind": "power",
                            "organ_id": "power_core",
                            "reason": "circuit_destroyed",
                        }),
                        None,
                    );
                }
                actor.power_fire_locked = true;
            } else {
                actor.power_fire_locked = frac < POWER_RESERVE_FRACTION;
            }
        } else {
            actor.power_fire_locked = false;
        }

        // Organic bleed-out: at blood/bio_fluid below the critical floor, HP
        // drains; at zero, push to DYING.
        let blood_like = if profile.has_blood() {
            Some((actor.resources.blood, profile.blood_max_ml))
        } else if profile.has_bio_fluid() {
            Some((actor.resources.bio_fluid, profile.bio_fluid_max_ml))
        } else {
            None
        };
        if let Some((vol, max)) = blood_like {
            if max > 0.0 {
                let frac = vol / max;
                if vol <= 0.0 {
                    if !matches!(actor.status, Status::Dying | Status::Dead) {
                        let _ = actor.apply_damage(actor.hp.max(1.0));
                    }
                    death_cause
                        .entry(aid)
                        .or_insert_with(|| "Bled out: blood volume reached 0 mL.".to_string());
                } else if frac < 0.10 {
                    let _ = actor.apply_damage(BLEEDOUT_HP_PER_S * dt);
                }
            }
        }

        // Reseal helmet once back in atmosphere (breach is a vacuum hazard).
        if actor.atmosphere_sample.pressure_kpa >= VACUUM_PRESSURE_KPA {
            breached.remove(&aid);
        }

        // Resource → move-speed coupling (multiplies the m16-set base, which is
        // recomputed each tick so this never compounds).
        let mult = cf_actor::resource_speed_mult(actor);
        if mult < 1.0 {
            actor.affliction_speed_multiplier = (actor.affliction_speed_multiplier * mult)
                .clamp(crate::m16c_psych::COMBINED_MOVE_SPEED_FLOOR, 1.0);
        }
    }
}

impl M0Engine {
    /// Drive the M17 origin pass at an explicit tick (headless / replay /
    /// acceptance drivers).
    pub fn m17_drive_origin_tick(&self, tick: u64) {
        self.tick_m17_origin(Tick(tick), 0.0);
    }

    /// Emit the full per-origin shot reaction (the M17 origin reaction matrix)
    /// for a hit on `target`: always `origin.shot_force_feedback`; then either
    /// a per-origin concussion dose (organic, susceptibility-scaled + capped)
    /// or an internal-shock dose + rolled module-damage (synthetic); plus a
    /// helmet breach when a sealed-helmet head zone is penetrated. Shared by
    /// the live combat path + the acceptance injector.
    pub(crate) fn emit_m17_shot_reaction(
        &self,
        target: ActorId,
        damage: f32,
        zone: &str,
        parent_hit_event_id: &str,
        wound_event_id: &str,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        let (origin, helmet_sealed, reduced_blackout) = self
            .state
            .read()
            .ok()
            .and_then(|s| {
                let w = s.actor_state.as_ref()?;
                let a = w.world.actors.get(&target)?;
                Some((
                    a.origin(),
                    a.body_armor.helmet_seal_active() || a.body_armor.dive_suit_equipped(),
                    s.settings.reduced_g_force_blackout,
                ))
            })
            .unwrap_or((Origin::Human, false, false));
        let profile = OriginProfile::canonical(origin);
        let is_synth = profile.uses_internal_shock;

        // Roll the internal module deterministically (synthetic only).
        let rng_roll = {
            let mixed = (tick.0 ^ target.0).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            ((mixed >> 40) as f32) / ((1u64 << 24) as f32)
        };
        let circuit_id = cf_internal::route_internal_damage(
            cf_internal::InternalGraphKind::Robot,
            zone,
            damage,
            rng_roll,
        )
        .map(|d| d.target_id)
        .unwrap_or("power_core");

        let feedback_kind = match profile.feedback_kind {
            cf_actor::origin::FeedbackKind::Hybrid => "pain_jolt",
            other => other.as_str(),
        };
        let dose_delta = if profile.accumulates_g_load {
            damage * 0.6 * profile.concussion_susceptibility
        } else {
            0.0
        };
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "origin",
            "shot_force_feedback",
            json!({
                "actor_id": target.0,
                "parent_hit_event_id": parent_hit_event_id,
                "impulse_vector": [damage, 0.0],
                "impulse_magnitude": damage,
                "origin_id": origin.replay_id(),
                "chassis_layer": if is_synth { "circuit" } else { "flesh" },
                "feedback_kind": feedback_kind,
                "frame_ring": is_synth,
                "g_load_delta": if profile.accumulates_g_load { dose_delta } else { 0.0 },
                "concussion_dose_delta": dose_delta,
                "screen_kick_intensity": (damage * 0.01).min(1.0),
                "internal_shock_module_id": if is_synth { json!(circuit_id) } else { json!(null) },
                "internal_shock_damage": if is_synth { json!(damage * cf_internal::DEFAULT_INTERNAL_DAMAGE_MODIFIER) } else { json!(null) },
            }),
            Some(parent_hit_event_id.to_string()),
        );

        if profile.accumulates_g_load {
            self.emit_m17_concussion(target, damage, origin, reduced_blackout, parent_hit_event_id, wound_event_id, tick, sim_time_ms);
        } else if is_synth && cf_actor::internal_shock::impulse_arms_internal_shock(damage) {
            let prev = self
                .state
                .read()
                .ok()
                .and_then(|s| s.m17_internal_shock_dose.get(&target).copied())
                .unwrap_or(0.0);
            let new = cf_actor::internal_shock::accrue_dose(prev, damage);
            if let Ok(mut s) = self.state.write() {
                s.m17_internal_shock_dose.insert(target, new);
                if let Some(w) = s.actor_state.as_mut() {
                    if let Some(a) = w.world.actors.get_mut(&target) {
                        a.resources.internal_shock_dose = new;
                    }
                }
            }
            let is_id = self.recorder.record(
                tick,
                sim_time_ms,
                "internal_shock",
                "dose_changed",
                json!({
                    "actor_id": target.0,
                    "from_dose": prev,
                    "to_dose": new,
                    "source_event_id": parent_hit_event_id,
                }),
                Some(wound_event_id.to_string()),
            );
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "internal_shock",
                "module_damaged",
                json!({
                    "actor_id": target.0,
                    "module_id": circuit_id,
                    "damage_amount": damage * cf_internal::DEFAULT_INTERNAL_DAMAGE_MODIFIER,
                    "hit_zone": zone,
                    "source_hit_event_id": parent_hit_event_id,
                }),
                Some(is_id),
            );
        }

        if profile.oxygen_required && helmet_sealed && zone == "head" {
            if let Ok(mut s) = self.state.write() {
                s.m17_helmet_breached.insert(target);
            }
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "origin",
                "helmet_breach",
                json!({
                    "actor_id": target.0,
                    "helmet_item_id": 0,
                    "breach_pos": [0.0, 0.0],
                    "oxygen_loss_rate": cf_actor::oxygen::HELMET_BREACH_DRAIN_MULTIPLIER,
                }),
                Some(parent_hit_event_id.to_string()),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_m17_concussion(
        &self,
        target: ActorId,
        damage: f32,
        origin: Origin,
        reduced_blackout: bool,
        parent_hit_event_id: &str,
        wound_event_id: &str,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        let profile = OriginProfile::canonical(origin);
        let susc = profile.concussion_susceptibility.max(0.0);
        let prev = self
            .state
            .read()
            .ok()
            .and_then(|s| s.m9_concussion_dose.get(&target).copied())
            .unwrap_or(0.0);
        let raw_dose = (prev + damage * 0.6 * susc).clamp(0.0, 100.0);
        let cap = cf_actor::concussion::band_cap(origin, reduced_blackout);
        let raw_band = ConcussionBand::from_dose(raw_dose);
        let band = if raw_band > cap { cap } else { raw_band };
        let new_dose = if raw_band > cap { cap.dose_floor() } else { raw_dose };
        let new_band = band.as_str();
        let prev_band = self
            .state
            .read()
            .ok()
            .and_then(|s| s.m9_concussion_band.get(&target).copied())
            .unwrap_or("Clear");
        if let Ok(mut s) = self.state.write() {
            s.m9_concussion_dose.insert(target, new_dose);
            s.m9_concussion_recovery_lockout_ticks
                .insert(target, self.config.tick_rate_hz.max(1));
        }
        let dose_event_id = self.recorder.record(
            tick,
            sim_time_ms,
            "concussion",
            "dose_changed",
            json!({
                "actor_id": target.0,
                "from_dose": prev,
                "to_dose": new_dose,
                "source_event_id": parent_hit_event_id,
                "origin_id": origin.replay_id(),
            }),
            Some(wound_event_id.to_string()),
        );
        if prev_band != new_band {
            if let Ok(mut s) = self.state.write() {
                s.m9_concussion_band.insert(target, new_band);
            }
            self.recorder.record(
                tick,
                sim_time_ms,
                "concussion",
                "band_changed",
                json!({
                    "actor_id": target.0,
                    "from_band": prev_band,
                    "to_band": new_band,
                    "dose": new_dose,
                }),
                Some(dose_event_id.clone()),
            );
        }
        if matches!(band, ConcussionBand::Ko) {
            self.recorder.record(
                tick,
                sim_time_ms,
                "concussion",
                "ko_threshold_crossed",
                json!({
                    "actor_id": target.0,
                    "ko_duration_s": cf_actor::concussion::ko_duration_seconds(raw_dose),
                }),
                Some(dose_event_id),
            );
        }
    }

    /// Inject a combat-shot reaction on `target` (acceptance / scenario
    /// driver) — runs the same per-origin reaction the live combat path emits.
    pub fn m17_inject_hit(&self, target_id: u64, damage: f32, zone: &str) -> bool {
        let (tick, sim_time_ms) = {
            let state = self.state.read().expect("engine state poisoned");
            (Tick(state.clock.tick().0), state.clock.sim_time_ms())
        };
        let exists = self
            .state
            .read()
            .ok()
            .and_then(|s| s.actor_state.as_ref().map(|w| w.world.actors.contains_key(&ActorId(target_id))))
            .unwrap_or(false);
        if !exists {
            return false;
        }
        let parent = self.recorder.record(
            tick,
            sim_time_ms,
            "combat",
            "projectile_hit_mo",
            json!({ "actor_id": target_id, "damage": damage, "zone": zone, "synthetic": true }),
            None,
        );
        self.emit_m17_shot_reaction(ActorId(target_id), damage, zone, &parent, &parent, tick, sim_time_ms);
        true
    }

    /// The active AI difficulty (from settings) as the canonical TTD preset.
    fn m17_difficulty(&self) -> cf_actor::AiDifficulty {
        let id = self
            .state
            .read()
            .ok()
            .map(|s| s.settings.ai_difficulty.clone())
            .unwrap_or_else(|| "tough_crowd".to_string());
        cf_actor::AiDifficulty::from_str(&id)
    }

    /// Per-origin survival-resource snapshot for `actor_id` (cfctl
    /// `inspect.actor.resources` + acceptance assertions).
    pub fn m17_actor_resources(&self, actor_id: u64) -> serde_json::Value {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return json!(null),
        };
        let Some(actor) = state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
        else {
            return json!(null);
        };
        let origin = actor.origin();
        let profile = state.m17_origin_registry.profile(origin);
        let r = &actor.resources;
        let conc_band = ConcussionBand::from_dose(state.m9_concussion_dose.get(&ActorId(actor_id)).copied().unwrap_or(r.concussion_dose));
        json!({
            "actor_id": actor_id,
            "origin": origin.as_str(),
            "status": actor.status.as_str(),
            "hp": actor.hp,
            "blood": r.blood,
            "blood_max": profile.blood_max_ml,
            "oil": r.oil,
            "bio_fluid": r.bio_fluid,
            "power": r.power,
            "power_max": profile.power_max_kwh,
            "caloric_energy": r.caloric_energy,
            "oxygen_supply": r.oxygen_supply,
            "heat": r.heat,
            "concussion_dose": r.concussion_dose,
            "concussion_band": conc_band.as_str(),
            "internal_shock_dose": r.internal_shock_dose,
            "power_fire_locked": actor.power_fire_locked,
            "overclock_tier": actor.overclock.tier,
            "throttled": actor.overclock.throttled,
        })
    }

    /// Directly set one survival resource (acceptance seeding). `kind` is one
    /// of blood/oil/power/caloric/bio_fluid/oxygen_supply/heat/concussion/
    /// internal_shock. Returns `true` when applied.
    pub fn m17_set_resource(&self, actor_id: u64, kind: &str, value: f32) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let Some(world) = state.actor_state.as_mut() else { return false };
        let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) else { return false };
        match kind {
            "blood" => actor.resources.blood = value,
            "oil" => actor.resources.oil = value,
            "power" => actor.resources.power = value,
            "battery_charge" => actor.resources.battery_charge = value,
            "caloric" | "caloric_energy" => actor.resources.caloric_energy = value,
            "bio_fluid" => actor.resources.bio_fluid = value,
            "oxygen" | "oxygen_supply" => actor.resources.oxygen_supply = value,
            "heat" => actor.resources.heat = value,
            "concussion" | "concussion_dose" => actor.resources.concussion_dose = value,
            "internal_shock" | "internal_shock_dose" => actor.resources.internal_shock_dose = value,
            _ => return false,
        }
        true
    }

    /// Equip a sealed helmet on `actor_id` (acceptance / scenario driver for
    /// the vacuum + helmet-breach paths). Returns `true` when sealed.
    pub fn m17_equip_sealed_helmet(&self, actor_id: u64) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let Some(world) = state.actor_state.as_mut() else { return false };
        let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) else { return false };
        let _ = actor.body_armor.equip("helmet_heavy_titanium");
        actor.body_armor.helmet_seal_active()
    }

    /// Set an actor's ambient atmospheric pressure (kPa) — the vacuum signal
    /// the oxygen pass reads (acceptance / scenario driver).
    pub fn m17_set_atmosphere_pressure(&self, actor_id: u64, pressure_kpa: f32) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let Some(world) = state.actor_state.as_mut() else { return false };
        let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) else { return false };
        actor.atmosphere_sample.pressure_kpa = pressure_kpa;
        true
    }

    /// The actor's status string (e.g. `inert` for an offline robot).
    pub fn m17_actor_status(&self, actor_id: u64) -> Option<String> {
        let state = self.state.read().ok()?;
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| a.status.as_str().to_string())
    }

    /// True when `actor_id` can fire (not reserve-locked / inert / dead).
    pub fn m17_can_fire(&self, actor_id: u64) -> bool {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return false,
        };
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| !a.power_fire_locked && !matches!(a.status, Status::Inert | Status::Dead | Status::Dying))
            .unwrap_or(false)
    }

    /// Resource-derived aim-accuracy multiplier (1.0 nominal; 0.85 when an
    /// organic origin's caloric energy is low — spec scenario 7).
    pub fn m17_aim_accuracy_mult(&self, actor_id: u64) -> f32 {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return 1.0,
        };
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| {
                let p = state.m17_origin_registry.profile(a.origin());
                if p.has_caloric() && a.resources.caloric_energy < 30.0 {
                    0.85
                } else {
                    1.0
                }
            })
            .unwrap_or(1.0)
    }

    /// Effective action-speed multiplier (overclock boost / thermal throttle).
    pub fn m17_action_speed(&self, actor_id: u64) -> f32 {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return 1.0,
        };
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| overclock::effective_action_speed(&a.overclock, a.resources.heat))
            .unwrap_or(1.0)
    }

    /// Request an overclock tier (0-3) for a power-survival actor. Emits
    /// `chassis.overclock_started`. Rejected for organics. Returns `true`.
    pub fn m17_request_overclock(&self, actor_id: u64, tier: u8) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let registry_owns_origin = {
            let Some(world) = state.actor_state.as_ref() else { return false };
            let Some(actor) = world.world.actors.get(&ActorId(actor_id)) else { return false };
            actor.origin().uses_internal_shock()
        };
        if !registry_owns_origin {
            return false;
        }
        let tier = tier.min(3);
        if let Some(world) = state.actor_state.as_mut() {
            if let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) {
                actor.overclock.tier = tier;
                actor.overclock.sustained_ticks = 0;
            }
        }
        let speed = overclock::overclock_action_speed(tier);
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "chassis",
            "overclock_started",
            json!({
                "actor_id": actor_id,
                "tier": tier,
                "action_speed_factor": speed,
            }),
            None,
        );
        true
    }

    /// Trigger a helmet breach for `actor_id` (penetrating round to a sealed
    /// helmet). Emits `origin.helmet_breach`; the next oxygen tick drains 3×.
    pub fn m17_breach_helmet(&self, actor_id: u64) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let breach_pos = {
            let Some(world) = state.actor_state.as_ref() else { return false };
            let Some(actor) = world.world.actors.get(&ActorId(actor_id)) else { return false };
            // Only sealed helmets can be breached.
            if !(actor.body_armor.helmet_seal_active() || actor.body_armor.dive_suit_equipped()) {
                return false;
            }
            [actor.position.x, actor.position.y]
        };
        state.m17_helmet_breached.insert(ActorId(actor_id));
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "origin",
            "helmet_breach",
            json!({
                "actor_id": actor_id,
                "helmet_item_id": 0,
                "breach_pos": breach_pos,
                "oxygen_loss_rate": cf_actor::oxygen::HELMET_BREACH_DRAIN_MULTIPLIER,
            }),
            None,
        );
        true
    }

    /// Apply an internal-shock impact to a robot/synthetic actor (combat path +
    /// tests). Accrues dose (gated by impulse) and returns the new dose. Emits
    /// `internal_shock.dose_changed`.
    pub fn m17_apply_internal_shock(&self, actor_id: u64, damage: f32, impulse_n_s: f32) -> f32 {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let uses_is = state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| a.origin().uses_internal_shock())
            .unwrap_or(false);
        if !uses_is || !cf_actor::internal_shock::impulse_arms_internal_shock(impulse_n_s) {
            return state.m17_internal_shock_dose.get(&ActorId(actor_id)).copied().unwrap_or(0.0);
        }
        let prev = state.m17_internal_shock_dose.get(&ActorId(actor_id)).copied().unwrap_or(0.0);
        let new = cf_actor::internal_shock::accrue_dose(prev, damage);
        state.m17_internal_shock_dose.insert(ActorId(actor_id), new);
        if let Some(world) = state.actor_state.as_mut() {
            if let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) {
                actor.resources.internal_shock_dose = new;
            }
        }
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "internal_shock",
            "dose_changed",
            json!({
                "actor_id": actor_id,
                "from_dose": prev,
                "to_dose": new,
                "source_event_id": format!("internal_shock:{}", tick.0),
            }),
            None,
        );
        new
    }

    /// Revive an INERT robot (repair tool + fresh battery). Restores power +
    /// clears INERT. Emits `resource.restored`. Returns `true` when revived.
    pub fn m17_repair_revive(&self, actor_id: u64) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let max_power = state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| state.m17_origin_registry.profile(a.origin()).power_max_kwh)
            .unwrap_or(0.0);
        let Some(world) = state.actor_state.as_mut() else { return false };
        let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) else { return false };
        if !matches!(actor.status, Status::Inert) {
            return false;
        }
        actor.resources.power = max_power;
        actor.resources.battery_charge = 100.0;
        actor.power_fire_locked = false;
        actor.status = Status::Stable;
        if actor.hp < actor.hp_max * 0.5 {
            actor.hp = actor.hp_max * 0.5;
        }
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "resource",
            "restored",
            json!({
                "actor_id": actor_id,
                "kind": "power",
                "amount": max_power,
                "source": "battery_swap",
            }),
            None,
        );
        true
    }

    /// Canonical per-damage-type × per-origin × per-difficulty TTD (seconds),
    /// or `null` when the source does not apply to that origin. cfctl + tests.
    pub fn m17_damage_type_ttd(&self, origin_id: &str, damage_type: &str, difficulty: &str) -> Option<f32> {
        let origin = cf_actor::TtdOrigin::from_origin_id(origin_id);
        let diff = cf_actor::AiDifficulty::from_str(difficulty);
        let dmg = match damage_type {
            "heart_wound" => cf_actor::DamageType::HeartWound,
            "arterial_wound" => cf_actor::DamageType::ArterialWound,
            "multi_wound_torso" => cf_actor::DamageType::MultiWoundTorso,
            "concussion_ko" => cf_actor::DamageType::ConcussionKo,
            "vacuum_exposure" => cf_actor::DamageType::VacuumExposure,
            "fire_tile" => cf_actor::DamageType::FireTile,
            "reactor_cascade" => cf_actor::DamageType::ReactorCascade,
            "acid_tile" => cf_actor::DamageType::AcidTile,
            "electric_tile" => cf_actor::DamageType::ElectricTile,
            "affliction_stack_3" => cf_actor::DamageType::AfflictionStack3,
            _ => return None,
        };
        cf_actor::damage_type_ttd(dmg, origin, diff)
    }

    /// The actor's current compound TTD (floored to the difficulty's p99
    /// reaction guarantee) + the active per-source breakdown.
    pub fn m17_actor_ttd(&self, actor_id: u64) -> serde_json::Value {
        let difficulty = self.m17_difficulty();
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return json!(null),
        };
        let Some(actor) = state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
        else {
            return json!(null);
        };
        let origin = cf_actor::TtdOrigin::from_origin_id(&actor.origin_id);
        let aff = state.m16_affliction_by_actor.get(&ActorId(actor_id));
        let mut individual: Vec<f32> = Vec::new();
        let mut breakdown = serde_json::Map::new();
        if let Some(a) = aff {
            let mut push = |dmg: cf_actor::DamageType, present: bool| {
                if present {
                    if let Some(t) = cf_actor::damage_type_ttd(dmg, origin, difficulty) {
                        individual.push(t);
                        breakdown.insert(dmg.as_str().to_string(), json!(t));
                    }
                }
            };
            push(cf_actor::DamageType::ArterialWound, a.severity_of(M16AfflictionKind::Bleeding) >= 0.8);
            push(
                cf_actor::DamageType::MultiWoundTorso,
                a.severity_of(M16AfflictionKind::Bleeding) > 0.0
                    && a.severity_of(M16AfflictionKind::Bleeding) < 0.8,
            );
            push(cf_actor::DamageType::FireTile, a.severity_of(M16AfflictionKind::Burning) > 0.0);
            push(cf_actor::DamageType::ConcussionKo, a.severity_of(M16AfflictionKind::Concussed) > 0.0);
            push(cf_actor::DamageType::VacuumExposure, a.severity_of(M16AfflictionKind::VacuumExposure) > 0.0);
            push(cf_actor::DamageType::ElectricTile, a.severity_of(M16AfflictionKind::Electrified) >= 0.8);
        }
        let compound = cf_actor::compound_ttd_floored(&individual, difficulty);
        json!({
            "actor_id": actor_id,
            "origin": actor.origin().as_str(),
            "difficulty": difficulty.as_str(),
            "compound_ttd_seconds": if compound.is_finite() { json!(compound) } else { json!(null) },
            "compound_floor_seconds": difficulty.compound_floor_seconds(),
            "breakdown": breakdown,
        })
    }

    /// Per-origin death-recap line (M10 debrief). Origin-aware: humans bleed /
    /// organ-fail, robots go offline / circuit-fail. Never wrong-origin.
    pub fn m17_death_recap(&self, actor_id: u64) -> String {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return String::new(),
        };
        if let Some(cause) = state.m17_death_cause.get(&ActorId(actor_id)) {
            return cause.clone();
        }
        let origin = state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| a.origin())
            .unwrap_or(Origin::Human);
        match origin {
            Origin::Robot | Origin::Drone | Origin::Crystalline => {
                "Went offline: circuit failure / power loss.".to_string()
            }
            _ => "Bled out: organ damage + blood loss.".to_string(),
        }
    }
}

/// One resource's drain delta this tick (for the change/critical events).
struct ResourceDelta {
    kind: ResKind,
    from: f32,
    to: f32,
}

impl ResourceDelta {
    fn fraction_to(&self, profile: &OriginProfile) -> f32 {
        let max = match self.kind {
            ResKind::Blood => profile.blood_max_ml,
            ResKind::Oil => profile.oil_max_ml,
            ResKind::Power => profile.power_max_kwh,
            ResKind::Caloric => profile.caloric_max,
            ResKind::BioFluid => profile.bio_fluid_max_ml,
        };
        if max > 0.0 {
            (self.to / max).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Drain `amount` from `value` (floored at 0), recording a delta if it moved.
fn drain_resource(value: &mut f32, amount: f32, out: &mut Vec<ResourceDelta>, kind: ResKind) {
    if amount <= 0.0 {
        return;
    }
    let from = *value;
    *value = (from - amount).max(0.0);
    if (from - *value).abs() > f32::EPSILON {
        out.push(ResourceDelta { kind, from, to: *value });
    }
}
