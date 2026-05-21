// **M15B** § GPU material cellular-automaton compute shader.
//
// Per the M15B spec § "Notes for the implementer":
// > Material reactions on the GPU require deterministic ordering — use a
// > per-chunk Margolus checker-pattern shader pass + atomic merge step.
// > NO atomic-CAS loops on shared sim state.
//
// One workgroup processes one 64x64 chunk (matches CHUNK_SIZE in
// cf-terrain). The Margolus checker-pattern walks the 2x2 cells with
// parity from a uniform; the parity alternates per tick. Each 2x2 cell
// applies the canonical sand/water/gas rule.
//
// Movement classes (mirrors `cf_terrain::ca::ca_movement_class`):
//   0 = Air, 1 = Powder, 2 = Liquid, 3 = Gas, 4 = Static.

struct CaConfig {
    chunk_size: u32,
    chunks_x: u32,
    chunks_y: u32,
    parity: u32,
    width_px: u32,
    height_px: u32,
};

@group(0) @binding(0) var<storage, read> in_pixels: array<u32>;
@group(0) @binding(1) var<storage, read_write> out_pixels: array<u32>;
@group(0) @binding(2) var<uniform> cfg: CaConfig;
@group(0) @binding(3) var<storage, read> movement_class_table: array<u32>;
@group(0) @binding(4) var<storage, read_write> pixels_moved_counter: atomic<u32>;

fn pixel_index(world_x: u32, world_y: u32) -> u32 {
    return world_y * cfg.width_px + world_x;
}

fn read_pixel(world_x: u32, world_y: u32) -> u32 {
    if (world_x >= cfg.width_px || world_y >= cfg.height_px) {
        return 0u; // out-of-bounds is air
    }
    return in_pixels[pixel_index(world_x, world_y)];
}

fn write_pixel(world_x: u32, world_y: u32, value: u32) {
    if (world_x >= cfg.width_px || world_y >= cfg.height_px) {
        return;
    }
    out_pixels[pixel_index(world_x, world_y)] = value;
}

fn movement_class(material: u32) -> u32 {
    // Saturate to 256 entries; bytes >= 256 wrap to air for safety.
    let idx = material & 0xffu;
    return movement_class_table[idx];
}

@compute @workgroup_size(8, 8, 1)
fn ca_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // Walk 2x2 Margolus cells. The shader is invoked over (width/2,
    // height/2, 1); each invocation owns one 2x2 cell.
    let pair_x = gid.x;
    let pair_y = gid.y;
    let offset = cfg.parity & 1u;
    let world_x = pair_x * 2u + offset;
    let world_y = pair_y * 2u + offset;
    if (world_x + 1u >= cfg.width_px || world_y + 1u >= cfg.height_px) {
        // Pass-through write (carry the input value forward; the kernel
        // owns the contract that out_pixels mirrors in_pixels for cells
        // that don't move).
        write_pixel(world_x, world_y, read_pixel(world_x, world_y));
        return;
    }

    let tl = read_pixel(world_x, world_y);
    let tr = read_pixel(world_x + 1u, world_y);
    let bl = read_pixel(world_x, world_y + 1u);
    let br = read_pixel(world_x + 1u, world_y + 1u);

    // Default: identity (no movement).
    var new_tl = tl;
    var new_tr = tr;
    var new_bl = bl;
    var new_br = br;

    let cls_tl = movement_class(tl);
    let cls_tr = movement_class(tr);
    let cls_bl = movement_class(bl);
    let cls_br = movement_class(br);

    let any_moved = false;

    // Powder/liquid: fall into air below.
    // Top-left → bottom-left if BL is air and TL is powder/liquid.
    if ((cls_tl == 1u || cls_tl == 2u) && cls_bl == 0u) {
        new_tl = bl;
        new_bl = tl;
    }
    if ((cls_tr == 1u || cls_tr == 2u) && cls_br == 0u) {
        new_tr = br;
        new_br = tr;
    }
    // Gas: rise into air above.
    if (cls_bl == 3u && cls_tl == 0u) {
        new_bl = tl;
        new_tl = bl;
    }
    if (cls_br == 3u && cls_tr == 0u) {
        new_br = tr;
        new_tr = br;
    }

    write_pixel(world_x, world_y, new_tl);
    write_pixel(world_x + 1u, world_y, new_tr);
    write_pixel(world_x, world_y + 1u, new_bl);
    write_pixel(world_x + 1u, world_y + 1u, new_br);

    var moved: u32 = 0u;
    if (new_tl != tl) { moved = moved + 1u; }
    if (new_tr != tr) { moved = moved + 1u; }
    if (new_bl != bl) { moved = moved + 1u; }
    if (new_br != br) { moved = moved + 1u; }
    if (moved > 0u) {
        atomicAdd(&pixels_moved_counter, moved);
    }
}
