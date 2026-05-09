---
type: spec
status: design-intent-post-m1
authority: "Canonical contract for the full voice + radio simulation: acoustic propagation through atmosphere (vacuum = no voice; walls / hills / materials attenuate), realistic radio with frequency tuning + antenna LOS + multipath terrain interference + per-band audio characteristics (HF / VHF / UHF / microwave), origin-gated radio hardware (humans equip; robots built-in; androids modular), and AI agents that hear and use radio chatter. Targets ACRE2-tier fidelity. Lands at new M9.5 between M9 (Dedicated Server) and M10 (LAN Co-op), with precursor hooks across M2 / M4 / M5 / M5.10."
ready_when: "Voice propagates through atmospheres (vacuum = no sound); radios have frequencies + antennas + power; terrain + walls + materials attenuate; multiple band profiles ship (HF / VHF / UHF / microwave); audio reconstruction includes static + band-limit + interference; origin gating works (humans equip; robots built-in; androids modular); AI subscribes to radio chatter; COMMS-A acceptance suite passes."
feeds:
  - DR-002
  - DR-005
  - DR-006
  - DR-008
  - DR-012
  - DR-013
  - DR-014
  - DR-018
  - DR-020
  - DR-022
  - DR-024
  - DR-027
  - DR-031
  - DR-033
  - DR-034
  - DR-035
  - DR-037
  - DR-038
  - DR-039
  - DR-040
  - DR-042
  - DR-043
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/audio-identity|audio identity]] · [[spec/atmospherics-and-chemistry-model|atmospherics/chemistry]] · [[spec/celestial-bodies-and-worlds-model|worlds catalog]] · [[spec/environmental-conditions-model|environmental conditions]] · [[spec/origin-reaction-and-resource-model|origin reaction/resource]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/game-modes-and-match-grammar|game modes]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/native-implementation-backlog|native backlog]] · [[decisions/dr-020-audio-identity|DR-020]] · [[decisions/dr-043-voice-comms-and-radio-direction|DR-043]]

# Comms, Voice, And Radio Model

> [!summary] What this page is
> Full-fidelity voice + radio simulation. The reference is **ACRE2** (Arma 3 mod; the gold standard for realistic radio sim) — multipath terrain propagation, frequency-aware antenna gain, per-band audio reconstruction. Voice propagation is **Steam Audio**-grade — geometry-occlusion + transmission + reverb. Vacuum = no voice. Walls attenuate. Hills break LOS for VHF but not HF (which can bounce off ionosphere on planets with one). Different radios have different frequency profiles (HAM bands, military rifleman radios, vehicle long-range, microwave dishes). Sound on radio is real — band-limited, statics-gated, distorted at low SNR.
>
> Origin gating: humans need an equipped radio; **robots have built-in radio** (powered by `power` resource); androids may have it built-in or as a modular upgrade that doesn't take an equipment slot.
>
> Targets a launch-quality realistic simulation that supports Bunker Defence + cross-world missions + Stationeers-grade sealed-room scenarios + the multiplayer ladder (DR-005).

> [!warning] Authority boundary
> Captured 2026-05-06 as **design intent**. The model (acoustic propagation source + receiver + medium; radio source + antenna + propagation + receiver + audio reconstruction; origin gating; band profiles) is committed. Specific tuning (per-radio range curves, terrain-loss coefficients, audio band-limit filter parameters) stays open until M9.5 prototype evidence backs them.

> [!important] Out of scope right now
> M0..M5.9 stay comms-config-only. The full kernel lands at **new M9.5** between M9 (Dedicated Server) and M10 (LAN Co-op). Precursor hooks land in:
>
> - **M2** (Pixel Terrain): terrain heightmap exposed to comms LOS queries
> - **M4** (HUD): comms chip / radio HUD widget reserved
> - **M5** (Equipment): radio + antenna role records in equipment catalog
> - **M5.7** (Hazard Package): acoustic-trauma affliction (deafness from blast)
> - **M5.10** (Environmental Conditions): EnvironmentSignal includes acoustic + EM + comms slices
> - **M6.6** (AI Environmental Competence): AI subscribes to radio chatter
>
> M9.5 lands the kernel + audio reconstruction + per-band profiles + multiplayer voice routing.

## Why This Page Exists

There is no canonical contract for voice + radio in the vault today. DR-020 (audio identity) commits to "diegetic industrial synth-dread" + captions + audio-as-tactical-UI but doesn't define how voice propagates or how radio works. The user's directive (2026-05-06) raises this to a launch-grade simulation surface:

- Voice bounces off walls; sounds redirected; players don't hear as well at distance
- Radio: hills + materials attenuate; different antennas + frequency profiles; HAM vs military rifleman radio; real static / interference
- Robots have built-in radio; androids modular; humans need equipped radio (slot-occupying)

This page locks the contract. Reference research:

- **ACRE2** (IDI-Systems) — Arma 3 realistic radio mod. Locked propagation model: free-space path loss + multipath terrain + antenna direction + frequency tuning. Source: [[references/sources#Comms research (voice + radio simulation, 2026-05-06)|sources ledger]].
- **TFAR** (Task Force Arrowhead Radio) — TeamSpeak integration with SR (rifleman / squad), LR (long range / vehicle), PRR (personal role) radio classes.
- **Steam Audio** (Valve, Apache-2.0) — geometry-based acoustic propagation: occlusion, transmission, reflection, baked + raytraced reverb. Reference for our acoustic model.
- **Real military radios**: AN/PRC-152 (30-512 MHz multiband), AN/PRC-148 MBITR (VHF/UHF), SINCGARS (VHF FM 30-87.975 MHz, frequency-hopping), Personal Role Radio (low-power short-range).
- **Real HAM radio**: HF (160m-10m), VHF (6m + 2m), UHF (70cm), microwave (23cm+); each band has propagation characteristics.

## Principles (locked)

1. **Voice and radio are SIMULATED, not abstracted.** Voice propagates through actual atmosphere; radio signals propagate through actual terrain + materials. No "magic global team chat" by default in realistic modes.
2. **Vacuum = no voice.** Sound needs a medium. In a vacuum scenario (Moon / Mimas / Phobos), the only way to hear another actor is via radio.
3. **Walls + materials attenuate sound.** Concrete > drywall > foliage. Doors that are sealed block sound; cracked doors leak. Hooks into [[spec/atmospherics-and-chemistry-model]] room model.
4. **Reverb is real.** Indoor spaces (bunkers, corridors, caves) reverberate; outdoor open ground does not. Steam Audio-style raytraced reverb at launch (post-launch could move to wave-based for indoor accuracy).
5. **Radio propagation is multipath.** Per ACRE2: signal pathways bounce off terrain heightmap; constructive + destructive interference; frequency × antenna gain × LOS = received signal strength.
6. **Audio reconstruction is real.** Voice received on radio is **band-limited**, **compressed**, **noise-gated**, **static-mixed at low SNR**, and **distorted at very low SNR**. Different bands sound different (HF SSB hissy, VHF FM cleaner, UHF FM clearer, microwave near-perfect).
7. **Origin gating is locked.** Humans equip radios (slot-occupying). Robots have built-in radio (powered by `power` per [[spec/origin-reaction-and-resource-model]]). Androids may be built-in OR modular-upgrade (doesn't take an equipment slot, powered by android battery).
8. **AI hears the radio.** AI agents subscribed to a frequency hear chatter and incorporate it into doctrine reasoning. AI commander uses radio to coordinate. Going-dark = tactical mute.
9. **Server-authoritative voice routing.** In multiplayer, voice + radio routing happens on `cf-server`; clients receive band-limited audio streams; anti-cheat enforces transmission rules.
10. **Accessibility floor (DR-012).** Captions for every voice + radio transmission. Visual indicators for transmission active / signal strength / band tuning. Text-only chat fallback always available. Optional "narrator" voice for AI dialogue.

## Acoustic Propagation Model (Voice)

Voice = sound from an actor's mouth or speaker propagating through the local medium.

```text
struct AcousticSource {
    position: Vec2,
    speaker_id: ActorId,
    base_loudness_db: f32,                 // 60 dB normal speech; 80 dB shout; 30 dB whisper
    spectrum: SpectralContent,              // for filter / band-limit
    sealed_in_helmet: bool,                 // sealed helmet attenuates source by ~30 dB before it leaves
}

struct AcousticReceiver {
    position: Vec2,
    listener_id: ActorId,
    sealed_in_helmet: bool,                 // sealed helmet attenuates incoming by ~25 dB
    hearing_damage_factor: f32,             // 0.0..1.0; 0 = full hearing; 1 = deaf (per [[spec/body-damage-model]])
    is_robot: bool,                         // robots may have wider/narrower spectrum response
}

fn propagate_acoustic(src: AcousticSource, dst: AcousticReceiver, medium: PropagationMedium) -> ReceivedSound {
    let distance = (src.position - dst.position).length();

    // 1. Vacuum check
    if medium == Vacuum {
        return ReceivedSound::silent("no_medium");
    }

    // 2. Free-field attenuation (inverse square law)
    let mut attenuation_db = 20.0 * log10(distance);

    // 3. Atmospheric absorption (high-frequency rolloff; depends on humidity / temperature / composition)
    attenuation_db += atmospheric_absorption(medium, distance, temperature, humidity);

    // 4. Occlusion (walls, doors, terrain) — Steam Audio-style raytraced
    let occlusion_db = occlusion_query(src.position, dst.position, scene_geometry);
    attenuation_db += occlusion_db;

    // 5. Reverb (Steam Audio-style raytraced reflection paths)
    let reverb = reverb_query(src.position, dst.position, scene_geometry);

    // 6. Helmet attenuation
    if src.sealed_in_helmet  { attenuation_db += 30.0; }
    if dst.sealed_in_helmet  { attenuation_db += 25.0; }

    // 7. Hearing damage
    let effective_loudness = src.base_loudness_db - attenuation_db;
    let hearing_threshold = 0.0 + dst.hearing_damage_factor * 60.0;  // damaged hearing raises threshold

    if effective_loudness < hearing_threshold {
        return ReceivedSound::silent("below_threshold");
    }

    ReceivedSound {
        loudness_db: effective_loudness,
        reverb,
        spectral_attenuation: ...,
        direction_of_arrival: ...,
    }
}
```

Speed of sound in medium:

| Medium | Speed (m/s) | Notes |
|---|---|---|
| Air at 20 °C | 343 | Default Earth-surface ambient. |
| Hot air (Vulcan-surface 400-938 K) | 400-580 | Higher temperature → faster sound. |
| Cold air (Mars-night 220 K) | 295 | Lower. |
| Liquid water | 1480 | Underwater scenarios (post-launch). |
| Vacuum | n/a (no propagation) | Voice doesn't carry. |
| Helium-rich atmosphere | 970 | "Donald Duck" pitch shift effect (gameplay flavor for helium-mix airlock). |

## Radio Propagation Model

Radio = RF signal from a transmitter antenna through space to a receiver antenna. Per ACRE2's locked model:

```text
struct RadioTransmitter {
    radio_id: RadioId,
    owner_id: ActorId,
    antenna: AntennaSpec,                  // type, gain pattern, direction
    frequency_hz: f64,                     // tuned channel
    power_w: f32,                           // transmit power (5W rifleman, 25W squad, 50W vehicle, 1kW dish)
    encryption: EncryptionState,           // None | Symmetric(KeyId) | FrequencyHopping(NetId)
    sidetone: bool,                         // hear yourself in own headset
}

struct RadioReceiver {
    radio_id: RadioId,
    owner_id: ActorId,
    antenna: AntennaSpec,
    frequency_hz: f64,                     // tuned channel
    sensitivity_dbm: f32,                   // -110 dBm typical
    encryption_keys: Vec<KeyId>,            // accepted keys
    band_limit_hz: (f32, f32),             // e.g. (300, 3000) for voice radios
    static_threshold_dbm: f32,              // SNR below which static dominates audio
}

fn propagate_radio(tx: RadioTransmitter, rx: RadioReceiver, scene: SceneGeometry, world: WorldRef) -> ReceivedSignal {
    // 1. Frequency match check
    if !frequencies_match(tx.frequency_hz, rx.frequency_hz, tx.encryption) {
        return ReceivedSignal::silent("frequency_mismatch");
    }

    // 2. Free-space path loss (FSPL)
    let distance = (tx.position - rx.position).length();
    let fspl_db = 20.0 * log10(distance) + 20.0 * log10(tx.frequency_hz) + FSPL_CONST;

    // 3. Antenna gain (transmitter)
    let gain_tx_dbi = antenna_gain_for_direction(tx.antenna, direction_to(rx.position), tx.frequency_hz);

    // 4. Multipath terrain (ACRE2 locked model)
    let terrain_paths = compute_multipath_terrain(tx.position, rx.position, world.terrain_heightmap);
    let multipath_loss_db = sum_constructive_destructive(terrain_paths, tx.frequency_hz);

    // 5. Material attenuation (walls, hills, foliage)
    let material_loss_db = scene.compute_material_attenuation_along_path(tx.position, rx.position, tx.frequency_hz);

    // 6. Atmospheric absorption (only at microwave; HF/VHF/UHF mostly negligible)
    let atmos_abs_db = atmospheric_absorption_for_radio(tx.frequency_hz, distance, world.atmosphere_ambient);

    // 7. Antenna gain (receiver)
    let gain_rx_dbi = antenna_gain_for_direction(rx.antenna, direction_from(tx.position), rx.frequency_hz);

    // 8. Ionospheric bounce (HF only; world-dependent — Earth has it, Mars almost none, Moon none)
    let iono_path_db = if tx.frequency_hz < 30e6 && world.has_ionosphere {
        ionospheric_skip_loss(tx.position, rx.position, tx.frequency_hz, world.ionosphere_state)
    } else { f32::INFINITY };  // no ionospheric path; LOS only

    // 9. EM noise / interference (from EnvironmentSignal.em.em_noise_db; solar flares, EMP)
    let noise_floor_dbm = base_noise_floor_dbm + em_environmental_noise_db;

    // Net received signal strength
    let received_dbm = tx.power_w_to_dbm() + gain_tx_dbi - fspl_db
                       - multipath_loss_db - material_loss_db - atmos_abs_db
                       + gain_rx_dbi
                       .min_path(iono_path_db);

    let snr_db = received_dbm - noise_floor_dbm;

    if received_dbm < rx.sensitivity_dbm {
        return ReceivedSignal::silent("below_sensitivity");
    }

    ReceivedSignal {
        snr_db,
        received_dbm,
        carries_voice: snr_db > rx.static_threshold_dbm,
        static_intensity_0_1: clamp01((rx.static_threshold_dbm + 10.0 - snr_db) / 20.0),
        path_kind: if iono_path_db < material_loss_db { PathKind::Ionospheric } else { PathKind::LineOfSight },
    }
}
```

Key constants:

- `FSPL_CONST = -147.55` (free-space path loss formula constant in dB·m/Hz unit form)
- Speed of light: 299,792,458 m/s

## Frequency Bands And Profiles

Locked launch band set with per-band characteristics:

| Band | Range | Range (typical) | Voice characteristic | Primary use | Penetration | Antenna size |
|---|---|---|---|---|---|---|
| **HF** (3-30 MHz) | Long range (over horizon via ionospheric skip on Earth-like worlds; LOS otherwise) | Earth: 1000+ km via skip; LOS-only 50 km. Mars: ~50 km LOS only. | Heavy compression; SSB sidebands; hissy; narrow band 300-3000 Hz; subject to fading. | Strategic / inter-shard / HAM long-range / over-horizon backup. | Best terrain penetration at low end (160m, 80m HAM); building penetration moderate. | Large (multi-meter dipole / loop / wire). |
| **VHF** (30-300 MHz) | Medium range, line-of-sight | 5-30 km LOS depending on power + antenna. | Cleaner FM voice; 25 kHz channel width; reliable. | Squad / platoon / vehicle / HAM 6m + 2m. SINCGARS lives here. | Limited terrain penetration (hills block); some building penetration. | Medium (whip / vertical / dipole; 0.5-2 m). |
| **UHF** (300 MHz - 3 GHz) | Short to medium, line-of-sight | 1-10 km LOS. | Clean FM voice; 12.5-25 kHz channels. | Squad / personal / public-safety / HAM 70cm. AN/PRC-148 / 152 multi-band. | Better building penetration than VHF; line-of-sight critical outdoors. | Small (10-30 cm). |
| **Microwave** (3-30 GHz) | Tight beam | LOS only; 10-100 km if perfect LOS dish-to-dish. | Near-perfect voice + data; high bandwidth. | Satellite uplink / dish-to-dish / data backbone / inter-base relay. | Blocked by ANY obstacle; rain/atmosphere absorption matters. | Dish (0.3-3 m). |

**HAM amateur bands (modder + civilian + roleplay):**

| HAM Band | Frequency | Notes |
|---|---|---|
| 160m | 1.8 MHz (HF) | Long-range night-time skip. |
| 80m | 3.5 MHz (HF) | Regional. |
| 40m | 7.0 MHz (HF) | Day + night skip. |
| 20m | 14.0 MHz (HF) | Worldwide skip. |
| 17m | 18.0 MHz (HF) | DX. |
| 15m | 21.0 MHz (HF) | DX. |
| 12m | 24.0 MHz (HF) | DX. |
| 10m | 28.0 MHz (HF) | Sporadic E. |
| 6m | 50 MHz (VHF) | "Magic band". |
| 2m | 144-148 MHz (VHF) | Most common; FM voice; repeaters. |
| 70cm | 430-440 MHz (UHF) | Common; smaller antennas. |
| 23cm | 1240-1300 MHz (microwave) | Specialty. |

## Radio Hardware Roster (Launch Set)

Mirrors ACRE2's military + amateur lineup, adapted to Corefall's setting (DR-016 frontier disaster-contract sci-fi). Fictional names where appropriate.

| Radio | Class | Band | Power | Range (LOS) | Origin compatibility | Inspired by |
|---|---|---|---|---|---|---|
| **PRR-Lite** | Personal Role Radio (PRR) | UHF | 0.5 W | 200-500 m | Human equip | AN/PRC-343 (squad PRR) |
| **Squad-Mk1** | Short-Range (SR) | VHF/UHF | 5 W | 2-5 km | Human equip; android modular | AN/PRC-148 MBITR / Harris RF-7800V |
| **Squad-Mk2** | Short-Range (SR) | VHF/UHF | 10 W | 5-15 km | Human equip; android modular | AN/PRC-152 |
| **LongHaul-AT** | Long-Range (LR), Vehicle-mounted | HF/VHF | 50 W | 50-500 km (via skip on Earth) | Vehicle / chassis-mounted | SINCGARS RT-1523 / AN/PRC-117F |
| **Dish-Beacon** | Microwave dish (stationary) | Microwave | 50 W tight beam | 10-100 km LOS dish-to-dish | Base module; any operator | Civilian satellite-uplink dish |
| **HAM-Field** | Civilian / amateur | HF/VHF/UHF (multiband) | 5-100 W | varies | Human equip; modder origin | Yaesu FT-857 / Icom IC-7300 type |
| **Ionopulse** (fictional) | Inter-shard / over-the-pole | HF | 100 W | inter-shard via portal anchor | Stationary | (Corefall lore) |
| **Robot-Internal** | Built-in chassis radio | UHF | 5 W | 2-5 km | Robot built-in | (per [[spec/origin-reaction-and-resource-model]]) |
| **Android-Module** | Modular synthetic-side radio | VHF/UHF | 5 W | 2-5 km | Android modular | (per [[spec/origin-reaction-and-resource-model]]) |

## Antenna Roster (Launch Set)

| Antenna | Pattern | Gain (dBi) | Bands | Notes |
|---|---|---|---|---|
| Whip vertical | Omnidirectional | 0-2 dBi | VHF/UHF | Standard squad antenna. Antenna direction matters per ACRE2 "relative to back". |
| Long whip | Omnidirectional | 2-4 dBi | HF | Long for HF wavelengths. |
| Dipole wire | Omnidirectional with nulls | 2 dBi | HF | Field-deployed wire; needs trees / anchors. |
| Yagi directional | Directional beam | 8-15 dBi | VHF/UHF | Aim at target; high gain in main lobe. |
| Microwave dish | Tight beam | 20-40 dBi | Microwave | Aim is critical; 1-3° beam width. |
| Helical / quadrifilar | Circular polarization | 4-8 dBi | UHF/microwave | Satellite uplink. |
| Ground-spike | Omnidirectional | 0 dBi | HF/VHF | Stationary; deployed once. |

Antenna direction is gameplay-relevant: per ACRE2, prone-vs-standing antenna alignment matters; manual / auto alignment toggle.

## Audio Reconstruction (How Radio Voice Sounds)

When a radio receiver picks up a voice transmission, the audio is reconstructed:

```text
fn reconstruct_radio_audio(received: ReceivedSignal, original_voice: VoiceSample) -> AudioOutput {
    let mut audio = original_voice;

    // 1. Band-limit to radio voice band (typical 300 Hz - 3 kHz)
    audio = bandpass(audio, rx.band_limit_hz);

    // 2. Compander (compression + expansion) per radio mode
    audio = compander(audio, radio.mode_compander_settings);

    // 3. Apply per-band EQ profile
    audio = eq_profile(audio, radio.band_profile);
    // HF SSB: heavy compression, low-end roll-off
    // VHF FM: cleaner, full voice range
    // UHF FM: similar VHF, sometimes brighter
    // Microwave: near-original (data-link quality)

    // 4. Mix in static gated by SNR
    let static_gain = received.static_intensity_0_1;
    audio = mix(audio, generate_static(received.path_kind), static_gain);

    // 5. Distortion at very low SNR
    if received.snr_db < radio.distortion_threshold {
        audio = soft_clip(audio, distortion_amount_from_snr(received.snr_db));
    }

    // 6. Squelch tail (crackling at end of transmission)
    audio = append_squelch_tail(audio, radio.squelch_profile);

    // 7. Sidetone if transmitting
    if tx.sidetone && self_is_transmitter {
        audio = mix_sidetone(audio, sidetone_level);
    }

    audio
}
```

## Origin Gating

Per [[spec/origin-reaction-and-resource-model]]:

| Origin | Radio access | How |
|---|---|---|
| **Human** | Equipped radio (occupies equipment slot) | Player chooses radio at loadout; uses suit power OR battery cell. |
| **Robot** | **Built-in radio** | Powered by chassis `power` resource. Frequency tuning via UI. Built-in antenna may be omnidirectional or chassis-shape-dependent. |
| **Android** | **Built-in OR modular upgrade** | Some android variants ship with built-in (default frequency-tuneable; powered by `battery_charge`). Modular upgrade adds the radio without taking an equipment slot. |
| Modder origin | Per modder spec | Schema declares radio access. |

Origin-gated equipment validation per [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]]: humans assigning a built-in-radio item slot rejects with `wrong_origin_for_equipment`.

## EnvironmentSignal Integration

Per [[spec/environmental-conditions-model]] the per-tick per-actor `EnvironmentSignal` includes:

- `acoustic`: propagation_medium, ambient_db, reverb_rt60_s, occlusion_db, derived_can_hear_voice_unaided
- `em`: em_noise_db, em_emp_recently — degrades radio reception
- `comms`: light_lag_to_command_anchor_s, active_radio_links

The kernel produces these slices once per tick; AI / HUD / audio mixer / accessibility caption renderer all read the same slice.

## Run-Bundle Event Family Extensions

`voice` and `radio` event categories. Locked per DR-002.

| Event Type | Required Fields |
|---|---|
| `voice.transmission_started` | actor_id, source_pos, base_loudness_db, parent_event_id |
| `voice.transmission_received` | listener_id, source_pos, source_actor_id, effective_loudness_db, occlusion_db, reverb_rt60, parent_event_id |
| `voice.transmission_blocked` | listener_id, source_actor_id, reason (vacuum / sealed_helmet / hearing_damage / below_threshold), parent_event_id |
| `voice.shouted` | actor_id, content_hash | (sparse; for AI affordances) |
| `radio.tuned` | actor_id, radio_id, old_frequency, new_frequency |
| `radio.transmission_started` | tx_actor_id, radio_id, frequency_hz, power_w, parent_event_id |
| `radio.transmission_received` | rx_actor_id, tx_actor_id, radio_id, snr_db, path_kind, parent_event_id |
| `radio.transmission_blocked` | rx_actor_id, reason (frequency_mismatch / below_sensitivity / encryption_mismatch / em_disrupted), parent_event_id |
| `radio.encryption_changed` | actor_id, radio_id, old_state, new_state |
| `radio.antenna_alignment_changed` | actor_id, radio_id, old_align, new_align |
| `radio.interference_event` | em_source_id, affected_radios, snr_drop_db, parent_event_id |
| `comms.captioned` | listener_id, caption_text_id, source_kind (voice / radio), accessibility_consumer | (DR-012 caption pipeline) |

## AI Doctrine Integration (M6.6 promoted to AI Environmental Competence)

| AI Need | Reasoning |
|---|---|
| Hear teammate radio chatter | Subscribe to assigned frequency; if `radio.transmission_received` fires for our radio, parse + react. |
| Coordinate with squad | AI commander broadcasts orders on squad radio; AI subordinates parse + execute. |
| Switch to voice when in bunker | Indoor + close range = voice; outdoor or distance = radio. |
| Encrypt sensitive ops | Switch to encrypted channel for op-sec; reason label `comms_op_sec_required`. |
| Refuse mission if comms blackout | If `EnvironmentSignal.derived_hazards.contains(comms_blackout)` AND mission requires comms, AI refuses with `comms_required_unavailable`. |
| "Going dark" tactical mute | AI commander signals; squad mutes transmissions to avoid DF detection. |
| Direction-finding (DF) | If enabled in Match config, AI can DF enemy transmissions and route flanks. |
| Voice fallback if radio fails | If radio EMP-disrupted, AI falls back to voice if in range and atmosphere supports it. |

## Acceptance Tests (COMMS-A)

| Test | Setup | Pass Condition |
|---|---|---|
| COMMS-A-01 | Two actors in vacuum (Moon scenario), no radios. | Voice transmissions emit `voice.transmission_blocked{reason=vacuum}`. No audio output. |
| COMMS-A-02 | Two actors in vacuum with matched-frequency radios. | Voice transmits via radio; audio reconstruction includes static + band-limit; replay records full chain. |
| COMMS-A-03 | Indoor bunker, two actors at opposite ends of corridor. | Voice attenuates per Steam Audio occlusion + reverberates; if too far, replay records `voice.transmission_blocked{reason=below_threshold}`. |
| COMMS-A-04 | Outdoor scenario, two actors with VHF radios; hill between them. | Multipath terrain attenuates per ACRE2 model; signal weakens or breaks; if HF radio used instead with ionospheric world, signal may bounce around. |
| COMMS-A-05 | Solar flare event raises EM noise. | EnvironmentSignal.em.em_noise_db rises; received SNR drops; static increases; replay records `radio.interference_event`. |
| COMMS-A-06 | Robot uses built-in radio. | No equipment slot occupied; `power` resource ticks down per radio activity; transmission goes through. |
| COMMS-A-07 | Human attempts to use a robot's built-in radio item. | Slot-assign rejects with `wrong_origin_for_equipment`. |
| COMMS-A-08 | Android with modular radio upgrade vs without. | Without: needs equipped slot. With upgrade: built-in; doesn't take slot; powered by `battery_charge`. |
| COMMS-A-09 | Encrypted channel; one player has key, one does not. | Player without key receives `radio.transmission_blocked{reason=encryption_mismatch}`. |
| COMMS-A-10 | AI commander broadcasts attack order on squad radio. | All squad-frequency-tuned AI bots receive the order; replay records reception per actor. |
| COMMS-A-11 | EMP weapon detonates near a robot. | Robot's built-in radio temporarily disrupted; `radio.transmission_blocked{reason=em_disrupted}`; recovers after configured cooldown. |
| COMMS-A-12 | Determinism replay across full Bunker Defence with mixed voice + radio. | Same seed + same inputs = byte-identical event stream + audio reconstruction parameters. |
| COMMS-A-13 | Microwave dish-to-dish link; obstacle moves into beam. | Connection drops; replay records `radio.transmission_blocked{reason=below_sensitivity}`. |
| COMMS-A-14 | Caption coverage. | Every voice + radio transmission emits `comms.captioned`; HUD caption renderer consumes; accessibility test passes. |
| COMMS-A-15 | Cross-world latency. | Earth-Mars radio link respects astrography light-lag from [[spec/celestial-bodies-and-worlds-model]]. |

## Multiplayer Voice Routing

Per [[spec/server-app-architecture]] / [[spec/persistent-mmo-architecture]]:

- Voice + radio routing happens server-side on `cf-server`.
- Clients send raw voice samples (Opus-encoded; bandwidth-friendly) to server.
- Server runs propagation kernel; computes per-receiver received signals + audio reconstruction parameters.
- Server sends per-receiver mixed audio stream OR sends raw voice + reconstruction params for client-side reconstruction (lower bandwidth; enables cheating-resistance enforcement).
- Anti-cheat: clients can't bypass propagation rules to "hear everything"; server is authority.
- Comms policy from Match config (`Realistic` / `ProximityOnly` / `GlobalChat` / `CrossTeamDisabled`) determines server routing rules.

## Modding Contract

- Add a new radio: data row in `content/radios/<id>.radio.ron` with band, power, antenna compatibility, audio profile.
- Add a new antenna: data row in `content/antennas/<id>.antenna.ron` with pattern + gain curve.
- Add a new band profile: data row in `content/band_profiles/` with EQ + compander + static-character settings.
- Schema validates via `cargo run -p cf-mod -- validate content/radios/`.

## Performance Posture

- Voice propagation runs on Steam Audio / equivalent geometry-based engine; precomputed reverb where possible; raytraced occlusion + transmission per-frame for active sources.
- Radio propagation uses ACRE2-style multipath terrain: pre-bake LOS heightmap queries; sparse multipath samples; cached per-link.
- Per-tick: only active transmissions + active receivers are computed; sleeping when no one talks.
- Server multiplayer: voice routing is per-server-tick; bandwidth bounded by per-channel Opus stream count.

## Implementation Choices (Engine-Level)

| Layer | Choice | Notes |
|---|---|---|
| 3D positional voice | **Steam Audio** (Apache-2.0; Valve) wrapped in Rust binding | Geometry-based occlusion + transmission + reverb; production-quality. Fallback: bevy_oddio for simpler 3D positional + manual occlusion. |
| Audio middleware | **bevy_kira_audio** (Kira; Rust-native) OR **bevy_fmod** (FMOD wrapper) | Kira is Rust-native and Apache-2.0; FMOD is industry-standard but proprietary licensing. We default to Kira; FMOD as optional feature flag. |
| Voice codec | **Opus** (royalty-free; 6-510 kbps; speech-optimized) | Industry-standard; low-latency; Rust crate `opus`. |
| Radio multipath kernel | Custom Rust implementation in `cf-comms` per ACRE2 model | Use terrain heightmap from M2; per-frequency multipath sample count tunable. |
| TeamSpeak / Discord integration | NOT a launch dependency | The voice goes through the game server; no third-party voice service required. (Optional plugin post-launch for community-hosted shards.) |

## Out Of Scope (during M0..M9)

- M0..M5.9: scenario manifest may declare `comms_policy` placeholder; runtime no-op until M9.5.
- M2 (Pixel Terrain): heightmap exposed for future radio LOS queries.
- M4 (HUD): comms chip / radio HUD widget reserved with placeholder rendering.
- M5 (Equipment): radio + antenna role records ship; runtime is no-op.
- M5.7 (Hazard Package): acoustic-trauma affliction (`deafened_temp` / `deafened_perm`) added; ties to body damage.
- M5.10 (Environmental Conditions): EnvironmentSignal includes `acoustic` + `em` + `comms` slices producing placeholder values until M9.5.
- M6.6 (AI Environmental Competence): AI subscribes to radio chatter via stub; full integration at M9.5.
- M9 (Dedicated Server): server framework supports per-tick voice routing protocol but no kernel.
- M9.5 (NEW) lands the full kernel + audio reconstruction + per-band profiles + multiplayer voice routing + COMMS-A acceptance suite.
- M10 (LAN Co-op): voice usable in LAN.
- M11 (Online Co-op): voice + radio across hosts.
- M12 (PvP + MMO): full realistic comms + community-hosted shards.

## Source Trail

- [[spec/audio-identity]]
- [[spec/atmospherics-and-chemistry-model]]
- [[spec/celestial-bodies-and-worlds-model]]
- [[spec/environmental-conditions-model]]
- [[spec/origin-reaction-and-resource-model]]
- [[spec/equipment-loadout]]
- [[spec/full-collision-physics-plan]]
- [[spec/server-app-architecture]]
- [[spec/persistent-mmo-architecture]]
- [[spec/game-modes-and-match-grammar]]
- [[references/prototype-run-bundle-schema]]
- [[references/sources]] — see "Comms research" section
- [[decisions/dr-005-multiplayer-posture]]
- [[decisions/dr-008-ai-architecture]]
- [[decisions/dr-012-accessibility-comfort-readability]]
- [[decisions/dr-020-audio-identity]]
- [[decisions/dr-022-ai-humanlike-bar]]
- [[decisions/dr-027-combat-base-scope]]
- [[decisions/dr-034-dedicated-server-application]]
- [[decisions/dr-035-persistent-mmo-architecture]]
- [[decisions/dr-040-environmental-conditions-and-hazards-direction]]
- [[decisions/dr-042-game-modes-and-match-grammar-direction]]
- [[decisions/dr-043-voice-comms-and-radio-direction]]
- [[research-log/2026-05-06-celestial-bodies-environments-mining-bunker-defence-design-intent]]

## Change Log

- 2026-05-06: Captured during M1 from user-supplied design intent ("full fledged comm system in game (full environmental influence on the voice, inside, outside, voice bounces of walls...) also full Radio system, where hills, material in between, type of radio (different antennas, etc, different frequency profiles, like real life, HAM radio, military rifleman radio, etc... humans need a radio, but robots, and sometimes androids have it built in"). Status: `design-intent-post-m1`. ACRE2 multipath model + Steam Audio acoustic model adopted as references. Lands at new M9.5; precursor hooks across M2 / M4 / M5 / M5.7 / M5.10 / M6.6.
