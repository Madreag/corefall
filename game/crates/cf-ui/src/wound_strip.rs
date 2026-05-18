//! **M14G** § Per-zone wound-strip silhouette badges.
//!
//! Pure helpers that turn an actor's [`cf_wound::ActorWoundList`] into a
//! deterministic vector of [`WoundBadge`] entries the M11 silhouette
//! renderer consumes.
//!
//! Invariants:
//! - Max 5 full badges per zone; the 6th wound and beyond collapse into a
//!   single `"+N"` overflow badge.
//! - Selection of which 5 wounds remain visible is severity-descending +
//!   wound-id-ascending (deterministic across reseeded runs).
//! - The Medic auto-triage scorer reads `medic_utility_delta` per badge so
//!   the scorer can sum contributions across an actor.

use cf_wound::registry::ZoneId;
use cf_wound::severity::SeverityBand;
use cf_wound::{ActorWoundList, Wound, WoundKind, WoundVisibleState};

pub const MAX_BADGES_PER_ZONE: usize = 5;

/// **M14G** wound-strip badge.
#[derive(Debug, Clone, PartialEq)]
pub struct WoundBadge {
    pub zone: ZoneId,
    pub kind: Option<WoundKind>,
    pub severity: f32,
    pub label: String,
    pub color: [u8; 4],
    pub decal_id: String,
    pub visible_state: Option<WoundVisibleState>,
    pub medic_utility_delta: f32,
    pub is_overflow: bool,
    pub overflow_count: u32,
}

impl WoundBadge {
    /// Per-band caption used by the silhouette renderer.
    pub fn caption(&self) -> &str {
        &self.label
    }
}

/// Render the per-actor wound strip.
///
/// `decal_for(kind)` lets the caller plug in the registry's `WoundSpec.decal_id`
/// mapping; pass `|_| String::new()` if the caller doesn't need decal ids.
pub fn render<F>(list: &ActorWoundList, mut decal_for: F) -> Vec<WoundBadge>
where
    F: FnMut(WoundKind) -> String,
{
    let mut out: Vec<WoundBadge> = Vec::new();
    for (zone, wounds) in list.iter() {
        for badge in render_zone_with(zone.clone(), wounds, &mut decal_for) {
            out.push(badge);
        }
    }
    out
}

/// Render the badges for a single zone (used by the M11 silhouette renderer).
pub fn render_zone<F>(zone: ZoneId, wounds: &[Wound], decal_for: F) -> Vec<WoundBadge>
where
    F: FnMut(WoundKind) -> String,
{
    let mut f = decal_for;
    render_zone_with(zone, wounds, &mut f)
}

fn render_zone_with<F>(zone: ZoneId, wounds: &[Wound], decal_for: &mut F) -> Vec<WoundBadge>
where
    F: FnMut(WoundKind) -> String,
{
    if wounds.is_empty() {
        return Vec::new();
    }
    // Deterministic ordering: severity-descending, then wound-id-ascending.
    let mut ordered: Vec<&Wound> = wounds.iter().collect();
    ordered.sort_by(|a, b| {
        b.severity
            .partial_cmp(&a.severity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut out: Vec<WoundBadge> = Vec::with_capacity(MAX_BADGES_PER_ZONE + 1);
    let max_full = MAX_BADGES_PER_ZONE.min(ordered.len());
    for w in ordered.iter().take(max_full) {
        let band = SeverityBand::from_severity(w.severity);
        let triage = band.auto_triage_threshold();
        out.push(WoundBadge {
            zone: zone.clone(),
            kind: Some(w.kind),
            severity: w.severity,
            label: band.label().to_string(),
            color: band.color(),
            decal_id: decal_for(w.kind),
            visible_state: Some(w.visible_state),
            medic_utility_delta: triage,
            is_overflow: false,
            overflow_count: 0,
        });
    }
    if ordered.len() > MAX_BADGES_PER_ZONE {
        let extra = ordered.len() - MAX_BADGES_PER_ZONE;
        out.push(WoundBadge {
            zone,
            kind: None,
            severity: 0.0,
            label: format!("+{}", extra),
            color: [80, 80, 80, 255],
            decal_id: String::new(),
            visible_state: None,
            medic_utility_delta: 0.0,
            is_overflow: true,
            overflow_count: extra as u32,
        });
    }
    out
}

/// Determine the badge variant for a single wound (used by tests +
/// VAL-M14G-042 to assert 6 distinguishable visible states).
pub fn badge_state(w: &Wound) -> WoundBadgeVariant {
    if w.scarred || matches!(w.visible_state, WoundVisibleState::Scar) {
        return WoundBadgeVariant::Scar;
    }
    if w.scabbed || matches!(w.visible_state, WoundVisibleState::Scab) {
        return WoundBadgeVariant::Scab;
    }
    match w.visible_state {
        WoundVisibleState::Fresh => WoundBadgeVariant::Fresh,
        WoundVisibleState::BandageSoaked => WoundBadgeVariant::BandageSoaked,
        WoundVisibleState::CleanBandage => WoundBadgeVariant::CleanBandage,
        WoundVisibleState::SutureLine => WoundBadgeVariant::SutureLine,
        WoundVisibleState::Scab => WoundBadgeVariant::Scab,
        WoundVisibleState::Scar => WoundBadgeVariant::Scar,
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WoundBadgeVariant {
    Fresh = 0,
    BandageSoaked = 1,
    CleanBandage = 2,
    Scab = 3,
    SutureLine = 4,
    Scar = 5,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_wound::severity::BAND_LABEL_CRITICAL;
    use cf_wound::{WoundId, WoundKind};

    fn mk_wound(id: u64, kind: WoundKind, severity: f32, zone: &str) -> Wound {
        Wound::new(WoundId(id), kind, severity, ZoneId::from(zone))
    }

    /// VAL-M14G-020: severity 0.85 surfaces `[!!] CRITICAL` caption and the
    /// medic auto-triage delta is 0.4.
    #[test]
    fn critical_badge_caption_and_triage() {
        let w = mk_wound(1, WoundKind::Burn3rd, 0.85, "torso_front");
        let badges = render_zone(ZoneId::from("torso_front"), std::slice::from_ref(&w), |_| String::new());
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].caption(), BAND_LABEL_CRITICAL);
        assert!((badges[0].medic_utility_delta - 0.4).abs() < 1e-6);
    }

    /// VAL-M14G-026: per-zone overflow collapse at 6 wounds → 5 badges + `+1`.
    #[test]
    fn per_zone_overflow_collapse() {
        let zone = ZoneId::from("arm_left");
        let wounds: Vec<Wound> = (1..=6)
            .map(|i| mk_wound(i, WoundKind::LacerationLight, 0.1 * i as f32, "arm_left"))
            .collect();
        let badges = render_zone(zone.clone(), &wounds, |_| String::new());
        assert_eq!(badges.len(), 6, "5 full + 1 overflow");
        let overflow = badges.last().unwrap();
        assert!(overflow.is_overflow);
        assert_eq!(overflow.overflow_count, 1);
        assert_eq!(overflow.label, "+1");

        let wounds10: Vec<Wound> = (1..=10)
            .map(|i| mk_wound(i, WoundKind::LacerationLight, 0.1 * i as f32, "arm_left"))
            .collect();
        let badges10 = render_zone(zone, &wounds10, |_| String::new());
        assert_eq!(badges10.len(), 6);
        assert_eq!(badges10.last().unwrap().label, "+5");
        assert_eq!(badges10.last().unwrap().overflow_count, 5);
    }

    /// VAL-M14G-049: mixed-kind overflow collapse renders identically to
    /// same-kind overflow.
    #[test]
    fn mixed_kind_overflow_collapse() {
        let zone = ZoneId::from("torso_front");
        let mut wounds: Vec<Wound> = Vec::new();
        for i in 1..=3 {
            wounds.push(mk_wound(i, WoundKind::LacerationLight, 0.1 * i as f32, "torso_front"));
        }
        for i in 4..=5 {
            wounds.push(mk_wound(i, WoundKind::LacerationModerate, 0.2 + 0.1 * i as f32, "torso_front"));
        }
        wounds.push(mk_wound(6, WoundKind::LacerationSevere, 0.95, "torso_front"));
        let badges = render_zone(zone, &wounds, |_| String::new());
        assert_eq!(badges.len(), 6);
        let overflow = badges.last().unwrap();
        assert!(overflow.is_overflow);
        assert_eq!(overflow.overflow_count, 1);
    }

    /// VAL-M14G-050: deterministic ordering when n > 5 — two identical
    /// inputs return byte-identical badge buffers.
    #[test]
    fn overflow_selection_deterministic() {
        let mut list = ActorWoundList::new();
        let zone = ZoneId::from("torso_front");
        for i in 1..=10 {
            list.push(
                zone.clone(),
                mk_wound(i, WoundKind::LacerationLight, 0.05 + (i as f32) * 0.05, "torso_front"),
            );
        }
        let first = render_zone(zone.clone(), list.zone(&zone), |_| String::new());
        let second = render_zone(zone.clone(), list.zone(&zone), |_| String::new());
        assert_eq!(first, second);
    }

    /// VAL-M14G-031: badge decal_id is taken from the spec mapping.
    #[test]
    fn badge_decal_id_and_caption_mapping() {
        let zone = ZoneId::from("torso_front");
        let wounds = vec![
            mk_wound(1, WoundKind::LacerationLight, 0.2, "torso_front"),
            mk_wound(2, WoundKind::Burn3rd, 0.85, "torso_front"),
        ];
        let badges = render_zone(zone, &wounds, |k| format!("decal.{}", k.as_str()));
        // Sorted severity-descending → Burn3rd first.
        assert_eq!(badges[0].kind, Some(WoundKind::Burn3rd));
        assert_eq!(badges[0].decal_id, "decal.Burn3rd");
        assert_eq!(badges[0].caption(), BAND_LABEL_CRITICAL);
        assert_eq!(badges[1].kind, Some(WoundKind::LacerationLight));
        assert_eq!(badges[1].decal_id, "decal.LacerationLight");
    }

    /// VAL-M14G-042: each of the 6 visible states produces a distinct
    /// `WoundBadgeVariant`.
    #[test]
    fn six_visible_states_distinct_badges() {
        let states = [
            WoundVisibleState::Fresh,
            WoundVisibleState::BandageSoaked,
            WoundVisibleState::CleanBandage,
            WoundVisibleState::SutureLine,
            WoundVisibleState::Scab,
            WoundVisibleState::Scar,
        ];
        let mut set = std::collections::HashSet::new();
        for s in &states {
            let mut w = mk_wound(1, WoundKind::LacerationLight, 0.3, "torso_front");
            w.visible_state = *s;
            if *s == WoundVisibleState::Scab {
                w.scabbed = true;
            }
            if *s == WoundVisibleState::Scar {
                w.scarred = true;
            }
            set.insert(badge_state(&w));
        }
        assert_eq!(set.len(), 6);
    }

    /// VAL-M14G-012: through-and-through emits 2 badges on the silhouette.
    #[test]
    fn through_and_through_bleed_sums_and_badge_count() {
        let mut list = ActorWoundList::new();
        list.push(
            ZoneId::from("torso_front"),
            mk_wound(1, WoundKind::GunshotEntry, 0.4, "torso_front"),
        );
        list.push(
            ZoneId::from("torso_back"),
            mk_wound(2, WoundKind::GunshotExit, 0.4, "torso_back"),
        );
        let badges = render(&list, |_| String::new());
        assert_eq!(badges.len(), 2);
    }
}
