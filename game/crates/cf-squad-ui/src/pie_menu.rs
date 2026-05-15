//! T-key pie menu — 8-slice radial action wheel per spec § Pie menu —
//! context-sensitive radial actions (M1 reserves; M8 fills baseline).
//!
//! Distinct from the Q-hold context wheel: the context wheel issues squad
//! orders to bots based on what's under the reticle; the pie menu fires
//! the PLAYER's own 8 actor actions (pickup / drop / switch weapon / throw
//! grenade / melee bash / deploy bipod / signal squad / use medkit).
//!
//! Spec-mandated 8 slices + 6 reason labels for invalid selection.
//!
//! Sim slowdown gate (per spec): in single-player, opening the pie menu
//! slows sim to [`SINGLE_PLAYER_SLOWDOWN_PCT`] of normal speed (cosmetic
//! UX — sim ticks remain deterministic for replay). In multiplayer, no
//! slowdown.

use serde::{Deserialize, Serialize};

/// Number of slices in the radial wheel.
pub const PIE_MENU_SLICES_LEN: usize = 8;

/// Single-player sim-speed percentage while the pie menu is open. Sim
/// ticks continue to advance deterministically; this field is read by
/// cf-app to scale wall-clock-to-tick pacing for a slow-motion UX feel.
pub const SINGLE_PLAYER_SLOWDOWN_PCT: u8 = 20;

/// Multiplayer sim-speed percentage while the pie menu is open (no
/// slowdown — opening the menu must not stall the networked sim).
pub const MULTIPLAYER_NO_SLOWDOWN_PCT: u8 = 100;

/// What the player was looking at / closest to when the pie menu opened.
/// Determines which slices are valid + which reason labels surface for
/// disabled slices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PieMenuTarget {
    /// No interactable in range. Pickup + Drop disabled; SwitchWeapon /
    /// ThrowGrenade / MeleeBash / DeployBipod / SignalSquad / UseMedkit
    /// may still apply.
    Void,
    /// Nearest interactable actor (squadmate, downed ally, enemy).
    NearestActor {
        /// Actor id in range of the player.
        actor_id: u64,
    },
    /// A door interactable (open / breach context).
    Door {
        /// Door entity id.
        entity_id: u64,
    },
    /// A dropped item the player can pick up.
    Item {
        /// Item entity id.
        entity_id: u64,
    },
}

impl PieMenuTarget {
    /// Canonical snake_case identifier of the variant kind.
    pub fn kind_str(&self) -> &'static str {
        match self {
            PieMenuTarget::Void => "void",
            PieMenuTarget::NearestActor { .. } => "nearest_actor",
            PieMenuTarget::Door { .. } => "door",
            PieMenuTarget::Item { .. } => "item",
        }
    }

    /// Numeric id payload (when the variant carries one).
    pub fn target_id(&self) -> Option<u64> {
        match *self {
            PieMenuTarget::Void => None,
            PieMenuTarget::NearestActor { actor_id } => Some(actor_id),
            PieMenuTarget::Door { entity_id } | PieMenuTarget::Item { entity_id } => Some(entity_id),
        }
    }

    /// Parse the cfctl wire form. `target_id` is required for non-Void
    /// kinds and ignored otherwise.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(kind: &str, target_id: Option<u64>) -> Option<PieMenuTarget> {
        Some(match kind {
            "void" => PieMenuTarget::Void,
            "nearest_actor" => PieMenuTarget::NearestActor { actor_id: target_id? },
            "door" => PieMenuTarget::Door { entity_id: target_id? },
            "item" => PieMenuTarget::Item { entity_id: target_id? },
            _ => return None,
        })
    }
}

/// One of the 8 default slices per spec § Pie menu. M13+ extends with
/// chassis-specific slices (eject pilot / repair chassis / overclock /
/// cockpit camera anchor / ability slots / brain_hop); M8 ships the
/// baseline 8.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PieMenuSlice {
    /// Slot 0 — pickup the item under the reticle / nearest in-range.
    Pickup,
    /// Slot 1 — drop the currently-held item.
    Drop,
    /// Slot 2 — cycle to the next weapon slot (1..=8).
    SwitchWeapon,
    /// Slot 3 — throw the equipped grenade.
    ThrowGrenade,
    /// Slot 4 — melee bash with the held weapon.
    MeleeBash,
    /// Slot 5 — deploy bipod (requires crouch/prone + bipod-compatible
    /// weapon).
    DeployBipod,
    /// Slot 6 — signal squadmates (signal_friendly + mark_waypoint).
    SignalSquad,
    /// Slot 7 — use medkit on self.
    UseMedkit,
}

impl PieMenuSlice {
    /// Spec-mandated 8 default slices in slot order (0..=7).
    pub const ALL: [PieMenuSlice; PIE_MENU_SLICES_LEN] = [
        PieMenuSlice::Pickup,
        PieMenuSlice::Drop,
        PieMenuSlice::SwitchWeapon,
        PieMenuSlice::ThrowGrenade,
        PieMenuSlice::MeleeBash,
        PieMenuSlice::DeployBipod,
        PieMenuSlice::SignalSquad,
        PieMenuSlice::UseMedkit,
    ];

    /// Canonical snake_case identifier (cfctl wire form).
    pub fn as_str(self) -> &'static str {
        match self {
            PieMenuSlice::Pickup => "pickup",
            PieMenuSlice::Drop => "drop",
            PieMenuSlice::SwitchWeapon => "switch_weapon",
            PieMenuSlice::ThrowGrenade => "throw_grenade",
            PieMenuSlice::MeleeBash => "melee_bash",
            PieMenuSlice::DeployBipod => "deploy_bipod",
            PieMenuSlice::SignalSquad => "signal_squad",
            PieMenuSlice::UseMedkit => "use_medkit",
        }
    }

    /// Player-facing label (mirrors the en localization key
    /// `pie_menu.slice.<id>`).
    pub fn label(self) -> &'static str {
        match self {
            PieMenuSlice::Pickup => "Pickup",
            PieMenuSlice::Drop => "Drop",
            PieMenuSlice::SwitchWeapon => "Switch Weapon",
            PieMenuSlice::ThrowGrenade => "Throw Grenade",
            PieMenuSlice::MeleeBash => "Melee Bash",
            PieMenuSlice::DeployBipod => "Deploy Bipod",
            PieMenuSlice::SignalSquad => "Signal Squad",
            PieMenuSlice::UseMedkit => "Use Medkit",
        }
    }

    /// Localization key for the slice label (used by cf-localization).
    pub fn localization_key(self) -> &'static str {
        match self {
            PieMenuSlice::Pickup => "pie_menu.slice.pickup",
            PieMenuSlice::Drop => "pie_menu.slice.drop",
            PieMenuSlice::SwitchWeapon => "pie_menu.slice.switch_weapon",
            PieMenuSlice::ThrowGrenade => "pie_menu.slice.throw_grenade",
            PieMenuSlice::MeleeBash => "pie_menu.slice.melee_bash",
            PieMenuSlice::DeployBipod => "pie_menu.slice.deploy_bipod",
            PieMenuSlice::SignalSquad => "pie_menu.slice.signal_squad",
            PieMenuSlice::UseMedkit => "pie_menu.slice.use_medkit",
        }
    }

    /// The slot index this slice occupies (0..=7).
    pub fn slot(self) -> u8 {
        self as u8
    }

    /// Resolve a slice from a 0..=7 slot index.
    pub fn from_slot(slot: u8) -> Option<PieMenuSlice> {
        PieMenuSlice::ALL.get(slot as usize).copied()
    }

    /// Parse from cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<PieMenuSlice> {
        Some(match value {
            "pickup" => PieMenuSlice::Pickup,
            "drop" => PieMenuSlice::Drop,
            "switch_weapon" => PieMenuSlice::SwitchWeapon,
            "throw_grenade" => PieMenuSlice::ThrowGrenade,
            "melee_bash" => PieMenuSlice::MeleeBash,
            "deploy_bipod" => PieMenuSlice::DeployBipod,
            "signal_squad" => PieMenuSlice::SignalSquad,
            "use_medkit" => PieMenuSlice::UseMedkit,
            _ => return None,
        })
    }
}

/// Why a slice was rejected on selection per spec § Pie menu — Per-slice
/// reason labels for disabled actions. CCCP ref `Controller.cpp:280` +
/// `AHuman.cpp:2962-3020`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PieMenuReason {
    /// "No Arm" — Pickup rejected when the arm is destroyed.
    NoArm,
    /// "Not Holding Anything" — Drop rejected when no item is held in
    /// the active slot, OR Switch Weapon rejected when there is no
    /// other weapon to swap to.
    NotHoldingAnything,
    /// "Needs Digger" — context tool refusal (slice requires the digger
    /// to be equipped / unstowed).
    NeedsDigger,
    /// "Out of Range" — context proximity refusal (target outside the
    /// slice's effective range).
    OutOfRange,
    /// "No Grenade Equipped" — Throw Grenade rejected when no grenade
    /// is loaded in inventory.
    NoGrenadeEquipped,
    /// "Standing — Requires Crouch/Prone" — Deploy Bipod rejected when
    /// the actor's stance does not permit bipod deployment.
    Standing,
}

impl PieMenuReason {
    /// Spec-mandated 6 reason labels.
    pub const ALL: [PieMenuReason; 6] = [
        PieMenuReason::NoArm,
        PieMenuReason::NotHoldingAnything,
        PieMenuReason::NeedsDigger,
        PieMenuReason::OutOfRange,
        PieMenuReason::NoGrenadeEquipped,
        PieMenuReason::Standing,
    ];

    /// Canonical snake_case identifier (cfctl wire form).
    pub fn as_str(self) -> &'static str {
        match self {
            PieMenuReason::NoArm => "no_arm",
            PieMenuReason::NotHoldingAnything => "not_holding_anything",
            PieMenuReason::NeedsDigger => "needs_digger",
            PieMenuReason::OutOfRange => "out_of_range",
            PieMenuReason::NoGrenadeEquipped => "no_grenade_equipped",
            PieMenuReason::Standing => "standing",
        }
    }

    /// Player-facing label string (mirrors the en localization key
    /// `pie_menu.slice.disabled.<id>`).
    pub fn label(self) -> &'static str {
        match self {
            PieMenuReason::NoArm => "No Arm",
            PieMenuReason::NotHoldingAnything => "Not Holding Anything",
            PieMenuReason::NeedsDigger => "Needs Digger",
            PieMenuReason::OutOfRange => "Out of Range",
            PieMenuReason::NoGrenadeEquipped => "No Grenade Equipped",
            PieMenuReason::Standing => "Standing — Requires Crouch/Prone",
        }
    }

    /// Localization key for the reason label.
    pub fn localization_key(self) -> &'static str {
        match self {
            PieMenuReason::NoArm => "pie_menu.slice.disabled.no_arm",
            PieMenuReason::NotHoldingAnything => "pie_menu.slice.disabled.not_holding_anything",
            PieMenuReason::NeedsDigger => "pie_menu.slice.disabled.needs_digger",
            PieMenuReason::OutOfRange => "pie_menu.slice.disabled.out_of_range",
            PieMenuReason::NoGrenadeEquipped => "pie_menu.slice.disabled.no_grenade_equipped",
            PieMenuReason::Standing => "pie_menu.slice.disabled.standing",
        }
    }

    /// Parse from cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<PieMenuReason> {
        Some(match value {
            "no_arm" => PieMenuReason::NoArm,
            "not_holding_anything" => PieMenuReason::NotHoldingAnything,
            "needs_digger" => PieMenuReason::NeedsDigger,
            "out_of_range" => PieMenuReason::OutOfRange,
            "no_grenade_equipped" => PieMenuReason::NoGrenadeEquipped,
            "standing" => PieMenuReason::Standing,
            _ => return None,
        })
    }
}

/// Engine-side pie menu state. cf-control owns one instance per session.
/// While `open=true`, the cf-app keyboard/mouse layer routes T-key inputs
/// to slice rotation + Enter routes to `act.player.pie_menu_select`. The
/// `slowdown_factor_pct` field is read by cf-app's pacing loop to scale
/// wall-clock pacing while the menu is open (cosmetic; sim ticks remain
/// deterministic for replay).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieMenuState {
    /// Whether the menu is currently open.
    pub open: bool,
    /// What the player was looking at when the menu opened.
    pub target: PieMenuTarget,
    /// Tick at which the menu opened (so the engine can compute
    /// open-duration for analytics + auto-close timeouts).
    pub open_tick: Option<u64>,
    /// Currently-highlighted slot under the cursor (0..=7), if any.
    pub slice_under_cursor: Option<u8>,
    /// Sim-speed percentage while the menu is open: 20 in single-player
    /// (heavy slowdown), 100 in multiplayer (no slowdown).
    pub slowdown_factor_pct: u8,
    /// Total times the pie menu has been opened this session.
    pub open_count: u32,
}

impl Default for PieMenuState {
    fn default() -> Self {
        Self::closed()
    }
}

impl PieMenuState {
    /// Build the closed-state default.
    pub fn closed() -> Self {
        Self {
            open: false,
            target: PieMenuTarget::Void,
            open_tick: None,
            slice_under_cursor: None,
            slowdown_factor_pct: MULTIPLAYER_NO_SLOWDOWN_PCT,
            open_count: 0,
        }
    }

    /// Open the menu for `target` at `current_tick`. `multiplayer`
    /// selects the spec-mandated slowdown factor (single-player slows to
    /// [`SINGLE_PLAYER_SLOWDOWN_PCT`]; multiplayer stays at
    /// [`MULTIPLAYER_NO_SLOWDOWN_PCT`]). Returns `true` if the menu
    /// transitioned from closed → open; `false` if it was already open
    /// (idempotent).
    pub fn open(&mut self, target: PieMenuTarget, multiplayer: bool, current_tick: u64) -> bool {
        if self.open {
            return false;
        }
        self.open = true;
        self.target = target;
        self.open_tick = Some(current_tick);
        self.slice_under_cursor = None;
        self.slowdown_factor_pct = if multiplayer {
            MULTIPLAYER_NO_SLOWDOWN_PCT
        } else {
            SINGLE_PLAYER_SLOWDOWN_PCT
        };
        self.open_count = self.open_count.saturating_add(1);
        true
    }

    /// Move the cursor highlight to `slot`. Returns `false` if `slot`
    /// is out of range or the menu is closed.
    pub fn hover(&mut self, slot: u8) -> bool {
        if !self.open || (slot as usize) >= PIE_MENU_SLICES_LEN {
            return false;
        }
        self.slice_under_cursor = Some(slot);
        true
    }

    /// Attempt to select `slot`. Returns `Ok(slice)` on a valid
    /// selection; `Err(PieMenuReason)` on a rejected one. The reason
    /// comes from the caller — typically cf-control's dispatcher
    /// resolves it from actor state (arm destroyed → `NoArm`, empty
    /// inventory slot → `NotHoldingAnything`, etc.). The state machine
    /// itself only validates the slot is in range + the menu is open.
    pub fn select(&mut self, slot: u8, reason: Option<PieMenuReason>) -> Result<PieMenuSlice, PieMenuSelectError> {
        if !self.open {
            return Err(PieMenuSelectError::NotOpen);
        }
        let slice = PieMenuSlice::from_slot(slot).ok_or(PieMenuSelectError::InvalidSlot(slot))?;
        if let Some(r) = reason {
            return Err(PieMenuSelectError::Rejected { slice, reason: r });
        }
        Ok(slice)
    }

    /// Close the menu (idempotent). Returns the previous `open` flag.
    pub fn close(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        self.target = PieMenuTarget::Void;
        self.open_tick = None;
        self.slice_under_cursor = None;
        self.slowdown_factor_pct = MULTIPLAYER_NO_SLOWDOWN_PCT;
        was_open
    }

    /// Read the 8 default slices.
    pub fn slices(&self) -> [PieMenuSlice; PIE_MENU_SLICES_LEN] {
        PieMenuSlice::ALL
    }
}

/// Failure modes for [`PieMenuState::select`]. The cfctl dispatcher
/// emits `ux.pie_menu_slice_rejected` for `Rejected` and treats
/// `NotOpen` / `InvalidSlot` as cfctl param-validation rejections
/// (rejected at the server layer, never produce a `slice_rejected`
/// event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PieMenuSelectError {
    /// `select` invoked while the menu is closed.
    NotOpen,
    /// Slot index >= [`PIE_MENU_SLICES_LEN`].
    InvalidSlot(u8),
    /// Slot is in range but the slice is disabled in the current
    /// context. Caller emits `ux.pie_menu_slice_rejected { slice,
    /// reason }`.
    Rejected {
        /// The slice the player tried to fire.
        slice: PieMenuSlice,
        /// Why the slice is currently disabled.
        reason: PieMenuReason,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_default_no_slowdown() {
        let s = PieMenuState::closed();
        assert!(!s.open);
        assert_eq!(s.slowdown_factor_pct, MULTIPLAYER_NO_SLOWDOWN_PCT);
        assert_eq!(s.open_count, 0);
    }

    #[test]
    fn open_single_player_slows_to_20() {
        let mut s = PieMenuState::closed();
        let opened = s.open(PieMenuTarget::Void, false, 100);
        assert!(opened);
        assert!(s.open);
        assert_eq!(s.slowdown_factor_pct, SINGLE_PLAYER_SLOWDOWN_PCT);
        assert_eq!(s.slowdown_factor_pct, 20);
        assert_eq!(s.open_tick, Some(100));
        assert_eq!(s.open_count, 1);
    }

    #[test]
    fn open_multiplayer_no_slowdown() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, true, 50);
        assert_eq!(s.slowdown_factor_pct, MULTIPLAYER_NO_SLOWDOWN_PCT);
        assert_eq!(s.slowdown_factor_pct, 100);
    }

    #[test]
    fn open_twice_is_idempotent() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, false, 1);
        let again = s.open(PieMenuTarget::Void, false, 2);
        assert!(!again);
        assert_eq!(s.open_count, 1);
    }

    #[test]
    fn close_resets_to_no_slowdown() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Door { entity_id: 9 }, false, 10);
        assert!(s.close());
        assert!(!s.open);
        assert_eq!(s.slowdown_factor_pct, MULTIPLAYER_NO_SLOWDOWN_PCT);
        assert_eq!(s.open_tick, None);
    }

    #[test]
    fn close_when_already_closed_is_idempotent() {
        let mut s = PieMenuState::closed();
        assert!(!s.close());
    }

    #[test]
    fn target_kind_and_id_round_trip() {
        let t = PieMenuTarget::NearestActor { actor_id: 7 };
        assert_eq!(t.kind_str(), "nearest_actor");
        assert_eq!(t.target_id(), Some(7));
        let parsed = PieMenuTarget::from_str("nearest_actor", Some(7)).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn target_void_has_no_id() {
        let t = PieMenuTarget::Void;
        assert_eq!(t.kind_str(), "void");
        assert_eq!(t.target_id(), None);
        let parsed = PieMenuTarget::from_str("void", None).unwrap();
        assert_eq!(parsed, t);
    }

    #[test]
    fn target_from_str_rejects_unknown() {
        assert!(PieMenuTarget::from_str("vehicle", Some(1)).is_none());
    }

    #[test]
    fn all_eight_slices_present() {
        assert_eq!(PieMenuSlice::ALL.len(), PIE_MENU_SLICES_LEN);
        assert_eq!(PIE_MENU_SLICES_LEN, 8);
    }

    #[test]
    fn slice_slot_matches_index() {
        for (i, slice) in PieMenuSlice::ALL.iter().enumerate() {
            assert_eq!(slice.slot() as usize, i);
            assert_eq!(PieMenuSlice::from_slot(i as u8), Some(*slice));
        }
    }

    #[test]
    fn slice_from_slot_rejects_out_of_range() {
        assert!(PieMenuSlice::from_slot(8).is_none());
        assert!(PieMenuSlice::from_slot(255).is_none());
    }

    #[test]
    fn slice_str_round_trip() {
        for slice in PieMenuSlice::ALL {
            assert_eq!(PieMenuSlice::from_str(slice.as_str()), Some(slice));
        }
    }

    #[test]
    fn reason_round_trip_all_six() {
        assert_eq!(PieMenuReason::ALL.len(), 6);
        for r in PieMenuReason::ALL {
            assert_eq!(PieMenuReason::from_str(r.as_str()), Some(r));
        }
    }

    #[test]
    fn reason_labels_match_spec() {
        assert_eq!(PieMenuReason::NoArm.label(), "No Arm");
        assert_eq!(PieMenuReason::NotHoldingAnything.label(), "Not Holding Anything");
        assert_eq!(PieMenuReason::NeedsDigger.label(), "Needs Digger");
        assert_eq!(PieMenuReason::OutOfRange.label(), "Out of Range");
        assert_eq!(PieMenuReason::NoGrenadeEquipped.label(), "No Grenade Equipped");
        assert_eq!(PieMenuReason::Standing.label(), "Standing — Requires Crouch/Prone");
    }

    #[test]
    fn slice_labels_match_spec() {
        assert_eq!(PieMenuSlice::Pickup.label(), "Pickup");
        assert_eq!(PieMenuSlice::Drop.label(), "Drop");
        assert_eq!(PieMenuSlice::SwitchWeapon.label(), "Switch Weapon");
        assert_eq!(PieMenuSlice::ThrowGrenade.label(), "Throw Grenade");
        assert_eq!(PieMenuSlice::MeleeBash.label(), "Melee Bash");
        assert_eq!(PieMenuSlice::DeployBipod.label(), "Deploy Bipod");
        assert_eq!(PieMenuSlice::SignalSquad.label(), "Signal Squad");
        assert_eq!(PieMenuSlice::UseMedkit.label(), "Use Medkit");
    }

    #[test]
    fn select_rejected_returns_reason() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, false, 1);
        let res = s.select(3, Some(PieMenuReason::NoGrenadeEquipped));
        assert!(matches!(
            res,
            Err(PieMenuSelectError::Rejected {
                slice: PieMenuSlice::ThrowGrenade,
                reason: PieMenuReason::NoGrenadeEquipped,
            })
        ));
    }

    #[test]
    fn select_invalid_slot_errs() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, false, 1);
        assert!(matches!(s.select(8, None), Err(PieMenuSelectError::InvalidSlot(8))));
    }

    #[test]
    fn select_when_closed_errs() {
        let mut s = PieMenuState::closed();
        assert!(matches!(s.select(0, None), Err(PieMenuSelectError::NotOpen)));
    }

    #[test]
    fn select_valid_returns_slice() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, false, 1);
        let slice = s.select(0, None).unwrap();
        assert_eq!(slice, PieMenuSlice::Pickup);
    }

    #[test]
    fn hover_records_under_cursor() {
        let mut s = PieMenuState::closed();
        s.open(PieMenuTarget::Void, false, 1);
        assert!(s.hover(4));
        assert_eq!(s.slice_under_cursor, Some(4));
    }

    #[test]
    fn hover_when_closed_returns_false() {
        let mut s = PieMenuState::closed();
        assert!(!s.hover(0));
    }

    #[test]
    fn slices_returns_all_eight_in_slot_order() {
        let s = PieMenuState::closed();
        let slices = s.slices();
        assert_eq!(slices[0], PieMenuSlice::Pickup);
        assert_eq!(slices[7], PieMenuSlice::UseMedkit);
    }
}
