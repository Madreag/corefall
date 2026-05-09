---
type: decision
id: DR-020
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Diegetic-first mix proves too quiet/hard-to-parse, or synth-dread layer fights the pulpy tone."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/audio-identity|audio identity spec]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-012-accessibility-comfort-readability|DR-012]]

# DR-020: Audio Identity

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Diegetic industrial synth-dread. The battlefield soundscape is the main music; sparse synth/dread scoring layered on top only when it improves tension.

## Decision

**Diegetic industrial synth-dread.**

Layers:

1. **Diegetic physical layer (primary, always on)**: gunfire, impacts, drilling, concrete sprayers, jetpacks, servos, hydraulics, reactor hums, shield buzz, turret motors, alarms, radio chatter, collapsing terrain, sparks, smoke vents, repair tools, damaged mech modules. The simulation makes the rhythm of play.
2. **Synth/dread emotional layer (sparse, situational)**: Carpenter-esque synth, ambient drones, mech-power hums. Only triggers when tension justifies it: command core uprooted, base power failing, enemy commander push, pilot trapped, mech reactor critical, extraction window, post-mission replay/death recap.
3. **Caption / event layer (mandatory)**: every critical audio cue has a caption equivalent and emits an event. The game must remain playable and readable without sound.

## Audio As Tactical UI

Audio is a first-class HUD surface, not decoration:

- Loud weapons create AI alarm events; player hears their own noise footprint.
- Mech damage has servo grind, hydraulic hiss, smoke/spark crackle, warning tones — audible status update.
- Base systems (shields, turrets, reactors, doors) have recognizable power states.
- Origin classes (organic, android, robot, mech, command-core avatar) have distinct failure sounds.
- Pilot ejection has a distinct alarm-sequence cue.

## What This Locks In

| Spec Area | Implication |
|---|---|
| Sound design | Physical/diegetic SFX are top priority; music budget stays sparse. |
| Caption system | Every critical SFX has a caption. See [[spec/accessibility-comfort-slice-a]] and [[decisions/dr-012-accessibility-comfort-readability]]. |
| Replay/event | Audio cue events must be in the event taxonomy (so replay can subtitle correctly). See [[systems/replay-event-architecture]]. |
| Mix policy | Synth music ducks under critical alarms; never masks tactical-cue audio. |
| Mod tool | Modders can author SFX + caption together; missing caption = validator warning. See [[spec/modding-model]]. |
| Chassis | Each chassis class authors a failure-sound family. See [[spec/chassis-armor-mechs-and-origins]]. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific AI soundtrack provider / model stack | Open. Must be AI-generated or procedural, license-reviewed, caption-bound, and usage-ledger logged per DR-044 + DR-053. |
| Whether the synth layer is generative or pre-composed | Open. Generative is moonshot. |
| Voice acting depth (radio chatter style) | Open. Could be procedural snippets, full VO, or none. |
| 3D / spatial audio implementation | Open. 2D side-view doesn't strictly need full HRTF, but stereo positioning matters. |
| Localization of voice content | Tied to general localization plan (still open). |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Industrial / military realism (no synth) | Loses emotional dread layer; battles feel flat. |
| Synth / dread alone (Stranger-Things mode) | Loses physical tactical audio cues; mechs/destruction feel weightless. |
| Orchestral cinematic | Wrong tone; X-COM scoring doesn't fit "tactical pulp sci-fi disaster sandbox". |
| Diegetic-first minimalism (no music at all) | Misses opportunity for tension-amplification at key moments. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 3): "Diegetic industrial synth-dread… The battlefield soundscape is the main music… Layer sparse synth/dread scoring on top only when it improves tension."
- Captured in [[research-log/2026-05-04-spec-round-3-visuals-audio-tutorial-mechs-ai]].
- Spec page: [[spec/audio-identity]].
- Aligns with [[decisions/dr-014-tone-player-promise]] diegetic feedback layer requirements (smoke, sparks, alarms, hydraulic whine, servo failure).

## Revisit Trigger

- Diegetic-first mix proves too quiet or too hard to parse.
- Synth-dread layer fights the pulpy tone.
- Caption coverage gaps make the game inaccessible.
