# specs/active/

Milestone(s) currently being implemented. ONE file per milestone. The implementer reads ONLY files in this folder + source code.

## Naming

`<milestone-id>.md` — e.g., `M14.md`, `M15.md`. Core milestones are numbered sequentially 1..49.

**Suffix-letter convention:** when an additional milestone is slotted between two existing milestones (typically production-track / asset-pipeline / platform-integration), it uses a letter suffix: `M4A.md` sits between M4 and M5; `M36A.md` sits between M36 and M37; `M36B.md` sits between M36A and M37. This avoids renumbering everything downstream when new milestones are inserted.

Read order is alphanumeric: M4 → M4A → M4B → M5 → M5A → M6 → ...

## Production-track milestones (16 added; suffix-letter)

These run alongside gameplay milestones and own the AI-driven asset / audio / narrative / platform pipelines. Per DR-044 + DR-045 + DR-053, every asset is AI-generated + ledgered + regenerable. Human authoring is minimal-to-zero; LLM + ComfyUI + Stable Audio Open + voice synthesis pipelines author everything, ledgered for deterministic regeneration.

| ID | Position | Owns |
|---|---|---|
| M4A | between M4 + M5 | Asset ledger infrastructure (`cf-asset-ledger`; DR-053) |
| M9A | between M9 + M10 | Tier 1 SVG asset pipeline (Python + cairo-svg + Pillow + LLM-prompted SVG) |
| M12A | between M12 + M13 | Tier 1 audio pipeline (LLM-generated SFX via Stable Audio Open / AudioCraft) |
| M18A | between M18 + M19 | Animation production Tier 1 (walk cycles + hit reactions + death anim via AnimateDiff) |
| M24A | between M24 + M25 | VFX + particle production Tier 1 (impact variants + spark systems + explosion VFX) |
| M25A | between M25 + M26 | Narrative + codex production (LLM-driven story bible + 600 codex entries + 80k words) |
| M32A | between M32 + M33 | Tier 2 ComfyUI asset pipeline (SDXL + Flux + AnimateDiff + ControlNet + LoRA training) |
| M33A | between M33 + M34 | Tutorial lab production + onboarding wizard (8 modular labs from DR-023) |
| M36A | between M36 + M37 | Platform integration (Steam SDK + Discord rich presence + EOS adapter) |
| M36B | between M36A + M37 | Telemetry + crash + bug-report pipeline (`cf-telemetry`; opt-in privacy) |
| M37A | between M37 + M38 | Tier 2 audio + voice synthesis + music composition (30+ tracks + 80+ voice lines) |
| M38A | between M38 + M39 | Localization production (LLM auto-translation for 19 launch languages) |
| M40A | between M40 + M41 | Spectator + streamer polish (Twitch/YouTube + replay-to-MP4 + branded overlays) |
| M45A | between M45 + M46 | Cosmetic production pipeline (50+ items per actor × 44+ actors via Tier 2 ComfyUI) |
| M48A | between M48 + M49 | Tier 3 asset + audio polish (final art consistency + cinematic VFX + FMOD/Kira mix) |
| M48B | between M48A + M49 | Steam store + marketing pipeline (capsule art + 6 trailer types + press kit) |

## UX/UI milestones (8 added; suffix-letter)

These polish the player-facing surfaces beyond what core milestones reserve. Per DR-046 + DR-012, every shell + game-mode UI screen meets ACC-A floor (text_scale 2.0 / high_contrast / keyboard + controller navigation / caption + voice). Together with M11 (HUD foundation) + M27 (inventory base) these milestones close every L1-L4 surface the player ever sees.

| ID | Position | Owns |
|---|---|---|
| M11A | between M11 + M12 | Shell UI foundation (title + main menu + pause + save-load + settings tree + credits + loading screens) |
| M27A | between M27 + M28 | Player game UI (inventory Tetris + loadout + cosmetic + codex 600 + achievements 75 + tutorial menu) |
| M28A | between M28 + M29 | Base build mode UX (palette + ghost preview + rotation + room detection + blueprints + demolish/repair + co-build) |
| M29A | between M29 + M30 | Power grid + IC10 editor UX (Factorio-style overlay + IC10 editor with breakpoints + per-generator dashboard) |
| M32B | between M32A + M33 | Crafting + research tree + salvage UX (3-pane crafting + research tree pan/zoom + 30+ mod slots + material flow) |
| M40B | between M40A + M41 | Online UX (server browser + friends + party + lobby + admin web panel + mod hash sync + voice chat) |
| M43A | between M43 + M44 | Map + mission select + campaign + travel planner UX (world map + solar system map + campaign tree + briefing) |
| M48C | between M48B + M49 | Endgame + workshop UX polish (debrief + replay browser + photo mode + mech bay + dossiers + faction + quest + hub + mod manager) |

## Read order summary (alphanumeric)

Core + production-track + UX/UI milestones interleave; the implementer reads in alphanumeric order:

```
M4 → M4A → M5 → M6 → M7 → M8 → M9 → M9A → M10 → M11 → M11A → M12 → M12A → M13 → ...
... → M24 → M24A → M25 → M25A → M26 → M27 → M27A → M28 → M28A → M29 → M29A → M30 → ...
... → M32 → M32A → M32B → M33 → M33A → M34 → ... → M36 → M36A → M36B → M37 → M37A → M38 → M38A → M39 → M40 → M40A → M40B → M41 → ... → M43 → M43A → M44 → M45 → M45A → M46 → M47 → M48 → M48A → M48B → M48C → M49
```

Total: 70 active milestone files (46 core M4-M49 + 16 production-track + 8 UX/UI).

## Lifecycle

1. **Planner session:** writes a new spec here from `specs/backlog/`. Stops without implementing. (See `specs/_planner.md`.)
2. **Human review:** project owner reads the spec (~5 min) and approves or corrects.
3. **Implementer session:** reads ONLY `specs/active/<id>.md` + source files. Implements until every Acceptance Criterion is satisfied. Commits.
4. **On completion:** spec is moved to `specs/done/<id>.md`. New spec written for the next milestone.

## Rule

If `specs/active/` has more than 2-3 files, you've over-committed. Finish one before starting the next.
