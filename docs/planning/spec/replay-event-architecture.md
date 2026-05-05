---
type: spec
status: stub
ready_when: "Recorder + viewer prototype runs against a 5-minute battle."
---

← [[spec/index|spec section]] · [[spec/replay-recorder-slice-a|recorder Slice A]] · [[systems/replay-event-architecture|replay/event brief]] · [[decisions/dr-002-replay-event-architecture|DR-002]]

# Replay/Event Architecture

> [!warning] Stub

## What goes here when ready

- Event taxonomy (combat, body, terrain, AI, logistics, mission).
- Snapshot cadence and storage.
- Replay file format and migration policy.
- Networking implications (event broadcast for co-op).
- Player-facing surfaces (death recap, mission recap, AI debug).

## Current Build Checklist

The implementation-facing first slice is [[spec/replay-recorder-slice-a]]. Keep this page as the curated future spec stub, and promote only proven recorder/viewer behavior back here after the Slice A acceptance tests pass.

## Inputs

- [[systems/replay-event-architecture]]
- [[spec/replay-recorder-slice-a]]
- [[engine/network-terrain-replication-lifecycle]]
- [[systems/ai-trust-test-suite]]
- [[decisions/dr-002-replay-event-architecture]]
