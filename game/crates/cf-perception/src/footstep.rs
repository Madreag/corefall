//! M6: per-surface footstep loudness (carpet vs metal vs dirt).
//!
//! `MaterialDef.loudness_modifier` is what the engine reads from terrain at
//! the actor's feet each tick; we encode the canonical surface kinds here
//! as discrete bands so the perception kernel stays terrain-agnostic.

use serde::{Deserialize, Serialize};

use cf_actor::Vec2;

/// Canonical surface kind for footstep emission. Each kind carries a default
/// loudness modifier that the engine multiplies against the actor's stance
/// loudness.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    Dirt = 0,
    LooseFill = 1,
    Concrete = 2,
    Metal = 3,
    Water = 4,
    Carpet = 5,
    Glass = 6,
    Sand = 7,
}

impl SurfaceKind {
    pub fn loudness_modifier(self) -> f32 {
        match self {
            SurfaceKind::Carpet => 0.25,
            SurfaceKind::Dirt => 0.4,
            SurfaceKind::Sand => 0.5,
            SurfaceKind::LooseFill => 0.55,
            SurfaceKind::Concrete => 0.8,
            SurfaceKind::Water => 0.9,
            SurfaceKind::Metal => 1.2,
            SurfaceKind::Glass => 1.4,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Dirt => "dirt",
            SurfaceKind::LooseFill => "loose_fill",
            SurfaceKind::Concrete => "concrete",
            SurfaceKind::Metal => "metal",
            SurfaceKind::Water => "water",
            SurfaceKind::Carpet => "carpet",
            SurfaceKind::Glass => "glass",
            SurfaceKind::Sand => "sand",
        }
    }
}

/// One footstep emission record passed into the perception kernel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FootstepEmission {
    pub actor: u64,
    pub position: Vec2,
    pub surface: SurfaceKind,
    /// Base loudness for the actor's locomotion stance (sprint > run > walk > crouch).
    pub stance_loudness: f32,
}

/// Compute the effective footstep loudness for the given emission. Multiplies
/// the stance loudness by the surface modifier; clamps to [0, 1].
#[must_use]
pub fn footstep_loudness(emission: FootstepEmission) -> f32 {
    if !emission.stance_loudness.is_finite() {
        return 0.0;
    }
    (emission.stance_loudness.max(0.0) * emission.surface.loudness_modifier()).clamp(0.0, 1.0)
}

/// **M14A** § "actor.on_stride emits hearing signals scaled by material
/// loudness". Convert an `actor.on_stride` event into a hearing-grade
/// emission record. `actor_id` + `position` come from the event payload;
/// `material_id` resolves to a SurfaceKind via [`surface_kind_for_material`].
pub fn on_stride_to_hearing_emission(
    actor_id: u64,
    position: Vec2,
    material_id: u8,
    move_state: &str,
) -> FootstepEmission {
    let surface = surface_kind_for_material(material_id);
    let stance_loudness = match move_state {
        "stand" | "no_move" => 0.1,
        "walk" => 0.5,
        "crouch" | "crawl" | "arm_crawl" => 0.3,
        "climb" => 0.35,
        "jump" => 0.7,
        "dislodge" => 0.6,
        "hover" => 0.2,
        _ => 0.4,
    };
    FootstepEmission {
        actor: actor_id,
        position,
        surface,
        stance_loudness,
    }
}

/// **M14A** § "AI hearing — on_stride → AudioCue.hearing_signal_db" —
/// map a chunked-terrain material id to the perception SurfaceKind band.
///
/// M14A's launch material set (DR-007: 0=air, 1=dirt, 2=concrete,
/// 3=metal_nohook, 4=hazard, 5=loose_fill, 6=repair_fill, 7=anchor;
/// M15 extension: 12=lava, 13=acid, 14=ice, 15=snow, 16=oil, 17=mud,
/// 18=water).
pub fn surface_kind_for_material(material_id: u8) -> SurfaceKind {
    match material_id {
        1 => SurfaceKind::Dirt,
        2 => SurfaceKind::Concrete,
        3 => SurfaceKind::Metal,
        4 => SurfaceKind::Concrete,
        5 => SurfaceKind::LooseFill,
        6 => SurfaceKind::Concrete,
        7 => SurfaceKind::Concrete,
        14 | 15 => SurfaceKind::Dirt,
        16 => SurfaceKind::Carpet,
        17 => SurfaceKind::Dirt,
        18 => SurfaceKind::Water,
        _ => SurfaceKind::Concrete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metal_louder_than_dirt() {
        let metal = FootstepEmission {
            actor: 0,
            position: Vec2::ZERO,
            surface: SurfaceKind::Metal,
            stance_loudness: 0.5,
        };
        let dirt = FootstepEmission {
            surface: SurfaceKind::Dirt,
            ..metal
        };
        assert!(footstep_loudness(metal) > footstep_loudness(dirt));
    }

    #[test]
    fn carpet_quietest() {
        for s in [
            SurfaceKind::Dirt,
            SurfaceKind::LooseFill,
            SurfaceKind::Concrete,
            SurfaceKind::Metal,
            SurfaceKind::Glass,
            SurfaceKind::Water,
            SurfaceKind::Sand,
        ] {
            assert!(SurfaceKind::Carpet.loudness_modifier() < s.loudness_modifier());
        }
    }

    #[test]
    fn nan_stance_returns_zero() {
        let e = FootstepEmission {
            actor: 0,
            position: Vec2::ZERO,
            surface: SurfaceKind::Metal,
            stance_loudness: f32::NAN,
        };
        assert_eq!(footstep_loudness(e), 0.0);
    }
}
