---
type: spec
status: closed-direction
authority: "Modding ecosystem extensions: versioning + dependency mgmt + conflict detection + analytics + voluntary tip jar + mod-of-the-week curation + collab tools + AI-driven test runs + private cloud + auto-update + rollback + author-controlled localization + auto-docs."
ready_when: "Mod versioning resolves; conflict detection works in-game; modder analytics opt-in; tip jar URL aggregation; mod-test-run AI agent validates submitted mods; auto-update + rollback functional."
feeds:
  - DR-006
  - DR-024
  - DR-031
  - DR-034
  - DR-045
  - DR-046
  - DR-047
  - DR-050
---

← [[spec/index|spec section]] · [[decisions/dr-050-modding-social-onboarding-and-ai-extensions|DR-050]] · [[decisions/dr-006-modding-data-model|DR-006]] · [[spec/launch-content-roster|launch roster]]

# Modding Ecosystem Extensions

## Versioning + Dependency Management

```ron
// mods/my-faction-mod/mod.ron
mod: (
    id: "my-faction-mod",
    version: "1.2.3",
    author: "modder@example.com",
    requires_version: "0.1.0..1.0.0",  // version range of base game
    depends_on: [
        ( id: "core-faction-helpers", version: "0.5..2.0" ),
        ( id: "advanced-weapons-pack", version: "1.0..2.0", optional: true ),
    ],
    conflicts_with: [
        ( id: "different-mod-version", version: "1.0..1.5", reason: "asset override conflict" ),
    ],
    license: "CC-BY-SA-4.0",
    workshop_id: 123456789,
)
```

| Aspect | Detail |
|---|---|
| Auto-resolve dependency graph | Pre-load step. |
| Block load on conflict | Clear diagnostic; player chooses. |
| Auto-migrate to current version | If patch update + opt-in. |
| Version range validator | `cargo` semver-style range. |

## Conflict Detection

| Conflict Type | Detail |
|---|---|
| Asset override | Multiple mods override same asset; load-order ranking + per-asset granular override UI. |
| Stat conflict | Two mods modify same item stats; visible diff + manual choose. |
| Capability conflict | Two mods declare same equipment slot; merge / pick-one UI. |
| Script-host conflict | Two mods register same script handler; first-registered wins; warn. |

## Mod-Creator Analytics (Opt-In)

| Metric | Detail |
|---|---|
| Per-mod usage rate | How often mod is loaded. |
| Per-mod mission success rate | Win-rate of missions including mod content. |
| Per-mod crash signature | If crashes correlate with mod load. |
| Average play time | Per-mod session minutes. |
| Top-conflicting mods | Which other mods this conflicts with. |
| Privacy-by-default | Modder opts in. |

## Voluntary Tip Jar

Modder-set tip URL via Stripe / Ko-Fi / Patreon. Project takes 0% cut per DR-031.

| Aspect | Detail |
|---|---|
| Discoverable | Mod-page in launcher shows tip-jar link. |
| Optional | Modder-controlled. |
| 0% project cut | Per DR-031 marketplace cut. |
| Per-mod or per-modder | Modder decides. |

## Mod-of-the-Week Curation

Project-owner curated; rotating featured mod in main menu launcher banner.

| Aspect | Detail |
|---|---|
| Cadence | Weekly. |
| Selection | Project-owner + community nominations. |
| Reward | Featured banner; cosmetic emblem to modder. |
| Criteria | Quality + completeness + community vote. |

## Modder Collab Tools

In-Discord modder rooms; shared package projects; code-review tooling for mod-package PRs.

| Aspect | Detail |
|---|---|
| Discord modder rooms | Per-modder; per-collaboration. |
| Shared mod-projects | Multi-modder package authoring. |
| Code review | Mod-package PR review tools. |

## Mod-Test-Run AI Agent

Modder submits chassis → AI agent generates test scenarios for it; auto-runs balance + AI behavior validation; reports issues.

```bash
$ cf-mod test-run my-mod
[INFO] Loading my-mod...
[INFO] Generating 50 balance scenarios...
[INFO] Running scenarios in parallel...
[INFO] Results:
  - Coverage: 90% of mod equipment used in scenarios
  - Balance: 8/50 scenarios show TTK > 99th percentile (review)
  - AI compatibility: All 50 scenarios complete without AI refusal regression
  - Crashes: 0
  - Issues: 2 (low severity)
[INFO] Submit ready: yes
```

## Mod-Private Cloud

Modder hosts pre-release content for friends-only access (e.g., paid playtest builds).

| Aspect | Detail |
|---|---|
| Friends-only | Modder controls access list. |
| Pre-release | Test builds before public release. |
| Optional paid | Modder can monetize via tip jar. |
| Workshop-style infrastructure | Self-hostable per cf-server. |

## Auto-Update + Rollback

| Aspect | Detail |
|---|---|
| Auto-update | Workshop publishes new mod version; subscribers auto-update. |
| Rollback | If breakage detected via mod-test-run AI agent, auto-rollback to previous stable version. |
| Subscriber control | Player can opt-out of auto-update per mod. |

## Mod-Author-Controlled Localization

Mod author submits per-locale `.ftl` packs per [[spec/localization-plan]].

## Mod SDK Auto-Docs

Generated from Rust trait impls; published to dedicated docs site via cargo-doc + custom theme.

| Aspect | Detail |
|---|---|
| Site | https://corefall.dev/mod-sdk/ |
| Auto-generated | Per release. |
| Searchable | Full-text search. |
| Examples | Per API endpoint. |

## Modder Events

| Event | Cadence |
|---|---|
| Mod-of-the-week | Weekly. |
| Mod showcase Discord stream | Monthly. |
| Mod jam (per [[spec/endgame-modes-and-retention-loops]]) | Quarterly. |
| Modder-vs-modder collab projects | Open. |

## Done-Criteria

- [ ] Mod versioning resolves dependency graph.
- [ ] Conflict detection works in-game.
- [ ] Modder analytics opt-in.
- [ ] Tip jar URL aggregation.
- [ ] Mod-test-run AI agent validates submitted mods.
- [ ] Auto-update + rollback functional.
- [ ] Mod-author-controlled localization integrated.
- [ ] Mod SDK auto-docs published.

## Source Trail

- [[decisions/dr-050-modding-social-onboarding-and-ai-extensions]]
- [[decisions/dr-006-modding-data-model]]
- Path of Exile mod ecosystem: voluntary tip jars + modder credits.
- Skyrim Workshop: load-order management + conflict detection precedent.
- Cargo (Rust): semver dependency model precedent.
