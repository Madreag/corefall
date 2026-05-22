//! **M14H** § Treatment workflow dispatch handlers.
//!
//! Owns the engine-side mutation + event-emit logic for the six new
//! cfctl methods:
//!
//! - `act.player.treat { kind, target_actor_id }`
//! - `act.player.scan { target_actor_id }`
//! - `act.player.cpr_round { target_actor_id }`
//! - `act.player.defib { target_actor_id }`
//! - `act.player.surgery_start { target_actor_id, wounds_to_treat, surgeon_t1, seed? }`
//! - `act.player.triage_select { target_actor_id? }`
//!
//! Event surface (18 spec-locked entries):
//! `treatment.applied/completed/failed/cancelled`,
//! `cardiac.arrested/cpr_round/defib_attempted/restored/expired`,
//! `surgery.phase_started/phase_completed/skill_check/completed/failed`,
//! `scan.started/completed`,
//! `triage.queue_changed`,
//! `patient.assessed`.
//!
//! In-engine effect resolution: treatments mutate the patient's
//! `ActorState` per [`cf_treatment::TreatmentEffect`] (bandage flag flips,
//! wound emissions for cauterize/defib/CPR, tourniquet timer, buff
//! application, cardiac restoration).
//!
//! Determinism: per the M14H Gherkin scenario "Determinism — same seed
//! reproduces surgery outcome", the surgery skill-check + defib success
//! roll flow through `cf_treatment::SurgerySession` + the actor's
//! persistent `m14h_cardiac` component (read at defib time for the
//! +10%-per-CPR-round boost).

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde_json::json;

use cf_actor::cardiac::ActorCardiacComponent;
use cf_actor::m14h_state::{ActiveBuff, AntibioticCourseState, BuffKind};
use cf_actor::{ActorId, IntentSource};
use cf_sim_core::Tick;
use cf_treatment::{
    effect_for, CardiacEvent, SurgeryEvent, SurgeryPhase, SurgerySession, TreatmentApply,
    TreatmentApplyError, TreatmentContext, TreatmentEffect, TreatmentEvent, TreatmentKind,
    DEFIB_BASE_SUCCESS, DEFIB_CPR_BOOST_PER_ROUND, DEFIB_RECHARGE_SECONDS, CPR_BRUISE_THRESHOLD_ROUNDS,
    SCAN_DURATION_SECONDS_DEFAULT,
};
use cf_wound::{registry::ZoneId, Wound, WoundId, WoundKind, WoundVisibleState};

use crate::engine::M0Engine;
use crate::server::CommandResult;

fn source_label(source: IntentSource) -> &'static str {
    match source {
        IntentSource::Human => "human",
        IntentSource::Cfctl => "cfctl",
        IntentSource::Ai => "ai",
        IntentSource::Replay => "replay",
    }
}

const CHEST_ZONE: &str = "torso_front";

impl M0Engine {
    pub(crate) fn dispatch_m14h_treat(
        &self,
        kind_str: String,
        target_actor_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let Some(kind) = TreatmentKind::from_str(&kind_str) else {
            self.record_command_rejected(
                tick,
                sim_time_ms,
                "act.player.treat",
                "unknown_treatment_kind",
            );
            return CommandResult::rejected("unknown_treatment_kind", tick.0);
        };
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.treat",
                "kind": kind.as_str(),
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let ctx = self.m14h_context_for(target_actor_id);
        let seed = self.config.seed.wrapping_add(tick.0.wrapping_mul(31)) ^ target_actor_id;
        let mut apply = match TreatmentApply::start(kind, ctx, seed) {
            Ok(a) => a,
            Err(err) => {
                let reason = match err {
                    TreatmentApplyError::WrongOrigin => "wrong_origin",
                    TreatmentApplyError::MissingTool(_) => "missing_tool",
                    TreatmentApplyError::MissingSkill(_) => "missing_skill",
                    TreatmentApplyError::OutOfCharges => "out_of_charges",
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "treatment",
                    "failed",
                    json!({
                        "actor_id": target_actor_id,
                        "tick": tick.0,
                        "kind": kind.as_str(),
                        "reason": reason,
                    }),
                    Some(action_id),
                );
                return CommandResult::rejected(reason, tick.0);
            }
        };
        // Drive the apply state machine to completion in-place (the engine
        // bridge fast-paths the apply window per the M14H Gherkin "5s
        // elapse" expectation; persistent multi-tick state lives on the
        // actor).
        let events = apply.tick(0.0, tick.0, &ctx);
        let mut completed = false;
        for ev in events {
            if matches!(ev, TreatmentEvent::Completed { .. }) {
                completed = true;
            }
            self.m14h_record_treatment_event(action_id.clone(), &ev);
        }
        if !apply.is_terminal() {
            let events = apply.tick(apply.apply_seconds_total, tick.0, &ctx);
            for ev in events {
                if matches!(ev, TreatmentEvent::Completed { .. }) {
                    completed = true;
                }
                self.m14h_record_treatment_event(action_id.clone(), &ev);
            }
        }
        if completed {
            let effect = effect_for(kind, None);
            self.m14h_apply_effect(
                target_actor_id,
                &effect,
                kind,
                action_id.clone(),
                tick,
                sim_time_ms,
            );
            // ScarRecord entries on the actor's m14i_long_term timeline.
            if matches!(
                kind,
                TreatmentKind::SuturesV1
                    | TreatmentKind::CauterizeV1
                    | TreatmentKind::SurgeryKitV1
            ) {
                let closure_method = match kind {
                    TreatmentKind::SuturesV1 => cf_wound::registry::TreatmentKind::SutureKit,
                    TreatmentKind::CauterizeV1 => cf_wound::registry::TreatmentKind::BurnGel,
                    TreatmentKind::SurgeryKitV1 => cf_wound::registry::TreatmentKind::SurgeryKit,
                    _ => cf_wound::registry::TreatmentKind::SutureKit,
                };
                let _ = self.m14i_record_scars_for_closure(
                    target_actor_id,
                    closure_method,
                    None,
                    tick,
                    sim_time_ms,
                    Some(action_id.clone()),
                );
            }
        }
        CommandResult::accepted(tick.0)
    }

    /// read against `target_actor_id`. Emits `scan.started` + `scan.completed`
    /// with a real wound + affliction + buff snapshot.
    pub(crate) fn dispatch_m14h_scan(
        &self,
        target_actor_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let scanner_actor_id = {
            let state = self.state.read().ok();
            state
                .and_then(|s| s.player_actor.map(|a| a.0))
                .unwrap_or(0)
        };
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.scan",
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let started_id = self.recorder.record(
            tick,
            sim_time_ms,
            "scan",
            "started",
            json!({
                "scanner_actor_id": scanner_actor_id,
                "target_actor_id": target_actor_id,
                "tick": tick.0,
                "duration_seconds": SCAN_DURATION_SECONDS_DEFAULT,
            }),
            Some(action_id),
        );
        let (wound_count, pain_total, buff_count) = self.m14h_scan_snapshot_for(target_actor_id);
        self.recorder.record(
            tick,
            sim_time_ms,
            "scan",
            "completed",
            json!({
                "scanner_actor_id": scanner_actor_id,
                "target_actor_id": target_actor_id,
                "tick": tick.0,
                "wound_count": wound_count,
                "disease_count": 0,
                "psych_count": buff_count,
                "pain_total": pain_total,
            }),
            Some(started_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// actor's persistent [`ActorCardiacComponent`] so consecutive_cpr_rounds
    /// is honored across multiple invocations (Gherkin scenario 2 "2
    /// cardiac.cpr_round events fire (40s elapsed) + 50% + 20% (2 CPR
    /// rounds) = 70%"). After 3+ CPR rounds emits a BruiseLight wound on
    /// torso_front per spec § "Bruise wound on chest after 3+ rounds".
    pub(crate) fn dispatch_m14h_cpr_round(
        &self,
        target_actor_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.cpr_round",
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let (round_index, consecutive, just_bruised) = {
            let mut s = match self.state.write() {
                Ok(s) => s,
                Err(_) => return CommandResult::rejected("engine_state_poisoned", tick.0),
            };
            let Some(sim) = s.actor_state.as_mut() else {
                return CommandResult::rejected("no_actor_world", tick.0);
            };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(target_actor_id)) else {
                return CommandResult::rejected("unknown_target_actor", tick.0);
            };
            let was_bruised_before = actor.m14h_cardiac.chest_bruised;
            actor.m14h_cardiac.apply_cpr_round();
            let now_bruised = actor.m14h_cardiac.chest_bruised && !was_bruised_before;
            (
                actor.m14h_cardiac.cpr_rounds_total,
                actor.m14h_cardiac.consecutive_cpr_rounds,
                now_bruised,
            )
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "cardiac",
            "cpr_round",
            json!({
                "actor_id": target_actor_id,
                "tick": tick.0,
                "round_index": round_index,
                "consecutive_cpr_rounds": consecutive,
            }),
            Some(action_id.clone()),
        );
        if just_bruised {
            self.m14h_emit_wound(
                target_actor_id,
                WoundKind::BruiseLight,
                CHEST_ZONE,
                0.25,
                action_id,
                tick,
                sim_time_ms,
            );
        }
        CommandResult::accepted(tick.0)
    }

    /// [`ActorCardiacComponent`] for the consecutive_cpr_rounds boost,
    /// honors the 8s recharge interval, consumes one charge, and emits a
    /// Burn1st wound at the chest zone per shock.
    pub(crate) fn dispatch_m14h_defib(
        &self,
        target_actor_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.defib",
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let recharge_ticks =
            (DEFIB_RECHARGE_SECONDS * self.config.tick_rate_hz.max(1) as f32) as u64;
        let seed = self.config.seed.wrapping_add(tick.0) ^ target_actor_id;
        let mut rng = Xoshiro256StarStar::seed_from_u64(seed);
        let (success_prob_x1000, roll_x1000, passed, charges_remaining, recharge_blocked) = {
            let mut s = match self.state.write() {
                Ok(s) => s,
                Err(_) => return CommandResult::rejected("engine_state_poisoned", tick.0),
            };
            let Some(sim) = s.actor_state.as_mut() else {
                return CommandResult::rejected("no_actor_world", tick.0);
            };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(target_actor_id)) else {
                return CommandResult::rejected("unknown_target_actor", tick.0);
            };
            // 8s recharge gate.
            if let Some(last) = actor.m14h_last_defib_tick {
                if tick.0.saturating_sub(last) < recharge_ticks {
                    let charges = actor.m14h_cardiac.charges_remaining;
                    (0u32, 0u32, false, charges, true)
                } else {
                    Self::do_defib_attempt(actor, &mut rng, tick.0)
                }
            } else {
                Self::do_defib_attempt(actor, &mut rng, tick.0)
            }
        };
        if recharge_blocked {
            self.recorder.record(
                tick,
                sim_time_ms,
                "treatment",
                "failed",
                json!({
                    "actor_id": target_actor_id,
                    "tick": tick.0,
                    "kind": TreatmentKind::DefibrillatorV1.as_str(),
                    "reason": "out_of_charges",
                }),
                Some(action_id.clone()),
            );
            return CommandResult::rejected("recharge_in_progress", tick.0);
        }
        self.recorder.record(
            tick,
            sim_time_ms,
            "cardiac",
            "defib_attempted",
            json!({
                "actor_id": target_actor_id,
                "tick": tick.0,
                "success_probability_x1000": success_prob_x1000,
                "roll_x1000": roll_x1000,
                "passed": passed,
                "charges_remaining": charges_remaining,
            }),
            Some(action_id.clone()),
        );
        if passed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "cardiac",
                "restored",
                json!({
                    "actor_id": target_actor_id,
                    "tick": tick.0,
                }),
                Some(action_id.clone()),
            );
        }
        // Always emit a Burn1st on chest per shock (spec § "Burn1st at
        // chest from each shock") — regardless of success/failure, as long
        // as a charge was consumed.
        if charges_remaining < cf_treatment::DEFIB_CHARGES_DEFAULT
            || (charges_remaining == 0 && success_prob_x1000 > 0)
        {
            self.m14h_emit_wound(
                target_actor_id,
                WoundKind::Burn1st,
                CHEST_ZONE,
                0.15,
                action_id,
                tick,
                sim_time_ms,
            );
        }
        CommandResult::accepted(tick.0)
    }

    /// Helper: perform a defib attempt on a mutable actor, mutating
    /// charges + cardiac state. Returns
    /// `(success_prob_x1000, roll_x1000, passed, charges_remaining, false)`.
    fn do_defib_attempt(
        actor: &mut cf_actor::ActorState,
        rng: &mut Xoshiro256StarStar,
        sim_tick: u64,
    ) -> (u32, u32, bool, u32, bool) {
        if actor.m14h_cardiac.charges_remaining == 0 {
            return (0, 0, false, 0, false);
        }
        let consecutive = actor.m14h_cardiac.consecutive_cpr_rounds;
        let p = (DEFIB_BASE_SUCCESS + consecutive as f32 * DEFIB_CPR_BOOST_PER_ROUND)
            .clamp(0.0, 1.0);
        let p_x1000 = (p * 1000.0).round() as u32;
        let roll = (rng.next_u64() % 1000) as u32;
        let passed = roll < p_x1000;
        actor.m14h_cardiac.consume_defib_charge();
        actor.m14h_last_defib_tick = Some(sim_tick);
        if passed {
            actor.m14h_cardiac.clear();
        }
        (
            p_x1000,
            roll,
            passed,
            actor.m14h_cardiac.charges_remaining,
            false,
        )
    }

    ///
    /// Per Gherkin scenario 3: "the full 5-phase sequence completes, then
    /// 3× treatment.applied fires (one per shrapnel removed)". This
    /// dispatcher drives the [`SurgerySession`] state machine inline and
    /// emits one `treatment.applied { kind: SurgeryKitV1 }` event per
    /// Operate-phase pass — in addition to the `surgery.phase_started/
    /// phase_completed/skill_check/completed/failed` event surface.
    /// The patient's ShrapnelEmbedded wounds are also removed from the
    /// `m14g_wound_list` per shrapnel successfully removed.
    pub(crate) fn dispatch_m14h_surgery_start(
        &self,
        target_actor_id: u64,
        wounds_to_treat: u32,
        surgeon_t1: bool,
        seed_override: Option<u64>,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let seed = seed_override.unwrap_or(
            self.config
                .seed
                .wrapping_add(tick.0.wrapping_mul(17))
                .wrapping_add(target_actor_id),
        );
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.surgery_start",
                "target_actor_id": target_actor_id,
                "wounds_to_treat": wounds_to_treat,
                "surgeon_t1": surgeon_t1,
                "seed": seed,
                "source": source_label_str,
            }),
            None,
        );
        let mut s = SurgerySession::new(target_actor_id, wounds_to_treat, surgeon_t1, seed);
        let max_steps = (cf_treatment::SURGERY_PHASE_OPEN_SECONDS
            + cf_treatment::SURGERY_PHASE_DIAGNOSE_SECONDS
            + cf_treatment::SURGERY_PHASE_OPERATE_SECONDS_PER_STEP * wounds_to_treat.max(1) as f32
            + cf_treatment::SURGERY_PHASE_CLOSE_SECONDS
            + cf_treatment::SURGERY_PHASE_RECOVER_SECONDS) as u64
            + 5;
        for offset in 0..=max_steps {
            let sim_tick = tick.0.saturating_add(offset);
            for ev in s.tick(1.0, sim_tick) {
                if let SurgeryEvent::SkillCheck { passed, .. } = &ev {
                    // Per Gherkin S3: emit treatment.applied per shrapnel
                    // removed (one per successful Operate-phase pass).
                    if *passed {
                        self.recorder.record(
                            Tick(sim_tick),
                            0.0,
                            "treatment",
                            "applied",
                            json!({
                                "actor_id": target_actor_id,
                                "tick": sim_tick,
                                "kind": TreatmentKind::SurgeryKitV1.as_str(),
                                "apply_seconds": cf_treatment::SURGERY_PHASE_OPERATE_SECONDS_PER_STEP,
                            }),
                            Some(action_id.clone()),
                        );
                        self.m14h_remove_one_shrapnel(target_actor_id);
                    }
                }
                self.m14h_record_surgery_event(action_id.clone(), &ev);
            }
            if s.is_terminal() {
                break;
            }
        }
        // on the patient as a ScarRecord (surgery is the heaviest closure
        // method + carries the SurgeryKit FunctionalDebuff matrix).
        let _ = self.m14i_record_scars_for_closure(
            target_actor_id,
            cf_wound::registry::TreatmentKind::SurgeryKit,
            None,
            tick,
            sim_time_ms,
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `triage.queue_changed` event reflecting the new selected actor.
    /// Pulls per-actor wound state to populate `actor_ids_sorted` with the
    /// current squad's compound-TTD-sorted row list.
    pub(crate) fn dispatch_m14h_triage_select(
        &self,
        target_actor_id: Option<u64>,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.triage_select",
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let actor_ids_sorted = self.m14h_triage_queue_ordering();
        let row_count = actor_ids_sorted.len();
        self.recorder.record(
            tick,
            sim_time_ms,
            "triage",
            "queue_changed",
            json!({
                "tick": tick.0,
                "row_count": row_count,
                "selected_actor_id": target_actor_id,
                "actor_ids_sorted": actor_ids_sorted,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    // -----------------------------------------------------------------
    // Effect application
    // -----------------------------------------------------------------

    fn m14h_apply_effect(
        &self,
        actor_id: u64,
        effect: &TreatmentEffect,
        kind: TreatmentKind,
        parent: String,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        match effect {
            TreatmentEffect::BandageBleed { zone } | TreatmentEffect::TraumaPack { zone } => {
                self.m14h_bandage_zone(actor_id, zone.as_deref());
            }
            TreatmentEffect::Tourniquet { zone } => {
                self.m14h_apply_tourniquet(actor_id, zone, tick.0);
            }
            TreatmentEffect::Suture { zone } => {
                self.m14h_suture_zone(actor_id, zone.as_deref());
            }
            TreatmentEffect::Cauterize { zone } => {
                self.m14h_bandage_zone(actor_id, Some(zone));
                self.m14h_emit_wound(
                    actor_id,
                    WoundKind::Burn1st,
                    zone,
                    0.20,
                    parent.clone(),
                    tick,
                    sim_time_ms,
                );
            }
            TreatmentEffect::Splint { zone } => {
                self.m14h_splint_zone(actor_id, zone);
            }
            TreatmentEffect::PainkillerOpioidT1 => {
                self.m14h_apply_buff(actor_id, BuffKind::PainkillerOpioidT1, tick.0);
            }
            TreatmentEffect::AntiAnxietyBenzoT1 => {
                self.m14h_apply_buff(actor_id, BuffKind::AntiAnxietyBenzoT1, tick.0);
            }
            TreatmentEffect::CombatStimT1 => {
                self.m14h_apply_buff(actor_id, BuffKind::CombatStimT1, tick.0);
            }
            TreatmentEffect::IvFluids => {
                self.m14h_apply_buff(actor_id, BuffKind::IvFluidsV1, tick.0);
            }
            TreatmentEffect::OxygenTherapy => {
                self.m14h_apply_buff(actor_id, BuffKind::OxygenTherapyV1, tick.0);
            }
            TreatmentEffect::AntiRadiationChelation => {
                self.m14h_apply_buff(actor_id, BuffKind::AntiRadiationChelation, tick.0);
            }
            TreatmentEffect::HospitalBedV1 => {
                self.m14h_apply_buff(actor_id, BuffKind::HospitalBedV1, tick.0);
            }
            TreatmentEffect::AntibioticCourseT1 => {
                self.m14h_start_antibiotic_course(actor_id, 1, tick.0);
            }
            TreatmentEffect::AntibioticCourseT2 => {
                self.m14h_start_antibiotic_course(actor_id, 2, tick.0);
            }
            TreatmentEffect::TransfusionBag
            | TreatmentEffect::AntidoteUniversal
            | TreatmentEffect::AntidoteOrganophosphate
            | TreatmentEffect::MedicalScannerT1
            | TreatmentEffect::SurgeryRemoveShrapnel
            | TreatmentEffect::DefibShock
            | TreatmentEffect::CprRound => {
                // These either have no persistent state change OR are
                // handled by the surgery / cpr / defib dispatchers.
            }
        }
        let _ = kind;
    }

    fn m14h_bandage_zone(&self, actor_id: u64, zone_hint: Option<&str>) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        Self::bandage_actor_zone(actor, zone_hint);
    }

    fn bandage_actor_zone(actor: &mut cf_actor::ActorState, zone_hint: Option<&str>) {
        // Strategy: if a zone hint is provided AND there is a bleeding
        // wound in that zone, bandage every wound in that zone. Otherwise
        // bandage every wound across the actor (matches the implicit
        // "bandage the most-bleeding wound" semantics for the simplified
        // cfctl one-shot dispatch).
        let zones: Vec<ZoneId> = if let Some(z) = zone_hint {
            vec![ZoneId::from(z)]
        } else {
            actor.m14g_wound_list.wounds_by_zone.keys().cloned().collect()
        };
        for zone in &zones {
            if let Some(wounds) = actor.m14g_wound_list.wounds_by_zone.get_mut(zone) {
                for w in wounds.iter_mut() {
                    w.bandaged = true;
                    if matches!(
                        w.visible_state,
                        WoundVisibleState::Fresh | WoundVisibleState::BandageSoaked
                    ) {
                        w.visible_state = WoundVisibleState::CleanBandage;
                    }
                    // Reset age so the M14G aging pass measures soak-through
                    // from the moment of bandage application.
                    w.age_ticks = 0;
                }
            }
        }
    }

    fn m14h_suture_zone(&self, actor_id: u64, zone_hint: Option<&str>) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        let zones: Vec<ZoneId> = if let Some(z) = zone_hint {
            vec![ZoneId::from(z)]
        } else {
            actor.m14g_wound_list.wounds_by_zone.keys().cloned().collect()
        };
        for zone in &zones {
            if let Some(wounds) = actor.m14g_wound_list.wounds_by_zone.get_mut(zone) {
                for w in wounds.iter_mut() {
                    w.sutured = true;
                    w.visible_state = WoundVisibleState::SutureLine;
                }
            }
        }
    }

    fn m14h_apply_tourniquet(&self, actor_id: u64, zone: &str, sim_tick: u64) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        let zone_id = ZoneId::from(zone);
        // Bandage the zone first.
        Self::bandage_actor_zone(actor, Some(zone));
        // Track tourniquet apply tick for the necrosis pass.
        actor.m14h_tourniquets.insert(zone_id, sim_tick);
    }

    fn m14h_splint_zone(&self, actor_id: u64, zone: &str) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        let zone_id = ZoneId::from(zone);
        if let Some(wounds) = actor.m14g_wound_list.wounds_by_zone.get_mut(&zone_id) {
            for w in wounds.iter_mut() {
                if matches!(
                    w.kind,
                    WoundKind::FractureSimple
                        | WoundKind::FractureCompound
                        | WoundKind::FractureComminuted
                        | WoundKind::Dislocation
                        | WoundKind::SprainStrain
                ) {
                    // Halve the wound's residual age — splint cuts heal
                    // time roughly in half per spec table.
                    w.age_ticks = w.age_ticks.saturating_add(w.age_ticks);
                    w.bandaged = true;
                    if matches!(w.visible_state, WoundVisibleState::Fresh) {
                        w.visible_state = WoundVisibleState::CleanBandage;
                    }
                }
            }
        }
    }

    fn m14h_apply_buff(&self, actor_id: u64, kind: BuffKind, sim_tick: u64) {
        let tick_rate = self.config.tick_rate_hz;
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        actor
            .m14h_buffs
            .retain(|b| b.kind != kind || !b.is_expired(sim_tick));
        if !actor.m14h_buffs.iter().any(|b| b.kind == kind) {
            actor
                .m14h_buffs
                .push(ActiveBuff::new(kind, sim_tick, tick_rate));
        }
    }

    fn m14h_start_antibiotic_course(&self, actor_id: u64, tier: u8, sim_tick: u64) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        let state = if tier == 2 {
            AntibioticCourseState::t2(sim_tick)
        } else {
            AntibioticCourseState::t1(sim_tick)
        };
        actor.m14h_antibiotic_course = Some(state);
    }

    fn m14h_remove_one_shrapnel(&self, actor_id: u64) {
        let Ok(mut s) = self.state.write() else { return };
        let Some(sim) = s.actor_state.as_mut() else { return };
        let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
        let mut removed = false;
        // First pass: try to remove an entire ShrapnelEmbedded wound.
        let zone_keys: Vec<ZoneId> = actor.m14g_wound_list.wounds_by_zone.keys().cloned().collect();
        for zone in zone_keys {
            if removed { break; }
            if let Some(wounds) = actor.m14g_wound_list.wounds_by_zone.get_mut(&zone) {
                if let Some(idx) = wounds
                    .iter()
                    .position(|w| w.kind == WoundKind::ShrapnelEmbedded)
                {
                    let removed_wound = wounds.remove(idx);
                    // If the zone had multiple shrapnel counts, only the
                    // wound's shrapnel_count gets decremented per pass.
                    if removed_wound.shrapnel_count > 1 {
                        let mut rest = removed_wound.clone();
                        rest.shrapnel_count -= 1;
                        wounds.push(rest);
                    }
                    removed = true;
                }
            }
        }
    }

    fn m14h_emit_wound(
        &self,
        actor_id: u64,
        kind: WoundKind,
        zone: &str,
        severity: f32,
        parent: String,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        // Push the wound on the actor + emit `wound.created`.
        let (wound_id, dirt_pct) = {
            let Ok(mut s) = self.state.write() else {
                return;
            };
            let Some(sim) = s.actor_state.as_mut() else { return };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else { return };
            let zone_id = ZoneId::from(zone);
            let id = actor.m14g_wound_list.push(
                zone_id.clone(),
                Wound::new(WoundId(0), kind, severity, zone_id),
            );
            (id.raw(), 0.0_f32)
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "wound",
            "created",
            json!({
                "actor_id": actor_id,
                "tick": tick.0,
                "wound_id": wound_id,
                "kind": kind.as_str(),
                "zone": zone,
                "severity": severity,
                "dirt_pct": dirt_pct,
            }),
            Some(parent),
        );
    }

    // -----------------------------------------------------------------
    // Context resolution + scan snapshot + triage ordering
    // -----------------------------------------------------------------

    /// Builds a [`TreatmentContext`] from the patient's actor state.
    ///
    /// For cfctl test convenience the default context populates all
    /// skills (medic_t1, medic_t2, surgeon_t1). Real scenarios override
    /// via actor metadata (TODO: M14H+1 hook to actor inventory + role).
    fn m14h_context_for(&self, actor_id: u64) -> TreatmentContext {
        let mut ctx = TreatmentContext::for_clean_human(actor_id);
        ctx.has_medic_t2 = true;
        ctx.has_surgeon_t1 = true;
        // Pull dirt_pct from the most-dirty wound across the actor's
        // m14g_wound_list — drives the SuturesV1 risk roll path.
        if let Ok(state) = self.state.read() {
            if let Some(sim) = state.actor_state.as_ref() {
                if let Some(actor) = sim.world.actors.get(&ActorId(actor_id)) {
                    let mut max_dirt = 0.0_f32;
                    for (_, wounds) in actor.m14g_wound_list.iter() {
                        for w in wounds {
                            if w.dirt_pct > max_dirt {
                                max_dirt = w.dirt_pct;
                            }
                        }
                    }
                    ctx.wound_dirt_pct = max_dirt;
                }
            }
        }
        ctx
    }

    fn m14h_scan_snapshot_for(&self, actor_id: u64) -> (u64, f32, u64) {
        let mut wound_count = 0u64;
        let mut pain_total = 0.0_f32;
        let mut buff_count = 0u64;
        if let Ok(state) = self.state.read() {
            if let Some(sim) = state.actor_state.as_ref() {
                if let Some(actor) = sim.world.actors.get(&ActorId(actor_id)) {
                    wound_count = actor.m14g_wound_list.total_count() as u64;
                    for (_, wounds) in actor.m14g_wound_list.iter() {
                        for w in wounds {
                            pain_total += w.severity * 0.5;
                        }
                    }
                    buff_count = actor.m14h_buffs.len() as u64;
                }
            }
        }
        (wound_count, pain_total, buff_count)
    }

    fn m14h_triage_queue_ordering(&self) -> Vec<u64> {
        let mut rows: Vec<(u64, f32)> = Vec::new();
        if let Ok(state) = self.state.read() {
            if let Some(sim) = state.actor_state.as_ref() {
                for (id, actor) in sim.world.actors.iter() {
                    if actor.m14g_wound_list.total_count() == 0 && !actor.m14h_cardiac.in_arrest {
                        continue;
                    }
                    let mut severity_sum = 0.0_f32;
                    for (_, wounds) in actor.m14g_wound_list.iter() {
                        for w in wounds {
                            severity_sum += w.severity;
                        }
                    }
                    let ttd = (60.0 / (severity_sum.max(0.01))).min(1_000_000.0);
                    rows.push((id.0, ttd));
                }
            }
        }
        rows.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        rows.into_iter().map(|(id, _)| id).collect()
    }

    // -----------------------------------------------------------------
    // Per-tick aging pass: tourniquet necrosis + buff expiry + antibiotic
    // dosing.
    // -----------------------------------------------------------------

    /// alongside the M14G wound aging pass.
    ///
    /// Mutates:
    /// - tourniquet zones past the 90-min threshold → mark zone necrotic.
    /// - expired buffs are removed; CombatStimT1 expiry inserts the
    ///   CombatStimT1Crash debuff.
    /// - antibiotic course advances doses on schedule; missed doses set
    ///   `resistance_risk = true`.
    pub fn m14h_tick(&self, tick: Tick, sim_time_ms: f64) -> usize {
        let tick_rate = self.config.tick_rate_hz.max(1) as u64;
        let necrosis_threshold_ticks =
            (cf_treatment::TOURNIQUET_NECROSIS_THRESHOLD_SECONDS as u64) * tick_rate;
        let mut emissions = 0usize;
        let mut newly_necrotic: Vec<(u64, String)> = Vec::new();
        let mut crash_to_insert: Vec<u64> = Vec::new();
        {
            let Ok(mut s) = self.state.write() else { return 0 };
            let Some(sim) = s.actor_state.as_mut() else { return 0 };
            for (actor_id, actor) in sim.world.actors.iter_mut() {
                // Tourniquet necrosis.
                let to_promote: Vec<ZoneId> = actor
                    .m14h_tourniquets
                    .iter()
                    .filter(|(zone, apply_tick)| {
                        tick.0.saturating_sub(**apply_tick) >= necrosis_threshold_ticks
                            && !actor.m14g_wound_list.necrotic_zones.contains(*zone)
                    })
                    .map(|(z, _)| z.clone())
                    .collect();
                for zone in to_promote {
                    actor.m14g_wound_list.necrotic_zones.insert(zone.clone());
                    newly_necrotic.push((actor_id.0, zone.as_str().to_string()));
                }
                // Buff expiry.
                let mut crashed_combat_stim = false;
                actor.m14h_buffs.retain(|b| {
                    if b.is_expired(tick.0) {
                        if matches!(b.kind, BuffKind::CombatStimT1) {
                            crashed_combat_stim = true;
                        }
                        false
                    } else {
                        true
                    }
                });
                if crashed_combat_stim {
                    crash_to_insert.push(actor_id.0);
                }
            }
        }
        for (actor_id, zone) in newly_necrotic {
            self.recorder.record(
                tick,
                sim_time_ms,
                "wound",
                "aged",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "zone": zone,
                    "wound_id": 0,
                    "new_state": "necrotic",
                }),
                None,
            );
            emissions += 1;
        }
        for actor_id in crash_to_insert {
            self.m14h_apply_buff(actor_id, BuffKind::CombatStimT1Crash, tick.0);
            emissions += 1;
        }
        emissions
    }

    // -----------------------------------------------------------------
    // Event recording helpers
    // -----------------------------------------------------------------

    fn m14h_record_treatment_event(&self, parent: String, ev: &TreatmentEvent) {
        match ev {
            TreatmentEvent::Applied {
                kind,
                actor_id,
                tick,
                apply_seconds,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "treatment",
                    "applied",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "kind": kind.as_str(),
                        "apply_seconds": apply_seconds,
                    }),
                    Some(parent),
                );
            }
            TreatmentEvent::Completed {
                kind,
                actor_id,
                tick,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "treatment",
                    "completed",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "kind": kind.as_str(),
                    }),
                    Some(parent),
                );
            }
            TreatmentEvent::Failed {
                kind,
                actor_id,
                tick,
                reason,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "treatment",
                    "failed",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "kind": kind.as_str(),
                        "reason": reason.as_str(),
                    }),
                    Some(parent),
                );
            }
        }
    }

    #[allow(dead_code)]
    fn m14h_record_cardiac_event(&self, parent: String, ev: &CardiacEvent) {
        match ev {
            CardiacEvent::Arrested {
                actor_id,
                tick,
                trigger,
                grace_seconds,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "cardiac",
                    "arrested",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "trigger": trigger.as_str(),
                        "grace_seconds": grace_seconds,
                    }),
                    Some(parent),
                );
            }
            CardiacEvent::CprRound {
                actor_id,
                tick,
                round_index,
                consecutive_cpr_rounds,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "cardiac",
                    "cpr_round",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "round_index": round_index,
                        "consecutive_cpr_rounds": consecutive_cpr_rounds,
                    }),
                    Some(parent),
                );
            }
            CardiacEvent::DefibAttempted {
                actor_id,
                tick,
                success_probability_x1000,
                roll_x1000,
                passed,
                charges_remaining,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "cardiac",
                    "defib_attempted",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "success_probability_x1000": success_probability_x1000,
                        "roll_x1000": roll_x1000,
                        "passed": passed,
                        "charges_remaining": charges_remaining,
                    }),
                    Some(parent),
                );
            }
            CardiacEvent::Restored { actor_id, tick } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "cardiac",
                    "restored",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                    }),
                    Some(parent),
                );
            }
            CardiacEvent::Expired { actor_id, tick } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "cardiac",
                    "expired",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                    }),
                    Some(parent),
                );
            }
        }
    }

    fn m14h_record_surgery_event(&self, parent: String, ev: &SurgeryEvent) {
        match ev {
            SurgeryEvent::PhaseStarted {
                actor_id,
                phase,
                tick,
                duration_seconds,
            } => {
                if matches!(phase, SurgeryPhase::Completed | SurgeryPhase::Failed) {
                    return;
                }
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "surgery",
                    "phase_started",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "phase": phase.as_str(),
                        "duration_seconds": duration_seconds,
                    }),
                    Some(parent),
                );
            }
            SurgeryEvent::PhaseCompleted {
                actor_id,
                phase,
                tick,
            } => {
                if matches!(phase, SurgeryPhase::Completed | SurgeryPhase::Failed) {
                    return;
                }
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "surgery",
                    "phase_completed",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "phase": phase.as_str(),
                    }),
                    Some(parent),
                );
            }
            SurgeryEvent::SkillCheck {
                actor_id,
                tick,
                step_index,
                passed,
                roll_x1000,
                threshold_x1000,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "surgery",
                    "skill_check",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "step_index": step_index,
                        "passed": passed,
                        "roll_x1000": roll_x1000,
                        "threshold_x1000": threshold_x1000,
                    }),
                    Some(parent),
                );
            }
            SurgeryEvent::Completed {
                actor_id,
                tick,
                wounds_treated,
                steps_passed,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "surgery",
                    "completed",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "wounds_treated": wounds_treated,
                        "steps_passed": steps_passed,
                    }),
                    Some(parent),
                );
            }
            SurgeryEvent::Failed {
                actor_id,
                tick,
                reason,
            } => {
                self.recorder.record(
                    Tick(*tick),
                    0.0,
                    "surgery",
                    "failed",
                    json!({
                        "actor_id": actor_id,
                        "tick": tick,
                        "reason": reason.as_str(),
                    }),
                    Some(parent),
                );
            }
        }
    }
}

impl M0Engine {
    /// given buff (by canonical snake_case id).
    pub fn m14h_actor_has_buff(&self, actor_id: u64, buff_id: &str) -> bool {
        let Ok(s) = self.state.read() else { return false };
        let Some(sim) = s.actor_state.as_ref() else { return false };
        let Some(actor) = sim.world.actors.get(&ActorId(actor_id)) else { return false };
        actor.m14h_buffs.iter().any(|b| b.kind.as_str() == buff_id)
    }

    /// doses_required + dose_interval_hours. Returns `None` when no
    /// course is active.
    pub fn m14h_actor_antibiotic_state(&self, actor_id: u64) -> Option<(u8, u32, f32)> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&ActorId(actor_id))?;
        actor
            .m14h_antibiotic_course
            .as_ref()
            .map(|c| (c.tier, c.doses_required, c.dose_interval_hours))
    }

    /// shocks delivered, consecutive CPR rounds, chest_bruised flag).
    pub fn m14h_actor_cardiac(&self, actor_id: u64) -> Option<ActorCardiacComponent> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&ActorId(actor_id))?;
        Some(actor.m14h_cardiac.clone())
    }

    /// ticks (zone → apply_tick).
    pub fn m14h_actor_tourniquets(
        &self,
        actor_id: u64,
    ) -> Option<std::collections::BTreeMap<String, u64>> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&ActorId(actor_id))?;
        Some(
            actor
                .m14h_tourniquets
                .iter()
                .map(|(z, t)| (z.as_str().to_string(), *t))
                .collect(),
        )
    }
}

/// Quiet the unused-imports linter for the standalone trait paths.
#[allow(dead_code)]
const _UNUSED_LINTS: ActorCardiacComponent = ActorCardiacComponent {
    in_arrest: false,
    onset_tick: 0,
    cpr_rounds_total: 0,
    consecutive_cpr_rounds: 0,
    charges_remaining: cf_treatment::DEFIB_CHARGES_DEFAULT,
    defib_shocks: 0,
    chest_bruised: false,
};

#[allow(dead_code)]
const _CPR_THRESHOLD_LINK: u32 = CPR_BRUISE_THRESHOLD_ROUNDS;
