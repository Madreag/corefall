---
type: decision
id: DR-042
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Bunker Defence proof mission at M7 doesn't deliver A-FEEL gate; team config grammar fails to support modder match modes; server-authoritative match enforcement conflicts with MMO bandwidth at 50-200 concurrent; or asymmetric Bunker Defence proves balance-prohibitive in PvP after M12 evidence."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|tracker]] · [[spec/game-modes-and-match-grammar|game modes spec]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-027-combat-base-scope|DR-027]] · [[decisions/dr-034-dedicated-server-application|DR-034]] · [[decisions/dr-035-persistent-mmo-architecture|DR-035]]

# DR-042: Game Modes And Match Grammar Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06; **Bunker Defence is the flagship game mode**; full Match grammar covers symmetric / asymmetric / FFA / coop-vs-AI / campaign)

## Decision

Every playable match is one `Match` record (per [[spec/game-modes-and-match-grammar#The Match Schema]]). The grammar covers:

- **Bunker Defence** (flagship) — asymmetric attacker-vs-defender with rooted bunker (DR-027), dropship attacker, full coop on either side. Variants: 1v1, 2v2, 3v3, 4v4, Coop-Defence, Coop-Attack.
- **Symmetric Arena** — 1v1, 2v2, 3v3, NvN with equal starts.
- **Free-For-All** — 1v1v1, 1v1v1v1, etc. Each player is their own team.
- **Asymmetric N-Team** — 2v1, 3v1, 4v2 with per-team different starting conditions.
- **Coop-vs-AI** — humans on one team vs AI Hostile.
- **Campaign** — solo or coop linear / branching.

AI fills empty player slots. Comms policy (`Realistic` / `ProximityOnly` / `GlobalChat` / `CrossTeamDisabled`) is per-Match. Mission director authors objectives, victory conditions, dynamic events. Server modes (`coop_room`, `pvp_arena`, `lan_room`, `mmo_shard`) accept Match configs.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Match schema | Locked per [[spec/game-modes-and-match-grammar#The Match Schema]]. Mode preset + asymmetric flag + teams + spawn rules + objectives + victory conditions + comms policy. |
| Bunker Defence as flagship | M7 ships the **Bunker Defence Proof Mission** as the A-FEEL gate. M12 ships the full PvP launch. |
| Coop within teams | All multi-slot teams support coop; AI fills empty slots. |
| Flexible team configs | 1v1, 2v2, 3v3, 1v1v1, 1v1v1v1, 2v1, 3v1, 4v2 — all valid Match instances. |
| Mode presets | Bunker Defence, Symmetric Arena, Free-For-All, Asymmetric N-Team, Coop-vs-AI, Campaign. Modder presets via data row. |
| Server-authoritative match | `cf-server` validates Match on join + enforces rules + replicates state. |
| Replay | `match` event category covers full match lifecycle. |
| Environment-aware | Bunker Defence fights vary by world (vacuum / hot / low-g / storm-active). |

## What This Explicitly REJECTS

- "Two-team symmetric only" — explicitly rejected; user wants any combo (1v1v1v1, 2v1, etc.).
- Hardcoded match modes per server binary — must be data-driven Match grammar.
- Bunker Defence as "post-launch DLC" — rejected; flagship at launch.
- Asymmetric matches without per-team objectives + victory conditions — rejected; must be data-driven.

## Why Not The Alternatives

- **Bake match modes into server code**: every new mode would be a server release; modders couldn't author. Match grammar fixes this.
- **Hard 2-team-only**: explicitly rejected by user.
- **Free-for-all only or symmetric only**: rejected; user wants the full ladder.

## Cross-DR Anchors

- DR-005 multiplayer posture — match grammar implements the multiplayer ladder.
- DR-013 backend service scope — `lobby_directory` server mode lists open matches.
- DR-014 tone player promise — Bunker Defence is the tactical pulp sci-fi disaster sandbox at its purest.
- DR-015 player identity control posture — bunker defenders embody the rooted command-core; attackers fight as commander-controlled squad.
- DR-016 setting and world frame — frontier merc/rescue/salvage frame fits attacker + defender roles.
- DR-017 mission generation strategy — Match block lives inside the typed mission manifest.
- DR-022 humanlike AI bar — AI fills slots; doctrine matches team kind.
- DR-027 combat-base scope — Bunker is the defender's home.
- DR-029 save game model — Campaign mode persists per-mission Match state.
- DR-031 content economy — premium one-time + free modder match modes.
- DR-034 dedicated server application — server modes accept Match configs.
- DR-035 persistent MMO architecture — shard rulesets declare allowed Match modes.
- DR-039 worlds — Match references World for map.
- DR-040 environmental conditions — Bunker Defence fights shaped by world environment.
- DR-043 voice/radio — Match comms policy.

## Revisit Trigger

- Bunker Defence proof mission at M7 doesn't deliver A-FEEL gate.
- Team config grammar fails to support modder match modes.
- Server-authoritative match enforcement conflicts with MMO bandwidth.
- Asymmetric Bunker Defence proves balance-prohibitive in PvP after M12 evidence.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/game-modes-and-match-grammar]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]
