# cf-audio — AGENTS.md

## Owns
- Sound playback + spatial audio (real implementation pending BP6 / DR-020 + DR-053).
- Diegetic-first audio mix with captions.
- AI-authored audio pipeline (DR-053).
- ACRE2-tier radio + Steam Audio-tier voice (M9.5).

## Public API Boundary
- (Stub until BP6.)

## Does NOT Own
- Caption text rendering → `cf-ui`.
- Caption accessibility → `cf-control` (observe.captions).

## Test Surface
- (Stub.) Real coverage lands at BP6.

## Source Trail
- DR-020 (audio identity; OPEN).
- DR-053 (AI audio pipeline; OPEN).
