"""M12A § Tier 1 Audio Pipeline — LLM-driven SFX generation.

This package is the spec-canonical home for the M12A audio pipeline. It
provides:

- `generate_sfx` — the main orchestrator (reads sfx_manifest entries +
  drives a chosen adapter + ledgers + emits captions).
- Adapter trait + four concrete adapters (stable_audio, audiocraft,
  openai_audio, elevenlabs_sfx).
- `envelope_shaper` — per-SFX attack/decay/sustain/release shaping;
  trims silence; normalizes loudness per EBU R 128.
- `caption_authorer` — LLM-authored caption template per SFX from the
  manifest entry.
- `ledger_writer` — writes via cf-asset-ledger.
- `audio_palettes/` — per-faction, per-material, per-origin audio
  signatures consumed by the adapters' prompt-shaping pass.
- `caption_templates.ron` — central caption-template registry hydrated
  by `cf-audio::caption_bridge::CaptionRegistry` at runtime.
- `sfx_manifest.ron` — the M49-launch roster of 1200+ SFX entries.

The package is intentionally thin — it composes the existing
`tools/audio_synth/` procedural Tier 1 synth + `tools/audio_pipeline/`
ElevenLabs Tier 2 upgrade paths. The two prior tools become the
underlying adapters for the spec-canonical pipeline.
"""
__all__ = [
    "generate_sfx",
    "stable_audio_adapter",
    "audiocraft_adapter",
    "openai_audio_adapter",
    "elevenlabs_sfx_adapter",
    "envelope_shaper",
    "caption_authorer",
    "ledger_writer",
]
