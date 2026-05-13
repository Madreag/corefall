//! M3 — Bullet/projectile vs terrain penetration formula.
//!
//! Per the M3 spec's "## Files" section, `cf-physics/src/penetration.rs`
//! hosts the `try_penetrate` formula (`impulse² > integrity²` per CCCP
//! `SceneMan.cpp:571`). The current implementation lives inline in
//! `cf-control/src/engine.rs`; this module exposes a pure helper that
//! consumers (M14+ projectile/terrain integration) can call directly.

/// **M3**: penetration verdict per CCCP `SceneMan.cpp:571`. Returns true if
/// the squared impulse exceeds the squared integrity — i.e. the projectile
/// passes through the pixel and clears it to air. Squared comparison avoids
/// `sqrt` in the hot path.
///
/// `mass` * `velocity` * `sharpness` = impulse; `integrity` is the
/// material's hardness/integrity scalar from `MaterialDef`.
#[must_use]
pub fn try_penetrate_impulse_vs_integrity(
    mass: f32,
    velocity: f32,
    sharpness: f32,
    integrity: f32,
) -> Penetration {
    let impulse = mass * velocity * sharpness;
    let impulse_sq = impulse * impulse;
    let integrity_sq = integrity * integrity;
    Penetration {
        passes: impulse_sq > integrity_sq,
        retardation: if impulse_sq > integrity_sq {
            (integrity_sq / impulse_sq).clamp(0.0, 1.0)
        } else {
            1.0
        },
    }
}

/// **M3**: result of [`try_penetrate_impulse_vs_integrity`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Penetration {
    /// True if the projectile passes through and clears the pixel.
    pub passes: bool,
    /// Velocity multiplier applied to the projectile on the OUT side of the
    /// hit. 1.0 = absorbed; <1.0 = passes through with reduced velocity.
    pub retardation: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirt_penetrates_rifle() {
        // mass=0.05, v=400, sharpness=0.8 → impulse=16, impulse_sq=256
        // dirt integrity=10 → integrity_sq=100 → 256>100 → passes.
        let p = try_penetrate_impulse_vs_integrity(0.05, 400.0, 0.8, 10.0);
        assert!(p.passes);
    }

    #[test]
    fn concrete_stops_rifle() {
        // Same projectile, concrete integrity=40 → integrity_sq=1600 →
        // 256<1600 → does not penetrate.
        let p = try_penetrate_impulse_vs_integrity(0.05, 400.0, 0.8, 40.0);
        assert!(!p.passes);
    }
}
