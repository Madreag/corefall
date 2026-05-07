//! M0-003: fixed-tick scheduler, deterministic RNG, run-id helpers.
//!
//! Determinism contract for M0:
//! - Fixed 60 Hz default tick rate; 120 Hz selectable.
//! - Seeded `Xoshiro256**` RNG. Sim code MUST NOT call `rand::thread_rng` (lints disallow it).
//! - `WallClock` exposes UTC time only outside the sim island; sim systems use the integer `Tick`.
//! - The `sim_state_v1` checksum scope at M0 covers `tick_counter || rng_state_bytes`. M1
//!   appends caller-supplied bytes (e.g. `cf_actor::sim::ActorSimState::checksum_bytes()`)
//!   so authoritative actor/inventory/projectile state participates in the divergence
//!   guarantee. The byte layout is append-only so the `_v1` suffix is preserved;
//!   layout-changing bumps go to `sim_state_v2`. See
//!   `docs/implementation-log/2026-05-05-m0-engine-bootstrap.md`.

use std::time::{Duration, Instant};

use rand_core::{Rng as RandRng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

pub mod checksum;
pub mod ids;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Tick = Tick(0);
    #[inline]
    pub fn next(self) -> Tick {
        Tick(self.0 + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimMode {
    Running,
    Paused,
    Stepping(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct SimConfig {
    pub tick_rate_hz: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self { tick_rate_hz: 60 }
    }
}

impl SimConfig {
    pub fn tick_dt(&self) -> Duration {
        Duration::from_nanos(1_000_000_000 / u64::from(self.tick_rate_hz))
    }
    pub fn tick_dt_ms(&self) -> f64 {
        1000.0 / f64::from(self.tick_rate_hz)
    }
}

/// Deterministic seeded RNG. Wrap a `Xoshiro256StarStar` so callers can't access
/// state-mutation paths that bypass the recorder.
#[derive(Debug)]
pub struct Rng {
    inner: Xoshiro256StarStar,
    seed: u64,
}

impl Rng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            inner: Xoshiro256StarStar::seed_from_u64(seed),
            seed,
        }
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }
    /// 32-byte snapshot of the internal state. Used for the M0 checksum scope.
    pub fn state_bytes(&self) -> [u8; 32] {
        let mut clone = self.inner.clone();
        let mut out = [0u8; 32];
        for chunk in out.chunks_exact_mut(8) {
            chunk.copy_from_slice(&clone.next_u64().to_le_bytes());
        }
        out
    }
}

/// Wall-clock helper. `now_utc` is the only sanctioned source of `SystemTime` data;
/// sim systems must not call it directly.
#[derive(Debug, Default, Clone, Copy)]
pub struct WallClock;

impl WallClock {
    pub fn now_utc(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
    pub fn now_instant(&self) -> Instant {
        Instant::now()
    }
}

/// Fixed-tick scheduler. `tick(...)` advances by exactly one tick when allowed by mode.
#[derive(Debug)]
pub struct SimClock {
    tick: Tick,
    mode: SimMode,
    config: SimConfig,
}

impl SimClock {
    pub fn new(config: SimConfig) -> Self {
        Self {
            tick: Tick::ZERO,
            mode: SimMode::Running,
            config,
        }
    }
    pub fn tick(&self) -> Tick {
        self.tick
    }
    pub fn mode(&self) -> SimMode {
        self.mode
    }
    pub fn config(&self) -> SimConfig {
        self.config
    }
    pub fn pause(&mut self) {
        self.mode = SimMode::Paused;
    }
    pub fn resume(&mut self) {
        self.mode = SimMode::Running;
    }
    pub fn step(&mut self, ticks: u64) {
        if ticks == 0 {
            // L2: `step(0)` is a no-op. Previously the post-advance check `remaining <= 1`
            // still permitted one tick to elapse, which surprised callers asking for "no
            // step". Leave the current mode untouched.
            return;
        }
        self.mode = SimMode::Stepping(ticks);
    }
    /// Returns the new tick if the scheduler allowed an advance, or `None` if paused.
    pub fn advance(&mut self) -> Option<Tick> {
        match self.mode {
            SimMode::Running => {
                self.tick = self.tick.next();
                Some(self.tick)
            }
            SimMode::Stepping(remaining) => {
                self.tick = self.tick.next();
                if remaining <= 1 {
                    self.mode = SimMode::Paused;
                } else {
                    self.mode = SimMode::Stepping(remaining - 1);
                }
                Some(self.tick)
            }
            SimMode::Paused => None,
        }
    }
    pub fn sim_time_ms(&self) -> f64 {
        self.tick.0 as f64 * self.config.tick_dt_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_starts_at_zero_and_advances_when_running() {
        let mut clock = SimClock::new(SimConfig::default());
        assert_eq!(clock.tick(), Tick::ZERO);
        for expected in 1..=5 {
            assert_eq!(clock.advance(), Some(Tick(expected)));
        }
    }

    #[test]
    fn pause_blocks_advance_until_resume() {
        let mut clock = SimClock::new(SimConfig::default());
        clock.pause();
        assert!(clock.advance().is_none());
        clock.resume();
        assert_eq!(clock.advance(), Some(Tick(1)));
    }

    #[test]
    fn step_advances_n_then_pauses() {
        let mut clock = SimClock::new(SimConfig::default());
        clock.step(3);
        for expected in 1..=3 {
            assert_eq!(clock.advance(), Some(Tick(expected)));
        }
        assert!(clock.advance().is_none());
    }

    #[test]
    fn step_zero_is_a_no_op() {
        let mut clock = SimClock::new(SimConfig::default());
        clock.pause();
        clock.step(0);
        assert!(matches!(clock.mode(), SimMode::Paused));
        assert!(clock.advance().is_none());
        assert_eq!(clock.tick(), Tick::ZERO);

        let mut running = SimClock::new(SimConfig::default());
        running.step(0);
        // step(0) on a running clock leaves it running; the tick advances naturally on the
        // next advance() call (still one tick, not "one extra tick from step").
        assert!(matches!(running.mode(), SimMode::Running));
    }

    #[test]
    fn rng_is_deterministic_for_a_seed() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_eq!(a.state_bytes(), b.state_bytes());
    }

    #[test]
    fn rng_diverges_on_different_seeds() {
        let mut a = Rng::from_seed(1);
        let mut b = Rng::from_seed(2);
        let a0 = a.next_u64();
        let b0 = b.next_u64();
        assert_ne!(a0, b0);
    }

    #[test]
    fn tick_dt_matches_60_hz() {
        let cfg = SimConfig::default();
        let nanos = cfg.tick_dt().as_nanos();
        assert!((16_600_000..=16_700_000).contains(&nanos));
    }

    #[test]
    fn tick_dt_120_hz() {
        let cfg = SimConfig { tick_rate_hz: 120 };
        let nanos = cfg.tick_dt().as_nanos();
        assert!((8_300_000..=8_400_000).contains(&nanos));
    }
}
