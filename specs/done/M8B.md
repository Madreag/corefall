# M8B — QUIC Wire Protocol + Rollback Prediction Model + Packet Loss Recovery + NAT Punch-Through Deep

## Status

`done`

## Intent

M8B locks the deep network protocol layer beneath the M8A scaffold: a semver-versioned QUIC frame format with byte-stable encoding, reliable QUIC streams for the canonical event log + unreliable QUIC datagrams for per-tick input + snapshot deltas, a fully-specified 6-frame rollback prediction algorithm with per-frame input commit ordering, packet-loss compensation via redundant-input encoding + forward error correction on small payloads, NAT punch-through with ICE-lite + STUN discovery + TURN relay fallback, and a deterministic transport-selection policy that distinguishes dedicated-server authority topology from P2P lockstep topology. M8A declared the QUIC primitives + the rollback budget; M8B nails down the on-wire bytes, the rollback algorithm, the loss recovery math, and the NAT traversal flow so that M40 / M41 / M42 / M49 can ship without re-architecting the transport.

## Player-facing behavior

Player does not see new content from M8B — but every networked match a player ever plays after M8B runs over a defined protocol with locked frame bytes, defined loss recovery, and defined NAT traversal. Specifically:

- **Online match join time stays under 6 seconds even behind double-NAT routers.** STUN-discovered candidate pairs are tried in parallel; TURN relay is engaged only when ICE-lite negotiation fails. The status line surfaces `connecting via direct` / `connecting via relay` so the player knows what tier they got.
- **Lossy networks stay smooth.** 5% packet loss on a 100 ms RTT link feels like a 5% slower-reacting world, not a stuttering one — redundant-input encoding means a dropped input datagram does not stall the rollback queue.
- **Rollback corrections are imperceptible.** When a misprediction is detected, the 6-frame resimulation completes under 8 ms p99 on the reference platform; the player sees a smooth correction, not a snap.
- **Dedicated server vs P2P transport is automatic.** Player joining a `cf-server --mode coop_room` instance gets server-authoritative transport with input upload + snapshot download; player joining a 2-player LAN session via `cf-server --mode lan_room` gets host-authoritative lockstep with merged input broadcast. Player does not configure this.
- **Protocol version bumps are forward-compatible within a major.** A v0.1.4 client connects to a v0.1.7 server and gets a clean version-negotiated session with the v0.1.4 feature subset; a v0.2.x client connecting to a v0.1.x server gets a clean `protocol_major_mismatch` error with a download-update prompt, not a confusing connection failure.
- **The replay viewer can read every M8B-recorded session.** Wire frame layout is byte-stable per semver minor; M10 replay can decode any v0.1.x bundle deterministically.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-net` | MODIFY (from M8A) | Promote the M8A wire scaffold to v0.1 frozen + add semver gate + new payload variants for redundant-input + FEC. |
| `cf-net::protocol::frame_v01` | NEW | Locked v0.1 frame layout module with byte-pinning unit tests. |
| `cf-net::protocol::semver` | NEW | Major / minor / patch negotiation handshake; forward-compat behavior. |
| `cf-net::rollback::prediction` | NEW | Per-frame input prediction model; 6-frame ring buffer; commit ordering. |
| `cf-net::rollback::resimulate` | NEW | Resimulation driver; replays the deterministic sim core over the rollback window. |
| `cf-net::loss_recovery` | NEW | Redundant-input encoding (last-N inputs piggybacked on each datagram) + Reed-Solomon FEC on small reliable payloads. |
| `cf-net::nat::ice_lite` | NEW | ICE-lite candidate gathering + STUN discovery. |
| `cf-net::nat::turn_relay` | NEW | TURN relay client fallback when ICE-lite fails. |
| `cf-net::nat::candidate_pair` | NEW | Parallel candidate-pair connectivity check with deterministic tiebreak. |
| `cf-net::transport_select` | NEW | Decides per-session transport mode: `dedicated_server_auth` / `p2p_lockstep` / `host_authoritative`. |
| `cf-server` | MODIFY (from M36) | Server side of the NAT punch flow + protocol semver gate at join. |
| `cf-control` | MODIFY | New cfctl: `observe.net.session_transport`, `observe.net.rollback_stats`, `observe.net.loss_recovery`, `admin.net.force_relay`. |
| `cf-replay` | MODIFY | 5 new event schemas for protocol / rollback / NAT events. |

## Files

Source:

- `game/crates/cf-net/src/protocol/frame_v01.rs` (NEW)
- `game/crates/cf-net/src/protocol/semver.rs` (NEW)
- `game/crates/cf-net/src/protocol/byte_pinning_tests.rs` (NEW)
- `game/crates/cf-net/src/rollback/prediction.rs` (NEW)
- `game/crates/cf-net/src/rollback/resimulate.rs` (NEW)
- `game/crates/cf-net/src/rollback/ring_buffer.rs` (NEW)
- `game/crates/cf-net/src/loss_recovery/redundant_input.rs` (NEW)
- `game/crates/cf-net/src/loss_recovery/fec.rs` (NEW)
- `game/crates/cf-net/src/nat/ice_lite.rs` (NEW)
- `game/crates/cf-net/src/nat/stun_client.rs` (NEW)
- `game/crates/cf-net/src/nat/turn_relay.rs` (NEW)
- `game/crates/cf-net/src/nat/candidate_pair.rs` (NEW)
- `game/crates/cf-net/src/transport_select.rs` (NEW)
- `game/crates/cf-net/src/lib.rs` (MODIFY: wire new modules, expose `PROTOCOL_SEMVER`)
- `game/crates/cf-server/src/m8b_nat_punch.rs` (NEW)
- `game/crates/cf-control/src/m8b_net_admin.rs` (NEW)

Schemas:

- `game/crates/cf-replay/schemas/event/net_protocol_negotiated.json` (NEW)
- `game/crates/cf-replay/schemas/event/net_rollback_window.json` (NEW)
- `game/crates/cf-replay/schemas/event/net_input_resent_redundant.json` (NEW)
- `game/crates/cf-replay/schemas/event/net_fec_recovered.json` (NEW)
- `game/crates/cf-replay/schemas/event/net_nat_traversal_outcome.json` (NEW)

Tests:

- `game/crates/cf-net/tests/frame_v01_byte_pin.rs` (NEW: fixture-vector encoding regression)
- `game/crates/cf-net/tests/semver_negotiation.rs` (NEW)
- `game/crates/cf-net/tests/rollback_window_p99.rs` (NEW: 6-frame budget bench)
- `game/crates/cf-net/tests/loss_recovery_5pct.rs` (NEW)
- `game/crates/cf-net/tests/nat_punch_simulator.rs` (NEW: in-process NAT emulator)
- `game/crates/cf-net/tests/transport_select_matrix.rs` (NEW)

Content:

- `game/content/net/protocol/frame_v01_fixtures.json` (NEW: byte-pinned reference vectors for every NetPayload variant)

CI:

- `game/tools/ci/m8b_protocol_byte_pin.sh` (NEW: fail on any byte-layout drift)
- `game/tools/ci/m8b_rollback_p99.sh` (NEW: fail if p99 resim > 8 ms on reference platform)

## Acceptance criteria

```gherkin
Scenario: QUIC frame v0.1 layout is byte-pinned across releases
  Given the locked v0.1 fixture vector "input_command_v01_minimal.bin"
  When the encoder serializes the same NetPayload::InputCommand on Linux, macOS, and Windows
  Then all three serialized payloads equal the fixture byte-for-byte
  And the byte-pin CI gate fails any patch that changes a single byte without bumping minor version

Scenario: Semver negotiation accepts a minor-newer server
  Given a client at PROTOCOL_SEMVER 0.1.4
  And a server at PROTOCOL_SEMVER 0.1.7
  When the client sends Handshake { semver: 0.1.4, supported_features: [...] }
  Then the server responds with Handshake { semver: 0.1.4, granted_features: [...] }
  And the session uses the v0.1.4 feature subset
  And no NetError::ProtocolVersionMismatch is raised

Scenario: Semver negotiation rejects a major-mismatched client
  Given a client at PROTOCOL_SEMVER 0.2.0
  And a server at PROTOCOL_SEMVER 0.1.7
  When the client sends Handshake { semver: 0.2.0 }
  Then the server responds with NetError::ProtocolVersionMismatch { server: 0x0107, client: 0x0200 }
  And the client surfaces a "download client update" prompt with the server's exposed download URL

Scenario: Reliable QUIC stream carries the canonical event log
  Given a session in flight at tick 600
  When the server emits an EventBatch containing 12 events of cumulative size 4.2 kB
  Then the EventBatch travels over the reliable bidi stream "event_log"
  And the client acks within one RTT
  And no event is dropped, reordered, or duplicated even at 5% link loss

Scenario: Unreliable datagram carries per-tick input
  Given a client at tick 612 with input frame I_612
  When the client sends InputCommand { tick: 612, frame: I_612 }
  Then the InputCommand travels over a QUIC unreliable datagram
  And payload size for the InputCommand alone is <= 96 bytes
  And the unreliable path never blocks the reliable event stream

Scenario: 6-frame rollback resimulates inside budget
  Given a client running rollback prediction at 60 Hz
  When the server's authoritative input set at tick 620 mispredicts the client's prediction at tick 614
  Then the client rolls back to tick 614 within 1 ms
  And resimulates ticks 614..=620 within 7 ms wall clock at p99
  And the total rollback window cost is <= 8 ms p99 on the reference platform

Scenario: Redundant-input encoding recovers from single-datagram loss
  Given a client sending InputCommand datagrams at 60 Hz with last-3 redundant tail
  When InputCommand { tick: 700 } is dropped by the network
  Then the server still receives I_700 piggybacked on InputCommand { tick: 701 }'s redundant tail
  And the server's authoritative tick 700 is not stalled
  And no rollback is triggered

Scenario: Reed-Solomon FEC recovers a single-byte-corrupted reliable payload
  Given a reliable EventBatch payload with k=4 data shards + m=2 parity shards
  When 1 of the 6 shards is corrupted in transit
  Then the receiver reconstructs the original payload from the surviving 5 shards
  And emits net.fec_recovered { shards_lost: 1, k: 4, m: 2 }
  And no application-level retransmit is required

Scenario: ICE-lite + STUN punches a symmetric-NAT to symmetric-NAT pair
  Given Client A behind a symmetric NAT
  And Client B behind a symmetric NAT
  And a STUN server reachable at stun.corefall.example:3478
  When both clients gather candidates and run candidate-pair connectivity checks in parallel
  Then a working candidate pair is selected within 4 seconds
  And the chosen transport is "direct" not "relay"
  And the session reports nat_traversal_outcome { method: "ice_lite", path: "direct" }

Scenario: TURN relay engages when ICE-lite fails
  Given Client A behind a symmetric NAT with port restriction
  And Client B behind a symmetric NAT with port restriction
  When ICE-lite candidate-pair checks all fail within 4 seconds
  Then the client falls back to the TURN relay at turn.corefall.example:3478
  And the session reports nat_traversal_outcome { method: "turn_relay", path: "relay" }
  And the player UI shows "connecting via relay" with an expected-latency notice

Scenario: Transport-select picks dedicated-server-authority for cf-server connections
  Given a player joining cf-server --mode coop_room
  When transport_select runs at session start
  Then the chosen mode is TransportMode::DedicatedServerAuth
  And the client uploads inputs + receives snapshots
  And no peer-to-peer datagrams are exchanged

Scenario: Transport-select picks host-authoritative lockstep for cf-server lan_room
  Given a player joining cf-server --mode lan_room as a guest
  When transport_select runs at session start
  Then the chosen mode is TransportMode::HostAuthoritativeLockstep
  And inputs from all guests are merged on the host and broadcast per tick
  And all guests sim-step on identical merged input sets

Scenario: Protocol downgrade attack is rejected
  Given a man-in-the-middle attacker that rewrites the client's Handshake to advertise semver 0.0.1
  When the server processes the Handshake
  Then the server detects the QUIC TLS-bound handshake integrity violation
  And the session is closed with NetError::Transport("tls handshake mismatch")
  And the protocol semver is never silently downgraded below the client's true advertised version
```

## Out of scope

- WebTransport / browser spectator transport (planned for M40A).
- Steam Datagram Relay integration (planned for M41 behind `cargo feature net-steam`).
- EOS adapter (planned for M41 behind `cargo feature net-eos`).
- Anti-cheat input validation rules (M41 `competitive` profile).
- Mod hash sync at join (M41B).
- Cross-region MMR balancing (M41B).
- Shard mesh handoff transport (M36E).
- Replay-over-network spectator streaming (M40A).
- IPv6-only NAT64 traversal (deferred until cf-net v0.2).
- WebRTC fallback (never; QUIC is the only transport).

## Dependencies

- M8A closed (cf-net scaffold + QUIC primitives + rollback budget declared) — commit `b3f9f1c` per `specs/done/M8A.md`.
- M36 dedicated server modes (`coop_room` / `pvp_arena` / `lan_room` / `mmo_shard` / `lobby_directory`) — server mode enum exists.
- `quinn` 0.11+ with QUIC datagram extension support (already pinned in `game/Cargo.toml`).
- BLAKE3 (already pinned) for content-hash integrity on the handshake.
- A STUN reference deployment plan for the launch infra (operator deliverable; can be a third-party STUN cluster initially; doc PR adds the reference list).

## Notes for the implementer

- Frame v0.1 layout MUST be expressible as a single `#[repr(C, packed)]`-equivalent serde encoder; use `bincode` 2.x with a fixed-int + little-endian options + the `serde_with::DisplayFromStr` adapter banned in this crate (it produces variable-length encodings).
- The 6-frame rollback resimulation MUST reuse the deterministic sim core verbatim — no parallel rollback codepath; the rollback driver wraps the same `World::tick(...)` entry used by the live sim. Anything else is a determinism risk.
- Per-frame input prediction is "last-input-repeat" baseline. Smarter prediction (extrapolated aim, decayed move) is explicitly out of scope; the determinism cost of cleverness here is not worth the perceived smoothness.
- Redundant-input encoding piggybacks the last 3 inputs on each datagram (configurable `net.redundant_input.window_ticks`). The cost is +192 bytes/datagram at the default; the recovery benefit on a 5% loss link is enormous.
- Reed-Solomon FEC is used ONLY on small reliable payloads (event batches < 8 kB). Larger payloads rely on QUIC's own stream retransmit, which is more efficient.
- ICE-lite (not full ICE) is the chosen NAT-traversal profile per IETF RFC 5245: the cf-net client is the controlling agent, the cf-net server is the controlled agent + always has a server-reflexive candidate. This is the simplest workable profile for a client-server game; full ICE is overkill.
- The TURN relay is a deliberate operational dependency. Self-hosted servers (M42) MUST be able to operate without it; the M42 docs include "running without TURN" guidance for community hosts. The first-party launch infra includes a TURN cluster.
- The transport-select policy is deterministic: same server mode + same client capabilities → same transport choice. CI gate runs the full matrix.
- Byte-pinning the wire format is non-negotiable. Any change that flips a single byte in a v0.1 fixture vector MUST bump `PROTOCOL_SEMVER` minor and add a new fixture; the CI gate enforces this.
- Do NOT introduce `f64` anywhere in the wire encoding even though wire-encoding is not a "sim crate" per the AGENTS.md rule. Mixed `f32` / `f64` in serialization paths is a common source of cross-OS divergence on edge cases (subnormals, NaN canonicalization). Keep it `f32`.
- The rollback algorithm uses the engine's seeded RNG, not `thread_rng()`, for any predicted-input fill (e.g., dead-reckoning aim drift if we ever add that). Pre-roll into a `Vec<u64>` before the resim block, just like the parallel sim does.
- All new events MUST cap their cosmetic flag at `false` — protocol-layer events are sim-relevant + non-droppable under M4 cosmetic backpressure.
- All new schemas MUST be added to `dump_schemas --check` so schema drift fails CI.
