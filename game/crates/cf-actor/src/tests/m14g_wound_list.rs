//! VAL-M14G-007 + VAL-M14G-048 — ActorWoundList field on cf-actor::ActorState
//! is keyed by ZoneId, supports stable iteration, allocates unique wound ids,
//! and round-trips through the actor checksum without losing state.

use cf_wound::registry::ZoneId;
use cf_wound::{ActorWoundList, Wound, WoundId, WoundKind};

use crate::{ActorId, ActorState, Inventory, Vec2};

fn make_actor() -> ActorState {
    let inv = Inventory::with_rifle("rifle_m1_default");
    ActorState::player(ActorId(7), "blue", Vec2::ZERO, 100.0, inv)
}

/// VAL-M14G-007: ActorWoundList stores wounds keyed by zone with stable
/// (BTreeMap-ordered) iteration.
#[test]
fn m14g_actor_wound_list_per_zone() {
    let mut actor = make_actor();
    actor.m14g_wound_list.push(
        ZoneId::from("torso_front"),
        Wound::new(WoundId(0), WoundKind::GunshotEntry, 0.4, ZoneId::from("torso_front")),
    );
    actor.m14g_wound_list.push(
        ZoneId::from("torso_back"),
        Wound::new(WoundId(0), WoundKind::GunshotExit, 0.4, ZoneId::from("torso_back")),
    );
    actor.m14g_wound_list.push(
        ZoneId::from("leg_left"),
        Wound::new(WoundId(0), WoundKind::Puncture, 0.4, ZoneId::from("leg_left")),
    );

    let zones: Vec<&str> = actor.m14g_wound_list.iter().map(|(z, _)| z.as_str()).collect();
    assert_eq!(zones, vec!["leg_left", "torso_back", "torso_front"]);
    assert_eq!(actor.m14g_wound_list.total_count(), 3);
    assert_eq!(actor.m14g_wound_list.zone_count(&ZoneId::from("torso_front")), 1);
}

/// VAL-M14G-048: monotone wound ids across mixed-zone emits on one actor.
#[test]
fn m14g_wound_id_unique_per_actor() {
    let mut actor = make_actor();
    let zones = [
        ZoneId::from("torso_front"),
        ZoneId::from("arm_left"),
        ZoneId::from("leg_right"),
    ];
    for i in 0..50 {
        let zone = zones[i % zones.len()].clone();
        actor.m14g_wound_list.push(
            zone.clone(),
            Wound::new(WoundId(0), WoundKind::LacerationLight, 0.1, zone),
        );
    }
    let mut ids: std::collections::HashSet<WoundId> = std::collections::HashSet::new();
    for (_, ws) in actor.m14g_wound_list.iter() {
        for w in ws {
            ids.insert(w.id);
        }
    }
    assert_eq!(ids.len(), 50);
}

/// VAL-CROSS-029 surface — the checksum_bytes envelope includes the wound list
/// when populated.
#[test]
fn m14g_checksum_bytes_includes_wound_list_when_populated() {
    let mut a = make_actor();
    let baseline = a.checksum_bytes();
    a.m14g_wound_list.push(
        ZoneId::from("torso_front"),
        Wound::new(WoundId(0), WoundKind::GunshotEntry, 0.5, ZoneId::from("torso_front")),
    );
    let with_wound = a.checksum_bytes();
    assert_ne!(baseline, with_wound, "wound list must alter the checksum");
}

/// Empty wound list does NOT change checksum_bytes vs a baseline (append-only
/// invariant).
#[test]
fn m14g_empty_wound_list_does_not_change_checksum() {
    let mut a = make_actor();
    let baseline = a.checksum_bytes();
    // Re-construct the wound list with no entries — checksum should be
    // identical because the field is gated by non-empty.
    a.m14g_wound_list = ActorWoundList::new();
    assert_eq!(baseline, a.checksum_bytes());
}
