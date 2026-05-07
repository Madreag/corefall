---
type: spec
status: closed-direction
authority: "Customization depth: weapon attachments + salvage crafting + intrinsic mastery skill tree + loadout sharing + paint/decal/skin/voice/emblem variants + vendor economy. Item-comparison UI. Cosmetic earn paths. NEVER pay-to-win per DR-031."
ready_when: "All weapon classes have attachment slots; salvage crafting recipes data-driven; mastery rank 1-30 functional per chassis/faction/weapon; loadout sharing via Workshop; vendor NPCs travel between worlds; cosmetic earn paths defined."
feeds:
  - DR-006
  - DR-009
  - DR-019
  - DR-031
  - DR-041
  - DR-045
  - DR-046
  - DR-047
  - DR-049
---

← [[spec/index|spec section]] · [[decisions/dr-049-customization-tournament-and-competitive|DR-049]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/launch-content-roster|launch roster]]

# Customization & Progression Depth

> [!summary] What this page is
> Per-player customization depth: weapon attachments + salvage crafting + intrinsic mastery skill tree + loadout templates + paint/decal/skin/voice/emblem variants + vendor economy. NEVER pay-to-win per DR-031. All gates intrinsic (mastery, achievement, mission).

## Weapon Attachments

Each weapon class declares attachment slots in its RON manifest per DR-006:

```ron
// content/weapons/ar_m4.ron (excerpt)
weapon: (
    id: "ar_m4",
    attachment_slots: [
        ( id: "sight", required: false, types: ["red_dot", "acog", "holographic", "iron", "scope", "thermal"] ),
        ( id: "muzzle", required: false, types: ["compensator", "muzzle_brake", "suppressor"] ),
        ( id: "grip", required: false, types: ["vertical", "angled"] ),
        ( id: "magazine", required: false, types: ["std_30", "extended_60", "drum_100", "fast_mag_30"] ),
        ( id: "stock", required: false, types: ["folding", "retractable", "heavy"] ),
        ( id: "barrel", required: false, types: ["std", "long", "short", "heavy"] ),
        ( id: "laser", required: false, types: ["red", "green", "ir"] ),
    ],
)
```

### Launch attachment count
~80 attachments across ~40 firearms. Per attachment: schema + tradeoff (e.g., suppressor reduces noise but caps muzzle velocity; ACOG zooms but slower target acquisition).

### Mod extension
Modders add new attachments via `content/attachments/` per DR-006.

## Salvage Crafting

Per DR-041 mining + DR-027 base.

### Recipes
Data-driven; modders extend.

```ron
// content/recipes/recipe_breach_charge_premium.ron
recipe: (
    id: "recipe_breach_charge_premium",
    inputs: [
        ( item: "ore_iron_smelted", quantity: 5 ),
        ( item: "ore_perchlorate", quantity: 2 ),
        ( item: "salvage_circuitry", quantity: 1 ),
    ],
    outputs: [
        ( item: "tool_breach_charge_handheld_premium", quantity: 1 ),
    ],
    crafting_time_s: 30,
    requires_workbench: true,
)
```

### Launch recipe count
~50 recipes covering: ammo refills, attachment crafting, advanced equipment, faction-specific gear. Modders extend.

## Intrinsic Mastery Skill Tree

NEVER power upgrades per DR-031. Pure intrinsic.

### Per-chassis mastery
- Rank 1-30 per chassis variant.
- Unlocks: variants (different weapon flavor, NEVER stat upgrade), paint, voice lines, lore entries.
- Rank gain: combat XP per match.

### Per-faction mastery
- Rank 1-30 per faction.
- Unlocks: faction-specific paint, faction emblems, faction-themed voice packs.

### Per-weapon mastery
- Rank 1-30 per weapon.
- Unlocks: per-weapon paint, attachment variants, sound packs.

### What it does NOT unlock
- Power upgrades (forbidden per DR-031).
- New equipment slots.
- Damage multipliers.
- HP boosts.

### What it DOES unlock
- Cosmetic variants (different weapon model with same stats).
- Paint mask layers.
- Voice line variations.
- Lore entries (codex unlocks).

## Loadout Templates + Sharing

| Aspect | Detail |
|---|---|
| Template count | 5 quick-swap slots per profile + unlimited save slots. |
| Template metadata | Loadout name, faction, role, AI doctrine hint. |
| Export/import | RON file; share via filename + clipboard. |
| Workshop publish | One-button publish loadout template to Workshop. |
| Hot-swap | Press 1-5 in lobby/match-prep to instantly swap. |
| Per-loadout custom hotbar | Define hotkey scheme per loadout. |

## Paint Jobs + Decals

| Layer | Detail |
|---|---|
| Per-chassis paint | Alpha-mask painting on metallic regions. Custom color palette per layer. |
| Decal placement | Custom decal placement on chassis surfaces. Faction emblems available. |
| Faction emblems | Per-faction official emblem + community-authored emblems via Workshop. |
| Save/share | Paint design exports to RON; share to Workshop. |

## Voice Packs

| Aspect | Detail |
|---|---|
| Per-faction voice variant | For player's commander voice. AI-generated baseline; commission-able post-launch. |
| Voice line evolution | Per veteran experience, voice-lines evolve (per [[spec/endgame-modes-and-retention-loops]] persistent veterans). |
| Mod-extensible | Modders add voice packs via Workshop. |

## Victory Poses

Animated end-of-match poses; earned via play (mastery, victory streak, achievement).

## Vendor / Economy NPCs

Per CCCP precedent.

| Aspect | Detail |
|---|---|
| Currency | `oz` (Cortex Command precedent). |
| Persistent | Currency persists across runs (campaign + roguelite + endless). |
| Buy menu | At base. CCCP-style flat list + categories. |
| NPC merchants | Travel between worlds with stock variation. Per-NPC dialogue. |
| Reputation | Per-faction reputation affects vendor pricing + stock availability. |
| Modder extension | Modders add new vendors via `content/npc_vendors/`. |

## Item-Comparison Side-by-Side UI

In workbench: select 2 items → side-by-side stat overlay + AI-generated "differs in X" callout.

```
  AK-47                M4
  ---                  --
  Damage: 35           Damage: 30
  Recoil: 0.6          Recoil: 0.4    [LOWER]
  Range: 200m          Range: 250m    [HIGHER]
  AI score: 0.78       AI score: 0.81
  
  Differs in: M4 has lower recoil + higher range
              AK-47 has higher damage
              Both reliable; M4 prefers mid-range engagement
```

## Cosmetic Earn Paths

Per DR-049 + DR-031:

| Path | Reward |
|---|---|
| Mastery rank | Variants, paint, voice lines, lore entries |
| Achievement | Skin, decal, emblem |
| Mission completion | Mission-specific cosmetic (per-mission unique) |
| Replay share count | "Star creator" emblem, voice pack |
| Bunker-Defence wins | Defender-elite paint, victory pose |
| Speedrun verification | Speedrun-elite emblem |
| Daily seed leaderboard top-100 | Daily-elite emblem, paint |
| Tournament placement | Tournament-elite paint, voice pack |
| Modder published mod | Modder-elite emblem |
| Translator credit | Translator-credit emblem |
| Bug bounty acceptance | Bug-finder emblem |

## Done-Criteria

- [ ] All weapon classes have attachment slots.
- [ ] Salvage crafting recipes data-driven.
- [ ] Mastery rank 1-30 functional per chassis/faction/weapon.
- [ ] NEVER unlocks power per DR-031.
- [ ] Loadout sharing via Workshop functional.
- [ ] Paint mask painting + decal placement UI works.
- [ ] Voice packs swappable.
- [ ] Vendor NPCs travel between worlds.
- [ ] Item-comparison UI side-by-side.
- [ ] Cosmetic earn paths trigger correctly.

## Source Trail

- [[decisions/dr-049-customization-tournament-and-competitive]]
- [[decisions/dr-031-content-economy-and-monetization-posture]]
- [[decisions/dr-006-modding-data-model]]
- [[spec/equipment-loadout]]
- Helldivers 2 customization: ~80 attachments per weapon class.
- Deep Rock Galactic: extensive cosmetic + mastery system.
