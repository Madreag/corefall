//! **M15** § Material phase transition state machine.
//!
//! Per the M15 spec § "Material state transitions (water → steam → cloud
//! → rain)":
//!
//! - `water` solid at 0°C, liquid at 0-100°C, gas (steam) at >100°C
//! - `oil` liquid at -20-200°C, gas at >200°C
//! - `mercury` liquid at -38°C to 357°C
//! - `wood` solid; burns to ash at >500°C
//! - `iron` solid <1538°C, liquid at >1538°C, gas at >2862°C
//! - All transitions reversible (steam cools → water; obsidian heats → lava)
//! - Visual state changes when phase crosses threshold
//! - Phase change consumes/releases latent heat per material
//!
//! Events emitted: `material.phase_transition { material, from_state,
//! to_state, pos, temperature }`.
//!
//! M15B extends this with the precipitation chain (steam → cloud → rain).
//! M15D may extend with Arrhenius-rate phase kinetics. The state machine
//! here is the deterministic CPU baseline.

use serde::{Deserialize, Serialize};

use crate::MaterialId;

/// Phase state vocabulary. Locked at M15.
///
/// `Plasma` is reserved for M15D's energy materials (electric_arc, lightning,
/// fire_intense). `Powder` is a serializable alias for fine-grained solids
/// (sand, salt, sugar, ash); the CA stepper treats it as solid with gravity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Solid,
    Liquid,
    Gas,
    Plasma,
    Powder,
}

impl PhaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            PhaseState::Solid => "solid",
            PhaseState::Liquid => "liquid",
            PhaseState::Gas => "gas",
            PhaseState::Plasma => "plasma",
            PhaseState::Powder => "powder",
        }
    }
}

/// One transition rule. The state machine fires the transition when
/// `temperature_k` crosses `threshold_k` in the direction the rule
/// implies (forward = warming, reverse = cooling, bidirectional fires
/// either way + flips `from`/`to`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseTransition {
    pub material: MaterialId,
    pub from_state: PhaseState,
    pub to_state: PhaseState,
    /// Resulting material after the phase change (e.g. water → steam).
    /// `None` means the same material id is reused (visual-only change).
    #[serde(default)]
    pub product_material: Option<MaterialId>,
    /// Kelvin threshold at which the transition fires. Crossing this
    /// while warming triggers the forward transition; crossing while
    /// cooling triggers the reverse (if `reversible` is set).
    pub threshold_k: f32,
    /// Latent heat in J/kg consumed (positive) or released (negative)
    /// by the transition. The CA / thermal field uses this to budget
    /// the heat field per spec § "Phase change consumes/releases latent
    /// heat per material".
    pub latent_heat_j_per_kg: f32,
    /// True = the inverse transition fires on cooling. The spec says
    /// "All transitions reversible (steam cools → water; obsidian heats
    /// → lava)"; we honor that by default.
    #[serde(default = "default_reversible")]
    pub reversible: bool,
}

fn default_reversible() -> bool {
    true
}

impl PhaseTransition {
    /// True when the rule would fire for the given (material, temperature)
    /// sample. Direction-aware: forward means going from `from_state` to
    /// `to_state` (e.g. liquid → gas = warming past boil; liquid → solid
    /// = cooling past freeze). Reverse means the inverse path when the
    /// transition is `reversible`.
    ///
    /// We use the canonical "hotness" ordering Solid < Powder < Liquid <
    /// Gas < Plasma to decide whether the forward direction is the
    /// warming or the cooling cross.
    #[must_use]
    pub fn fires(&self, material: MaterialId, temperature_k: f32, prev_temperature_k: f32) -> Option<PhaseDirection> {
        if material != self.material && self.product_material != Some(material) {
            return None;
        }
        let warming_is_forward = phase_hotness(self.to_state) > phase_hotness(self.from_state);
        let crossed_above = prev_temperature_k <= self.threshold_k && temperature_k > self.threshold_k;
        let crossed_below = prev_temperature_k >= self.threshold_k && temperature_k < self.threshold_k;
        // Forward case requires the source material identity.
        if material == self.material {
            let forward = if warming_is_forward { crossed_above } else { crossed_below };
            if forward {
                return Some(PhaseDirection::Forward);
            }
        }
        // Reverse case requires the product material identity (post-transition).
        if self.reversible && self.product_material == Some(material) {
            let reverse = if warming_is_forward { crossed_below } else { crossed_above };
            if reverse {
                return Some(PhaseDirection::Reverse);
            }
        }
        None
    }

    /// The resulting `(material, state)` after the transition fires in
    /// the given direction.
    #[must_use]
    pub fn resolve(&self, direction: PhaseDirection) -> (MaterialId, PhaseState) {
        match direction {
            PhaseDirection::Forward => (self.product_material.unwrap_or(self.material), self.to_state),
            PhaseDirection::Reverse => (self.material, self.from_state),
        }
    }
}

/// Canonical "hotness" ranking used by [`PhaseTransition::fires`] to
/// disambiguate warming-cross vs cooling-cross transitions. Higher
/// hotness = more energetic state. Powder is between solid and liquid
/// because it represents a granular solid (it doesn't carry latent-heat
/// semantics on its own — see [`PhaseTransition::latent_heat_j_per_kg`]
/// for the actual energy budget).
#[must_use]
pub fn phase_hotness(state: PhaseState) -> u8 {
    match state {
        PhaseState::Solid => 0,
        PhaseState::Powder => 1,
        PhaseState::Liquid => 2,
        PhaseState::Gas => 3,
        PhaseState::Plasma => 4,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseDirection {
    Forward,
    Reverse,
}

/// The full phase-transition registry. M15 ships the launch rules per
/// the spec literal enumeration above; M15B extends with the
/// precipitation chain (steam → cloud at altitude).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseRegistry {
    pub schema_version: u32,
    pub transitions: Vec<PhaseTransition>,
}

impl PhaseRegistry {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(transitions: Vec<PhaseTransition>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            transitions,
        }
    }

    /// **M15B** § Load a `PhaseRegistry` from a JSON file. Modders +
    /// tuners edit `content/materials/phase_registry.json` (or a
    /// custom path) to add new phase transitions, tweak thresholds,
    /// or override latent-heat budgets without touching engine source.
    pub fn load_from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, PhaseRegistryLoadError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| PhaseRegistryLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let registry: PhaseRegistry =
            serde_json::from_str(&raw).map_err(|source| PhaseRegistryLoadError::Parse {
                path: path_ref.to_path_buf(),
                source,
            })?;
        if registry.schema_version != Self::SCHEMA_VERSION {
            return Err(PhaseRegistryLoadError::SchemaVersionMismatch {
                path: path_ref.to_path_buf(),
                expected: Self::SCHEMA_VERSION,
                actual: registry.schema_version,
            });
        }
        Ok(registry)
    }

    /// **M15B** § Resolve the canonical phase registry path.
    #[must_use]
    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/materials/phase_registry.json"),
            std::path::PathBuf::from("../content/materials/phase_registry.json"),
            std::path::PathBuf::from("game/content/materials/phase_registry.json"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// **M15B** § Load the canonical registry from the default JSON
    /// path, or fall back to the hardcoded `default_phase_registry`
    /// when the file isn't present.
    #[must_use]
    pub fn load_default_or_hardcoded() -> Self {
        match Self::locate_default().and_then(|p| Self::load_from_file(&p).ok()) {
            Some(r) => r,
            None => default_phase_registry(),
        }
    }

    /// Find the first transition that fires for `(material, prev_t, t)`.
    /// Direction-aware.
    #[must_use]
    pub fn evaluate(
        &self,
        material: MaterialId,
        prev_temperature_k: f32,
        temperature_k: f32,
    ) -> Option<(&PhaseTransition, PhaseDirection)> {
        self.transitions
            .iter()
            .find_map(|t| t.fires(material, temperature_k, prev_temperature_k).map(|d| (t, d)))
    }

    pub fn len(&self) -> usize {
        self.transitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }

    /// Build the set of materials that appear as either source
    /// (`material`) or product (`product_material`) in any transition.
    /// The CA kernel uses this to skip pixels that can't possibly
    /// undergo a phase transition (inert solids like concrete, dirt,
    /// metal). Per-tick scans drop from O(pixels) to O(phase_pixels).
    #[must_use]
    pub fn phase_material_set(&self) -> std::collections::BTreeSet<MaterialId> {
        let mut set = std::collections::BTreeSet::new();
        for t in &self.transitions {
            set.insert(t.material);
            if let Some(p) = t.product_material {
                set.insert(p);
            }
        }
        set
    }
}

/// **M15B** § Errors from [`PhaseRegistry::load_from_file`].
#[derive(Debug, thiserror::Error)]
pub enum PhaseRegistryLoadError {
    #[error("failed to read phase registry at {}: {source}", path.display())]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse phase registry at {}: {source}", path.display())]
    Parse {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error(
        "schema_version mismatch in {}: expected {expected}, got {actual}",
        path.display()
    )]
    SchemaVersionMismatch {
        path: std::path::PathBuf,
        expected: u32,
        actual: u32,
    },
}

/// **M15** § the canonical launch phase-transition registry. Material
/// ids match `content/materials/material_registry.json`:
/// - 13=water, 15=ice, 50=steam, 23=blood, 72=frozen_blood,
/// - 26=lava, 70=obsidian, 34=ore_iron, 68=iron,
/// - 19=oil, 24=alcohol, 59=ethanol_vapor, 8=wood, 40=ash, 41=charcoal,
/// - 25=mercury.
#[must_use]
pub fn default_phase_registry() -> PhaseRegistry {
    let raw: &[PhaseTransition] = &[
        // Water → steam at 373.15K (100°C).
        PhaseTransition {
            material: 13,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Gas,
            product_material: Some(50),
            threshold_k: 373.15,
            latent_heat_j_per_kg: 2_260_000.0,
            reversible: true,
        },
        // Water ↔ ice at 273.15K (0°C). `reversible=true` so the inverse
        // (ice → water) fires on warming past 273.15K via [`PhaseDirection::Reverse`].
        PhaseTransition {
            material: 13,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Solid,
            product_material: Some(15),
            threshold_k: 273.15,
            latent_heat_j_per_kg: -334_000.0,
            reversible: true,
        },
        // Obsidian → lava at 1373K (volcanic melt threshold).
        PhaseTransition {
            material: 70,
            from_state: PhaseState::Solid,
            to_state: PhaseState::Liquid,
            product_material: Some(26),
            threshold_k: 1373.0,
            latent_heat_j_per_kg: 400_000.0,
            reversible: true,
        },
        // Iron ore → iron at 1811K (forward smelt).
        PhaseTransition {
            material: 34,
            from_state: PhaseState::Solid,
            to_state: PhaseState::Liquid,
            product_material: Some(68),
            threshold_k: 1811.0,
            latent_heat_j_per_kg: 247_000.0,
            reversible: true,
        },
        // Iron → vapor at 3134K (gas phase, melt point for steel-grade).
        PhaseTransition {
            material: 68,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Gas,
            product_material: None,
            threshold_k: 3134.0,
            latent_heat_j_per_kg: 6_300_000.0,
            reversible: false,
        },
        // Oil → ethanol_vapor proxy at 473K (gas phase per spec § "oil liquid at -20-200°C, gas at >200°C").
        PhaseTransition {
            material: 19,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Gas,
            product_material: Some(59),
            threshold_k: 473.0,
            latent_heat_j_per_kg: 270_000.0,
            reversible: true,
        },
        // Alcohol → ethanol_vapor at 351K.
        PhaseTransition {
            material: 24,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Gas,
            product_material: Some(59),
            threshold_k: 351.0,
            latent_heat_j_per_kg: 841_000.0,
            reversible: true,
        },
        // Mercury → vapor at 630K (boil point per spec § "mercury liquid at -38°C to 357°C").
        PhaseTransition {
            material: 25,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Gas,
            product_material: None,
            threshold_k: 630.0,
            latent_heat_j_per_kg: 295_000.0,
            reversible: false,
        },
        // Wood → ash at 773K (combustion-grade threshold per spec § "wood solid; burns to ash at >500°C").
        PhaseTransition {
            material: 8,
            from_state: PhaseState::Solid,
            to_state: PhaseState::Powder,
            product_material: Some(40),
            threshold_k: 773.0,
            latent_heat_j_per_kg: -2_800_000.0,
            reversible: false,
        },
        // Blood → frozen_blood at 273K.
        PhaseTransition {
            material: 23,
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Solid,
            product_material: Some(72),
            threshold_k: 273.0,
            latent_heat_j_per_kg: -334_000.0,
            reversible: true,
        },
        // Snow (12) → water at 273K
        PhaseTransition {
            material: 12,
            from_state: PhaseState::Solid,
            to_state: PhaseState::Liquid,
            product_material: Some(13),
            threshold_k: 273.15,
            latent_heat_j_per_kg: 334_000.0,
            reversible: false,
        },
        // **M15B** § Steam (50) → cloud (71) when temperature drops below
        // 353.15 K (80°C). Per spec § "steam particles reach altitude >
        // 80 px with ambient temp < 80°C Then material_phase_nucleated
        // event fires with from='steam' to='cloud'". Note: the altitude
        // gate lives in the per-cell precipitation evaluator
        // (`crate::precipitation::evaluate_steam_nucleation`); this
        // entry covers the temperature side of the gate so the kernel
        // can fire the transition via the phase registry path too.
        PhaseTransition {
            material: 50, // steam
            from_state: PhaseState::Gas,
            to_state: PhaseState::Gas,
            product_material: Some(71), // cloud
            threshold_k: 353.15,
            latent_heat_j_per_kg: -200_000.0,
            reversible: true,
        },
        // **M15B** § Cloud (71) → rain (87) at 273.15K (precipitation
        // forms when the cloud is cool enough for droplets). Saturation
        // gating + tick-gate enforcement happens in
        // `crate::precipitation::update_cloud_cell`; this entry serves
        // as the temperature backstop so a deeply-cooled cloud cell
        // condenses out via the kernel path even when the saturation
        // tracker is bypassed (e.g. when a scenario manually seeds
        // cloud).
        PhaseTransition {
            material: 71, // cloud
            from_state: PhaseState::Gas,
            to_state: PhaseState::Liquid,
            product_material: Some(87), // rain
            threshold_k: 273.15,
            latent_heat_j_per_kg: -334_000.0,
            reversible: false,
        },
        // **M15B** § Rain (87) → water (13) on landing — the spec says
        // "puddles accumulate in low ground via cf-terrain liquid_flow",
        // so rain reverts to water once it pools. The transition fires
        // when the local temperature crosses below 273.15K (freeze) OR
        // when the surface is reached (handled by cf-terrain
        // liquid_flow). This phase entry is the temperature backstop
        // for the freezing case (rain → ice → water on melt) so the
        // chain stays reversible.
        PhaseTransition {
            material: 87, // rain
            from_state: PhaseState::Liquid,
            to_state: PhaseState::Solid,
            product_material: Some(15), // ice
            threshold_k: 273.15,
            latent_heat_j_per_kg: -334_000.0,
            reversible: true,
        },
    ];
    PhaseRegistry::new(raw.to_vec())
}

/// **M15** § event emitted on a phase transition. Per spec literal:
/// > Events: `material.phase_transition { material, from_state, to_state,
/// > pos, temperature }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseTransitionEvent {
    pub material: MaterialId,
    pub product_material: MaterialId,
    pub from_state: PhaseState,
    pub to_state: PhaseState,
    pub pos: [i32; 2],
    pub temperature_k: f32,
    pub latent_heat_j_per_kg: f32,
    pub direction: String,
    pub tick: u64,
}

/// Build a [`PhaseTransitionEvent`] from a matched transition.
#[must_use]
pub fn phase_transition_event(
    transition: &PhaseTransition,
    direction: PhaseDirection,
    pos: [i32; 2],
    temperature_k: f32,
    tick: u64,
) -> PhaseTransitionEvent {
    let (product, to_state) = transition.resolve(direction);
    let (from_state, latent_sign) = match direction {
        PhaseDirection::Forward => (transition.from_state, 1.0),
        PhaseDirection::Reverse => (transition.to_state, -1.0),
    };
    PhaseTransitionEvent {
        material: transition.material,
        product_material: product,
        from_state,
        to_state,
        pos,
        temperature_k,
        latent_heat_j_per_kg: transition.latent_heat_j_per_kg * latent_sign,
        direction: match direction {
            PhaseDirection::Forward => "forward".to_string(),
            PhaseDirection::Reverse => "reverse".to_string(),
        },
        tick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15-phase-001: default registry covers water/ice/steam at 273K
    /// and 373K thresholds.
    #[test]
    fn water_steam_threshold_is_373k() {
        let r = default_phase_registry();
        let water_to_steam = r
            .transitions
            .iter()
            .find(|t| t.material == 13 && matches!(t.to_state, PhaseState::Gas))
            .expect("water→steam present");
        assert!((water_to_steam.threshold_k - 373.15).abs() < 0.5);
    }

    /// VAL-M15-phase-002: forward firing — water at 360K → 380K transitions
    /// to steam.
    #[test]
    fn water_warms_past_boil_triggers_forward() {
        let r = default_phase_registry();
        let (t, dir) = r.evaluate(13, 360.0, 380.0).expect("fires");
        assert_eq!(dir, PhaseDirection::Forward);
        assert_eq!(t.product_material, Some(50));
    }

    /// VAL-M15-phase-003: reverse firing — steam at 380K → 360K condenses
    /// back to water.
    #[test]
    fn steam_cools_past_boil_triggers_reverse() {
        let r = default_phase_registry();
        let (t, dir) = r.evaluate(50, 380.0, 360.0).expect("fires");
        assert_eq!(dir, PhaseDirection::Reverse);
        let (mat, _state) = t.resolve(dir);
        assert_eq!(mat, 13, "back to water");
    }

    /// VAL-M15-phase-004: no transition mid-band.
    #[test]
    fn no_transition_within_a_band() {
        let r = default_phase_registry();
        assert!(r.evaluate(13, 290.0, 295.0).is_none(), "still liquid water");
        assert!(r.evaluate(50, 400.0, 405.0).is_none(), "still gas steam");
    }

    /// VAL-M15-phase-005: ice + heat → water. Implemented as the
    /// `reversible` reverse direction of water↔ice; `resolve(Reverse)`
    /// returns water id=13.
    #[test]
    fn ice_warms_past_melt_triggers_water() {
        let r = default_phase_registry();
        let (t, dir) = r.evaluate(15, 270.0, 280.0).expect("fires");
        assert_eq!(dir, PhaseDirection::Reverse);
        let (resulting_mat, resulting_state) = t.resolve(dir);
        assert_eq!(resulting_mat, 13, "ice melts back to water");
        assert_eq!(resulting_state, PhaseState::Liquid);
    }

    /// VAL-M15-phase-006: latent heat sign — water → steam consumes heat
    /// (positive); steam → water releases heat (negative).
    #[test]
    fn latent_heat_sign_inverts_on_reverse() {
        let r = default_phase_registry();
        let (t, _) = r.evaluate(13, 360.0, 380.0).expect("forward fires");
        let fwd_evt = phase_transition_event(t, PhaseDirection::Forward, [0, 0], 380.0, 1);
        let rev_evt = phase_transition_event(t, PhaseDirection::Reverse, [0, 0], 360.0, 2);
        assert!(fwd_evt.latent_heat_j_per_kg > 0.0);
        assert!(rev_evt.latent_heat_j_per_kg < 0.0);
    }

    /// VAL-M15-phase-007: event payload is round-trip serde-stable.
    #[test]
    fn event_round_trips_via_serde() {
        let r = default_phase_registry();
        let (t, dir) = r.evaluate(13, 360.0, 380.0).expect("fires");
        let evt = phase_transition_event(t, dir, [4, 5], 380.0, 99);
        let json = serde_json::to_string(&evt).expect("ser");
        let back: PhaseTransitionEvent = serde_json::from_str(&json).expect("de");
        assert_eq!(back, evt);
    }

    /// VAL-M15-phase-008: registry round-trips via serde.
    #[test]
    fn registry_round_trips() {
        let r = default_phase_registry();
        let json = serde_json::to_string(&r).expect("ser");
        let back: PhaseRegistry = serde_json::from_str(&json).expect("de");
        assert_eq!(back.len(), r.len());
    }

    /// VAL-M15-phase-009: blood freezes at 273K → frozen_blood
    #[test]
    fn blood_freezes_at_273k() {
        let r = default_phase_registry();
        let (t, dir) = r.evaluate(23, 280.0, 260.0).expect("fires");
        assert_eq!(dir, PhaseDirection::Forward);
        assert_eq!(t.product_material, Some(72));
    }
}
