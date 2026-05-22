//! M3 — Hazard tile contact damage routing.
//!
//! Per the M3 spec's "## Files" section, `cf-physics/src/hazard.rs` hosts
//! the helper that decides whether an actor standing on a hazard tile takes
//! damage this tick. The wiring (actor → engine.apply_damage) is in
//! `cf-control/src/engine.rs`; this module exposes the pure decision
//! function so consumers (and tests) can call it without an engine.

/// tile. `damage_per_tick` comes from `MaterialDef.damage_per_tick`. If the
/// material is not hazardous or `damage_on_touch=false`, returns 0.
#[must_use]
pub fn hazard_damage_per_tick(damage_per_tick: f32, damage_on_touch: bool) -> f32 {
    if damage_on_touch && damage_per_tick > 0.0 {
        damage_per_tick
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_damages_on_contact() {
        assert!((hazard_damage_per_tick(2.0, true) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn non_hazard_no_damage() {
        assert!(hazard_damage_per_tick(2.0, false).abs() < f32::EPSILON);
    }
}
