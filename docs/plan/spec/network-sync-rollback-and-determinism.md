---
type: spec
status: closed-direction
authority: "Network sync architecture: server owns truth, GPU owns richness, client owns feel. Server-authoritative + client prediction + reconciliation + rollback netcode (PvP) + deterministic lockstep (LAN) + snapshot interpolation (online co-op) + bit-deterministic replay across all platforms. AI-agent-driven sync tests via cfctl."
ready_when: "All match modes pass deterministic-replay CI; rollback netcode for PvP latency <50ms feel-test; lag compensation for shooter combat <120ms; cross-platform determinism verified."
feeds:
  - DR-002
  - DR-005
  - DR-013
  - DR-024
  - DR-025
  - DR-028
  - DR-034
  - DR-035
  - DR-046
  - DR-047
  - DR-052
  - DR-054
---

← [[spec/index|spec section]] · [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism|DR-052]] · [[spec/server-app-architecture|server app]] · [[spec/persistent-mmo-architecture|MMO architecture]] · [[spec/ai-control-observability-layer|cfctl/T-CONTROL]]

# Network Sync, Rollback Netcode & Cross-Platform Determinism

> [!summary] What this page is
> The complete network architecture and determinism contract. Solves the owner's "all players perfectly in sync with every action without delay" target by making local feel immediate while server truth prevents permanent divergence. AI-agent-testable via cfctl scripts. Cross-platform replay-determinism is the foundation; per-mode network adapter is the implementation.

## Per-Mode Network Architecture

| Mode | Architecture | Frame budget |
|---|---|---|
| Solo | Local in-process authoritative server/sim path; no internet transport | N/A |
| LAN co-op (M10) | Deterministic lockstep | 16ms input delay (1 frame at 60Hz) |
| Online co-op (M11) | Server-auth + client prediction + snapshot interp + lag compensation | 50-200ms target latency tolerance |
| PvP arena (M12) | Rollback netcode (GGPO) + server validation | 16-50ms input delay (1-3 frames at 60Hz) |
| MMO shard (M12) | Server-auth + interest mgmt + snapshot delta | Adaptive 60-120Hz tick |
| Cross-shard events | Eventually-consistent broadcaster | Not real-time |

## Authority And GPU Tiering

| Class | Examples | Rule |
|---|---|---|
| Truth | Health, inventory, mission state, terrain collision, material/gas/fire truth, confirmed projectile hits, base power, doors/platforms, AI final decisions, save/replay state, PvP validation. | Server/local authoritative sim decides. GPU may affect only after DR-054 Tier 4 certification. |
| Prediction | Local movement, provisional projectile path, provisional impact, held-weapon response. | Client CPU/GPU predicts immediately; server correction path reconciles. |
| Presentation | Lighting, smoke, particles, trails, decals, camera shake, audio, interpolation, debug overlays. | Client GPU should be used aggressively; machine-to-machine divergence is acceptable. |
| Advisory | Broadphase candidates, pathfinding heatmaps, visibility hints, AI perception maps, compression hints. | Can be GPU-computed; CPU/server validates before truth changes. |

True zero internet lag is impossible. The target is no perceptible input lag for local feel and no divergent authoritative game state.

## Determinism Contract

### Sim tick output bit-identical
- Same seed + same input sequence → same output state.
- Per-tick checksum via `blake3` of sim state.
- CI matrix: 100 runs/seed × Win/Lin/Mac × x86/ARM.

### Cross-platform float
- `f32` only in sim islands; no `f64`.
- `RUSTFLAGS="-C target-feature=+sse2,+sse4.2"` baseline.
- LLVM `-ffast-math` disabled in sim crates (`cargo:rustc-env=NO_FAST_MATH=1`).
- `STD::FROUND_TO_NEAREST` rounding mode.
- No transcendental functions (sin/cos/exp) on hot paths without LLVM-stabilized impl OR fixed-point alternative.

### Replay reproducibility
- `cf-replay` event log + per-tick checksums.
- `cf-headless replay --verify-checksums` walks every event; asserts state matches.

### Cosmetic vs gameplay
- Cosmetic flag for non-deterministic systems (particle effects, decals beyond gameplay impact).
- Gameplay-critical (physics, sim, AI decisions, projectile trajectories) IS deterministic.

### Network input ordering
- Per-tick input batches collected on server.
- Deterministic ordering: by client_id ascending; tie-break by player_id.

## Client Prediction + Reconciliation (M11 + M12 MMO)

```
Client Tick T:
  1. Read player input.
  2. Predict local actor state (move, fire, reload).
  3. Render predicted state.
  4. Send input to server.
  5. Wait for server snapshot.

Server Tick T (1 RTT later):
  6. Process input.
  7. Validate (anti-cheat).
  8. Update authoritative state.
  9. Send snapshot to clients.

Client Tick T + N (snapshot arrives):
  10. Compare predicted state at tick T to server state.
  11. If matches: continue.
  12. If mismatch: rewind predicted state to tick T - 1 (last-acknowledged) + replay client inputs forward with corrected state.
  13. Visual smoothing if mismatch < threshold; snap if > threshold.
  14. Emit `prediction_corrected` event with reason label.
```

### Prediction window
- 1-3 frames at 60Hz (16-50ms).
- Capped at server's max-allowed-prediction.

### Anti-cheat validation
- Server validates every client input against current state.
- Rejects impossible inputs (shoot-through-wall, teleport, infinite-ammo).
- Reason label `input_rejected_by_anticheat` emitted.

## Rollback Netcode (M12 PvP Arena)

GGPO-style with deterministic sim:

```
Tick T (local input arrives):
  1. Predict opponent's input (default: repeat-last-input).
  2. Run sim with both inputs.
  3. Render predicted state.
  4. Send local input to opponent.

Tick T + 1 frame (opponent input arrives):
  5. If matches prediction: continue.
  6. If mismatch:
     a. Rollback to tick T.
     b. Re-run sim with correct opponent input.
     c. Re-render.
  7. Bounded rollback window: 8 frames (133ms).
```

### Determinism requirement
- All sim systems must be deterministic (per DR-052).
- Cosmetic systems excluded.

### Server validation
- Server runs canonical sim.
- Client rollback is local optimization.
- Server-state wins on any disagreement.

## Lag Compensation (CS:Source Model, Adapted)

For projectile-based combat:

```
Client Tick T:
  1. Player fires weapon.
  2. Send shot input + client view tick (T - interp_delay).

Server Tick T + RTT/2:
  3. Receive shot input + client view tick.
  4. Server rewinds world state by client's interp delay + ping/2.
  5. Validate hit at rewind tick.
  6. If hit valid (target visible from shooter's POV at rewind tick): apply damage.
  7. If invalid: emit `shot_rejected_by_lag_compensation` event.
```

### Rewind cap
- Max 200ms rewind. Prevents "shoot from behind a wall."

### Reconciliation invariants
- Hit valid only if target was visible from shooter's POV at rewind tick.
- Server-side validation of shot angle + range + LoS.

## CLI Testability

Per [[spec/ai-control-observability-layer]] T-CONTROL extension.

| Command | Purpose |
|---|---|
| `cfctl test sync-drift --client-count 4 --duration 60s --scenario X` | Multi-client sync drift detection. |
| `cfctl test latency-injection --add-ms 200 --packet-loss 5% --jitter 50ms` | Network degradation simulation. |
| `cfctl test rollback-burst --frames 8 --frequency 1hz --duration 60s` | Rollback stress test. |
| `cfctl test replay-determinism --runs 100 --scenario X --seed-set Y` | 100-run determinism verification. |
| `cfctl test cross-platform-determinism --platforms win,lin,mac --runs 100` | Cross-platform CI matrix. |
| `cfctl test multi-shard --shards 4 --players-per 50 --duration 1h` | MMO load test. |
| `cfctl test combat-ttk --weapon X --chassis Y --runs 100` | TTK regression. |
| `cfctl test network-jitter --jitter 5..200ms --packet-loss 0..15% --runs 100` | Resilience boundary. |
| `cfctl test prediction-correction-rate --duration 60s` | Prediction accuracy. |
| `cfctl test anti-cheat-injection --type X --runs 50` | Anti-cheat validation. |
| `cfctl test replay-bit-identical --bundle X --runs 100` | Replay 100x; assert match. |
| `cfctl test replay-divergence --bundle X --inject-noise per-tick` | Determinism recovery. |
| `cfctl bench replay --bundle X --measure throughput` | Replay throughput. |

## Network Simulator (`cf-network-sim`)

Dev-tool for development testing.

| Feature | Detail |
|---|---|
| Latency injection | 0-500ms |
| Packet loss simulation | 0-50% |
| Jitter | 0-200ms variation |
| Bandwidth cap | 10kbps - 100Mbps |
| Per-platform emulation | Steam Deck network, mobile hotspot, satellite, dial-up |

CI integration: pre-merge tests with simulated 200ms RTT + 5% loss + 50ms jitter.

## Performance Budget

Per DR-054.

| Tier | Network budget |
|---|---|
| Online co-op (4 players) | <50KB/s upstream + <100KB/s downstream per client |
| PvP arena (8 players) | <200KB/s per client |
| MMO shard (50 players) | <500KB/s per client peak |
| MMO shard (200 players) | <2MB/s per client peak |

## Done-Criteria

- [ ] All match modes pass deterministic-replay CI.
- [ ] Rollback netcode PvP latency <50ms feel-test.
- [ ] Lag compensation shooter combat <120ms.
- [ ] Cross-platform determinism verified (Win/Lin/Mac × x86/ARM).
- [ ] All cfctl test commands functional.
- [ ] Network simulator integrated.
- [ ] Anti-cheat invariants enforced.
- [ ] Per-milestone sync verification per DR-056.
- [ ] `cfctl test sim-backend-authority --backend cpu,gpu_advisory,gpu_certified` proves CPU truth, advisory non-authority, and certified manifest enforcement.

## Source Trail

- [[decisions/dr-052-network-sync-rollback-and-cli-testable-determinism]]
- [[decisions/dr-054-performance-optimization-and-profiling]]
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- GGPO: https://www.ggpo.net/
- SnapNet Netcode Architectures: https://snapnet.dev/blog/netcode-architectures-part-3-snapshot-interpolation/
- Source Multiplayer Networking: https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking
- Bevy lightyear: https://github.com/cBournhonesque/lightyear
- quinn (QUIC): https://github.com/quinn-rs/quinn
