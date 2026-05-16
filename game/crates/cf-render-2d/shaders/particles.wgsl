// M8A § GPU compute offload — cosmetic particle integration shader.
//
// Per M8A spec § Architecture rules rule "GPU is presentation, never
// sim": GPU particle state is `cosmetic: true` and excluded from the
// determinism island. Same wgsl shader produces different driver-level
// output across NVIDIA / AMD / Apple Silicon; that's acceptable here
// because the underlying terrain mutation is hashed CPU-side via
// `terrain.chunk_mutated`, and the particles are presentation only.

struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    age_ms: f32,
    seed: u32,
};

struct Globals {
    tick: u32,
    dt_seconds: f32,
    gravity: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;

@compute @workgroup_size(64)
fn integrate_particles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= arrayLength(&particles) {
        return;
    }
    var p = particles[idx];
    p.vel.y -= globals.gravity * globals.dt_seconds;
    p.pos += p.vel * globals.dt_seconds;
    p.age_ms += globals.dt_seconds * 1000.0;
    particles[idx] = p;
}
