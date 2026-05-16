//! M6: bipod attachment (deployable when crouched/prone).
//!
//! Spec § "Bipod attachment (deployable when crouched/prone) reduces recoil
//! 70% + bloom 50%": when deployed, recoil × 0.3 and bloom × 0.5.

use serde::{Deserialize, Serialize};

/// M6 § bipod recoil factor when deployed.
pub const BIPOD_RECOIL_FACTOR: f32 = 0.3;
/// M6 § bipod bloom factor when deployed.
pub const BIPOD_BLOOM_FACTOR: f32 = 0.5;

/// Bipod state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BipodState {
    #[default]
    Stowed = 0,
    Deployed = 1,
}

impl BipodState {
    pub fn as_str(self) -> &'static str {
        match self {
            BipodState::Stowed => "stowed",
            BipodState::Deployed => "deployed",
        }
    }
}

/// Bipod attachment + state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bipod {
    pub equipped: bool,
    pub state: BipodState,
}

impl Default for Bipod {
    fn default() -> Self {
        Self {
            equipped: false,
            state: BipodState::Stowed,
        }
    }
}

impl Bipod {
    pub fn equipped_default() -> Self {
        Self {
            equipped: true,
            state: BipodState::Stowed,
        }
    }

    /// Try to deploy when crouched OR prone. Returns true if deployed.
    pub fn try_deploy(&mut self, can_deploy: bool) -> bool {
        if !self.equipped {
            return false;
        }
        if !can_deploy {
            return false;
        }
        if self.state == BipodState::Deployed {
            return false;
        }
        self.state = BipodState::Deployed;
        true
    }

    /// Stow the bipod (e.g. on stand-up).
    pub fn stow(&mut self) -> bool {
        if !self.equipped {
            return false;
        }
        if self.state == BipodState::Stowed {
            return false;
        }
        self.state = BipodState::Stowed;
        true
    }

    /// Recoil multiplier for next shot.
    pub fn recoil_factor(self) -> f32 {
        if self.equipped && self.state == BipodState::Deployed {
            BIPOD_RECOIL_FACTOR
        } else {
            1.0
        }
    }

    /// Bloom multiplier while deployed.
    pub fn bloom_factor(self) -> f32 {
        if self.equipped && self.state == BipodState::Deployed {
            BIPOD_BLOOM_FACTOR
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deploy_only_when_eligible() {
        let mut b = Bipod::equipped_default();
        assert!(!b.try_deploy(false));
        assert_eq!(b.state, BipodState::Stowed);
        assert!(b.try_deploy(true));
        assert_eq!(b.state, BipodState::Deployed);
    }

    #[test]
    fn stow_auto() {
        let mut b = Bipod {
            equipped: true,
            state: BipodState::Deployed,
        };
        assert!(b.stow());
        assert_eq!(b.state, BipodState::Stowed);
    }

    #[test]
    fn deployed_reduces_recoil() {
        let mut b = Bipod::equipped_default();
        let _ = b.try_deploy(true);
        assert!((b.recoil_factor() - BIPOD_RECOIL_FACTOR).abs() < 1e-3);
        assert!((b.bloom_factor() - BIPOD_BLOOM_FACTOR).abs() < 1e-3);
    }

    #[test]
    fn unequipped_no_effect() {
        let b = Bipod::default();
        assert!((b.recoil_factor() - 1.0).abs() < 1e-3);
    }
}
