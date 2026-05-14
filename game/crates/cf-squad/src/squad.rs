//! M6: squad state + member tracking.

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, Vec2};

use crate::command::SquadCommand;

/// Role of a member within the squad. M6 ships Leader + Follower; M7 layers
/// archetypes (Rifleman, Medic, Engineer, Recon, Heavy) on top.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadRole {
    Leader = 0,
    Follower = 1,
}

impl SquadRole {
    pub fn as_str(self) -> &'static str {
        match self {
            SquadRole::Leader => "leader",
            SquadRole::Follower => "follower",
        }
    }
}

/// One squad slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadMember {
    pub actor: ActorId,
    pub role: SquadRole,
    pub display_name: String,
    pub current_command: SquadCommand,
    /// Persistent waypoint for `DefendPoint`/`PushToWaypoint`.
    pub waypoint: Option<Vec2>,
    /// Cached HP / max-HP for HUD strip.
    pub hp: f32,
    pub hp_max: f32,
}

impl SquadMember {
    pub fn new(actor: ActorId, role: SquadRole, display_name: impl Into<String>, hp_max: f32) -> Self {
        Self {
            actor,
            role,
            display_name: display_name.into(),
            current_command: SquadCommand::default(),
            waypoint: None,
            hp: hp_max,
            hp_max,
        }
    }
}

/// One squad. Max two members at M6 (leader + follower); M7+ relaxes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Squad {
    pub leader: Option<SquadMember>,
    pub followers: Vec<SquadMember>,
}

impl Squad {
    pub fn member_count(&self) -> usize {
        self.leader.iter().count() + self.followers.len()
    }

    pub fn add_leader(&mut self, member: SquadMember) -> bool {
        if self.leader.is_some() {
            return false;
        }
        self.leader = Some(SquadMember {
            role: SquadRole::Leader,
            ..member
        });
        true
    }

    pub fn add_follower(&mut self, mut member: SquadMember) -> bool {
        member.role = SquadRole::Follower;
        self.followers.push(member);
        true
    }

    pub fn remove(&mut self, actor: ActorId) -> bool {
        if let Some(l) = &self.leader {
            if l.actor == actor {
                self.leader = None;
                return true;
            }
        }
        if let Some(pos) = self.followers.iter().position(|m| m.actor == actor) {
            self.followers.remove(pos);
            return true;
        }
        false
    }

    pub fn iter(&self) -> impl Iterator<Item = &SquadMember> {
        self.leader.iter().chain(self.followers.iter())
    }

    /// Look up a member by actor id.
    pub fn find_member(&self, actor: ActorId) -> Option<&SquadMember> {
        self.iter().find(|m| m.actor == actor)
    }

    /// Look up a mutable member by actor id.
    pub fn find_member_mut(&mut self, actor: ActorId) -> Option<&mut SquadMember> {
        if let Some(l) = &mut self.leader {
            if l.actor == actor {
                return Some(l);
            }
        }
        self.followers.iter_mut().find(|m| m.actor == actor)
    }

    /// Issue a command to a specific member. Returns false if member not present.
    pub fn issue_command(&mut self, actor: ActorId, command: SquadCommand) -> bool {
        if let Some(m) = self.find_member_mut(actor) {
            m.waypoint = command.waypoint;
            m.current_command = command;
            true
        } else {
            false
        }
    }

    /// Issue a command to all followers (leader unchanged).
    pub fn broadcast_to_followers(&mut self, command: &SquadCommand) -> usize {
        let n = self.followers.len();
        for m in &mut self.followers {
            m.waypoint = command.waypoint;
            m.current_command = command.clone();
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::SquadCommandKind;

    fn member(id: u64) -> SquadMember {
        SquadMember::new(ActorId(id), SquadRole::Follower, format!("Bot {id}"), 100.0)
    }

    #[test]
    fn add_and_remove() {
        let mut s = Squad::default();
        assert!(s.add_leader(member(1)));
        assert!(s.add_follower(member(2)));
        assert_eq!(s.member_count(), 2);
        assert!(s.remove(ActorId(2)));
        assert_eq!(s.member_count(), 1);
    }

    #[test]
    fn issue_command_updates_member() {
        let mut s = Squad::default();
        s.add_follower(member(7));
        let cmd = SquadCommand {
            kind: SquadCommandKind::DefendPoint,
            waypoint: Some(Vec2::new(10.0, 0.0)),
            issuer: ActorId(1),
        };
        assert!(s.issue_command(ActorId(7), cmd.clone()));
        assert_eq!(
            s.find_member(ActorId(7)).unwrap().current_command.kind,
            SquadCommandKind::DefendPoint
        );
    }

    #[test]
    fn issue_command_missing_member() {
        let mut s = Squad::default();
        assert!(!s.issue_command(ActorId(99), SquadCommand::default()));
    }

    #[test]
    fn broadcast_hits_all_followers() {
        let mut s = Squad::default();
        s.add_follower(member(1));
        s.add_follower(member(2));
        let cmd = SquadCommand {
            kind: SquadCommandKind::HoldPosition,
            waypoint: None,
            issuer: ActorId(0),
        };
        assert_eq!(s.broadcast_to_followers(&cmd), 2);
        for m in &s.followers {
            assert_eq!(m.current_command.kind, SquadCommandKind::HoldPosition);
        }
    }
}
