//! M9C minefield kernel — authored minefield templates +
//! deploy-by-origin placement under `act.player.deploy_minefield_template`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::{FortificationFaction, FortificationId};
use crate::minefield_types::{Mine, MineArmedEvent, MineKind};

/// One placement instruction in a `*.minefield.ron` template — a
/// `kind` + tile-relative offset from the template origin + per-mine
/// metadata. The 4 launch templates land under
/// `content/mine_fields/<id>.minefield.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinefieldPlacement {
    pub kind: MineKind,
    pub offset_tiles: (i32, i32),
    /// HE yield override (defaults to kind baseline). IED chain
    /// templates author per-mine yield in the 200..400 J range.
    #[serde(default)]
    pub yield_joules: Option<u32>,
    /// Blast radius override (defaults to kind baseline).
    #[serde(default)]
    pub blast_radius_tiles: Option<u32>,
    /// Tripwire endpoints relative to the template origin. Only the
    /// `tripwire_mine` kind uses this field.
    #[serde(default)]
    pub tripwire_endpoints: Option<((i32, i32), (i32, i32))>,
    /// IED chain wired-link index list (indices into the template's
    /// `placements` vec). Only the `ied_chain` kind uses this field.
    /// The loader resolves the indices to actual `FortificationId`s
    /// once the engine allocates ids.
    #[serde(default)]
    pub wired_links: Vec<usize>,
}

/// On-disk spec for one of the 4 minefield templates under
/// `content/mine_fields/<id>.minefield.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinefieldTemplateSpec {
    pub id: String,
    pub display_name: String,
    pub placements: Vec<MinefieldPlacement>,
}

impl MinefieldTemplateSpec {
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<MinefieldTemplateSpec>(text)
    }
}

/// Outcome of a `act.player.deploy_minefield_template` call. The
/// engine consumes `mines` to insert into the world + `armed_events`
/// to fan out to the recorder + `inventory_consumed` to decrement
/// pool slots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinefieldDeployOutcome {
    pub mines: Vec<Mine>,
    pub armed_events: Vec<MineArmedEvent>,
    pub inventory_consumed: BTreeMap<MineKind, u32>,
}

/// Compute the per-kind inventory cost of placing this template.
#[must_use]
pub fn template_inventory_cost(template: &MinefieldTemplateSpec) -> BTreeMap<MineKind, u32> {
    let mut costs: BTreeMap<MineKind, u32> = BTreeMap::new();
    for p in &template.placements {
        *costs.entry(p.kind).or_insert(0) += 1;
    }
    costs
}

/// Apply a template at `origin` and return the placed mines + the
/// armed events. `next_id` is incremented for each placed mine; the
/// caller seeds it from the world's id allocator.
#[must_use]
pub fn deploy_template(
    template: &MinefieldTemplateSpec,
    origin: (i32, i32),
    owner: FortificationFaction,
    mut next_id: u32,
    tick_index: u64,
) -> MinefieldDeployOutcome {
    let mut mines = Vec::with_capacity(template.placements.len());
    let mut armed_events = Vec::with_capacity(template.placements.len());
    let mut placement_to_id: Vec<FortificationId> = Vec::with_capacity(template.placements.len());
    // First pass: allocate ids + push base mines (without wire links
    // resolved yet).
    for p in &template.placements {
        let id = FortificationId(next_id);
        next_id = next_id.wrapping_add(1);
        let pos = (
            origin.0 + p.offset_tiles.0,
            origin.1 + p.offset_tiles.1,
        );
        let mut mine = Mine::new(id, p.kind, pos, owner);
        if let Some(y) = p.yield_joules {
            mine.yield_joules = y;
        }
        if let Some(r) = p.blast_radius_tiles {
            mine.blast_radius_tiles = r;
        }
        if let Some((a, b)) = p.tripwire_endpoints {
            mine.tripwire_endpoints = Some((
                (origin.0 + a.0, origin.1 + a.1),
                (origin.0 + b.0, origin.1 + b.1),
            ));
        }
        placement_to_id.push(id);
        mines.push(mine);
        armed_events.push(MineArmedEvent {
            mine_id: id,
            mine_kind: p.kind,
            pos,
            tick_index,
        });
    }
    // Second pass: resolve wired-link index list to per-mine id list.
    for (idx, p) in template.placements.iter().enumerate() {
        let mine = &mut mines[idx];
        for link in &p.wired_links {
            if let Some(&resolved) = placement_to_id.get(*link) {
                if resolved != mine.id {
                    mine.wired_links.push(resolved);
                }
            }
        }
        mine.wired_links.sort();
        mine.wired_links.dedup();
    }
    let inventory_consumed = template_inventory_cost(template);
    MinefieldDeployOutcome {
        mines,
        armed_events,
        inventory_consumed,
    }
}
