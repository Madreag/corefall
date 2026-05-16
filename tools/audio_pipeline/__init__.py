"""cf-audio-pipeline — Tier 2 audio bake orchestrator.

Sub-modules:
- keys             — API key loader (~/.config/cf-audio/*.toml or env vars)
- post_process     — trim/fade/normalize/loop-align on every baked WAV
- ledger_supersede — Tier 1 → Tier 2 ledger replace
- aiva_capture     — Playwright headed login → save session state
- aiva_bake        — Playwright headless bake loop (120 music tracks)
- eleven_voice_design — 35 voice designs via ElevenLabs Voice Design
- eleven_voice_lines  — 242 voice line bakes via Multilingual v2 + Flash v2.5
- eleven_sfx          — 242 SFX bakes via ElevenLabs SFX v2
- eleven_music        — fallback music bake via ElevenLabs Music v1
- cli              — entry point orchestrator
"""
