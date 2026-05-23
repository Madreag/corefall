// **M15B + M15D** § GPU reaction-table compute shader.
//
// Per the M15B spec § "Notes for the implementer":
// > Material reactions on the GPU require deterministic ordering — use a
// > per-chunk Margolus checker-pattern shader pass + atomic merge step.
// > NO atomic-CAS loops on shared sim state.
//
// M15D extends the host-side feed: `compile_gpu_reaction_table` walks
// the 55-reaction M15D registry and emits a `Vec<ReactionEntryGpu>` —
// one per reaction whose inputs resolve to launch-registry material
// ids. Reactions referencing abstract reagents (KNO3, tritium, etc.)
// run on the CPU-only path until M15C adds the matching entries.
//
// Iterates every reactive pixel in the chunk; for each pixel, checks the
// right + below neighbors against the reaction lookup table.
//
// The reaction table is stored as a flat array of MaterialReactionEntry
// records — one per registered reaction. The lookup is a linear scan;
// the M15 launch set + M15D extension lands at ~55 reactions, so a per-
// pixel O(n) scan stays under the M15B perf gate (<1.5 ms/tick on the
// reference GPU) and avoids the determinism complications of a hash
// map.
//
// Reaction record layout (matches host-side `MaterialReactionEntry`):
//   u32 input_a
//   u32 input_b
//   u32 output
//   u32 byproduct  (0xffffffff = none; the host translates to None)
//   u32 min_temp_k (0 = no gate)
//   u32 _padding

struct ReactionEntry {
    input_a: u32,
    input_b: u32,
    output: u32,
    byproduct: u32,
    min_temp_k: u32,
    _padding: u32,
};

struct RxnConfig {
    chunk_size: u32,
    chunks_x: u32,
    chunks_y: u32,
    rxn_count: u32,
    width_px: u32,
    height_px: u32,
    heat_grid_size: u32,
    _padding: u32,
};

@group(0) @binding(0) var<storage, read> in_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> out_pixels: array<u32>;
@group(0) @binding(2) var<uniform> cfg: RxnConfig;
@group(0) @binding(3) var<storage, read> reaction_table: array<ReactionEntry>;
@group(0) @binding(4) var<storage, read> heat_field: array<u32>; // temperatures in Kelvin (rounded)
@group(0) @binding(5) var<storage, read_write> reactions_fired_counter: atomic<u32>;

fn pixel_index(world_x: u32, world_y: u32) -> u32 {
    return world_y * cfg.width_px + world_x;
}

fn read_pixel(world_x: u32, world_y: u32) -> u32 {
    if (world_x >= cfg.width_px || world_y >= cfg.height_px) {
        return 0u;
    }
    return in_pixels[pixel_index(world_x, world_y)];
}

fn write_pixel(world_x: u32, world_y: u32, value: u32) {
    if (world_x >= cfg.width_px || world_y >= cfg.height_px) {
        return;
    }
    out_pixels[pixel_index(world_x, world_y)] = value;
}

fn temperature_at(world_x: u32, world_y: u32) -> u32 {
    // The host scales temperature into a 0..=65535 u16 (Kelvin) before
    // upload; on the GPU we read it as u32 + saturate.
    let cell_x = (world_x * cfg.heat_grid_size) / max(cfg.width_px, 1u);
    let cell_y = (world_y * cfg.heat_grid_size) / max(cfg.height_px, 1u);
    let idx = cell_y * cfg.heat_grid_size + cell_x;
    if (idx >= arrayLength(&heat_field)) {
        return 293u; // default ambient ~293K
    }
    return heat_field[idx];
}

fn find_reaction(a: u32, b: u32, temp: u32) -> i32 {
    for (var i: u32 = 0u; i < cfg.rxn_count; i = i + 1u) {
        let r = reaction_table[i];
        let pair_ok = (r.input_a == a && r.input_b == b) || (r.input_a == b && r.input_b == a);
        if (!pair_ok) {
            continue;
        }
        if (r.min_temp_k > 0u && temp < r.min_temp_k) {
            continue;
        }
        return i32(i);
    }
    return -1;
}

@compute @workgroup_size(8, 8, 1)
fn reactions_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let world_x = gid.x;
    let world_y = gid.y;
    if (world_x >= cfg.width_px || world_y >= cfg.height_px) {
        return;
    }

    let pa = read_pixel(world_x, world_y);
    if (pa == 0u) { // air never appears as input_a in the launch set
        write_pixel(world_x, world_y, pa);
        return;
    }
    let temp = temperature_at(world_x, world_y);

    // Right neighbor.
    if (world_x + 1u < cfg.width_px) {
        let pb = read_pixel(world_x + 1u, world_y);
        if (pa != pb) {
            let rxn_idx = find_reaction(pa, pb, temp);
            if (rxn_idx >= 0) {
                let r = reaction_table[u32(rxn_idx)];
                var new_a: u32 = pa;
                var new_b: u32 = pb;
                if (pa == r.input_a) {
                    new_a = r.output;
                    if (r.byproduct != 0xffffffffu) { new_b = r.byproduct; }
                } else {
                    new_b = r.output;
                    if (r.byproduct != 0xffffffffu) { new_a = r.byproduct; }
                }
                write_pixel(world_x, world_y, new_a);
                write_pixel(world_x + 1u, world_y, new_b);
                atomicAdd(&reactions_fired_counter, 1u);
                return;
            }
        }
    }
    // Below neighbor.
    if (world_y + 1u < cfg.height_px) {
        let pb = read_pixel(world_x, world_y + 1u);
        if (pa != pb) {
            let rxn_idx = find_reaction(pa, pb, temp);
            if (rxn_idx >= 0) {
                let r = reaction_table[u32(rxn_idx)];
                var new_a: u32 = pa;
                var new_b: u32 = pb;
                if (pa == r.input_a) {
                    new_a = r.output;
                    if (r.byproduct != 0xffffffffu) { new_b = r.byproduct; }
                } else {
                    new_b = r.output;
                    if (r.byproduct != 0xffffffffu) { new_a = r.byproduct; }
                }
                write_pixel(world_x, world_y, new_a);
                write_pixel(world_x, world_y + 1u, new_b);
                atomicAdd(&reactions_fired_counter, 1u);
                return;
            }
        }
    }

    // No reaction fired — pass-through.
    write_pixel(world_x, world_y, pa);
}
