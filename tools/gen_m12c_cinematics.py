#!/usr/bin/env python3
"""Generate M12C cinematic RON scripts for openings / between / endings.

The spec ships:
- 30+ mission-opening cinematics (cf_intro_<mission_id>.cinematic.ron)
- 5 storytellers x 3 variants = 15 between-mission cinematics
- 5 storyteller-specific endings
- Per-cinematic narration_track.json (baked stub at M12C; production
  bakes happen at M37A).

Run from the repo root: ``python3 tools/gen_m12c_cinematics.py``.
"""

import json
import os
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CONTENT = REPO_ROOT / "game" / "content"
CINE = CONTENT / "cinematics"
OPENING_DIR = CINE / "opening"
BETWEEN_DIR = CINE / "between"
ENDING_DIR = CINE / "ending"
NARRATION_DIR = CONTENT / "audio" / "voice" / "cinematic"

OPENING_MISSIONS = [
    # 30 launch openings per spec.  Authored content uses the mission id
    # as the cinematic id.  Per spec § "30+ launch missions each have a
    # <mission_id>.cinematic.ron script under content/cinematics/opening/."
    ("cin_intro_micro_breach", "Punch through the breach line."),
    ("cin_intro_outpost_recon", "Recon the outpost; mark every door."),
    ("cin_intro_drainage_flood", "The trench drainage failed; hold the line."),
    ("cin_intro_fire_step_duel", "Hold the fire step; sharpshooters at 200m."),
    ("cin_intro_zigzag_assault", "Push the zigzag trench; clear corner traverses."),
    ("cin_intro_breastwork_breach", "Breach the breastwork before the wave arrives."),
    ("cin_intro_two_line_defense", "Two trench lines; you choose which holds."),
    ("cin_intro_template_drop_test", "First template drop; tutorial-adjacent."),
    ("cin_intro_minefield_clearance", "Clear the minefield before the convoy."),
    ("cin_intro_mg_nest_assault", "MG nest crewed; suppress and flank."),
    ("cin_intro_watchtower_spotter", "Spotter chain spotted you; move now."),
    ("cin_intro_wire_breach", "Wire field; cutters out, eyes up."),
    ("cin_intro_sandbag_erosion", "Sandbags eroding under shellfire."),
    ("cin_intro_ied_chain", "IED chain wired across the killzone."),
    ("cin_intro_camo_netting", "Camo nets up; enemies are in concealment."),
    ("cin_intro_anti_tank_layered", "Anti-tank ditch + dragon's teeth; armor coming."),
    ("cin_intro_electrified_fence", "Fence is hot; find the breaker."),
    ("cin_intro_full_strongpoint", "Full strongpoint; complete fortification doctrine."),
    ("cin_intro_chassis_salvage", "Wrecked chassis ahead; recovery under fire."),
    ("cin_intro_chassis_climb", "Climb the chassis lift before the bombardment."),
    ("cin_intro_chassis_module_swap", "Hot-swap a chassis module mid-mission."),
    ("cin_intro_chassis_jet", "Jet across the chasm; one fuel charge."),
    ("cin_intro_anchor_lane", "Anchor lane defense; hold the choke."),
    ("cin_intro_material_lane", "Material lane recon; tag every substrate."),
    ("cin_intro_actor_range_shotgun", "Shotgun range duel; ten seconds to load."),
    ("cin_intro_actor_range_tracer", "Tracer range; identify every chassis."),
    ("cin_intro_micro_reactor_defense", "Micro reactor defense; classic Cassandra."),
    ("cin_intro_tutorial_onboarding", "First mission; storyteller introduces themselves."),
    ("cin_intro_replay_determinism_lab", "Determinism lab; replay-only opener."),
    ("cin_intro_lab_squad_drill", "Squad drill; four-line breach choreography."),
    ("cin_intro_lab_terrain_drill", "Terrain drill; pixel-perfect carve test."),
    ("cin_intro_lab_movement_drill", "Movement drill; jump-vault-slide combo."),
]

# Per spec § "5 storytellers × 3 variants = 15".
STORYTELLERS = [
    ("cassandra_classic", "Cassandra: dread monologue"),
    ("phoebe_chillax", "Phoebe: quirky aside"),
    ("randy_random", "Randy: chaotic shrug"),
    ("ironman", "Ironman: challenge"),
    ("sandbox", "Sandbox: skipped"),
]

# Per-storyteller monologue lines for the between-mission variants.
BETWEEN_LINES = {
    "cassandra_classic": [
        ["The next contract is heavier than the last.", "Sleep when you can."],
        ["Something on this rock changed last night.", "The cell readings don't lie."],
        ["You earned this rest.", "Don't trust it."],
    ],
    "phoebe_chillax": [
        ["There's a tea-stain shaped like Phobos on my chart.", "Take it as a sign."],
        ["I packed an extra ration.", "You always forget."],
        ["The base lights blinked when you came back.", "I told them you would."],
    ],
    "randy_random": [
        ["HA — coin says south.", "South it is."],
        ["I rolled three sixes.", "Mission probably explodes."],
        ["Dice say bring the heavy.", "I packed it for you anyway."],
    ],
    "ironman": [
        ["You survived.", "Now earn the next."],
        ["The frontier remembers.", "So do I."],
        ["No retreat from this one.", "Win clean."],
    ],
    "sandbox": [
        ["Mission select ready.", ""],
        ["Mission select ready.", ""],
        ["Mission select ready.", ""],
    ],
}

# Per-storyteller ending lines (Act 1 framing for the 2-5min ending).
ENDING_LINES = {
    "cassandra_classic": [
        "The frontier was always going to fall.",
        "You stayed long enough to remember the names.",
        "That counts for something.",
    ],
    "phoebe_chillax": [
        "It's quiet now.",
        "The squad made breakfast.",
        "You should join us.",
    ],
    "randy_random": [
        "Last roll: sixes across the board.",
        "Ha. Of course.",
        "Roll the credits.",
    ],
    "ironman": [
        "The line held.",
        "Hold it tomorrow.",
        "Salute.",
    ],
    "sandbox": [
        "Campaign complete.",
        "No monologue authored.",
        "Acts 1-2 skipped; Act 3 painted slides only.",
    ],
}


def shot_block(label, duration_ms, moves, actor_poses=None):
    moves_str = "\n".join(["                " + m + "," for m in moves])
    poses = actor_poses or []
    poses_str = "\n".join(
        [
            f"                ( actor_id: \"{aid}\", pose_id: \"{pid}\" ),"
            for aid, pid in poses
        ]
    )
    actor_poses_block = (
        f"            actor_poses: [\n{poses_str}\n            ],\n" if poses_str else "            actor_poses: [],\n"
    )
    return (
        f"        (\n"
        f"            label: \"{label}\",\n"
        f"            duration_ms: {duration_ms},\n"
        f"            moves: [\n{moves_str}\n            ],\n"
        f"{actor_poses_block}"
        f"        ),"
    )


def write_opening(mission_id, headline):
    # Per spec § "Live actors play scripted poses — squad members enter
    # their pre-mission stance (chassis idle + weapon at low-ready +
    # storyteller-specific body language)".  Authored pose IDs resolve
    # against the M9A animation catalog.
    shots = [
        shot_block(
            "dropship_door_opens",
            8000,
            [
                "( kind: Dolly, start_ms: 0, duration_ms: 8000, easing: EaseInOutCubic, dolly_target: (0.0, 0.0), dolly_distance: 12.0 )",
            ],
            actor_poses=[("player", "chassis_idle")],
        ),
        shot_block(
            "squad_silhouettes",
            6000,
            [
                "( kind: Pan, start_ms: 0, duration_ms: 6000, easing: EaseInOutCubic, pan: (8.0, 0.0) )",
            ],
            actor_poses=[
                ("squad_alpha", "low_ready"),
                ("squad_bravo", "low_ready"),
            ],
        ),
        shot_block(
            "mission_pov",
            9000,
            [
                "( kind: Pan, start_ms: 0, duration_ms: 9000, easing: EaseInOutCubic, pan: (10.0, -2.0) )",
                "( kind: Shake, start_ms: 4000, duration_ms: 600, shake: ( amplitude_px: 4.0, frequency_hz: 30.0, decay_s: 0.6 ) )",
            ],
        ),
        shot_block(
            "boss_silhouette_reveal",
            7000,
            [
                "( kind: Zoom, start_ms: 0, duration_ms: 7000, easing: EaseInOutCubic, zoom_to: 10.0 )",
            ],
        ),
    ]
    text = (
        f"// M12C § Mission-opening cinematic — {mission_id}.\n"
        f"// 30-60s in-engine cinematic per spec § Mission-opening cinematic (30-60s).\n"
        f"(\n"
        f"    schema_version: 1,\n"
        f"    id: \"{mission_id}\",\n"
        f"    source: opening,\n"
        f"    storyteller: None,\n"
        f"    shots: [\n" + "\n".join(shots) + "\n    ],\n"
        f"    chapters: [\n"
        f"        ( id: \"dropship_door_opens\", at_ms: 8000 ),\n"
        f"        ( id: \"squad_silhouettes_revealed\", at_ms: 14000 ),\n"
        f"        ( id: \"boss_silhouette_reveal\", at_ms: 23000 ),\n"
        f"    ],\n"
        f"    narration_track_id: Some(\"{mission_id}\"),\n"
        f"    briefing_card_lines: [\n"
        f"        \"{headline}\",\n"
        f"        \"Objective: complete primary mission goal.\",\n"
        f"        \"Reward: per mission balance.\",\n"
        f"        \"Risk: ladder per storyteller.\",\n"
        f"        \"Time on station: 16:00.\",\n"
        f"        \"Storyteller stinger plays at +T28s.\",\n"
        f"    ],\n"
        f"    briefing_at_ms: 15000,\n"
        f")\n"
    )
    OPENING_DIR.mkdir(parents=True, exist_ok=True)
    (OPENING_DIR / f"{mission_id}.cinematic.ron").write_text(text)


def write_between(storyteller_id, variant_index, lines):
    cid = f"{storyteller_id}_v{variant_index}"
    shots = [
        shot_block(
            "base_dolly",
            10000,
            [
                "( kind: Dolly, start_ms: 0, duration_ms: 10000, easing: EaseInOutCubic, dolly_target: (0.0, 0.0), dolly_distance: 14.0 )",
            ],
        ),
        shot_block(
            "storyteller_pov",
            10000,
            [
                "( kind: Pan, start_ms: 0, duration_ms: 10000, easing: EaseInOutCubic, pan: (4.0, 0.0) )",
            ],
        ),
    ]
    duration = 20000
    text = (
        f"// M12C § Between-mission cinematic — {storyteller_id} variant {variant_index}.\n"
        f"// 15-30s monologue per spec § Between-mission cinematic.\n"
        f"(\n"
        f"    schema_version: 1,\n"
        f"    id: \"{cid}\",\n"
        f"    source: between,\n"
        f"    storyteller: Some({storyteller_id}),\n"
        f"    shots: [\n" + "\n".join(shots) + "\n    ],\n"
        f"    chapters: [\n"
        f"        ( id: \"monologue_start\", at_ms: 1000 ),\n"
        f"        ( id: \"rival_taunt\", at_ms: 12000 ),\n"
        f"    ],\n"
        f"    narration_track_id: Some(\"{cid}\"),\n"
        f"    briefing_card_lines: [\n"
        f"        \"{lines[0]}\",\n"
        f"        \"{lines[1]}\",\n"
        f"    ],\n"
        f"    briefing_at_ms: 4000,\n"
        f")\n"
    )
    BETWEEN_DIR.mkdir(parents=True, exist_ok=True)
    (BETWEEN_DIR / f"{cid}.cinematic.ron").write_text(text)


def write_ending(storyteller_id, lines):
    # Sandbox ending suppresses Acts 1-2 per spec; we still write a
    # 2-minute placeholder script so the kernel can run the parity
    # event stream.  120000 ms is the minimum per spec § Campaign-
    # ending cinematic (2-5min).
    duration_per_act = 60000
    if storyteller_id == "sandbox":
        shots_text = shot_block(
            "act3_only",
            duration_per_act * 2,
            [
                "( kind: Pan, start_ms: 0, duration_ms: 60000, easing: EaseInOutCubic, pan: (10.0, 0.0) )",
                "( kind: Pan, start_ms: 60000, duration_ms: 60000, easing: EaseInOutCubic, pan: (10.0, 0.0) )",
            ],
        )
        chapter_block = (
            "        ( id: \"act3_resolution_start\", at_ms: 0 ),\n"
        )
    else:
        shots = [
            shot_block(
                "act1_denouement",
                duration_per_act,
                [
                    "( kind: Pan, start_ms: 0, duration_ms: 60000, easing: EaseInOutCubic, pan: (15.0, 0.0) )",
                ],
            ),
            shot_block(
                "act2_montage",
                duration_per_act,
                [
                    "( kind: Pan, start_ms: 0, duration_ms: 60000, easing: EaseInOutCubic, pan: (-12.0, 4.0) )",
                ],
            ),
            shot_block(
                "act3_resolution",
                duration_per_act,
                [
                    "( kind: Zoom, start_ms: 0, duration_ms: 60000, easing: EaseInOutCubic, zoom_to: 8.0 )",
                ],
            ),
        ]
        shots_text = "\n".join(shots)
        chapter_block = (
            "        ( id: \"act1_denouement_start\", at_ms: 0 ),\n"
            "        ( id: \"act2_montage_start\", at_ms: 60000 ),\n"
            "        ( id: \"act3_resolution_start\", at_ms: 120000 ),\n"
        )
    text = (
        f"// M12C § Campaign-ending cinematic — {storyteller_id}.\n"
        f"// 2-5min 3-act structure per spec § Campaign-ending cinematic.\n"
        f"(\n"
        f"    schema_version: 1,\n"
        f"    id: \"{storyteller_id}\",\n"
        f"    source: ending,\n"
        f"    storyteller: Some({storyteller_id}),\n"
        f"    shots: [\n" + shots_text + "\n    ],\n"
        f"    chapters: [\n" + chapter_block + "    ],\n"
        f"    narration_track_id: Some(\"{storyteller_id}\"),\n"
        f"    briefing_card_lines: [\n"
        f"        \"{lines[0]}\",\n"
        f"        \"{lines[1]}\",\n"
        f"        \"{lines[2]}\",\n"
        f"    ],\n"
        f"    briefing_at_ms: 120000,\n"
        f")\n"
    )
    ENDING_DIR.mkdir(parents=True, exist_ok=True)
    (ENDING_DIR / f"{storyteller_id}.cinematic.ron").write_text(text)


def write_narration_track(cinematic_id, words):
    """Write a placeholder narration track JSON.

    Per spec § "Empty array = no caption highlighting; cinematic still
    plays."  For openings we author a minimal 4-word track; for
    between/ending we author the briefing-card lines as one word each.
    """
    NARRATION_DIR.mkdir(parents=True, exist_ok=True)
    payload = {"words": words}
    (NARRATION_DIR / f"{cinematic_id}.narration_track.json").write_text(
        json.dumps(payload, indent=2)
    )


def main():
    # 30+ openings.
    for mid, headline in OPENING_MISSIONS:
        write_opening(mid, headline)
        write_narration_track(
            mid,
            [
                {"word": "the", "start_ms": 1000, "end_ms": 1300},
                {"word": "dropship", "start_ms": 2100, "end_ms": 2700},
                {"word": "hovers", "start_ms": 2700, "end_ms": 3500},
                {"word": "down", "start_ms": 3500, "end_ms": 4000},
            ],
        )
    # 5 storytellers x 3 between variants.
    for sid, _label in STORYTELLERS:
        for vi in range(3):
            lines = BETWEEN_LINES[sid][vi]
            write_between(sid, vi, lines)
            write_narration_track(
                f"{sid}_v{vi}",
                [
                    {"word": "monologue", "start_ms": 1000, "end_ms": 1500},
                    {"word": "online", "start_ms": 1500, "end_ms": 2000},
                ],
            )
    # 5 endings.
    for sid, _label in STORYTELLERS:
        write_ending(sid, ENDING_LINES[sid])
        write_narration_track(
            sid,
            [
                {"word": "epilogue", "start_ms": 1000, "end_ms": 1800},
                {"word": "and", "start_ms": 1800, "end_ms": 2000},
                {"word": "credits", "start_ms": 2000, "end_ms": 2700},
            ],
        )
    print(f"Wrote {len(OPENING_MISSIONS)} openings, "
          f"{len(STORYTELLERS) * 3} between cinematics, "
          f"{len(STORYTELLERS)} endings.")


if __name__ == "__main__":
    main()
