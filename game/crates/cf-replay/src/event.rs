//! DR-002 v1 event envelope (locked at v0.1).

use serde::{Deserialize, Serialize};

/// One DR-002 v1 event. M4 envelope is locked at v0.1; the optional fields
/// (`parent_event_id`, `actor_id`, `source_id`, `team`, `pos`, `bbox`,
/// `dropped_count`, `cosmetic`, `asset_ref`) are envelope-level so consumers
/// (cause-chain walker, replay viewer, M4A asset ledger) can index by them
/// without reaching into `payload`. Additive envelope extensions require a
/// schema bump (locked at v0.1 at M4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema_version: String,
    pub run_id: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub event_id: String,
    pub category: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent_event_id: Option<String>,
    /// the event is about / caused by a specific actor, set this so
    /// downstream consumers can filter without parsing the payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actor_id: Option<u64>,
    /// Distinct from `actor_id` (the affected actor) — e.g. shooter vs
    /// victim, or carrier vs item.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<u64>,
    /// "neutral" / faction name) for fast filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub team: Option<String>,
    /// event happened. Surface-level convenience for spatial filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pos: Option<[f32; 2]>,
    /// max_y] for events that span an area (terrain carve, blast, hazard
    /// cell). Surface-level convenience for spatial filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bbox: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dropped_count: Option<u64>,
    /// M4 § DR-052 cosmetic vs gameplay split. When `Some(true)`, this event
    /// is a cosmetic surface (particle, debris spawn, UI banner, etc.) and
    /// MUST be excluded from `determinism.sim_checksum` hashing AND
    /// preferentially dropped first under recorder backpressure. The
    /// underlying STATE change (terrain integrity, hazard intensity,
    /// affliction severity) is hashed through the actor/world state — the
    /// cosmetic event only DESCRIBES the change. When `None` or `Some(false)`
    /// the event is a gameplay surface.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cosmetic: Option<bool>,
    /// entry. Set on events that reference an AI-generated asset (capture
    /// grid screenshot, audio playback, mod-supplied content). M4A's
    /// `cf-mod ledger verify` cross-references this against the ledger's
    /// `AssetId` registry. The asset_ref value is a string-encoded
    /// `AssetId` (blake3 hex prefix).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub asset_ref: Option<String>,
    /// hash linking this event to the immediately-prior event in
    /// tournament-mode bundles. The chain is verified by `cf-mod ledger
    /// verify --bundle <path>` + `cf-tools-replay-viewer validate`. Dev
    /// runs leave this `None` so existing M0/M3A bundles continue to
    /// parse unchanged.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prev_event_hash: Option<String>,
    /// hash for THIS event (binding `prev_event_hash` + canonical payload).
    /// Stored alongside `prev_event_hash` so the verifier can pinpoint the
    /// tamper to the exact event_id rather than the next one. `None` when
    /// the recorder is not in chain mode.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub chained_hash_hex: Option<String>,
}
