//! M16C § Mental-health engine integration.
//!
//! [`M0Engine::tick_m16c_psych`] runs once per advanced tick (after the M16
//! affliction tick so Pain severity is settled) and drives:
//!   - the **witness-death** PTSD trigger (3 squadmate deaths within 60s in an
//!     actor's witness radius) — edge-detected centrally so it is robust to
//!     every death cause,
//!   - the per-condition **lifecycle advance** (panic attacks, stage
//!     transitions, treated/natural remission, relapse),
//!   - the **withdrawal** onset check for addicted actors,
//!   - **trait persistence** — `chronic_*` / `recovered_from_*` / `refractory_*`
//!     traits granted by the lifecycle are written onto the actor's M14I
//!     long-term `TraitSet` (read by the M41 veteran dossier).
//!
//! The trigger / treatment **control surface** (`m16c_use_combat_stim`,
//! `m16c_record_therapy_session`, `m16c_start_medication`,
//! `m16c_trigger_condition`) mirrors the `m16_*` engine API: each acquires the
//! state lock, mutates the per-actor mental-health state, and emits the matching
//! `psych.*` replay event.

use serde_json::json;

use cf_actor::ActorId;
use cf_mental_health::{
    comorbidity::apply_comorbidities, ActorMentalHealth, AddictionDevelopedEvent,
    ComorbidityDetectedEvent, ConditionKind, ConditionTriggeredEvent,
    MedicationStartedEvent, OriginId, PanicAttackEvent, PsychMedClass, PsychRelapsedEvent,
    PsychStageChangedEvent, RemissionAchievedEvent, TherapySessionEvent, TriggerReason,
    WithdrawalStartedEvent,
};
use cf_replay::Recorder;
use cf_sim_core::Tick;

use crate::engine::{EngineMutable, M0Engine};

/// Witness radius (world px) within which an actor sees a squadmate die. A
/// clustered squad (the squad-wipe scenario) is well inside this.
pub const WITNESS_RADIUS: f32 = 384.0;

fn within_witness_radius(a: [f32; 2], b: [f32; 2]) -> bool {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy <= WITNESS_RADIUS * WITNESS_RADIUS
}

// ---- event emitters (payloads mirror cf-replay/schemas/event/psych_*.json) ----

fn emit_triggered(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &ConditionTriggeredEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "condition_triggered",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "reason": ev.reason.as_str(),
            "stage": ev.stage.as_str(),
        }),
        None,
    );
}

fn emit_comorbidity(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &ComorbidityDetectedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "comorbidity_detected",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "primary": ev.primary.as_str(),
            "comorbid": ev.comorbid.as_str(),
        }),
        None,
    );
}

fn emit_stage_changed(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &PsychStageChangedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "stage_changed",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "from": ev.from.as_str(),
            "to": ev.to.as_str(),
            "trait_granted": ev.trait_granted,
        }),
        None,
    );
}

fn emit_panic(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &PanicAttackEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "panic_attack",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "freeze_seconds": ev.freeze_seconds,
            "freeze_until_tick": ev.freeze_until_tick,
        }),
        None,
    );
}

fn emit_remission(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &RemissionAchievedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "remission_achieved",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "treated": ev.treated,
            "trait_granted": ev.trait_granted,
        }),
        None,
    );
}

fn emit_relapsed(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &PsychRelapsedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "relapsed",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
        }),
        None,
    );
}

fn emit_addiction(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &AddictionDevelopedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "addiction_developed",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "dose_count": ev.dose_count,
            "trait_granted": ev.trait_granted,
        }),
        None,
    );
}

fn emit_withdrawal(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &WithdrawalStartedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "withdrawal_started",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "hours_since_dose": ev.hours_since_dose,
            "aim_wobble_multiplier": ev.aim_wobble_multiplier,
        }),
        None,
    );
}

fn emit_therapy(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &TherapySessionEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "therapy_session",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "session_index": ev.session_index,
            "sessions_required": ev.sessions_required,
            "efficacy": ev.efficacy,
        }),
        None,
    );
}

fn emit_medication(recorder: &Recorder, tick: Tick, t_ms: f64, ev: &MedicationStartedEvent) {
    let _ = recorder.record(
        tick,
        t_ms,
        "psych",
        "medication_started",
        json!({
            "actor_id": ev.actor_id,
            "tick": ev.tick,
            "condition": ev.condition.as_str(),
            "medication": ev.medication.as_str(),
        }),
        None,
    );
}

impl M0Engine {
    /// Per-tick mental-health pass. See the module docs.
    pub(crate) fn tick_m16c_psych(&self, tick: Tick, sim_time_ms: f64) {
        let seed = self.config.seed;
        let tick_rate_hz = self.config.tick_rate_hz;
        let recorder = &self.recorder;
        let mut state = self.state.write().expect("engine state poisoned for m16c psych tick");
        if state.actor_state.is_none() {
            return;
        }
        let EngineMutable {
            actor_state,
            m16c_mental_health_by_actor: mh_map,
            m16c_condition_registry: registry,
            m16c_comorbidity_matrix: comorbid,
            m16c_processed_deaths: processed_deaths,
            m16_affliction_by_actor: affliction_by_actor,
            ..
        } = &mut *state;
        let actor_world = actor_state.as_mut().expect("checked actor_state is_some");

        // Read snapshot: (id, origin, position, down). A squadmate going down
        // fatally (Dead or Dying) is the witnessed-death moment.
        let actors: Vec<(ActorId, String, [f32; 2], bool)> = actor_world
            .world
            .actors
            .iter()
            .map(|(id, a)| {
                let down = matches!(a.status, cf_actor::Status::Dead | cf_actor::Status::Dying);
                (*id, a.origin_id.clone(), [a.position.x, a.position.y], down)
            })
            .collect();

        // Lazy-init each living actor's mental-health state with its origin.
        for (id, origin, _pos, dead) in &actors {
            if *dead {
                continue;
            }
            mh_map
                .entry(*id)
                .or_insert_with(|| ActorMentalHealth::with_origin(OriginId::from_str(origin)));
        }

        let mut trait_grants: Vec<(u64, String)> = Vec::new();

        // ----- 1) Witness-death PTSD trigger (edge-detected per death) -----
        let new_deaths: Vec<(ActorId, [f32; 2])> = actors
            .iter()
            .filter(|(id, _, _, dead)| *dead && !processed_deaths.contains(id))
            .map(|(id, _, pos, _)| (*id, *pos))
            .collect();
        for (dead_id, dead_pos) in &new_deaths {
            processed_deaths.insert(*dead_id);
            let witnesses: Vec<ActorId> = actors
                .iter()
                .filter(|(wid, _, wpos, wdead)| {
                    !*wdead && *wid != *dead_id && within_witness_radius(*wpos, *dead_pos)
                })
                .map(|(wid, _, _, _)| *wid)
                .collect();
            for wid in witnesses {
                let Some(mh) = mh_map.get_mut(&wid) else {
                    continue;
                };
                if let Some(ev) = mh.record_witnessed_death(wid.0, tick.0, tick_rate_hz) {
                    emit_triggered(recorder, tick, sim_time_ms, &ev);
                    // Squad-wipe trauma banner (UX surfacing).
                    let _ = recorder.record(
                        tick,
                        sim_time_ms,
                        "ux",
                        "banner_raised",
                        json!({
                            "actor_id": wid.0,
                            "text": "TRAUMA: squad wipe witnessed",
                            "severity": "critical",
                            "source": "m16c_trauma",
                            "kind": "ptsd",
                        }),
                        None,
                    );
                    let (ct, cd) =
                        apply_comorbidities(mh, comorbid, wid.0, ev.condition, seed, tick.0);
                    for t in &ct {
                        emit_triggered(recorder, tick, sim_time_ms, t);
                    }
                    for d in &cd {
                        emit_comorbidity(recorder, tick, sim_time_ms, d);
                    }
                }
            }
        }

        // ----- 2) Per-actor withdrawal check + lifecycle advance -----
        let living_ids: Vec<ActorId> = actors
            .iter()
            .filter(|(_, _, _, dead)| !*dead)
            .map(|(id, _, _, _)| *id)
            .collect();
        for aid in &living_ids {
            // Withdrawal onset (addicted + last dose > 12h ago).
            if let Some(mh) = mh_map.get_mut(aid) {
                if let Some(wev) = mh.check_withdrawal(aid.0, tick.0, tick_rate_hz) {
                    emit_withdrawal(recorder, tick, sim_time_ms, &wev);
                }
            }
            // Lifecycle: panic / stage / remission / relapse.
            let Some(mh) = mh_map.get_mut(aid) else {
                continue;
            };
            let out = mh.tick(aid.0, registry, tick.0, tick_rate_hz, seed);
            for ev in &out.panic_attacks {
                emit_panic(recorder, tick, sim_time_ms, ev);
            }
            for ev in &out.stage_changed {
                emit_stage_changed(recorder, tick, sim_time_ms, ev);
                if let Some(tg) = &ev.trait_granted {
                    trait_grants.push((ev.actor_id, tg.clone()));
                }
            }
            for ev in &out.remissions {
                emit_remission(recorder, tick, sim_time_ms, ev);
                trait_grants.push((ev.actor_id, ev.trait_granted.clone()));
            }
            for ev in &out.relapses {
                emit_relapsed(recorder, tick, sim_time_ms, ev);
            }
        }

        // ----- 3) Trait persistence (M41 veteran dossier consumes these) -----
        for (aid, trait_id) in trait_grants {
            if let Some(actor) = actor_world.world.actors.get_mut(&ActorId(aid)) {
                actor.m14i_long_term.traits.insert(trait_id);
            }
        }

        // ----- 4) Affliction → combat/movement modifiers (M5/M16 consumer) -----
        // Push each actor's aggregate affliction aim-spread + move-speed onto
        // the actor so the next sim step consumes them (Pain wobble + slow,
        // concussed/blinded aim penalty, thirst/frozen slow, …). Reset to
        // identity for unafflicted actors so the effect never goes stale.
        let actor_ids: Vec<ActorId> = actor_world.world.actors.keys().copied().collect();
        for aid in actor_ids {
            let (speed, spread) = match affliction_by_actor.get(&aid) {
                Some(aff) => (
                    cf_affliction::affliction_move_speed_multiplier(aff),
                    cf_affliction::affliction_aim_spread_bonus_radians(aff),
                ),
                None => (1.0, 0.0),
            };
            if let Some(actor) = actor_world.world.actors.get_mut(&aid) {
                actor.affliction_speed_multiplier = speed;
                actor.affliction_aim_spread_bonus_rad = spread;
            }
        }
    }

    /// Ensure a mental-health entry exists for `actor_id`, seeded with the
    /// actor's origin. Returns the actor's origin string (for callers that
    /// need it). `None` when the actor does not exist.
    fn m16c_ensure_state(state: &mut EngineMutable, actor_id: u64) -> bool {
        let Some(actor_world) = state.actor_state.as_ref() else {
            return false;
        };
        let Some(actor) = actor_world.world.actors.get(&ActorId(actor_id)) else {
            return false;
        };
        let origin = OriginId::from_str(&actor.origin_id);
        state
            .m16c_mental_health_by_actor
            .entry(ActorId(actor_id))
            .or_insert_with(|| ActorMentalHealth::with_origin(origin));
        true
    }

    /// Drive the mental-health pass at an explicit `tick` (deterministic;
    /// drive_tick calls the internal pass with the live clock tick). Exposed so
    /// headless / replay drivers — and acceptance tests of the time-gated
    /// lifecycle (withdrawal onset, medication-onset remission) — can advance
    /// psych state to a given in-game tick without stepping the full sim.
    pub fn m16c_drive_psych_tick(&self, tick: u64) {
        self.tick_m16c_psych(Tick(tick), 0.0);
    }

    /// Administer one combat-stim dose to `actor_id`. Records the dose toward
    /// the 30-day addiction window; on the 7th dose develops an Addiction
    /// (emits `psych.addiction_developed`, grants `chronic_addiction`, rolls
    /// comorbidities). Returns `true` when addiction developed on this dose.
    pub fn m16c_use_combat_stim(&self, actor_id: u64) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let tick_rate_hz = self.config.tick_rate_hz;
        let seed = self.config.seed;
        if !Self::m16c_ensure_state(&mut state, actor_id) {
            return false;
        }
        let EngineMutable {
            m16c_mental_health_by_actor: mh_map,
            m16c_comorbidity_matrix: comorbid,
            actor_state,
            ..
        } = &mut *state;
        let Some(mh) = mh_map.get_mut(&ActorId(actor_id)) else {
            return false;
        };
        let Some(ev) = mh.record_stim_dose(actor_id, tick.0, tick_rate_hz) else {
            return false;
        };
        emit_addiction(&self.recorder, tick, sim_time_ms, &ev);
        if let Some(world) = actor_state.as_mut() {
            if let Some(actor) = world.world.actors.get_mut(&ActorId(actor_id)) {
                actor.m14i_long_term.traits.insert(ev.trait_granted.clone());
            }
        }
        let (ct, cd) = apply_comorbidities(mh, comorbid, actor_id, ConditionKind::Addiction, seed, tick.0);
        for t in &ct {
            emit_triggered(&self.recorder, tick, sim_time_ms, t);
        }
        for d in &cd {
            emit_comorbidity(&self.recorder, tick, sim_time_ms, d);
        }
        true
    }

    /// Directly trigger a mental-health condition for `actor_id` (the
    /// survive-critical-wound / concussion-at-KO PTSD paths + test seeding).
    /// Emits `psych.condition_triggered` (+ comorbidities). Returns `true`
    /// when a new condition was triggered.
    pub fn m16c_trigger_condition(&self, actor_id: u64, condition: &str, reason: &str) -> bool {
        let Some(kind) = ConditionKind::from_str(condition) else {
            return false;
        };
        let reason = parse_reason(reason);
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let seed = self.config.seed;
        if !Self::m16c_ensure_state(&mut state, actor_id) {
            return false;
        }
        let EngineMutable {
            m16c_mental_health_by_actor: mh_map,
            m16c_comorbidity_matrix: comorbid,
            ..
        } = &mut *state;
        let Some(mh) = mh_map.get_mut(&ActorId(actor_id)) else {
            return false;
        };
        let Some(ev) = mh.trigger(actor_id, kind, reason, tick.0) else {
            return false;
        };
        emit_triggered(&self.recorder, tick, sim_time_ms, &ev);
        let (ct, cd) = apply_comorbidities(mh, comorbid, actor_id, kind, seed, tick.0);
        for t in &ct {
            emit_triggered(&self.recorder, tick, sim_time_ms, t);
        }
        for d in &cd {
            emit_comorbidity(&self.recorder, tick, sim_time_ms, d);
        }
        true
    }

    /// Record one completed therapy session for `actor_id`'s `condition`.
    /// Emits `psych.therapy_session`. Returns `true` when recorded.
    pub fn m16c_record_therapy_session(&self, actor_id: u64, condition: &str) -> bool {
        let Some(kind) = ConditionKind::from_str(condition) else {
            return false;
        };
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let seed = self.config.seed;
        let EngineMutable {
            m16c_mental_health_by_actor: mh_map,
            m16c_condition_registry: registry,
            ..
        } = &mut *state;
        let Some(mh) = mh_map.get_mut(&ActorId(actor_id)) else {
            return false;
        };
        match mh.record_therapy_session(actor_id, kind, registry, seed, tick.0) {
            Some(ev) => {
                emit_therapy(&self.recorder, tick, sim_time_ms, &ev);
                true
            }
            None => false,
        }
    }

    /// Begin a medication course for `actor_id`'s `condition`. Emits
    /// `psych.medication_started`. Returns `true` when started.
    pub fn m16c_start_medication(&self, actor_id: u64, condition: &str, medication: &str) -> bool {
        let Some(kind) = ConditionKind::from_str(condition) else {
            return false;
        };
        let Some(med) = PsychMedClass::from_str(medication) else {
            return false;
        };
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let Some(mh) = state.m16c_mental_health_by_actor.get_mut(&ActorId(actor_id)) else {
            return false;
        };
        match mh.start_medication(actor_id, kind, med, tick.0) {
            Some(ev) => {
                emit_medication(&self.recorder, tick, sim_time_ms, &ev);
                true
            }
            None => false,
        }
    }

    /// Apply `amount` combat damage to `actor_id` (the same `apply_damage`
    /// primitive the live combat path uses). Lethal damage drives the actor
    /// to Dying, which the witness-death pass counts on the next tick. Returns
    /// the actor's new status string when it changed.
    pub fn m16c_apply_combat_damage(&self, actor_id: u64, amount: f32) -> Option<String> {
        let mut state = self.state.write().expect("engine state poisoned");
        let world = state.actor_state.as_mut()?;
        let actor = world.world.actors.get_mut(&ActorId(actor_id))?;
        actor.apply_damage(amount).map(|s| s.as_str().to_string())
    }

    /// `Some(reason)` when sleep is rejected for `actor_id` (active insomnia).
    /// The bed/sleep action consumes this as the `act.player.sleep` rejection.
    pub fn m16c_can_sleep(&self, actor_id: u64) -> Option<String> {
        let state = self.state.read().ok()?;
        state
            .m16c_mental_health_by_actor
            .get(&ActorId(actor_id))
            .and_then(|mh| mh.can_sleep())
            .map(|s| s.to_string())
    }

    /// The active (condition, stage) pairs on `actor_id` — for the psych
    /// dashboard + acceptance assertions.
    pub fn m16c_mental_health_summary(&self, actor_id: u64) -> Vec<(String, String)> {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        state
            .m16c_mental_health_by_actor
            .get(&ActorId(actor_id))
            .map(|mh| {
                mh.active
                    .iter()
                    .map(|c| (c.kind.as_str().to_string(), c.stage.as_str().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The actor's current affliction-derived walk-speed multiplier (≤1.0;
    /// 1.0 = unafflicted) and aim-spread bonus (radians; 0.0 = unafflicted) —
    /// the M5/M16 affliction→combat consumer, set each tick from the M16
    /// affliction state. Returns `(1.0, 0.0)` when the actor is unknown.
    pub fn m16c_actor_combat_modifiers(&self, actor_id: u64) -> (f32, f32) {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return (1.0, 0.0),
        };
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| (a.affliction_speed_multiplier, a.affliction_aim_spread_bonus_rad))
            .unwrap_or((1.0, 0.0))
    }

    /// True when `actor_id` carries `trait_id` in its M14I long-term TraitSet
    /// (e.g. `recovered_from_ptsd`, `chronic_addiction`). Scenario 7.
    pub fn m16c_has_trait(&self, actor_id: u64, trait_id: &str) -> bool {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return false,
        };
        state
            .actor_state
            .as_ref()
            .and_then(|w| w.world.actors.get(&ActorId(actor_id)))
            .map(|a| a.m14i_long_term.traits.has(trait_id))
            .unwrap_or(false)
    }
}

/// Tolerant trigger-reason parse (defaults to sustained_stress).
fn parse_reason(s: &str) -> TriggerReason {
    match s {
        "witness_deaths" => TriggerReason::WitnessDeaths,
        "survive_critical_wound" => TriggerReason::SurviveCriticalWound,
        "concussion_at_ko" => TriggerReason::ConcussionAtKo,
        "sustained_stress" => TriggerReason::SustainedStress,
        "loss_of_squadmate" => TriggerReason::LossOfSquadmate,
        "drug_doses" => TriggerReason::DrugDoses,
        "drug_absence" => TriggerReason::DrugAbsence,
        "panic_threshold" => TriggerReason::PanicThreshold,
        "anxiety_chronic" => TriggerReason::AnxietyChronic,
        "imminent_trauma" => TriggerReason::ImminentTrauma,
        "sleep_deprivation" => TriggerReason::SleepDeprivation,
        "comorbidity" => TriggerReason::Comorbidity,
        _ => TriggerReason::SustainedStress,
    }
}
