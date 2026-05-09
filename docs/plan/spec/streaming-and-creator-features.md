---
type: spec
status: closed-direction
authority: "Streaming + creator features: photo mode, replay sharing, highlight reel auto-detect, OBS overlay, Twitch integration, spectator mode, streamer mode (hidden enemy positions), press/influencer keys, AI-managed CRM."
ready_when: "Photo mode functional; replay sharing endpoint live; auto-highlight reel works; OBS overlay app installable; Twitch integration optional but functional; spectator multi-POV implemented."
feeds:
  - DR-002
  - DR-005
  - DR-019
  - DR-024
  - DR-031
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[spec/replay-recorder-slice-a|replay]] · [[spec/server-app-architecture|server app]]

# Streaming & Creator Features

## Photo Mode

- Free camera (full 6DOF in 2D scene; pan + zoom + rotate)
- Freeze sim (no time progression)
- Filter presets:
  - Comic-noir
  - Pixel-pure
  - Dramatic-light
  - Tactical-overlay (shows AI labels, threat zones, capabilities)
  - Cinematic (depth-of-field + chromatic aberration)
- Screenshot export with optional credits stamp
- Animation export (4-8s GIF / WEBM)
- Per-shot EXIF (mission, scene, time of day, settings)

## Replay Viewer

Per [[spec/replay-recorder-slice-a]] + DR-002. Beyond M3 baseline:

- Scrub control (frame-by-frame; second; minute; per-event)
- Speed control (0.1x to 10x)
- Multi-camera view (player POV, commander map, first-person, spectator)
- Bookmark + named timestamp
- Clip export (5-30s MP4 with audio)
- Shareable link (uploads to community replay-share endpoint)
- Comments / annotations on shared replays

## Auto-Highlight Reel

Auto-detect interesting moments:

- Kills (especially multi-kill, headshot, long-range)
- Narrow escapes (HP <10% then survived)
- Base breaches (player breached enemy bunker)
- Reactor breaches
- Command core uproot
- Mech eject (successful)
- Veteran death (named operative)
- Underdog victory (last man standing)

Each highlighted moment becomes a 5-15s clip with auto-edit (slow-mo on impact, music sting, faction-tinted color grade).

## OBS Overlay (Streamer Companion)

Standalone Rust app `cf-obs-overlay`:

- Live match state for stream overlay (HP, ammo, objectives, kills, deaths)
- Per-streamer customizable layout
- Theme presets (faction-tinted)
- Twitch chat integration (chat reads on overlay)
- Donation/sub alerts
- Free download from Steam page + GitHub

## Twitch Integration

Optional. Twitch chat → in-game commands during streamer's matches:

- Vote on AI doctrine (cautious / aggressive / mixed)
- Vote on next match seed
- Vote on dropship LZ
- Send tactical advice (banner text)

Configurable per streamer. Optional.

## Spectator Mode

Per DR-005 + M9. Multi-POV. Free camera. Replay-scrub during live match.

- Following a player
- Free roam
- Commander map view
- Multi-window (PiP)
- Late-join
- Tournament/match host mode

## Streamer Mode

Hides enemy positions for delayed streams. Toggleable in settings. Useful for streamers playing PvP delayed broadcast.

## Press / Influencer Keys

- Time-limited Steam keys for press + content creators pre-launch.
- AI-managed CRM (one-button issue + revoke + track-coverage).
- Per-recipient keys; revoke if leaked.
- AI-generated personalized outreach emails.

## Replay Sharing

- Upload replay to community server (cf-server hosts the replay-share endpoint)
- Sharable link
- Embeds in Discord, Reddit, Twitter
- Replay metadata (mission, mode, faction, players)
- Comments
- "Of the day" curated by community

## Done-Criteria

- [ ] Photo mode functional.
- [ ] Replay sharing endpoint live.
- [ ] Auto-highlight reel works (8+ trigger types).
- [ ] OBS overlay app installable + customizable.
- [ ] Twitch integration optional but functional.
- [ ] Spectator multi-POV.
- [ ] Streamer mode hides enemy positions.
- [ ] Press/influencer keys CRM functional.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- [[spec/replay-recorder-slice-a]]
- [[spec/server-app-architecture]]
