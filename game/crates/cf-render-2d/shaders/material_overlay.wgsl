// M8A § GPU fragment shader — 5-mode material overlay.
//
// Per M8A spec § Acceptance criteria — GPU compute offload: the material
// overlay (M3 Integrity / Pathability / Mobility / Hazard / BuildRepair)
// tint runs as a fragment shader reading the per-fragment material id +
// the `MaterialAffordance::overlay_rgba` uniform buffer.

struct OverlayParams {
    mode: u32,
    overlay_rgba: array<vec4<f32>, 5>,
};

@group(0) @binding(0) var<uniform> params: OverlayParams;
@group(0) @binding(1) var material_id_tex: texture_2d<u32>;

@fragment
fn fragment_overlay(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let pos = vec2<i32>(frag_pos.xy);
    let material_id: u32 = textureLoad(material_id_tex, pos, 0).x;
    let mode_idx: u32 = clamp(params.mode, 0u, 4u);
    let lookup_idx: u32 = (material_id + mode_idx) % 5u;
    return params.overlay_rgba[lookup_idx];
}
