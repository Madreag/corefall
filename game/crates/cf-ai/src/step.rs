use cf_actor::{ActorState, Status, Vec2};
use cf_sim_core::Rng;

use crate::reactive_guard_params::seconds_to_ticks;
use crate::{
    AlarmInput, EnemyTickReport, FireRecord, GuardState, GuardStateTransition, GuardTickInputs,
    MissedShotReason, PerceptionRecord, PerceptionSignal, ReactiveGuard, StuckRecoveryRecord, Tactic,
    TacticRecord, TargetAcquiredRecord, TargetLostRecord,
};

/// One reactive-guard tick. Returns a structured report the engine turns into
/// recorder events; the engine is responsible for spawning the projectile and
/// applying damage when `fire.is_some()` AND `!fire.will_miss`.
#[must_use]
pub fn step(guard: &mut ReactiveGuard, inputs: GuardTickInputs<'_>, rng: &mut Rng) -> EnemyTickReport {
    let mut report = EnemyTickReport::default();

    let killed_by_cause = match inputs.last_damage_source {
        Some(id) => format!("killed_by_{id}"),
        None => "killed_by_unknown".to_string(),
    };
    if inputs.self_actor.status == Status::Dead || guard.state == GuardState::Dead {
        if guard.state != GuardState::Dead {
            let prev = guard.state;
            guard.state = GuardState::Dead;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Dead,
                cause: "dying_dwell_elapsed".to_string(),
            });
        }
        return report;
    }
    if inputs.self_actor.status == Status::Dying || guard.state == GuardState::Dying {
        if guard.state != GuardState::Dying {
            let prev = guard.state;
            guard.state = GuardState::Dying;
            guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Dying,
                cause: killed_by_cause.clone(),
            });
            return report;
        }
        if guard.dying_dwell_remaining_ticks > 0 {
            guard.dying_dwell_remaining_ticks -= 1;
            if guard.dying_dwell_remaining_ticks == 0 {
                let prev = guard.state;
                guard.state = GuardState::Dead;
                report.state_changes.push(GuardStateTransition {
                    previous: prev,
                    next: GuardState::Dead,
                    cause: "dying_dwell_elapsed".to_string(),
                });
            }
        }
        return report;
    }
    if inputs.self_actor.hp <= 0.0 && guard.state != GuardState::Dying {
        let prev = guard.state;
        guard.state = GuardState::Dying;
        guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
        report.state_changes.push(GuardStateTransition {
            previous: prev,
            next: GuardState::Dying,
            cause: killed_by_cause,
        });
        return report;
    }

    guard.heard_alarm_this_tick = None;

    if guard.params.hearing_radius > 0.0 && !inputs.alarms.is_empty() {
        let self_pos = inputs.self_actor.position;
        let mut closest: Option<(f32, &AlarmInput)> = None;
        for alarm in inputs.alarms {
            let dx = alarm.source_position[0] - self_pos.x;
            let dy = alarm.source_position[1] - self_pos.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let effective_radius = alarm.loudness_radius.min(guard.params.hearing_radius);
            if dist <= effective_radius && closest.as_ref().is_none_or(|(d, _)| dist < *d) {
                closest = Some((dist, alarm));
            }
        }
        if let Some((dist, alarm)) = closest {
            let confidence = if guard.params.hearing_radius > 0.0 {
                (1.0 - dist / guard.params.hearing_radius).clamp(0.0, 1.0)
            } else {
                0.0
            };
            guard.heard_alarm_this_tick = Some(alarm.source_position);
            guard.last_player_position = Some(alarm.source_position);
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            report.perception_signals.push(PerceptionSignal {
                kind: "hearing",
                source_actor: Some(alarm.source_actor),
                source_position: Some(alarm.source_position),
                confidence,
                tick: inputs.tick,
                alarm_event_id: alarm.alarm_event_id.clone(),
            });
            if guard.state == GuardState::Idle {
                guard.state = GuardState::Alert;
                report.state_changes.push(GuardStateTransition {
                    previous: GuardState::Idle,
                    next: GuardState::Alert,
                    cause: "heard_shot".to_string(),
                });
            }
        }
    }

    let prev_alert_dwell_remaining_ticks = guard.alert_dwell_remaining_ticks;
    let prev_burst_pause_remaining_ticks = guard.burst_pause_remaining_ticks;
    decrement(&mut guard.fire_cooldown_ticks, 1);
    decrement(&mut guard.aim_settle_remaining_ticks, 1);
    decrement(&mut guard.burst_pause_remaining_ticks, 1);
    decrement(&mut guard.alert_dwell_remaining_ticks, 1);

    if guard.reload_remaining_ticks > 0 {
        guard.reload_remaining_ticks -= 1;
        if guard.reload_remaining_ticks == 0 {
            guard.ammo_in_mag = guard.params.mag_capacity;
            guard.burst_shots_fired = 0;
            report.reload_completed = true;
        }
    }

    let perception = compute_perception(guard, &inputs);
    report.perception.clone_from(&perception);

    let player_visible_now = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_was_visible = guard
        .last_player_seen_tick
        .is_some_and(|t| t == inputs.tick.saturating_sub(1));
    if let Some(p) = &perception {
        if p.player_seen {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 1.0,
                tick: inputs.tick,
                alarm_event_id: None,
            });
        } else if player_was_visible {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight_lost",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 0.0,
                tick: inputs.tick,
                alarm_event_id: None,
            });
        }
    }

    if guard.params.memory_decay_ticks > 0 && !player_visible_now && guard.heard_alarm_this_tick.is_none() {
        if let Some(last_refresh) = guard.memory_last_refresh_tick {
            let age = inputs.tick.saturating_sub(last_refresh);
            if age >= u64::from(guard.params.memory_decay_ticks) && guard.last_player_position.is_some() {
                let pos = guard.last_player_position.take();
                guard.memory_last_refresh_tick = None;
                report.perception_signals.push(PerceptionSignal {
                    kind: "memory_decayed",
                    source_actor: inputs.player.map(|pl| pl.id.0),
                    source_position: pos,
                    confidence: 0.0,
                    tick: inputs.tick,
                    alarm_event_id: None,
                });
            }
        }
    }

    let hp_pct = if guard.max_hp > 0.0 {
        inputs.self_actor.hp / guard.max_hp
    } else {
        1.0
    };
    let should_retreat = hp_pct < guard.params.retreat_hp_pct;
    if should_retreat && guard.state != GuardState::Retreating {
        if matches!(guard.state, GuardState::Engaged | GuardState::Alert | GuardState::Idle) {
            let prev = guard.state;
            guard.state = GuardState::Retreating;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Retreating,
                cause: "low_hp".to_string(),
            });
        }
    } else if !should_retreat && hp_pct >= guard.params.recover_hp_pct && guard.state == GuardState::Retreating {
        let prev = guard.state;
        guard.state = if player_visible_now {
            GuardState::Engaged
        } else {
            GuardState::Alert
        };
        report.state_changes.push(GuardStateTransition {
            previous: prev,
            next: guard.state,
            cause: "hp_recovered".to_string(),
        });
    }
    if let Some(p) = &perception {
        if p.player_seen {
            guard.last_player_seen_tick = Some(inputs.tick);
            guard.last_player_position = p.last_seen_position;
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            let prev = guard.state;
            if guard.state != GuardState::Retreating {
                if prev == GuardState::Idle {
                    guard.state = GuardState::Alert;
                    guard.aim_settle_remaining_ticks = guard.params.aim_settle_ticks(inputs.tick_rate_hz);
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Idle,
                        next: GuardState::Alert,
                        cause: "saw_player_in_cone".to_string(),
                    });
                } else if prev == GuardState::Alert && guard.aim_settle_remaining_ticks == 0 {
                    guard.state = GuardState::Engaged;
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Alert,
                        next: GuardState::Engaged,
                        cause: "target_acquired".to_string(),
                    });
                    if let Some(player) = inputs.player {
                        report.target_acquired = Some(TargetAcquiredRecord {
                            target_actor: player.id.0,
                            via: "sight",
                        });
                    }
                } else if prev == GuardState::Retreating || prev == GuardState::Engaged {
                    guard.state = GuardState::Engaged;
                }
            }
        } else if prev_alert_dwell_remaining_ticks > 0 {
            let prev = guard.state;
            if guard.state == GuardState::Engaged {
                guard.state = GuardState::Alert;
                if prev != GuardState::Alert {
                    report.state_changes.push(GuardStateTransition {
                        previous: prev,
                        next: GuardState::Alert,
                        cause: "target_lost".to_string(),
                    });
                    if let Some(player) = inputs.player {
                        report.target_lost = Some(TargetLostRecord {
                            target_actor: player.id.0,
                            reason: "los_blocked",
                        });
                    }
                }
            }
        } else if guard.state != GuardState::Idle && guard.state != GuardState::Retreating {
            let prev = guard.state;
            guard.state = GuardState::Idle;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Idle,
                cause: "alert_expired".to_string(),
            });
        }
    }

    if matches!(
        guard.state,
        GuardState::Alert | GuardState::Engaged | GuardState::Retreating
    ) && !player_visible_now
    {
        guard.stuck_ticks = guard.stuck_ticks.saturating_add(1);
        if guard.stuck_ticks >= 60 && !guard.stuck_recovery_latched {
            guard.stuck_recovery_latched = true;
            report.stuck_recovery = Some(StuckRecoveryRecord {
                stuck_ticks: guard.stuck_ticks,
                blocker: "no_path",
                action: "wait_then_search",
                reason: "los_blocked_too_long",
            });
            guard.stuck_ticks = 0;
        }
    } else {
        guard.stuck_ticks = 0;
        guard.stuck_recovery_latched = false;
    }

    update_aim(guard, &perception, inputs.self_actor.position);

    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_distance = perception.as_ref().and_then(|p| p.distance);
    let scores = score_tactics(guard, player_visible, player_distance, prev_burst_pause_remaining_ticks);
    let (tactic, reason) = pick_tactic(guard, &scores, player_visible);
    guard.last_tactic = tactic;
    report.tactic_chosen = Some(TacticRecord {
        tactic,
        reason,
        score_attack: scores.attack,
        score_reload: scores.reload,
        score_hold: scores.hold,
        score_search: scores.search,
    });

    match tactic {
        Tactic::Reload => {
            if guard.reload_remaining_ticks == 0 && guard.ammo_in_mag < guard.params.mag_capacity {
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                guard.fire_cooldown_ticks = 0;
                guard.burst_pause_remaining_ticks = 0;
                guard.burst_shots_fired = 0;
                report.reload_started = true;
                if guard.state == GuardState::Engaged {
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Engaged,
                        next: GuardState::Engaged,
                        cause: "reloading".to_string(),
                    });
                }
            }
        }
        Tactic::Attack => {
            if let Some(fire) = try_fire(
                guard,
                inputs.self_actor,
                &perception,
                rng,
                inputs.tick_rate_hz,
                prev_burst_pause_remaining_ticks,
            ) {
                if fire.will_miss {
                    report.missed_shot_reason = Some(classify_miss_reason(fire.miss_roll));
                }
                report.fire = Some(fire);
            } else if guard.ammo_in_mag == 0 && guard.reload_remaining_ticks == 0 {
                report.dry_fire = true;
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                report.reload_started = true;
            }
        }
        Tactic::Hold | Tactic::Search | Tactic::AimSettle => {}
    }

    report
}

fn decrement(value: &mut u32, by: u32) {
    if *value >= by {
        *value -= by;
    } else {
        *value = 0;
    }
}

fn classify_miss_reason(miss_roll: f32) -> MissedShotReason {
    let r = miss_roll.clamp(0.0, 0.9999);
    if r < 0.25 {
        MissedShotReason::RecoilDeviation
    } else if r < 0.50 {
        MissedShotReason::TargetMoved
    } else if r < 0.75 {
        MissedShotReason::Occlusion
    } else {
        MissedShotReason::LuckyDodge
    }
}

fn compute_perception(guard: &ReactiveGuard, inputs: &GuardTickInputs<'_>) -> Option<PerceptionRecord> {
    let player = inputs.player?;
    if player.status.is_dead() {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: None,
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let dx = player.position.x - inputs.self_actor.position.x;
    let dy = player.position.y - inputs.self_actor.position.y;
    let distance = ((dx * dx) + (dy * dy)).sqrt();
    if distance > guard.params.sight_radius {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: Some(distance),
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let facing = if inputs.self_actor.aim != Vec2::ZERO {
        inputs.self_actor.aim.normalize_or_x()
    } else {
        Vec2::new(-1.0, 0.0)
    };
    let to_player = if distance > 1e-3 {
        Vec2::new(dx / distance, dy / distance)
    } else {
        return Some(PerceptionRecord {
            player_seen: true,
            distance: Some(distance),
            angle_degrees: Some(0.0),
            last_seen_position: Some([player.position.x, player.position.y]),
            state: guard.state,
        });
    };
    let dot = (facing.x * to_player.x + facing.y * to_player.y).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();
    let angle_deg = angle_rad * 180.0 / std::f32::consts::PI;
    let half_cone = (guard.params.sight_cone_degrees / 2.0).max(0.0);
    let visible = angle_deg <= half_cone;
    Some(PerceptionRecord {
        player_seen: visible,
        distance: Some(distance),
        angle_degrees: Some(angle_deg),
        last_seen_position: if visible {
            Some([player.position.x, player.position.y])
        } else {
            guard.last_player_position
        },
        state: guard.state,
    })
}

fn update_aim(guard: &mut ReactiveGuard, perception: &Option<PerceptionRecord>, self_pos: Vec2) {
    let target = match perception {
        Some(p) if p.player_seen => p.last_seen_position,
        _ => guard.last_player_position,
    };
    if let Some([tx, ty]) = target {
        let dx = tx - self_pos.x;
        let dy = ty - self_pos.y;
        let len = ((dx * dx) + (dy * dy)).sqrt();
        if len > 1e-3 {
            guard.aim = [dx / len, dy / len];
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TacticScores {
    attack: f32,
    reload: f32,
    hold: f32,
    search: f32,
}

fn score_tactics(
    guard: &ReactiveGuard,
    player_visible: bool,
    player_distance: Option<f32>,
    prev_burst_pause_remaining_ticks: u32,
) -> TacticScores {
    let mut scores = TacticScores::default();
    let ammo_ratio = if guard.params.mag_capacity == 0 {
        0.0
    } else {
        guard.ammo_in_mag as f32 / guard.params.mag_capacity as f32
    };
    let reloading = guard.reload_remaining_ticks > 0;

    if reloading {
        scores.reload = -1.0;
    } else if ammo_ratio <= 0.0 {
        scores.reload = 1.0;
    } else if ammo_ratio < 0.34 {
        scores.reload = 0.6;
    } else {
        scores.reload = 0.05;
    }

    if player_visible && guard.ammo_in_mag > 0 && guard.fire_cooldown_ticks == 0 && !reloading {
        let distance_pull = match player_distance {
            Some(d) => {
                let normalized = (1.0 - (d / guard.params.sight_radius)).clamp(0.0, 1.0);
                0.4 + 0.6 * normalized
            }
            None => 0.6,
        };
        let burst_penalty = if prev_burst_pause_remaining_ticks > 0 {
            -0.5
        } else {
            0.0
        };
        let aim_penalty = if guard.aim_settle_remaining_ticks > 0 {
            -0.25
        } else {
            0.0
        };
        scores.attack = (distance_pull + burst_penalty + aim_penalty).clamp(-1.0, 1.0);
    }

    scores.hold = 0.1;

    if guard.state == GuardState::Alert && !player_visible {
        scores.search = 0.3;
    }

    scores
}

fn pick_tactic(guard: &ReactiveGuard, scores: &TacticScores, player_visible: bool) -> (Tactic, &'static str) {
    if guard.reload_remaining_ticks > 0 {
        return (Tactic::Reload, "reload_in_progress");
    }
    if guard.aim_settle_remaining_ticks > 0 && player_visible {
        return (Tactic::AimSettle, "initial_acquisition");
    }
    if guard.ammo_in_mag == 0 {
        return (Tactic::Reload, "magazine_empty");
    }
    let mut best = (Tactic::Hold, scores.hold, "hold_default");
    if scores.attack > best.1 {
        best = (Tactic::Attack, scores.attack, "attack_target");
    }
    if scores.reload > best.1 {
        best = (Tactic::Reload, scores.reload, "low_ammo");
    }
    if scores.search > best.1 {
        best = (Tactic::Search, scores.search, "search_alerted");
    }
    (best.0, best.2)
}

fn try_fire(
    guard: &mut ReactiveGuard,
    self_actor: &ActorState,
    perception: &Option<PerceptionRecord>,
    rng: &mut Rng,
    tick_rate_hz: u32,
    prev_burst_pause_remaining_ticks: u32,
) -> Option<FireRecord> {
    if guard.aim_settle_remaining_ticks > 0 {
        return None;
    }
    if guard.fire_cooldown_ticks > 0 {
        return None;
    }
    if prev_burst_pause_remaining_ticks > 0 {
        return None;
    }
    if guard.ammo_in_mag == 0 {
        return None;
    }
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    if !player_visible {
        return None;
    }
    let aim_unit = Vec2::new(guard.aim[0], guard.aim[1]).normalize_or_x();
    let muzzle = [
        self_actor.position.x + aim_unit.x * guard.params.muzzle_forward_offset,
        self_actor.position.y + guard.params.muzzle_vertical_offset + aim_unit.y * guard.params.muzzle_forward_offset,
    ];
    let raw = rng.next_u64();
    let unit_roll = ((raw >> 11) as f64 / ((1u64 << 53) as f64)) as f32;
    let miss_threshold = guard.params.miss_chance.clamp(0.0, 1.0);
    let will_miss = miss_threshold >= 1.0 || unit_roll < miss_threshold;
    let velocity = if will_miss {
        let drift: f32 = 0.18
            * if guard.burst_shots_fired.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
        let cos = drift.cos();
        let sin = drift.sin();
        let dx = aim_unit.x * cos - aim_unit.y * sin;
        let dy = aim_unit.x * sin + aim_unit.y * cos;
        [dx * guard.params.projectile_speed, dy * guard.params.projectile_speed]
    } else {
        [
            aim_unit.x * guard.params.projectile_speed,
            aim_unit.y * guard.params.projectile_speed,
        ]
    };
    guard.ammo_in_mag = guard.ammo_in_mag.saturating_sub(1);
    guard.burst_shots_fired += 1;
    guard.fire_cooldown_ticks = seconds_to_ticks(0.20, tick_rate_hz);
    if guard.burst_shots_fired >= guard.params.burst_shots {
        guard.burst_pause_remaining_ticks = guard.params.burst_pause_ticks(tick_rate_hz);
        guard.burst_shots_fired = 0;
    }
    let lifetime_ticks = guard.params.projectile_lifetime_ticks(tick_rate_hz);
    Some(FireRecord {
        muzzle_origin: muzzle,
        velocity,
        aim: [aim_unit.x, aim_unit.y],
        damage: guard.params.damage_per_hit,
        miss_roll: unit_roll,
        miss_threshold,
        will_miss,
        lifetime_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReactiveGuardParams;
    use cf_actor::{ActorId, Inventory, InventoryItem, ItemSlot};

    fn guard_actor() -> ActorState {
        let inv = Inventory {
            items: vec![InventoryItem::Empty; 4],
            selected: ItemSlot(0),
        };
        let mut a = ActorState::player(ActorId(2), "red", Vec2::new(900.0, 32.0), 80.0, inv);
        a.controllable = false;
        a.aim = Vec2::new(-1.0, 0.0);
        a
    }

    fn player_actor(x: f32, y: f32) -> ActorState {
        ActorState::player(ActorId(1), "blue", Vec2::new(x, y), 100.0, Inventory::default())
    }

    fn rng() -> Rng {
        Rng::from_seed(13)
    }

    fn tick_inputs<'a>(tick: u64, guard_a: &'a ActorState, player: Option<&'a ActorState>) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
            alarms: &[],
            last_damage_source: player.map(|p| p.id.0),
        }
    }

    fn tick_inputs_with_alarms<'a>(
        tick: u64,
        guard_a: &'a ActorState,
        player: Option<&'a ActorState>,
        alarms: &'a [AlarmInput],
    ) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
            alarms,
            last_damage_source: player.map(|p| p.id.0),
        }
    }

    #[test]
    fn idle_when_player_not_present() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
        assert!(report.fire.is_none());
        assert!(report.tactic_chosen.is_some());
    }

    #[test]
    fn engages_when_player_in_cone() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        assert!(!report.state_changes.is_empty());
        let perception = report.perception.unwrap();
        assert!(perception.player_seen);
        assert!(perception.distance.unwrap() > 0.0);
        let settle_ticks = guard.aim_settle_remaining_ticks;
        for t in 2..=(2 + settle_ticks as u64) {
            let _ = step(&mut guard, tick_inputs(t, &actor, Some(&player)), &mut rng);
        }
        assert_eq!(guard.state, GuardState::Engaged);
    }

    #[test]
    fn does_not_fire_during_aim_settle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(report.fire.is_none());
        assert!(guard.aim_settle_remaining_ticks > 0);
    }

    #[test]
    fn fires_after_aim_settles() {
        let mut params = ReactiveGuardParams::default();
        params.miss_chance = 0.0;
        params.aim_settle_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);
        let mut shots = 0;
        for tick in 1..=120 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.fire.is_some() {
                shots += 1;
            }
        }
        assert!(shots > 0, "guard must fire at least once after aim settle");
    }

    #[test]
    fn out_of_cone_does_not_engage() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.aim = Vec2::new(1.0, 0.0);
        let player = player_actor(0.0, 32.0);
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        let perception = report.perception.unwrap();
        assert!(!perception.player_seen);
        assert_ne!(guard.state, GuardState::Engaged);
    }

    #[test]
    fn dead_actor_locks_state_to_dead() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.hp = 0.0;
        actor.status = Status::Dead;
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Dead);
        assert!(!report.state_changes.is_empty());
    }

    #[test]
    fn deterministic_under_same_seed() {
        fn play_500_ticks(seed: u64) -> Vec<bool> {
            let mut params = ReactiveGuardParams::default();
            params.aim_settle_seconds = 0.05;
            let mut guard = ReactiveGuard::new(ActorId(2), params);
            let actor = guard_actor();
            let player = player_actor(700.0, 32.0);
            let mut rng = Rng::from_seed(seed);
            let mut fires = Vec::new();
            for tick in 1..=500 {
                let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
                fires.push(report.fire.is_some());
            }
            fires
        }
        let a = play_500_ticks(13);
        let b = play_500_ticks(13);
        assert_eq!(a, b, "same seed must produce identical fire pattern");
    }

    #[test]
    fn out_of_ammo_triggers_reload() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.05;
        params.miss_chance = 0.0;
        params.mag_capacity = 2;
        params.burst_shots = 2;
        params.burst_pause_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let mut reload_started = false;
        for tick in 1..=300 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.reload_started {
                reload_started = true;
                break;
            }
        }
        assert!(reload_started);
    }

    #[test]
    fn reset_returns_full_mag_and_idle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        guard.ammo_in_mag = 0;
        guard.state = GuardState::Engaged;
        guard.reload_remaining_ticks = 30;
        guard.reset();
        assert_eq!(guard.state, GuardState::Idle);
        assert_eq!(guard.ammo_in_mag, ReactiveGuardParams::default().mag_capacity);
        assert_eq!(guard.reload_remaining_ticks, 0);
    }

    #[test]
    fn alert_dwell_lasts_full_configured_duration_after_player_lost() {
        let mut params = ReactiveGuardParams::default();
        params.alert_dwell_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player_visible = player_actor(700.0, 32.0);
        let player_lost = player_actor(2000.0, 32.0);
        let mut rng = rng();

        let _ = step(&mut guard, tick_inputs(1, &actor, Some(&player_visible)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        assert_eq!(guard.alert_dwell_remaining_ticks, 3);

        let _ = step(&mut guard, tick_inputs(2, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        let _ = step(&mut guard, tick_inputs(3, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        let _ = step(&mut guard, tick_inputs(4, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        let _ = step(&mut guard, tick_inputs(5, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
    }

    #[test]
    fn burst_pause_blocks_fire_for_full_configured_duration() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.0;
        params.miss_chance = 0.0;
        params.mag_capacity = 10;
        params.burst_shots = 1;
        params.burst_pause_seconds = 0.30;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);

        let r1 = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(r1.fire.is_some(), "tick 1: zero aim_settle, must fire");
        assert_eq!(guard.burst_pause_remaining_ticks, 18);

        for tick in 2..=19 {
            let r = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            assert!(
                r.fire.is_none(),
                "tick {tick}: burst_pause should block fire for the full 18-tick configured duration"
            );
        }

        let r20 = step(&mut guard, tick_inputs(20, &actor, Some(&player)), &mut rng);
        assert!(
            r20.fire.is_some(),
            "tick 20: pause + cooldown expired, fire should resume"
        );
    }

    #[test]
    fn ai_h_01_sentry_hears_threat_without_los() {
        let mut params = ReactiveGuardParams::default();
        params.hearing_radius = 480.0;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let mut rng = rng();
        let alarms = [AlarmInput {
            source_actor: 1,
            source_position: [actor.position.x + 200.0, actor.position.y],
            loudness_radius: 480.0,
            alarm_event_id: None,
        }];
        let report = step(&mut guard, tick_inputs_with_alarms(1, &actor, None, &alarms), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        let transitioned = report
            .state_changes
            .first()
            .cloned()
            .expect("state must change on heard_shot");
        assert_eq!(transitioned.previous, GuardState::Idle);
        assert_eq!(transitioned.next, GuardState::Alert);
        assert_eq!(transitioned.cause, "heard_shot");
        let hearing = report
            .perception_signals
            .iter()
            .find(|s| s.kind == "hearing")
            .expect("hearing perception_signal must fire");
        assert_eq!(hearing.source_actor, Some(1));
        assert!(hearing.confidence > 0.0 && hearing.confidence <= 1.0);
    }

    #[test]
    fn classify_miss_reason_buckets_are_stable() {
        assert_eq!(classify_miss_reason(0.0), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.24), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.26), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.49), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.51), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.74), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.76), MissedShotReason::LuckyDodge);
        assert_eq!(classify_miss_reason(0.99), MissedShotReason::LuckyDodge);
    }

    #[test]
    fn low_hp_transitions_to_retreating() {
        let mut params = ReactiveGuardParams::default();
        params.retreat_hp_pct = 0.5;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        guard.max_hp = 100.0;
        let mut actor = guard_actor();
        actor.hp = 40.0;
        let player = player_actor(80.0, 32.0);
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Retreating);
        let transitioned = report.state_changes.first().cloned().expect("hp gate must transition");
        assert_eq!(transitioned.cause, "low_hp");
        assert_eq!(transitioned.next, GuardState::Retreating);
    }
}
