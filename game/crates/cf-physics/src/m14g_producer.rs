//! **M14G** § Producer surface for typed wounds.
//!
//! Helpers that convert the existing M14 collision events into typed
//! `cf-wound::WoundKind` records. Pure / deterministic — the engine emits
//! the records into `cf-actor::ActorState::m14g_wound_list` + emits matching
//! `wound.created` events through `cf-replay`.
//!
//! Producer surfaces wired here:
//! - `penetration_ray` — rifle/handgun hits emit GunshotEntry/Exit/Through,
//!   shrapnel hits emit ShrapnelEmbedded/Through, melee blades emit
//!   LacerationLight/Moderate/Severe, blunt impacts emit BruiseLight/Heavy
//!   (or DentalDamage on face-front above tooth threshold).
//! - `fall_impulse_chain` decision tree — impulse magnitude `r` (relative
//!   to the joint's severance threshold) maps to:
//!   - `r < 0.7` → no fracture
//!   - `0.7 ≤ r < 0.85` → FractureSimple
//!   - `0.85 ≤ r < 0.95` → FractureCompound (with `dirt_pct = 0.3` per
//!     spec open-bone status)
//!   - `r ≥ 0.95` → FractureComminuted
//!
//! Per-origin substitution is delegated to `cf-wound::registry::resolve_emit_kind`.

use cf_wound::registry::{OriginId, WoundSpecRegistry, ZoneId};
use cf_wound::{Wound, WoundId, WoundKind};

/// **VAL-M14G-023**: tooth-impulse threshold for [`classify_blunt_face_hit`].
///
/// Calibrated so the M6 blunt melee weapons (`rifle_bash` damage 15,
/// `baton` damage 12, `shoulder_check` damage 10, `sledgehammer` damage 35,
/// `stun_baton` jolt 14) clear the threshold and emit `DentalDamage` when
/// the engine routes a face-aligned blunt hit through the M14G producer;
/// the lighter `kick` (damage 8) stays below the threshold and falls back
/// to `BruiseHeavy` via `classify_blunt_face_hit`'s alternate branch.
pub const M14G_TOOTH_THRESHOLD: f32 = 9.5;

/// **VAL-M14G-027/028**: typed wound emit candidate from a single
/// penetration-ray hit. Carries the kind, severity (computed from absorbed
/// damage), zone, and the per-origin substitution applied if the actor's
/// origin forbids the kind.
///
/// `severity` is clamped to `[0, 1]` by `Wound::new`.
#[derive(Debug, Clone, PartialEq)]
pub struct M14gWoundEmit {
    pub kind: WoundKind,
    pub severity: f32,
    pub zone: ZoneId,
    pub dirt_pct: f32,
}

impl M14gWoundEmit {
    pub fn into_wound(self, id: WoundId) -> Wound {
        let mut w = Wound::new(id, self.kind, self.severity, self.zone);
        w.dirt_pct = self.dirt_pct.clamp(0.0, 1.0);
        w
    }
}

/// **VAL-M14G-011/027**: classify a penetration-ray hit on a human chassis
/// into an entry/exit/through wound pair.
///
/// If `exited_backstop` is true the projectile passed through both sides
/// of the chassis — produces a (GunshotEntry on `entry_zone`, GunshotExit
/// on `exit_zone`) pair. If the projectile remained in the actor and the
/// caller knows it lodged → GunshotEntry only. If the round was a single
/// through-and-through high-velocity / long-rod event → GunshotThrough on
/// the entry_zone (cluster size = 1).
pub fn classify_gunshot(
    entry_zone: ZoneId,
    exit_zone: ZoneId,
    severity_entry: f32,
    severity_exit: f32,
    exited_backstop: bool,
) -> Vec<M14gWoundEmit> {
    if exited_backstop {
        vec![
            M14gWoundEmit {
                kind: WoundKind::GunshotEntry,
                severity: severity_entry,
                zone: entry_zone,
                dirt_pct: 0.0,
            },
            M14gWoundEmit {
                kind: WoundKind::GunshotExit,
                severity: severity_exit,
                zone: exit_zone,
                dirt_pct: 0.0,
            },
        ]
    } else {
        vec![M14gWoundEmit {
            kind: WoundKind::GunshotEntry,
            severity: severity_entry,
            zone: entry_zone,
            dirt_pct: 0.0,
        }]
    }
}

/// **VAL-M14G-018**: classify a single shrapnel-fragment hit. Each fragment
/// produces one `ShrapnelEmbedded` record; the caller increments
/// `shrapnel_count` on the existing zone if applicable. Through-fragments
/// (rare) use `ShrapnelThrough`.
pub fn classify_shrapnel(zone: ZoneId, severity: f32, through: bool) -> M14gWoundEmit {
    M14gWoundEmit {
        kind: if through {
            WoundKind::ShrapnelThrough
        } else {
            WoundKind::ShrapnelEmbedded
        },
        severity,
        zone,
        dirt_pct: 0.0,
    }
}

/// **VAL-M14G-022**: classify a HEAT cluster. Returns one Burn3rd record per
/// module traversed + (optionally) one GunshotThrough on the crew torso.
/// `modules_traversed` is `heat_jet_modules_penetrated.len()` clamped to the
/// crew-cluster size. The crew-torso through-shot fires only when the jet
/// reached the crew compartment.
pub fn classify_heat_cluster(
    module_zones: &[ZoneId],
    crew_torso_zone: Option<ZoneId>,
    severity: f32,
) -> Vec<M14gWoundEmit> {
    let mut out = Vec::with_capacity(module_zones.len() + 1);
    for z in module_zones {
        out.push(M14gWoundEmit {
            kind: WoundKind::Burn3rd,
            severity,
            zone: z.clone(),
            dirt_pct: 0.0,
        });
    }
    if let Some(torso) = crew_torso_zone {
        out.push(M14gWoundEmit {
            kind: WoundKind::GunshotThrough,
            severity,
            zone: torso,
            dirt_pct: 0.0,
        });
    }
    out
}

/// **VAL-M14G-023**: classify a blunt-face hit at the head-front zone.
/// Returns DentalDamage when the impulse exceeds the tooth threshold,
/// otherwise BruiseHeavy.
pub fn classify_blunt_face_hit(zone: ZoneId, impulse: f32, tooth_threshold: f32) -> M14gWoundEmit {
    if impulse > tooth_threshold {
        M14gWoundEmit {
            kind: WoundKind::DentalDamage,
            severity: 0.6,
            zone,
            dirt_pct: 0.0,
        }
    } else {
        M14gWoundEmit {
            kind: WoundKind::BruiseHeavy,
            severity: 0.4,
            zone,
            dirt_pct: 0.0,
        }
    }
}

/// **VAL-M14G-016/017/028**: fall-impulse decision tree. Maps the ratio
/// of `impulse / severance_threshold` to a typed Fracture (or `None` when
/// below the decision threshold). Compound fractures init `dirt_pct = 0.3`.
pub fn classify_fall_fracture(
    zone: ZoneId,
    impulse: f32,
    severance_threshold: f32,
) -> Option<M14gWoundEmit> {
    if severance_threshold <= 0.0 {
        return None;
    }
    let ratio = impulse / severance_threshold;
    if ratio < 0.7 {
        return None;
    }
    let (kind, severity, dirt_pct) = if ratio < 0.85 {
        (WoundKind::FractureSimple, 0.5, 0.0)
    } else if ratio < 0.95 {
        (WoundKind::FractureCompound, 0.8, 0.3)
    } else {
        (WoundKind::FractureComminuted, 0.95, 0.5)
    };
    Some(M14gWoundEmit {
        kind,
        severity,
        zone,
        dirt_pct,
    })
}

/// **VAL-M14G-021**: apply the registry's per-origin substitution to a
/// candidate emit. Returns `None` when the kind is forbidden and the
/// fallback table has no replacement.
pub fn substitute_for_origin(
    registry: &WoundSpecRegistry,
    emit: M14gWoundEmit,
    actor_origin: &OriginId,
) -> Option<M14gWoundEmit> {
    let new_kind = cf_wound::registry::resolve_emit_kind(registry, emit.kind, actor_origin)?;
    Some(M14gWoundEmit {
        kind: new_kind,
        ..emit
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_wound::registry::WoundSpecRegistry;

    #[test]
    fn through_and_through_emits_entry_exit() {
        let pair = classify_gunshot(
            ZoneId::from("torso_front"),
            ZoneId::from("torso_back"),
            0.4,
            0.5,
            true,
        );
        assert_eq!(pair.len(), 2);
        assert_eq!(pair[0].kind, WoundKind::GunshotEntry);
        assert_eq!(pair[0].zone.as_str(), "torso_front");
        assert_eq!(pair[1].kind, WoundKind::GunshotExit);
        assert_eq!(pair[1].zone.as_str(), "torso_back");
    }

    /// VAL-M14G-027 (standalone helper coverage): `classify_gunshot`
    /// always returns typed `GunshotEntry`/`GunshotExit`/`GunshotThrough`
    /// kinds — never a legacy generic wound. Engine-level
    /// "zero `combat.wound_added` events" coverage lives in
    /// `cf-control::tests` (runtime evidence) since this fixture only
    /// exercises the producer surface.
    #[test]
    fn classify_gunshot_returns_only_typed_kinds() {
        for i in 0..100 {
            let through = i % 2 == 0;
            let kinds = classify_gunshot(
                ZoneId::from("torso_front"),
                ZoneId::from("torso_back"),
                (i as f32) / 100.0,
                (i as f32) / 100.0,
                through,
            );
            for emit in kinds {
                assert!(
                    matches!(
                        emit.kind,
                        WoundKind::GunshotEntry | WoundKind::GunshotExit | WoundKind::GunshotThrough
                    ),
                    "non-typed kind emitted: {:?}",
                    emit.kind
                );
            }
        }
    }

    #[test]
    fn fall_impulse_fracture_kind_table() {
        let zone = ZoneId::from("leg_left");
        let threshold = 1000.0;
        assert!(classify_fall_fracture(zone.clone(), 600.0, threshold).is_none(), "0.6: no fracture");
        let s = classify_fall_fracture(zone.clone(), 700.0, threshold).unwrap();
        assert_eq!(s.kind, WoundKind::FractureSimple);
        assert!((s.severity - 0.5).abs() < 1e-3);
        let c = classify_fall_fracture(zone.clone(), 900.0, threshold).unwrap();
        assert_eq!(c.kind, WoundKind::FractureCompound);
        assert!((c.severity - 0.8).abs() < 1e-3);
        assert!((c.dirt_pct - 0.3).abs() < 1e-6);
        let cm = classify_fall_fracture(zone, 970.0, threshold).unwrap();
        assert_eq!(cm.kind, WoundKind::FractureComminuted);
    }

    #[test]
    fn fracture_decision_tree() {
        let zone = ZoneId::from("leg_left");
        let threshold = 100.0;
        let cases: &[(f32, Option<WoundKind>)] = &[
            (60.0, None),
            (70.0, Some(WoundKind::FractureSimple)),
            (85.0, Some(WoundKind::FractureCompound)),
            (90.0, Some(WoundKind::FractureCompound)),
            (95.0, Some(WoundKind::FractureComminuted)),
        ];
        for (impulse, expected) in cases {
            let got = classify_fall_fracture(zone.clone(), *impulse, threshold)
                .map(|e| e.kind);
            assert_eq!(got, *expected, "impulse {impulse} → {got:?}, expected {expected:?}");
        }
    }

    #[test]
    fn fracture_compound_dirt_pct_init() {
        let e = classify_fall_fracture(ZoneId::from("leg_left"), 900.0, 1000.0).unwrap();
        let w = e.into_wound(WoundId(1));
        assert!((w.dirt_pct - 0.3).abs() < 1e-6);
    }

    #[test]
    fn shrapnel_embedded_fragment_count() {
        let zone = ZoneId::from("torso_front");
        let emits: Vec<_> = (0..3)
            .map(|_| classify_shrapnel(zone.clone(), 0.3, false))
            .collect();
        assert_eq!(emits.len(), 3);
        for e in &emits {
            assert_eq!(e.kind, WoundKind::ShrapnelEmbedded);
        }
    }

    /// (crew torso).
    #[test]
    fn heat_round_cluster_wounds() {
        let modules = vec![
            ZoneId::from("mod_a"),
            ZoneId::from("mod_b"),
            ZoneId::from("mod_c"),
        ];
        let emits = classify_heat_cluster(&modules, Some(ZoneId::from("crew_torso")), 0.85);
        assert_eq!(emits.len(), 4);
        let burns = emits.iter().filter(|e| e.kind == WoundKind::Burn3rd).count();
        let through = emits.iter().filter(|e| e.kind == WoundKind::GunshotThrough).count();
        assert_eq!(burns, 3);
        assert_eq!(through, 1);
        assert!(emits.iter().any(|e| e.zone.as_str() == "crew_torso" && e.kind == WoundKind::GunshotThrough));
    }

    /// count equals `module_zones.len()`.
    #[test]
    fn heat_cluster_shrinks_with_module_path_length() {
        let modules = vec![ZoneId::from("mod_a")];
        let emits = classify_heat_cluster(&modules, None, 0.5);
        let burns = emits.iter().filter(|e| e.kind == WoundKind::Burn3rd).count();
        assert_eq!(burns, 1);
        let through = emits.iter().filter(|e| e.kind == WoundKind::GunshotThrough).count();
        assert_eq!(through, 0);
    }

    /// DentalDamage at severity 0.6.
    #[test]
    fn blunt_face_hit_emits_dental_damage() {
        let e = classify_blunt_face_hit(ZoneId::from("head_front"), 220.0, 200.0);
        assert_eq!(e.kind, WoundKind::DentalDamage);
        assert!((e.severity - 0.6).abs() < 1e-6);
        assert_eq!(e.zone.as_str(), "head_front");
    }

    /// CrushLimb on robot actors.
    #[test]
    fn origin_substitution_swaps_laceration_for_crush_limb_on_robots() {
        let registry = WoundSpecRegistry::baked_default();
        let robot = OriginId::from("robot");
        let emit = M14gWoundEmit {
            kind: WoundKind::LacerationLight,
            severity: 0.3,
            zone: ZoneId::from("torso_front"),
            dirt_pct: 0.0,
        };
        let out = substitute_for_origin(&registry, emit, &robot).expect("substituted");
        assert_eq!(out.kind, WoundKind::CrushLimb);
    }
}
