//! M8A § Performance budgets — per-subsystem latency sampler.
//!
//! `PerfSampler` accumulates per-subsystem tick latency samples (in
//! microseconds) into rolling windows and computes p50/p99/p999 per
//! cadence emit. The samples drive the `perf.sample` cosmetic event +
//! the `summary.json.performance` per-subsystem keys consumed by
//! `cf-mod validate-bundle`.

use serde::{Deserialize, Serialize};

/// Per-subsystem latency window. Rolls 1024 samples; emits percentiles
/// per cadence-trigger.
#[derive(Debug, Clone, Default)]
pub struct WindowedSampler {
    samples: Vec<u64>,
    cap: usize,
}

impl WindowedSampler {
    pub fn new(cap: usize) -> Self {
        Self {
            samples: Vec::with_capacity(cap),
            cap,
        }
    }

    pub fn record(&mut self, sample_us: u64) {
        if self.samples.len() == self.cap {
            self.samples.remove(0);
        }
        self.samples.push(sample_us);
    }

    pub fn percentiles(&self) -> PerSubsystemPerf {
        if self.samples.is_empty() {
            return PerSubsystemPerf::default();
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        // **M14 audit fix** (pre-existing M8A bug): use `ceil` instead of
        // `round` so the 99th percentile of [0..100] resolves to 99 (not
        // 98). The previous round-half-even mapped 99*0.99 = 98.01 down to
        // 98, mis-reporting p99 by one unit. `ceil` gives the standard
        // nearest-rank percentile.
        let n_minus_1 = (sorted.len() - 1) as f64;
        let pct = |p: f64| -> u64 {
            let raw = n_minus_1 * p;
            let idx = raw.ceil() as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        PerSubsystemPerf {
            p50_us: pct(0.5),
            p99_us: pct(0.99),
            p999_us: pct(0.999),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Per-subsystem perf percentile snapshot. Lives under
/// `summary.json.performance.<subsystem>` and on the `perf.sample` event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerSubsystemPerf {
    pub p50_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
}

/// Per-subsystem PerfSampler bundle.
#[derive(Debug, Clone, Default)]
pub struct PerfSampler {
    pub actor: WindowedSampler,
    pub ai: WindowedSampler,
    pub projectile: WindowedSampler,
    pub terrain: WindowedSampler,
    pub mission: WindowedSampler,
    pub recorder: WindowedSampler,
    pub render: WindowedSampler,
}

impl PerfSampler {
    pub fn new(window_cap: usize) -> Self {
        Self {
            actor: WindowedSampler::new(window_cap),
            ai: WindowedSampler::new(window_cap),
            projectile: WindowedSampler::new(window_cap),
            terrain: WindowedSampler::new(window_cap),
            mission: WindowedSampler::new(window_cap),
            recorder: WindowedSampler::new(window_cap),
            render: WindowedSampler::new(window_cap),
        }
    }

    /// Emit a snapshot of per-subsystem p50/p99/p999. Used to fill
    /// `summary.json.performance` at run finish.
    pub fn snapshot(&self) -> M8aPerfSummary {
        M8aPerfSummary {
            actor: self.actor.percentiles(),
            ai: self.ai.percentiles(),
            projectile: self.projectile.percentiles(),
            terrain: self.terrain.percentiles(),
            mission: self.mission.percentiles(),
            recorder: self.recorder.percentiles(),
            render: self.render.percentiles(),
        }
    }
}

/// Per-subsystem perf summary. Surfaced under
/// `summary.json.performance.subsystems` (M8A additive extension; legacy
/// readers ignore the field).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct M8aPerfSummary {
    pub actor: PerSubsystemPerf,
    pub ai: PerSubsystemPerf,
    pub projectile: PerSubsystemPerf,
    pub terrain: PerSubsystemPerf,
    pub mission: PerSubsystemPerf,
    pub recorder: PerSubsystemPerf,
    pub render: PerSubsystemPerf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windowed_sampler_percentiles_basic() {
        let mut w = WindowedSampler::new(100);
        for v in 0..100u64 {
            w.record(v);
        }
        let p = w.percentiles();
        assert_eq!(p.p50_us, 50);
        assert_eq!(p.p99_us, 99);
    }

    #[test]
    fn windowed_sampler_drops_oldest_on_cap() {
        let mut w = WindowedSampler::new(3);
        w.record(1);
        w.record(2);
        w.record(3);
        w.record(4);
        assert_eq!(w.samples.len(), 3);
        assert_eq!(w.samples[0], 2);
    }

    #[test]
    fn perf_sampler_snapshot_round_trip() {
        let mut s = PerfSampler::new(10);
        s.actor.record(100);
        s.ai.record(200);
        let snap = s.snapshot();
        assert_eq!(snap.actor.p99_us, 100);
        assert_eq!(snap.ai.p99_us, 200);
    }
}
