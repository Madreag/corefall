"""M12A § Caption authorer — generate caption template + severity + categories
from a SfxManifestEntry. Per spec § Caption authoring (ACC-A integration):

>   Input to caption_authorer.py:
>     - sfx_id: "weapon_fire_iron_rifle_single"
>     - category: WeaponFire
>     - manifest_prompt: "Industrial rifle gunshot..."
>   LLM output:
>     - caption_template: "GUNSHOT — {direction} ({weapon_kind})"
>     - caption_severity: info
>     - caption_categories: [combat]
>     - per-language strings (M38A produces translations later)

The default authoring is RULE-BASED, not LLM-based — categories are
inferred deterministically from the SFX id prefix. The LLM path is
optional and gated by the `--llm-author` flag on `generate_sfx.py`;
without it, the rule-based defaults emit per-spec captions for every
SFX. M38A localizes per-language strings.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class AuthoredCaption:
    """One authored caption template entry."""

    sfx_id: str
    template: str
    severity: str  # "critical" | "warning" | "info"
    categories: list[str]


_PREFIX_RULES: list[tuple[str, str, str, str]] = [
    # (id-prefix-startswith, template, severity, categories)
    ("sfx_pistol_fire", "GUNSHOT — {direction} (pistol)", "info", "combat"),
    ("sfx_smg", "GUNSHOT — {direction} (SMG)", "info", "combat"),
    ("sfx_rifle", "GUNSHOT — {direction} (rifle)", "info", "combat"),
    ("sfx_sniper", "GUNSHOT — {direction} (sniper)", "info", "combat"),
    ("sfx_shotgun", "SHOTGUN — {direction}", "info", "combat"),
    ("sfx_gl", "GRENADE LAUNCHER — {direction}", "warning", "combat"),
    ("sfx_heavy", "HEAVY WEAPON — {direction}", "warning", "combat"),
    ("sfx_flamer", "FLAMER — {direction}", "warning", "combat"),
    ("sfx_drill", "DRILL ACTIVATED — {direction}", "info", "combat"),
    ("sfx_grappler", "GRAPPLE — {direction}", "info", "combat"),
    ("sfx_drone", "DRONE — {direction}", "info", "ai"),
    ("sfx_melee", "MELEE — {direction}", "warning", "combat"),
    ("sfx_weapon_reload", "RELOAD click", "info", "combat"),
    ("sfx_weapon_jam", "WEAPON JAM", "warning", "combat"),
    ("sfx_weapon_swap", "WEAPON SWAP whoosh", "info", "combat"),
    ("sfx_weapon_mag", "MAGAZINE drop", "info", "combat"),
    ("sfx_weapon_shell", "SHELL ejected", "info", "combat"),
    ("footstep_", "FOOTSTEP — {direction}", "info", "combat"),
    ("locomotion_", "LOCOMOTION — {direction}", "info", "combat"),
    ("impact_concrete", "IMPACT — concrete ({direction})", "info", "combat"),
    ("impact_metal", "IMPACT — metal ({direction})", "info", "combat"),
    ("impact_wood", "IMPACT — wood ({direction})", "info", "combat"),
    ("impact_glass", "GLASS shatters", "warning", "combat"),
    ("impact_dirt", "IMPACT — dirt ({direction})", "info", "combat"),
    ("impact_sand", "IMPACT — sand ({direction})", "info", "combat"),
    ("impact_ice", "IMPACT — ice ({direction})", "info", "combat"),
    ("impact_water", "IMPACT — water splash", "info", "combat"),
    ("impact_", "IMPACT — {direction}", "info", "combat"),
    ("sfx_projectile", "ROUND zips by — {direction}", "info", "combat"),
    ("sfx_body_hit", "BODY HIT — {direction}", "critical", "combat"),
    ("sfx_dismember", "DISMEMBERMENT — {direction}", "critical", "combat"),
    ("sfx_death", "DEATH — {direction}", "critical", "combat"),
    ("sfx_grenade", "GRENADE — {direction}", "warning", "combat"),
    ("sfx_explosion", "EXPLOSION — {direction}", "critical", "combat"),
    ("sfx_atmos_breach", "BREACH — {direction}", "critical", "system"),
    ("sfx_atmos", "ATMOSPHERIC — {direction}", "info", "system"),
    ("sfx_hazard_fire", "FIRE crackle — {direction}", "warning", "system"),
    ("sfx_hazard_electric", "ELECTRIC arc — {direction}", "warning", "system"),
    ("sfx_hazard_acid", "ACID hiss — {direction}", "warning", "system"),
    ("sfx_hazard", "HAZARD — {direction}", "warning", "system"),
    ("sfx_chassis", "CHASSIS — {direction}", "info", "system"),
    ("sfx_fluid", "FLUID — {direction}", "info", "system"),
    ("sfx_power", "POWER — {direction}", "info", "system"),
    ("sfx_crafting", "CRAFTING — {direction}", "info", "system"),
    ("sfx_voice_grunt", "GRUNT — {direction}", "info", "ai"),
    ("sfx_pet", "PET — {direction}", "info", "ai"),
    ("sfx_storyteller", "EVENT — {direction}", "info", "mission"),
    ("sfx_reactor_pressure", "REACTOR — {state}", "warning", "mission"),
    ("sfx_mission_start", "MISSION START", "info", "mission"),
    ("sfx_mission_objective", "OBJECTIVE — {state}", "info", "mission"),
    ("sfx_mission_win", "MISSION WIN", "info", "mission"),
    ("sfx_mission_loss", "MISSION LOSS", "critical", "mission"),
    ("sfx_banner_critical", "BANNER — critical alert", "critical", "system"),
    ("sfx_banner_warning", "BANNER — warning", "warning", "system"),
    ("sfx_banner_info", "BANNER — info", "info", "system"),
    ("sfx_ui", "UI {action}", "info", "system"),
    ("sfx_ambient_", "AMBIENT", "info", "system"),
    ("sfx_weather_", "WEATHER", "info", "system"),
    ("sfx_ai_chatter_", "{actor} chatter", "info", "ai"),
]


def author_caption(sfx_id: str, manifest_prompt: str = "") -> AuthoredCaption:
    """Deterministically derive a caption template for `sfx_id`.

    Walks the rule prefix list in order; first match wins. Falls back to
    a generic template using the manifest prompt's first 30 chars.
    """
    _ = manifest_prompt
    for prefix, template, severity, category in _PREFIX_RULES:
        if sfx_id.startswith(prefix):
            return AuthoredCaption(
                sfx_id=sfx_id,
                template=template,
                severity=severity,
                categories=[category],
            )
    # Generic fallback — every SFX gets a caption per the M12A
    # acceptance criterion "Caption per SFX is mandatory".
    return AuthoredCaption(
        sfx_id=sfx_id,
        template=f"{sfx_id.upper()} — " + "{direction}",
        severity="info",
        categories=["system"],
    )


def author_caption_dict(sfx_id: str, manifest_prompt: str = "") -> dict[str, Any]:
    """Same as `author_caption` but returns a plain dict for JSON / RON
    serialization."""
    cap = author_caption(sfx_id, manifest_prompt)
    return {
        "sfx_id": cap.sfx_id,
        "template": cap.template,
        "severity": cap.severity,
        "categories": cap.categories,
    }


__all__ = [
    "AuthoredCaption",
    "author_caption",
    "author_caption_dict",
]


if __name__ == "__main__":
    # Quick sanity check: print the caption surface for a sample SFX.
    for sample in [
        "sfx_pistol_fire",
        "sfx_weapon_reload_pistol",
        "sfx_grenade_frag_detonation",
        "footstep_metal_walk",
        "impact_metal_small",
        "sfx_explosion_large",
        "sfx_banner_critical_alarm",
        "sfx_voice_grunt_pain_severe",
        "sfx_unknown_xyz",
    ]:
        c = author_caption(sample)
        print(f"{sample:40s}  -> {c.template:45s}  [{c.severity:8s}] {c.categories}")
