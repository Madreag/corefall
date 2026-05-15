//! M7-A: first-class behavior reach/apply windows.
//!
//! These constants make the auto-triage + auto-repair contracts machine-
//! verifiable. The Gherkin scenarios in M7.md mandate exact second budgets;
//! cf-control's emission sites re-export these so engine code and AI code
//! agree on the same numbers.

/// **M7-A**: maximum seconds (wall-clock at 60 Hz default) for a Medic in
/// `FullAuto` mode to reach a DYING / Downed squadmate before the scenario
/// considers the auto-triage failed. Spec § Auto-triage Gherkin.
pub const MEDIC_AUTO_TRIAGE_REACH_SECONDS: f32 = 6.0;

/// **M7-A**: maximum seconds (wall-clock) from a squadmate entering DYING to
/// the Medic applying stabilization (Bleed timer pauses; HP regen begins).
/// Spec § Auto-triage Gherkin.
pub const MEDIC_AUTO_TRIAGE_APPLY_SECONDS: f32 = 8.0;

/// **M7-A**: maximum seconds for an Engineer in `FullAuto` to reach an ally
/// chassis module that just entered `Critical`/`Degraded`/`Failed` state.
/// Spec § Auto-repair Gherkin.
pub const ENGINEER_AUTO_REPAIR_REACH_SECONDS: f32 = 6.0;

/// **M7-A**: maximum seconds for the Engineer to apply the first repair
/// tick after the module's state transition. Spec § Auto-repair Gherkin.
pub const ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS: f32 = 8.0;

/// **M7-A**: chatter cooldown between consecutive chatter events from the
/// same bot (seconds). Spec § Chatter scaffold cooldown table.
pub const CHATTER_COOLDOWN_SECONDS: f32 = 4.0;

/// **M7-A**: squad-comm relay delay between an alarmed bot and its
/// squadmates (seconds). Spec § Squad communication.
pub const SQUAD_COMM_RELAY_DELAY_SECONDS: f32 = 0.5;

/// **M7-A**: idle pause range at each patrol waypoint (seconds).
pub const PATROL_IDLE_MIN_SECONDS: f32 = 5.0;
pub const PATROL_IDLE_MAX_SECONDS: f32 = 10.0;

/// **M8A**: per-tick AI sim p99 latency budget (milliseconds). Retuned from
/// the original 2.0 ms at M8A to cover the M7-shipped 5-layer thinking
/// stack (Reactive + Utility + Behavior Tree + HTN + LLM-prior) +
/// 16 KB BotMemory per bot + 22-task PriorityTable + deterministic
/// reason-label structured strings. Owned by M8A's
/// `docs/plan/spec/perf-budget-contract.md`; downstream milestones must
/// not re-tune.
pub const AI_SIM_P99_BUDGET_MS: f32 = 4.0;

/// **M8A**: per-bot AI tick effective parallel slice (microseconds) at the
/// 50-bot stress (16 cores, par_iter). Derived from
/// `AI_SIM_P99_BUDGET_MS * 1000.0 / 50.0` with rounding for the 16-core
/// effective parallel budget; the exact figure published by the spec is
/// 80 µs. Used by archetype tuning + utility scoring complexity audits.
pub const PER_BOT_AI_TICK_BUDGET_US: u32 = 80;

/// Convert a seconds budget to ticks at the configured tick rate. Identical
/// rounding to `seconds_to_ticks` in lib.rs (sub-tick durations round up to
/// 1 so a configured timer can never silently disappear).
#[inline]
pub fn seconds_to_ticks_for(seconds: f32, tick_rate_hz: u32) -> u32 {
    let rate = tick_rate_hz.max(1);
    let clamped = seconds.max(0.0);
    if clamped == 0.0 {
        return 0;
    }
    let ticks = (f64::from(clamped) * f64::from(rate)).round();
    if ticks < 1.0 {
        1
    } else if ticks > f64::from(u32::MAX) {
        u32::MAX
    } else {
        ticks as u32
    }
}
