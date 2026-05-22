use crate::GuardState;

/// Recorded `ai.perception` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct PerceptionRecord {
    pub player_seen: bool,
    pub distance: Option<f32>,
    pub angle_degrees: Option<f32>,
    pub last_seen_position: Option<[f32; 2]>,
    pub state: GuardState,
}
