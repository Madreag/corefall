//! M7-A: basic 3-faction registry.
//!
//! 3 factions (Player, AiAllied, AiEnemy) + relationship matrix in
//! `[-100, +100]`. M7-B promotes this into the dedicated `cf-faction` crate
//! with full diplomacy + war declaration; M7-A ships the surface so the
//! engine can route friendly-fire damage through the matrix.

use serde::{Deserialize, Serialize};

/// **M7-A**: faction identity.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FactionId {
    #[default]
    Player,
    AiAllied,
    AiEnemy,
}

impl FactionId {
    pub const ALL: [FactionId; 3] = [FactionId::Player, FactionId::AiAllied, FactionId::AiEnemy];

    pub fn as_str(self) -> &'static str {
        match self {
            FactionId::Player => "player",
            FactionId::AiAllied => "ai_allied",
            FactionId::AiEnemy => "ai_enemy",
        }
    }

    pub fn from_str(value: &str) -> Option<FactionId> {
        Some(match value {
            "player" => FactionId::Player,
            "ai_allied" => FactionId::AiAllied,
            "ai_enemy" => FactionId::AiEnemy,
            _ => return None,
        })
    }

    pub fn ordinal(self) -> usize {
        self as usize
    }
}

/// **M7-A**: 3×3 relationship matrix. Symmetric per spec — pair `(a, b)`
/// uses `a.ordinal() * 3 + b.ordinal()` for stable layout. Default
/// initialisation: self = +100; Player↔AiAllied = +75; Player↔AiEnemy =
/// −75; AiAllied↔AiEnemy = −50.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionRelationships {
    pub matrix: [[i16; 3]; 3],
}

impl FactionRelationships {
    pub fn new() -> Self {
        let mut m = [[0i16; 3]; 3];
        for i in 0..3 {
            m[i][i] = 100;
        }
        m[FactionId::Player.ordinal()][FactionId::AiAllied.ordinal()] = 75;
        m[FactionId::AiAllied.ordinal()][FactionId::Player.ordinal()] = 75;
        m[FactionId::Player.ordinal()][FactionId::AiEnemy.ordinal()] = -75;
        m[FactionId::AiEnemy.ordinal()][FactionId::Player.ordinal()] = -75;
        m[FactionId::AiAllied.ordinal()][FactionId::AiEnemy.ordinal()] = -50;
        m[FactionId::AiEnemy.ordinal()][FactionId::AiAllied.ordinal()] = -50;
        Self { matrix: m }
    }

    pub fn get(&self, a: FactionId, b: FactionId) -> i16 {
        self.matrix[a.ordinal()][b.ordinal()]
    }

    pub fn set(&mut self, a: FactionId, b: FactionId, value: i16) {
        let clamped = value.max(-100).min(100);
        self.matrix[a.ordinal()][b.ordinal()] = clamped;
        self.matrix[b.ordinal()][a.ordinal()] = clamped;
    }

    pub fn adjust(&mut self, a: FactionId, b: FactionId, delta: i16) {
        let current = self.get(a, b);
        self.set(a, b, current.saturating_add(delta));
    }

    pub fn is_hostile(&self, a: FactionId, b: FactionId) -> bool {
        self.get(a, b) <= -50
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(18);
        for row in &self.matrix {
            for v in row {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }
}

impl Default for FactionRelationships {
    fn default() -> Self {
        Self::new()
    }
}

/// **M7-A**: `faction.relationship_changed` event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipChangedEvent {
    pub a: FactionId,
    pub b: FactionId,
    pub delta: i16,
    pub new_value: i16,
    pub cause: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matrix_seeded() {
        let r = FactionRelationships::new();
        assert_eq!(r.get(FactionId::Player, FactionId::Player), 100);
        assert_eq!(r.get(FactionId::Player, FactionId::AiAllied), 75);
        assert_eq!(r.get(FactionId::Player, FactionId::AiEnemy), -75);
    }

    #[test]
    fn adjust_symmetric() {
        let mut r = FactionRelationships::new();
        r.adjust(FactionId::Player, FactionId::AiAllied, -30);
        assert_eq!(r.get(FactionId::Player, FactionId::AiAllied), 45);
        assert_eq!(r.get(FactionId::AiAllied, FactionId::Player), 45);
    }

    #[test]
    fn hostile_at_minus_fifty_or_lower() {
        let r = FactionRelationships::new();
        assert!(r.is_hostile(FactionId::Player, FactionId::AiEnemy));
        assert!(!r.is_hostile(FactionId::Player, FactionId::AiAllied));
    }
}
