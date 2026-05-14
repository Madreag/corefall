//! M6: per-tool durability (0-100%).
//!
//! Per spec § "Tool degradation": each tool has durability that drops with
//! use; broken tool emits `equipment.tool_broken`; repair tool restores.

use serde::{Deserialize, Serialize};

/// Maximum durability (100% baseline).
pub const DURABILITY_MAX: f32 = 100.0;

/// Per-tool durability state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Durability {
    pub current: f32,
    pub max: f32,
}

impl Default for Durability {
    fn default() -> Self {
        Self {
            current: DURABILITY_MAX,
            max: DURABILITY_MAX,
        }
    }
}

impl Durability {
    pub fn new(max: f32) -> Self {
        let max = max.max(1.0);
        Self { current: max, max }
    }

    pub fn fraction(self) -> f32 {
        if self.max <= 0.0 {
            return 0.0;
        }
        (self.current / self.max).clamp(0.0, 1.0)
    }

    pub fn is_broken(self) -> bool {
        self.current <= 0.0
    }

    /// Apply use-wear; returns true if the tool just broke this call.
    pub fn apply_wear(&mut self, amount: f32) -> bool {
        if !amount.is_finite() || amount <= 0.0 {
            return false;
        }
        if self.is_broken() {
            return false;
        }
        let prev = self.current;
        self.current = (self.current - amount).max(0.0);
        prev > 0.0 && self.current <= 0.0
    }

    /// Restore durability (repair tool effect).
    pub fn restore(&mut self, amount: f32) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        self.current = (self.current + amount).clamp(0.0, self.max);
    }

    pub fn reset(&mut self) {
        self.current = self.max;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_at_zero() {
        let mut d = Durability::new(10.0);
        assert!(!d.apply_wear(5.0));
        let broke = d.apply_wear(10.0);
        assert!(broke);
        assert!(d.is_broken());
    }

    #[test]
    fn already_broken_no_event() {
        let mut d = Durability {
            current: 0.0,
            max: 100.0,
        };
        assert!(!d.apply_wear(5.0));
    }

    #[test]
    fn repair_restores() {
        let mut d = Durability::new(100.0);
        let _ = d.apply_wear(60.0);
        d.restore(30.0);
        assert!((d.current - 70.0).abs() < 1e-3);
    }

    #[test]
    fn repair_capped_at_max() {
        let mut d = Durability::new(50.0);
        d.restore(200.0);
        assert_eq!(d.current, 50.0);
    }

    #[test]
    fn nan_amount_ignored() {
        let mut d = Durability::new(10.0);
        assert!(!d.apply_wear(f32::NAN));
        assert_eq!(d.current, 10.0);
    }

    #[test]
    fn fraction_correct() {
        let d = Durability {
            current: 50.0,
            max: 100.0,
        };
        assert!((d.fraction() - 0.5).abs() < 1e-3);
    }
}
