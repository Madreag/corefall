<div align="center">

# Corefall

**A 2D side-view tactical physics sandbox where every gas, every grain, every bullet, every body, every world, and every transmission is real.**

**Featuring Bunker Defence as the flagship game mode** — attackers vs defenders with full coop on either side, 1v1 / 2v2 / 3v3 / 4v4 / 1v1v1v1 / 2v1 / any combination. Ten worlds at launch (Earth, Mars, Phobos, Deimos, Earth's Moon, Mimas, Europa, Vulcan, Venus, Sol — plus belt-asteroid + orbital-station classes), real Stationeers-grade atmospherics, universal gravity, ACRE2-tier voice + radio simulation, mining, and origin-aware bodies (humans, androids, robots — each with structurally different physics).

[![Rust 1.93](https://img.shields.io/badge/Rust-1.93.0-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Bevy 0.14](https://img.shields.io/badge/Bevy-0.14-232326?style=for-the-badge&logo=bevy&logoColor=white)](https://bevyengine.org)
[![wgpu](https://img.shields.io/badge/wgpu-render-FFCC00?style=for-the-badge&logo=webgpu&logoColor=black)](https://wgpu.rs)
[![Tokio](https://img.shields.io/badge/Tokio-async-3F8FFF?style=for-the-badge&logo=tokio&logoColor=white)](https://tokio.rs)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=for-the-badge)](#license)

[![CI](https://img.shields.io/github/actions/workflow/status/Madreag/corefall/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/Madreag/corefall/actions)
[![Linux](https://img.shields.io/badge/Linux-supported-FCC624?style=flat-square&logo=linux&logoColor=black)](#)
[![macOS](https://img.shields.io/badge/macOS-supported-000?style=flat-square&logo=apple&logoColor=white)](#)
[![Windows](https://img.shields.io/badge/Windows-supported-0078D6?style=flat-square&logo=windows&logoColor=white)](#)
[![Steam Deck](https://img.shields.io/badge/Steam_Deck-floor_target-1A9FFF?style=flat-square&logo=steamdeck&logoColor=white)](#)

[![Status](https://img.shields.io/badge/status-pre--alpha%20%28M1%20active%29-orange?style=flat-square)](#project-status)
[![Vault](https://img.shields.io/badge/research-research%20vault-purple?style=flat-square)](https://github.com/Madreag/corefall#research-vault)

</div>

---

## What Corefall Is

Corefall is the implementation repo for a **tactical pulp sci-fi disaster sandbox** — a side-view, pixel-physics game where every body, every machine, every pipe, every droplet of coolant, every cubic meter of gas, every world in the sky, and every voice on the radio is a real first-class simulated thing. You command a small mercenary, rescue, and salvage outfit on a collapsing frontier. You can play strategically as a continuity commander, or take direct control of a body, android, power armor suit, or mech when the moment requires.

You will:

- **Defend the bunker** as 1-4 humans + AI guards holding a rooted command-core base against a dropship-deployed attacker team — or BE the attackers, breaching pressure seals and venting the defender's atmosphere. (Or 2v2. Or 3v3. Or 4v4. Or coop-vs-AI on either side. **Bunker Defence is the flagship mode.**)
- **Choose a planet, moon, or asteroid** for your match. Each world has real atmospheric ambient + gravity + day length + weather. Mars has dust storms; Vulcan ignites if you spark a flammable mix; Mimas at 0.0064g lets grenades arc for hours.
- **Coordinate your squad over real radio** with realistic propagation: hills break VHF line-of-sight, HAM radios bounce off the ionosphere on Earth, EMP weapons disrupt your robot's built-in radio, solar flares dump static into your link to base.
- **Breach a sealed bunker** by knowing that the room behind the airlock is filled with high-O2 atmosphere, that your enemy is using oil-fed tools, and that one round into a coolant line will produce a steam-flash that buys you three seconds of cover.
- **Watch your android operative** start to slow down as her battery runs low, because the EMP took out her organic-side comms and the synthetic side is now drawing more than she can replenish.
- **Carry a wounded human teammate** through a Mars surface section while watching the oxygen meter on her helmet drain faster than usual because there's a 1.5mm puncture nobody's noticed yet.
- **Lose a robot** to a thermal-throttle cascade because you ordered too many overclocks while the foundry pumped heat into the room.
- **Build a base** with real pipe networks, real pressure regulators, real airlocks. Or capture an enemy base by venting it.
- **Mine ore** from a vacuum belt asteroid (your robot operative is the right tool for this; your human can't survive there without sealed life-support overhead).

This is not a Cortex Command remake. It is a **best-of-genre synthesis** that takes Cortex Command's command-core / dropship / chassis / digging fantasy and sets it on top of Stationeers-grade atmospherics, Noita-grade systemic materials, full collision physics, universal gravity, ACRE2-tier voice + radio simulation, and a full astrography of playable worlds. AI bots are first-class teammates and rivals. Replay is deterministic. Modding is data-first. Accessibility is a floor, not an afterthought.

---

## The Layered Simulation

Every system reads from one source of truth. Nothing is faked.

```
                 ┌────────────────────────────────────────────────────────┐
                 │  AI Doctrine  •  Mission Director  •  Replay  •  HUD   │
                 └────────────────────────────────────────────────────────┘
                                          ▲
        ┌─────────────────────────────────┴─────────────────────────────────┐
        │                                                                   │
        │  Equipment + Chassis (origin: human / android / robot)            │
        │  Body damage + Wound model + Module damage + Origin reactions     │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Stationeers-grade Atmospherics (real PV = nRT, R = 8314.46)      │
        │  • 10 launch gases + 6 liquid mixtures, locked specific heats     │
        │  • 6 deterministic combustion reactions with autoignition T       │
        │  • Gradual phase change with latent heat                          │
        │  • Pipe networks with pumps, valves, regulators, filtration       │
        │  • Room atmospheres + airlock state machines + suit life-support  │
        │  • Per-planet ambient (Earth / Mars / Moon / Mimas / Europa /     │
        │    Vulcan / Venus) and modder-defined planets                     │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Systemic Materials (Noita-grade chunked CA kernel)               │
        │  17 launch materials + reaction table + density layering          │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Full Collision Physics (everything physical collides by default) │
        │  Limb / weapon / armor / chassis / projectile / terrain / debris  │
        │  CCD tiers + impulse-to-damage routing                            │
        │                                                                   │
        ├───────────────────────────────────────────────────────────────────┤
        │                                                                   │
        │  Universal Gravity Field (one source; sampled per-cell per-tick)  │
        │  Per-planet ambient + per-cell overrides (gravity wells, low-g    │
        │  labs, magnetic boots, damaged grav generators, reverse-g rooms)  │
        │  Reads through to ballistic drag + atmospheric stratification +   │
        │  material settling + actor falls + every dropped casing           │
        │                                                                   │
        └───────────────────────────────────────────────────────────────────┘
                                          ▲
                 ┌────────────────────────┴───────────────────────┐
                 │  Deterministic 60 Hz sim core (120 Hz path     │
                 │  validated; 128 Hz under evaluation)           │
                 └────────────────────────────────────────────────┘
```

Every layer emits replay events. Every cause chain is reproducible. Every AI agent reads the same data the player sees.

---

## Core Pillars

| Pillar | What It Means |
|---|---|
| **Real physics, end to end** | No arcade approximations. PV = nRT for atmospheres. Universal gravity for everything. Full collision by default. Stoichiometric combustion. Gradual phase change. |
| **Origin-aware bodies** | Humans, androids, and robots have **structurally different reaction chains**. Robots take internal-shock damage, leak coolant, and downclock under heat. Androids breathe, bleed, and overclock per installed module. Humans concuss, eat, and need oxygen tanks. |
| **AI as teammate and rival** | Bots are first-class. They reason, plan, panic, recover, and explain themselves through reason labels. The 8-criteria humanlike-AI bar is testable. An optional async LLM "mind" layer proposes doctrine without ever blocking the local AI. |
| **Replay determinism** | Same seed + same inputs = byte-identical event stream. Debug with replay scrubbing. Network with confidence. Audit AI behavior with cause chains. |
| **Modding as a first-class promise** | Schema-first, Lua escape hatches where useful, workbench tooling. Add a gas, a reaction, an origin, a planet — all data rows. |
| **Multiplayer ladder** | Solo + LAN co-op + online co-op + community-hostable public PvP arenas + persistent MMO shards. Same `cf-server` binary, multi-mode. Anyone can host. |
| **Accessibility floor** | Captions, contrast, no-color-only UI, focus traversal, reduced motion, reduced shake, reduced flash, reduced G-Force blackout — all from Slice A onward. |
| **No-compromise performance defaults** | Performance-sensitive values are config-driven, never hardcoded. Steam Deck floor at 1080p/60. 4K/120 strong-desktop ceiling. |

---

## Inspirations And Credits

Corefall stands on the shoulders of an exceptional set of games that figured out parts of the genre we want to weave together. **None of the work here is a copy** — but each of these projects taught us something we built on, and they deserve explicit credit.

| Inspiration | What We Learned |
|---|---|
| **[Cortex Command](https://datarealms.com)** by Data Realms | The command-core / dropship / chassis / digging / pixel-actor fantasy. The tone of "every body is physical and damageable". The mod ecosystem grammar. The actor-status / wound-state / inventory-fallout triangle. Deep, deep love. |
| **[Noita](https://noitagame.com)** by Nolla Games | Per-pixel material simulation as a core feel pillar. Alchemy / reaction / emergence as a retention loop. Hidden chemistry that rewards experimentation. The replay-able cause-chain culture. |
| **[Stationeers](https://stationeers.com)** by RocketWerkz | Real ideal-gas-law atmospherics. Specific heats, autoignition temperatures, combustion stoichiometry. Pipe networks as first-class atmospheres. Suit life-support with canister + filter + waste-tank slots. Per-planet ambient. |
| **[Barotrauma](https://barotraumagame.com)** by FakeFish + Undertow Games | Rooms-with-state architecture. Breach flooding. Crew dynamics where roles matter. Mission storytelling that emerges from system failure. |
| **[The Powder Toy](https://powdertoy.co.uk)** | Open-source falling-sand chemistry. The discipline of element-grammar reaction tables. Educational transparency. |
| **[OpenSoldat](https://opensoldat.org) / [Soldat](https://forums.soldat.pl)** | Side-view multiplayer combat feel. Movement nuance. Map mutability. Community-hosted server culture. |
| **[Liero](https://liero.be) / [OpenLieroX](https://openlierox.sourceforge.io)** | Short, intense, weapon-rich arena combat. The proof that small arenas + many extreme weapons + short rounds can produce decades of replay value. |
| **[Teardown](https://teardowngame.com)** by Tuxedo Labs | Tools that change the map become real tactics. Destruction is design. |
| **[Oxygen Not Included](https://klei.com/games/oxygen-not-included)** by Klei | Per-cell atmospheric simulation at habitat scale. Pressure / temperature / gas density storytelling. |
| **[Rain World](https://rainworld.net)** by Videocult | Behavioral creatures that don't need stat-bars to feel real. |

We **also** lean on the open Rust gamedev ecosystem: [Bevy](https://bevyengine.org), [wgpu](https://wgpu.rs), [Rapier / Avian](https://rapier.rs), [Tokio](https://tokio.rs), [serde](https://serde.rs), [BLAKE3](https://github.com/BLAKE3-team/BLAKE3), and many more. See [game/Cargo.toml](game/Cargo.toml) for the full dependency tree.

> [!important] Reuse posture
> No code, no assets, no sprites, no audio, no scripting from any of the inspiration games is copied into Corefall. Everything is implemented from chemistry/physics/game-design first principles plus public documentation (wikis, GDC talks, modding docs, public source where applicable). Provenance is logged in the canonical vault's [usage ledger](https://github.com/Madreag/corefall#research-vault) when any specific snippet of public documentation is quoted in spec/research notes.

---

## Tech Stack

| Layer | Tooling |
|---|---|
| Language | [Rust](https://www.rust-lang.org) edition 2021, MSRV 1.84, dev toolchain pinned to 1.93.0 |
| Engine | [Bevy](https://bevyengine.org) 0.14 + [wgpu](https://wgpu.rs) for 2D / GPU; custom core crates for sim |
| Physics | Custom collision + custom material kernel + custom atmospherics kernel + universal gravity field |
| Async | [Tokio](https://tokio.rs) for the JSON-RPC control plane and dedicated server |
| Networking (planned) | TBD between [Lightyear](https://github.com/cBournhonesque/lightyear) / [renet](https://github.com/lucaspoffo/renet) / [quinn](https://github.com/quinn-rs/quinn); decision deferred to M9/M10 |
| Modding host (planned) | [mlua](https://github.com/khvzak/mlua) (Lua) candidate; deferred to M5 |
| Determinism | [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) for state checksums; [rand_xoshiro](https://docs.rs/rand_xoshiro) for seeded RNG |
| Schemas | [serde](https://serde.rs) + [schemars](https://github.com/GREsau/schemars) + JSON Schema validation in CI |
| Testing | `cargo test` matrix (Linux + macOS + Windows) + scripted E2E + run-bundle checker (Python `tools/prototype_run_check.py`) |
| Editor | [Visual Studio Code](https://code.visualstudio.com) with [rust-analyzer](https://rust-analyzer.github.io); [Helix](https://helix-editor.com) and [Zed](https://zed.dev) supported via per-project `.gitignore` |

---

## The Workspace

29 crates today (see [game/Cargo.toml](game/Cargo.toml)). Each crate carries its own `AGENTS.md` boundary contract.

```text
game/crates/
├── cf-app                  # Bevy app shell + window
├── cf-sim-core             # fixed-tick scheduler + RNG + checksum
├── cf-control              # JSON-RPC 2.0 control surface (cf-control)
├── cfctl                   # operator CLI (observe, run, scenario, settings, runbundle, system)
├── cf-replay               # run-bundle writer + event envelope
├── cf-actor                # actor records + control intent
├── cf-equipment            # role records + slot model
├── cf-chassis              # armor zones + modules + pilot binding
├── cf-physics              # collision matrix + CCD + impulse routing + GravityField (post-M5.5)
├── cf-terrain              # pixel terrain + chunk grid (post-M2)
├── cf-material             # systemic material kernel (post-M5.6)
├── cf-atmos                # Stationeers-grade atmospherics kernel (post-M5.9)
├── cf-mission              # mission director + objectives
├── cf-ai                   # perception + utility + doctrine + LLM mind hooks
├── cf-net                  # client/server transport (post-M9)
├── cf-render-2d            # wgpu 2D pipeline
├── cf-ui                   # comic-noir UI presentation
├── cf-audio                # sound + captions
├── cf-save                 # versioned .cfsave format
├── cf-mod                  # content schema validator + manifest
├── cf-tools-editor         # in-engine scenario / package / mod editors
├── cf-headless             # CI-friendly headless runner
├── cf-bench                # perf benchmark harness
├── cf-e2e                  # scripted end-to-end runner
├── cf-server               # multi-mode dedicated server (coop_room/pvp_arena/lan_room/mmo_shard/lobby_directory)
├── cf-server-ops           # ops dashboards + observability
├── cf-server-persistence   # MMO shard persistence
├── cf-server-anti-cheat    # anti-cheat foundation
└── cf-server-admin         # admin tooling
```

---

## Project Status

> [!warning] Pre-alpha
> Corefall is in active development. The repo is public so CI can run unrestricted (free GitHub Actions minutes for public repos), but the game is **not** ready to play yet.

| Milestone | Status | What It Proves |
|---|---|---|
| **M0 — Engine Bootstrap** | ✅ **Closed** ([PR #1](https://github.com/Madreag/corefall/pull/1) merged) | 29-crate workspace, JSON-RPC control plane, cfctl, replay run-bundle writer, deterministic 60 Hz / 120 Hz sim, panic capture, CI matrix on Linux + macOS + Windows. |
| **M1 — Actor Controller And Sim Core** | 🔄 **Active** | Single actor, fixed-tick controller, basic rifle loop, deterministic replay events. |
| M1.5 — Micro Breach Fun Slice | 🔜 Next | First-fun-evidence run before deeper systems land. |
| M2 — Pixel Terrain And Materials | ⏳ Planned | Deformable terrain + material kernel scaffold. |
| M3 — Replay And Event Recorder | ⏳ Planned | DR-002 v1 lock — full event recorder + viewer. |
| M4 — HUD And Comic-Noir UI | ⏳ Planned | Silhouette HUD + module strip + accessibility floor. |
| M5 — Equipment, Chassis, And Damage Grammar | ⏳ Planned | Per-origin chassis records + damage stages + wreck/eject/salvage. |
| M5.5 — Full Collision Gauntlet | ⏳ Planned | DR-033 closure: full collision + projectile-projectile + CCD tiers + universal gravity field integration. |
| M5.6 — Material Kernel | ⏳ Planned | DR-036 partial closure: chunked CA + reaction table + density layering. |
| M5.7 — Hazard Package | ⏳ Planned | Acid + electricity + debris + ingestion + affliction layer. |
| **M5.8 — Origin Resource & Overclock Pass** | 🆕 Proposed | Per-origin reaction matrix runtime: humans concuss, androids battery-drain, robots overclock + leak coolant. G-Force vision blackout HUD. |
| **M5.9 — Atmospherics-Grade Kernel** | 🆕 Proposed | DR-037 closure: real PV=nRT, 10 launch gases, 6 combustion reactions, pipe networks, suit life-support, per-planet ambient, universal gravity ballistic drag. |
| M6 — AI Core And Trust Harness | ⏳ Planned | DR-022 8-criteria humanlike bar testable. |
| M6.5 — LLM Mind Lab | ⏳ Planned | Async LLM mind layer; local AI never blocks; no API key required. |
| M6.6 — AI Material Competence | ⏳ Planned | AI hazard perception with reason labels. |
| M7 — Mission Director And Breach Contract | ⏳ Planned | Proof mission. A-FEEL gate. |
| M7.5 — Base Atmospherics (extended for Stationeers-grade per DR-037) | ⏳ Planned | Base modules wired into M5.9 kernel. |
| M8 — Scenario Editor And Mod Tools | ⏳ Planned | First-class in-engine editor at launch. |
| M8.5 — Material Lab | ⏳ Planned | Material/reaction lab for promotions to launch set. |
| M9 — Dedicated Server App | ⏳ Planned | `cf-server` multi-mode binary; SERVER-001..016 acceptance suite begins. |
| M10 — LAN Co-op | ⏳ Planned | Local 2-4 player co-op. |
| M11 — Online Co-op (Self-Hosted Dedicated Servers) | ⏳ Planned | Community-hostable online co-op. |
| M12 — Public PvP Arenas + Persistent MMO Shards | ⏳ Planned | DR-035 MMO-001..012 readiness gate. |

---

## Research Vault

Corefall is built from a deliberate, opinionated, evidence-tracked **research vault** that lives outside this repo:

```text
~/projects/cortex-command-repos-all/cortext_command_vault
```

The vault contains:

- **Decision records** (DR-001 through DR-038, plus open topics) — every major direction choice with pros, cons, evidence, revisit triggers.
- **Spec pages** for product promise, body damage, chassis/armor/mechs/origins, equipment/loadout, atmospherics & chemistry, gravity & ballistics, AI, replay, mission director, full collision physics, and more.
- **Comparable game audits** — local code audits of Cortex Command (CCCP), OpenSoldat, OpenLieroX, The Powder Toy, plus public-source / public-doc research on Noita, Stationeers, Barotrauma, Oxygen Not Included.
- **Research log** — chronological record of every research pass with source citations.
- **Prototype evidence** — run bundles + smokes + acceptance test results.

The vault is the long-term knowledge base. This repo is the implementation. The two are separate by design so the vault can survive engine changes, language changes, or fork events.

> [!note]
> The vault is currently a private workspace. If you want to contribute design research or comparable-game audits, open an issue here so we can route the conversation.

---

## Getting Started

### Prerequisites

| Tool | Version |
|---|---|
| Rust toolchain | 1.93.0 (pinned via `game/rust-toolchain.toml`) |
| Cargo | bundled with rustup |
| Python | 3.x (for `game/tools/prototype_run_check.py` run-bundle validator) |
| OS | Linux + macOS + Windows; Steam Deck floor target |

### Build And Run

```bash
git clone https://github.com/Madreag/corefall.git
cd corefall/game

# Workspace sanity
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Smoke runs (M0 / M1 placeholder scenarios)
cargo run -p cfctl -- observe --once --scenario m0_blank
cargo run -p cfctl -- run --scenario m0_blank --ticks 300 --tick-rate-hz 60 --paced --write-run-bundle
cargo run -p cf-app -- --scenario m0_blank --headless-smoke --run-seconds 5 --write-run-bundle

# Validate the run bundle
python3 tools/prototype_run_check.py ../prototype_runs/native/m0_*
```

### CLI Reference

`cfctl` is the operator + AI control client. The full surface is documented in the canonical vault roadmap (CLI Reference section); the M0 subset is:

| Command | What |
|---|---|
| `cfctl observe --once --scenario <id>` | One-shot snapshot of game state. |
| `cfctl observe --stream --hz <N>` | Stream observation frames at N Hz. |
| `cfctl run --scenario <id> --ticks <N> --paced --write-run-bundle` | Run a scenario for N ticks paced to wall clock. |
| `cfctl scenario load <id> [--seed <N>]` | Load a scenario (seed override is M0-rejected). |
| `cfctl pause` / `step --ticks <N>` / `version` | Sim control + protocol version. |

Post-M5+ CLI extensions (atmospherics, materials, gravity, ballistics, origin-state, suit, pipe-network, room) are documented in [the canonical roadmap](https://github.com/Madreag/corefall#research-vault).

---

## CI

GitHub Actions runs on every push and PR:

- `cargo fmt --all -- --check` (with `.gitattributes` locking LF line endings cross-OS)
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --release`
- `cf-mod validate content/`
- Schema drift check (`dump_schemas --check`)
- `cfctl observe smoke` + `cfctl run smoke` (60 Hz + 120 Hz)
- `cf-app headless run-seconds 5`
- Validate every produced run bundle through `tools/prototype_run_check.py`
- Enforce repo-root `prototype_runs/` path (M0.4-F7 guard)

Matrix: Linux + macOS + Windows.

---

## AI Code Review

This repo uses **Cursor Bugbot** as a GitHub App for advisory PR review. Bugbot's loop runs three iterations per push, and autofix commits are authored as `Cursor Agent <cursoragent@cursor.com>`. We treat Bugbot's findings and autofixes as **advisory**, not authoritative — every Cursor Agent commit is audited against the actual source before merge, and false positives are reverted via `git revert` (not force-push) so the audit trail stays intact. See [`AGENTS.md` § Cursor Bugbot Loop](AGENTS.md) for the full protocol.

The repo also ships a project-local Claude Code review skill at [`.claude/skills/corefall-review/`](https://github.com/Madreag/corefall/tree/main/.claude/skills/corefall-review) that runs a deeper review pass (diff review, full affected-code review, contract gap review, edge-case hunt, test audit, determinism / replay review, security, performance, `cfctl` observability, vault coherence, synthesis judge). Invoke via `/corefall-review <milestone-or-range>`.

---

## Contributing

Right now Corefall is in early implementation. The vault is the design source; this repo executes against it. If you want to contribute:

1. **File an issue first** — propose what you want to work on and we'll align it with the next milestone.
2. **Read [AGENTS.md](AGENTS.md)** — the full agent contract (covers AI workers and human contributors equally). Especially the Milestone Authority Stack, Milestone Acceptance Gate, Contract Integrity Gate, No-Compromise Performance Defaults, and Cursor Bugbot Loop sections.
3. **Branch from `origin/main`** with a milestone-prefixed name (`m1/scoped-feature` etc.). Direct commits to `main` are allowed for solo prototyping; PRs are required for any non-trivial change.
4. **Run Standard Validation locally** (`cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`) before pushing.

Per-crate `AGENTS.md` files in each `game/crates/cf-*/` directory describe owned APIs, public boundaries, common pitfalls, and source trails.

---

## License

Corefall is dual-licensed under your choice of:

- [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](https://opensource.org/licenses/MIT)

This is the standard Rust ecosystem dual-license, chosen so users can pick whichever license is most compatible with their project. Workspace-level license declaration lives in [game/Cargo.toml](game/Cargo.toml). Per-file `SPDX-License-Identifier` headers ship as the codebase grows.

> [!note]
> Inspiration credits and a usage ledger for any externally-derived material are tracked in the canonical vault's [`references/usage-ledger.md`](https://github.com/Madreag/corefall#research-vault). No code, assets, sprites, or audio from the inspiration games is copied into Corefall.

---

## Acknowledgements

Built on Rust. Built on Bevy. Inspired by Cortex Command, Noita, Stationeers, Barotrauma, The Powder Toy, OpenSoldat, Liero/OpenLieroX, Teardown, Oxygen Not Included, and Rain World. Made possible by every open-source maintainer who took the time to write a wiki, publish a GDC talk, push public source, or answer a Steam discussion thread at 2 AM.

---

<div align="center">

**[Project status](#project-status) · [Inspirations](#inspirations-and-credits) · [Tech stack](#tech-stack) · [Getting started](#getting-started) · [License](#license)**

*One field for gravity. One kernel for atmospheres. One source of truth for everything.*

</div>
