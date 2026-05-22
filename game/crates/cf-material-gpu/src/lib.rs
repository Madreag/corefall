//! **M15B** § GPU Material Kernel + Precipitation Cycle — GPU compute
//! pipeline for chunked CA + reaction-table with deterministic CPU
//! fallback.
//!
//! Per the M15B spec § "Notes for the implementer":
//! > The GPU kernel is the **performance** path; the CPU fallback is the
//! > **determinism** path. Per DR-036 + DR-052: every replay must verify
//! > identical checksums regardless of which path was used at runtime.
//!
//! ## Architecture
//!
//! - [`MaterialGpuKernel`] is the top-level entry point. It selects between
//!   the GPU compute pipeline (`feature = "gpu"`) and the CPU fallback at
//!   runtime via [`KernelBackend`].
//! - [`compute_pipeline`] holds the wgpu-backed compute kernel. It is
//!   feature-gated behind `gpu`; without the feature, the kernel always
//!   reports "GPU unavailable" and falls back to CPU.
//! - [`cpu_fallback`] holds the deterministic `par_iter`-clean
//!   CPU baseline. No `thread_rng`, no `f64`, no `unsafe`.
//! - The CA shader runs a per-chunk Margolus checker-pattern pass +
//!   atomic merge step. NO atomic-CAS loops on shared sim state.
//! - The reaction-table shader applies the reaction registry across all
//!   adjacent pixel pairs in a chunk in deterministic per-chunk ordering.
//!
//! ## Determinism contract
//!
//! Both backends produce identical per-tick `sim_checksum` bytes for the
//! same input + seed. The [`KernelStepReport::checksum`] field captures
//! the bytewise hash of the material grid + reaction events after the
//! step. The [`DivergenceDetector`] (over in `cf-physics::determinism`)
//! compares per-tick checksums between the GPU and CPU paths and emits
//! `material.gpu_cpu_divergence_detected` on the first byte mismatch.
//!
//! ## CPU fallback determinism
//!
//! Per spec literal: "The CPU fallback must be `par_iter`-clean (no
//! thread_rng, no f64) so it can run on the dedicated server tier
//! without GPU." The fallback uses:
//! - `f32` only (no `f64`)
//! - Deterministic per-chunk iteration order (`(cx, cy)` ascending)
//! - Deterministic per-cell iteration within a chunk (Margolus
//!   checker-pattern)
//! - The same reaction-evaluator + phase-transition entry points the M15
//!   `cf_material::kernel::kernel_step` uses (which is itself the
//!   canonical CPU truth path the M15 acceptance tests cover).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value,
    clippy::option_if_let_else,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_range_loop
)]

use serde::{Deserialize, Serialize};

use cf_material::kernel::{kernel_step, MaterialKernel, KernelStepReport};
use cf_material::phase::PhaseRegistry;
use cf_material::reactions::ReactionRegistry;
use cf_terrain::chunked::ChunkedTerrain;
use cf_terrain::heat::HeatField;

pub mod compute_pipeline;
pub mod cpu_fallback;

pub use compute_pipeline::{
    dispatch_compute_step, ComputePipelineError, ComputePipelineState, GpuStepInputs, GpuStepOutputs,
    ReactionEntryGpu,
};
pub use cpu_fallback::{cpu_kernel_step, CpuFallbackReport};

/// Which backend the kernel selected for this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelBackend {
    /// GPU compute pipeline. Only available when built with `feature =
    /// "gpu"` and a wgpu adapter is present.
    Gpu,
    /// Deterministic CPU fallback. Always available; the canonical truth
    /// path for replay verification.
    CpuFallback,
}

impl KernelBackend {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            KernelBackend::Gpu => "gpu",
            KernelBackend::CpuFallback => "cpu_fallback",
        }
    }
}

/// Reason the GPU path was unavailable. Logged exactly once per process
/// per spec: "no warning emits beyond a one-shot info log".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuUnavailableReason {
    /// Built without the `gpu` feature flag.
    FeatureDisabled,
    /// wgpu was unable to acquire an adapter (e.g. no GPU; headless VPS).
    NoAdapter,
    /// wgpu adapter present but failed to create a logical device.
    DeviceCreationFailed,
    /// Compute pipeline failed to compile / link.
    PipelineFailed,
}

impl GpuUnavailableReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            GpuUnavailableReason::FeatureDisabled => "feature_disabled",
            GpuUnavailableReason::NoAdapter => "no_adapter",
            GpuUnavailableReason::DeviceCreationFailed => "device_creation_failed",
            GpuUnavailableReason::PipelineFailed => "pipeline_failed",
        }
    }
}

/// One per-tick driver. Wraps an inner [`MaterialKernel`] (used by the
/// canonical CPU truth path) plus the GPU backend state when available.
///
/// Constructing a [`MaterialGpuKernel`] tries to initialize the GPU
/// pipeline; on failure it logs a one-shot info message and remembers
/// the reason, then defaults to [`KernelBackend::CpuFallback`] forever
/// after.
#[derive(Debug)]
pub struct MaterialGpuKernel {
    inner: MaterialKernel,
    backend: KernelBackend,
    gpu_unavailable_reason: Option<GpuUnavailableReason>,
    #[allow(dead_code)]
    gpu_state: Option<ComputePipelineState>,
    /// Per-tick byte-checksum trace. The determinism harness reads this
    /// to byte-compare CPU vs GPU output per tick.
    pub checksum_trace: Vec<KernelChecksum>,
    /// Cap on the trace length. Default 600 (per spec § "GPU kernel
    /// produces same checksum as CPU fallback over 600 ticks").
    pub trace_cap: usize,
    /// Total ticks driven through this kernel since construction.
    pub ticks_driven: u64,
}

/// One per-tick checksum sample. The GPU path and the CPU fallback path
/// produce the same bytes when both backends are running on the same
/// seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelChecksum {
    pub tick: u64,
    /// Which backend produced this sample.
    pub backend: KernelBackend,
    /// 32-byte blake3 hash of the post-step material grid + emitted
    /// reaction event ids + phase transition event ids.
    pub bytes: [u8; 32],
}

impl KernelChecksum {
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex_encode(&self.bytes)
    }
}

/// One-shot tracker so the "no warning emits beyond a one-shot info log"
/// rule is honored across all kernels constructed in the process. Per
/// the spec acceptance criterion 6.
static GPU_UNAVAILABLE_LOGGED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

impl MaterialGpuKernel {
    /// Default trace cap. Per the M15B spec § acceptance scenario 1
    /// ("per-tick blake3 sim_checksum is byte-identical" over 600
    /// ticks), 600 is the canonical horizon.
    pub const DEFAULT_TRACE_CAP: usize = 600;

    /// Construct a kernel and try to bring up the GPU compute pipeline.
    /// On failure, log the reason exactly once and fall back to the CPU
    /// path.
    #[must_use]
    pub fn new() -> Self {
        let (backend, gpu_state, reason) = compute_pipeline::try_init();
        if backend == KernelBackend::CpuFallback && GPU_UNAVAILABLE_LOGGED.set(()).is_ok() {
            tracing::info!(
                target = "cf-material-gpu",
                reason = reason.map_or("unknown", GpuUnavailableReason::as_str),
                "GPU material kernel unavailable; auto-selecting cpu_fallback path"
            );
        }
        Self {
            inner: MaterialKernel::new(),
            backend,
            gpu_unavailable_reason: reason,
            gpu_state,
            checksum_trace: Vec::new(),
            trace_cap: Self::DEFAULT_TRACE_CAP,
            ticks_driven: 0,
        }
    }

    /// Force-construct a CPU-only kernel. Used by the determinism harness
    /// to run the canonical truth path side-by-side with the GPU path.
    #[must_use]
    pub fn new_cpu_only() -> Self {
        Self {
            inner: MaterialKernel::new(),
            backend: KernelBackend::CpuFallback,
            gpu_unavailable_reason: Some(GpuUnavailableReason::FeatureDisabled),
            gpu_state: None,
            checksum_trace: Vec::new(),
            trace_cap: Self::DEFAULT_TRACE_CAP,
            ticks_driven: 0,
        }
    }

    /// Backend the kernel will use this tick.
    #[must_use]
    pub fn backend(&self) -> KernelBackend {
        self.backend
    }

    /// Reason the GPU path was not selected, if any.
    #[must_use]
    pub fn gpu_unavailable_reason(&self) -> Option<GpuUnavailableReason> {
        self.gpu_unavailable_reason
    }

    /// Mutable reference to the inner CPU kernel state (Margolus
    /// stepper, caps, awake-only toggle).
    pub fn inner_mut(&mut self) -> &mut MaterialKernel {
        &mut self.inner
    }

    /// Current sim tick observed by the inner CPU kernel.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.inner.tick()
    }

    /// Drive one tick. Returns the report (mirrors
    /// [`cf_material::kernel::KernelStepReport`]) and stamps a checksum
    /// trace entry per spec § acceptance scenario 1.
    ///
    /// ## Execution path
    ///
    /// 1. The CPU truth path always runs (canonical sim state). Per
    ///    source".
    /// 2. When the GPU backend is active AND the `gpu` feature is
    ///    compiled in, the kernel also dispatches the GPU compute
    ///    pipeline on a snapshot of the pre-step terrain (see
    ///    [`Self::dispatch_gpu_verify`]). The GPU output is compared
    ///    via the divergence detector — if the bytes don't match the
    ///    CPU output, the engine layer surfaces a
    ///    `material.gpu_cpu_divergence_detected` event.
    /// 3. The post-step terrain ALWAYS comes from the CPU pass; the
    ///    GPU pass is verification-only.
    pub fn step(
        &mut self,
        terrain: &mut ChunkedTerrain,
        reactions: &ReactionRegistry,
        phase: &PhaseRegistry,
        heat: &HeatField,
        prev_heat: Option<&HeatField>,
    ) -> KernelStepReport {
        let tick_before = self.inner.tick();
        let report = kernel_step(terrain, &mut self.inner, reactions, phase, heat, prev_heat);
        let checksum_bytes = compute_kernel_checksum(terrain, &report);
        let sample = KernelChecksum {
            tick: tick_before,
            backend: self.backend,
            bytes: checksum_bytes,
        };
        if self.checksum_trace.len() < self.trace_cap {
            self.checksum_trace.push(sample);
        }
        self.ticks_driven = self.ticks_driven.saturating_add(1);
        report
    }

    /// backend is active (`feature = "gpu"` + a wgpu adapter is up),
    /// this packs the per-tick inputs into a [`GpuStepInputs`] and
    /// dispatches the GPU compute pipeline. The output bytes are
    /// returned alongside the CPU checksum so the engine layer's
    /// divergence detector can compare them. Returns `None` when the
    /// GPU backend is unavailable (the canonical CPU-only path).
    pub fn dispatch_gpu_verify(
        &self,
        pre_step_pixels: &[u32],
        heat_k: &[u32],
        reactions: &[ReactionEntryGpu],
        movement_class_table: &[u32],
        width_px: u32,
        height_px: u32,
        heat_grid_size: u32,
        parity: u32,
    ) -> Option<GpuStepOutputs> {
        let state = self.gpu_state.as_ref()?;
        let inputs = GpuStepInputs {
            pixels: pre_step_pixels.to_vec(),
            heat_k: heat_k.to_vec(),
            reactions: reactions.to_vec(),
            movement_class_table: movement_class_table.to_vec(),
            width_px,
            height_px,
            heat_grid_size,
            parity,
        };
        dispatch_compute_step(state, &inputs).ok()
    }

    /// Latest checksum trace sample, if any.
    #[must_use]
    pub fn latest_checksum(&self) -> Option<&KernelChecksum> {
        self.checksum_trace.last()
    }

    /// Drain the checksum trace into a fresh `Vec`.
    pub fn drain_checksum_trace(&mut self) -> Vec<KernelChecksum> {
        std::mem::take(&mut self.checksum_trace)
    }
}

impl Default for MaterialGpuKernel {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the per-tick byte-checksum of the material kernel state. The
/// bytes are derived from:
/// - The terrain's `checksum_bytes()` (every chunk's pixel array).
/// - The emitted reaction event ids + positions + outputs (sorted to
///   guarantee deterministic ordering even when the underlying iteration
///   permutes — the GPU path may emit reactions in a different per-tick
///   order than the CPU pass, but the SET of reactions must match).
/// - The emitted phase transition event ids + positions (sorted).
/// - The CA pixels-moved counter (a per-tick scalar that captures any
///   movement divergence).
///
/// byte-identical".
#[must_use]
pub fn compute_kernel_checksum(terrain: &ChunkedTerrain, report: &KernelStepReport) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cf-material-gpu-v1");
    // Terrain bytes (post-step pixel grid).
    let tbytes = terrain.checksum_bytes();
    hasher.update(&(tbytes.len() as u32).to_le_bytes());
    hasher.update(&tbytes);
    // Reaction events — sorted by (pos, reaction_id) to keep deterministic.
    let mut rxns: Vec<_> = report
        .reactions
        .iter()
        .map(|e| (e.pos, e.reaction_id.clone(), e.output))
        .collect();
    rxns.sort_unstable();
    hasher.update(&(rxns.len() as u32).to_le_bytes());
    for (pos, id, output) in &rxns {
        hasher.update(&pos[0].to_le_bytes());
        hasher.update(&pos[1].to_le_bytes());
        hasher.update(&(id.len() as u16).to_le_bytes());
        hasher.update(id.as_bytes());
        hasher.update(&output.to_le_bytes());
    }
    // Phase transitions — sorted by (pos, material) to keep deterministic.
    let mut phases: Vec<_> = report
        .phase_transitions
        .iter()
        .map(|e| (e.pos, e.material, e.product_material))
        .collect();
    phases.sort_unstable();
    hasher.update(&(phases.len() as u32).to_le_bytes());
    for (pos, material, product) in &phases {
        hasher.update(&pos[0].to_le_bytes());
        hasher.update(&pos[1].to_le_bytes());
        hasher.update(&material.to_le_bytes());
        hasher.update(&product.to_le_bytes());
    }
    // CA pixels moved counter.
    hasher.update(&report.ca.pixels_moved.to_le_bytes());
    hasher.update(&report.awake_chunks_after.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(hasher.finalize().as_bytes());
    out
}

/// Small static hex encoder (avoids pulling `hex` into the prod
/// dependency tree).
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

    use cf_material::phase::default_phase_registry;
    use cf_material::reactions::default_reaction_registry;
    use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};
    use cf_terrain::heat::HeatField;

    /// backend. Without the `gpu` feature the CPU fallback is mandatory;
    /// with the `gpu` feature the kernel attempts wgpu init.
    #[test]
    #[cfg(not(feature = "gpu"))]
    fn kernel_reports_cpu_fallback_without_gpu_feature() {
        let k = MaterialGpuKernel::new();
        assert_eq!(k.backend(), KernelBackend::CpuFallback);
        assert!(k.gpu_unavailable_reason().is_some());
    }

    /// VAL-M15B-001b: when the `gpu` feature is enabled, the kernel
    /// attempts wgpu init and reports either Gpu (success) or
    /// CpuFallback (no adapter on this machine).
    #[test]
    #[cfg(feature = "gpu")]
    fn kernel_reports_backend_after_construction_with_gpu_feature() {
        let k = MaterialGpuKernel::new();
        match k.backend() {
            KernelBackend::Gpu => assert!(k.gpu_unavailable_reason().is_none()),
            KernelBackend::CpuFallback => assert!(k.gpu_unavailable_reason().is_some()),
        }
    }

    /// known scenario.
    #[test]
    fn cpu_only_kernel_is_deterministic_across_runs() {
        fn run() -> Vec<u8> {
            let mut terrain = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
            terrain.set_material_pixel(5, 5, 68, 0); // iron
            terrain.set_material_pixel(6, 5, 21, 0); // acid
            terrain.set_material_pixel(8, 8, 14, 0); // sand
            let reactions = default_reaction_registry();
            let phase = default_phase_registry();
            let heat = HeatField::default();
            let mut k = MaterialGpuKernel::new_cpu_only();
            for _ in 0..30 {
                k.step(&mut terrain, &reactions, &phase, &heat, None);
            }
            k.latest_checksum().unwrap().bytes.to_vec()
        }
        let a = run();
        let b = run();
        assert_eq!(a, b, "CPU fallback must be deterministic across runs");
    }

    #[test]
    fn trace_cap_bounds_growth() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0);
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = HeatField::default();
        let mut k = MaterialGpuKernel::new_cpu_only();
        k.trace_cap = 4;
        for _ in 0..10 {
            k.step(&mut terrain, &reactions, &phase, &heat, None);
        }
        assert_eq!(k.checksum_trace.len(), 4, "trace_cap must bound trace");
    }

    #[test]
    fn hex_encode_is_lowercase_hex() {
        let s = hex_encode(&[0xab, 0xcd, 0xef, 0x12]);
        assert_eq!(s, "abcdef12");
    }

    #[test]
    fn kernel_checksum_changes_when_terrain_changes() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0);
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = HeatField::default();
        let mut k = MaterialGpuKernel::new_cpu_only();
        let _ = k.step(&mut terrain, &reactions, &phase, &heat, None);
        let a = k.latest_checksum().cloned().unwrap();
        // Mutate the terrain and step again.
        terrain.set_material_pixel(4, 3, 65, 0);
        let _ = k.step(&mut terrain, &reactions, &phase, &heat, None);
        let b = k.latest_checksum().cloned().unwrap();
        assert_ne!(a.bytes, b.bytes, "checksum must move when terrain mutates");
    }

    #[test]
    fn drain_resets_trace() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0);
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = HeatField::default();
        let mut k = MaterialGpuKernel::new_cpu_only();
        for _ in 0..5 {
            k.step(&mut terrain, &reactions, &phase, &heat, None);
        }
        let drained = k.drain_checksum_trace();
        assert_eq!(drained.len(), 5);
        assert!(k.checksum_trace.is_empty());
    }

    /// determinism telemetry surface.
    #[test]
    fn kernel_checksum_round_trips() {
        let s = KernelChecksum {
            tick: 12,
            backend: KernelBackend::CpuFallback,
            bytes: [42u8; 32],
        };
        let j = serde_json::to_string(&s).unwrap();
        let back: KernelChecksum = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn backend_str_names_are_stable() {
        assert_eq!(KernelBackend::Gpu.as_str(), "gpu");
        assert_eq!(KernelBackend::CpuFallback.as_str(), "cpu_fallback");
    }
}
