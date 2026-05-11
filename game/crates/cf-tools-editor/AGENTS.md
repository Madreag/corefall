# cf-tools-editor — AGENTS.md

## Owns
- In-engine scenario / package / mod editors (real implementation pending M8).
- First-class editor at launch per the roadmap.
- Same typed manifest as engine + director + procedural + player-authored.

## Public API Boundary
- (Stub. 38-line scaffold.)

## Does NOT Own
- Scenario loading → `cf-control::scenario`.
- Content validation → `cf-mod`.
- Replay viewing → `cf-tools-replay-viewer`.

## Test Surface
- (Stub.) Real coverage lands at M8.

## Source Trail
- DR-030 (scenario editor commitment; CLOSED direction).
