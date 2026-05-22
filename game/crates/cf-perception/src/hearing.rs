//! M6: hearing channel (distance-attenuated loudness + reaction trigger).

use serde::{Deserialize, Serialize};

/// Distance-attenuation factor for a sound emitted at `emitter` heard at
/// `receiver` with the listener's `hearing_range`. Returns 1.0 at distance 0,
/// linearly fades to 0.0 at distance >= hearing_range. NaN/Inf-safe.
#[must_use]
pub fn distance_attenuation(distance: f32, hearing_range: f32) -> f32 {
    if !distance.is_finite() || !hearing_range.is_finite() {
        return 0.0;
    }
    if hearing_range <= 0.0 || distance >= hearing_range {
        return 0.0;
    }
    if distance <= 0.0 {
        return 1.0;
    }
    (1.0 - (distance / hearing_range)).clamp(0.0, 1.0)
}

/// One hearing probe: observer wants to know whether they hear a specific
/// emission.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HearingProbe {
    pub raw_loudness: f32,
    pub distance: f32,
    pub hearing_range: f32,
    pub occlusion_factor: f32,
    pub deafened: bool,
}

impl Default for HearingProbe {
    fn default() -> Self {
        Self {
            raw_loudness: 1.0,
            distance: 0.0,
            hearing_range: 480.0,
            occlusion_factor: 1.0,
            deafened: false,
        }
    }
}

/// Returns true if the receiver reacts to the emission given their hearing
/// range and any active deafen affliction.
///
/// non-linear. Heavy walls (occlusion ≈ 0.2) drop the signal far enough
/// below the reaction threshold that a closed-door listener doesn't
/// react to a gunshot the way an open-air listener does. The previous
/// linear multiplication left occlusion = 0.2 at eff = 0.12, just above
/// the 0.1 trigger — making "occluded" hearing identical to "open" for
/// reaction purposes. Squaring the occlusion factor matches the dB-style
/// logarithmic attenuation of sound through walls.
#[must_use]
pub fn hearing_reaction(probe: HearingProbe) -> bool {
    if probe.deafened {
        return false;
    }
    let att = distance_attenuation(probe.distance, probe.hearing_range);
    let occ = probe.occlusion_factor.clamp(0.0, 1.0);
    let occ_sq = occ * occ;
    let eff = (probe.raw_loudness * att * occ_sq).clamp(0.0, 1.0);
    eff > 0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_audible() {
        let probe = HearingProbe {
            distance: 10.0,
            hearing_range: 100.0,
            raw_loudness: 1.0,
            ..HearingProbe::default()
        };
        assert!(hearing_reaction(probe));
    }

    #[test]
    fn out_of_range_silent() {
        let probe = HearingProbe {
            distance: 200.0,
            hearing_range: 80.0,
            ..HearingProbe::default()
        };
        assert!(!hearing_reaction(probe));
    }

    #[test]
    fn deafened_ignores() {
        let probe = HearingProbe {
            distance: 5.0,
            hearing_range: 100.0,
            deafened: true,
            ..HearingProbe::default()
        };
        assert!(!hearing_reaction(probe));
    }

    #[test]
    fn occluded_reduces_response() {
        let open = HearingProbe {
            distance: 40.0,
            hearing_range: 100.0,
            occlusion_factor: 1.0,
            ..HearingProbe::default()
        };
        let closed = HearingProbe {
            occlusion_factor: 0.2,
            ..open
        };
        assert!(hearing_reaction(open));
        assert!(
            !hearing_reaction(closed)
                || distance_attenuation(closed.distance, closed.hearing_range) * closed.occlusion_factor <= 0.1
        );
    }

    #[test]
    fn nan_distance_silent() {
        assert_eq!(distance_attenuation(f32::NAN, 100.0), 0.0);
    }
}
