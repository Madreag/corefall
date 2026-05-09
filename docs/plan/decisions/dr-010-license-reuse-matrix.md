---
type: decision
id: DR-010
status: open
priority: P1
revisit_trigger: "When public release becomes a near-term goal (alpha/beta/store launch)."
---

← [[decisions/index|decision records]] · [[references/sources|sources]] · [[references/usage-ledger|usage ledger]] · [[repos/cccp-active-unified-repo|CCCP active]] · [[repos/c4-continuation-engine|C4 fork]]

# DR-010: License And Reuse Matrix

> [!info] Posture
> **This is a personal project.** Reuse is allowed during research and prototyping. The point of this record is **documentation, not gating**. The license matrix exists so that if/when we decide to release publicly, we have a clear ledger of what was used, what tier each piece sits in, and what would need to be replaced or properly licensed before launch.
>
> Until then: use what helps; **track it in [[references/usage-ledger]]**; flag anything that would need legal review for a public release. Do not let license uncertainty block prototyping.

> [!warning] Legal disclaimer
> This is research notes, not legal advice. All license claims here are flagged with confidence. Before any **public release** (Steam, itch.io, GitHub public, distribution to non-personal builds), revisit this record and consult actual legal review.

## Context

We have multiple cloned repositories that are research material today and may become reuse sources tomorrow. Each carries its own license. For a personal/private project the practical question is "what did I actually use, and what would I need to renegotiate if I ever ship publicly?"

This record answers that as a register. It is not a gate.

## Reuse Posture (Personal Build vs Public Release)

| Question | Personal/private build | Public release |
|---|---|---|
| Ship CCCP engine code? | Allowed; track in usage ledger. | Re-check AGPL-3.0 distribution requirement. |
| Use CCCP `Base.rte` art/audio as starter content? | Allowed for prototyping. | Replace, license, or accept AGPL terms before release. |
| Copy `.rte` data conventions/naming? | Allowed. | Conventions/schemas are largely uncopyrightable; safe. |
| Reuse Lua AI scripts as-is? | Allowed. | Re-implement or accept AGPL terms before release. |
| Reuse C4 networking (RakNet)? | Allowed but risky (legacy + license). | Replace with modern transport before release. |
| Reuse OpenSoldat code (MIT)? | Allowed; attribution. | Allowed; attribution + NOTICES. |
| Reuse OpenLiero code (BSD)? | Allowed; attribution. | Allowed; attribution + NOTICES. |
| Reuse OpenLieroX code (zlib lineage)? | Allowed; verify version. | Verify exact license per file. |
| Reuse Powder Toy code? | Allowed for personal use; GPL-3.0 viral if linked. | Avoid linking; redesign or relicense. |
| Reuse GameNetworkingSockets? | Allowed (BSD). | Allowed; NOTICES. |
| Reuse classic Cortex Command (Data Realms) assets? | Allowed for personal study. | Treat as off-limits for public ship without per-asset clearance. |

## License Snapshot

| Asset | License | Confidence | Reuse Bar |
|---|---|---|---|
| CCCP engine code | AGPL-3.0 | High (visible in `LICENSE` of CCCP repo) | Viral. Distributing modified versions requires releasing source. |
| CCCP `Base.rte` content (art/audio/Lua) | AGPL-3.0 (with possible per-asset overrides) | Medium - likely; verify per asset | Same viral terms; per-file license headers must be checked. |
| C4 engine code | AGPL-3.0 (likely; verify) | Medium | Same viral terms expected. |
| C4 RakNet inclusion | RakNet license (BSD-style with conditions) | Medium | Modern projects often replace RakNet; verify if compatible. |
| Classic Cortex Command (Data Realms) assets | Unclear; some open-source release in 2019 (AGPL-3.0) | Medium | Treat as off-limits unless verified per asset. |
| OpenSoldat code | MIT (per repo metadata) | High | Permissive; can reuse with attribution. |
| OpenSoldat content (`base` repo) | Mixed; verify | Low | Likely needs per-asset review. |
| OpenLiero code | BSD-2-Clause (per repo notes) | High | Permissive; attribution. |
| OpenLiero content | Replaced libre sounds noted; verify | Medium | Per-asset. |
| OpenLieroX code | zlib license noted in upstream history | Medium | Permissive but verify exact version. |
| OpenLieroX content | Mixed | Low | Per-asset. |
| The Powder Toy code | GPL-3.0 (likely) | Medium | Viral if linked into our binary. |
| The Powder Toy content | Mixed | Low | Per-asset. |
| GameNetworkingSockets | BSD-style by Valve | High | Permissive; reusable as transport. |
| LuaJIT | MIT | High | Permissive. |
| Lua | MIT | High | Permissive. |
| FMOD (if used by CCCP) | Proprietary; commercial licensing required | High | Likely needs commercial agreement or replacement. |
| SDL2/SDL3 | zlib | High | Permissive. |
| OpenAL Soft | LGPL | Medium | Dynamic linking common. |
| FreeType | FTL/GPL | Medium | Permissive in most uses. |
| miniz / lz4 / zstd | Permissive (zlib/Apache/BSD) | High | OK. |

## Reuse Tiers (For Future Public Release Only)

These tiers are guides for **public release**, not gates for personal prototyping.

| Tier | Meaning | Public-release action |
|---|---|---|
| Tier 0 | Permissive open source (MIT/BSD/zlib). | Reusable with attribution; track in NOTICES. |
| Tier 1 | Weak copyleft (LGPL). | Dynamic-link + isolation; document. |
| Tier 2 | Strong copyleft (GPL/AGPL). | Whole project becomes copyleft if linked. Replace or accept terms. |
| Tier 3 | Proprietary (FMOD, RakNet variants). | Commercial license or replacement required. |
| Tier 4 | Unknown / mixed. | Per-asset review before release. |

## Tracking

When something is actually used (code copied, asset reused, dependency added), record it in [[references/usage-ledger]] with:

- What was used (file path, asset, code chunk).
- Where it came from (repo, commit, license).
- Where it's used (in our future game's path).
- Tier per the table above.
- Replacement plan if Tier 2/3/4 and public release becomes a goal.

## Risks (Public Release Only)

| Risk | Mitigation if releasing publicly |
|---|---|
| Inadvertent AGPL inheritance | Audit usage ledger; replace Tier 2 items or accept copyleft. |
| Asset reuse without permission | Replace with original or licensed art/audio before release. |
| RakNet legal trap | Replace with modern transport before release. |
| FMOD commercial trap | Replace with OpenAL Soft / SoLoud / miniaudio or buy license. |
| LuaJIT vs Lua choice | Both MIT; choose based on platform support and JIT requirements. |

## Open Questions

| Question | Next Evidence |
|---|---|
| Exact SPDX for each `Base.rte` asset? | Per-asset license headers + repo `LICENSE`. |
| Does CCCP currently link FMOD or has it migrated? | Inspect `meson.build` and `subprojects/`. |
| What RakNet variant does C4 ship? | Inspect `Licences/RakNet.txt`. |
| Are there contributor-license agreements (CLAs) for CCCP that affect reuse? | Project contribution docs. |

## Revisit Trigger

Reopen this decision when:

- Public release (alpha/beta/store) becomes a near-term goal.
- A new external asset/code source enters the [[references/usage-ledger]] that is Tier 2/3/4.
- Distribution model changes (commercial, free, open source).

## Source Trail

- `../Cortex-Command-Community-Project/LICENSE`
- `../Cortex-Command-Community-Project/Licences/`
- `../Cortex-Command-Community-Continuation-Engine/Licences/`
- `https://github.com/opensoldat/opensoldat` (MIT)
- `https://github.com/openliero/openliero` (BSD-2-Clause)
- `https://github.com/openlierox/openlierox` (zlib lineage)
- `https://github.com/The-Powder-Toy/The-Powder-Toy` (GPL-3.0)
- `https://github.com/ValveSoftware/GameNetworkingSockets` (BSD)
