//! F3 — Collision shapes overlay. AABB outlines for actors + items +
//! projectiles per spec § Debug overlays.

use serde::{Deserialize, Serialize};

/// What kind of collidable an AABB belongs to (drives outline color).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollidableKind {
    /// An actor body.
    Actor,
    /// A dropped or world item.
    Item,
    /// An in-flight projectile.
    Projectile,
}

/// One AABB to render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollisionAabb {
    /// Source kind (drives the renderer's per-kind color).
    pub kind: CollidableKind,
    /// Source entity id (actor / item / projectile).
    pub entity_id: u64,
    /// Bottom-left corner in world units.
    pub min: (f32, f32),
    /// Top-right corner in world units.
    pub max: (f32, f32),
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CollisionOverlayData {
    /// All AABBs to draw this frame.
    pub aabbs: Vec<CollisionAabb>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_round_trips() {
        let kinds = [CollidableKind::Actor, CollidableKind::Item, CollidableKind::Projectile];
        for k in kinds {
            let s = serde_json::to_string(&k).unwrap();
            let back: CollidableKind = serde_json::from_str(&s).unwrap();
            assert_eq!(k, back);
        }
    }
}
