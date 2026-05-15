//! M8 — Squad strip HUD widget (top-right next to ammo).
//!
//! Per spec § UX widgets: 1-4 members with role + HP + alert badge.

use bevy::prelude::*;

/// Maximum members displayed in the strip (matches the spec's 1-4 cap;
/// follow-up squads beyond 4 spill into other HUD surfaces).
pub const SQUAD_STRIP_MAX_MEMBERS: usize = 4;

/// One squadmate row.
#[derive(Debug, Clone, PartialEq)]
pub struct SquadStripMember {
    /// Member actor id.
    pub actor_id: u64,
    /// Display name.
    pub display_name: String,
    /// Role label (rifleman / sniper / assault / engineer / spotter / medic).
    pub role: String,
    /// HP fraction in `[0, 1]`.
    pub hp_fraction: f32,
    /// Optional alert badge (e.g. "LOW HP", "DOWNED", "BLEEDING").
    pub alert: Option<String>,
}

/// Squad strip widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct SquadStripState {
    /// Up to `SQUAD_STRIP_MAX_MEMBERS` rows in display order.
    pub members: Vec<SquadStripMember>,
}

impl SquadStripState {
    /// Replace the entire row list, truncating to the spec cap.
    pub fn set_members(&mut self, mut members: Vec<SquadStripMember>) {
        members.truncate(SQUAD_STRIP_MAX_MEMBERS);
        self.members = members;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_members_truncates_to_cap() {
        let mut s = SquadStripState::default();
        let members: Vec<_> = (0..6)
            .map(|i| SquadStripMember {
                actor_id: i,
                display_name: format!("M{i}"),
                role: "rifleman".into(),
                hp_fraction: 1.0,
                alert: None,
            })
            .collect();
        s.set_members(members);
        assert_eq!(s.members.len(), SQUAD_STRIP_MAX_MEMBERS);
    }
}
