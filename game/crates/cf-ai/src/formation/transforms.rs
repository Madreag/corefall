//! M7B: commander-facing local → world transforms.
//!
//! Formation slot offsets are authored in commander-facing local space
//! (x forward, y left). The world-space anchor is the commander's
//! current position rotated by the commander's facing.

/// `facing_radians` is the commander's facing direction in standard math
/// convention (0 = +x, π/2 = +y, etc.).
pub fn rotate_local_to_world(local: [f32; 2], facing_radians: f32) -> [f32; 2] {
    let (sin, cos) = facing_radians.sin_cos();
    [
        local[0] * cos - local[1] * sin,
        local[0] * sin + local[1] * cos,
    ]
}

/// position + facing.
pub fn world_anchor_for_slot(
    commander_pos: [f32; 2],
    facing_radians: f32,
    slot_local_offset: [f32; 2],
) -> [f32; 2] {
    let r = rotate_local_to_world(slot_local_offset, facing_radians);
    [commander_pos[0] + r[0], commander_pos[1] + r[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn identity_rotation() {
        let p = rotate_local_to_world([3.0, 4.0], 0.0);
        assert!(approx(p[0], 3.0));
        assert!(approx(p[1], 4.0));
    }

    #[test]
    fn quarter_turn() {
        let p = rotate_local_to_world([1.0, 0.0], std::f32::consts::FRAC_PI_2);
        assert!(approx(p[0], 0.0));
        assert!(approx(p[1], 1.0));
    }

    #[test]
    fn anchor_adds_commander_pos() {
        let a = world_anchor_for_slot([10.0, 5.0], 0.0, [-6.0, 4.0]);
        assert!(approx(a[0], 4.0));
        assert!(approx(a[1], 9.0));
    }
}
