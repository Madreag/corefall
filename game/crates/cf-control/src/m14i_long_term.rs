//! **M14I** § Long-term-consequence engine dispatchers + per-tick passive
//! pass.
//!
//! Owns:
//! - `act.player.install_prosthetic` / `maintain_prosthetic` /
//!   `retire_veteran` cfctl method dispatchers.
//! - Per-tick `m14i_tick` pass for biological aging, prosthetic wear,
//!   phantom-limb panic-roll cadence, radiation→cancer hand-off,
//!   chronic-condition baseline application, and per-actor terminal
//!   roll resolution.
//! - Scar acquisition hook (`m14i_record_scar_for_closed_wound`) called
//!   from the M14H treatment dispatchers.
//! - Phantom-limb hook (`m14i_record_phantom_limb`) called when an
//!   attachable detaches.
//! - Concussion hook (`m14i_record_concussion`) called when a Concussion
//!   wound at KO-threshold severity is emitted.
//! - Radiation dose hook (`m14i_add_radiation_dose`) for M17 / M16B
//!   integration.
//!
//! Determinism: every roll uses a seeded `Xoshiro256StarStar` derived
//! from `engine.seed ⊕ tick ⊕ actor_id`.

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde_json::json;

use cf_actor::long_term::{
    FunctionalAggregate, LongTermState, SeveredLimbRecord, MEMORY_LOSS_MAJOR_THRESHOLD,
    MEMORY_LOSS_MINOR_THRESHOLD, PHANTOM_LIMB_PANIC_INTERVAL_SECONDS,
    RADIATION_CANCER_THRESHOLD,
};
use cf_actor::traits::{ids as trait_ids, TraitSet};
use cf_actor::{ActorId, IntentSource, Status};
use cf_aging::{
    AgingEvent, AgingOrigin, BiologicalAge, TerminalRollOutcome, SECONDS_PER_IN_GAME_YEAR,
};
use cf_prosthetic::{
    maintain_prosthetic, prosthetic_spec, InstallSession, ProstheticInstance, ProstheticKind,
    ProstheticTier, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS,
};
use cf_scar::{functional_debuff_for, FunctionalDebuff, ScarId, ScarRecord};
use cf_sim_core::Tick;
use cf_wound::registry::{OriginId, TreatmentKind, VisualDecalId, ZoneId};
use cf_wound::WoundKind;

use crate::engine::M0Engine;
use crate::server::CommandResult;

/// Minimum age delta past `retirement_age` required for the Retire
/// action to commit. Spec § "When an actor reaches `retirement_age + 5`,
/// the M41 veteran roster offers a Retire action".
const RETIREMENT_LOCK_DELTA_YEARS: f32 = 5.0;

fn source_label(source: IntentSource) -> &'static str {
    match source {
        IntentSource::Human => "human",
        IntentSource::Cfctl => "cfctl",
        IntentSource::Ai => "ai",
        IntentSource::Replay => "replay",
    }
}

impl M0Engine {
    /// **M14I** § `act.player.install_prosthetic` dispatch.
    pub(crate) fn dispatch_m14i_install_prosthetic(
        &self,
        target_actor_id: u64,
        kind_str: String,
        zone_str: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label_str = source_label(source);
        let Some(kind) = ProstheticKind::from_str(&kind_str) else {
            self.record_command_rejected(
                tick,
                sim_time_ms,
                "act.player.install_prosthetic",
                "unknown_prosthetic_kind",
            );
            return CommandResult::rejected("unknown_prosthetic_kind", tick.0);
        };
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.install_prosthetic",
                "target_actor_id": target_actor_id,
                "kind": kind.as_str(),
                "zone": zone_str,
                "source": source_label_str,
            }),
            None,
        );
        let zone = ZoneId::from(zone_str.as_str());
        let spec = prosthetic_spec(kind);
        // Pull origin + severed-status from the actor.
        let (origin_label, zone_severed) = {
            let Ok(state) = self.state.read() else {
                return CommandResult::rejected("engine_state_poisoned", tick.0);
            };
            let Some(sim) = state.actor_state.as_ref() else {
                return CommandResult::rejected("no_actor_world", tick.0);
            };
            let Some(actor) = sim.world.actors.get(&ActorId(target_actor_id)) else {
                return CommandResult::rejected("unknown_target_actor", tick.0);
            };
            let zone_severed = actor.m14i_long_term.severed_limbs.contains_key(&zone);
            (actor.origin_id.clone(), zone_severed)
        };
        let origin = OriginId::from(origin_label.as_str());
        let install_result = InstallSession::start(
            target_actor_id,
            kind,
            zone.clone(),
            &spec,
            &origin,
            true, // medic_t2 assumed (engine-side fast-path; real game gates on actor.skills).
            true, // surgery table assumed.
            zone_severed,
        );
        let mut session = match install_result {
            Ok(s) => s,
            Err(err) => {
                let reason = match err {
                    cf_prosthetic::install::InstallError::WrongOrigin => "wrong_origin",
                    cf_prosthetic::install::InstallError::WrongZone => "wrong_zone",
                    cf_prosthetic::install::InstallError::MissingSkill => "missing_skill",
                    cf_prosthetic::install::InstallError::MissingTool => "missing_tool",
                    cf_prosthetic::install::InstallError::NotSevered => "zone_not_severed",
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "method": "act.player.install_prosthetic",
                        "reason": reason,
                    }),
                    Some(action_id),
                );
                return CommandResult::rejected(reason, tick.0);
            }
        };
        // Run install to completion in-place (spec § "60s sequence" runs
        // inline in the engine bridge fast-path).
        let inst = session.install(tick.0);
        let restoration = inst.tier.functional_restoration();
        let inst_kind = inst.kind;
        let inst_tier = inst.tier;
        let inst_zone = inst.zone.clone();
        let mut phantom_panic_chance_after: Option<f32> = None;
        {
            let Ok(mut s) = self.state.write() else {
                return CommandResult::rejected("engine_state_poisoned", tick.0);
            };
            let Some(sim) = s.actor_state.as_mut() else {
                return CommandResult::rejected("no_actor_world", tick.0);
            };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(target_actor_id)) else {
                return CommandResult::rejected("unknown_target_actor", tick.0);
            };
            actor.m14i_long_term.prosthetics.push(inst.clone());
            actor.m14i_long_term.aggregate.apply_prosthetic(&inst);
            // Spec § "phantom_limb panic-roll multiplier × 0.25" once a
            // prosthetic restores the severed zone.
            if actor.m14i_long_term.severed_limbs.contains_key(&inst.zone) {
                actor.m14i_long_term.aggregate.phantom_panic_chance =
                    (actor.m14i_long_term.aggregate.phantom_panic_chance * 0.25).clamp(0.0, 1.0);
                phantom_panic_chance_after =
                    Some(actor.m14i_long_term.aggregate.phantom_panic_chance);
            }
        }
        self.recorder.record(
            tick,
            sim_time_ms,
            "prosthetic",
            "installed",
            json!({
                "actor_id": target_actor_id,
                "tick": tick.0,
                "kind": inst_kind.as_str(),
                "tier": inst_tier.as_str(),
                "zone": inst_zone.as_str(),
                "functional_restoration": restoration,
                "phantom_panic_chance_after": phantom_panic_chance_after,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// **M14I** § `act.player.maintain_prosthetic` dispatch.
    pub(crate) fn dispatch_m14i_maintain_prosthetic(
        &self,
        target_actor_id: u64,
        zone_str: String,
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
                "method": "act.player.maintain_prosthetic",
                "target_actor_id": target_actor_id,
                "zone": zone_str,
                "source": source_label_str,
            }),
            None,
        );
        let zone = ZoneId::from(zone_str.as_str());
        let maintained_kind = {
            let Ok(mut s) = self.state.write() else {
                return CommandResult::rejected("engine_state_poisoned", tick.0);
            };
            let Some(sim) = s.actor_state.as_mut() else {
                return CommandResult::rejected("no_actor_world", tick.0);
            };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(target_actor_id)) else {
                return CommandResult::rejected("unknown_target_actor", tick.0);
            };
            let mut maintained: Option<ProstheticKind> = None;
            for inst in actor.m14i_long_term.prosthetics.iter_mut() {
                if inst.zone == zone {
                    match maintain_prosthetic(inst, true, tick.0) {
                        Ok(_) => {
                            maintained = Some(inst.kind);
                            break;
                        }
                        Err(err) => {
                            let reason = match err {
                                cf_prosthetic::install::MaintenanceError::MissingSkill => {
                                    "missing_skill"
                                }
                                cf_prosthetic::install::MaintenanceError::NotInstalled => {
                                    "no_prosthetic_at_zone"
                                }
                            };
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "control",
                                "command_rejected",
                                json!({
                                    "method": "act.player.maintain_prosthetic",
                                    "reason": reason,
                                }),
                                Some(action_id.clone()),
                            );
                            return CommandResult::rejected(reason, tick.0);
                        }
                    }
                }
            }
            maintained
        };
        let Some(kind) = maintained_kind else {
            self.recorder.record(
                tick,
                sim_time_ms,
                "control",
                "command_rejected",
                json!({
                    "method": "act.player.maintain_prosthetic",
                    "reason": "no_prosthetic_at_zone",
                }),
                Some(action_id),
            );
            return CommandResult::rejected("no_prosthetic_at_zone", tick.0);
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "prosthetic",
            "maintained",
            json!({
                "actor_id": target_actor_id,
                "tick": tick.0,
                "kind": kind.as_str(),
                "zone": zone.as_str(),
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// **M14I** § `act.player.retire_veteran` dispatch.
    pub(crate) fn dispatch_m14i_retire_veteran(
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
                "method": "act.player.retire_veteran",
                "target_actor_id": target_actor_id,
                "source": source_label_str,
            }),
            None,
        );
        let age_when_retired: Option<f32> = {
            let Ok(mut s) = self.state.write() else {
                return CommandResult::rejected("engine_state_poisoned", tick.0);
            };
            // Pre-flight + actor mutations in a tight scope so the
            // veteran-roster + retirement-narrative writes below can
            // re-borrow `s` once we're done.
            let outcome: Option<(f32, cf_actor::long_term::LongTermState, String, String)> = {
                let Some(sim) = s.actor_state.as_mut() else {
                    return CommandResult::rejected("no_actor_world", tick.0);
                };
                let Some(actor) = sim.world.actors.get_mut(&ActorId(target_actor_id)) else {
                    return CommandResult::rejected("unknown_target_actor", tick.0);
                };
                let actor_team = actor.team.clone();
                let actor_origin = actor.origin_id.clone();
                let lt = &mut actor.m14i_long_term;
                let age_now = lt
                    .biological_age
                    .as_ref()
                    .map(|a| a.age_in_game_years)
                    .unwrap_or(0.0);
                let retire_age = lt
                    .biological_age
                    .as_ref()
                    .map(|a| a.retirement_age)
                    .unwrap_or(f32::INFINITY);
                if !lt.retirement_offered
                    || age_now < retire_age + RETIREMENT_LOCK_DELTA_YEARS
                {
                    None
                } else {
                    lt.retired = true;
                    lt.retired_tick = tick.0;
                    lt.traits.insert(trait_ids::RETIRED_VETERAN);
                    Some((age_now, lt.clone(), actor_team, actor_origin))
                }
            };
            outcome.map(|(age_now, lt_snapshot, team, origin)| {
                // M14I § veteran roster + storyteller integration —
                // recorded after the actor borrow is dropped.
                let dossier = s.m14i_veteran_roster.entry_mut(target_actor_id);
                dossier.display_name = team;
                dossier.origin_label = origin;
                dossier.biological_age = lt_snapshot.biological_age.clone();
                dossier.scar_timeline = lt_snapshot.scar_timeline.clone();
                dossier.prosthetics = lt_snapshot.prosthetics.clone();
                dossier.retire(tick.0);
                cf_storyteller::register_retirement_narrative(
                    &mut s.m14i_retirement_narratives,
                    target_actor_id,
                    age_now,
                    tick.0,
                );
                age_now
            })
        };
        let Some(age) = age_when_retired else {
            self.recorder.record(
                tick,
                sim_time_ms,
                "control",
                "command_rejected",
                json!({
                    "method": "act.player.retire_veteran",
                    "reason": "not_yet_eligible",
                }),
                Some(action_id),
            );
            return CommandResult::rejected("not_yet_eligible", tick.0);
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "veteran",
            "retired",
            json!({
                "actor_id": target_actor_id,
                "tick": tick.0,
                "age_in_game_years": age,
                "narrative_event_id":
                    cf_storyteller::retirement_event::NARRATIVE_EVENT_ID_VETERAN_RETIRED,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// **M14I** § per-tick long-term-consequence pass — wired into the
    /// M0 engine tick loop alongside M14H. Returns the number of events
    /// emitted. Uses the engine's configured tick rate to derive
    /// `dt_seconds`.
    pub fn m14i_tick(&self, tick: Tick, sim_time_ms: f64) -> usize {
        let tick_rate = self.config.tick_rate_hz.max(1) as f32;
        let dt_seconds = 1.0 / tick_rate;
        self.m14i_tick_with_dt(tick, sim_time_ms, dt_seconds)
    }

    /// **M14I** § per-tick long-term-consequence pass with explicit
    /// `dt_seconds`. Tests use this to fast-forward through years of
    /// in-game time without simulating millions of sim ticks. Returns
    /// the number of events emitted.
    pub fn m14i_tick_with_dt(&self, tick: Tick, sim_time_ms: f64, dt_seconds: f32) -> usize {
        let mut emissions = 0usize;
        let mut aging_events: Vec<(u64, AgingEvent)> = Vec::new();
        let mut malfunction_events: Vec<(u64, ProstheticKind, ZoneId, f32)> = Vec::new();
        let mut phantom_events: Vec<(u64, ZoneId, u32, bool)> = Vec::new();
        let mut cancer_handoffs: Vec<(u64, f32)> = Vec::new();
        let mut terminal_died: Vec<u64> = Vec::new();
        // **M14I** post-survival pass: actors who were DYING at the
        // moment of attachable.detached have a severed limb but no
        // phantom_limb trait yet. Promote them once they're back in a
        // survivable status.
        let mut post_survival_promotions: Vec<(u64, ZoneId, u64)> = Vec::new();
        let mut current_seed = self.config.seed;
        {
            let Ok(mut s) = self.state.write() else { return 0 };
            let Some(sim) = s.actor_state.as_mut() else { return 0 };
            current_seed = current_seed.wrapping_add(tick.0);
            for (actor_id_key, actor) in sim.world.actors.iter_mut() {
                let actor_id = actor_id_key.0;
                let actor_status = actor.status;
                let lt = &mut actor.m14i_long_term;
                // ---- Post-survival phantom-limb promotion ----
                let actor_survivable = matches!(
                    actor_status,
                    cf_actor::Status::Stable | cf_actor::Status::Unstable | cf_actor::Status::Downed
                );
                if actor_survivable {
                    let mut promoted_any = false;
                    for (zone, rec) in lt.severed_limbs.iter_mut() {
                        if !rec.phantom_limb_registered {
                            rec.phantom_limb_registered = true;
                            post_survival_promotions.push((
                                actor_id,
                                zone.clone(),
                                rec.tick_severed,
                            ));
                            promoted_any = true;
                        }
                    }
                    if promoted_any {
                        lt.traits.insert(cf_actor::traits::ids::PHANTOM_LIMB);
                        // Spec § "phantom-limb panic-roll 1× / week" with
                        // baseline chance for the per-week roll.
                        lt.aggregate.phantom_panic_chance = (lt.aggregate.phantom_panic_chance
                            + cf_actor::long_term::PHANTOM_LIMB_PANIC_BASE_CHANCE)
                            .clamp(0.0, 1.0);
                    }
                }
                // ---- Biological aging clock + retirement / terminal ----
                if let Some(age) = lt.biological_age.as_mut() {
                    if age.origin.is_biological() {
                        let result = age.tick(dt_seconds, tick.0);
                        if result.year_advanced {
                            aging_events.push((
                                actor_id,
                                AgingEvent::YearAdvanced {
                                    actor_id,
                                    tick: tick.0,
                                    new_age_years: result.new_age_years,
                                    caloric_max_decay: age.caloric_max_decay,
                                    max_speed_decay: age.max_speed_decay,
                                    heal_rate_decay: age.heal_rate_decay,
                                },
                            ));
                        }
                        if result.retirement_offered {
                            lt.retirement_offered = true;
                            aging_events.push((
                                actor_id,
                                AgingEvent::RetirementOffered {
                                    actor_id,
                                    tick: tick.0,
                                    age_in_game_years: age.age_in_game_years,
                                },
                            ));
                        }
                        if let Some(roll) = result.terminal_roll {
                            let died = age.resolve_terminal_roll(
                                roll,
                                current_seed ^ actor_id,
                            );
                            aging_events.push((
                                actor_id,
                                AgingEvent::TerminalRoll {
                                    actor_id,
                                    tick: tick.0,
                                    probability_x1000: roll.probability_x1000,
                                    outcome: if died {
                                        TerminalRollOutcome::Death
                                    } else {
                                        TerminalRollOutcome::Survived
                                    },
                                },
                            ));
                            if died {
                                terminal_died.push(actor_id);
                            }
                        }
                    } else {
                        // Mechanical origins accumulate chassis wear in lieu
                        // of biological aging.
                        lt.chassis_wear_pct =
                            (lt.chassis_wear_pct + dt_seconds * 1e-7).min(1.0);
                    }
                }
                // ---- Prosthetic wear ----
                for inst in lt.prosthetics.iter_mut() {
                    let crossed = inst.advance_wear(
                        dt_seconds,
                        PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS,
                    );
                    if crossed {
                        malfunction_events.push((
                            actor_id,
                            inst.kind,
                            inst.zone.clone(),
                            inst.wear_pct,
                        ));
                    }
                }
                // ---- Phantom-limb panic-roll cadence ----
                let phantom_chance = lt.aggregate.phantom_panic_chance;
                let mut rng = Xoshiro256StarStar::seed_from_u64(
                    current_seed ^ actor_id ^ 0x504c504c5050414b,
                );
                let prosthetic_zones: std::collections::BTreeSet<ZoneId> = lt
                    .prosthetics
                    .iter()
                    .map(|p| p.zone.clone())
                    .collect();
                for (zone, rec) in lt.severed_limbs.iter_mut() {
                    if !rec.phantom_limb_registered {
                        continue;
                    }
                    rec.seconds_since_last_panic += dt_seconds;
                    if rec.seconds_since_last_panic >= PHANTOM_LIMB_PANIC_INTERVAL_SECONDS {
                        rec.seconds_since_last_panic = 0.0;
                        let prosthetic_installed =
                            prosthetic_zones.contains(zone);
                        let chance = if prosthetic_installed {
                            (phantom_chance * 0.25).clamp(0.0, 1.0)
                        } else {
                            phantom_chance.clamp(0.0, 1.0)
                        };
                        let p_x1000 = (chance * 1000.0).round() as u32;
                        let roll = (rng.next_u64() % 1000) as u32;
                        if roll < p_x1000 {
                            rec.panic_rolls_fired =
                                rec.panic_rolls_fired.saturating_add(1);
                            phantom_events.push((
                                actor_id,
                                zone.clone(),
                                rec.panic_rolls_fired,
                                prosthetic_installed,
                            ));
                        }
                    }
                }
                // ---- Radiation → cancer handoff ----
                if !lt.cancer_handoff_fired
                    && lt.cumulative_radiation_dose >= RADIATION_CANCER_THRESHOLD
                {
                    lt.cancer_handoff_fired = true;
                    cancer_handoffs.push((actor_id, lt.cumulative_radiation_dose));
                }
            }
        }
        for (actor_id, ev) in aging_events {
            self.emit_m14i_aging_event(tick, sim_time_ms, actor_id, &ev);
            emissions += 1;
        }
        for (actor_id, kind, zone, wear) in malfunction_events {
            self.recorder.record(
                tick,
                sim_time_ms,
                "prosthetic",
                "malfunctioned",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "kind": kind.as_str(),
                    "zone": zone.as_str(),
                    "wear_pct": wear,
                }),
                None,
            );
            emissions += 1;
        }
        for (actor_id, zone, roll_index, prosthetic_installed) in phantom_events {
            self.recorder.record(
                tick,
                sim_time_ms,
                "phantom_limb",
                "panic_attack",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "zone": zone.as_str(),
                    "roll_index": roll_index,
                    "prosthetic_installed": prosthetic_installed,
                }),
                None,
            );
            emissions += 1;
        }
        for (actor_id, dose) in cancer_handoffs {
            self.recorder.record(
                tick,
                sim_time_ms,
                "disease",
                "exposed",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "vector": "long_term_radiation",
                    "cumulative_dose": dose,
                    "threshold": RADIATION_CANCER_THRESHOLD,
                }),
                None,
            );
            emissions += 1;
        }
        // **M14I** § post-survival phantom-limb emissions.
        for (actor_id, zone, tick_severed) in post_survival_promotions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "phantom_limb",
                "acquired",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "zone": zone.as_str(),
                    "tick_severed": tick_severed,
                }),
                None,
            );
            emissions += 1;
        }
        // Apply terminal deaths (status transition + event).
        for actor_id in terminal_died {
            let prev_status = {
                let Ok(mut s) = self.state.write() else { continue };
                let Some(sim) = s.actor_state.as_mut() else { continue };
                let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else {
                    continue;
                };
                let prev = actor.status;
                actor.status = Status::Dead;
                prev
            };
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "actor_status_changed",
                json!({
                    "actor": actor_id,
                    "previous_status": prev_status.as_str(),
                    "new_status": Status::Dead.as_str(),
                    "cause": "old_age",
                }),
                None,
            );
            emissions += 1;
        }
        emissions
    }

    fn emit_m14i_aging_event(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        _actor_id: u64,
        ev: &AgingEvent,
    ) {
        match ev {
            AgingEvent::YearAdvanced {
                actor_id,
                tick: t,
                new_age_years,
                caloric_max_decay,
                max_speed_decay,
                heal_rate_decay,
            } => {
                self.recorder.record(
                    Tick(*t),
                    sim_time_ms,
                    "age",
                    "year_advanced",
                    json!({
                        "actor_id": actor_id,
                        "tick": t,
                        "new_age_years": new_age_years,
                        "caloric_max_decay": caloric_max_decay,
                        "max_speed_decay": max_speed_decay,
                        "heal_rate_decay": heal_rate_decay,
                    }),
                    None,
                );
                let _ = tick;
            }
            AgingEvent::RetirementOffered {
                actor_id,
                tick: t,
                age_in_game_years,
            } => {
                self.recorder.record(
                    Tick(*t),
                    sim_time_ms,
                    "age",
                    "retirement_offered",
                    json!({
                        "actor_id": actor_id,
                        "tick": t,
                        "age_in_game_years": age_in_game_years,
                    }),
                    None,
                );
            }
            AgingEvent::TerminalRoll {
                actor_id,
                tick: t,
                probability_x1000,
                outcome,
            } => {
                self.recorder.record(
                    Tick(*t),
                    sim_time_ms,
                    "age",
                    "terminal_roll",
                    json!({
                        "actor_id": actor_id,
                        "tick": t,
                        "probability_x1000": probability_x1000,
                        "outcome": outcome.as_str(),
                    }),
                    None,
                );
            }
        }
    }

    /// **M14I** § scar-acquisition hook. Called from the M14H treatment
    /// dispatchers whenever a closure-type treatment completes against a
    /// matching wound.
    ///
    /// Walks the actor's `m14g_wound_list` for the given zone and
    /// promotes every wound whose visible state has reached `SutureLine`
    /// / `Scar` (or whose `WoundSpec.closes_to_scar` is true) to a
    /// `ScarRecord` on the actor's `m14i_long_term.scar_timeline`. Each
    /// promotion fires a `scar.acquired` event.
    pub fn m14i_record_scars_for_closure(
        &self,
        actor_id: u64,
        closure_method: TreatmentKind,
        zone_filter: Option<&ZoneId>,
        tick: Tick,
        sim_time_ms: f64,
        parent: Option<String>,
    ) -> usize {
        use cf_wound::WoundVisibleState;
        let mut to_emit: Vec<(ScarId, WoundKind, ZoneId, f32, FunctionalDebuff, String)> = Vec::new();
        let mut chronic_pain_delta: f32 = 0.0;
        {
            let Ok(mut s) = self.state.write() else { return 0 };
            let Some(sim) = s.actor_state.as_mut() else { return 0 };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else {
                return 0;
            };
            // Collect candidate wounds (mark visited so we don't double-
            // count a wound on the same closure pass).
            let zones: Vec<ZoneId> = actor
                .m14g_wound_list
                .wounds_by_zone
                .keys()
                .filter(|z| zone_filter.is_none_or(|zf| *z == zf))
                .cloned()
                .collect();
            for zone in zones {
                if let Some(wounds) =
                    actor.m14g_wound_list.wounds_by_zone.get_mut(&zone)
                {
                    for w in wounds.iter_mut() {
                        // Already promoted to scar — skip.
                        if w.scarred {
                            continue;
                        }
                        // Only close wounds that the closure method can
                        // address (sutures / cauterize / surgery).
                        let is_closure = matches!(
                            closure_method,
                            TreatmentKind::SutureKit
                                | TreatmentKind::SurgeryKit
                                | TreatmentKind::BurnGel
                        );
                        if !is_closure {
                            continue;
                        }
                        // Also gate on the wound's actual visible state:
                        // either it's already sutured / scab / scar OR the
                        // applied treatment matches the closure type.
                        if !matches!(
                            w.visible_state,
                            WoundVisibleState::SutureLine
                                | WoundVisibleState::Scab
                                | WoundVisibleState::Scar
                                | WoundVisibleState::CleanBandage
                        ) {
                            // Force the visible state forward so the M14G
                            // aging pass doesn't re-emit `scarred` later.
                            w.visible_state = WoundVisibleState::SutureLine;
                        }
                        let debuff = functional_debuff_for(
                            w.kind,
                            closure_method,
                            w.severity,
                            &zone,
                        );
                        let pain_bonus =
                            cf_scar::functional_debuff::chronic_pain_bonus_for(
                                w.kind,
                                closure_method,
                                w.severity,
                            );
                        chronic_pain_delta += pain_bonus;
                        let scar_id =
                            actor.m14i_long_term.scar_timeline.alloc_id();
                        let decal_id =
                            VisualDecalId::from(scar_decal_id(w.kind).as_str());
                        let record = ScarRecord {
                            scar_id,
                            source_wound_kind: w.kind,
                            zone: zone.clone(),
                            severity_at_close: w.severity,
                            closure_method,
                            tick_acquired: tick.0,
                            functional_debuff: debuff.clone(),
                            cosmetic_decal_id: decal_id.clone(),
                            narrative_context: None,
                        };
                        actor
                            .m14i_long_term
                            .scar_timeline
                            .scars
                            .push(record);
                        actor.m14i_long_term.aggregate.add_debuff(&debuff);
                        w.scarred = true;
                        to_emit.push((
                            scar_id,
                            w.kind,
                            zone.clone(),
                            w.severity,
                            debuff,
                            decal_id.as_str().to_string(),
                        ));
                    }
                }
            }
            actor.m14i_long_term.chronic_pain_baseline += chronic_pain_delta;
        }
        let count = to_emit.len();
        for (scar_id, wound_kind, zone, severity, debuff, decal) in to_emit {
            self.recorder.record(
                tick,
                sim_time_ms,
                "scar",
                "acquired",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "scar_id": scar_id.raw(),
                    "kind": wound_kind.as_str(),
                    "zone": zone.as_str(),
                    "severity_at_close": severity,
                    "closure_method": closure_method.as_str(),
                    "functional_debuff": debuff.tag(),
                    "cosmetic_decal_id": decal,
                }),
                parent.clone(),
            );
        }
        count
    }

    /// **M14I** § phantom-limb hook. Called when an actor's `attachable`
    /// detaches via the M14 detach pass.
    ///
    /// If the actor is currently in a survivable status
    /// (Stable / Unstable / Downed), the phantom-limb trait registers
    /// immediately and `phantom_limb.acquired` fires here. If the actor
    /// is currently DYING the trait remains pending — the per-tick
    /// `m14i_tick` post-survival pass promotes it once the actor
    /// recovers (or never, if the actor dies).
    pub fn m14i_record_phantom_limb(
        &self,
        actor_id: u64,
        zone_label: &str,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        self.m14i_record_phantom_limb_with_parent(
            actor_id,
            zone_label,
            tick,
            sim_time_ms,
            None,
        );
    }

    /// **M14I** § phantom-limb hook variant that chains the
    /// `phantom_limb.acquired` event to a parent (e.g. the
    /// `attachable.detached` event id) so the cause-chain walker can
    /// trace the severance to the original projectile hit.
    pub fn m14i_record_phantom_limb_with_parent(
        &self,
        actor_id: u64,
        zone_label: &str,
        tick: Tick,
        sim_time_ms: f64,
        parent: Option<String>,
    ) {
        let zone = ZoneId::from(zone_label);
        let registered_now;
        {
            let Ok(mut s) = self.state.write() else { return };
            let Some(sim) = s.actor_state.as_mut() else { return };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else {
                return;
            };
            let lt = &mut actor.m14i_long_term;
            let entry = lt
                .severed_limbs
                .entry(zone.clone())
                .or_insert_with(|| SeveredLimbRecord::new(zone.clone(), tick.0));
            // Spec scenario 2: "When the post-survival pass runs:
            // phantom_limb.acquired fires". Only when the actor isn't
            // currently DYING; DYING actors emit the trait only after
            // recovery (handled by the per-tick post-survival pass).
            let actor_status = actor.status;
            registered_now = if matches!(actor_status, Status::Stable | Status::Unstable | Status::Downed)
                && !entry.phantom_limb_registered
            {
                entry.phantom_limb_registered = true;
                actor.m14i_long_term.traits.insert(trait_ids::PHANTOM_LIMB);
                // Spec § "phantom-limb panic-roll 1× / week" — bias the
                // chance to PHANTOM_LIMB_PANIC_BASE_CHANCE default.
                actor.m14i_long_term.aggregate.phantom_panic_chance =
                    (actor.m14i_long_term.aggregate.phantom_panic_chance
                        + cf_actor::long_term::PHANTOM_LIMB_PANIC_BASE_CHANCE)
                        .clamp(0.0, 1.0);
                true
            } else {
                false
            };
        }
        if registered_now {
            self.recorder.record(
                tick,
                sim_time_ms,
                "phantom_limb",
                "acquired",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "zone": zone.as_str(),
                    "tick_severed": tick.0,
                }),
                parent,
            );
        }
    }

    /// **M14I** § concussion hook. Called when a Concussion wound at
    /// KO-threshold severity (>= 0.5) fires. Bumps the actor's
    /// `concussion_count` and emits `memory_loss.minor_acquired` /
    /// `memory_loss.major_acquired` on threshold crossings.
    pub fn m14i_record_concussion(&self, actor_id: u64, tick: Tick, sim_time_ms: f64) {
        let (minor_now, major_now, count) = {
            let Ok(mut s) = self.state.write() else { return };
            let Some(sim) = s.actor_state.as_mut() else { return };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else {
                return;
            };
            let lt = &mut actor.m14i_long_term;
            lt.concussion_count = lt.concussion_count.saturating_add(1);
            let count = lt.concussion_count;
            let minor_now = count == MEMORY_LOSS_MINOR_THRESHOLD
                && lt.traits.insert(trait_ids::MEMORY_LOSS_MINOR);
            let major_now = count == MEMORY_LOSS_MAJOR_THRESHOLD
                && lt.traits.insert(trait_ids::MEMORY_LOSS_MAJOR);
            (minor_now, major_now, count)
        };
        if minor_now {
            self.recorder.record(
                tick,
                sim_time_ms,
                "memory_loss",
                "minor_acquired",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "concussion_count": count,
                }),
                None,
            );
        }
        if major_now {
            self.recorder.record(
                tick,
                sim_time_ms,
                "memory_loss",
                "major_acquired",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "concussion_count": count,
                }),
                None,
            );
        }
    }

    /// **M14I** § radiation dose accumulator. Called by M17 cumulative-
    /// dose tracker. Emits `disease.exposed` once on threshold crossing
    /// (M16B consumer activates `cancer.lifecycle`).
    pub fn m14i_add_radiation_dose(
        &self,
        actor_id: u64,
        delta_sieverts: f32,
        tick: Tick,
        sim_time_ms: f64,
    ) {
        let (crossed, dose) = {
            let Ok(mut s) = self.state.write() else { return };
            let Some(sim) = s.actor_state.as_mut() else { return };
            let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) else {
                return;
            };
            let lt = &mut actor.m14i_long_term;
            lt.cumulative_radiation_dose = (lt.cumulative_radiation_dose + delta_sieverts).max(0.0);
            let crossed =
                !lt.cancer_handoff_fired && lt.cumulative_radiation_dose >= RADIATION_CANCER_THRESHOLD;
            if crossed {
                lt.cancer_handoff_fired = true;
            }
            (crossed, lt.cumulative_radiation_dose)
        };
        if crossed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "disease",
                "exposed",
                json!({
                    "actor_id": actor_id,
                    "tick": tick.0,
                    "vector": "long_term_radiation",
                    "cumulative_dose": dose,
                    "threshold": RADIATION_CANCER_THRESHOLD,
                }),
                None,
            );
        }
    }

    /// **M14I** § ensure the actor carries a biological age clock for
    /// the resolved origin id. No-op if the actor already has one. Idempotent.
    pub fn m14i_ensure_age_clock(&self, actor_id: u64, initial_age_years: f32) {
        if let Ok(mut s) = self.state.write() {
            if let Some(sim) = s.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) {
                    let origin = AgingOrigin::from_label(actor.origin_id.as_str());
                    let lt = &mut actor.m14i_long_term;
                    if lt.biological_age.is_none() {
                        lt.biological_age =
                            Some(BiologicalAge::new_for_origin(origin, initial_age_years));
                    }
                }
            }
        }
    }

    /// **M14I** § test helper — read the actor's long-term snapshot.
    pub fn m14i_actor_long_term_snapshot(
        &self,
        actor_id: u64,
    ) -> Option<LongTermState> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&ActorId(actor_id))?;
        Some(actor.m14i_long_term.clone())
    }

    /// **M14I** § test helper — inject a chronic-condition trait without
    /// the M16C lifecycle. Used by scenario 9 (chronic-depression
    /// baseline) and content-validation tests.
    pub fn m14i_apply_chronic_condition(&self, actor_id: u64, trait_id: &str) {
        if !trait_id.starts_with(trait_ids::CHRONIC_PREFIX) {
            return;
        }
        if let Ok(mut s) = self.state.write() {
            if let Some(sim) = s.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) {
                    actor.m14i_long_term.traits.insert(trait_id);
                    if trait_id == trait_ids::CHRONIC_PAIN {
                        actor.m14i_long_term.chronic_pain_baseline += 4.0;
                    }
                }
            }
        }
    }

    /// **M14I** § test helper — run `f` with mutable access to the
    /// engine's actor sim. Returns `Some(R)` when the state was held,
    /// `None` when the engine wasn't fully initialized.
    pub fn with_mut_state<R>(
        &self,
        f: impl FnOnce(&mut cf_actor::sim::ActorSimState) -> R,
    ) -> Option<R> {
        let mut s = self.state.write().ok()?;
        let sim = s.actor_state.as_mut()?;
        Some(f(sim))
    }

    /// **M14I** § scenario 9 — chronic_depression "Refuse non-essential
    /// order" roll. Returns true ~10% of the time when the actor has the
    /// `chronic_depression` trait; always false otherwise. Determinism
    /// is anchored by the engine seed + sim tick + actor id so the AI
    /// tick (cf-ai) reproduces the same answer across saves.
    pub fn m14i_chronic_depression_refuse_roll(&self, actor_id: u64, tick: Tick) -> bool {
        let Ok(s) = self.state.read() else { return false };
        let Some(sim) = s.actor_state.as_ref() else { return false };
        let Some(actor) = sim.world.actors.get(&ActorId(actor_id)) else {
            return false;
        };
        let seed = self
            .config
            .seed
            .wrapping_add(tick.0)
            .wrapping_add(actor_id)
            ^ 0x4348524F4E494343; // "CHRONICCC"
        actor
            .m14i_long_term
            .chronic_depression_refuse_roll(seed)
    }

    /// **M14I** § read the engine's veteran roster (cf-veteran). The
    /// roster is populated when an actor commits to retirement; M41 +
    /// M48C consume it.
    pub fn m14i_veteran_roster(&self) -> Option<cf_veteran::VeteranRoster> {
        let s = self.state.read().ok()?;
        Some(s.m14i_veteran_roster.clone())
    }

    /// **M14I** § read the engine's retirement-narrative registry
    /// (cf-storyteller). Populated when retire commits.
    pub fn m14i_retirement_narratives(
        &self,
    ) -> Option<cf_storyteller::retirement_event::RetirementNarrativeRegistry> {
        let s = self.state.read().ok()?;
        Some(s.m14i_retirement_narratives.clone())
    }

    /// **M14I** § build the per-actor dossier view (cf-ui::veteran_dossier).
    /// M48C consumes this to render the pilot dossier tab.
    pub fn m14i_actor_dossier_view(
        &self,
        actor_id: u64,
    ) -> Option<cf_ui::veteran_dossier::VeteranDossierView> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        let actor = sim.world.actors.get(&ActorId(actor_id))?;
        let persisted = s.m14i_veteran_roster.get(actor_id);
        Some(cf_ui::veteran_dossier::build_view(
            actor_id,
            actor.team.as_str(),
            actor.origin_id.as_str(),
            &actor.m14i_long_term,
            persisted,
        ))
    }

    /// **M14I** § resolve an actor's effective move-speed multiplier
    /// (consumers: cf-actor walking sim + AI doctrine).
    pub fn m14i_actor_move_speed_multiplier(&self, actor_id: u64) -> f32 {
        let Ok(s) = self.state.read() else { return 1.0 };
        let Some(sim) = s.actor_state.as_ref() else { return 1.0 };
        let Some(actor) = sim.world.actors.get(&ActorId(actor_id)) else { return 1.0 };
        let has_chronic_depression = actor
            .m14i_long_term
            .traits
            .has(trait_ids::CHRONIC_DEPRESSION);
        actor
            .m14i_long_term
            .aggregate
            .move_speed_multiplier(has_chronic_depression)
    }
}

/// **M14I** § resolve the cosmetic decal id for a scar of a given wound
/// kind. Mirrors the M14G decal vocabulary so M45A can render the
/// resulting decals from the same atlas.
fn scar_decal_id(kind: WoundKind) -> String {
    use WoundKind::*;
    match kind {
        LacerationLight | LacerationModerate | LacerationSevere | Puncture | StabThrough => {
            "scar_suture_line".to_string()
        }
        GunshotEntry | GunshotExit | GunshotThrough | ShrapnelEmbedded | ShrapnelThrough => {
            "scar_bullet".to_string()
        }
        BruiseLight | BruiseHeavy => "scar_bruise_fade".to_string(),
        CrushLimb => "scar_crush".to_string(),
        Concussion => "scar_none".to_string(),
        FractureSimple | FractureCompound | FractureComminuted => "scar_fracture".to_string(),
        Dislocation | SprainStrain => "scar_joint".to_string(),
        Burn1st | Burn2nd | Burn3rd => "scar_burn".to_string(),
        Frostbite1st | Frostbite2nd | Frostbite3rd => "scar_frostbite".to_string(),
        AcidBurn | ChemicalBurn => "scar_chemical".to_string(),
        EyeInjury => "scar_eye_patch".to_string(),
        EarInjury => "scar_ear".to_string(),
        DentalDamage => "scar_dental".to_string(),
    }
}

#[allow(dead_code)]
const _LINT_ANCHORS: (FunctionalAggregate, TraitSet) =
    (FunctionalAggregate {
        max_blood_ml_lost: 0.0,
        zone_strength_loss: std::collections::BTreeMap::new(),
        aim_accuracy_loss: 0.0,
        move_speed_loss: 0.0,
        sensory_loss: std::collections::BTreeMap::new(),
        limp: false,
        phantom_panic_chance: 0.0,
    }, TraitSet { traits: Vec::new() });

#[allow(dead_code)]
const _SECONDS_PER_YEAR_LINK: f32 = SECONDS_PER_IN_GAME_YEAR;
#[allow(dead_code)]
const _RADIATION_THRESHOLD_LINK: f32 = RADIATION_CANCER_THRESHOLD;
#[allow(dead_code)]
const _RETIRE_LOCK_LINK: f32 = RETIREMENT_LOCK_DELTA_YEARS;
#[allow(dead_code)]
const _TIER_LINK: ProstheticTier = ProstheticTier::T1;
#[allow(dead_code)]
const _INST_LINK: Option<ProstheticInstance> = None;
