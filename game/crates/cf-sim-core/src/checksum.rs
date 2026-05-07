//! `sim_state_v1` checksum helpers (DR-002 schema lock).
//!
//! M0 scope: `tick_counter || rng_state_bytes` (40 bytes total).
//! M1 appends caller-supplied `extra` bytes (e.g. `ActorSimState::checksum_bytes()`)
//! to the stream so authoritative actor/inventory/projectile state participates
//! in the divergence guarantee. The byte layout is append-only so the `_v1`
//! suffix is preserved; layout-breaking bumps move to `_v2` and register a
//! migration in the run-bundle schema.

use blake3::Hasher;

use crate::{Rng, Tick};

pub const CHECKSUM_SCOPE: &str = "sim_state_v1";
pub const CHECKSUM_ALGORITHM: &str = "blake3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimChecksum(pub [u8; 32]);

impl SimChecksum {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

pub fn sim_state_v1(tick: Tick, rng: &Rng, extra: &[u8]) -> SimChecksum {
    let mut hasher = Hasher::new();
    hasher.update(&tick.0.to_le_bytes());
    hasher.update(&rng.state_bytes());
    hasher.update(extra);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    SimChecksum(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_deterministic() {
        let rng_a = Rng::from_seed(7);
        let rng_b = Rng::from_seed(7);
        let a = sim_state_v1(Tick(42), &rng_a, &[]);
        let b = sim_state_v1(Tick(42), &rng_b, &[]);
        assert_eq!(a, b);
        assert_eq!(a.to_hex().len(), 64);
    }

    #[test]
    fn checksum_changes_with_tick() {
        let rng = Rng::from_seed(7);
        let a = sim_state_v1(Tick(1), &rng, &[]);
        let b = sim_state_v1(Tick(2), &rng, &[]);
        assert_ne!(a, b);
    }

    #[test]
    fn checksum_changes_with_seed() {
        let a_rng = Rng::from_seed(1);
        let b_rng = Rng::from_seed(2);
        let a = sim_state_v1(Tick(0), &a_rng, &[]);
        let b = sim_state_v1(Tick(0), &b_rng, &[]);
        assert_ne!(a, b);
    }

    #[test]
    fn checksum_changes_with_extra_bytes() {
        let rng = Rng::from_seed(7);
        let a = sim_state_v1(Tick(1), &rng, &[]);
        let b = sim_state_v1(Tick(1), &rng, &[1, 2, 3]);
        assert_ne!(a, b);
    }
}
