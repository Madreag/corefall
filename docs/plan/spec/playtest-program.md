---
type: spec
status: closed-direction
authority: "Playtest program: closed alpha + closed beta + Steam Next Fest demo + soak testing + AI-simulated playtest. AI agents auto-generate playtest reports."
ready_when: "Closed alpha cohort active; closed beta cohort active; soak schedules met; AI-simulated playtests run nightly."
feeds:
  - DR-005
  - DR-022
  - DR-024
  - DR-031
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[spec/marketing-and-launch|marketing]]

# Playtest Program

## Phases

| Phase | Cohort | Build cycle | Duration |
|---|---|---|---|
| **Internal alpha** | Project owner + 0-3 trusted advisors | Daily | M0..M5 |
| **Closed alpha** | ~20-50 invited testers (Discord) | Daily | M6..M9 |
| **Closed beta** | ~200-500 testers (Steam closed beta) | Weekly | M10..M11 |
| **Open beta / Steam Next Fest demo** | ~5K-50K wishlist conversions | Per festival | M12 |
| **Live playtest events** | Community-wide via Discord | Monthly | Post-beta |

## Soak Testing

| Soak | Duration | What it tests |
|---|---|---|
| **Multiplayer netcode** | 24h | Latency, packet loss, reconnect, jitter, NAT punch-through |
| **MMO shard** | 7-day | Persistence, snapshot restore, journal recovery, interest mgmt |
| **Replay determinism** | 100K-tick | Bit-identical replay across runs |
| **AI mission director** | 24h chaos | Procedural mission generation, AI coverage, anomaly detection |
| **Material kernel** | 50K-tick | Reaction stability, chunk budget, perf consistency |
| **Atmospherics** | 50K-tick | Pressure stability, combustion, suit life-support |

## AI-Simulated Playtests

Run nightly. AI agent runs 1000s of scripted scenarios per night to surface:

- Balance outliers (TTK > 99th percentile, win-rate divergence)
- AI-bot regressions (refusal-rate divergence)
- Replay drift
- Perf regressions
- Mission completion-rate divergence

Reports fed to weekly review.

## Done-Criteria

- [ ] All cohorts assembled + active.
- [ ] All soak schedules met.
- [ ] AI-simulated playtests run nightly.
- [ ] Weekly review covers AI report + cohort feedback + telemetry.
- [ ] Steam Next Fest demo build cut.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- Steam Next Fest: https://store.steampowered.com/sale/nextfest
