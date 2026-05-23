//! M6C: sensor SKU registry (5 SKUs).
//!
//! Per M6C § "Sensors (5 new beyond M29B's existing list)":
//! - radio_direction_finder, sound_detector_passive, heat_camera_handheld,
//!   radar_compact_t2, geological_surveyor_m30d.

use serde::{Deserialize, Serialize};

pub const RADIO_DIRECTION_FINDER_ID: &str = "radio_direction_finder";
pub const SOUND_DETECTOR_PASSIVE_ID: &str = "sound_detector_passive";
pub const HEAT_CAMERA_HANDHELD_ID: &str = "heat_camera_handheld";
pub const RADAR_COMPACT_T2_ID: &str = "radar_compact_t2";
pub const GEOLOGICAL_SURVEYOR_M30D_ID: &str = "geological_surveyor_m30d";
/// M16 § Anomaly detector item id. Carrying this satisfies the Gherkin
/// "Given anomaly detector in inventory" surface contract.
pub const ANOMALY_DETECTOR_ID: &str = "anomaly_detector";

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    RadioDf = 0,
    Acoustic = 1,
    Thermal = 2,
    Radar = 3,
    Geological = 4,
}

impl SensorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SensorKind::RadioDf => "radio_df",
            SensorKind::Acoustic => "acoustic",
            SensorKind::Thermal => "thermal",
            SensorKind::Radar => "radar",
            SensorKind::Geological => "geological",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorPreset {
    pub id: String,
    pub display_name: String,
    pub kind: SensorKind,
    /// Detection radius in world units.
    pub radius: f32,
    /// Active power draw in watts (drives the M19/M19N battery model).
    pub power_draw_w: f32,
    /// True when the sensor radiates a detectable signature (radar);
    /// passive sensors stay stealthy.
    pub active_emission: bool,
    pub mass_kg: f32,
}

#[must_use]
pub fn m6c_sensor_presets() -> Vec<SensorPreset> {
    vec![
        SensorPreset {
            id: RADIO_DIRECTION_FINDER_ID.to_string(),
            display_name: "Radio Direction Finder".to_string(),
            kind: SensorKind::RadioDf,
            radius: 800.0,
            power_draw_w: 5.0,
            active_emission: false,
            mass_kg: 1.4,
        },
        SensorPreset {
            id: SOUND_DETECTOR_PASSIVE_ID.to_string(),
            display_name: "Passive Sound Detector".to_string(),
            kind: SensorKind::Acoustic,
            radius: 400.0,
            power_draw_w: 2.0,
            active_emission: false,
            mass_kg: 0.8,
        },
        SensorPreset {
            id: HEAT_CAMERA_HANDHELD_ID.to_string(),
            display_name: "Handheld Heat Camera".to_string(),
            kind: SensorKind::Thermal,
            radius: 250.0,
            power_draw_w: 4.0,
            active_emission: false,
            mass_kg: 1.0,
        },
        SensorPreset {
            id: RADAR_COMPACT_T2_ID.to_string(),
            display_name: "Compact Radar (T2)".to_string(),
            kind: SensorKind::Radar,
            radius: 1500.0,
            power_draw_w: 28.0,
            active_emission: true,
            mass_kg: 6.0,
        },
        SensorPreset {
            id: GEOLOGICAL_SURVEYOR_M30D_ID.to_string(),
            display_name: "Geological Surveyor (M30D)".to_string(),
            kind: SensorKind::Geological,
            radius: 50.0,
            power_draw_w: 12.0,
            active_emission: false,
            mass_kg: 3.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_five_skus() {
        assert_eq!(m6c_sensor_presets().len(), 5);
    }

    #[test]
    fn radar_is_active_emission() {
        let v = m6c_sensor_presets();
        let r = v.iter().find(|p| p.kind == SensorKind::Radar).unwrap();
        assert!(r.active_emission);
    }
}
