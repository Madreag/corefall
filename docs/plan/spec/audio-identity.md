---
type: spec
status: planning-anchor-v0
authority: "Audio identity and mix policy. Specific instruments/tools/composer remain open."
ready_when: "First playable Slice A demonstrates diegetic-first mix with one situational synth-dread cue and full caption coverage."
feeds:
  - DR-012
  - DR-014
  - DR-020
---

← [[spec/index|spec section]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/comms-voice-and-radio-model|comms/voice/radio]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-012-accessibility-comfort-readability|DR-012]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-043-voice-comms-and-radio-direction|DR-043]]

# Audio Identity

> [!summary] What this page is
> Diegetic industrial synth-dread. The simulation makes the music; sparse synth dread layers on top only when tension justifies it; captions are mandatory for every critical cue.

## Three Layers

| Layer | Always On? | Purpose | Examples |
|---|---|---|---|
| Diegetic physical | Yes | Tactical UI; world feel; combat readability. | Gunfire, drilling, jetpack, hydraulics, servos, reactor hums, shield buzz, alarms, radio chatter, collapsing terrain, sparks, smoke vents, repair tools. |
| Synth / dread emotional | Situational | Tension amplification at key narrative moments. | Command core uprooted; base power failing; enemy commander push; pilot trapped; mech reactor critical; extraction window; post-mission debrief. |
| Caption / event | Always (required) | Accessibility + replay playback fidelity. | Every critical SFX has a caption. Every audio event is in the event taxonomy. |

## Audio As Tactical UI

Audio is **not** decoration. It is HUD.

| Cue | What It Tells The Player |
|---|---|
| Loud weapon report (own actor) | "You just made noise; AI alarm triggered." |
| Servo grind (mech) | "A leg/arm joint is degrading." |
| Hydraulic hiss (mech) | "Module pressure drop; failure imminent." |
| Spark/crackle (chassis) | "Armor cracked here." |
| Reactor hum (base) | "Base power is rooted/healthy." |
| Reactor hum dropping out (base) | "Command core uprooted; base systems shedding." |
| Shield buzz | "Shield active; emitter operational." |
| Shield buzz pitch shift | "Shield overheating; cooldown imminent." |
| Pilot eject alarm | "PILOT EJECTING NOW — extraction window open." |
| Radio chatter — "covering door" | "Friendly bot intent: covering this door." |
| Radio chatter — "low ammo, falling back" | "Friendly bot will retreat unless re-supplied." |
| Enemy commander voice | "Commander is making a strategic decision now." |

## Mix Policy

| Rule | Why |
|---|---|
| Synth music ducks under critical alarms. | Never mask tactical-cue audio. |
| Diegetic SFX are positioned in stereo. | Side-view battlefield uses left-right stereo pan to convey direction. |
| Caption events fire at the same tick as the SFX they describe. | Replay subtitle fidelity. |
| Music auto-fades within 2 seconds of all tension triggers clearing. | Avoid lingering dread that confuses pacing. |
| Player can cap music volume independently from SFX. | Accessibility + personal preference. |
| Origin-class failure sounds (organic / android / robot / mech / command-core) are distinct families. | Player can identify the dying actor from sound alone. |

## Caption Contract

Per [[decisions/dr-012-accessibility-comfort-readability]] and [[spec/accessibility-comfort-slice-a]]:

- Every critical audio cue has a caption.
- Captions are styled per category (combat / mech damage / commander / environment / radio).
- Captions queue with priority (critical alarms first; ambient hum lowest).
- Captions are replay-faithful — replaying an event re-fires the caption.
- Modders authoring SFX must provide a caption; missing caption = validator warning per [[decisions/dr-006-modding-data-model]].

## Origin-Specific Failure Sound Families

| Origin | Failure Sound Family |
|---|---|
| Organic actor | Wet impact, breathing, groan, blood spatter, body fall thud. |
| Android | Servo whine, capacitor pop, voice modulator stutter, shell rattle. |
| Robot | Mechanical clunk, loose bolt, gear grind, optical static. |
| Powered armor | Hydraulic hiss + organic groan layered. |
| Light/medium mech | Servo grind, alarm sequence, hydraulic burst, pilot voice. |
| Heavy mech | Deep reactor whine, structural creak, alarm cascade, ejection rocket. |
| Command-core avatar | Reactor pulse, distortion artifact, command-channel warning, identity-failure tone. |

## Open Questions

| Question | Status |
|---|---|
| Composer / soundtrack vendor | Open. Could be commissioned, library-based, procedural. |
| Synth layer: generative or pre-composed? | Open. Generative is moonshot. |
| Voice acting depth (radio chatter style) | Open. Procedural snippets, full VO, or none. |
| 3D / spatial audio implementation | Open. Stereo positioning at minimum. |
| Localization of voice content | Tied to general localization plan (still open). |
| Audio mod loader (replace SFX without recompiling) | Open. Likely yes per modding model. |

## Source Trail

- [[decisions/dr-020-audio-identity]]
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-012-accessibility-comfort-readability]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/accessibility-comfort-slice-a]]
- [[systems/ux-overlay-screen-brief]]
- [[systems/replay-event-architecture]]
