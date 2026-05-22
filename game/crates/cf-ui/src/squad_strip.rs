//! M8 / M11 — Squad strip HUD widget (top-right next to ammo).
//!
//! Per spec § UX widgets: 1-4 members with role + HP + alert badge.
//! M11 (c4b4ea0) extension: each row also carries `current_command` +
//! `autonomy_mode` + `reason_label_recent` badge + a top-priority
//! icon (see `priority_indicator.rs`).

use bevy::prelude::*;

use crate::priority_indicator::PriorityIcon;

/// Maximum members displayed in the strip (matches the spec's 1-4 cap;
/// follow-up squads beyond 4 spill into other HUD surfaces).
pub const SQUAD_STRIP_MAX_MEMBERS: usize = 4;

/// M11: autonomy mode mirror (cf-priority owns the canonical enum).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SquadAutonomyMode {
    /// FullAuto — bot acts on its own initiative.
    FullAuto,
    /// Standard — bot accepts player orders + acts on initiative when idle.
    #[default]
    Standard,
    /// Manual — bot only acts on direct player orders.
    Manual,
}

impl SquadAutonomyMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SquadAutonomyMode::FullAuto => "full_auto",
            SquadAutonomyMode::Standard => "standard",
            SquadAutonomyMode::Manual => "manual",
        }
    }

    /// Short HUD tag.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            SquadAutonomyMode::FullAuto => "AUTO",
            SquadAutonomyMode::Standard => "STD",
            SquadAutonomyMode::Manual => "MAN",
        }
    }

    /// Parse from a (case-insensitive) wire string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "full_auto" | "auto" => SquadAutonomyMode::FullAuto,
            "manual" => SquadAutonomyMode::Manual,
            _ => SquadAutonomyMode::Standard,
        }
    }
}

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
    /// "ADVANCE", "FOLLOW"). `None` when no order is active.
    pub current_command: Option<String>,
    pub autonomy_mode: SquadAutonomyMode,
    /// (latest reason first). Surfaces in the row badge + tooltip.
    pub reason_label_recent: Option<String>,
    pub priority_icon: Option<PriorityIcon>,
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

/// Compose one HUD row line per spec § Squad Strip.
#[must_use]
pub fn squad_row_line(member: &SquadStripMember) -> String {
    let hp_pct = (member.hp_fraction.clamp(0.0, 1.0) * 100.0) as u32;
    let role_short: String = member.role.chars().take(3).collect::<String>().to_uppercase();
    let mut line = format!(
        "{name} {role} HP{hp}% [{mode}]",
        name = member.display_name,
        role = role_short,
        hp = hp_pct,
        mode = member.autonomy_mode.tag(),
    );
    if let Some(cmd) = &member.current_command {
        line.push_str(&format!(" / {}", cmd));
    }
    if let Some(icon) = member.priority_icon {
        line.push_str(&format!(" <{}>", icon.ascii_glyph()));
    }
    if let Some(alert) = &member.alert {
        line.push_str(&format!(" {}", alert));
    }
    if let Some(reason) = &member.reason_label_recent {
        line.push_str(&format!(" «{}»", reason));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_member(id: u64) -> SquadStripMember {
        SquadStripMember {
            actor_id: id,
            display_name: format!("M{id}"),
            role: "rifleman".into(),
            hp_fraction: 1.0,
            alert: None,
            current_command: None,
            autonomy_mode: SquadAutonomyMode::Standard,
            reason_label_recent: None,
            priority_icon: None,
        }
    }

    #[test]
    fn set_members_truncates_to_cap() {
        let mut s = SquadStripState::default();
        let members: Vec<_> = (0..6).map(mk_member).collect();
        s.set_members(members);
        assert_eq!(s.members.len(), SQUAD_STRIP_MAX_MEMBERS);
    }

    #[test]
    fn row_line_renders_command_and_icon_and_reason() {
        let mut m = mk_member(1);
        m.current_command = Some("HOLD".into());
        m.priority_icon = Some(PriorityIcon::HoldingCover);
        m.reason_label_recent = Some("enemy_spotted".into());
        m.autonomy_mode = SquadAutonomyMode::FullAuto;
        let line = squad_row_line(&m);
        assert!(line.contains("[AUTO]"));
        assert!(line.contains("HOLD"));
        assert!(line.contains("<HC>"));
        assert!(line.contains("enemy_spotted"));
    }

    #[test]
    fn autonomy_mode_from_str_handles_aliases() {
        assert_eq!(SquadAutonomyMode::from_str("auto"), SquadAutonomyMode::FullAuto);
        assert_eq!(SquadAutonomyMode::from_str("MANUAL"), SquadAutonomyMode::Manual);
        assert_eq!(SquadAutonomyMode::from_str("unknown"), SquadAutonomyMode::Standard);
    }
}
