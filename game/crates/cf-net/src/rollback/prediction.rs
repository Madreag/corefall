//! M8B § Per-frame input prediction model.
//!
//! Per M8B spec § Notes: prediction is "last-input-repeat" baseline.
//! Smarter prediction (extrapolated aim, decayed move) is explicitly
//! out of scope; the determinism cost of cleverness here is not worth
//! the perceived smoothness.

use serde::{Deserialize, Serialize};

/// M8B prediction policy. Only `LastInputRepeat` is permitted by the
/// spec. The other variant exists ONLY to document why it is forbidden
/// and to make any future PR that tries to enable it fail review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionMode {
    /// **M8B locked**: repeat the last known input verbatim until the
    /// authoritative server input arrives.
    #[default]
    LastInputRepeat,
    /// Forbidden at M8B (kept for documentation). Selecting this
    /// variant MUST cause `InputPredictor::predict` to return None and
    /// the recorder to record a `net.prediction_forbidden_mode` event.
    /// Producers are NOT expected to use this — it exists so the spec's
    /// "explicitly out of scope" clause survives review.
    #[allow(dead_code)]
    ExtrapolatedAim,
}

/// **M8B § locked**: predictor for unknown future ticks. Returns the
/// last known input bytes for every requested forward tick.
#[derive(Debug, Clone)]
pub struct InputPredictor {
    pub mode: PredictionMode,
    last_input: Option<(u64, Vec<u8>)>,
}

impl InputPredictor {
    pub fn new(mode: PredictionMode) -> Self {
        Self {
            mode,
            last_input: None,
        }
    }

    /// Record an authoritative input received from the server. Updates
    /// the last-known input for the predictor.
    pub fn record_authoritative(&mut self, tick: u64, intent_bytes: Vec<u8>) {
        self.last_input = Some((tick, intent_bytes));
    }

    /// Predict the input for `target_tick`. Returns `None` when no
    /// prior input is known OR when the forbidden mode is selected.
    pub fn predict(&self, target_tick: u64) -> Option<Vec<u8>> {
        if !matches!(self.mode, PredictionMode::LastInputRepeat) {
            return None;
        }
        self.last_input.as_ref().map(|(prev_tick, bytes)| {
            // Always repeat; `target_tick` is forward of `prev_tick`
            // by construction.
            let _ = (prev_tick, target_tick);
            bytes.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_last_input_repeat() {
        let p = InputPredictor::new(PredictionMode::default());
        assert!(matches!(p.mode, PredictionMode::LastInputRepeat));
    }

    #[test]
    fn predict_returns_last_known_intent() {
        let mut p = InputPredictor::new(PredictionMode::LastInputRepeat);
        p.record_authoritative(100, vec![1, 2, 3]);
        assert_eq!(p.predict(101), Some(vec![1, 2, 3]));
        p.record_authoritative(101, vec![4, 5, 6]);
        assert_eq!(p.predict(102), Some(vec![4, 5, 6]));
    }

    #[test]
    fn predict_without_authoritative_returns_none() {
        let p = InputPredictor::new(PredictionMode::LastInputRepeat);
        assert!(p.predict(0).is_none());
    }

    #[test]
    fn forbidden_mode_returns_none() {
        let mut p = InputPredictor::new(PredictionMode::ExtrapolatedAim);
        p.record_authoritative(100, vec![1]);
        assert!(p.predict(101).is_none(), "M8B forbids extrapolated aim");
    }
}
