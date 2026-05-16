//! M9 — Target selection via utility scoring (DR-008 hybrid jobs+utility).
//!
//! Spec § Reactive guard targeting + path reaction — when the guard has
//! multiple candidate targets (reactor + player), choose by
//! `score(target) = w_proximity * (1/distance) + w_los * has_los +
//! w_threat * is_player + w_value * is_high_value_static`. Default
//! weights are tuned per difficulty preset. Emits `ai.target_scored`
//! with all candidates + chosen + reason (engine fires the event; this
//! module is the pure scorer).

/// Weight set used by the utility scorer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetWeights {
    pub proximity: f32,
    pub los: f32,
    pub threat: f32,
    pub value: f32,
}

impl Default for TargetWeights {
    fn default() -> Self {
        // Defaults match the spec's reactive_guard behavior: prefer the
        // player when in LOS, fall back to the reactor.
        //
        // **M14 audit fix** (pre-existing M9 bug): `threat` (player-aliveness)
        // MUST dominate `value` (reactor-aliveness) when both are visible —
        // otherwise the AI ignores the player and snipes the reactor through
        // the player's body. Swap the two weights so the
        // `player_in_los_outscores_reactor` regression test holds while
        // `reactor_wins_when_player_out_of_los` (player out of LOS) still
        // resolves to the reactor.
        Self {
            proximity: 0.4,
            los: 0.3,
            threat: 0.4,
            value: 0.2,
        }
    }
}

/// One target candidate the scorer evaluates.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetCandidate<Id: Clone + PartialEq> {
    pub id: Id,
    pub distance: f32,
    pub has_los: bool,
    pub is_player: bool,
    pub is_high_value_static: bool,
}

/// Per-candidate score breakdown the engine can fold into the
/// `ai.target_scored` event payload.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredTarget<Id: Clone + PartialEq> {
    pub id: Id,
    pub score: f32,
    pub reason: String,
}

/// Score a single candidate.
#[must_use]
pub fn score_candidate<Id: Clone + PartialEq>(c: &TargetCandidate<Id>, w: &TargetWeights) -> f32 {
    let proximity_term = if c.distance > 0.0 { 1.0 / c.distance } else { 0.0 };
    let los_term = if c.has_los { 1.0 } else { 0.0 };
    let threat_term = if c.is_player { 1.0 } else { 0.0 };
    let value_term = if c.is_high_value_static { 1.0 } else { 0.0 };
    w.proximity * proximity_term + w.los * los_term + w.threat * threat_term + w.value * value_term
}

/// Score every candidate, returning the chosen id + per-candidate
/// breakdown. Returns `None` when the input is empty.
#[must_use]
pub fn score_all<Id: Clone + PartialEq>(
    candidates: &[TargetCandidate<Id>],
    w: &TargetWeights,
) -> Option<(Id, Vec<ScoredTarget<Id>>)> {
    if candidates.is_empty() {
        return None;
    }
    let scored: Vec<ScoredTarget<Id>> = candidates
        .iter()
        .map(|c| {
            let s = score_candidate(c, w);
            let reason = format!(
                "proximity={:.2} los={} threat={} value={}",
                if c.distance > 0.0 { 1.0 / c.distance } else { 0.0 },
                c.has_los,
                c.is_player,
                c.is_high_value_static,
            );
            ScoredTarget {
                id: c.id.clone(),
                score: s,
                reason,
            }
        })
        .collect();
    let mut best_idx = 0;
    let mut best_score = scored[0].score;
    for (i, st) in scored.iter().enumerate().skip(1) {
        if st.score > best_score {
            best_score = st.score;
            best_idx = i;
        }
    }
    Some((scored[best_idx].id.clone(), scored))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_in_los_outscores_reactor() {
        let player = TargetCandidate {
            id: "player",
            distance: 50.0,
            has_los: true,
            is_player: true,
            is_high_value_static: false,
        };
        let reactor = TargetCandidate {
            id: "reactor",
            distance: 200.0,
            has_los: true,
            is_player: false,
            is_high_value_static: true,
        };
        let (chosen, _) = score_all(&[player, reactor], &TargetWeights::default()).expect("non-empty");
        assert_eq!(chosen, "player");
    }

    #[test]
    fn reactor_wins_when_player_out_of_los() {
        let player = TargetCandidate {
            id: "player",
            distance: 80.0,
            has_los: false,
            is_player: true,
            is_high_value_static: false,
        };
        let reactor = TargetCandidate {
            id: "reactor",
            distance: 200.0,
            has_los: true,
            is_player: false,
            is_high_value_static: true,
        };
        let (chosen, _) = score_all(&[player, reactor], &TargetWeights::default()).expect("non-empty");
        assert_eq!(chosen, "reactor");
    }

    #[test]
    fn empty_candidates_returns_none() {
        let v: Vec<TargetCandidate<&str>> = Vec::new();
        assert!(score_all(&v, &TargetWeights::default()).is_none());
    }
}
