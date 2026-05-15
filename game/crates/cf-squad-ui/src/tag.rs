//! MMB tag — non-intrusive priority signal. Tagged entities feed the
//! Utility scorer as a +0.5 weight bonus on tasks targeting them per spec
//! § MMB tag (priority signal — non-intrusive).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Spec-mandated utility weight bonus for tagged targets.
pub const TAG_UTILITY_BONUS: f32 = 0.5;

/// Default tag TTL in ticks (60-300 seconds depending on category; this
/// is the conservative 60-second default at 60Hz). Engine may override
/// per category.
pub const DEFAULT_TAG_TTL_TICKS: u64 = 60 * 60;

/// One tag entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagInfo {
    /// Tick at which the tag was dropped.
    pub tagged_at_tick: u64,
    /// Tick after which the tag expires.
    pub expires_at_tick: u64,
    /// Utility weight bonus applied to scoring tasks for this target.
    pub weight_bonus: f32,
    /// Issuer actor id (player typically).
    pub issuer_actor_id: u64,
}

/// Tag state. cf-control owns one instance per session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TagState {
    /// Currently-active tags keyed by target id.
    pub tagged: BTreeMap<u64, TagInfo>,
}

impl TagState {
    /// Drop a tag on `target_id`. Replaces any existing tag on the same
    /// target so the player can refresh expiry.
    pub fn add_tag(&mut self, target_id: u64, current_tick: u64, ttl_ticks: u64, issuer: u64) -> &TagInfo {
        let info = TagInfo {
            tagged_at_tick: current_tick,
            expires_at_tick: current_tick.saturating_add(ttl_ticks),
            weight_bonus: TAG_UTILITY_BONUS,
            issuer_actor_id: issuer,
        };
        self.tagged.insert(target_id, info);
        self.tagged.get(&target_id).expect("just inserted")
    }

    /// Whether `target_id` carries a (non-expired) tag at `current_tick`.
    pub fn is_tagged(&self, target_id: u64, current_tick: u64) -> bool {
        self.tagged
            .get(&target_id)
            .is_some_and(|t| current_tick <= t.expires_at_tick)
    }

    /// Utility weight bonus for `target_id` at `current_tick` (0.0 when
    /// not tagged or expired).
    pub fn weight_bonus(&self, target_id: u64, current_tick: u64) -> f32 {
        if self.is_tagged(target_id, current_tick) {
            TAG_UTILITY_BONUS
        } else {
            0.0
        }
    }

    /// Clean up expired tags. Returns the number removed.
    pub fn expire_old(&mut self, current_tick: u64) -> usize {
        let before = self.tagged.len();
        self.tagged.retain(|_, t| current_tick <= t.expires_at_tick);
        before - self.tagged.len()
    }

    /// Number of currently-active tags.
    pub fn len(&self) -> usize {
        self.tagged.len()
    }

    /// Whether there are any active tags.
    pub fn is_empty(&self) -> bool {
        self.tagged.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tag_records_expiry_and_bonus() {
        let mut s = TagState::default();
        let info = s.add_tag(7, 100, 600, 1);
        assert_eq!(info.tagged_at_tick, 100);
        assert_eq!(info.expires_at_tick, 700);
        assert_eq!(info.weight_bonus, TAG_UTILITY_BONUS);
        assert_eq!(info.issuer_actor_id, 1);
    }

    #[test]
    fn is_tagged_honors_expiry() {
        let mut s = TagState::default();
        s.add_tag(7, 100, 60, 1);
        assert!(s.is_tagged(7, 100));
        assert!(s.is_tagged(7, 160));
        assert!(!s.is_tagged(7, 161));
    }

    #[test]
    fn weight_bonus_zero_when_not_tagged() {
        let s = TagState::default();
        assert_eq!(s.weight_bonus(99, 0), 0.0);
    }

    #[test]
    fn weight_bonus_returns_constant_when_tagged() {
        let mut s = TagState::default();
        s.add_tag(7, 100, 60, 1);
        assert_eq!(s.weight_bonus(7, 120), TAG_UTILITY_BONUS);
    }

    #[test]
    fn expire_old_removes_expired() {
        let mut s = TagState::default();
        s.add_tag(1, 0, 50, 1);
        s.add_tag(2, 0, 200, 1);
        let removed = s.expire_old(100);
        assert_eq!(removed, 1);
        assert!(!s.tagged.contains_key(&1));
        assert!(s.tagged.contains_key(&2));
    }
}
