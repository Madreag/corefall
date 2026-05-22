//! **M14A** § "Jetpack Physics — full algorithm".
//!
//! Algorithmic port of CCCP `Entities/AEJetpack.cpp` (full 232 lines). The
//! Rust is original; the calibration constants + tuning shape match CC.
//!
//! Two jetpack types:
//!   - **Standard**: throttle-controlled; fuel drains while emit is on.
//!   - **JumpPack**: one-shot full discharge; refills 100% before relight.
//!
//! Throttle-for-weight: when `adjusts_throttle_for_weight`, fuel drain scales
//! linearly with `actor_total_mass / baseline_mass`. A heavy soldier burns
//! through fuel ~2.5× faster.
//!
//! Atmospheric pressure efficiency: jet thrust output is scaled by an
//! efficiency curve over `local_pressure_kpa`. Vacuum is more efficient
//! (×1.5); Venus-like atmospheres are less efficient (×0.5).

use serde::{Deserialize, Serialize};

/// CCCP `AEJetpack::JetpackType` enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JetpackType {
    /// Standard throttle-controlled jet (CCCP default).
    Standard = 0,
    /// One-shot full-discharge jet; refills 100% before relight.
    JumpPack = 1,
}

impl Default for JetpackType {
    fn default() -> Self {
        JetpackType::Standard
    }
}

impl JetpackType {
    pub fn as_str(self) -> &'static str {
        match self {
            JetpackType::Standard => "standard",
            JetpackType::JumpPack => "jump_pack",
        }
    }

    pub fn parse(s: &str) -> Option<JetpackType> {
        match s {
            "standard" => Some(JetpackType::Standard),
            "jump_pack" | "jumppack" => Some(JetpackType::JumpPack),
            _ => None,
        }
    }
}

/// runtime + spec.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Jetpack {
    /// Stable id (from RON content).
    pub id: String,
    pub jetpack_type: JetpackType,
    /// Total fuel capacity in ms of full thrust.
    pub jet_time_total_ms: u32,
    /// Current fuel reservoir (ms).
    pub jet_time_left_ms: u32,
    /// Replenish rate (ms-of-fuel per ms-of-real-time) when not emitting.
    pub jet_replenish_rate: f32,
    /// Minimum fuel ratio required to begin a new burn (0..1).
    pub minimum_fuel_ratio: f32,
    /// Aim → thrust angle range (0 = locked up; 1 = full ±π/2).
    pub jet_angle_range: f32,
    /// If `false`, the thrust direction locks on activation (CCCP
    /// `m_CanAdjustAngleWhileFiring=false`).
    pub can_adjust_angle_while_firing: bool,
    /// When true, fuel drain auto-scales with `total_mass / baseline_mass`.
    pub adjusts_throttle_for_weight: bool,
    /// Base thrust in newtons (peak).
    pub base_thrust_n: f32,
    /// Burst thrust multiplier (initial impulse on activation).
    pub burst_thrust_multiplier: f32,
    /// Dry mass in kg (subtracted from total when backpack lost).
    pub dry_mass_kg: f32,
    /// kg of fuel per ms of fuel (12 kg full tank / 4500 ms = ~0.00267).
    pub fuel_density_kg_per_ms: f32,
    /// Which body zone the jet mounts on ("backpack" by default).
    pub bound_zone: String,
    /// Local emitter offset from chassis origin.
    pub emitter_offset: [f32; 2],
    /// `true` while the jet is currently emitting (visible exhaust).
    pub is_emitting: bool,
    /// Last computed throttle (0..1) — surfaces to HUD.
    pub throttle: f32,
    /// Last computed emit angle (radians) — surfaces to renderer.
    pub emit_angle: f32,
    /// Locked emit direction (set on activation if `can_adjust_angle_while_firing=false`).
    pub locked_emit_angle: Option<f32>,
    /// `true` once a JumpPack has fully discharged and is refilling.
    pub jumppack_refilling: bool,
}

impl Default for Jetpack {
    fn default() -> Self {
        Self::standard_powered_armor()
    }
}

impl Jetpack {
    pub fn standard_powered_armor() -> Self {
        Self {
            id: "standard_powered_armor".to_string(),
            jetpack_type: JetpackType::Standard,
            jet_time_total_ms: 4500,
            jet_time_left_ms: 4500,
            jet_replenish_rate: 1.0,
            minimum_fuel_ratio: 0.25,
            jet_angle_range: 0.6,
            can_adjust_angle_while_firing: true,
            adjusts_throttle_for_weight: true,
            base_thrust_n: 3500.0,
            burst_thrust_multiplier: 2.0,
            dry_mass_kg: 5.0,
            fuel_density_kg_per_ms: 12.0 / 4500.0,
            bound_zone: "backpack".to_string(),
            emitter_offset: [-8.0, 18.0],
            is_emitting: false,
            throttle: 0.0,
            emit_angle: -std::f32::consts::FRAC_PI_2,
            locked_emit_angle: None,
            jumppack_refilling: false,
        }
    }

    pub fn jump_pack_light_mech() -> Self {
        Self {
            id: "jump_pack_light_mech".to_string(),
            jetpack_type: JetpackType::JumpPack,
            jet_time_total_ms: 1500,
            jet_time_left_ms: 1500,
            jet_replenish_rate: 0.3,
            minimum_fuel_ratio: 1.0,
            jet_angle_range: 0.6,
            can_adjust_angle_while_firing: false,
            adjusts_throttle_for_weight: true,
            base_thrust_n: 30_000.0,
            burst_thrust_multiplier: 3.0,
            dry_mass_kg: 20.0,
            fuel_density_kg_per_ms: 40.0 / 1500.0,
            bound_zone: "backpack".to_string(),
            emitter_offset: [-12.0, 24.0],
            is_emitting: false,
            throttle: 0.0,
            emit_angle: -std::f32::consts::FRAC_PI_2,
            locked_emit_angle: None,
            jumppack_refilling: false,
        }
    }

    pub fn standard_heavy_trooper() -> Self {
        Self {
            id: "standard_heavy_trooper".to_string(),
            jetpack_type: JetpackType::Standard,
            jet_time_total_ms: 3000,
            jet_time_left_ms: 3000,
            jet_replenish_rate: 0.7,
            minimum_fuel_ratio: 0.30,
            jet_angle_range: 0.6,
            can_adjust_angle_while_firing: true,
            adjusts_throttle_for_weight: true,
            base_thrust_n: 5500.0,
            burst_thrust_multiplier: 2.0,
            dry_mass_kg: 8.0,
            fuel_density_kg_per_ms: 18.0 / 3000.0,
            bound_zone: "backpack".to_string(),
            emitter_offset: [-10.0, 22.0],
            is_emitting: false,
            throttle: 0.0,
            emit_angle: -std::f32::consts::FRAC_PI_2,
            locked_emit_angle: None,
            jumppack_refilling: false,
        }
    }

    /// Current fuel ratio (0..1).
    pub fn fuel_ratio(&self) -> f32 {
        if self.jet_time_total_ms == 0 {
            return 0.0;
        }
        (self.jet_time_left_ms as f32 / self.jet_time_total_ms as f32).clamp(0.0, 1.0)
    }

    /// Current fuel mass in kg.
    pub fn fuel_mass_kg(&self) -> f32 {
        self.fuel_density_kg_per_ms * self.jet_time_left_ms as f32
    }

    /// Reject activation reason when activation is not allowed.
    pub fn check_activation_reject(&self, actor_jump_intent: bool) -> Option<&'static str> {
        if !actor_jump_intent {
            return Some("not_requested");
        }
        if self.jet_time_left_ms == 0 {
            return Some("jet_empty");
        }
        if self.jumppack_refilling && self.fuel_ratio() < 1.0 {
            return Some("jumppack_refilling");
        }
        if !self.is_emitting && self.fuel_ratio() < self.minimum_fuel_ratio {
            return Some("jet_below_minimum_fuel_ratio");
        }
        None
    }

    pub fn burst(&mut self, fuel_use_multiplier: f32, dt_ms: u32, aim_angle: f32, h_flipped: bool) -> f32 {
        self.is_emitting = true;
        let burst_size = self.jet_time_total_ms.max(2) as f32;
        let fuel_usage =
            (dt_ms as f32 * burst_size * self.burst_thrust_multiplier * 0.001 * fuel_use_multiplier) + dt_ms as f32 * fuel_use_multiplier;
        self.spend_fuel(fuel_usage);
        if !self.can_adjust_angle_while_firing {
            self.locked_emit_angle = Some(self.compute_emit_angle(aim_angle, h_flipped));
        }
        self.emit_angle = self.locked_emit_angle.unwrap_or_else(|| self.compute_emit_angle(aim_angle, h_flipped));
        self.base_thrust_n * self.burst_thrust_multiplier
    }

    pub fn thrust(&mut self, fuel_use_multiplier: f32, dt_ms: u32, aim_angle: f32, h_flipped: bool) -> f32 {
        self.is_emitting = true;
        let fuel_usage = dt_ms as f32 * fuel_use_multiplier;
        self.spend_fuel(fuel_usage);
        let angle = if self.can_adjust_angle_while_firing || self.locked_emit_angle.is_none() {
            self.compute_emit_angle(aim_angle, h_flipped)
        } else {
            self.locked_emit_angle.unwrap_or(self.emit_angle)
        };
        self.emit_angle = angle;
        self.base_thrust_n
    }

    pub fn recharge(&mut self, dt_ms: u32) {
        self.is_emitting = false;
        let restore = (dt_ms as f32 * self.jet_replenish_rate) as i64;
        let next = self.jet_time_left_ms as i64 + restore;
        self.jet_time_left_ms = next.clamp(0, self.jet_time_total_ms as i64) as u32;
        if self.jumppack_refilling && self.fuel_ratio() >= 1.0 {
            self.jumppack_refilling = false;
        }
        self.locked_emit_angle = None;
    }

    fn spend_fuel(&mut self, fuel_ms: f32) {
        let n = fuel_ms.round() as i64;
        let left = self.jet_time_left_ms as i64 - n;
        self.jet_time_left_ms = left.max(0) as u32;
        if matches!(self.jetpack_type, JetpackType::JumpPack) && self.jet_time_left_ms == 0 {
            self.jumppack_refilling = true;
        }
    }

    fn compute_emit_angle(&self, aim_angle: f32, h_flipped: bool) -> f32 {
        let max_angle = std::f32::consts::FRAC_PI_2 * self.jet_angle_range;
        let mut a = aim_angle * self.jet_angle_range;
        if a > max_angle {
            a = max_angle;
        } else if a < -max_angle {
            a = -max_angle;
        }
        let flip = if h_flipped { -1.0 } else { 1.0 };
        a * flip - std::f32::consts::FRAC_PI_2
    }
}

/// vacuum ×1.5, Earth ×1.0, Venus ×0.5.
pub fn jet_pressure_efficiency(local_pressure_kpa: f32) -> f32 {
    const VACUUM_KPA: f32 = 1.0;
    const EARTH_KPA: f32 = 101.0;
    const VENUS_KPA: f32 = 239.0;
    const VACUUM_EFF: f32 = 1.5;
    const EARTH_EFF: f32 = 1.0;
    const VENUS_EFF: f32 = 0.5;
    if local_pressure_kpa < VACUUM_KPA {
        VACUUM_EFF
    } else if local_pressure_kpa < EARTH_KPA {
        let t = (local_pressure_kpa - VACUUM_KPA) / (EARTH_KPA - VACUUM_KPA);
        VACUUM_EFF + (EARTH_EFF - VACUUM_EFF) * t
    } else if local_pressure_kpa < VENUS_KPA {
        let t = (local_pressure_kpa - EARTH_KPA) / (VENUS_KPA - EARTH_KPA);
        EARTH_EFF + (VENUS_EFF - EARTH_EFF) * t
    } else {
        VENUS_EFF
    }
}

///
/// Returns the thrust vector to apply to the actor body this tick (N).
///
/// Pure: takes context, mutates `Jetpack`, returns vector. No clock reads.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn jetpack_tick(
    jet: &mut Jetpack,
    actor_jump_intent: bool,
    actor_jumpstart_intent: bool,
    actor_inactive: bool,
    total_mass_kg: f32,
    baseline_mass_kg: f32,
    aim_angle: f32,
    h_flipped: bool,
    local_pressure_kpa: f32,
    dt_ms: u32,
) -> JetpackTickOutcome {
    if actor_inactive {
        jet.recharge(dt_ms);
        return JetpackTickOutcome::idle();
    }

    let mut fuel_use_multiplier = 1.0;
    if jet.adjusts_throttle_for_weight {
        let ratio = (total_mass_kg / baseline_mass_kg.max(1.0)).clamp(0.5, 4.0);
        jet.throttle = ratio.min(1.0);
        fuel_use_multiplier = ratio;
    } else {
        let jet_ratio = jet.fuel_ratio();
        jet.throttle = (jet_ratio * 2.0 - 1.0).clamp(-1.0, 1.0);
    }

    let was_emitting = jet.is_emitting;
    let min_fuel_to_begin = (250.0 * fuel_use_multiplier) as u32;
    let fuel_ok = jet.jet_time_left_ms > min_fuel_to_begin || was_emitting || jet.fuel_ratio() >= 1.0;

    if !fuel_ok {
        jet.recharge(dt_ms);
        return JetpackTickOutcome::idle();
    }

    let mut thrust_n = 0.0;
    let mut event_kind = JetpackEvent::None;
    match jet.jetpack_type {
        JetpackType::Standard => {
            if actor_jumpstart_intent && jet.jet_time_left_ms > 0 {
                thrust_n = jet.burst(fuel_use_multiplier, dt_ms, aim_angle, h_flipped);
                event_kind = JetpackEvent::Fired;
            } else if actor_jump_intent
                && jet.jet_time_left_ms > 0
                && (jet.fuel_ratio() >= jet.minimum_fuel_ratio || was_emitting)
            {
                thrust_n = jet.thrust(fuel_use_multiplier, dt_ms, aim_angle, h_flipped);
                if jet.jet_time_left_ms == 0 {
                    event_kind = JetpackEvent::Exhausted;
                }
            } else {
                jet.recharge(dt_ms);
                if was_emitting && !jet.is_emitting && jet.jet_time_left_ms == 0 {
                    event_kind = JetpackEvent::Exhausted;
                } else if was_emitting && !jet.is_emitting {
                    event_kind = JetpackEvent::Relit;
                }
            }
        }
        JetpackType::JumpPack => {
            if was_emitting && jet.jet_time_left_ms > 0 {
                thrust_n = jet.thrust(fuel_use_multiplier, dt_ms, aim_angle, h_flipped);
                if jet.jet_time_left_ms == 0 {
                    event_kind = JetpackEvent::Exhausted;
                }
            } else if actor_jumpstart_intent && jet.fuel_ratio() >= jet.minimum_fuel_ratio {
                thrust_n = jet.burst(fuel_use_multiplier, dt_ms, aim_angle, h_flipped);
                event_kind = JetpackEvent::Fired;
            } else {
                jet.recharge(dt_ms);
            }
        }
    }

    // Atmospheric efficiency scales realized thrust.
    let efficiency = jet_pressure_efficiency(local_pressure_kpa);
    let realized = thrust_n * efficiency;

    // Emit angle is the direction the *exhaust* points; the actor accelerates
    // in the *opposite* direction (Newton's third law). Negate to get the
    // thrust vector applied to the actor.
    let angle = jet.emit_angle;
    let thrust_vec = [-angle.cos() * realized, -angle.sin() * realized];

    JetpackTickOutcome {
        thrust_n: realized,
        thrust_vec,
        event: event_kind,
        was_emitting_before: was_emitting,
        is_emitting_after: jet.is_emitting,
        efficiency,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JetpackTickOutcome {
    pub thrust_n: f32,
    pub thrust_vec: [f32; 2],
    pub event: JetpackEvent,
    pub was_emitting_before: bool,
    pub is_emitting_after: bool,
    pub efficiency: f32,
}

impl JetpackTickOutcome {
    pub fn idle() -> Self {
        Self {
            thrust_n: 0.0,
            thrust_vec: [0.0, 0.0],
            event: JetpackEvent::None,
            was_emitting_before: false,
            is_emitting_after: false,
            efficiency: 1.0,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JetpackEvent {
    None = 0,
    Fired = 1,
    Exhausted = 2,
    Relit = 3,
}

impl JetpackEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            JetpackEvent::None => "none",
            JetpackEvent::Fired => "fired",
            JetpackEvent::Exhausted => "exhausted",
            JetpackEvent::Relit => "relit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_powered_armor_defaults() {
        let jet = Jetpack::standard_powered_armor();
        assert_eq!(jet.jetpack_type, JetpackType::Standard);
        assert_eq!(jet.jet_time_total_ms, 4500);
        assert!((jet.minimum_fuel_ratio - 0.25).abs() < 1e-6);
    }

    #[test]
    fn jump_pack_full_discharge() {
        let mut jet = Jetpack::jump_pack_light_mech();
        // Burst first (jet_press_edge=true), then sustain.
        let mut total_thrust = 0.0;
        for i in 0..200 {
            let press_edge = i == 0;
            let outcome = jetpack_tick(&mut jet, true, press_edge, false, 1900.0, 80.0, 0.0, false, 101.0, 16);
            total_thrust += outcome.thrust_n.abs();
            if jet.jet_time_left_ms == 0 {
                break;
            }
        }
        assert!(jet.jumppack_refilling);
        assert_eq!(jet.jet_time_left_ms, 0);
        assert!(total_thrust > 0.0);
    }

    #[test]
    fn minimum_fuel_ratio_blocks_activation() {
        let mut jet = Jetpack::standard_powered_armor();
        jet.jet_time_left_ms = (jet.jet_time_total_ms as f32 * 0.2) as u32; // 20% < 25% min
        assert_eq!(
            jet.check_activation_reject(true),
            Some("jet_below_minimum_fuel_ratio")
        );
    }

    #[test]
    fn pressure_efficiency_in_vacuum_higher() {
        let vac = jet_pressure_efficiency(0.01);
        let earth = jet_pressure_efficiency(101.0);
        let venus = jet_pressure_efficiency(239.0);
        assert!((vac - 1.5).abs() < 1e-3);
        assert!((earth - 1.0).abs() < 1e-3);
        assert!((venus - 0.5).abs() < 1e-3);
    }

    #[test]
    fn throttle_for_weight_increases_fuel_drain() {
        let mut light = Jetpack::standard_powered_armor();
        let mut heavy = Jetpack::standard_powered_armor();
        // Light actor (80 kg) and heavy actor (220 kg) burning for 200 ms.
        for _ in 0..12 {
            jetpack_tick(&mut light, true, false, false, 80.0, 80.0, 0.0, false, 101.0, 16);
            jetpack_tick(&mut heavy, true, false, false, 220.0, 80.0, 0.0, false, 101.0, 16);
        }
        let light_used = 4500_u32.saturating_sub(light.jet_time_left_ms);
        let heavy_used = 4500_u32.saturating_sub(heavy.jet_time_left_ms);
        assert!(
            heavy_used > light_used * 2,
            "heavy_used={heavy_used}, light_used={light_used}"
        );
    }

    #[test]
    fn fuel_mass_decreases_with_burn() {
        let mut jet = Jetpack::standard_powered_armor();
        let initial = jet.fuel_mass_kg();
        for _ in 0..63 {
            jetpack_tick(&mut jet, true, false, false, 200.0, 80.0, 0.0, false, 101.0, 16);
        }
        let after = jet.fuel_mass_kg();
        assert!(after < initial - 1.0, "no measurable burn: {initial} → {after}");
    }

    #[test]
    fn locked_angle_persists_while_firing() {
        let mut jet = Jetpack::jump_pack_light_mech();
        // Burst at aim=0.5
        jetpack_tick(&mut jet, true, true, false, 200.0, 80.0, 0.5, false, 101.0, 16);
        let locked = jet.locked_emit_angle.expect("angle should lock");
        // Change aim — locked angle should not move.
        jetpack_tick(&mut jet, true, false, false, 200.0, 80.0, -1.0, false, 101.0, 16);
        assert_eq!(jet.locked_emit_angle, Some(locked));
    }
}
