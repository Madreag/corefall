//! M7-A: per-bot `BotMemory` (16 KB).
//!
//! Each bot maintains a private world model the 5-layer thinking stack reads:
//!
//! - **Perception grid** — 64×64 cells; per-cell `seen_recently` tick +
//!   threat_level + cover_quality + friendly_marked flag.
//! - **Threat memory** — bounded vec of per-enemy records (last-seen pos,
//!   last-known weapon, suspected role, age tick). Decays after 60-300 sec.
//! - **Ally memory** — bounded vec of per-ally records.
//! - **Recent events ring** — last 64 witnessed events (HTN re-planner reads
//!   this to detect goal failure).
//!
//! Memory ages out deterministically via tick-counter comparison. Replay
//! determinism preserved — no wall-clock reads.
//!
//! M8A locks the storage shape; M7-A pre-declares so the thinking stack
//! has a stable surface to read.

use serde::{Deserialize, Serialize};

/// Perception grid size (locked at 64×64).
pub const PERCEPTION_GRID_DIM: usize = 64;
pub const PERCEPTION_GRID_CELLS: usize = PERCEPTION_GRID_DIM * PERCEPTION_GRID_DIM;

/// Max threat-memory entries per bot.
pub const THREAT_MEMORY_CAPACITY: usize = 16;
/// Max ally-memory entries per bot.
pub const ALLY_MEMORY_CAPACITY: usize = 8;
/// Recent-events ring depth.
pub const RECENT_EVENTS_RING_DEPTH: usize = 64;

/// **M7-A**: per-cell summary in the perception grid. Compact (8 bytes) so
/// the 64×64 grid fits in 32 KB even with redundancy. Threat / cover /
/// friendly fields are `0..=255` quantized for tight packing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PerceptionCell {
    /// Tick number of the most recent observation that touched this cell.
    /// `0` = never observed. Used to age out stale cells.
    pub last_seen_tick: u32,
    /// Threat level (0..=255). Higher = more recent enemy contact / fire.
    pub threat_level: u8,
    /// Cover quality (0..=255). Latched during cover-seek pass.
    pub cover_quality: u8,
    /// True if a friendly currently occupies this cell.
    pub friendly_marked: bool,
    /// Reserved for M7-B / M9 hazard flag (chemistry / atmos heat); zeroed at
    /// M7-A.
    pub flags: u8,
}

/// **M7-A**: one threat memory record (per-enemy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatMemoryRecord {
    pub actor_id: u64,
    pub last_seen_position: [f32; 2],
    pub last_known_weapon: ThreatWeaponClass,
    pub suspected_role: u8,
    /// Tick when this record was last refreshed. Aged out when
    /// `current_tick - last_refresh_tick > decay_ticks`.
    pub last_refresh_tick: u64,
    /// Tick budget before decay (per spec: 60-300 seconds; longer for
    /// player-tagged threats).
    pub decay_ticks: u64,
    /// True if the player MMB-tagged this enemy (extends decay to 600s
    /// per spec § Live Orders).
    pub player_tagged: bool,
}

/// **M7-A**: coarse weapon class the bot remembers about a threat. M13+
/// chassis adds tank/gunship/heavy-weapon variants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThreatWeaponClass {
    #[default]
    Unknown,
    Rifle,
    Smg,
    Shotgun,
    Sniper,
    Pistol,
    Grenade,
    Melee,
    Heavy,
}

impl ThreatWeaponClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ThreatWeaponClass::Unknown => "unknown",
            ThreatWeaponClass::Rifle => "rifle",
            ThreatWeaponClass::Smg => "smg",
            ThreatWeaponClass::Shotgun => "shotgun",
            ThreatWeaponClass::Sniper => "sniper",
            ThreatWeaponClass::Pistol => "pistol",
            ThreatWeaponClass::Grenade => "grenade",
            ThreatWeaponClass::Melee => "melee",
            ThreatWeaponClass::Heavy => "heavy",
        }
    }
}

/// **M7-A**: one ally memory record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllyMemoryRecord {
    pub actor_id: u64,
    pub last_known_position: [f32; 2],
    pub last_known_hp: f32,
    pub last_known_hp_max: f32,
    pub last_known_cover_quality: u8,
    pub last_known_order: u8,
    pub last_refresh_tick: u64,
}

/// **M7-A**: one entry in the recent-events ring. Compact for cache
/// efficiency; payload is a coarse classification + tick. The HTN re-planner
/// matches on `kind` + delta-tick to detect goal failure / opportunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentEvent {
    pub tick: u64,
    pub kind: RecentEventKind,
    /// Source-actor id (0 if not applicable).
    pub source_id: u64,
    /// Target-actor id (0 if not applicable).
    pub target_id: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RecentEventKind {
    #[default]
    None,
    SawEnemy,
    SawAlly,
    HeardShot,
    AllyDamaged,
    AllyDowned,
    EnemyKilled,
    OrderReceived,
    TookDamage,
    PathBlocked,
    ChassisModuleDegraded,
    TerrainBreached,
}

impl RecentEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RecentEventKind::None => "none",
            RecentEventKind::SawEnemy => "saw_enemy",
            RecentEventKind::SawAlly => "saw_ally",
            RecentEventKind::HeardShot => "heard_shot",
            RecentEventKind::AllyDamaged => "ally_damaged",
            RecentEventKind::AllyDowned => "ally_downed",
            RecentEventKind::EnemyKilled => "enemy_killed",
            RecentEventKind::OrderReceived => "order_received",
            RecentEventKind::TookDamage => "took_damage",
            RecentEventKind::PathBlocked => "path_blocked",
            RecentEventKind::ChassisModuleDegraded => "chassis_module_degraded",
            RecentEventKind::TerrainBreached => "terrain_breached",
        }
    }
}

/// **M7-A**: full per-bot memory state.
///
/// **Storage discipline**: the perception grid is stored as a `Vec` sized to
/// exactly `PERCEPTION_GRID_CELLS` (= 4096). Length is enforced by `new` and
/// every constructor; mutators that resize are not exposed. The grid uses
/// `Vec` instead of a fixed-size array because serde's derive macros only
/// support arrays up to 32 elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotMemory {
    pub perception_grid: Vec<PerceptionCell>,
    pub threat_memory: Vec<ThreatMemoryRecord>,
    pub ally_memory: Vec<AllyMemoryRecord>,
    pub recent_events: RecentEventRing,
    /// Tick of the last full-grid age pass. The decay scan uses this to
    /// keep the per-tick cost constant.
    pub last_age_pass_tick: u64,
}

impl BotMemory {
    pub fn new() -> Self {
        Self {
            perception_grid: vec![PerceptionCell::default(); PERCEPTION_GRID_CELLS],
            threat_memory: Vec::with_capacity(THREAT_MEMORY_CAPACITY),
            ally_memory: Vec::with_capacity(ALLY_MEMORY_CAPACITY),
            recent_events: RecentEventRing::new(),
            last_age_pass_tick: 0,
        }
    }

    /// Restore the grid invariants after a snapshot deserialize: ensures
    /// `perception_grid.len() == PERCEPTION_GRID_CELLS` exactly.
    pub fn normalize_grid(&mut self) {
        if self.perception_grid.len() < PERCEPTION_GRID_CELLS {
            self.perception_grid
                .resize(PERCEPTION_GRID_CELLS, PerceptionCell::default());
        } else if self.perception_grid.len() > PERCEPTION_GRID_CELLS {
            self.perception_grid.truncate(PERCEPTION_GRID_CELLS);
        }
    }

    /// Look up a perception cell by (cell_x, cell_y) — both in `0..GRID_DIM`.
    pub fn cell(&self, cx: usize, cy: usize) -> Option<&PerceptionCell> {
        if cx >= PERCEPTION_GRID_DIM || cy >= PERCEPTION_GRID_DIM {
            return None;
        }
        self.perception_grid.get(cy * PERCEPTION_GRID_DIM + cx)
    }

    pub fn cell_mut(&mut self, cx: usize, cy: usize) -> Option<&mut PerceptionCell> {
        if cx >= PERCEPTION_GRID_DIM || cy >= PERCEPTION_GRID_DIM {
            return None;
        }
        self.perception_grid.get_mut(cy * PERCEPTION_GRID_DIM + cx)
    }

    /// Refresh-or-insert a threat record. Caller is responsible for trimming
    /// to capacity; this returns true when an existing record was updated.
    pub fn observe_threat(&mut self, actor_id: u64, position: [f32; 2], weapon: ThreatWeaponClass, tick: u64) -> bool {
        for r in &mut self.threat_memory {
            if r.actor_id == actor_id {
                r.last_seen_position = position;
                r.last_known_weapon = weapon;
                r.last_refresh_tick = tick;
                return true;
            }
        }
        if self.threat_memory.len() < THREAT_MEMORY_CAPACITY {
            self.threat_memory.push(ThreatMemoryRecord {
                actor_id,
                last_seen_position: position,
                last_known_weapon: weapon,
                suspected_role: 0,
                last_refresh_tick: tick,
                decay_ticks: 300 * 60,
                player_tagged: false,
            });
        } else {
            // Replace the oldest entry.
            let idx = self
                .threat_memory
                .iter()
                .enumerate()
                .min_by_key(|(_, r)| r.last_refresh_tick)
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.threat_memory[idx] = ThreatMemoryRecord {
                actor_id,
                last_seen_position: position,
                last_known_weapon: weapon,
                suspected_role: 0,
                last_refresh_tick: tick,
                decay_ticks: 300 * 60,
                player_tagged: false,
            };
        }
        false
    }

    /// Refresh-or-insert an ally record.
    pub fn observe_ally(
        &mut self,
        actor_id: u64,
        position: [f32; 2],
        hp: f32,
        hp_max: f32,
        cover_quality: u8,
        order: u8,
        tick: u64,
    ) {
        for r in &mut self.ally_memory {
            if r.actor_id == actor_id {
                r.last_known_position = position;
                r.last_known_hp = hp;
                r.last_known_hp_max = hp_max;
                r.last_known_cover_quality = cover_quality;
                r.last_known_order = order;
                r.last_refresh_tick = tick;
                return;
            }
        }
        if self.ally_memory.len() < ALLY_MEMORY_CAPACITY {
            self.ally_memory.push(AllyMemoryRecord {
                actor_id,
                last_known_position: position,
                last_known_hp: hp,
                last_known_hp_max: hp_max,
                last_known_cover_quality: cover_quality,
                last_known_order: order,
                last_refresh_tick: tick,
            });
        }
    }

    /// Push an event into the ring (oldest displaced).
    pub fn push_event(&mut self, event: RecentEvent) {
        self.recent_events.push(event);
    }

    /// Drop any threat record whose `last_refresh_tick + decay_ticks < tick`.
    /// Per-tick cost is `O(THREAT_MEMORY_CAPACITY)` (= 16) so cheap.
    pub fn age_threats(&mut self, tick: u64) {
        self.threat_memory.retain(|r| {
            if r.player_tagged {
                let extended = r.decay_ticks.saturating_mul(2);
                tick.saturating_sub(r.last_refresh_tick) <= extended
            } else {
                tick.saturating_sub(r.last_refresh_tick) <= r.decay_ticks
            }
        });
    }

    /// Bytes for the determinism checksum. Layout is append-only.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&(self.threat_memory.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.ally_memory.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.last_age_pass_tick.to_le_bytes());
        for r in &self.threat_memory {
            out.extend_from_slice(&r.actor_id.to_le_bytes());
            out.extend_from_slice(&r.last_refresh_tick.to_le_bytes());
            out.push(r.last_known_weapon as u8);
        }
        for r in &self.ally_memory {
            out.extend_from_slice(&r.actor_id.to_le_bytes());
            out.extend_from_slice(&r.last_refresh_tick.to_le_bytes());
        }
        out.extend_from_slice(&(self.recent_events.len() as u32).to_le_bytes());
        out
    }
}

impl Default for BotMemory {
    fn default() -> Self {
        Self::new()
    }
}

/// **M7-A**: fixed-depth ring buffer of recent witnessed events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentEventRing {
    entries: Vec<RecentEvent>,
    /// Index where the next `push` writes (mod `RECENT_EVENTS_RING_DEPTH`).
    head: usize,
    /// True once the ring has wrapped at least once.
    wrapped: bool,
}

impl RecentEventRing {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(RECENT_EVENTS_RING_DEPTH),
            head: 0,
            wrapped: false,
        }
    }

    pub fn push(&mut self, event: RecentEvent) {
        if self.entries.len() < RECENT_EVENTS_RING_DEPTH {
            self.entries.push(event);
            self.head = self.entries.len() % RECENT_EVENTS_RING_DEPTH;
        } else {
            self.entries[self.head] = event;
            self.head = (self.head + 1) % RECENT_EVENTS_RING_DEPTH;
            self.wrapped = true;
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate from oldest to newest (deterministic order).
    pub fn iter_chronological(&self) -> impl Iterator<Item = &RecentEvent> + '_ {
        let len = self.entries.len();
        let start = if self.wrapped { self.head } else { 0 };
        (0..len).map(move |i| &self.entries[(start + i) % len])
    }
}

impl Default for RecentEventRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_addressing_round_trip() {
        let mut m = BotMemory::new();
        m.cell_mut(10, 20).unwrap().threat_level = 200;
        assert_eq!(m.cell(10, 20).unwrap().threat_level, 200);
        assert!(m.cell(64, 0).is_none());
    }

    #[test]
    fn threat_observation_refreshes_existing() {
        let mut m = BotMemory::new();
        let updated = m.observe_threat(42, [100.0, 50.0], ThreatWeaponClass::Rifle, 10);
        assert!(!updated);
        let updated = m.observe_threat(42, [110.0, 60.0], ThreatWeaponClass::Rifle, 25);
        assert!(updated);
        assert_eq!(m.threat_memory.len(), 1);
        assert_eq!(m.threat_memory[0].last_refresh_tick, 25);
    }

    #[test]
    fn threat_capacity_evicts_oldest() {
        let mut m = BotMemory::new();
        for i in 0..(THREAT_MEMORY_CAPACITY as u64 + 4) {
            m.observe_threat(i, [0.0, 0.0], ThreatWeaponClass::Rifle, i);
        }
        assert_eq!(m.threat_memory.len(), THREAT_MEMORY_CAPACITY);
    }

    #[test]
    fn ring_buffer_wraps_chronologically() {
        let mut r = RecentEventRing::new();
        for i in 0..(RECENT_EVENTS_RING_DEPTH as u64 + 5) {
            r.push(RecentEvent {
                tick: i,
                kind: RecentEventKind::SawEnemy,
                source_id: i,
                target_id: 0,
            });
        }
        let collected: Vec<u64> = r.iter_chronological().map(|e| e.tick).collect();
        assert_eq!(collected.len(), RECENT_EVENTS_RING_DEPTH);
        for win in collected.windows(2) {
            assert!(win[0] < win[1], "chronological order violated: {win:?}");
        }
    }
}
