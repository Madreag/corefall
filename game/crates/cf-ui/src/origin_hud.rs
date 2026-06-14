//! M17 — per-origin resource HUD widget.
//!
//! Renders the survival bars that matter for the player's origin: organic
//! actors get Blood / Caloric / Oxygen, synthetics get Power / Oil / Heat,
//! hybrids get Blood / Power / Caloric. Bar selection is driven by the
//! canonical [`OriginProfile`] so a content override of an origin's resource
//! pools changes the HUD without touching this code.

use cf_actor::origin::{BodyPowerNeed, Origin, OriginProfile};

use crate::hud_model::HudResources;

/// Warning threshold — a resource bar flips to `warning` below 30%.
const LOW_FRACTION: f32 = 0.30;
/// Heat warning threshold — the heat bar flips to `warning` above 70%.
const HOT_FRACTION: f32 = 0.70;

/// One bar in the per-origin resource strip.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginHudBar {
    pub label: String,
    pub fraction: f32,
    pub color: &'static str,
    pub warning: bool,
}

/// HUD archetype the origin maps to (governs which bars appear).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HudArchetype {
    Organic,
    Synthetic,
    Hybrid,
}

fn archetype(origin: Origin, profile: &OriginProfile) -> HudArchetype {
    if origin.is_power_survival() {
        HudArchetype::Synthetic
    } else if profile.body_power_need == BodyPowerNeed::Partial {
        HudArchetype::Hybrid
    } else {
        HudArchetype::Organic
    }
}

fn bar(label: &str, value: f32, max: f32, color: &'static str) -> OriginHudBar {
    let fraction = if max > 0.0 {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    OriginHudBar {
        label: label.to_string(),
        fraction,
        color,
        warning: fraction < LOW_FRACTION,
    }
}

fn heat_bar(heat: f32) -> OriginHudBar {
    OriginHudBar {
        label: "HEAT".to_string(),
        fraction: heat.clamp(0.0, 1.0),
        color: "orange",
        warning: heat > HOT_FRACTION,
    }
}

/// Resolve the resource bars for the given origin snapshot. Bars appear only
/// for pools the origin actually owns; BOOST / THROTTLE / RESERVE indicators
/// are appended when the matching power flags are set.
#[must_use]
pub fn origin_hud_lines(res: &HudResources) -> Vec<OriginHudBar> {
    let origin = Origin::from_str(&res.origin);
    let profile = OriginProfile::canonical(origin);
    let mut bars = Vec::new();

    match archetype(origin, &profile) {
        HudArchetype::Organic => {
            if profile.has_blood() {
                let max = if res.blood_max > 0.0 { res.blood_max } else { profile.blood_max_ml };
                bars.push(bar("BLOOD", res.blood, max, "red"));
            }
            if profile.has_caloric() {
                bars.push(bar("CALORIC", res.caloric, profile.caloric_max, "orange"));
            }
            if res.oxygen_seconds > 0.0 && profile.oxygen_supply_seconds > 0.0 {
                bars.push(bar("O2", res.oxygen_seconds, profile.oxygen_supply_seconds, "cyan"));
            }
        }
        HudArchetype::Synthetic => {
            if profile.has_power() {
                let max = if res.power_max > 0.0 { res.power_max } else { profile.power_max_kwh };
                bars.push(bar("POWER", res.power, max, "cyan"));
            }
            if profile.has_oil() {
                let max = if res.oil_max > 0.0 { res.oil_max } else { profile.oil_max_ml };
                bars.push(bar("OIL", res.oil, max, "yellow"));
            }
            bars.push(heat_bar(res.heat));
        }
        HudArchetype::Hybrid => {
            if profile.has_blood() {
                let max = if res.blood_max > 0.0 { res.blood_max } else { profile.blood_max_ml };
                bars.push(bar("BLOOD", res.blood, max, "red"));
            }
            if profile.has_power() {
                let max = if res.power_max > 0.0 { res.power_max } else { profile.power_max_kwh };
                bars.push(bar("POWER", res.power, max, "cyan"));
            }
            if profile.has_caloric() {
                bars.push(bar("CALORIC", res.caloric, profile.caloric_max, "orange"));
            }
        }
    }

    if res.overclock_tier > 0 {
        bars.push(OriginHudBar {
            label: "BOOST".to_string(),
            fraction: 1.0,
            color: "magenta",
            warning: false,
        });
    }
    if res.throttled {
        bars.push(OriginHudBar {
            label: "THROTTLE".to_string(),
            fraction: 1.0,
            color: "red",
            warning: true,
        });
    }
    if res.power_fire_locked {
        bars.push(OriginHudBar {
            label: "RESERVE".to_string(),
            fraction: 1.0,
            color: "yellow",
            warning: true,
        });
    }

    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(bars: &[OriginHudBar]) -> Vec<String> {
        bars.iter().map(|b| b.label.clone()).collect()
    }

    #[test]
    fn human_shows_blood_caloric_oxygen() {
        let res = HudResources {
            origin: "human".to_string(),
            blood: 5000.0,
            blood_max: 5000.0,
            caloric: 80.0,
            oxygen_seconds: 1800.0,
            ..Default::default()
        };
        let bars = origin_hud_lines(&res);
        assert_eq!(labels(&bars), vec!["BLOOD", "CALORIC", "O2"]);
        assert!(bars.iter().all(|b| b.label != "OIL" && b.label != "POWER" && b.label != "HEAT"));
    }

    #[test]
    fn human_hides_oxygen_when_depleted() {
        let res = HudResources {
            origin: "human".to_string(),
            blood: 5000.0,
            blood_max: 5000.0,
            caloric: 80.0,
            oxygen_seconds: 0.0,
            ..Default::default()
        };
        let bars = origin_hud_lines(&res);
        assert_eq!(labels(&bars), vec!["BLOOD", "CALORIC"]);
    }

    #[test]
    fn robot_shows_power_oil_heat() {
        let res = HudResources {
            origin: "robot".to_string(),
            power: 60.0,
            power_max: 100.0,
            oil: 4000.0,
            oil_max: 5000.0,
            heat: 0.4,
            ..Default::default()
        };
        let bars = origin_hud_lines(&res);
        assert_eq!(labels(&bars), vec!["POWER", "OIL", "HEAT"]);
        assert!(bars.iter().all(|b| b.label != "BLOOD" && b.label != "CALORIC" && b.label != "O2"));
    }

    #[test]
    fn android_shows_blood_power_caloric_no_oil() {
        let res = HudResources {
            origin: "android".to_string(),
            blood: 4000.0,
            blood_max: 4000.0,
            power: 30.0,
            power_max: 60.0,
            oil: 3000.0,
            oil_max: 3000.0,
            caloric: 40.0,
            ..Default::default()
        };
        let bars = origin_hud_lines(&res);
        assert_eq!(labels(&bars), vec!["BLOOD", "POWER", "CALORIC"]);
        assert!(bars.iter().all(|b| b.label != "OIL" && b.label != "HEAT"));
    }

    #[test]
    fn low_resource_flips_warning() {
        let res = HudResources {
            origin: "human".to_string(),
            blood: 1000.0,
            blood_max: 5000.0,
            caloric: 90.0,
            ..Default::default()
        };
        let bars = origin_hud_lines(&res);
        let blood = bars.iter().find(|b| b.label == "BLOOD").unwrap();
        assert!(blood.warning, "20% blood should warn");
        let caloric = bars.iter().find(|b| b.label == "CALORIC").unwrap();
        assert!(!caloric.warning, "90% caloric should not warn");
    }

    #[test]
    fn hot_robot_flips_heat_warning() {
        let cool = HudResources {
            origin: "robot".to_string(),
            power: 80.0,
            power_max: 100.0,
            heat: 0.1,
            ..Default::default()
        };
        let heat = origin_hud_lines(&cool).into_iter().find(|b| b.label == "HEAT").unwrap();
        assert!(!heat.warning, "cold robot heat should not warn");

        let hot = HudResources { heat: 0.9, ..cool };
        let heat = origin_hud_lines(&hot).into_iter().find(|b| b.label == "HEAT").unwrap();
        assert!(heat.warning, "hot robot heat should warn");
    }

    #[test]
    fn power_flags_surface_indicators() {
        let res = HudResources {
            origin: "robot".to_string(),
            power: 80.0,
            power_max: 100.0,
            overclock_tier: 2,
            throttled: true,
            power_fire_locked: true,
            ..Default::default()
        };
        let labels = labels(&origin_hud_lines(&res));
        assert!(labels.contains(&"BOOST".to_string()));
        assert!(labels.contains(&"THROTTLE".to_string()));
        assert!(labels.contains(&"RESERVE".to_string()));
    }
}
