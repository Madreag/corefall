//! M9 — Reactor sprite (4 visual states + state-from-pressure mapping).
//!
//! Spec § Reactor visual feedback — the reactor renders one of four
//! visual states per pressure_state: nominal (clean) / cracked / critical
//! (smoking) / destroyed (debris cluster). Sprite swap happens on
//! `mission.reactor_pressure_state_changed`; the render code consumes
//! this enum and picks the right texture variant.
//!
//! Sprite assets live under `content/assets/placeholders/actor/reactor_*`
//! at M9A bake time; M9 ships the resolver so the renderer asks for the
//! right canonical key.

/// Reactor sprite variant. Maps from the M9 pressure-state ladder.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ReactorSprite {
    Nominal,
    Cracked,
    Critical,
    Destroyed,
}

impl ReactorSprite {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReactorSprite::Nominal => "nominal",
            ReactorSprite::Cracked => "cracked",
            ReactorSprite::Critical => "critical",
            ReactorSprite::Destroyed => "destroyed",
        }
    }

    /// Canonical asset key used by cf-render-2d's loader. M9A bakes the
    /// asset under this name; the renderer resolves via the asset ledger.
    pub fn asset_key(&self) -> &'static str {
        match self {
            ReactorSprite::Nominal => "actor/reactor_nominal",
            ReactorSprite::Cracked => "actor/reactor_cracked",
            ReactorSprite::Critical => "actor/reactor_critical",
            ReactorSprite::Destroyed => "actor/reactor_destroyed",
        }
    }

    /// Resolve a sprite variant from the canonical pressure_state string
    /// (`Nominal | Stressed | Critical | Venting | Destroyed`). Stressed
    /// shares the nominal sprite; Venting reuses critical because both
    /// pre-destruction states show the "stressed reactor" silhouette per
    /// spec § Reactor sprite swaps on pressure state.
    #[must_use]
    pub fn from_pressure_state(state: &str) -> Self {
        match state {
            "Nominal" => ReactorSprite::Nominal,
            "Stressed" => ReactorSprite::Cracked,
            "Critical" => ReactorSprite::Critical,
            "Venting" => ReactorSprite::Critical,
            "Destroyed" => ReactorSprite::Destroyed,
            _ => ReactorSprite::Nominal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_pressure_states_to_variants() {
        assert_eq!(ReactorSprite::from_pressure_state("Nominal"), ReactorSprite::Nominal);
        assert_eq!(ReactorSprite::from_pressure_state("Stressed"), ReactorSprite::Cracked);
        assert_eq!(ReactorSprite::from_pressure_state("Critical"), ReactorSprite::Critical);
        assert_eq!(ReactorSprite::from_pressure_state("Venting"), ReactorSprite::Critical);
        assert_eq!(
            ReactorSprite::from_pressure_state("Destroyed"),
            ReactorSprite::Destroyed
        );
        assert_eq!(ReactorSprite::from_pressure_state("unknown"), ReactorSprite::Nominal);
    }

    #[test]
    fn asset_keys_unique() {
        let variants = [
            ReactorSprite::Nominal,
            ReactorSprite::Cracked,
            ReactorSprite::Critical,
            ReactorSprite::Destroyed,
        ];
        let mut keys: Vec<&str> = variants.iter().map(|v| v.asset_key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 4);
    }
}
