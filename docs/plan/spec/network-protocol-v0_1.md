# cf-net wire protocol v0.1

Authored 2026-05-15 by m8a-impl. **Locked** at M8A. Additive-only
extensions to `NetPayload` variants only.

## Wire frame

```rust
NetFrame {
    version: u16,        // protocol_version, LOCKED to 1 at v0.1
    seq: u32,            // monotonic per-sender frame counter
    timestamp_ms: u64,   // sender's monotonic clock (telemetry only)
    payload: NetPayload, // tagged union
}
```

Schema at `game/crates/cf-net/schemas/v0_1/net_frame.schema.json`.

## NetPayload variants (locked at v0.1)

- `Handshake { client_version, capabilities, session_token?, content_hash }`
- `HandshakeAck(Accept { session_id, server_version, content_hash } | Reject { reason })`
- `InputCommand { tick, intent_event_id, control_command_json }`
- `SnapshotDelta { from_tick, to_tick, delta_bytes }`
- `EventBatch { tick, events_jsonl[] }` (reliable stream)
- `ChecksumProbe { tick, checksum_hex }` (every 64-tick cadence)
- `Ping { send_ms }`, `Pong { send_ms, recv_ms }`
- `Disconnect { reason }`

## Version negotiation

- Server compares the client's `Handshake.client_version` against
  `PROTOCOL_VERSION`. Mismatch → `HandshakeAck::Reject { reason:
  "protocol_version_mismatch" }` BEFORE any sim frame.
- A v0.2 client connecting to a v0.1 server is rejected at handshake.

## Content manifest hash on join

Per `cortext_command_vault/systems/networking-backend-frontend.md` §
Backend Service Slice A: server rejects clients whose content manifest
checksum diverges from the server's. The `content_hash` field on
Handshake is the canonical comparison key.

## Rollback semantics

- 6-frame rollback budget at p99 ≤ 8 ms total resimulation
  (`cf-net::rollback::ROLLBACK_BUDGET_FRAMES` /
  `ROLLBACK_RESIM_BUDGET_MS`).
- Rollback re-uses the same deterministic sim core (no separate code
  path); resimulate forward after rolling back to the first divergent
  frame.
- Rollbacks larger than the budget trigger a full snapshot resync per
  Gaffer.

## Snapshot cadence

- Keyframe every 64 ticks (`cf-net::protocol::SNAPSHOT_CADENCE_TICKS`).
- Delta every 1 tick.
- Configurable at runtime via cfctl `srv.set_cvar
  net.snapshot.cadence_ticks <n>`.

## Transport

- **QUIC via `quinn`** for primary transport (reliable streams for the
  canonical event stream; unreliable datagrams for inputs + snapshot
  deltas). Wired at M9+.
- **WebSocket fallback** for browser spectator clients (M11+).
- Per-frame max size: 1450 bytes (Ethernet MTU minus IP+UDP+QUIC).

## Lobby flow

`cf-net/lobby` minimum slice (M8A scaffold): server discovery returns
the canonical server row shape per the vault — `version` + `protocol`
+ `content_hash` + `region` + `map` + `ruleset` + `humans` + `bots` +
`trust_tier`.

## Deep link

`corefall://join?server=<addr>&token=<one-time>&content_hash=<sha>`.
Locked at M8A; content-aware; version-locked; no plain passwords in
URIs.

## Cvars via cfctl

The cfctl additive surface (no `SCHEMA_VERSION` bump):

| Method | Purpose |
|---|---|
| `srv.set_cvar { key, value }` | Server runtime config (cadence, max_clients, etc.) |
| `srv.get_cvar { key }` | Read current cvar |
| `srv.list_cvars` | Inventory |
| `srv.kick_client { client_id }` | Disconnect a specific client |
| `srv.run_bundle_path` | Returns server-side run bundle path |
