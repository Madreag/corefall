---
type: decision
id: DR-052
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Network sync drift detected in production; rollback netcode adds unacceptable latency; CLI-driven sync tests fail to catch regressions; per-platform float determinism breaks."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/network-sync-rollback-and-determinism|network sync spec]] · [[spec/ai-control-observability-layer|cfctl/T-CONTROL]] · [[decisions/dr-002-replay-event-architecture|DR-002]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-024-native-engine-stack|DR-024]] · [[decisions/dr-034-dedicated-server-application|DR-034]]

# DR-052: Network Synchronization, Rollback Netcode & CLI-Testable Determinism

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Hybrid network model per match mode: **server-authoritative simulation** (DR-005) + **client-side prediction with reconciliation** + **rollback netcode for PvP arenas** (M12) + **deterministic lockstep for LAN co-op** (M10) + **snapshot interpolation for online co-op** (M11) + **bit-deterministic replay across all platforms** (DR-002). Owner architecture: **server owns truth, GPU owns richness, client owns feel.** All sync paths fully testable via cfctl scripts: `cfctl test sync-drift`, `cfctl test latency-injection`, `cfctl test rollback-burst`, `cfctl test replay-determinism`. Floating-point determinism guaranteed via `f32` strict ordering + `STD::FROUND_TO_NEAREST` + cross-platform LLVM compile flags + per-tick checksum.

## Decision

### Per-mode network architecture

| Mode | Architecture | Rationale |
|---|---|---|
| **Solo / single-player** | Local in-process authoritative server/sim path; no internet/network transport required. Per-tick replay event chain. | Same authority, replay, save, cfctl, and future multiplayer migration path as online modes without network cost. |
| **LAN co-op (M10)** | **Deterministic lockstep**. All clients run identical sim; only inputs sent. Sync probe per 60-tick interval; first-divergence-event detector. | Matches GGPO/Rollback precedent for tactical games; near-zero latency on LAN; bandwidth bounded by input size. |
| **Online co-op (M11)** | **Server-authoritative simulation + client prediction + reconciliation + snapshot interpolation**. Clients predict locally for player-driven actor; server is canonical. Snapshot interp for remote actors at -100ms history buffer. Lag compensation per CS:Source model. | Tolerates 50-200ms latency; remote actor positions hidden behind interp buffer; player actor feels responsive. |
| **PvP arena (M12)** | **Rollback netcode** (GGPO-style) + server-authoritative state validation + anti-cheat invariants. Local prediction + 8-frame rollback window per match-tick mismatch. Input delay 1-3 frames at 60Hz (16-50ms). | Matches fighting-game precedent; sub-perceptual latency for competitive PvP. |
| **MMO shard (M12)** | **Server-authoritative + interest management + snapshot delta encoding**. Per-actor interest set per [[spec/persistent-mmo-architecture]]. Tick rate adaptive per client (60Hz floor, 120Hz ceiling for high-bandwidth). | Scales to 50-200 concurrent players; bandwidth-aware. |
| **Server-server (cross-shard events)** | **Eventually-consistent event broadcaster**. Per [[spec/server-wide-events-and-meta-narrative]]. Centralized event-state authority; per-shard verifies signed JSON. | Cross-shard MMO events; not real-time; eventually-consistent. |

### Authority taxonomy

| Class | Meaning | GPU/client role | Can diverge visually? |
|---|---|---|---|
| **Truth** | Actor health, inventory/resources, mission state, terrain collision, material/gas/fire truth, projectile hit confirmation, base power, doors/platforms, AI final decisions, saves/replays, PvP validation. | Server/local authoritative sim computes or validates. GPU may accelerate only if certified equivalent to CPU. | No. |
| **Prediction** | Local player movement, temporary projectile path, provisional impact, held-weapon response. | Client CPU/GPU predicts immediately, then reconciles to server. | Briefly, then corrected. |
| **Presentation** | Lighting, smoke visuals, particles, trails, decals, interpolation, camera shake, audio, debug overlays. | Client GPU should do this aggressively. | Yes. |
| **Advisory** | Broadphase candidates, pathfinding heatmaps, visibility hints, AI perception maps, compression/decompression hints. | GPU/server/client can compute hints; CPU/server validates before truth changes. | Yes, until validated. |

### Determinism guarantees

| Layer | Guarantee | Mechanism |
|---|---|---|
| **Sim tick output** | Bit-identical given same seed + same input sequence. | Fixed-tick 60Hz; deterministic system order; per-tick checksum (`blake3` of sim state). |
| **Cross-platform float** | Bit-identical across Windows/Linux/macOS x86 + ARM. | `f32` only in sim islands (no `f64`); `RUSTFLAGS="-C target-feature=+sse2,+sse4.2"` baseline; LLVM `-ffast-math` disabled in sim crates; verified via determinism CI suite (matrix Win/Lin/Mac × x86/ARM × 100 runs/seed). |
| **Replay reproducibility** | Same manifest + seed + inputs reproduces same final state byte-for-byte. | `cf-replay` event log + per-tick checksums; `cf-headless replay --verify-checksums` walks every event, asserts state matches. |
| **Cosmetic vs gameplay** | Cosmetic systems (particles, decals beyond gameplay impact, audio cosmetic flag) **NOT** in determinism island. Gameplay-critical (physics, sim, AI decisions, projectile trajectories) **IS** deterministic. | Per-system `cosmetic: true` flag; replay events tagged. |
| **Network input ordering** | Per-tick input batches collected on server; deterministic ordering by client_id. | Server-authoritative input queue; tie-break by player_id ascending. |

### GPU authority rule

GPU work is welcome for richness and performance, but it is not authoritative by default. A GPU kernel may affect Truth only after the certification matrix in DR-054 passes across NVIDIA, AMD, Intel, Apple, and Steam Deck with the same seed, same inputs, same mod set, 10K+ ticks per kernel, per-tick BLAKE3 checksums, and final byte-identical state against the CPU path. Until then, GPU output is Presentation, Prediction, or Advisory.

### Client prediction + reconciliation

For online co-op (M11) + MMO (M12):

| Aspect | Detail |
|---|---|
| **Predicted state** | Player actor + held weapon + own projectiles spawned this tick. |
| **Authoritative state** | Server snapshot every 60 ticks (1s); delta-encoded per actor. |
| **Reconciliation** | On snapshot arrival: rewind predicted state to last-acknowledged tick + replay client inputs forward. Visual smoothing via interpolation if mismatch < threshold; snap if > threshold. |
| **Prediction window** | 1-3 frames at 60Hz (16-50ms). Capped at server's max-allowed-prediction. |
| **Mispredict handling** | Server-state-wins; client visually corrects with smooth interpolation. Reason label `prediction_corrected` emitted to replay. |
| **Anti-cheat** | Server validates every client input against current state; rejects impossible inputs (e.g., shoot-through-wall, teleport, infinite-ammo). |

Prediction exists to remove perceptible input lag, not to remove server authority. True zero internet lag is impossible; the target is immediate local feel plus no permanent divergent game state.

### Rollback netcode (M12 PvP arena)

GGPO-style:

| Aspect | Detail |
|---|---|
| **Algorithm** | Local input → predict opponent's input → run sim. On opponent input arrival: if matches prediction, continue. If mismatch: rollback to mismatch-tick + re-run sim with correct input. |
| **Input delay** | 1-3 frames at 60Hz (16-50ms). Matches fighting-game precedent. |
| **Rollback window** | 8 frames (133ms). Bounded to prevent compounding rollback. |
| **Determinism requirement** | All sim systems must be deterministic. Cosmetic systems excluded. |
| **Input prediction** | Default: repeat-last-input. Optional: ML-based input prediction (post-launch). |
| **Server validation** | Server runs canonical sim; client rollback is local optimization. Server-state wins on any disagreement. |
| **Anti-rollback-abuse** | Server tracks rollback frequency per client; bans clients with anomalous patterns. |

### Lag compensation (CS:Source model, adapted)

For projectile-based combat:

| Aspect | Detail |
|---|---|
| **Server rewind** | On client shot, server rewinds world state by client's interp delay + ping/2 to validate hit. |
| **Rewind cap** | Max 200ms rewind. Prevents "shoot from behind a wall after they ran past." |
| **Reconciliation invariants** | Hit valid only if target was visible from shooter's POV at rewind tick. |
| **Server-side validation** | Anti-cheat: rejected shots with impossible angles, infinite-range hits, etc. |

### CLI-testable sync (cfctl extension)

Every sync path is testable via cfctl + automated scripts.

| Test command | What it does |
|---|---|
| `cfctl test sync-drift --client-count 4 --duration 60s --scenario X` | 4 clients run scenario; assert per-tick checksum matches across clients; report first-divergence tick. |
| `cfctl test latency-injection --add-ms 200 --packet-loss 5% --jitter 50ms --duration 60s` | Inject network degradation; assert match still completes; assert no permanent state corruption. |
| `cfctl test rollback-burst --frames 8 --frequency 1hz --duration 60s` | Force 8-frame rollback at 1Hz cadence; assert sim recovers correctly. |
| `cfctl test replay-determinism --runs 100 --scenario X --seed-set Y` | Run scenario 100 times; assert all 100 final-state checksums match. |
| `cfctl test cross-platform-determinism --platforms win,lin,mac --runs 100` | CI matrix runs scenario across Win/Lin/Mac (x86 + ARM); assert checksums match. |
| `cfctl test multi-shard --shards 4 --players-per 50 --duration 1h` | MMO-scale soak test; assert bandwidth + latency within budget. |
| `cfctl test combat-ttk --weapon X --chassis Y --runs 100` | TTK regression test; deterministic outcome. |
| `cfctl test network-jitter --jitter 5..200ms --packet-loss 0..15% --runs 100` | Probe network resilience boundary; auto-find break point. |
| `cfctl test prediction-correction-rate --duration 60s --scenario X` | Per-client prediction-correction frequency; assert <5% to flag UX-blocking mispredictions. |
| `cfctl test anti-cheat-injection --type X --runs 50` | Inject cheat patterns; assert server rejects all. |

### CLI-testable replay determinism

| Test | Detail |
|---|---|
| `cfctl test replay-bit-identical --bundle X --runs 1` | Replay bundle once; assert checksums match. |
| `cfctl test replay-bit-identical --bundle X --runs 100` | Replay bundle 100 times; assert ALL match. |
| `cfctl test replay-divergence --bundle X --inject-noise per-tick` | Replay with per-tick noise injection; assert deterministic recovery. |
| `cfctl bench replay --bundle X --measure throughput` | Replay throughput benchmark per platform. |

### Network simulator for development

Per [[spec/post-launch-operations-and-platform]]. `cf-network-sim` dev-tool:
- Artificial latency / packet loss / jitter / bandwidth cap.
- Run pre-merge in CI: simulated 200ms RTT + 5% loss + 50ms jitter; assert sync passes.
- Per-platform: emulate Steam Deck network conditions, mobile hotspot, satellite, dial-up.

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-net` | Per-mode adapter (lockstep, prediction+reconciliation, rollback, snapshot delta, eventually-consistent). |
| `cf-net-determinism` | Floating-point determinism contract enforcement; CI grep gates; cross-platform CI matrix. |
| `cf-rollback` | New crate (or extension of `cf-net`) for GGPO-style rollback; PvP arena. |
| `cf-lag-compensation` | New crate for server rewind + reconciliation invariants. |
| `cf-net-sim` | Dev-tool network simulator. |
| `cfctl` | Extended with all `cfctl test sync-*` commands. |
| `cf-headless` | Extended with `replay --verify-checksums` modes. |
| CI | Matrix runs determinism tests on every PR; cross-platform x86 + ARM. |
| `cf-replay` | Extended with cosmetic flag + per-tick checksum + replay-determinism CI. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific NAT/relay library | Open. Default `quinn` (QUIC) + `lightyear` (Bevy networking). Revisit if perf inadequate. |
| Tournament-grade rollback patent licensing | Open. GGPO is open-source; if patent issues arise, custom alternative possible. |
| ML-based input prediction | Post-launch. Default repeat-last-input is sufficient. |
| Dedicated server proxy / mediator role | Open. Default direct-connect; relay fallback if NAT punch-through fails. |
| Specific tick rate per mode | Open. Default 60Hz floor; 120Hz ceiling for high-refresh inputs. |

## Why This Direction

| Driver | Detail |
|---|---|
| Tactical game requirement | Players need responsive feel + fair PvP + verifiable replay. Hybrid model achieves all three. |
| Determinism critical | Per DR-002 replay-event architecture + DR-022 AI humanlike bar + DR-046 tutorial show-me-why; all require bit-identical replay. |
| Cross-platform play | Per DR-025 desktop-first across Win/Lin/Mac; per DR-028 Steam Deck floor; bit-identical determinism mandatory. |
| Anti-cheat foundation | Per DR-005 + DR-034; server-authoritative validation prevents most cheats. |
| AI agent testability | Per DR-026 AI-augmented solo dev; CLI testability mandatory for AI agents to drive E2E sync tests. |
| Performance budget | Per DR-028; sync must fit in tick budget; rollback must complete in 1-2 frames. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Pure peer-to-peer | Anti-cheat impossible; can't enforce server invariants. |
| Pure server-only (no client prediction) | High-latency feel; player input lag intolerable. |
| Lockstep for everything | Stalls on packet loss; doesn't scale to MMO. |
| Pure rollback for everything | Determinism requirement too strict for MMO scale; bandwidth issues. |
| Custom protocol over UDP | Reinventing the wheel; QUIC handles encryption + reliability + multiplexing. |
| `f64` instead of `f32` | Cross-platform `f64` issues even more pronounced; `f32` with strict ordering is industry standard. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "network system needs to have all players perfectly in sync with every action without delay - and this should be able to be tested by the AI coding agent via the CLI and scripts since all actions can be controlled and viewed."
- Project owner clarification (2026-05-07): "Server owns truth. GPU owns richness. Client owns feel."
- GGPO Rollback Networking SDK: https://www.ggpo.net/
- SnapNet Netcode Architectures Part 3: Snapshot Interpolation: https://snapnet.dev/blog/netcode-architectures-part-3-snapshot-interpolation/
- Source Multiplayer Networking (Valve): https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking
- "How It Works: Lag compensation and Interp in CS:GO" reference.
- A physics engine with incremental rollback: https://news.ycombinator.com/item?id=47981979
- Cross-platform floating-point determinism: https://github.com/Unity-Technologies/Unity.Mathematics/issues/88
- Bevy lightyear: https://github.com/cBournhonesque/lightyear
- Captured in [[research-log/2026-05-07-comprehensive-audit-report]].

## Revisit Trigger

- Network sync drift detected in production.
- Rollback netcode adds unacceptable latency.
- CLI-driven sync tests fail to catch regressions.
- Per-platform float determinism breaks.
- Bandwidth budget exceeded under MMO load.
- Anti-cheat invariants insufficient against new attack vectors.
- A gameplay system tries to make uncertified GPU output authoritative.
