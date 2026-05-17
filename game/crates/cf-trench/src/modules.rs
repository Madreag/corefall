//! M9B: embedded trench modules.
//!
//! Per spec §"Trench modules (embedded sub-content)" the launch surface
//! ships six modules:
//!
//! | Module          | Function                                                                | Material cost    | Build time |
//! |---|---|---|---|
//! | `duckboard`     | Floor planks over mud; converts wet-mud slipping to firm footing       | 2 wood           | 4 s        |
//! | `fire_step`     | Raised 8-px platform in trench wall; standing on step = exposed shoot  | 4 dirt + 1 wood  | 8 s        |
//! | `breastwork`    | Sandbag wall above grade (M9C parts); HP 400 vs small-arms             | 6 sandbags       | 12 s       |
//! | `drainage_sump` | Gravity-fed drain at trench low point; flushes water tiles             | 2 dirt + 1 pipe  | 6 s        |
//! | `revetment`     | Wood/iron-mesh side reinforcement; M14E integrity field 600            | 4 wood + 2 iron  | 10 s       |
//! | `corner_traverse`| Reinforced corner at zigzag kinks; M14 prevents grenade-frag funneling | 2 dirt + 4 sandbags | 6 s     |

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One of the six embedded trench modules. Owned by m9b-1; consumed by
/// m9b-3 (cfctl `act.player.place_trench_module` + `repair_trench_module`)
/// and m9b-4 (HUD + render layers).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrenchModule {
    Duckboard = 0,
    FireStep = 1,
    Breastwork = 2,
    DrainageSump = 3,
    Revetment = 4,
    CornerTraverse = 5,
}

impl TrenchModule {
    pub const ALL: [TrenchModule; 6] = [
        TrenchModule::Duckboard,
        TrenchModule::FireStep,
        TrenchModule::Breastwork,
        TrenchModule::DrainageSump,
        TrenchModule::Revetment,
        TrenchModule::CornerTraverse,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            TrenchModule::Duckboard => "duckboard",
            TrenchModule::FireStep => "fire_step",
            TrenchModule::Breastwork => "breastwork",
            TrenchModule::DrainageSump => "drainage_sump",
            TrenchModule::Revetment => "revetment",
            TrenchModule::CornerTraverse => "corner_traverse",
        }
    }
}

/// On-disk schema for `content/trench_modules/<module>.ron`. Material cost
/// is authored as an ordered map of `(resource id → integer count)` so
/// modders can extend with new resource ids without churning the kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub module: TrenchModule,
    pub material_cost: BTreeMap<String, u32>,
    pub build_time_seconds: u32,
}

impl ModuleSpec {
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<ModuleSpec>(text)
    }

    /// Sum of the unit-counts in the cost map (used by HUD tooltips +
    /// AI doctrine cost heuristics).
    #[must_use]
    pub fn total_units(&self) -> u32 {
        self.material_cost.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(module: TrenchModule) -> ModuleSpec {
        let path = match module {
            TrenchModule::Duckboard => "duckboard",
            TrenchModule::FireStep => "fire_step_module",
            TrenchModule::Breastwork => "breastwork",
            TrenchModule::DrainageSump => "drainage_sump",
            TrenchModule::Revetment => "revetment",
            TrenchModule::CornerTraverse => "corner_traverse",
        };
        let rel = format!("../../content/trench_modules/{}.ron", path);
        let bytes = std::fs::read_to_string(&rel)
            .unwrap_or_else(|e| panic!("read {}: {}", rel, e));
        ModuleSpec::from_ron_str(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {}", rel, e))
    }

    #[test]
    fn loads_duckboard() {
        let m = load(TrenchModule::Duckboard);
        assert_eq!(m.module, TrenchModule::Duckboard);
        assert_eq!(m.material_cost.get("wood"), Some(&2));
        assert_eq!(m.material_cost.len(), 1);
        assert_eq!(m.build_time_seconds, 4);
    }

    #[test]
    fn loads_fire_step_module() {
        let m = load(TrenchModule::FireStep);
        assert_eq!(m.material_cost.get("dirt"), Some(&4));
        assert_eq!(m.material_cost.get("wood"), Some(&1));
        assert_eq!(m.build_time_seconds, 8);
    }

    #[test]
    fn loads_breastwork() {
        let m = load(TrenchModule::Breastwork);
        assert_eq!(m.material_cost.get("sandbag"), Some(&6));
        assert_eq!(m.build_time_seconds, 12);
    }

    #[test]
    fn loads_drainage_sump() {
        let m = load(TrenchModule::DrainageSump);
        assert_eq!(m.material_cost.get("dirt"), Some(&2));
        assert_eq!(m.material_cost.get("pipe"), Some(&1));
        assert_eq!(m.build_time_seconds, 6);
    }

    #[test]
    fn loads_revetment() {
        let m = load(TrenchModule::Revetment);
        assert_eq!(m.material_cost.get("wood"), Some(&4));
        assert_eq!(m.material_cost.get("iron"), Some(&2));
        assert_eq!(m.build_time_seconds, 10);
    }

    #[test]
    fn loads_corner_traverse() {
        let m = load(TrenchModule::CornerTraverse);
        assert_eq!(m.material_cost.get("dirt"), Some(&2));
        assert_eq!(m.material_cost.get("sandbag"), Some(&4));
        assert_eq!(m.build_time_seconds, 6);
    }

    /// VAL-M9B-MODULES-001: every module's authored RON round-trips
    /// cleanly through `ron`.
    #[test]
    fn module_ron_load_round_trip_all() {
        for module in TrenchModule::ALL {
            let m = load(module);
            let serialized =
                ron::ser::to_string_pretty(&m, ron::ser::PrettyConfig::default()).expect("ser");
            let parsed = ModuleSpec::from_ron_str(&serialized).expect("reparse");
            assert_eq!(m, parsed, "module round-trip diverged for {:?}", module);
        }
    }

    #[test]
    fn module_as_str_round_trip() {
        for module in TrenchModule::ALL {
            let s = module.as_str();
            let parsed: TrenchModule =
                ron::from_str(s).expect("module serde matches as_str");
            assert_eq!(parsed, module);
        }
    }
}
