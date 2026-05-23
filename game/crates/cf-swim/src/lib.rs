//! M16 § Swimming + underwater combat.
//!
//! Water tiles permit swim stance with 70% baseline speed; underwater
//! actors drain oxygen and stack `hypoxic`; prolonged drowning ends in
//! `actor.drowning_lethal`. This crate owns the M16-level swim contract
//! (entry/exit detection, oxygen tracking, drowning triggers); the
//! per-stroke limb-path advance + breath drain remain in
//! `cf-actor::m14j_sim` to preserve the existing tick contract.
//!
//! Underwater-specific weapons (harpoon, spear gun) are tagged via
//! [`is_underwater_weapon`] so the firing path can gate normal weapons
//! while swimming.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::similar_names
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Swim speed multiplier per spec § "Swim stance: 70% speed".
pub const SWIM_SPEED_MULTIPLIER: f32 = 0.70;

/// Underwater weapon range scale per spec § "reduced weapon range".
pub const UNDERWATER_WEAPON_RANGE_MULTIPLIER: f32 = 0.50;

/// Maximum oxygen reservoir in seconds (race-default; M17 origin reaction
/// matrix overrides this).
pub const DEFAULT_OXYGEN_RESERVOIR_SECONDS: f32 = 30.0;

/// Per-tick oxygen drain rate while submerged (seconds per second).
pub const OXYGEN_DRAIN_PER_SECOND_SUBMERGED: f32 = 1.0;

/// Per-tick oxygen recovery rate while at surface (seconds per second).
pub const OXYGEN_RECOVER_PER_SECOND_SURFACE: f32 = 2.0;

/// Continuous drowning duration (seconds) past zero oxygen before
/// `actor.drowning_lethal` fires.
pub const DROWNING_LETHAL_DURATION_SECONDS: f32 = 6.0;

/// Submersion state used by the swim tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmersionState {
    Dry,
    Surface,
    Submerged,
}

/// Live state per actor. Engine stores one of these per actor that has
/// ever entered water (lazy-initialized).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwimState {
    pub state: SubmersionState,
    /// Oxygen reservoir in seconds [0, oxygen_max_seconds].
    pub oxygen_seconds: f32,
    pub oxygen_max_seconds: f32,
    /// True when the actor is currently in the swim stance (per M14J).
    pub swim_active: bool,
    /// Accumulated time at zero oxygen (seconds). Trips lethal at
    /// `DROWNING_LETHAL_DURATION_SECONDS`.
    pub drowning_seconds: f32,
    /// True for one tick after `actor.drowning_lethal` fires; engine
    /// clears the actor to dead and resets this.
    pub lethal_triggered: bool,
    /// True when the actor's helmet seal suppresses drowning per the
    /// M19C dive-suit pathway.
    pub helmet_sealed: bool,
}

impl SwimState {
    pub fn new(oxygen_max_seconds: f32) -> Self {
        Self {
            state: SubmersionState::Dry,
            oxygen_seconds: oxygen_max_seconds,
            oxygen_max_seconds,
            swim_active: false,
            drowning_seconds: 0.0,
            lethal_triggered: false,
            helmet_sealed: false,
        }
    }
}

impl Default for SwimState {
    fn default() -> Self {
        Self::new(DEFAULT_OXYGEN_RESERVOIR_SECONDS)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwimStartedEvent {
    pub actor_id: u64,
    pub position: [f32; 2],
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwimEndedEvent {
    pub actor_id: u64,
    pub position: [f32; 2],
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrowningStartedEvent {
    pub actor_id: u64,
    pub position: [f32; 2],
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrowningLethalEvent {
    pub actor_id: u64,
    pub position: [f32; 2],
    pub tick: u64,
    pub depth_m: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SwimTickEvents {
    pub swim_started: Vec<SwimStartedEvent>,
    pub swim_ended: Vec<SwimEndedEvent>,
    pub drowning_started: Vec<DrowningStartedEvent>,
    pub drowning_lethal: Vec<DrowningLethalEvent>,
    /// True when the actor is currently in a hypoxic state — engine
    /// stacks the `affliction.applied { kind:"hypoxic" }` event with this.
    pub hypoxic_actor_ids: Vec<u64>,
}

/// Inputs for one swim tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwimTickInput {
    pub actor_id: u64,
    pub position: [f32; 2],
    /// True when the actor is overlapping a water tile.
    pub in_water: bool,
    /// True when the actor's head/upper torso is fully submerged (depth
    /// greater than head-height under the water surface).
    pub submerged: bool,
    /// True when the actor wears a sealed helmet or dive suit (M19C
    /// integration).
    pub helmet_sealed: bool,
    /// Effective depth in meters below surface (used in the
    /// `actor.drowning_lethal` payload).
    pub depth_m: f32,
}

/// Advance one actor's swim state for one sim tick. Pure / deterministic.
pub fn tick_actor(
    state: &mut SwimState,
    input: SwimTickInput,
    tick: u64,
    tick_rate_hz: u32,
) -> SwimTickEvents {
    let dt_seconds = 1.0_f32 / (tick_rate_hz.max(1) as f32);
    let mut out = SwimTickEvents::default();
    state.helmet_sealed = input.helmet_sealed;
    let prev = state.state;
    // Determine new state.
    let new_state = if input.submerged {
        SubmersionState::Submerged
    } else if input.in_water {
        SubmersionState::Surface
    } else {
        SubmersionState::Dry
    };
    state.state = new_state;

    let was_swimming = state.swim_active;
    let is_swimming = matches!(new_state, SubmersionState::Surface | SubmersionState::Submerged);
    state.swim_active = is_swimming;
    if is_swimming && !was_swimming {
        out.swim_started.push(SwimStartedEvent {
            actor_id: input.actor_id,
            position: input.position,
            tick,
        });
    } else if !is_swimming && was_swimming {
        out.swim_ended.push(SwimEndedEvent {
            actor_id: input.actor_id,
            position: input.position,
            tick,
        });
    }

    // Oxygen accounting.
    if matches!(new_state, SubmersionState::Submerged) && !input.helmet_sealed {
        let prev_oxygen = state.oxygen_seconds;
        state.oxygen_seconds = (state.oxygen_seconds - OXYGEN_DRAIN_PER_SECOND_SUBMERGED * dt_seconds).max(0.0);
        if prev_oxygen > 0.0 && state.oxygen_seconds <= 0.0 {
            out.drowning_started.push(DrowningStartedEvent {
                actor_id: input.actor_id,
                position: input.position,
                tick,
            });
        }
        if state.oxygen_seconds <= 0.0 {
            out.hypoxic_actor_ids.push(input.actor_id);
            state.drowning_seconds += dt_seconds;
            if state.drowning_seconds >= DROWNING_LETHAL_DURATION_SECONDS && !state.lethal_triggered {
                state.lethal_triggered = true;
                out.drowning_lethal.push(DrowningLethalEvent {
                    actor_id: input.actor_id,
                    position: input.position,
                    tick,
                    depth_m: input.depth_m,
                });
            }
        }
    } else {
        // Surface or dry — recover oxygen + reset drowning timer.
        if state.oxygen_seconds < state.oxygen_max_seconds {
            state.oxygen_seconds =
                (state.oxygen_seconds + OXYGEN_RECOVER_PER_SECOND_SURFACE * dt_seconds).min(state.oxygen_max_seconds);
        }
        if state.drowning_seconds > 0.0 {
            state.drowning_seconds = 0.0;
            state.lethal_triggered = false;
        }
    }

    let _ = prev;
    out
}

/// True when the given equipment id is an underwater-specific weapon
/// (harpoon, spear_gun). The firing path consults this to gate firearm
/// shots while swimming.
pub fn is_underwater_weapon(equipment_id: &str) -> bool {
    matches!(
        equipment_id,
        "harpoon" | "spear_gun" | "spear" | "underwater_harpoon" | "trident"
    )
}

/// The carry-stance speed multiplier the engine applies when the actor's
/// swim_active flag is true. Combined with any other modifiers from
/// stamina / encumbrance / artifacts.
pub fn swim_speed_multiplier(state: &SwimState) -> f32 {
    if state.swim_active {
        SWIM_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

/// Per-actor swim state map. Engine stores this on its mutable state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwimWorld {
    pub by_actor: BTreeMap<u64, SwimState>,
}

impl SwimWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_actor(&mut self, actor_id: u64, oxygen_max_seconds: f32) -> &mut SwimState {
        self.by_actor
            .entry(actor_id)
            .or_insert_with(|| SwimState::new(oxygen_max_seconds))
    }

    pub fn tick_one(
        &mut self,
        input: SwimTickInput,
        tick: u64,
        tick_rate_hz: u32,
        oxygen_max_seconds: f32,
    ) -> SwimTickEvents {
        let state = self.ensure_actor(input.actor_id, oxygen_max_seconds);
        tick_actor(state, input, tick, tick_rate_hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_water_emits_swim_started() {
        let mut state = SwimState::default();
        let ev = tick_actor(
            &mut state,
            SwimTickInput {
                actor_id: 1,
                position: [0.0, 0.0],
                in_water: true,
                submerged: false,
                helmet_sealed: false,
                depth_m: 0.0,
            },
            1,
            60,
        );
        assert_eq!(ev.swim_started.len(), 1);
        assert_eq!(ev.swim_ended.len(), 0);
        assert!(state.swim_active);
    }

    #[test]
    fn submersion_drains_oxygen_and_eventually_drowns() {
        let mut state = SwimState::default();
        // Run 60s submerged at 60 Hz.
        let mut started = false;
        let mut lethal = false;
        for tick in 1..=(60 * 60 + 600) as u64 {
            let ev = tick_actor(
                &mut state,
                SwimTickInput {
                    actor_id: 1,
                    position: [0.0, -2.0],
                    in_water: true,
                    submerged: true,
                    helmet_sealed: false,
                    depth_m: 2.0,
                },
                tick,
                60,
            );
            if !ev.drowning_started.is_empty() {
                started = true;
            }
            if !ev.drowning_lethal.is_empty() {
                lethal = true;
                break;
            }
        }
        assert!(started, "drowning_started must fire when oxygen hits zero");
        assert!(lethal, "drowning_lethal must fire after sustained drowning");
    }

    #[test]
    fn surface_recovers_oxygen() {
        let mut state = SwimState::default();
        state.oxygen_seconds = 5.0;
        for tick in 1..=2000u64 {
            tick_actor(
                &mut state,
                SwimTickInput {
                    actor_id: 1,
                    position: [0.0, 0.0],
                    in_water: true,
                    submerged: false,
                    helmet_sealed: false,
                    depth_m: 0.0,
                },
                tick,
                60,
            );
        }
        assert!(
            state.oxygen_seconds > 25.0,
            "oxygen should recover near full at the surface; got {}",
            state.oxygen_seconds
        );
    }

    #[test]
    fn helmet_seal_suppresses_drowning() {
        let mut state = SwimState::default();
        for tick in 1..=600u64 {
            let ev = tick_actor(
                &mut state,
                SwimTickInput {
                    actor_id: 1,
                    position: [0.0, -2.0],
                    in_water: true,
                    submerged: true,
                    helmet_sealed: true,
                    depth_m: 2.0,
                },
                tick,
                60,
            );
            assert!(ev.drowning_lethal.is_empty());
        }
        assert!(state.oxygen_seconds > 0.0, "helmet seal preserves oxygen");
    }

    #[test]
    fn swim_speed_multiplier_applies_when_active() {
        let mut state = SwimState::default();
        state.swim_active = true;
        assert!((swim_speed_multiplier(&state) - 0.70).abs() < 1e-3);
        state.swim_active = false;
        assert!((swim_speed_multiplier(&state) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn underwater_weapons_tagged() {
        assert!(is_underwater_weapon("harpoon"));
        assert!(is_underwater_weapon("spear_gun"));
        assert!(!is_underwater_weapon("rifle_m1_default"));
    }

    #[test]
    fn exiting_water_emits_swim_ended() {
        let mut state = SwimState::default();
        tick_actor(
            &mut state,
            SwimTickInput {
                actor_id: 1,
                position: [0.0, 0.0],
                in_water: true,
                submerged: false,
                helmet_sealed: false,
                depth_m: 0.0,
            },
            1,
            60,
        );
        let ev = tick_actor(
            &mut state,
            SwimTickInput {
                actor_id: 1,
                position: [1.0, 1.0],
                in_water: false,
                submerged: false,
                helmet_sealed: false,
                depth_m: 0.0,
            },
            2,
            60,
        );
        assert_eq!(ev.swim_ended.len(), 1);
        assert!(!state.swim_active);
    }
}
