//! M9C § "MG nest module + ammo box + tripod variant": MG-nest
//! crewing kernel, ammo-box auto-feed, tripod deploy/pack state
//! machine, spotter-scope adjacency bonus, and bunker firing slit
//! aperture-based damage routing.
//!
//! Spec § "Notes for the implementer":
//!
//! > **Crewing semantics** (the hardest part): a crewed fortification
//! > has a 1:1 actor→fortification binding. The actor's stance becomes
//! > `Crewing { fortification_id }`. Movement inputs are suspended;
//! > firing inputs are rebound to the fortification's mounted weapon.
//! > Use M44D crew-slot grammar — same pattern, fortification-side
//! > instead of vehicle-side. Uncrew via cfctl OR via actor-death OR
//! > via fortification-destruction.
//!
//! This module owns the pure state machines + per-tick decision
//! helpers that the cf-control engine drives. Replay-event emission
//! (`mg_nest_crewed`, `mg_nest_uncrewed`, `mg_nest_fired_burst`,
//! `ammo_box_depleted`, `mg_tripod_deployed`) is performed by the
//! engine when its tick consumes the outputs of these helpers.
//!
//! VAL-M9C-011 / 012 / 013 / 014 / 015 / 018 land here.
//! VAL-M9C-BUNKER-SLIT-DAMAGE lands here.
//! VAL-M9C-SPOTTER-SCOPE-BEHAVIOR lands here.
//! VAL-M9C-UNCREW-EMIT lands here.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// Spec table row HP for a built `mg_nest_static`.
pub const MG_NEST_STATIC_MAX_HP: u32 = 800;
/// Spec table row HP for an `ammo_box_mg`.
pub const AMMO_BOX_MG_MAX_HP: u32 = 200;
/// Spec table row HP for a packed `mg_tripod_portable` once deployed.
pub const MG_TRIPOD_DEPLOYED_HP: u32 = 400;
/// Spec table: 800-round belt cache for the canonical ammo box.
pub const AMMO_BOX_MG_ROUNDS: u32 = 800;
/// Spec table row HP for a placed `bunker_firing_slit` (concrete).
pub const BUNKER_FIRING_SLIT_HP: u32 = 1600;
/// Spec § Notes: bunker firing slit aperture in pixels (rounds outside
/// the 8-px aperture bounce off the surrounding concrete).
pub const BUNKER_FIRING_SLIT_APERTURE_PX: u32 = 8;
/// Spec table row HP for a standalone `spotter_scope`.
pub const SPOTTER_SCOPE_HP: u32 = 100;
/// Spec § "Spotter scope is a passive force-multiplier": +50% adjusted
/// acquisition rate for the paired actor.
pub const SPOTTER_SCOPE_ACQUISITION_MULTIPLIER: f32 = 1.5;
/// Spec § "Tripod portable is the squad-portable counterpart": 4-second
/// deploy timer + 4-second pack-up timer.
pub const MG_TRIPOD_DEPLOY_SECONDS: u32 = 4;
/// Spec § AI-MG-A-02 doctrine: AI uncrew + retreat when nest HP drops
/// below this threshold.
pub const MG_DOCTRINE_RETREAT_HP_THRESHOLD: u32 = 200;
/// Spec § AI-MG-A-02 doctrine: AI crew nearest empty MG nest when a
/// threat is detected within this range (in tiles).
pub const MG_DOCTRINE_THREAT_RANGE_TILES: u32 = 24;
/// Spec § AI-MG-A-02 doctrine: search radius around AI for an empty
/// MG nest to crew (in tiles). The spec text reads "crew nearest empty
/// MG within 8 tiles".
pub const MG_DOCTRINE_CREW_SEARCH_RADIUS_TILES: u32 = 8;

/// Cause stamped on the `mg_nest_uncrewed` replay event. Matches the
/// `reason` enum declared in
/// `game/crates/cf-replay/schemas/event/mg_nest_uncrewed.json`:
/// `voluntary | actor_killed | nest_destroyed`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MgNestUncrewReason {
    /// Player or AI issued `act.player.uncrew_fortification`.
    Voluntary = 0,
    /// Crewing actor died while crewing.
    ActorKilled = 1,
    /// MG nest HP reached 0 while crewed.
    NestDestroyed = 2,
}

impl MgNestUncrewReason {
    pub const ALL: [MgNestUncrewReason; 3] = [
        MgNestUncrewReason::Voluntary,
        MgNestUncrewReason::ActorKilled,
        MgNestUncrewReason::NestDestroyed,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MgNestUncrewReason::Voluntary => "voluntary",
            MgNestUncrewReason::ActorKilled => "actor_killed",
            MgNestUncrewReason::NestDestroyed => "nest_destroyed",
        }
    }
}

/// Replay-event-payload shape for `mg_nest_uncrewed`. The recorder
/// converts this into the JSON envelope declared in
/// `mg_nest_uncrewed.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgNestUncrewedEvent {
    pub nest_id: FortificationId,
    pub actor_id: Option<u32>,
    pub reason: MgNestUncrewReason,
    pub tick_index: u64,
}

/// MG-nest crew-slot. The full state machine: crew binding, ammo-box
/// auto-feed, bind/release lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgNest {
    pub id: FortificationId,
    pub hp: u32,
    /// Currently-bound ammo box (Some when an `ammo_box_mg` is
    /// adjacent + auto-feeding). The kernel decrements its
    /// `rounds_remaining` per fired round and emits
    /// `ammo_box_depleted` when it reaches 0.
    pub ammo_box: Option<AmmoBoxMg>,
    /// Actor id of the crewing actor; `None` when un-crewed.
    pub crewed_by: Option<u32>,
}

impl MgNest {
    #[must_use]
    pub fn new_built(id: FortificationId) -> Self {
        Self {
            id,
            hp: MG_NEST_STATIC_MAX_HP,
            ammo_box: Some(AmmoBoxMg::new()),
            crewed_by: None,
        }
    }

    /// True when an actor is currently bound to the nest.
    #[must_use]
    pub fn is_crewed(&self) -> bool {
        self.crewed_by.is_some()
    }

    /// True when the nest's HP has reached 0 (destruction emits
    /// `mg_nest_uncrewed { reason: nest_destroyed }`).
    #[must_use]
    pub fn is_destroyed(&self) -> bool {
        self.hp == 0
    }

    /// True when the nest's HP has dropped below the AI-MG-A-02
    /// retreat threshold (spec § doctrine: "uncrew + retreat when nest
    /// HP < 200").
    #[must_use]
    pub fn below_retreat_threshold(&self) -> bool {
        self.hp < MG_DOCTRINE_RETREAT_HP_THRESHOLD
    }

    /// Crew the nest with the supplied actor. Returns `Err` when the
    /// nest is already crewed.
    pub fn crew(&mut self, actor_id: u32) -> Result<(), MgNestError> {
        if self.crewed_by.is_some() {
            return Err(MgNestError::AlreadyCrewed);
        }
        if self.is_destroyed() {
            return Err(MgNestError::Destroyed);
        }
        self.crewed_by = Some(actor_id);
        Ok(())
    }

    /// Uncrew the nest, returning the prior actor id + the reason for
    /// the uncrewing. Returns `None` when the nest was already empty.
    pub fn uncrew(
        &mut self,
        reason: MgNestUncrewReason,
        tick_index: u64,
    ) -> Option<MgNestUncrewedEvent> {
        let actor_id = self.crewed_by.take()?;
        Some(MgNestUncrewedEvent {
            nest_id: self.id,
            actor_id: Some(actor_id),
            reason,
            tick_index,
        })
    }

    /// Apply damage to the nest. Returns the uncrew event when the
    /// damage drives HP to 0 while crewed.
    pub fn apply_damage(
        &mut self,
        damage: u32,
        tick_index: u64,
    ) -> Option<MgNestUncrewedEvent> {
        self.hp = self.hp.saturating_sub(damage);
        if self.hp == 0 {
            return self.uncrew(MgNestUncrewReason::NestDestroyed, tick_index);
        }
        None
    }

    /// Notify the nest that its crewing actor died this tick. Returns
    /// the uncrew event for emission by the recorder.
    pub fn on_crewing_actor_killed(
        &mut self,
        tick_index: u64,
    ) -> Option<MgNestUncrewedEvent> {
        self.uncrew(MgNestUncrewReason::ActorKilled, tick_index)
    }

    /// Try to fire `rounds` rounds through the mounted MG. Returns the
    /// number of rounds actually fired (capped by the bound ammo box's
    /// remaining capacity) and a flag that's true when the ammo box
    /// became depleted *this call* (the engine emits `ammo_box_depleted`
    /// then; subsequent calls are no-ops until a swap).
    ///
    /// Returns `(0, false)` when the nest isn't crewed.
    pub fn fire_rounds(&mut self, rounds: u32) -> MgNestFireOutcome {
        if !self.is_crewed() || self.is_destroyed() {
            return MgNestFireOutcome::default();
        }
        let Some(box_ref) = self.ammo_box.as_mut() else {
            return MgNestFireOutcome::default();
        };
        let to_fire = rounds.min(box_ref.rounds_remaining);
        box_ref.rounds_remaining -= to_fire;
        let depleted = box_ref.rounds_remaining == 0 && to_fire > 0;
        MgNestFireOutcome {
            rounds_fired: to_fire,
            ammo_box_depleted: depleted,
        }
    }

    /// Swap in a fresh `ammo_box_mg` from the player's inventory. The
    /// new box resets `rounds_remaining` to [`AMMO_BOX_MG_ROUNDS`]. The
    /// engine emits no special event for the swap itself (subsequent
    /// `mg_nest_fired_burst` events resume).
    pub fn swap_ammo_box(&mut self) {
        self.ammo_box = Some(AmmoBoxMg::new());
    }
}

/// Auto-feed ammo box for a placed `mg_nest_static`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmmoBoxMg {
    pub hp: u32,
    pub rounds_remaining: u32,
}

impl AmmoBoxMg {
    #[must_use]
    pub fn new() -> Self {
        Self {
            hp: AMMO_BOX_MG_MAX_HP,
            rounds_remaining: AMMO_BOX_MG_ROUNDS,
        }
    }

    /// True when the box has been completely drained.
    #[must_use]
    pub const fn is_depleted(self) -> bool {
        self.rounds_remaining == 0
    }
}

impl Default for AmmoBoxMg {
    fn default() -> Self {
        Self::new()
    }
}

/// Output of [`MgNest::fire_rounds`]. The engine consumes this each
/// tick to emit `mg_nest_fired_burst` + `ammo_box_depleted` events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MgNestFireOutcome {
    pub rounds_fired: u32,
    pub ammo_box_depleted: bool,
}

/// Typed error for `MgNest::crew` callers. cf-control converts to a
/// JSON-RPC `invalid_param_reason` shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MgNestError {
    AlreadyCrewed,
    Destroyed,
}

impl MgNestError {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MgNestError::AlreadyCrewed => "mg_nest_already_crewed",
            MgNestError::Destroyed => "mg_nest_destroyed",
        }
    }
}

/// Phase of the squad-portable MG tripod state machine. Per spec:
///
/// > **Tripod portable** is the **squad-portable** counterpart: M22
/// > squad doctrine lets one actor carry + deploy the tripod, then
/// > crew it; classic Cortex Command "set up the MG to lock the
/// > corridor" play.
///
/// Lifecycle: `Packed → Deploying (4s) → Deployed → Crewed → Deployed
/// → Packing (4s) → Packed`. Ammo is preserved bit-equal across the
/// pack/unpack cycle (VAL-M9C-018).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MgTripodPhase {
    /// Packed in inventory (1 slot, 50 kg per spec).
    Packed = 0,
    /// 4-second deploy timer is counting down.
    Deploying = 1,
    /// Tripod is placed at `pos`; not yet crewed.
    Deployed = 2,
    /// Tripod is placed at `pos` and an actor is crewing it.
    Crewed = 3,
    /// 4-second pack timer is counting down toward Packed.
    Packing = 4,
}

impl MgTripodPhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MgTripodPhase::Packed => "packed",
            MgTripodPhase::Deploying => "deploying",
            MgTripodPhase::Deployed => "deployed",
            MgTripodPhase::Crewed => "crewed",
            MgTripodPhase::Packing => "packing",
        }
    }
}

/// One placed (or packed) `mg_tripod_portable` instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MgTripod {
    pub id: FortificationId,
    pub phase: MgTripodPhase,
    /// Tile position when deployed (`(0, 0)` when packed).
    pub pos: (i32, i32),
    pub hp: u32,
    /// Rounds remaining; preserved across pack/unpack cycles.
    pub rounds_remaining: u32,
    /// Ticks remaining in the current deploy/pack transition. 0 when
    /// the tripod is stable (Packed / Deployed / Crewed).
    pub transition_ticks_remaining: u32,
    /// Crewing actor id when phase == Crewed; None otherwise.
    pub crewed_by: Option<u32>,
}

impl MgTripod {
    /// Build a freshly packed tripod with the full belt loaded.
    #[must_use]
    pub fn new_packed(id: FortificationId) -> Self {
        Self {
            id,
            phase: MgTripodPhase::Packed,
            pos: (0, 0),
            hp: MG_TRIPOD_DEPLOYED_HP,
            rounds_remaining: AMMO_BOX_MG_ROUNDS,
            transition_ticks_remaining: 0,
            crewed_by: None,
        }
    }

    /// Begin the 4-second deploy timer. `tick_rate_hz` is configurable
    /// (project AGENTS.md forbids hardcoding 60). Returns the tick
    /// budget seeded on the timer.
    pub fn start_deploy(
        &mut self,
        pos: (i32, i32),
        tick_rate_hz: u32,
    ) -> Result<u32, MgTripodError> {
        if self.phase != MgTripodPhase::Packed {
            return Err(MgTripodError::NotPacked);
        }
        let ticks = MG_TRIPOD_DEPLOY_SECONDS.saturating_mul(tick_rate_hz);
        self.phase = MgTripodPhase::Deploying;
        self.pos = pos;
        self.transition_ticks_remaining = ticks;
        Ok(ticks)
    }

    /// Begin the 4-second pack timer. The tripod must be Deployed
    /// (un-crewed) — packing while crewed is rejected so the engine
    /// can force an uncrew first.
    pub fn start_pack(&mut self, tick_rate_hz: u32) -> Result<u32, MgTripodError> {
        match self.phase {
            MgTripodPhase::Deployed => {}
            MgTripodPhase::Crewed => return Err(MgTripodError::StillCrewed),
            _ => return Err(MgTripodError::NotDeployed),
        }
        let ticks = MG_TRIPOD_DEPLOY_SECONDS.saturating_mul(tick_rate_hz);
        self.phase = MgTripodPhase::Packing;
        self.transition_ticks_remaining = ticks;
        Ok(ticks)
    }

    /// Tick the deploy/pack timer; transitions phase when the timer
    /// reaches 0. Returns true on the tick the phase actually changed
    /// (engine emits `mg_tripod_deployed` / `mg_tripod_packed` on that
    /// edge).
    pub fn tick_transition(&mut self) -> Option<MgTripodPhase> {
        if self.transition_ticks_remaining == 0 {
            return None;
        }
        self.transition_ticks_remaining -= 1;
        if self.transition_ticks_remaining > 0 {
            return None;
        }
        let next = match self.phase {
            MgTripodPhase::Deploying => MgTripodPhase::Deployed,
            MgTripodPhase::Packing => MgTripodPhase::Packed,
            other => return Some(other),
        };
        self.phase = next;
        if matches!(next, MgTripodPhase::Packed) {
            // Packed tripod has no on-map position; preserve rounds for
            // VAL-M9C-018 bit-equal claim.
            self.pos = (0, 0);
        }
        Some(next)
    }

    /// Crew the deployed tripod. Returns Err when the tripod is in any
    /// phase other than Deployed.
    pub fn crew(&mut self, actor_id: u32) -> Result<(), MgTripodError> {
        if self.phase != MgTripodPhase::Deployed {
            return Err(MgTripodError::NotDeployed);
        }
        self.phase = MgTripodPhase::Crewed;
        self.crewed_by = Some(actor_id);
        Ok(())
    }

    /// Uncrew the deployed tripod (Crewed → Deployed).
    pub fn uncrew(&mut self) -> Option<u32> {
        if self.phase != MgTripodPhase::Crewed {
            return None;
        }
        self.phase = MgTripodPhase::Deployed;
        self.crewed_by.take()
    }

    /// Fire `rounds` through the tripod; consumes from
    /// `rounds_remaining`. Returns the fire outcome for the recorder.
    pub fn fire_rounds(&mut self, rounds: u32) -> MgNestFireOutcome {
        if self.phase != MgTripodPhase::Crewed {
            return MgNestFireOutcome::default();
        }
        let to_fire = rounds.min(self.rounds_remaining);
        self.rounds_remaining -= to_fire;
        let depleted = self.rounds_remaining == 0 && to_fire > 0;
        MgNestFireOutcome {
            rounds_fired: to_fire,
            ammo_box_depleted: depleted,
        }
    }
}

/// Typed error for `MgTripod` transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MgTripodError {
    NotPacked,
    NotDeployed,
    StillCrewed,
}

impl MgTripodError {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MgTripodError::NotPacked => "mg_tripod_not_packed",
            MgTripodError::NotDeployed => "mg_tripod_not_deployed",
            MgTripodError::StillCrewed => "mg_tripod_still_crewed",
        }
    }
}

/// One placed standalone `spotter_scope` fortification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpotterScope {
    pub id: FortificationId,
    pub hp: u32,
}

impl SpotterScope {
    #[must_use]
    pub const fn new(id: FortificationId) -> Self {
        Self {
            id,
            hp: SPOTTER_SCOPE_HP,
        }
    }

    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        self.hp == 0
    }
}

/// Resolve the spotter-scope acquisition multiplier for a crewed
/// MG / sniper actor.
///
///
/// > standalone `spotter_scope` grants +50% acquisition to adjacent
/// > crewed MG / sniper (HP 100; destruction removes bonus).
///
/// Returns 1.5 when the scope is adjacent + intact; 1.0 otherwise.
/// Used by cf-ai target-selection to scale the per-actor acquisition
/// rate.
#[must_use]
pub fn spotter_scope_acquisition_multiplier(
    scope: Option<&SpotterScope>,
    adjacent: bool,
) -> f32 {
    match scope {
        Some(s) if adjacent && !s.is_destroyed() => SPOTTER_SCOPE_ACQUISITION_MULTIPLIER,
        _ => 1.0,
    }
}

/// One placed `bunker_firing_slit` instance — concrete-embedded
/// horizontal slit with an 8-px aperture. Spec § "Bunker firing slit":
///
/// > Bunker firing slit is pre-built into M28F bunker templates; HP
/// > 1600 vs HP 800 for a standalone MG nest. A penetrating round
/// > (HEAT, M14C) through the slit reaches the crewed actor; rounds
/// > that hit the surrounding concrete bounce harmlessly off the
/// > parapet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BunkerFiringSlit {
    pub id: FortificationId,
    pub hp: u32,
    /// Tile origin of the slit (top-left of the 2×1 footprint).
    pub tile_origin: (i32, i32),
    /// Pixel-y of the aperture top inside the 1-tile-tall band.
    pub aperture_top_y_px: u32,
    /// Pixel-y of the aperture bottom (exclusive).
    pub aperture_bottom_y_px: u32,
}

impl BunkerFiringSlit {
    /// Build a default slit with the spec's 8-px aperture centered in
    /// the 16-px-tall tile band (aperture rows 4..12).
    #[must_use]
    pub fn new(id: FortificationId, tile_origin: (i32, i32)) -> Self {
        let band_height_px = 16u32;
        let aperture_top = (band_height_px - BUNKER_FIRING_SLIT_APERTURE_PX) / 2;
        Self {
            id,
            hp: BUNKER_FIRING_SLIT_HP,
            tile_origin,
            aperture_top_y_px: aperture_top,
            aperture_bottom_y_px: aperture_top + BUNKER_FIRING_SLIT_APERTURE_PX,
        }
    }

    /// True when `pixel_y` (relative to the slit's top edge) falls
    /// inside the 8-px aperture.
    #[must_use]
    pub const fn pixel_through_aperture(self, pixel_y: u32) -> bool {
        pixel_y >= self.aperture_top_y_px && pixel_y < self.aperture_bottom_y_px
    }

    /// True when the slit's per-pixel concrete HP has been drained.
    #[must_use]
    pub const fn is_destroyed(self) -> bool {
        self.hp == 0
    }
}

/// Damage round-kinds supported by [`route_bunker_slit_damage`]. The
/// M14C HEAT/APFSDS taxonomy is wider but the slit only cares about
/// the bypass-concrete vs blocked-by-concrete distinction.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BunkerSlitRoundKind {
    /// Standard small-arms / MG rifle round (blocked by concrete).
    SmallArms = 0,
    /// HEAT / shaped-charge round (penetrates concrete per M14C; the
    /// spec calls this out: "HEAT through aperture reaches crew").
    Heat = 1,
}

/// Result of routing a single incoming round at a `bunker_firing_slit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BunkerSlitDamageResult {
    /// Damage delivered to the slit's per-pixel concrete material.
    pub slit_hp_decrement: u32,
    /// Damage delivered to the crewing actor's body graph.
    pub actor_damage: u32,
    /// True when the round bounced off the surrounding concrete (no
    /// damage anywhere).
    pub bounced: bool,
}

impl BunkerSlitDamageResult {
    #[must_use]
    pub const fn bounced() -> Self {
        Self {
            slit_hp_decrement: 0,
            actor_damage: 0,
            bounced: true,
        }
    }
}

/// Route one incoming round at a bunker firing slit per the spec's
/// per-aperture damage routing:
///
/// 1. Round outside aperture + small-arms → bounces off concrete; the
///    slit's HP only decrements on a penetrating hit.
/// 2. Round inside aperture + small-arms → delivers full damage to
///    the crewing actor's body graph.
/// 3. Round outside aperture + HEAT → penetrates the concrete; the
///    slit's per-pixel HP decrements AND damage continues to the
///    crewing actor (M14C HEAT semantics).
/// 4. Round inside aperture + HEAT → same as case 2 (HEAT through
///    aperture reaches crew without per-pixel concrete decrement).
///
/// `damage` is the per-hit projectile energy in HP-equivalents.
pub fn route_bunker_slit_damage(
    slit: &BunkerFiringSlit,
    pixel_y: u32,
    kind: BunkerSlitRoundKind,
    damage: u32,
) -> BunkerSlitDamageResult {
    let through_aperture = slit.pixel_through_aperture(pixel_y);
    match (through_aperture, kind) {
        // Outside aperture + small arms → bounce (zero damage anywhere).
        (false, BunkerSlitRoundKind::SmallArms) => BunkerSlitDamageResult::bounced(),
        // Inside aperture → damage reaches the crewing actor; slit HP
        // remains intact (the aperture is the opening, not concrete).
        (true, _) => BunkerSlitDamageResult {
            slit_hp_decrement: 0,
            actor_damage: damage,
            bounced: false,
        },
        // Outside aperture + HEAT → penetrates concrete; slit HP
        // decrements + crew takes residual damage.
        (false, BunkerSlitRoundKind::Heat) => BunkerSlitDamageResult {
            slit_hp_decrement: damage,
            actor_damage: damage,
            bounced: false,
        },
    }
}

/// Spec § "MG nest crewing": when crewed, the actor's personal weapon
/// is suspended and firing is rebound to the mounted MG.
///
/// > primary fire input now controls the MG (90° firing arc + 360°
/// > base traverse) … the personal weapon is suspended (visible
/// > weapon icon switches to MG)
///
/// This helper returns the firing-binding for the actor at the
/// current tick. cf-control consumes the [`FireBinding`] to choose
/// whether to dispatch the player's personal-weapon fire (the M1
/// rifle path) or the mounted-MG fire (the cf-fortification path).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FireBinding {
    /// Normal: route fire to the actor's personal weapon.
    PersonalWeapon,
    /// Rebound: route fire through the bound MG nest.
    MgNest(FortificationId),
    /// Rebound: route fire through the bound tripod.
    MgTripod(FortificationId),
    /// Rebound: route fire through the bound bunker firing slit.
    BunkerSlit(FortificationId),
}

impl FireBinding {
    /// True when the personal weapon is suspended (i.e., fire is
    /// rebound to a mounted fortification weapon). Equivalent to the
    /// spec's "personal_weapon.suspended==true" flag.
    #[must_use]
    pub const fn personal_weapon_suspended(self) -> bool {
        !matches!(self, FireBinding::PersonalWeapon)
    }
}

/// Compute the per-actor fire-binding from the actor's
/// `crewing_fortification_id` and the kind of fortification it's bound
/// to. cf-control's engine layer threads the lookup.
#[must_use]
pub fn fire_binding_for(
    crewing_id: Option<FortificationId>,
    kind: Option<CrewedKind>,
) -> FireBinding {
    match (crewing_id, kind) {
        (Some(id), Some(CrewedKind::MgNest)) => FireBinding::MgNest(id),
        (Some(id), Some(CrewedKind::MgTripod)) => FireBinding::MgTripod(id),
        (Some(id), Some(CrewedKind::BunkerSlit)) => FireBinding::BunkerSlit(id),
        _ => FireBinding::PersonalWeapon,
    }
}

/// What kind of fortification an actor is crewing. Used to dispatch
/// the right `FireBinding` variant.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrewedKind {
    MgNest = 0,
    MgTripod = 1,
    BunkerSlit = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mg_nest_static_construction_pins_spec_table_row() {
        let nest = MgNest::new_built(FortificationId(42));
        assert_eq!(nest.hp, MG_NEST_STATIC_MAX_HP);
        let ammo = nest.ammo_box.as_ref().unwrap();
        assert_eq!(ammo.rounds_remaining, AMMO_BOX_MG_ROUNDS);
        assert!(!nest.is_crewed());
    }

    /// mounted MG. The personal weapon is suspended; the fortification
    /// records each `fire_rounds` call decrementing rounds_remaining.
    #[test]
    fn mg_nest_primary_fire_rebound() {
        let mut nest = MgNest::new_built(FortificationId(1));
        nest.crew(101).expect("crew empty nest");
        assert!(nest.is_crewed());

        // FireBinding for a crewed actor → MgNest variant, personal
        // weapon suspended.
        let binding = fire_binding_for(Some(nest.id), Some(CrewedKind::MgNest));
        assert_eq!(binding, FireBinding::MgNest(nest.id));
        assert!(binding.personal_weapon_suspended());

        // Firing 5 rounds decrements the bound ammo box.
        let outcome = nest.fire_rounds(5);
        assert_eq!(outcome.rounds_fired, 5);
        assert!(!outcome.ammo_box_depleted);
        assert_eq!(
            nest.ammo_box.as_ref().unwrap().rounds_remaining,
            AMMO_BOX_MG_ROUNDS - 5
        );

        // emits ammo_box_depleted on the round that finishes the box.
        let outcome = nest.fire_rounds(AMMO_BOX_MG_ROUNDS - 5);
        assert_eq!(outcome.rounds_fired, AMMO_BOX_MG_ROUNDS - 5);
        assert!(outcome.ammo_box_depleted);
        assert_eq!(nest.ammo_box.as_ref().unwrap().rounds_remaining, 0);

        // Subsequent fire is a no-op until swap.
        let outcome = nest.fire_rounds(10);
        assert_eq!(outcome.rounds_fired, 0);
        assert!(!outcome.ammo_box_depleted);

        nest.swap_ammo_box();
        assert_eq!(
            nest.ammo_box.as_ref().unwrap().rounds_remaining,
            AMMO_BOX_MG_ROUNDS
        );
        let outcome = nest.fire_rounds(3);
        assert_eq!(outcome.rounds_fired, 3);

        // When un-crewed the nest cannot fire even with ammo present.
        let evt = nest.uncrew(MgNestUncrewReason::Voluntary, 100);
        assert!(evt.is_some());
        assert_eq!(evt.unwrap().reason, MgNestUncrewReason::Voluntary);
        let outcome = nest.fire_rounds(1);
        assert_eq!(outcome.rounds_fired, 0);
    }

    /// `mg_nest_uncrewed` event with the correct cause.
    #[test]
    fn mg_nest_uncrewed_emits_all_three_causes() {
        // Voluntary uncrew via cfctl.
        let mut nest = MgNest::new_built(FortificationId(1));
        nest.crew(10).unwrap();
        let evt = nest.uncrew(MgNestUncrewReason::Voluntary, 11).unwrap();
        assert_eq!(evt.reason, MgNestUncrewReason::Voluntary);
        assert_eq!(evt.nest_id, nest.id);
        assert_eq!(evt.actor_id, Some(10));

        // Actor death.
        let mut nest = MgNest::new_built(FortificationId(2));
        nest.crew(20).unwrap();
        let evt = nest.on_crewing_actor_killed(50).unwrap();
        assert_eq!(evt.reason, MgNestUncrewReason::ActorKilled);
        assert_eq!(evt.actor_id, Some(20));

        // Nest destroyed.
        let mut nest = MgNest::new_built(FortificationId(3));
        nest.crew(30).unwrap();
        let evt = nest.apply_damage(MG_NEST_STATIC_MAX_HP + 100, 99).unwrap();
        assert_eq!(evt.reason, MgNestUncrewReason::NestDestroyed);
        assert_eq!(evt.actor_id, Some(30));
        assert!(nest.is_destroyed());
        assert!(!nest.is_crewed());
    }

    /// `rounds_remaining` bit-equal across the cycle.
    #[test]
    fn mg_tripod_deploy_crew_pack_preserves_ammo() {
        let tick_rate_hz = 60u32;
        let mut tripod = MgTripod::new_packed(FortificationId(1));
        assert_eq!(tripod.phase, MgTripodPhase::Packed);
        let initial_ammo = tripod.rounds_remaining;

        // Deploy.
        let ticks = tripod.start_deploy((10, 5), tick_rate_hz).unwrap();
        assert_eq!(ticks, MG_TRIPOD_DEPLOY_SECONDS * tick_rate_hz);
        assert_eq!(tripod.phase, MgTripodPhase::Deploying);

        for _ in 0..ticks - 1 {
            assert!(tripod.tick_transition().is_none());
        }
        let final_phase = tripod.tick_transition();
        assert_eq!(final_phase, Some(MgTripodPhase::Deployed));
        assert_eq!(tripod.phase, MgTripodPhase::Deployed);
        assert_eq!(tripod.pos, (10, 5));
        assert_eq!(tripod.rounds_remaining, initial_ammo);

        // Crew + fire 60 rounds (per VAL-M9C-018).
        tripod.crew(7).unwrap();
        assert_eq!(tripod.phase, MgTripodPhase::Crewed);
        let outcome = tripod.fire_rounds(60);
        assert_eq!(outcome.rounds_fired, 60);
        let after_fire_ammo = tripod.rounds_remaining;
        assert_eq!(after_fire_ammo, initial_ammo - 60);

        // Uncrew + pack.
        let actor = tripod.uncrew().unwrap();
        assert_eq!(actor, 7);
        let ticks = tripod.start_pack(tick_rate_hz).unwrap();
        assert_eq!(ticks, MG_TRIPOD_DEPLOY_SECONDS * tick_rate_hz);
        for _ in 0..ticks - 1 {
            tripod.tick_transition();
        }
        let final_phase = tripod.tick_transition();
        assert_eq!(final_phase, Some(MgTripodPhase::Packed));
        assert_eq!(tripod.phase, MgTripodPhase::Packed);

        // Ammo bit-equal across the cycle.
        assert_eq!(tripod.rounds_remaining, after_fire_ammo);
    }

    #[test]
    fn mg_tripod_pack_rejected_while_crewed() {
        let mut tripod = MgTripod::new_packed(FortificationId(1));
        let ticks = tripod.start_deploy((0, 0), 1).unwrap();
        // Walk every tick of the deploy timer so the tripod reaches
        // Deployed; only then is crew + pack reachable.
        for _ in 0..ticks {
            let _ = tripod.tick_transition();
        }
        assert_eq!(tripod.phase, MgTripodPhase::Deployed);
        tripod.crew(1).unwrap();
        assert_eq!(tripod.start_pack(1), Err(MgTripodError::StillCrewed));
    }

    /// through to the crewing actor; rounds outside the aperture
    /// bounce off the surrounding concrete with zero damage to crew
    /// AND zero HP decrement to the slit.
    #[test]
    fn bunker_firing_slit_damage_routing() {
        let slit = BunkerFiringSlit::new(FortificationId(99), (4, 0));
        assert_eq!(slit.hp, BUNKER_FIRING_SLIT_HP);
        // Default aperture rows 4..12 inside the 16-px band.
        assert!(slit.pixel_through_aperture(4));
        assert!(slit.pixel_through_aperture(11));
        assert!(!slit.pixel_through_aperture(3));
        assert!(!slit.pixel_through_aperture(12));

        // Case 1: round outside aperture + small-arms → bounce.
        let r = route_bunker_slit_damage(&slit, 2, BunkerSlitRoundKind::SmallArms, 80);
        assert!(r.bounced);
        assert_eq!(r.slit_hp_decrement, 0);
        assert_eq!(r.actor_damage, 0);

        // Case 2: round inside aperture + small-arms → reaches crew.
        let r = route_bunker_slit_damage(&slit, 7, BunkerSlitRoundKind::SmallArms, 80);
        assert!(!r.bounced);
        assert_eq!(r.slit_hp_decrement, 0);
        assert_eq!(r.actor_damage, 80);

        // Case 3: HEAT outside aperture → penetrates concrete; slit HP
        // decrements AND crew takes damage (M14C).
        let r = route_bunker_slit_damage(&slit, 2, BunkerSlitRoundKind::Heat, 400);
        assert!(!r.bounced);
        assert_eq!(r.slit_hp_decrement, 400);
        assert_eq!(r.actor_damage, 400);

        // Case 4: HEAT through aperture → reaches crew without
        // decrementing the slit HP (the aperture is the opening, not
        // concrete).
        let r = route_bunker_slit_damage(&slit, 6, BunkerSlitRoundKind::Heat, 400);
        assert!(!r.bounced);
        assert_eq!(r.slit_hp_decrement, 0);
        assert_eq!(r.actor_damage, 400);
    }

    /// +50% acquisition to an adjacent crewed actor; destruction
    /// removes the bonus.
    #[test]
    fn spotter_scope_grants_fifty_percent_when_adjacent_and_intact() {
        let scope = SpotterScope::new(FortificationId(1));
        assert_eq!(scope.hp, SPOTTER_SCOPE_HP);
        // Adjacent + intact → 1.5×.
        let mult = spotter_scope_acquisition_multiplier(Some(&scope), true);
        assert!((mult - 1.5).abs() < f32::EPSILON);
        // Non-adjacent → baseline 1.0.
        let mult = spotter_scope_acquisition_multiplier(Some(&scope), false);
        assert!((mult - 1.0).abs() < f32::EPSILON);

        // Destroyed scope (HP=0) drops the bonus even when adjacent.
        let mut destroyed = SpotterScope::new(FortificationId(2));
        destroyed.hp = 0;
        let mult = spotter_scope_acquisition_multiplier(Some(&destroyed), true);
        assert!((mult - 1.0).abs() < f32::EPSILON);

        // No scope at all → 1.0.
        let mult = spotter_scope_acquisition_multiplier(None, true);
        assert!((mult - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fire_binding_personal_weapon_suspended() {
        assert!(!FireBinding::PersonalWeapon.personal_weapon_suspended());
        assert!(FireBinding::MgNest(FortificationId(1)).personal_weapon_suspended());
        assert!(FireBinding::MgTripod(FortificationId(2)).personal_weapon_suspended());
        assert!(FireBinding::BunkerSlit(FortificationId(3)).personal_weapon_suspended());
    }

    #[test]
    fn mg_nest_crew_rejects_when_already_crewed() {
        let mut nest = MgNest::new_built(FortificationId(1));
        nest.crew(1).unwrap();
        assert_eq!(nest.crew(2), Err(MgNestError::AlreadyCrewed));
    }

    #[test]
    fn mg_nest_uncrew_when_already_empty_returns_none() {
        let mut nest = MgNest::new_built(FortificationId(1));
        assert!(nest.uncrew(MgNestUncrewReason::Voluntary, 1).is_none());
    }

    #[test]
    fn mg_nest_uncrew_reason_as_str_matches_schema_enum() {
        assert_eq!(MgNestUncrewReason::Voluntary.as_str(), "voluntary");
        assert_eq!(MgNestUncrewReason::ActorKilled.as_str(), "actor_killed");
        assert_eq!(MgNestUncrewReason::NestDestroyed.as_str(), "nest_destroyed");
    }
}
