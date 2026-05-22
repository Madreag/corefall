//! **M15B** § Deterministic CPU fallback for the GPU material kernel.
//!
//! Per the M15B spec § "Notes for the implementer":
//! > The CPU fallback must be `par_iter`-clean (no thread_rng, no f64) so
//! > it can run on the dedicated server tier without GPU.
//!
//! The fallback is a thin wrapper over the canonical
//! [`cf_material::kernel::kernel_step`] (which itself is the deterministic
//! truth-source the M15 acceptance tests pin). Wrapping rather than
//! reimplementing keeps the two paths in lock-step: if the M15 kernel's
//! reaction-evaluator changes, the fallback inherits the change for free
//! and the GPU-vs-CPU divergence detector still works.
//!
//! ## Why a thin wrapper?
//!
//! The performance gain from the GPU path is real (10×-50× per spec) but
//! the canonical TRUTH the divergence detector compares against is the
//! CPU kernel. The fallback module exists so:
//! 1. A server-tier deployment without GPU still runs the kernel
//!    (acceptance scenario 6).
//! 2. The determinism harness can byte-compare GPU output against the
//!    CPU result the network sim trusts (acceptance scenario 1).
//! 3. The crate compiles in `default-features = []` builds (no wgpu in
//!    the sim crates dependency graph).

use cf_material::kernel::{kernel_step, KernelStepReport, MaterialKernel};
use cf_material::phase::PhaseRegistry;
use cf_material::reactions::ReactionRegistry;
use cf_terrain::chunked::ChunkedTerrain;
use cf_terrain::heat::HeatField;

/// Per-tick report from a CPU fallback step. Mirrors
/// [`cf_material::kernel::KernelStepReport`] but bound to the
/// `cf-material-gpu` crate name for cleaner API discovery.
#[derive(Debug, Clone)]
pub struct CpuFallbackReport {
    pub inner: KernelStepReport,
    pub checksum: [u8; 32],
}

/// Drive one CPU fallback step. Returns the kernel report and the
/// post-step 32-byte checksum (per [`crate::compute_kernel_checksum`]).
pub fn cpu_kernel_step(
    terrain: &mut ChunkedTerrain,
    kernel: &mut MaterialKernel,
    reactions: &ReactionRegistry,
    phase: &PhaseRegistry,
    heat: &HeatField,
    prev_heat: Option<&HeatField>,
) -> CpuFallbackReport {
    let report = kernel_step(terrain, kernel, reactions, phase, heat, prev_heat);
    let checksum = crate::compute_kernel_checksum(terrain, &report);
    CpuFallbackReport { inner: report, checksum }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cf_material::phase::default_phase_registry;
    use cf_material::reactions::default_reaction_registry;
    use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};

    /// report shape.
    #[test]
    fn cpu_kernel_step_matches_kernel_step() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 68, 0); // iron
        terrain.set_material_pixel(4, 3, 21, 0); // acid
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = HeatField::default();
        let mut k = MaterialKernel::new();
        let r = cpu_kernel_step(&mut terrain, &mut k, &reactions, &phase, &heat, None);
        assert!(!r.inner.reactions.is_empty());
        assert_eq!(terrain.material_at(3, 3), 38, "iron→rust on CPU fallback");
    }

    #[test]
    fn cpu_kernel_step_checksum_deterministic() {
        fn one_run() -> [u8; 32] {
            let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
            terrain.set_material_pixel(3, 3, 68, 0);
            terrain.set_material_pixel(4, 3, 21, 0);
            let reactions = default_reaction_registry();
            let phase = default_phase_registry();
            let heat = HeatField::default();
            let mut k = MaterialKernel::new();
            let r = cpu_kernel_step(&mut terrain, &mut k, &reactions, &phase, &heat, None);
            r.checksum
        }
        assert_eq!(one_run(), one_run());
    }
}
