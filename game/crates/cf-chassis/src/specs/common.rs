use crate::{ArmorLayer, ArmorLayerKind, BodyZone, ZoneState};

pub(crate) fn make_zone(
    zone: BodyZone,
    external_hp: f32,
    external_hardness: f32,
    internal_hp: f32,
    internal_hardness: f32,
    core_hp: f32,
    wound_hp: f32,
) -> ZoneState {
    let layers = vec![
        ArmorLayer::new(ArmorLayerKind::External, external_hp, external_hardness),
        ArmorLayer::new(ArmorLayerKind::Internal, internal_hp, internal_hardness),
        ArmorLayer::new(ArmorLayerKind::Core, core_hp, 0.0),
    ];
    ZoneState::new(zone, layers, wound_hp)
}
