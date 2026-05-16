"""ElevenLabs Voice Design bake — 35 named voices for Corefall.

Reads the unique `voice_id` set from the four voice prompt manifests
(`game/content/sfx/voice_*_prompts.json`), builds a rich `voice_description`
per ID using faction/role/trait heuristics, calls `text_to_voice.design`
to get three previews per voice, auto-picks the first preview, mints the
voice via `text_to_voice.create`, and persists the mapping to
`tools/audio_pipeline/voice_synthesis/per_npc_voice_registry.toml`.

The TOML registry survives across sessions and is committed to git. Voice IDs
themselves are NOT secrets, but the underlying prompts that produced them are
deterministic (use --reset-registry only when rebuilding from scratch).

Usage:
    python eleven_voice_design.py --dry-run
    python eleven_voice_design.py
    python eleven_voice_design.py --only coalition_marcus_authoritative_male
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path

from elevenlabs.client import ElevenLabs

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from keys import load_elevenlabs_key  # noqa: E402

REPO_ROOT = _HERE.parents[1]
SFX_DIR = REPO_ROOT / "game" / "content" / "sfx"
REGISTRY_DIR = _HERE / "voice_synthesis"
REGISTRY_PATH = REGISTRY_DIR / "per_npc_voice_registry.toml"

VOICE_MODEL = "eleven_ttv_v3"  # latest Voice Design model (highest quality, 2026)

# ─── Faction → voice persona modifier ──────────────────────────────────────
FACTION_PERSONA: dict[str, str] = {
    "coalition": (
        "disciplined modern military officer voice. clipped, authoritative, "
        "tactical bearing. used to giving orders over a radio. mid-Atlantic "
        "American accent. measured pace. minimal emotion except when tactical "
        "situation degrades."
    ),
    "frontier": (
        "rough, weathered settler voice. low-class American Western/Australian "
        "outback hybrid accent. independent, defiant, suspicious of central "
        "authority. dry humor. comfortable swearing softly. unhurried but "
        "decisive."
    ),
    "ronin": (
        "sparse, contemplative blade-for-hire voice. soft Japanese-American "
        "accent. neo-noir cyberpunk samurai aesthetic. economical with words, "
        "every syllable weighted. low register, smoky, faintly melancholic."
    ),
    "synth": (
        "synthetic neutral voice. monotone, slightly processed but not "
        "robotic-cartoon. machine-collective consensus speaker. precise, "
        "unhurried, faintly metallic resonance. genderless ideal. polite, "
        "informational delivery."
    ),
    "collective": (
        "industrial working-class voice. Eastern European / Slavic-inflected "
        "American English. gritty, hoarse from factory air, faintly sardonic. "
        "comfortable speaking through machinery noise. proletarian solidarity."
    ),
    "husks": (
        "distorted alien voice. unsettling, multi-layered whisper-and-roar "
        "blend. NOT human. faint chittering insectoid undertones. dispassionate "
        "but uncannily intimate. very short utterances. heavy reverb works."
    ),
    "collegium": (
        "scholarly monastic voice. measured, soft Oxford / Cambridge accent. "
        "careful enunciation, slightly archaic word choices. patient even "
        "under threat. occasional dry wit. comfortable in liturgical cadence."
    ),
    "starlight": (
        "ritualistic religious voice. fervent, ecstatic, slightly sing-song. "
        "Mediterranean / Spanish-inflected English. warm but uncompromising. "
        "speaks of the sun and the stars with reverence. capable of cold "
        "judgment when heretics are near."
    ),
}

# ─── Storyteller / boss / tutorial overrides ──────────────────────────────
SPECIAL_PERSONAS: dict[str, str] = {
    "cassandra_narrator_balanced_female": (
        "warm yet authoritative middle-aged female narrator. balanced, "
        "cinematic delivery. Helldivers-2-Eagle-1 meets calm BBC documentary. "
        "American accent. presents events neutrally without melodrama."
    ),
    "phoebe_narrator_warm_female": (
        "mellow warm young female narrator. friendly mentor energy. light "
        "Pacific Northwest American accent. comforting, encouraging, never "
        "patronizing. ideal for chill-mode storytelling."
    ),
    "randy_narrator_chaotic_male": (
        "manic gravelly male narrator. fast-talking, gleefully unhinged, "
        "carnival-barker energy with occasional dry-pause beats. faintly "
        "Brooklyn American accent. enjoys describing chaos."
    ),
    "ironman_narrator_grim_male": (
        "grim weathered older male narrator. Cormac-McCarthy-prose-aloud "
        "energy. American Midwest accent, low slow register. permadeath "
        "stakes told plainly without theatrics. quiet dread."
    ),
    "sandbox_narrator_observational_female": (
        "observational neutral female narrator. soft Scandinavian-inflected "
        "English. quietly curious, light, no-pressure delivery. nature "
        "documentary host energy applied to a sci-fi sandbox."
    ),
    "tutorial_narrator_calm_female": (
        "calm patient female tutorial narrator. neutral mid-Atlantic American "
        "accent. perfectly clear enunciation, friendly but professional. "
        "Apple Watch tutorial voice meets a thoughtful flight attendant."
    ),
    "hollow_king_boss_deep_male": (
        "deep regal commanding male boss voice. doom-metal-grim-warlord "
        "register. low chest resonance. Eastern European-inflected English. "
        "ancient flame king speaking from a throne of cooled magma."
    ),
    "frozen_heart_boss_cold_female": (
        "cold detached unsettling female boss voice. Slavic-inflected English. "
        "high airy register with cryogenic stillness. polite, almost gentle, "
        "while describing your incoming death."
    ),
    "crimson_tide_boss_distorted": (
        "distorted multi-voice male boss. Bedouin-warlord-channeled-through-"
        "static aesthetic. layered chant + sandstorm growl. NOT a single "
        "human voice. faint chant chorus underneath."
    ),
    "eclipse_walker_boss_synthetic": (
        "synthetic cyborg boss voice. Korean-inflected English. precise, "
        "philosophical, gently mocking. very lightly modulated, almost "
        "natural with subtle gravity-warp shimmer at sentence ends."
    ),
    "last_star_boss_ascendant": (
        "ascendant ethereal genderless boss voice. cathedral-organ resonance. "
        "American mid-Atlantic English with a faint choir layer. final "
        "campaign superboss; speaks as if from inside a star."
    ),
}


@dataclass
class VoicePlan:
    voice_id: str            # our internal canonical id (manifest key)
    description: str         # the prompt sent to ElevenLabs Voice Design
    sample_text: str         # ~50-200 char text for the previews
    seed: int                # deterministic-ish seed (best-effort across designs)
    line_count: int


def _scan_unique_voice_ids() -> dict[str, int]:
    """Return {voice_id: total_line_count_across_manifests}."""
    counters: dict[str, int] = {}
    for path, key in (
        (SFX_DIR / "voice_npc_prompts.json", "npc_voice_prompts"),
        (SFX_DIR / "voice_storyteller_boss_prompts.json", "storyteller_voice_prompts"),
        (SFX_DIR / "voice_storyteller_boss_prompts.json", "boss_voice_prompts"),
        (SFX_DIR / "voice_mission_tutorial_prompts.json", "mission_voice_prompts"),
        (SFX_DIR / "voice_mission_tutorial_prompts.json", "tutorial_voice_prompts"),
    ):
        if not path.exists():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for entry in data.get(key, []):
            vid = entry.get("voice_id")
            if vid:
                counters[vid] = counters.get(vid, 0) + 1
    return counters


def _persona_for(voice_id: str) -> str:
    if voice_id in SPECIAL_PERSONAS:
        return SPECIAL_PERSONAS[voice_id]
    parts = voice_id.split("_")
    faction = parts[0] if parts else ""
    base = FACTION_PERSONA.get(faction, "neutral cinematic voice")
    role_bits = parts[1:-1]
    trailing = parts[-1] if parts else ""
    role = " ".join(role_bits).replace("_", " ")
    suffix_bits = []
    if "male" == trailing:
        suffix_bits.append("male voice")
    elif "female" == trailing:
        suffix_bits.append("female voice")
    elif trailing in ("synthetic", "distorted", "neutral"):
        suffix_bits.append(f"{trailing} voice")
    if role:
        suffix_bits.append(f"role: {role}")
    voice_desc = base
    if suffix_bits:
        voice_desc = voice_desc + " " + " ".join(suffix_bits) + "."
    return voice_desc


def _sample_text_for(voice_id: str) -> str:
    """A 100-1000 char preview prompt exercising the voice's character.

    ElevenLabs Voice Design requires `text` to be 100-1000 chars; we keep
    samples close to 200-280 chars to give the model enough signal to converge.
    """
    if "boss" in voice_id:
        return (
            "I have watched your kind cross my borders for a thousand cycles. "
            "Each one believed they were different. None of them were. "
            "Lay down your weapons or do not. The outcome is already written."
        )
    if "tutorial" in voice_id or "narrator" in voice_id:
        return (
            "Aim with the right stick. Fire with the right trigger. Reload "
            "with X. The mech is yours. Walk it slowly until you trust it. "
            "The first hour you spend in the cockpit decides whether you "
            "survive the second."
        )
    if voice_id.startswith("husks_"):
        return (
            "Acceptable configuration. We logged your shape. We logged your "
            "frequencies. We logged your hesitation. Return now. The hive "
            "is patient. The hive is wide. The hive is already here."
        )
    if voice_id.startswith("synth_"):
        return (
            "Node response: query received and processed. Consensus is "
            "reached. Recommendation follows. Your previous deviation has "
            "been catalogued and forwarded to the relevant arbiter sub-process."
        )
    return (
        "Hold the line. We move on my mark. Three, two, one. Go. Watch the "
        "left flank, somebody is hugging that wall too well. Once we cross "
        "the kill zone the next breach is yours, and I do not want any "
        "improvising. Stay tight, stay quiet, stay alive."
    )


def _build_voice_plans(only: list[str] | None) -> list[VoicePlan]:
    counters = _scan_unique_voice_ids()
    plans: list[VoicePlan] = []
    for voice_id in sorted(counters.keys()):
        if only and voice_id not in only:
            continue
        plans.append(
            VoicePlan(
                voice_id=voice_id,
                description=_persona_for(voice_id),
                sample_text=_sample_text_for(voice_id),
                seed=abs(hash(voice_id)) & 0x7FFFFFFF,
                line_count=counters[voice_id],
            )
        )
    return plans


def _load_registry() -> dict[str, dict[str, str]]:
    if not REGISTRY_PATH.exists():
        return {}
    try:
        import tomllib
        with REGISTRY_PATH.open("rb") as f:
            return dict(tomllib.load(f))
    except Exception:
        return {}


def _save_registry(reg: dict[str, dict[str, str]]) -> None:
    REGISTRY_DIR.mkdir(parents=True, exist_ok=True)
    lines: list[str] = [
        "# Corefall voice registry — generated by eleven_voice_design.py.",
        "# Each entry maps an internal voice_id (from voice prompt manifests)",
        "# to a real ElevenLabs voice_id minted via Voice Design + create.",
        "# Re-run eleven_voice_design.py to refresh.",
        "",
    ]
    for vid in sorted(reg.keys()):
        e = reg[vid]
        lines.append(f"[{vid}]")
        for key in ("elevenlabs_voice_id", "model_id", "description", "sample_text", "designed_at"):
            val = e.get(key, "")
            esc = val.replace("\\", "\\\\").replace("\"", "\\\"")
            lines.append(f'{key} = "{esc}"')
        lines.append("")
    REGISTRY_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true",
                    help="Print plans without calling ElevenLabs.")
    ap.add_argument("--only", default=None,
                    help="Comma-separated voice_ids to design (for retries).")
    ap.add_argument("--reset-registry", action="store_true",
                    help="Wipe per_npc_voice_registry.toml first (re-design all).")
    ap.add_argument("--inter-call-sleep", type=float, default=1.0,
                    help="Seconds to sleep between API calls (default 1.0).")
    args = ap.parse_args()

    only = (args.only.split(",") if args.only else None)
    plans = _build_voice_plans(only)
    print(f"[design] voice plans = {len(plans)}")

    if args.reset_registry and REGISTRY_PATH.exists():
        REGISTRY_PATH.unlink(missing_ok=True)
    registry = _load_registry()

    if args.dry_run:
        for p in plans:
            preview = p.description[:78].replace("\n", " ")
            print(f"[dry] {p.voice_id:<48s} lines={p.line_count:<3d} :: {preview}…")
        return 0

    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    print(f"[design] client ready ({key!r})")

    for i, plan in enumerate(plans, start=1):
        if plan.voice_id in registry and registry[plan.voice_id].get("elevenlabs_voice_id"):
            print(f"[design] SKIP {plan.voice_id} (already in registry)")
            continue
        print(f"[design] {i}/{len(plans)} {plan.voice_id} → designing previews…")
        # Per-model parameter selection (eleven_ttv_v3 vs eleven_multilingual_ttv_v2):
        # - quality: only valid on multilingual_ttv_v2
        # - prompt_strength: only valid on ttv_v3
        design_kwargs = dict(
            voice_description=plan.description,
            text=plan.sample_text,
            model_id=VOICE_MODEL,
            seed=plan.seed,
            guidance_scale=12.0,
            stream_previews=False,
            output_format="mp3_44100_192",
        )
        if VOICE_MODEL == "eleven_multilingual_ttv_v2":
            design_kwargs["quality"] = 0.9
        # eleven_ttv_v3: no quality, no prompt_strength (latter requires reference_audio).
        # Voice character is fully driven by voice_description + guidance_scale.

        try:
            previews = client.text_to_voice.design(**design_kwargs)
        except Exception as exc:
            print(f"[design] FAIL design {plan.voice_id}: {exc}")
            continue

        first_preview_id: str | None = None
        all_preview_ids: list[str] = []
        try:
            for prev in (previews.previews or []):  # type: ignore[union-attr]
                pid = getattr(prev, "generated_voice_id", None)
                if pid:
                    all_preview_ids.append(pid)
                    if first_preview_id is None:
                        first_preview_id = pid
        except Exception:
            pass

        if not first_preview_id:
            print(f"[design] FAIL {plan.voice_id} — no preview returned")
            continue

        try:
            played_not_selected = [pid for pid in all_preview_ids if pid != first_preview_id]
            voice = client.text_to_voice.create(
                voice_name=plan.voice_id,
                voice_description=plan.description,
                generated_voice_id=first_preview_id,
                played_not_selected_voice_ids=played_not_selected,
            )
            real_voice_id = getattr(voice, "voice_id", None)
        except Exception as exc:
            print(f"[design] FAIL create {plan.voice_id}: {exc}")
            continue

        if not real_voice_id:
            print(f"[design] FAIL {plan.voice_id} — create returned no voice_id")
            continue

        registry[plan.voice_id] = {
            "elevenlabs_voice_id": real_voice_id,
            "model_id": VOICE_MODEL,
            "description": plan.description,
            "sample_text": plan.sample_text,
            "designed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        _save_registry(registry)
        print(f"[design] OK   {plan.voice_id} → {real_voice_id}")
        time.sleep(args.inter_call_sleep)

    print(f"[design] DONE — {len(registry)} voices in registry")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
