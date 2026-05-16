//! M8 — Cover state pip HUD widget (None / Partial / Full near reticle).

use bevy::prelude::*;

/// Discrete cover state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum CoverLevel {
    /// Exposed.
    #[default]
    None,
    /// Behind partial cover.
    Partial,
    /// Behind full cover.
    Full,
}

impl CoverLevel {
    /// Player-facing pip label.
    pub fn label(self) -> &'static str {
        match self {
            CoverLevel::None => "COVER: None",
            CoverLevel::Partial => "COVER: Partial",
            CoverLevel::Full => "COVER: Full",
        }
    }
}

/// Cover pip widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct CoverPipState {
    /// Active cover level for the player actor.
    pub level: CoverLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_spec() {
        assert_eq!(CoverLevel::None.label(), "COVER: None");
        assert_eq!(CoverLevel::Partial.label(), "COVER: Partial");
        assert_eq!(CoverLevel::Full.label(), "COVER: Full");
    }
}
