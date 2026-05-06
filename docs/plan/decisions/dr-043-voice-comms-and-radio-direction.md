---
type: decision
id: DR-043
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Acoustic propagation kernel cannot meet performance budget; ACRE2-style multipath terrain too expensive at 50-actor scenarios; Steam Audio integration produces licensing or dependency conflicts; per-band audio reconstruction overruns audio mixer budget; or Realistic comms policy proves to drive zero observable PvP value over Proximity / Global presets."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|tracker]] · [[spec/comms-voice-and-radio-model|comms spec]] · [[decisions/dr-005-multiplayer-posture|DR-005]] · [[decisions/dr-008-ai-architecture|DR-008]] · [[decisions/dr-012-accessibility-comfort-readability|DR-012]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]] · [[decisions/dr-040-environmental-conditions-and-hazards-direction|DR-040]] · [[decisions/dr-042-game-modes-and-match-grammar-direction|DR-042]]

# DR-043: Voice, Comms, And Radio Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06; **full-fledged voice + radio simulation targeting ACRE2-tier fidelity** with Steam Audio-style acoustic propagation)

## Decision

Voice and radio are FULLY SIMULATED launch-quality systems, not abstracted. Voice propagates through actual atmospheric medium (vacuum = no sound, walls/materials attenuate per Steam Audio-style geometry-based occlusion + transmission + reverb). Radio uses ACRE2's multipath terrain propagation model: free-space path loss + multipath terrain interference + frequency-aware antenna gain + per-band audio reconstruction (band-limit, compression, static-gating, distortion at low SNR).

Frequency bands: HF (3-30 MHz), VHF (30-300 MHz), UHF (300 MHz - 3 GHz), Microwave (3-30 GHz). HAM amateur band roster (160m through 23cm). Military rifleman / squad / vehicle / dish radio classes. Antenna types (whip, dipole, yagi, dish, helical, ground-spike).

Origin gating per [[spec/origin-reaction-and-resource-model]]:
- **Humans**: equipped radio (occupies equipment slot)
- **Robots**: built-in radio (powered by chassis `power` resource)
- **Androids**: built-in OR modular upgrade (no slot occupied; powered by `battery_charge`)

AI subscribes to radio chatter and uses it for doctrine reasoning. Captions for accessibility (DR-012). Server-authoritative voice routing in multiplayer.

Lands at **new milestone M9.5** (between M9 Dedicated Server and M10 LAN Co-op) with precursor hooks across M2 (terrain heightmap), M4 (HUD comms widget), M5 (equipment radios + antennas), M5.7 (acoustic-trauma afflictions), M5.10 (EnvironmentSignal acoustic + EM + comms slices), M6.6 (AI environmental competence subscribes to radio).

## What This Locks In

| Aspect | Commitment |
|---|---|
| Voice propagation | Steam Audio-style (Apache-2.0; Valve) raytraced occlusion + transmission + reverb. Vacuum = no sound. Walls + materials attenuate. |
| Radio propagation | ACRE2-style: free-space path loss + multipath terrain + antenna direction + frequency tuning. Hills break LOS for VHF/UHF; HF can bounce off ionosphere on Earth-class worlds. |
| Frequency bands | HF / VHF / UHF / Microwave with per-band characteristics (range, voice quality, penetration, antenna size). HAM band roster (160m through 23cm). |
| Radio hardware | PRR-Lite, Squad-Mk1/Mk2, LongHaul-AT, Dish-Beacon, HAM-Field, Ionopulse (lore), Robot-Internal, Android-Module. |
| Antenna roster | Whip, long whip, dipole wire, yagi, microwave dish, helical, ground-spike. |
| Audio reconstruction | Band-limit (300-3000 Hz typical voice), compander, static gated by SNR, distortion at low SNR, squelch tail. Per-band EQ profile. |
| Origin gating | Humans equip; robots built-in (powered by `power`); androids built-in or modular (powered by `battery_charge`). |
| AI integration | AI subscribes to assigned frequencies; reads `radio.transmission_received` events; doctrine includes "going dark", DF, comms-blackout refusal. |
| Accessibility (DR-012) | Captions for every voice + radio transmission; visual indicators for transmission state + signal strength + band tuning. |
| Server-authoritative | Voice routing on `cf-server`; clients send Opus streams; server runs propagation + reconstruction. |
| Comms policy | Per-Match: Realistic / ProximityOnly / GlobalChat / CrossTeamDisabled (per [[spec/game-modes-and-match-grammar]]). |
| Replay | `voice` and `radio` event categories with full transmission/reception/blocking/encryption/interference chain. |
| Modding | Radios / antennas / band profiles / comms presets all data-driven; schema validates. |

## What This Explicitly REJECTS

- "Magic global team chat" by default (proximity-only is opt-in per Match comms policy).
- "Game-abstracted radio range" (must be ACRE2-style multipath).
- TeamSpeak / Discord as a hard dependency (voice goes through `cf-server`).
- FMOD as the only audio middleware option (Kira / bevy_kira_audio is Rust-native default; FMOD optional).
- Hidden voice / radio without replay events.

## Why Not The Alternatives

- **Simple proximity voice + global radio chat**: cuts the realistic combat depth that user explicitly committed to. Default is full sim; ProximityOnly is a Match policy toggle.
- **Defer comms to post-launch**: weakens the multiplayer ladder + Bunker Defence flagship. Hooks are needed early; full kernel at M9.5 launch.

User chose **full-fledged comm system in game (full environmental influence on the voice... full Radio system, where hills, material in between, type of radio, different antennas, etc, different frequency profiles, like real life, HAM radio, military rifleman radio, etc)** explicitly.

## Cross-DR Anchors

- DR-005 multiplayer posture — voice routing on cf-server.
- DR-008 AI architecture — AI subscribes to radio.
- DR-012 accessibility — captions mandatory for every voice + radio transmission.
- DR-020 audio identity — extends with full comms surface.
- DR-022 humanlike AI bar — AI uses radio to coordinate; "going dark" is a tactic.
- DR-027 combat-base scope — Bunker Defence comms flavor (vent the bunker, lose voice; switch to radio).
- DR-031 economy — radio + antenna are bought equipment; mod-friendly.
- DR-034, DR-035 — server-authoritative; community-hostable.
- DR-037 atmospherics — voice propagation requires medium; vacuum = no sound.
- DR-038 universal gravity — irrelevant to comms but consistent treatment of universal physics.
- DR-039 worlds — comms light-lag from astrography.
- DR-040 environmental conditions — `acoustic` + `em` + `comms` slices in EnvironmentSignal.
- DR-042 game modes — Match comms policy.

## Revisit Trigger

- Acoustic propagation kernel cannot meet performance budget on Steam Deck floor.
- ACRE2-style multipath terrain is too expensive at 50-actor scenarios.
- Steam Audio integration produces licensing or dependency conflicts.
- Per-band audio reconstruction overruns audio mixer budget.
- Realistic comms policy proves to drive zero observable PvP value over Proximity / Global presets.

## Source Trail

- Project owner direction (2026-05-06).
- [[spec/comms-voice-and-radio-model]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]
- ACRE2 documentation: https://acre2.idi-systems.com/wiki/user/radio-signal-loss
- Steam Audio: https://valvesoftware.github.io/steam-audio/
- TFAR documentation: https://forums.bohemia.net/forums/topic/159393-task-force-arrowhead-radio/
- AN/PRC-152 specs: https://en.wikipedia.org/wiki/AN/PRC-152
- Bevy audio ecosystem (Kira, Oddio, FMOD wrappers): https://bevy-cheatbook.github.io/audio.html
