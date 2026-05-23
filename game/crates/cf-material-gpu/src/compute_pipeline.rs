//! **M15B** § GPU compute pipeline (wgpu) for the material kernel.
//!
//! The pipeline runs the per-chunk Margolus checker-pattern CA pass +
//! the per-pixel reaction-table pass on the GPU. Per spec § "Notes for
//! the implementer":
//!
//! > Material reactions on the GPU require deterministic ordering — use
//! > a per-chunk Margolus checker-pattern shader pass + atomic merge
//! > step. NO atomic-CAS loops on shared sim state.
//!
//! The shader sources live in `shaders/material_ca.wgsl` and
//! `shaders/reaction_table.wgsl`; they are `include_str!`-loaded so they
//! ship with the crate.
//!
//! ## Determinism contract
//!
//! Each invocation reads the current chunk grid, dispatches the
//! checker-pattern compute, reads back the post-step grid + emitted
//! events, and surfaces a 32-byte blake3 hash via
//! [`crate::compute_kernel_checksum`]. The hash MUST be byte-identical
//! to the CPU fallback's hash on the same input + seed (per the M15B
//! acceptance criterion 1).
//!
//! ## Auto-fallback
//!
//! [`try_init`] is the single entry point; it tries to bring up wgpu
//! when the `gpu` feature is enabled, and falls back to the CPU path
//! with a structured [`crate::GpuUnavailableReason`] otherwise. When
//! the `gpu` feature is disabled, this module is a pure stub that
//! always returns `(CpuFallback, None, FeatureDisabled)`.

use crate::{GpuUnavailableReason, KernelBackend};

/// Error variants from the GPU pipeline. Kept lightweight + serde-
/// friendly so the engine layer can surface them in telemetry.
#[derive(Debug, thiserror::Error)]
pub enum ComputePipelineError {
    #[error("GPU adapter unavailable")]
    NoAdapter,
    #[error("GPU device creation failed: {0}")]
    DeviceCreationFailed(String),
    #[error("GPU compute pipeline failed to build: {0}")]
    PipelineFailed(String),
    #[error("GPU feature disabled at compile time")]
    FeatureDisabled,
    #[error("GPU dispatch failed: {0}")]
    DispatchFailed(String),
}

/// One reaction-table entry packed for GPU upload. Layout matches the
/// WGSL `ReactionEntry` struct in `shaders/reaction_table.wgsl`.
#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ReactionEntryGpu {
    pub input_a: u32,
    pub input_b: u32,
    pub output: u32,
    pub byproduct: u32, // 0xffff_ffff = none
    pub min_temp_k: u32,
    /// Padding word so the layout matches the WGSL `ReactionEntry`
    /// struct alignment (6 × u32).
    pub padding: u32,
}

#[cfg(not(feature = "gpu"))]
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ReactionEntryGpu {
    pub input_a: u32,
    pub input_b: u32,
    pub output: u32,
    pub byproduct: u32,
    pub min_temp_k: u32,
    /// Padding word so the layout matches the WGSL `ReactionEntry`
    /// struct alignment (6 × u32).
    pub padding: u32,
}

impl ReactionEntryGpu {
    /// Sentinel value for "no byproduct" in the GPU table.
    pub const NO_BYPRODUCT: u32 = 0xffff_ffff;

    #[must_use]
    pub fn new(input_a: u8, input_b: u8, output: u8, byproduct: Option<u8>, min_temp_k: Option<f32>) -> Self {
        Self {
            input_a: input_a as u32,
            input_b: input_b as u32,
            output: output as u32,
            byproduct: byproduct.map_or(Self::NO_BYPRODUCT, |b| b as u32),
            min_temp_k: min_temp_k.map_or(0, |t| t as u32),
            padding: 0,
        }
    }

    /// Build from the M15D `GpuReactionRow` (per M15D § cf-material-gpu
    /// reaction_table MODIFY). Truncates 16-bit material ids to 8 bits
    /// for the launch GPU schema; material ids above 255 are not yet
    /// supported on the GPU path (use the CPU path).
    #[must_use]
    pub fn from_m15d_row(row: cf_material::GpuReactionRow) -> Self {
        Self::new(
            (row.input_a & 0xff) as u8,
            (row.input_b & 0xff) as u8,
            (row.output & 0xff) as u8,
            row.byproduct.map(|b| (b & 0xff) as u8),
            row.min_temperature_k,
        )
    }

    /// Compile a complete M15D registry into the GPU table. Skips rows
    /// whose inputs can't be resolved against the supplied lookup; per
    /// spec § "cf-material-gpu::reaction_table — Compile 55 entries
    /// into wgsl reaction-table struct".
    #[must_use]
    pub fn compile_m15d_table(
        registry: &cf_material::M15DReactionRegistry,
        name_to_id: &dyn Fn(&str) -> Option<u16>,
    ) -> Vec<Self> {
        cf_material::compile_gpu_reaction_table(registry, name_to_id)
            .into_iter()
            .map(Self::from_m15d_row)
            .collect()
    }
}

/// the chunked terrain + heat field + reaction registry, hands it to
/// [`dispatch_compute_step`], and reads back the resulting pixel grid
/// + reactions-fired counter.
#[derive(Debug, Clone)]
pub struct GpuStepInputs {
    /// Pixel grid (row-major, length = width × height).
    pub pixels: Vec<u32>,
    /// Per-cell ambient temperature in Kelvin (rounded to u32). Length
    /// = heat_grid_size² so the shader can index by `(world_x *
    /// heat_grid_size / width)`.
    pub heat_k: Vec<u32>,
    /// Reaction table.
    pub reactions: Vec<ReactionEntryGpu>,
    /// Per-material-id movement class. Length 256. Mirrors the CPU's
    /// `cf_terrain::ca::ca_movement_class`.
    /// Values: 0=Air, 1=Powder, 2=Liquid, 3=Gas, 4=Static.
    pub movement_class_table: Vec<u32>,
    pub width_px: u32,
    pub height_px: u32,
    pub heat_grid_size: u32,
    /// Margolus parity (0 or 1). Alternates per tick on the CPU.
    pub parity: u32,
}

#[derive(Debug, Clone)]
pub struct GpuStepOutputs {
    /// Post-step pixel grid. Length = `width × height`.
    pub pixels: Vec<u32>,
    /// Per-tick reactions-fired counter (atomic merge from the shader).
    pub reactions_fired: u32,
    /// Per-tick pixels-moved counter (atomic merge from the CA shader).
    pub pixels_moved: u32,
}

/// Bundle of wgpu primitives kept alive for the lifetime of the kernel.
/// When the `gpu` feature is disabled, this is a zero-sized opaque
/// struct so the rest of the codebase never sees a wgpu type leak.
#[cfg(feature = "gpu")]
pub struct ComputePipelineState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub material_ca_pipeline: wgpu::ComputePipeline,
    pub reaction_table_pipeline: wgpu::ComputePipeline,
}

/// CPU-build stub. Per spec § acceptance scenario 6, when the kernel is
/// built without `gpu`, [`try_init`] returns this stub and the kernel
/// auto-selects the CPU fallback.
#[cfg(not(feature = "gpu"))]
pub struct ComputePipelineState {
    _no_gpu: (),
}

#[cfg(feature = "gpu")]
impl std::fmt::Debug for ComputePipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputePipelineState")
            .field("device", &"<wgpu::Device>")
            .field("queue", &"<wgpu::Queue>")
            .field("material_ca_pipeline", &"<wgpu::ComputePipeline>")
            .field("reaction_table_pipeline", &"<wgpu::ComputePipeline>")
            .finish()
    }
}

#[cfg(not(feature = "gpu"))]
impl std::fmt::Debug for ComputePipelineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputePipelineState")
            .field("status", &"feature_disabled")
            .finish()
    }
}

/// WGSL source for the per-chunk Margolus CA pass. Loaded at compile
/// time. Per spec § "Notes for the implementer":
/// > Use a per-chunk Margolus checker-pattern shader pass + atomic
/// > merge step.
#[cfg(feature = "gpu")]
const MATERIAL_CA_WGSL: &str = include_str!("../shaders/material_ca.wgsl");

/// WGSL source for the per-pixel reaction-table pass.
#[cfg(feature = "gpu")]
const REACTION_TABLE_WGSL: &str = include_str!("../shaders/reaction_table.wgsl");

/// Try to initialize the GPU pipeline. Returns the chosen backend, the
/// (optional) pipeline state, and the reason the GPU was unavailable
/// when the fallback kicked in.
///
/// ## When `feature = "gpu"` is disabled
///
/// Always returns `(CpuFallback, None, Some(FeatureDisabled))`.
///
/// ## When `feature = "gpu"` is enabled
///
/// Walks the standard wgpu instance → adapter → device → pipeline
/// sequence. Each step's failure short-circuits to a CPU fallback
/// with a structured reason. The wgpu calls use `pollster::block_on`
/// because the kernel API itself is synchronous (per spec § "Per-tick
/// GPU compute completes in < 1.5 ms"; we cannot await an async runtime
/// inside the deterministic sim tick).
#[must_use]
pub fn try_init() -> (KernelBackend, Option<ComputePipelineState>, Option<GpuUnavailableReason>) {
    #[cfg(not(feature = "gpu"))]
    {
        (
            KernelBackend::CpuFallback,
            None,
            Some(GpuUnavailableReason::FeatureDisabled),
        )
    }

    #[cfg(feature = "gpu")]
    {
        match try_init_gpu_inner() {
            Ok(state) => (KernelBackend::Gpu, Some(state), None),
            Err(reason) => (KernelBackend::CpuFallback, None, Some(reason)),
        }
    }
}

#[cfg(feature = "gpu")]
fn try_init_gpu_inner() -> Result<ComputePipelineState, GpuUnavailableReason> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .map_err(|_| GpuUnavailableReason::NoAdapter)?;

    // Per-spec the reaction-table pipeline binds 5 storage buffers
    // (in/out pixels + reaction table + heat field + reactions counter)
    // which exceeds wgpu's downlevel default of 4. Bump the limit just
    // for that one parameter while leaving the rest at downlevel
    // defaults so the kernel still works on tier-2 mobile GPUs.
    let mut limits = wgpu::Limits::downlevel_defaults();
    if limits.max_storage_buffers_per_shader_stage < 5 {
        limits.max_storage_buffers_per_shader_stage = 5;
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("cf-material-gpu-device"),
        required_features: wgpu::Features::empty(),
        required_limits: limits,
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|_| GpuUnavailableReason::DeviceCreationFailed)?;

    let ca_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("material_ca_wgsl"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(MATERIAL_CA_WGSL)),
    });
    let rxn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("reaction_table_wgsl"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(REACTION_TABLE_WGSL)),
    });
    let material_ca_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("material_ca_pipeline"),
        layout: None,
        module: &ca_shader,
        entry_point: Some("ca_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let reaction_table_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("reaction_table_pipeline"),
        layout: None,
        module: &rxn_shader,
        entry_point: Some("reactions_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    Ok(ComputePipelineState {
        device,
        queue,
        material_ca_pipeline,
        reaction_table_pipeline,
    })
}

/// pipeline for chunked CA + reaction table + checksum readback":
/// uploads the pixel + heat + reaction-table buffers, runs the CA +
/// reaction-table compute pipelines back-to-back, reads back the post-
/// step pixel grid + atomic counters.
///
/// On builds without `feature = "gpu"`, returns a `DispatchFailed`
/// error so callers know to use the CPU fallback. This function exists
/// in BOTH builds so the API surface is identical — making it possible
/// to write tests that exercise the dispatch surface even without GPU.
pub fn dispatch_compute_step(
    state: &ComputePipelineState,
    inputs: &GpuStepInputs,
) -> Result<GpuStepOutputs, ComputePipelineError> {
    #[cfg(not(feature = "gpu"))]
    {
        let _ = state;
        let _ = inputs;
        Err(ComputePipelineError::FeatureDisabled)
    }

    #[cfg(feature = "gpu")]
    {
        dispatch_compute_step_inner(state, inputs)
    }
}

#[cfg(feature = "gpu")]
fn dispatch_compute_step_inner(
    state: &ComputePipelineState,
    inputs: &GpuStepInputs,
) -> Result<GpuStepOutputs, ComputePipelineError> {
    use std::borrow::Cow;
    use wgpu::util::DeviceExt;

    let device = &state.device;
    let queue = &state.queue;
    let pixel_count = inputs.pixels.len() as u64;
    if pixel_count == 0 {
        return Ok(GpuStepOutputs {
            pixels: vec![],
            reactions_fired: 0,
            pixels_moved: 0,
        });
    }
    if inputs.movement_class_table.len() != 256 {
        return Err(ComputePipelineError::DispatchFailed(
            "movement_class_table must be 256 entries".to_string(),
        ));
    }

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct CaCfg {
        chunk_size: u32,
        chunks_x: u32,
        chunks_y: u32,
        parity: u32,
        width_px: u32,
        height_px: u32,
    }
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct RxnCfg {
        chunk_size: u32,
        chunks_x: u32,
        chunks_y: u32,
        rxn_count: u32,
        width_px: u32,
        height_px: u32,
        heat_grid_size: u32,
        _padding: u32,
    }

    let chunks_x = inputs.width_px.div_ceil(64);
    let chunks_y = inputs.height_px.div_ceil(64);

    let ca_cfg = CaCfg {
        chunk_size: 64,
        chunks_x,
        chunks_y,
        parity: inputs.parity,
        width_px: inputs.width_px,
        height_px: inputs.height_px,
    };
    let rxn_cfg = RxnCfg {
        chunk_size: 64,
        chunks_x,
        chunks_y,
        rxn_count: inputs.reactions.len() as u32,
        width_px: inputs.width_px,
        height_px: inputs.height_px,
        heat_grid_size: inputs.heat_grid_size.max(1),
        _padding: 0,
    };

    let pixel_size_bytes = pixel_count * std::mem::size_of::<u32>() as u64;

    let in_pixels = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("in_pixels"),
        contents: bytemuck::cast_slice(&inputs.pixels),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let out_pixels = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("out_pixels"),
        size: pixel_size_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let ca_cfg_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ca_cfg"),
        contents: bytemuck::bytes_of(&ca_cfg),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let rxn_cfg_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rxn_cfg"),
        contents: bytemuck::bytes_of(&rxn_cfg),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let movement_class_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("movement_class_table"),
        contents: bytemuck::cast_slice(&inputs.movement_class_table),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let pixels_moved_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pixels_moved_counter"),
        contents: bytemuck::cast_slice(&[0u32]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let reactions_fired_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("reactions_fired_counter"),
        contents: bytemuck::cast_slice(&[0u32]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let placeholder_heat: [u32; 1] = [293];
    let heat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("heat_field"),
        contents: if inputs.heat_k.is_empty() {
            bytemuck::cast_slice(&placeholder_heat)
        } else {
            bytemuck::cast_slice(&inputs.heat_k)
        },
        usage: wgpu::BufferUsages::STORAGE,
    });
    let placeholder_rxn = ReactionEntryGpu::new(0, 0, 0, None, None);
    let rxn_table_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("reaction_table"),
        contents: if inputs.reactions.is_empty() {
            bytemuck::bytes_of(&placeholder_rxn)
        } else {
            bytemuck::cast_slice(&inputs.reactions)
        },
        usage: wgpu::BufferUsages::STORAGE,
    });

    // CA pass bind group + dispatch.
    let ca_bg_layout = state.material_ca_pipeline.get_bind_group_layout(0);
    let ca_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ca_bind_group"),
        layout: &ca_bg_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: in_pixels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: out_pixels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: ca_cfg_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: movement_class_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: pixels_moved_buf.as_entire_binding(),
            },
        ],
    });
    // Reaction pass bind group: reads in_pixels, writes out_pixels.
    let rxn_bg_layout = state.reaction_table_pipeline.get_bind_group_layout(0);
    let rxn_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rxn_bind_group"),
        layout: &rxn_bg_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: in_pixels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: out_pixels.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: rxn_cfg_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: rxn_table_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: heat_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: reactions_fired_buf.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("material_gpu_encoder"),
    });

    // Stage 1: reaction table pass writes into out_pixels.
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("reactions_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&state.reaction_table_pipeline);
        pass.set_bind_group(0, &rxn_bg, &[]);
        let wg_x = inputs.width_px.div_ceil(8);
        let wg_y = inputs.height_px.div_ceil(8);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }
    // Stage 2: CA pass reads out_pixels (post-reaction) via copy, then
    // writes back to out_pixels with the Margolus rule. Since both
    // pipelines bind in_pixels + out_pixels with the same names, we
    // need to copy out_pixels → in_pixels before stage 2.
    encoder.copy_buffer_to_buffer(&out_pixels, 0, &in_pixels, 0, pixel_size_bytes);
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ca_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&state.material_ca_pipeline);
        pass.set_bind_group(0, &ca_bg, &[]);
        let wg_x = (inputs.width_px / 2).div_ceil(8).max(1);
        let wg_y = (inputs.height_px / 2).div_ceil(8).max(1);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    // Readback: out_pixels + the two counter buffers.
    let pixel_readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pixel_readback"),
        size: pixel_size_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let counters_readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("counters_readback"),
        size: std::mem::size_of::<u32>() as u64 * 2,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&out_pixels, 0, &pixel_readback, 0, pixel_size_bytes);
    encoder.copy_buffer_to_buffer(&pixels_moved_buf, 0, &counters_readback, 0, std::mem::size_of::<u32>() as u64);
    encoder.copy_buffer_to_buffer(
        &reactions_fired_buf,
        0,
        &counters_readback,
        std::mem::size_of::<u32>() as u64,
        std::mem::size_of::<u32>() as u64,
    );

    queue.submit(std::iter::once(encoder.finish()));

    let pixel_slice = pixel_readback.slice(..);
    let counter_slice = counters_readback.slice(..);
    pixel_slice.map_async(wgpu::MapMode::Read, |_| {});
    counter_slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait)
        .map_err(|e| ComputePipelineError::DispatchFailed(format!("poll: {e:?}")))?;

    let pixels_out: Vec<u32> = bytemuck::cast_slice::<u8, u32>(&pixel_slice.get_mapped_range())
        .iter()
        .copied()
        .collect();
    let counters: Vec<u32> = bytemuck::cast_slice::<u8, u32>(&counter_slice.get_mapped_range())
        .iter()
        .copied()
        .collect();
    let pixels_moved = *counters.first().unwrap_or(&0);
    let reactions_fired = *counters.get(1).unwrap_or(&0);

    // Avoid unused-import warning when `gpu` feature is off (it isn't,
    // but the imports below are unused in some code paths).
    let _ = Cow::Borrowed("");

    Ok(GpuStepOutputs {
        pixels: pixels_out,
        reactions_fired,
        pixels_moved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the `gpu` feature (the default for sim crates).
    #[test]
    #[cfg(not(feature = "gpu"))]
    fn try_init_returns_cpu_fallback_without_gpu_feature() {
        let (backend, state, reason) = try_init();
        assert_eq!(backend, KernelBackend::CpuFallback);
        assert!(state.is_none());
        assert_eq!(reason, Some(GpuUnavailableReason::FeatureDisabled));
    }

    /// by telemetry).
    #[test]
    fn error_display_is_stable() {
        let e = ComputePipelineError::NoAdapter;
        assert_eq!(e.to_string(), "GPU adapter unavailable");
        let e = ComputePipelineError::FeatureDisabled;
        assert_eq!(e.to_string(), "GPU feature disabled at compile time");
        let e = ComputePipelineError::DeviceCreationFailed("oom".to_string());
        assert_eq!(e.to_string(), "GPU device creation failed: oom");
        let e = ComputePipelineError::DispatchFailed("oom".to_string());
        assert_eq!(e.to_string(), "GPU dispatch failed: oom");
    }

    /// `as_str`.
    #[test]
    fn unavailable_reason_str_names_are_stable() {
        assert_eq!(GpuUnavailableReason::FeatureDisabled.as_str(), "feature_disabled");
        assert_eq!(GpuUnavailableReason::NoAdapter.as_str(), "no_adapter");
        assert_eq!(
            GpuUnavailableReason::DeviceCreationFailed.as_str(),
            "device_creation_failed"
        );
        assert_eq!(GpuUnavailableReason::PipelineFailed.as_str(), "pipeline_failed");
    }

    /// (5 u32 fields + 1 padding word).
    #[test]
    fn reaction_entry_gpu_layout_is_six_u32() {
        assert_eq!(std::mem::size_of::<ReactionEntryGpu>(), 6 * 4);
        let e = ReactionEntryGpu::new(68, 21, 38, Some(55), Some(273.0));
        assert_eq!(e.input_a, 68);
        assert_eq!(e.input_b, 21);
        assert_eq!(e.output, 38);
        assert_eq!(e.byproduct, 55);
        assert_eq!(e.min_temp_k, 273);
    }

    /// sentinel.
    #[test]
    fn reaction_entry_none_byproduct_uses_sentinel() {
        let e = ReactionEntryGpu::new(8, 65, 41, None, None);
        assert_eq!(e.byproduct, ReactionEntryGpu::NO_BYPRODUCT);
        assert_eq!(e.min_temp_k, 0);
    }

    /// when the `gpu` feature is off (sim-crate default).
    #[test]
    #[cfg(not(feature = "gpu"))]
    fn dispatch_returns_feature_disabled_without_gpu_feature() {
        let state = ComputePipelineState { _no_gpu: () };
        let inputs = GpuStepInputs {
            pixels: vec![0u32; 16],
            heat_k: vec![293u32; 16],
            reactions: vec![ReactionEntryGpu::new(0, 0, 0, None, None)],
            movement_class_table: vec![0u32; 256],
            width_px: 4,
            height_px: 4,
            heat_grid_size: 4,
            parity: 0,
        };
        let r = dispatch_compute_step(&state, &inputs);
        assert!(matches!(r, Err(ComputePipelineError::FeatureDisabled)));
    }
}
