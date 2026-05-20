//! **M14A** § "Quick Action UX — sub-100ms action selection".
//!
//! 8-slot bar (keys 1-8) + tap-Q quick-toggle + hold-Q radial with sim
//! time-slow (25%). The radial opens in ≤ 80 ms and the whole pick-and-confirm
//! cycle is ~10× faster than CC's pie menu.

use serde::{Deserialize, Serialize};

/// Constants from `specs/active/M14A.md` § Constants.
pub const QUICK_ACTION_OPEN_MS: u32 = 80;
pub const QUICK_ACTION_TIME_SLOW: f32 = 0.25;
pub const QUICK_ACTION_TIME_SLOW_REDUCE_MOTION: f32 = 0.50;
pub const QUICK_ACTION_TAP_MAX_MS: u32 = 120;
pub const QUICK_ACTION_DEADZONE_PX: f32 = 12.0;
pub const QUICK_ACTION_SLOT_COUNT: usize = 8;

/// What a slot holds. Empty slots reject invocation. Item-id is opaque and
/// resolved by `cf-equipment` when the slot is invoked.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuickActionSlotKind {
    Empty = 0,
    Weapon = 1,
    Melee = 2,
    Grenade = 3,
    Consumable = 4,
    Ability = 5,
    Tool = 6,
}

impl Default for QuickActionSlotKind {
    fn default() -> Self {
        QuickActionSlotKind::Empty
    }
}

impl QuickActionSlotKind {
    pub fn as_str(self) -> &'static str {
        match self {
            QuickActionSlotKind::Empty => "empty",
            QuickActionSlotKind::Weapon => "weapon",
            QuickActionSlotKind::Melee => "melee",
            QuickActionSlotKind::Grenade => "grenade",
            QuickActionSlotKind::Consumable => "consumable",
            QuickActionSlotKind::Ability => "ability",
            QuickActionSlotKind::Tool => "tool",
        }
    }
}

/// One slot in the 8-slot bar.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickActionSlot {
    pub kind: QuickActionSlotKind,
    pub item_id: String,
    /// Cooldown ticks remaining (0 = ready).
    pub cooldown_ticks_remaining: u32,
    /// Cooldown total ticks (the radial overlay reads this to draw the wipe).
    pub cooldown_total_ticks: u32,
    /// Ammo / charge / count for the slot (0 when N/A).
    pub ammo: u32,
    /// Max ammo / charge.
    pub ammo_max: u32,
    /// `true` when this slot is disabled by an active hazard (EM disruption,
    /// comms blackout) — sets the rejection reason for invocations.
    pub disabled_by_hazard: bool,
    /// **M14J** § "context-sensitive (only visible when the situation
    /// supports them)" — `true` when the slot's contextual prerequisite is
    /// currently satisfied (e.g. vault is only visible when a vault
    /// candidate is in the parkour signal). Defaults to `true` for slots
    /// without a context predicate.
    #[serde(default = "default_context_available")]
    pub context_available: bool,
    /// **M14J** § set to a slice id when this slot represents one of the
    /// 4 advanced-mobility pie slices (`vault` / `grapple` / `mount` /
    /// `zip_brake`). Empty string for non-M14J slots.
    #[serde(default)]
    pub m14j_slice_id: String,
}

fn default_context_available() -> bool {
    true
}

impl QuickActionSlot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn weapon(item_id: impl Into<String>, ammo: u32, ammo_max: u32) -> Self {
        Self {
            kind: QuickActionSlotKind::Weapon,
            item_id: item_id.into(),
            ammo,
            ammo_max,
            ..Self::default()
        }
    }

    pub fn melee(item_id: impl Into<String>) -> Self {
        Self {
            kind: QuickActionSlotKind::Melee,
            item_id: item_id.into(),
            ..Self::default()
        }
    }

    pub fn grenade(item_id: impl Into<String>, count: u32) -> Self {
        Self {
            kind: QuickActionSlotKind::Grenade,
            item_id: item_id.into(),
            ammo: count,
            ammo_max: count.max(1),
            ..Self::default()
        }
    }

    pub fn consumable(item_id: impl Into<String>, count: u32) -> Self {
        Self {
            kind: QuickActionSlotKind::Consumable,
            item_id: item_id.into(),
            ammo: count,
            ammo_max: count.max(1),
            ..Self::default()
        }
    }

    pub fn ability(item_id: impl Into<String>, cooldown_total: u32) -> Self {
        Self {
            kind: QuickActionSlotKind::Ability,
            item_id: item_id.into(),
            cooldown_total_ticks: cooldown_total,
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.kind, QuickActionSlotKind::Empty)
    }

    /// Returns true when the slot can be invoked now (not on cooldown,
    /// ammo if needed, not disabled by hazard).
    pub fn ready(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.disabled_by_hazard {
            return false;
        }
        if self.cooldown_ticks_remaining > 0 {
            return false;
        }
        match self.kind {
            QuickActionSlotKind::Weapon | QuickActionSlotKind::Grenade | QuickActionSlotKind::Consumable => {
                self.ammo > 0 || self.ammo_max == 0
            }
            _ => true,
        }
    }

    /// Advance the cooldown by one tick.
    pub fn tick_cooldown(&mut self) {
        if self.cooldown_ticks_remaining > 0 {
            self.cooldown_ticks_remaining -= 1;
        }
    }

    /// Start the cooldown to its full reservation.
    pub fn start_cooldown(&mut self) {
        self.cooldown_ticks_remaining = self.cooldown_total_ticks;
    }
}

/// Radial UI phase.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadialPhase {
    /// Radial is dismissed; sim runs at 1.0×.
    #[default]
    Closed = 0,
    /// Radial fading in over 80 ms — sim time multiplier lerps 1.0 → 0.25.
    Opening = 1,
    /// Radial fully visible — sim runs at 0.25× (or 0.50× under reduce_motion).
    Open = 2,
    /// Radial fading out — sim time multiplier lerps back to 1.0.
    Closing = 3,
}

impl RadialPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            RadialPhase::Closed => "closed",
            RadialPhase::Opening => "opening",
            RadialPhase::Open => "open",
            RadialPhase::Closing => "closing",
        }
    }
}

/// Radial state (orthogonal to the 8-slot bar).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct RadialState {
    pub phase: RadialPhase,
    /// Ms elapsed in current phase.
    pub phase_elapsed_ms: u32,
    /// Selected slice (0..7); 255 = no selection (deadzone / cancel).
    pub selected_slice: u8,
    /// Active sim time multiplier — driven by the 80 ms open/close ramp.
    pub sim_time_multiplier: f32,
    /// Mouse cursor position within radial UI (relative to center). Used to
    /// detect deadzone cancel.
    pub cursor_x: f32,
    pub cursor_y: f32,
    /// Tick the radial opened on (for the < 80 ms PARITY-69 gate).
    pub opened_at_tick: u64,
    /// `true` when reduce_motion is on — sim slows to 50% instead of 25%.
    pub reduce_motion: bool,
}

impl RadialState {
    pub const NO_SLICE: u8 = 255;

    pub fn new_closed() -> Self {
        Self {
            phase: RadialPhase::Closed,
            phase_elapsed_ms: 0,
            selected_slice: Self::NO_SLICE,
            sim_time_multiplier: 1.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            opened_at_tick: 0,
            reduce_motion: false,
        }
    }

    pub fn is_open(self) -> bool {
        matches!(self.phase, RadialPhase::Open | RadialPhase::Opening)
    }

    /// Returns the slice the cursor is in given the current cursor position.
    /// `255` when in the deadzone.
    pub fn slice_under_cursor(self) -> u8 {
        let mag = (self.cursor_x * self.cursor_x + self.cursor_y * self.cursor_y).sqrt();
        if mag < QUICK_ACTION_DEADZONE_PX {
            return Self::NO_SLICE;
        }
        // 0 = up; clockwise.
        let mut angle = self.cursor_x.atan2(-self.cursor_y);
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        let slice = ((angle / std::f32::consts::TAU) * 8.0).floor() as u32;
        (slice % 8) as u8
    }
}

/// Per-actor quick-action runtime state.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickActionBarState {
    pub slots: [QuickActionSlot; QUICK_ACTION_SLOT_COUNT],
    pub last_used_slot: u8,
    pub radial: RadialState,
    /// Tick on which the most recent Q-tap began. Used to disambiguate
    /// tap-Q (≤ 120 ms) from hold-Q (> 120 ms → radial).
    pub q_press_tick: u64,
    /// `true` while the Q key is held.
    pub q_held: bool,
    /// **M14J § "Quick-action wheel (M14A) gets four new pie slices:
    /// `[Vault] [Grapple] [Mount/Dismount] [Zip-Brake]` — context-sensitive
    /// (only visible when the situation supports them)"**. Lives parallel
    /// to the 8-slot bar so the radial renderer can overlay them when
    /// `context_available` flips to true. Engine flips the flag per-tick
    /// based on actor state.
    #[serde(default)]
    pub m14j_slices: Vec<QuickActionSlot>,
}

impl QuickActionBarState {
    pub fn infantry_default() -> Self {
        let mut bar = Self::default();
        bar.slots[0] = QuickActionSlot::weapon("rifle_m1_default", 24, 30);
        bar.slots[1] = QuickActionSlot::weapon("pistol_m1_default", 12, 12);
        bar.slots[2] = QuickActionSlot::melee("knife_m6_default");
        bar.slots[3] = QuickActionSlot::grenade("frag_m6_default", 3);
        bar.slots[4] = QuickActionSlot::consumable("medkit", 1);
        // **M14J § "Quick-action wheel (M14A) gets four new pie slices:
        // `[Vault] [Grapple] [Mount/Dismount] [Zip-Brake]` — context-sensitive
        // (only visible when the situation supports them)". The slices live
        // in a parallel overlay (not in the 8-slot bar) so they don't
        // displace the player's primary weapon/melee/grenade slots.
        bar.register_m14j_slices();
        bar.radial.sim_time_multiplier = 1.0;
        bar
    }

    /// **M14J** § "register the four new context-sensitive slices via the
    /// M14A `qab.register` API". Pushes the four slices into the parallel
    /// `m14j_slices` overlay. All slices start with `context_available=false`
    /// so they appear only when the engine sets the relevant context flag.
    pub fn register_m14j_slices(&mut self) {
        self.m14j_slices.clear();
        for (slice_id, cooldown) in [
            ("vault", 12u32),
            ("grapple", 60),
            ("mount_dismount", 12),
            ("zip_brake", 6),
        ] {
            let mut s = QuickActionSlot::ability(slice_id, cooldown);
            s.m14j_slice_id = slice_id.to_string();
            s.context_available = false;
            self.m14j_slices.push(s);
        }
    }

    /// **M14J** § returns the list of registered M14J slice ids + whether
    /// they're currently context-available. Used by the HUD + observe surface.
    pub fn m14j_context_slices(&self) -> Vec<(String, bool)> {
        self.m14j_slices
            .iter()
            .map(|s| (s.m14j_slice_id.clone(), s.context_available))
            .collect()
    }

    /// **M14J** § per-tick update of the M14J slice context flags. Called
    /// by the engine with the actor's current parkour/rope/mount/zipline
    /// state booleans.
    pub fn update_m14j_context(
        &mut self,
        vault_available: bool,
        grapple_available: bool,
        mount_available: bool,
        zip_brake_available: bool,
    ) {
        for s in self.m14j_slices.iter_mut() {
            match s.m14j_slice_id.as_str() {
                "vault" => s.context_available = vault_available,
                "grapple" => s.context_available = grapple_available,
                "mount_dismount" => s.context_available = mount_available,
                "zip_brake" => s.context_available = zip_brake_available,
                _ => {}
            }
        }
    }

    pub fn powered_armor_default() -> Self {
        let mut bar = Self::infantry_default();
        bar.slots[0] = QuickActionSlot::weapon("carbine_m1_default", 30, 30);
        bar.slots[5] = QuickActionSlot::ability("shield_burst", 1200);
        bar.slots[6] = QuickActionSlot::ability("overdrive", 1800);
        bar
    }

    pub fn light_mech_default() -> Self {
        let mut bar = Self::default();
        bar.slots[0] = QuickActionSlot::weapon("autocannon_m1_default", 80, 80);
        bar.slots[1] = QuickActionSlot::weapon("pistol_m1_default", 12, 12);
        bar.slots[3] = QuickActionSlot::grenade("grenade_launcher", 6);
        bar.slots[4] = QuickActionSlot::consumable("repair_drone", 1);
        bar.slots[5] = QuickActionSlot::ability("time_slow", 2400);
        bar.slots[6] = QuickActionSlot::ability("overdrive", 1800);
        bar.slots[7] = QuickActionSlot::ability("repair_pulse", 1200);
        bar
    }

    pub fn heavy_trooper_default() -> Self {
        let mut bar = Self::default();
        bar.slots[0] = QuickActionSlot::weapon("rifle_m5_mech_heavy", 24, 24);
        bar.slots[1] = QuickActionSlot::weapon("pistol_m1_default", 12, 12);
        bar.slots[2] = QuickActionSlot::melee("melee_bash");
        bar.slots[3] = QuickActionSlot::grenade("he_grenade", 3);
        bar.slots[4] = QuickActionSlot::consumable("field_repair", 1);
        bar.slots[5] = QuickActionSlot::ability("shield_burst", 1200);
        bar.slots[6] = QuickActionSlot::ability("overdrive", 1800);
        bar.slots[7] = QuickActionSlot::ability("emp_pulse", 2400);
        bar
    }

    /// Tap key 1-8 — instant slot invoke. Returns true when accepted.
    pub fn try_invoke_slot(&mut self, slot: u8) -> InvokeOutcome {
        let idx = slot as usize;
        if idx >= QUICK_ACTION_SLOT_COUNT {
            return InvokeOutcome::Rejected("slot_out_of_range");
        }
        let s = &mut self.slots[idx];
        if s.is_empty() {
            return InvokeOutcome::Rejected("slot_empty");
        }
        if s.disabled_by_hazard {
            return InvokeOutcome::Rejected("em_disruption");
        }
        if s.cooldown_ticks_remaining > 0 {
            return InvokeOutcome::Rejected("on_cooldown");
        }
        if matches!(
            s.kind,
            QuickActionSlotKind::Weapon | QuickActionSlotKind::Grenade | QuickActionSlotKind::Consumable
        ) && s.ammo == 0
            && s.ammo_max > 0
        {
            return InvokeOutcome::Rejected("no_ammo");
        }
        self.last_used_slot = slot;
        s.start_cooldown();
        InvokeOutcome::Accepted
    }

    /// Tap-Q — invoke the last-used slot.
    pub fn try_invoke_toggle(&mut self) -> InvokeOutcome {
        let slot = self.last_used_slot;
        self.try_invoke_slot(slot)
    }

    /// Open the radial. Returns the tick the radial opened at — engine emits
    /// `quick_action_radial_opened` with the tick.
    pub fn open_radial(&mut self, tick: u64, reduce_motion: bool) {
        self.radial.phase = RadialPhase::Opening;
        self.radial.phase_elapsed_ms = 0;
        self.radial.opened_at_tick = tick;
        self.radial.selected_slice = RadialState::NO_SLICE;
        self.radial.reduce_motion = reduce_motion;
    }

    /// Close the radial. If `commit_slice` is `Some`, the slot at that index
    /// is invoked.
    pub fn close_radial(&mut self, commit_slice: Option<u8>) -> Option<u8> {
        let mut invoked = None;
        if let Some(s) = commit_slice {
            if s != RadialState::NO_SLICE && self.try_invoke_slot(s) == InvokeOutcome::Accepted {
                invoked = Some(s);
            }
        }
        self.radial.phase = RadialPhase::Closing;
        self.radial.phase_elapsed_ms = 0;
        self.radial.selected_slice = RadialState::NO_SLICE;
        invoked
    }

    /// Per-tick advance the radial open/close ramp + cool down each slot.
    pub fn tick(&mut self, dt_ms: u32) {
        for s in &mut self.slots {
            s.tick_cooldown();
        }
        let slow_target = if self.radial.reduce_motion {
            QUICK_ACTION_TIME_SLOW_REDUCE_MOTION
        } else {
            QUICK_ACTION_TIME_SLOW
        };
        match self.radial.phase {
            RadialPhase::Closed => {
                self.radial.sim_time_multiplier = 1.0;
            }
            RadialPhase::Opening => {
                self.radial.phase_elapsed_ms = self.radial.phase_elapsed_ms.saturating_add(dt_ms);
                let progress =
                    (self.radial.phase_elapsed_ms as f32 / QUICK_ACTION_OPEN_MS as f32).clamp(0.0, 1.0);
                self.radial.sim_time_multiplier = 1.0 + (slow_target - 1.0) * progress;
                if self.radial.phase_elapsed_ms >= QUICK_ACTION_OPEN_MS {
                    self.radial.phase = RadialPhase::Open;
                    self.radial.phase_elapsed_ms = 0;
                    self.radial.sim_time_multiplier = slow_target;
                }
            }
            RadialPhase::Open => {
                self.radial.sim_time_multiplier = slow_target;
            }
            RadialPhase::Closing => {
                self.radial.phase_elapsed_ms = self.radial.phase_elapsed_ms.saturating_add(dt_ms);
                let progress =
                    (self.radial.phase_elapsed_ms as f32 / QUICK_ACTION_OPEN_MS as f32).clamp(0.0, 1.0);
                self.radial.sim_time_multiplier = slow_target + (1.0 - slow_target) * progress;
                if self.radial.phase_elapsed_ms >= QUICK_ACTION_OPEN_MS {
                    self.radial.phase = RadialPhase::Closed;
                    self.radial.phase_elapsed_ms = 0;
                    self.radial.sim_time_multiplier = 1.0;
                }
            }
        }
    }

    /// Apply a hazard zone to slot-disabling. Pass `Some([6, 7, 8])` to
    /// disable electronic ability slots in an EM zone.
    pub fn apply_hazard_disabled_slots(&mut self, slots: &[u8]) {
        for s in &mut self.slots {
            s.disabled_by_hazard = false;
        }
        for &idx in slots {
            let i = idx as usize;
            if i < QUICK_ACTION_SLOT_COUNT {
                self.slots[i].disabled_by_hazard = true;
            }
        }
    }

    /// Cycle the weapon family within a slot's category. `direction` is +1 / -1.
    /// Returns the new item id when something changed.
    pub fn cycle_within_slot(&mut self, slot: u8, _direction: i32) -> Option<String> {
        let idx = slot as usize;
        if idx >= QUICK_ACTION_SLOT_COUNT {
            return None;
        }
        // In M14A baseline, weapon-cycle behavior is content-driven via the
        // inventory grid. We expose the surface as a no-op + return the
        // current item id so cfctl callers can still observe a stable read.
        Some(self.slots[idx].item_id.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeOutcome {
    Accepted,
    Rejected(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infantry_default_layout() {
        let bar = QuickActionBarState::infantry_default();
        assert_eq!(bar.slots[0].kind, QuickActionSlotKind::Weapon);
        assert_eq!(bar.slots[4].kind, QuickActionSlotKind::Consumable);
        assert!(bar.slots[7].is_empty());
    }

    #[test]
    fn invoke_empty_rejects() {
        let mut bar = QuickActionBarState::infantry_default();
        assert_eq!(bar.try_invoke_slot(7), InvokeOutcome::Rejected("slot_empty"));
    }

    #[test]
    fn invoke_records_last_used() {
        let mut bar = QuickActionBarState::infantry_default();
        assert_eq!(bar.try_invoke_slot(2), InvokeOutcome::Accepted);
        assert_eq!(bar.last_used_slot, 2);
    }

    #[test]
    fn tap_q_invokes_last_used() {
        let mut bar = QuickActionBarState::infantry_default();
        bar.try_invoke_slot(3);
        assert_eq!(bar.try_invoke_toggle(), InvokeOutcome::Accepted);
        assert_eq!(bar.last_used_slot, 3);
    }

    #[test]
    fn radial_opens_within_80ms() {
        let mut bar = QuickActionBarState::infantry_default();
        bar.open_radial(0, false);
        // Tick in 4-ms increments — 20 * 4 = 80 ms.
        for _ in 0..20 {
            bar.tick(4);
        }
        assert_eq!(bar.radial.phase, RadialPhase::Open);
        assert!((bar.radial.sim_time_multiplier - QUICK_ACTION_TIME_SLOW).abs() < 1e-6);
    }

    #[test]
    fn radial_close_returns_to_full_speed() {
        let mut bar = QuickActionBarState::infantry_default();
        bar.open_radial(0, false);
        for _ in 0..20 {
            bar.tick(4);
        }
        let invoked = bar.close_radial(Some(0));
        assert_eq!(invoked, Some(0));
        for _ in 0..20 {
            bar.tick(4);
        }
        assert!((bar.radial.sim_time_multiplier - 1.0).abs() < 1e-6);
    }

    #[test]
    fn deadzone_cancels_radial() {
        let mut bar = QuickActionBarState::infantry_default();
        bar.open_radial(0, false);
        bar.radial.cursor_x = 1.0;
        bar.radial.cursor_y = 1.0;
        // Magnitude sqrt(2) < 12 px → in deadzone.
        assert_eq!(bar.radial.slice_under_cursor(), RadialState::NO_SLICE);
        bar.close_radial(Some(RadialState::NO_SLICE));
        // last_used_slot unchanged.
        assert_eq!(bar.last_used_slot, 0);
    }

    #[test]
    fn em_disrupted_slots_reject_invocation() {
        let mut bar = QuickActionBarState::powered_armor_default();
        bar.apply_hazard_disabled_slots(&[5, 6, 7]);
        assert_eq!(bar.try_invoke_slot(5), InvokeOutcome::Rejected("em_disruption"));
        // Non-electronic slot still works.
        assert_eq!(bar.try_invoke_slot(0), InvokeOutcome::Accepted);
    }

    #[test]
    fn reduce_motion_uses_50_percent_time_slow() {
        let mut bar = QuickActionBarState::infantry_default();
        bar.open_radial(0, true);
        for _ in 0..20 {
            bar.tick(4);
        }
        assert!((bar.radial.sim_time_multiplier - QUICK_ACTION_TIME_SLOW_REDUCE_MOTION).abs() < 1e-6);
    }
}
