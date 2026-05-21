//! **M15B** § GPU vs CPU determinism divergence detector.
//!
//! Per the M15B spec § "Crates / modules touched":
//! > `cf-physics::determinism` — MODIFY: Replay verifier byte-compares
//! > CPU vs GPU output per N ticks; flags first divergence.
//!
//! And § acceptance criterion 7:
//! > Given an artificially-induced GPU drift (test harness only)
//! > When per-tick checksum diverges
//! > Then material_gpu_cpu_divergence_detected fires at the first
//! > divergent tick
//! > And the engine pauses + dumps both states for forensics
//!
//! The divergence detector is a pure-function comparator: it takes two
//! per-tick checksum traces (one from the GPU path, one from the CPU
//! truth path) and returns the first divergent sample as a structured
//! [`DivergenceEvent`]. The engine layer routes this into the
//! `material.gpu_cpu_divergence_detected` recorder event and the
//! pause + state-dump action.
//!
//! ## Why "physics" and not "material-gpu"?
//!
//! The spec literally pins this to `cf-physics::determinism` because:
//! 1. The replay-verifier surface lives in cf-physics (DR-052
//!    deterministic physics is the canonical truth).
//! 2. The divergence detector is also useful for non-material GPU paths
//!    (M48+ exotic compute), so it belongs in the broader physics
//!    crate's determinism module rather than the material-specific
//!    `cf-material-gpu`.
//!
//! ## Determinism guarantees
//!
//! - No `thread_rng`, no `f64`, no `unsafe`.
//! - Pure function: same inputs → same outputs across runs.
//! - The comparator walks both traces in lock-step; mismatched lengths
//!   are reported as a length divergence (NOT silently truncated).

use serde::{Deserialize, Serialize};

/// One per-tick checksum sample from a kernel path. Matches the shape
/// of `cf_material_gpu::KernelChecksum` but doesn't depend on it (kept
/// independent so cf-physics doesn't pull cf-material-gpu into its
/// graph; the engine layer translates between the two).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChecksumSample {
    pub tick: u64,
    /// Backend name. Per spec § acceptance scenario 6 vocabulary:
    /// `"gpu"` or `"cpu_fallback"`.
    pub backend: String,
    /// 32-byte blake3 hash.
    pub bytes: [u8; 32],
}

impl ChecksumSample {
    #[must_use]
    pub fn new(tick: u64, backend: impl Into<String>, bytes: [u8; 32]) -> Self {
        Self {
            tick,
            backend: backend.into(),
            bytes,
        }
    }
}

/// Result of one comparison pass between a GPU trace and a CPU trace.
/// `Match` means every per-tick sample agreed; `Diverged` carries the
/// first divergent tick and both backends' bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DivergenceCheckResult {
    /// All compared samples agreed.
    Match { ticks_compared: u32 },
    /// First divergent tick. Per spec § acceptance scenario 7: "fires
    /// at the first divergent tick".
    Diverged(DivergenceEvent),
}

/// Structured event the engine layer routes to
/// `material.gpu_cpu_divergence_detected`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceEvent {
    /// Sim tick at which the divergence was observed.
    pub tick: u64,
    /// Backend the GPU trace identified itself as (typically `"gpu"`).
    pub gpu_backend: String,
    /// Backend the CPU trace identified itself as (typically
    /// `"cpu_fallback"`).
    pub cpu_backend: String,
    /// 32-byte blake3 hash from the GPU path.
    pub gpu_bytes: [u8; 32],
    /// 32-byte blake3 hash from the CPU path.
    pub cpu_bytes: [u8; 32],
    /// Human-readable reason. One of:
    /// - `"length_mismatch"` — the two traces have different lengths,
    ///   so we cannot compare past the shorter one.
    /// - `"tick_skew"` — same index but different tick numbers (shouldn't
    ///   happen if both kernels were stepped in lock-step).
    /// - `"byte_mismatch"` — same tick but different checksum bytes
    ///   (the canonical case).
    pub reason: String,
}

impl DivergenceEvent {
    #[must_use]
    pub fn gpu_hex(&self) -> String {
        hex_encode(&self.gpu_bytes)
    }

    #[must_use]
    pub fn cpu_hex(&self) -> String {
        hex_encode(&self.cpu_bytes)
    }

    /// **M15B** § Build the recorder payload JSON for this divergence
    /// event. The shape matches `cf-replay/schemas/event/
    /// material_gpu_cpu_divergence_detected.json`. The engine layer
    /// plugs this into
    /// `Recorder::record(tick, sim_time_ms, "material",
    /// "gpu_cpu_divergence_detected", payload, None)`.
    #[must_use]
    pub fn to_recorder_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "gpu_backend": self.gpu_backend,
            "cpu_backend": self.cpu_backend,
            "gpu_checksum_hex": self.gpu_hex(),
            "cpu_checksum_hex": self.cpu_hex(),
            "reason": self.reason,
        })
    }
}

/// **M15B** § GPU vs CPU divergence detector. Walks both traces in
/// lock-step and returns the first divergence event (or `Match` when
/// every sample agreed).
///
/// Per spec § acceptance scenario 7:
/// > When per-tick checksum diverges Then
/// > material_gpu_cpu_divergence_detected fires at the first divergent
/// > tick
#[must_use]
pub fn detect_first_divergence(gpu: &[ChecksumSample], cpu: &[ChecksumSample]) -> DivergenceCheckResult {
    let n = gpu.len().min(cpu.len()) as u32;
    let mut compared = 0u32;
    for i in 0..(n as usize) {
        let g = &gpu[i];
        let c = &cpu[i];
        if g.tick != c.tick {
            return DivergenceCheckResult::Diverged(DivergenceEvent {
                tick: g.tick.min(c.tick),
                gpu_backend: g.backend.clone(),
                cpu_backend: c.backend.clone(),
                gpu_bytes: g.bytes,
                cpu_bytes: c.bytes,
                reason: "tick_skew".to_string(),
            });
        }
        if g.bytes != c.bytes {
            return DivergenceCheckResult::Diverged(DivergenceEvent {
                tick: g.tick,
                gpu_backend: g.backend.clone(),
                cpu_backend: c.backend.clone(),
                gpu_bytes: g.bytes,
                cpu_bytes: c.bytes,
                reason: "byte_mismatch".to_string(),
            });
        }
        compared = compared.saturating_add(1);
    }
    if gpu.len() != cpu.len() {
        let (longer, shorter) = if gpu.len() > cpu.len() { (gpu, cpu) } else { (cpu, gpu) };
        let next = &longer[shorter.len()];
        return DivergenceCheckResult::Diverged(DivergenceEvent {
            tick: next.tick,
            gpu_backend: gpu
                .first()
                .map_or_else(|| "gpu".to_string(), |s| s.backend.clone()),
            cpu_backend: cpu
                .first()
                .map_or_else(|| "cpu_fallback".to_string(), |s| s.backend.clone()),
            gpu_bytes: gpu.last().map_or([0u8; 32], |s| s.bytes),
            cpu_bytes: cpu.last().map_or([0u8; 32], |s| s.bytes),
            reason: "length_mismatch".to_string(),
        });
    }
    DivergenceCheckResult::Match {
        ticks_compared: compared,
    }
}

/// **M15B** § Per-tick incremental detector. The engine pushes one GPU
/// sample + one CPU sample per tick; the detector returns
/// `Some(DivergenceEvent)` the moment the two disagree (and
/// short-circuits all future samples until reset).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DivergenceDetector {
    /// Per-tick incrementing buffer of GPU samples.
    pub gpu_samples: Vec<ChecksumSample>,
    /// Per-tick incrementing buffer of CPU samples.
    pub cpu_samples: Vec<ChecksumSample>,
    /// Latched divergence — once set, the detector won't change its
    /// answer until [`reset`].
    pub latched: Option<DivergenceEvent>,
    /// Cap on the per-side sample buffer (default 600 ticks per spec
    /// § acceptance scenario 1 horizon).
    pub trace_cap: usize,
    /// True when the detector should pause the engine + dump state per
    /// spec § acceptance scenario 7. Set to `true` the same tick the
    /// latched divergence fires.
    pub pause_on_divergence: bool,
}

impl DivergenceDetector {
    pub const DEFAULT_TRACE_CAP: usize = 600;

    #[must_use]
    pub fn new() -> Self {
        Self {
            trace_cap: Self::DEFAULT_TRACE_CAP,
            pause_on_divergence: false,
            ..Default::default()
        }
    }

    /// Push a GPU sample. If a CPU sample at the same tick already
    /// exists, run the comparator; if a divergence emerges, latch it +
    /// optionally flag pause_on_divergence.
    pub fn push_gpu(&mut self, sample: ChecksumSample) -> Option<&DivergenceEvent> {
        if self.gpu_samples.len() < self.trace_cap {
            self.gpu_samples.push(sample);
        }
        self.check_after_push()
    }

    /// Push a CPU sample. Same semantics as [`push_gpu`].
    pub fn push_cpu(&mut self, sample: ChecksumSample) -> Option<&DivergenceEvent> {
        if self.cpu_samples.len() < self.trace_cap {
            self.cpu_samples.push(sample);
        }
        self.check_after_push()
    }

    fn check_after_push(&mut self) -> Option<&DivergenceEvent> {
        if self.latched.is_some() {
            return self.latched.as_ref();
        }
        // The incremental detector ONLY compares paired samples (the
        // overlap between the two buffers). A length mismatch at this
        // point is expected — one side is just ahead of the other. The
        // detector waits for the lagging side to catch up before
        // declaring divergence.
        let n = self.gpu_samples.len().min(self.cpu_samples.len());
        if n == 0 {
            return None;
        }
        let gpu_slice = &self.gpu_samples[..n];
        let cpu_slice = &self.cpu_samples[..n];
        match detect_first_divergence(gpu_slice, cpu_slice) {
            DivergenceCheckResult::Diverged(evt) => {
                self.pause_on_divergence = true;
                self.latched = Some(evt);
                self.latched.as_ref()
            }
            DivergenceCheckResult::Match { .. } => None,
        }
    }

    /// Reset the detector — used by tests + by the engine after a state
    /// dump.
    pub fn reset(&mut self) {
        self.gpu_samples.clear();
        self.cpu_samples.clear();
        self.latched = None;
        self.pause_on_divergence = false;
    }

    /// True when a divergence has been latched.
    #[must_use]
    pub fn diverged(&self) -> bool {
        self.latched.is_some()
    }

    /// **M15B** § Forensics state-dump. Per spec § acceptance scenario 7:
    /// > And the engine pauses + dumps both states for forensics
    ///
    /// Returns a deterministic, JSON-serializable forensics bundle
    /// containing the latched divergence event + the full pre-mismatch
    /// agreement window from both backends. The engine layer writes
    /// this to `<run_bundle>/diagnostics/material_gpu_cpu_divergence_<tick>.json`
    /// when a divergence fires.
    #[must_use]
    pub fn forensics_dump(&self) -> ForensicsDump {
        ForensicsDump {
            divergence: self.latched.clone(),
            gpu_trace: self.gpu_samples.clone(),
            cpu_trace: self.cpu_samples.clone(),
        }
    }
}

/// **M15B** § Forensics state-dump returned by [`DivergenceDetector::forensics_dump`].
/// JSON-serializable so the engine layer can persist it to disk and
/// surface it via cfctl. Per spec § acceptance scenario 7 "the engine
/// pauses + dumps both states for forensics".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForensicsDump {
    /// The latched divergence event (None if no divergence yet).
    pub divergence: Option<DivergenceEvent>,
    /// The full GPU-side checksum trace at the moment of dump.
    pub gpu_trace: Vec<ChecksumSample>,
    /// The full CPU-side checksum trace at the moment of dump.
    pub cpu_trace: Vec<ChecksumSample>,
}

impl ForensicsDump {
    /// Serialize the dump to a pretty-printed JSON string suitable for
    /// the `<run_bundle>/diagnostics/material_gpu_cpu_divergence_<tick>.json`
    /// file.
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// True when the dump carries a latched divergence.
    #[must_use]
    pub fn has_divergence(&self) -> bool {
        self.divergence.is_some()
    }
}

/// Small static hex encoder (mirrors the one in cf-material-gpu;
/// duplicated to keep cf-physics free of the cf-material-gpu dep).
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xF) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tick: u64, backend: &str, fill: u8) -> ChecksumSample {
        ChecksumSample::new(tick, backend, [fill; 32])
    }

    /// VAL-M15B-det-001: empty traces match (no work done = no
    /// divergence).
    #[test]
    fn empty_traces_match() {
        let r = detect_first_divergence(&[], &[]);
        match r {
            DivergenceCheckResult::Match { ticks_compared } => assert_eq!(ticks_compared, 0),
            DivergenceCheckResult::Diverged(_) => panic!("expected Match"),
        }
    }

    /// VAL-M15B-det-002: identical traces match.
    #[test]
    fn identical_traces_match() {
        let gpu = vec![sample(0, "gpu", 1), sample(1, "gpu", 2), sample(2, "gpu", 3)];
        let cpu = vec![
            sample(0, "cpu_fallback", 1),
            sample(1, "cpu_fallback", 2),
            sample(2, "cpu_fallback", 3),
        ];
        let r = detect_first_divergence(&gpu, &cpu);
        match r {
            DivergenceCheckResult::Match { ticks_compared } => assert_eq!(ticks_compared, 3),
            DivergenceCheckResult::Diverged(e) => panic!("expected Match, got Diverged({e:?})"),
        }
    }

    /// VAL-M15B-det-003: byte mismatch on tick 1 surfaces the
    /// divergence event with reason "byte_mismatch".
    #[test]
    fn byte_mismatch_surfaces_divergence() {
        let gpu = vec![sample(0, "gpu", 1), sample(1, "gpu", 2)];
        let cpu = vec![sample(0, "cpu_fallback", 1), sample(1, "cpu_fallback", 99)];
        let r = detect_first_divergence(&gpu, &cpu);
        match r {
            DivergenceCheckResult::Diverged(e) => {
                assert_eq!(e.tick, 1);
                assert_eq!(e.reason, "byte_mismatch");
                assert_eq!(e.gpu_backend, "gpu");
                assert_eq!(e.cpu_backend, "cpu_fallback");
            }
            DivergenceCheckResult::Match { .. } => panic!("expected Diverged"),
        }
    }

    /// VAL-M15B-det-004: length mismatch surfaces divergence.
    #[test]
    fn length_mismatch_surfaces_divergence() {
        let gpu = vec![sample(0, "gpu", 1), sample(1, "gpu", 2), sample(2, "gpu", 3)];
        let cpu = vec![sample(0, "cpu_fallback", 1), sample(1, "cpu_fallback", 2)];
        let r = detect_first_divergence(&gpu, &cpu);
        match r {
            DivergenceCheckResult::Diverged(e) => {
                assert_eq!(e.reason, "length_mismatch");
                assert_eq!(e.tick, 2);
            }
            DivergenceCheckResult::Match { .. } => panic!("expected Diverged"),
        }
    }

    /// VAL-M15B-det-005: tick skew surfaces divergence.
    #[test]
    fn tick_skew_surfaces_divergence() {
        let gpu = vec![sample(0, "gpu", 1), sample(2, "gpu", 2)];
        let cpu = vec![sample(0, "cpu_fallback", 1), sample(1, "cpu_fallback", 2)];
        let r = detect_first_divergence(&gpu, &cpu);
        match r {
            DivergenceCheckResult::Diverged(e) => {
                assert_eq!(e.reason, "tick_skew");
            }
            DivergenceCheckResult::Match { .. } => panic!("expected Diverged"),
        }
    }

    /// VAL-M15B-det-006: hex helpers produce stable strings.
    #[test]
    fn hex_helpers_are_stable() {
        let e = DivergenceEvent {
            tick: 7,
            gpu_backend: "gpu".to_string(),
            cpu_backend: "cpu_fallback".to_string(),
            gpu_bytes: [0xab; 32],
            cpu_bytes: [0xcd; 32],
            reason: "byte_mismatch".to_string(),
        };
        assert_eq!(e.gpu_hex().len(), 64);
        assert!(e.gpu_hex().starts_with("ab"));
        assert!(e.cpu_hex().starts_with("cd"));
    }

    /// VAL-M15B-det-007: incremental detector latches a divergence after
    /// it sees one + sets pause_on_divergence per spec § scenario 7.
    #[test]
    fn incremental_detector_latches_pause_flag() {
        let mut d = DivergenceDetector::new();
        d.push_gpu(sample(0, "gpu", 1));
        d.push_cpu(sample(0, "cpu_fallback", 1));
        assert!(!d.diverged());
        d.push_gpu(sample(1, "gpu", 2));
        d.push_cpu(sample(1, "cpu_fallback", 99));
        assert!(d.diverged());
        assert!(d.pause_on_divergence, "engine must pause on divergence");
        let event = d.latched.as_ref().unwrap();
        assert_eq!(event.tick, 1);
        assert_eq!(event.reason, "byte_mismatch");
    }

    /// VAL-M15B-det-008: reset clears the latched state.
    #[test]
    fn reset_clears_state() {
        let mut d = DivergenceDetector::new();
        d.push_gpu(sample(0, "gpu", 1));
        d.push_cpu(sample(0, "cpu_fallback", 99));
        assert!(d.diverged());
        d.reset();
        assert!(!d.diverged());
        assert!(!d.pause_on_divergence);
        assert!(d.gpu_samples.is_empty());
        assert!(d.cpu_samples.is_empty());
    }

    /// VAL-M15B-det-009: trace cap bounds the buffer.
    #[test]
    fn trace_cap_bounds_buffer() {
        let mut d = DivergenceDetector::new();
        d.trace_cap = 4;
        for t in 0..10 {
            d.push_gpu(sample(t, "gpu", t as u8));
            d.push_cpu(sample(t, "cpu_fallback", t as u8));
        }
        assert_eq!(d.gpu_samples.len(), 4);
        assert_eq!(d.cpu_samples.len(), 4);
    }

    /// VAL-M15B-det-010: DivergenceCheckResult round-trips via serde.
    #[test]
    fn check_result_round_trips() {
        let r = DivergenceCheckResult::Match { ticks_compared: 7 };
        let j = serde_json::to_string(&r).unwrap();
        let back: DivergenceCheckResult = serde_json::from_str(&j).unwrap();
        assert_eq!(r, back);

        let e = DivergenceCheckResult::Diverged(DivergenceEvent {
            tick: 3,
            gpu_backend: "gpu".to_string(),
            cpu_backend: "cpu_fallback".to_string(),
            gpu_bytes: [1u8; 32],
            cpu_bytes: [2u8; 32],
            reason: "byte_mismatch".to_string(),
        });
        let j = serde_json::to_string(&e).unwrap();
        let back: DivergenceCheckResult = serde_json::from_str(&j).unwrap();
        assert_eq!(e, back);
    }

    /// VAL-M15B-det-011: a 600-tick agreement window stays in `Match`.
    /// Per spec § acceptance scenario 1.
    #[test]
    fn match_holds_over_600_ticks() {
        let mut gpu = Vec::with_capacity(600);
        let mut cpu = Vec::with_capacity(600);
        for t in 0u64..600u64 {
            gpu.push(sample(t, "gpu", (t & 0xff) as u8));
            cpu.push(sample(t, "cpu_fallback", (t & 0xff) as u8));
        }
        match detect_first_divergence(&gpu, &cpu) {
            DivergenceCheckResult::Match { ticks_compared } => assert_eq!(ticks_compared, 600),
            DivergenceCheckResult::Diverged(e) => panic!("expected Match, got Diverged({e:?})"),
        }
    }

    /// VAL-M15B-det-012: forensics_dump carries the latched event and
    /// both backends' full traces. Per spec § acceptance scenario 7:
    /// > And the engine pauses + dumps both states for forensics
    #[test]
    fn forensics_dump_returns_latched_event_and_traces() {
        let mut d = DivergenceDetector::new();
        // Ticks 0..3 agree.
        for t in 0u64..3 {
            d.push_gpu(sample(t, "gpu", t as u8));
            d.push_cpu(sample(t, "cpu_fallback", t as u8));
        }
        // Tick 3: divergence.
        d.push_gpu(sample(3, "gpu", 99));
        d.push_cpu(sample(3, "cpu_fallback", 3));
        let dump = d.forensics_dump();
        assert!(dump.has_divergence(), "dump must carry the latched event");
        assert_eq!(dump.divergence.as_ref().unwrap().tick, 3);
        assert_eq!(dump.gpu_trace.len(), 4);
        assert_eq!(dump.cpu_trace.len(), 4);
    }

    /// VAL-M15B-det-013: forensics dump JSON serializes cleanly + the
    /// pretty-printed output is non-empty.
    #[test]
    fn forensics_dump_pretty_json_round_trips() {
        let mut d = DivergenceDetector::new();
        d.push_gpu(sample(0, "gpu", 1));
        d.push_cpu(sample(0, "cpu_fallback", 99));
        let dump = d.forensics_dump();
        let json = dump.to_pretty_json().expect("pretty json");
        assert!(json.contains("\"divergence\""));
        assert!(json.contains("\"gpu_trace\""));
        assert!(json.contains("\"cpu_trace\""));
        let back: ForensicsDump = serde_json::from_str(&json).expect("round trip");
        assert_eq!(back, dump);
    }

    /// VAL-M15B-det-014: forensics_dump with no divergence still
    /// returns the agreement window (useful for debugging in-flight
    /// determinism checks).
    #[test]
    fn forensics_dump_without_divergence_returns_agreement_window() {
        let mut d = DivergenceDetector::new();
        for t in 0u64..5 {
            d.push_gpu(sample(t, "gpu", t as u8));
            d.push_cpu(sample(t, "cpu_fallback", t as u8));
        }
        let dump = d.forensics_dump();
        assert!(!dump.has_divergence());
        assert_eq!(dump.gpu_trace.len(), 5);
        assert_eq!(dump.cpu_trace.len(), 5);
    }
}
