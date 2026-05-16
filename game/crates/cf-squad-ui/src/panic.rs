//! Single-key panic surfaces — M = call medic to me, R = call engineer
//! repair self, G = grenade out (everyone find cover) per spec § Single-key
//! panic commands.

use serde::{Deserialize, Serialize};

/// Which panic command was issued.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanicKind {
    /// "MEDIC ON ME!" — nearest available Medic routes to player.
    Medic,
    /// "REPAIR THIS!" — nearest available Engineer routes to target.
    Engineer,
    /// "GRENADE OUT!" — everyone find cover.
    Grenade,
}

impl PanicKind {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            PanicKind::Medic => "medic",
            PanicKind::Engineer => "engineer",
            PanicKind::Grenade => "grenade",
        }
    }

    /// Default key binding label (per spec § Single-key panic commands
    /// table).
    pub fn default_key(self) -> &'static str {
        match self {
            PanicKind::Medic => "M",
            PanicKind::Engineer => "R",
            PanicKind::Grenade => "G",
        }
    }

    /// Player-facing chatter mirror.
    pub fn caption(self) -> &'static str {
        match self {
            PanicKind::Medic => "MEDIC ON ME!",
            PanicKind::Engineer => "REPAIR THIS!",
            PanicKind::Grenade => "GRENADE OUT!",
        }
    }

    /// Parse from cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<PanicKind> {
        Some(match value {
            "medic" => PanicKind::Medic,
            "engineer" => PanicKind::Engineer,
            "grenade" => PanicKind::Grenade,
            _ => return None,
        })
    }
}

/// Outcome of a panic call (used by cf-control to decide whether to also
/// emit the `NO MEDIC — self-rescue` red caption).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanicCommand {
    /// Which panic was issued.
    pub kind: PanicKind,
    /// Issuer actor id.
    pub issuer_actor_id: u64,
    /// Whether a responder was found (Medic/Engineer present, or
    /// Grenade case = always true since it's a global broadcast).
    pub responder_found: bool,
    /// Responder actor id when found.
    pub responder_actor_id: Option<u64>,
}

impl PanicCommand {
    /// Build a successful panic-call (with responder).
    pub fn responded(kind: PanicKind, issuer: u64, responder: u64) -> Self {
        Self {
            kind,
            issuer_actor_id: issuer,
            responder_found: true,
            responder_actor_id: Some(responder),
        }
    }

    /// Build an unanswered panic-call (no responder available).
    pub fn no_responder(kind: PanicKind, issuer: u64) -> Self {
        Self {
            kind,
            issuer_actor_id: issuer,
            responder_found: false,
            responder_actor_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_str() {
        for k in [PanicKind::Medic, PanicKind::Engineer, PanicKind::Grenade] {
            assert_eq!(PanicKind::from_str(k.as_str()), Some(k));
        }
    }

    #[test]
    fn default_keys_are_m_r_g() {
        assert_eq!(PanicKind::Medic.default_key(), "M");
        assert_eq!(PanicKind::Engineer.default_key(), "R");
        assert_eq!(PanicKind::Grenade.default_key(), "G");
    }

    #[test]
    fn captions_match_spec() {
        assert_eq!(PanicKind::Medic.caption(), "MEDIC ON ME!");
        assert_eq!(PanicKind::Engineer.caption(), "REPAIR THIS!");
        assert_eq!(PanicKind::Grenade.caption(), "GRENADE OUT!");
    }

    #[test]
    fn responded_carries_responder() {
        let p = PanicCommand::responded(PanicKind::Medic, 1, 2);
        assert!(p.responder_found);
        assert_eq!(p.responder_actor_id, Some(2));
    }

    #[test]
    fn no_responder_clears_responder() {
        let p = PanicCommand::no_responder(PanicKind::Medic, 1);
        assert!(!p.responder_found);
        assert_eq!(p.responder_actor_id, None);
    }
}
