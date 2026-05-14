//! M6: the 4 squad commands.

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, Vec2};

/// One of the 4 commands a squad member can be issued. M7 layers full AI
/// archetypes on top; M6 ships the surface.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadCommandKind {
    /// Default: bot follows the leader.
    #[default]
    FollowLeader = 0,
    /// Bot stops + holds current position.
    HoldPosition = 1,
    /// Bot moves to a waypoint and holds, using cover.
    DefendPoint = 2,
    /// Bot moves to a waypoint while engaging targets on route.
    PushToWaypoint = 3,
}

impl SquadCommandKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SquadCommandKind::FollowLeader => "follow_leader",
            SquadCommandKind::HoldPosition => "hold_position",
            SquadCommandKind::DefendPoint => "defend_point",
            SquadCommandKind::PushToWaypoint => "push_to_waypoint",
        }
    }

    pub fn requires_waypoint(self) -> bool {
        matches!(self, SquadCommandKind::DefendPoint | SquadCommandKind::PushToWaypoint)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadCommand {
    pub kind: SquadCommandKind,
    pub waypoint: Option<Vec2>,
    pub issuer: ActorId,
}

impl Default for SquadCommand {
    fn default() -> Self {
        Self {
            kind: SquadCommandKind::FollowLeader,
            waypoint: None,
            issuer: ActorId::default(),
        }
    }
}

impl SquadCommand {
    pub fn follow(issuer: ActorId) -> Self {
        Self {
            kind: SquadCommandKind::FollowLeader,
            waypoint: None,
            issuer,
        }
    }

    pub fn hold(issuer: ActorId) -> Self {
        Self {
            kind: SquadCommandKind::HoldPosition,
            waypoint: None,
            issuer,
        }
    }

    pub fn defend(issuer: ActorId, point: Vec2) -> Self {
        Self {
            kind: SquadCommandKind::DefendPoint,
            waypoint: Some(point),
            issuer,
        }
    }

    pub fn push(issuer: ActorId, point: Vec2) -> Self {
        Self {
            kind: SquadCommandKind::PushToWaypoint,
            waypoint: Some(point),
            issuer,
        }
    }

    /// True if the command can be applied with the given waypoint state.
    /// Rejects DefendPoint / PushToWaypoint when waypoint is None.
    pub fn is_well_formed(&self) -> bool {
        if self.kind.requires_waypoint() && self.waypoint.is_none() {
            return false;
        }
        if let Some(p) = self.waypoint {
            if !p.x.is_finite() || !p.y.is_finite() {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_four_kinds_distinct() {
        let v = [
            SquadCommandKind::FollowLeader,
            SquadCommandKind::HoldPosition,
            SquadCommandKind::DefendPoint,
            SquadCommandKind::PushToWaypoint,
        ];
        for i in 0..v.len() {
            for j in (i + 1)..v.len() {
                assert_ne!(v[i], v[j]);
            }
        }
    }

    #[test]
    fn defend_requires_waypoint() {
        assert!(SquadCommandKind::DefendPoint.requires_waypoint());
        let bad = SquadCommand {
            kind: SquadCommandKind::DefendPoint,
            waypoint: None,
            issuer: ActorId(1),
        };
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn nan_waypoint_rejected() {
        let bad = SquadCommand {
            kind: SquadCommandKind::PushToWaypoint,
            waypoint: Some(Vec2::new(f32::NAN, 0.0)),
            issuer: ActorId(1),
        };
        assert!(!bad.is_well_formed());
    }
}
