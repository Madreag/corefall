# Coherence Tier 4 — Gap-Filling Additions

**Status:** `active` — optional but recommended; can run in parallel with Tier 3
**Prerequisite:** Tier 1 + Tier 2 PRs merged (Tier 3 NOT required as prerequisite)
**Estimated effort:** AI-scale 30-45 minutes (single PR, 2 commits)
**Output:** 1 PR titled `specs: tier-4 coherence gaps (M0.5 schema locks + M11.4 server deployment)`

---

## Goals

Add 2 missing milestones that are currently distributed-but-undefined:

1. **Edit 4.1** — Create M0.5 — Universal Schema Locks (centralize event/save/config schema-lock work)
2. **Edit 4.2** — Create M11.4 — Self-Hosted Server Deployment (Docker / systemd / launchd / cloud templates)

After Tier 4 PR merges:
- M0.5 is the single home for "lock the schema before producers ship" work
- M11.4 makes it explicit how community members deploy `cf-server` (Docker / systemd / launchd / cloud)
- 41 → 43 active specs

---

## Edit 4.1 — Create M0.5 — Universal Schema Locks

### Problem

Schema-lock work is currently distributed:

- **M3A** locks event envelope v0.1 + 36 category baseline + per-category event types
- **M2.5-SCHEMA** (after Tier 3) locks deep damage event families
- **M9** locks server protocol + admin CLI commands
- **M9.10** locks 200+ settings keys + 7-tier hierarchy
- **M11.5** locks PvE survival event family

Each milestone locks ITS schemas. But there's no single "schema-lock manifest" that says "here are all locked schemas; bumping any requires a migration."

When schema v0.1 → v0.2 happens (future BP6+), there's no central manifest to validate against.

### Fix

Create **M0.5 — Universal Schema Locks** as a meta-milestone that:

1. Lists ALL locked schemas across all milestones with their version + locking milestone
2. Defines schema-bump migration policy (when v0.1 → v0.2 happens, all consumers need migration)
3. Ships `cf-mod validate --all-schemas` that runs every schema's conformance check in one pass
4. Documents the schema-version lifecycle

M0.5 doesn't replace per-milestone schema work; it's a meta-manifest + tooling layer.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M0.5.md` | **CREATE** |
| `README.md` | **MODIFY** (add to a new "BP0+" row at the very top of BP table; update spec count) |

### Step 1: Create `specs/active/M0.5.md`

```markdown
# M0.5 — Universal Schema Locks (Cross-Milestone Manifest)

## Status

`active`

## Intent

**M0.5 is the universal schema-lock manifest milestone** — a meta-milestone that catalogs every locked schema across the project, defines schema-bump migration policy, and ships `cf-mod validate --all-schemas` for one-shot conformance check.

M0.5 doesn't OWN schemas — they live in their respective milestones (M3A locks envelope; M2.5-SCHEMA locks damage events; M9 locks server protocol; M9.10 locks settings keys; M11.5 locks survival events). M0.5 is the **manifest** that knows about all of them.

**Why a separate milestone?** Without M0.5, schema-bump migrations (v0.1 → v0.2 in BP6+) would have to coordinate across every owning milestone independently. M0.5 ships the migration tooling + lifecycle docs so future schema bumps are a one-pass operation.

M0.5 promise: **"every schema in the project is registered in one place; bumping v0.1 → v0.2 runs through one tool."**

## Player-facing behavior

(None — M0.5 is infrastructure.)

## What M0.5 ships

### 1. Schema manifest file

`game/crates/cf-mod/manifest/all_schemas.ron` — registers every locked schema:

```ron
SchemaManifest (
    version: "v1",
    schemas: [
        Schema(
            id: "recorder_event.envelope.v0.1",
            owner_milestone: "M3A",
            file_path: "game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json",
            locked_at: "M3A",
            current_version: "0.1",
            consumers: ["M3B", "M4A", "M5", "M5.5", ...],
            migration_handler: None,  // v0.1 is the locked baseline
        ),
        Schema(
            id: "armor.layer_destroyed.v0.1",
            owner_milestone: "M2.5-SCHEMA",
            file_path: "game/crates/cf-replay/schemas/event/armor_layer_destroyed.json",
            locked_at: "M2.5-SCHEMA",
            current_version: "0.1",
            consumers: ["M5", "M5.5", "M11.7"],
            migration_handler: None,
        ),
        // ... ~120 schemas across 11 owning milestones
    ],
)
```

### 2. `cf-mod validate --all-schemas` tool

`cargo run -p cf-mod -- validate --all-schemas`:

1. Reads `manifest/all_schemas.ron`
2. For each schema, runs JSON schema validation against `file_path`
3. Verifies each schema's `schema_version` field matches the manifest's `current_version`
4. Verifies each consumer-mentioned milestone exists in `specs/active/` or `specs/done/`
5. Exits 0 on full pass; non-zero with structured error per failure

### 3. Schema-bump migration policy

Defined in `docs/plan/spec/schema-bump-migration-policy.md`:

```markdown
# Schema Bump Migration Policy

## When to bump (v0.1 → v0.2)

A schema bump is required when:

1. A new REQUIRED field is added to an existing event
2. An existing field changes type (string → int)
3. An existing enum gains a new variant that consumers MUST handle
4. The envelope shape itself changes (new top-level field)

A schema bump is NOT required when:

1. A new OPTIONAL field is added (with serde(default))
2. A new event type is added to an existing category
3. A new category is added (with all schemas at v0.1+)
4. Internal payload structure changes within an unchanged top-level field

## Migration procedure

When bumping v0.X → v0.(X+1):

1. Update `manifest/all_schemas.ron` for ALL affected schemas (set `current_version: "0.X+1"`)
2. Write a migration handler at `game/crates/cf-replay/migration/v0_X_to_v0_X_plus_1.rs`
3. Migration handler:
   - Reads old-format bundle events
   - Transforms each event to new format (lossless OR with documented field defaults)
   - Writes transformed events to new bundle
4. Validate every consumer milestone (read its spec; verify it accepts both old + new formats during migration window)
5. Update `cf-headless replay --migrate v0.X-to-v0.X+1 <bundle>` to migrate old bundles
6. Bump `SCHEMA_VERSION` constant in `cf-replay/src/lib.rs`
7. Run `cf-mod validate --all-schemas` to verify manifest + JSON files are consistent
8. Ship migration in a single PR titled `schema: bump v0.X → v0.X+1 (<reason>)`

## Backward compatibility

- Old bundles (v0.X) can be migrated to new format (v0.(X+1)) but NOT vice versa (migrations are one-way)
- During migration window (e.g. 2-3 BPs after bump), both old + new bundles are accepted by `cf-headless replay`
- After migration window: old bundles must be migrated first; replays without migration fail with structured error
```

### 4. Schema lifecycle docs

Added to existing `docs/plan/spec/determinism-island-contract.md` (M3A's contract document):

```markdown
## Schema lifecycle (managed by M0.5)

- All schemas register at locking milestone (e.g. M3A locks envelope; M2.5-SCHEMA locks damage events)
- M0.5's manifest tracks every schema's owner + version + consumers
- Schema bumps follow migration policy at `docs/plan/spec/schema-bump-migration-policy.md`
- Per-platform float-determinism rules remain owned by M3A (unchanged by M0.5)
```

## Schemas tracked by M0.5

(This list is exhaustive at M0.5 close; future milestones add to manifest.)

| Schema family | Locking milestone | Count |
|---|---|---|
| Event envelope (v0.1) | M3A | 1 |
| Run manifest (v1) | M3A | 1 |
| Run summary (v1) | M3A | 1 |
| Event categories baseline (36) | M3A | 1 (combined list) |
| Per-event type schemas (input / control / system / mission / actor / terrain / equipment / combat / ai / snapshot / determinism / ux / accessibility / performance) | M3A | ~40 |
| Damage event families (armor / internal / concussion / fluid / origin / hazard / affliction / atmos / shield / environment / thermal) | M2.5-SCHEMA | ~60-80 |
| Mission state + objective | M1.5 | 2 |
| Material registry (MaterialDef v1) | M2 | 1 |
| Chassis state (ChassisSpec v1) | M5 | 1 |
| Reactor state (M2.5) | M2.5 | 1 |
| AI difficulty preset (M1.5 + M2.2B extends) | M1.5 | 1 |
| Save blob (SaveBlob v1) | M5 | 1 |
| Server config (server.ron schema; 200+ keys) | M9.10 | 1 (combined) |
| Settings schema (per-key schema for all 200+ keys) | M9.10 | ~200 (one per key) |
| Loadout JSON | M5 | 1 |
| Scenario manifest | M1.5 | 1 |
| Storyteller event (M7 trait + 12 launch event types) | M7 | 12 |
| Recipe schema | M7.8 | 1 |
| Boss schema | M7 | 1 |
| Dialog tree schema | M7.1 | 1 |
| Quest schema | M7.1 | 1 |
| Procgen world schema | M11.5 | 1 |

**Estimated total: ~120 schemas across ~15 owning milestones at M0.5 close.**

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-mod` | MODIFY (deep) | Add `--all-schemas` subcommand + manifest loader |
| `cf-mod::manifest` | NEW | Schema manifest loader + validator |
| `cf-replay::migration` | NEW (stub) | Migration handler scaffolding (no migrations yet at M0.5; first migration is whenever the first schema bump happens) |

## Acceptance criteria

```gherkin
Scenario: cf-mod validate --all-schemas exits 0 at M0.5 close
  Given M0.5 manifest registers all ~120 schemas
  When cargo run -p cf-mod -- validate --all-schemas runs
  Then exit code is 0
  And output: "Validated 120 schemas across 15 owning milestones; all consistent"

Scenario: cf-mod validate --all-schemas rejects manifest drift
  Given a schema's file_path in the manifest doesn't exist on disk
  When validation runs
  Then exit non-zero with structured error: { rule: "schema_file_missing", schema_id, file_path }

Scenario: cf-mod validate --all-schemas rejects schema_version drift
  Given a schema file's schema_version="0.2" but manifest says "0.1"
  When validation runs
  Then exit non-zero with structured error: { rule: "schema_version_drift", schema_id, manifest_version: "0.1", file_version: "0.2" }

Scenario: Schema-bump migration policy documented
  Given docs/plan/spec/schema-bump-migration-policy.md exists
  Then it specifies:
    - When to bump (4 cases)
    - When NOT to bump (4 cases)
    - 8-step migration procedure
    - Backward compatibility window (2-3 BPs)
  And the policy is referenced from M3A's determinism contract

Scenario: Adding a new schema requires manifest update
  Given a new event type added at M11.7 (e.g. boss.killed expanded payload)
  When M11.7 ships
  Then manifest is updated with the new schema entry
  And cf-mod validate --all-schemas passes
  (Enforced by code review; not by tooling at M0.5)
```

## Dependencies

- **M3A (event envelope locked at v0.1) must close** — M0.5 builds on M3A's locked envelope
- **M2.5-SCHEMA (damage event surfaces locked) must close** — M0.5 manifests damage schemas
- All other locking milestones (M1.5, M2, M5, M9, M9.10) close their schemas before M0.5

## Closure procedure

Manifest file exists; `cf-mod validate --all-schemas` passes; migration policy doc exists; M0.5 → done.

## Cross-DR

DR-002 (replay envelope), DR-024, DR-052 (determinism).

## Implementer notes

M0.5 is **mostly tooling + documentation**. The implementer:

1. Reads every closed + active milestone spec and inventories locked schemas
2. Writes `game/crates/cf-mod/manifest/all_schemas.ron` with ~120 entries
3. Adds `--all-schemas` subcommand to `cf-mod` CLI
4. Writes `docs/plan/spec/schema-bump-migration-policy.md`
5. Updates `docs/plan/spec/determinism-island-contract.md` with the lifecycle section
6. Runs `cf-mod validate --all-schemas` to verify; commits when green

Estimated implementation: 4-8 hours human-time; AI-scale 30-90 minutes.
```

### Step 2: Modify `README.md`

Find the active spec count badge (was 41 after Tier 3; or 40 if Tier 3 skipped):

**BEFORE (if Tier 3 done):**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-41%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER (Edit 4.1 alone bumps to 42; Edit 4.2 adds 1 more):**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-42%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Note the range starts at M0.5 now (was M2.2A).

Find the Build Points table. At the very top, BEFORE the first BP3 row (which currently starts the table since BP0/BP1/BP2 are closed), add a BP0+ row for M0.5:

```markdown
| **BP0+** | **M0.5 — Universal Schema Locks (Cross-Milestone Manifest)** | Planned | Meta-milestone: schema manifest at `game/crates/cf-mod/manifest/all_schemas.ron` listing ~120 locked schemas across ~15 owning milestones + `cf-mod validate --all-schemas` one-shot conformance check + schema-bump migration policy at `docs/plan/spec/schema-bump-migration-policy.md`. M0.5 doesn't OWN schemas; it manifests them all. Future v0.1 → v0.2 bumps run through one tool. |
```

Note: BP0+ is a "between BP0 and BP1" slot. M0.5 closes whenever M3A + M2.5-SCHEMA close (it depends on them).

### Acceptance criteria for Edit 4.1

```bash
# File exists
test -f specs/active/M0.5.md && echo "PASS: M0.5.md exists" || echo "FAIL"

# Manifest + tooling specified
grep -q "all_schemas.ron" specs/active/M0.5.md && echo "PASS: manifest file specified" || echo "FAIL"
grep -q "cf-mod validate --all-schemas" specs/active/M0.5.md && echo "PASS: CLI specified" || echo "FAIL"

# Migration policy specified
grep -q "schema-bump-migration-policy.md" specs/active/M0.5.md && echo "PASS: migration policy specified" || echo "FAIL"

# README updated
grep -q "M0.5..M12" README.md && echo "PASS: README badge range updated" || echo "FAIL"
grep -q "M0.5 — Universal Schema Locks" README.md && echo "PASS: README BP0+ lists M0.5" || echo "FAIL"
```

### Commit message for Edit 4.1

```
specs: Edit 4.1 — add M0.5 — Universal Schema Locks meta-milestone

Schema-lock work was distributed across M3A (event envelope), M2.5-SCHEMA
(damage events), M9 (server protocol), M9.10 (settings keys), M11.5
(survival events), etc. No single place catalogs all locked schemas.

Created M0.5 as a meta-milestone that:

- Lists ALL locked schemas in game/crates/cf-mod/manifest/all_schemas.ron
- Ships `cf-mod validate --all-schemas` one-shot conformance check
- Documents schema-bump migration policy at
  docs/plan/spec/schema-bump-migration-policy.md
- Tracks ~120 schemas across ~15 owning milestones

M0.5 doesn't OWN schemas (other milestones do); it MANIFESTS them.
Future v0.1 → v0.2 schema bumps run through one tool.

- specs/active/M0.5.md created
- README.md updated (badge range M2.2A..M12 → M0.5..M12; BP0+ row
  added; spec count +1)
```

---

## Edit 4.2 — Create M11.4 — Self-Hosted Server Deployment

### Problem

`specs/active/M9.md` ships `cf-server` binary with 5 launch modes (coop_room / pvp_arena / lan_room / mmo_shard / lobby_directory). `specs/active/M11.md` ships online co-op via self-hosted servers.

But **how does a community member actually deploy `cf-server`?** Docker / systemd / launchd / cloud-image templates / configuration management?

M11 mentions "reference systemd / launchd / Docker configs" for self-hosted operators but doesn't spec them deeply. M9 mentions "reference Docker image" briefly. No dedicated deployment milestone.

### Fix

Create **M11.4 — Self-Hosted Server Deployment** between M11.3 (none currently) and M11.5 (PvE Survival). Ships full deployment toolkit: Docker images / systemd service files / launchd plists / docker-compose / cloud-init templates / Terraform modules / community-run server tutorial.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M11.4.md` | **CREATE** |
| `README.md` | **MODIFY** (add to BP10; update spec count) |

### Step 1: Create `specs/active/M11.4.md`

```markdown
# M11.4 — Self-Hosted Server Deployment

## Status

`active`

## Intent

**M11.4 is the self-hosted deployment milestone** — the toolkit + docs + reference configs that turn `cf-server` from a binary you can compile into a service a community member can actually run on a VPS / dedicated server / home network / cloud instance.

M9 ships the `cf-server` binary. M10 ships LAN co-op. M11 ships online co-op via self-hosted servers. But there's a gap: HOW does a non-developer deploy `cf-server`? M11.4 fills it with Docker images / systemd / launchd / docker-compose / cloud templates.

M11.4 promise: **"a Discord modder can deploy a public Corefall server in 15 minutes — no Rust toolchain, no kernel config, no obscure incantations."**

## Player-facing behavior

(M11.4 is operational, not gameplay. But community-hosted servers + active server browser = better player experience.)

## What M11.4 ships

### 1. Docker images (3 launch tiers)

**Tier 1: Quick-Start (single-machine, single-mode)**
- `corefall/cf-server:coop-room` — preconfigured `--mode coop_room`
- `corefall/cf-server:pvp-arena` — preconfigured `--mode pvp_arena`
- `corefall/cf-server:lan-room` — preconfigured `--mode lan_room`

One-line deploy:
```bash
docker run -d -p 7777:7777 corefall/cf-server:coop-room
```

**Tier 2: Production (single-mode, persistent volumes)**
- `corefall/cf-server:latest` — full binary with mode selection via env vars
- Mounts: `/data` (saves + ban list + audit logs) + `/mods` (mod packages) + `/config` (server.ron)

`docker-compose.yml` example:
```yaml
version: '3.8'
services:
  cf-server:
    image: corefall/cf-server:latest
    environment:
      - CF_MODE=coop_room
      - CF_CONFIG=/config/server.ron
    ports:
      - "7777:7777"
    volumes:
      - ./data:/data
      - ./mods:/mods
      - ./config:/config
    restart: unless-stopped
```

**Tier 3: MMO-shard cluster (multi-machine)**
- `corefall/cf-server:mmo-shard` + `corefall/cf-server:lobby-directory`
- `docker-compose.cluster.yml` orchestrates 4 shards + 1 lobby directory
- Per-shard volumes + shared lobby state

### 2. systemd service files (Linux)

`/etc/systemd/system/cf-server.service`:

```ini
[Unit]
Description=Corefall Dedicated Server
After=network.target

[Service]
Type=simple
User=cf-server
Group=cf-server
WorkingDirectory=/var/lib/cf-server
ExecStart=/usr/local/bin/cf-server --mode coop_room --config /etc/cf-server/server.ron
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits
LimitNOFILE=65535
MemoryMax=4G
CPUQuota=200%

# Security
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
NoNewPrivileges=true
ReadWritePaths=/var/lib/cf-server

[Install]
WantedBy=multi-user.target
```

Plus install script `install-cf-server-systemd.sh` that:
1. Creates `cf-server` user + group
2. Creates `/var/lib/cf-server` + `/etc/cf-server`
3. Copies binary + default config + service file
4. Runs `systemctl daemon-reload && systemctl enable cf-server`
5. Sets up log rotation via journald

### 3. launchd plists (macOS)

`/Library/LaunchDaemons/io.corefall.cf-server.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC ...>
<plist version="1.0">
<dict>
    <key>Label</key><string>io.corefall.cf-server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/cf-server</string>
        <string>--mode</string>
        <string>coop_room</string>
        <string>--config</string>
        <string>/etc/cf-server/server.ron</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>/var/log/cf-server.out.log</string>
    <key>StandardErrorPath</key><string>/var/log/cf-server.err.log</string>
</dict>
</plist>
```

Plus `install-cf-server-launchd.sh` script.

### 4. Cloud templates (AWS / GCP / Azure / DigitalOcean)

**Terraform module** at `tooling/terraform/cf-server/`:

```hcl
module "cf_server" {
  source = "github.com/Madreag/corefall//tooling/terraform/cf-server?ref=v0.11.4-rc"

  cloud_provider  = "aws"
  region          = "us-east-1"
  instance_size   = "t3.medium"
  mode            = "coop_room"
  ssh_key_name    = "my-key"
  open_ports      = [7777]
  storage_gb      = 50
  enable_metrics  = true
  enable_backups  = true
}
```

**cloud-init template** at `tooling/cloud-init/cf-server.yaml`:

```yaml
#cloud-config
users:
  - name: cf-server
    system: true
    home: /var/lib/cf-server
packages:
  - docker.io
runcmd:
  - docker pull corefall/cf-server:latest
  - docker run -d -p 7777:7777 ... corefall/cf-server:latest
```

### 5. Configuration management (Ansible / Puppet / Chef)

**Ansible playbook** at `tooling/ansible/cf-server.yml`:

```yaml
- hosts: cf-server
  become: yes
  tasks:
    - name: Install Corefall server
      apt:
        deb: https://releases.corefall.io/v0.11.4/cf-server_amd64.deb
    - name: Configure server
      template:
        src: server.ron.j2
        dest: /etc/cf-server/server.ron
    - name: Start service
      systemd:
        name: cf-server
        enabled: yes
        state: started
```

### 6. Reference monitoring + observability

- `tooling/monitoring/grafana-dashboard.json` — Grafana dashboard for `cf-server-ops` metrics endpoint
- `tooling/monitoring/prometheus-scrape-config.yml` — Prometheus scrape config
- `tooling/monitoring/loki-promtail-config.yml` — log aggregation config

### 7. Community-run server tutorial

`docs/server-hosting.md` (NEW; comprehensive guide):

```markdown
# Self-Hosted Server Hosting Guide

## Quick start (15 minutes — Docker)

1. Get a server (DigitalOcean droplet $5/month works fine)
2. Install Docker (`curl -fsSL https://get.docker.com | sh`)
3. Run: `docker run -d -p 7777:7777 corefall/cf-server:coop-room`
4. Connect from in-game lobby browser: search for your server's IP

## Production setup (1 hour — systemd + persistent storage)

[step-by-step guide]

## MMO shard cluster (1 day — multi-machine deployment)

[step-by-step guide]

## Common issues

- Port forwarding behind NAT
- Firewall (ufw / firewalld) configuration
- Backup + restore procedures
- Mod whitelist management
- Anti-cheat profile selection
- Log file rotation

## Performance tuning

[per-mode tuning guide]

## Community Discord

Join the Corefall ops community at [Discord link] for help.
```

### 8. Mod pack server templates

- `tooling/templates/server.ron.template-coop-vanilla`
- `tooling/templates/server.ron.template-pvp-arena`
- `tooling/templates/server.ron.template-pve-survival`
- `tooling/templates/server.ron.template-mmo-shard`
- `tooling/templates/server.ron.template-modded-roleplay`

Each template is a working `server.ron` with comments explaining every setting (200+ keys per M9.10).

## Acceptance criteria

```gherkin
Scenario: Docker quick-start deploys in 15 minutes
  Given a fresh Ubuntu 22.04 VM
  When `docker run -d -p 7777:7777 corefall/cf-server:coop-room` runs
  Then `cf-server` is running within 60 seconds
  And clients can connect from in-game lobby browser
  (Measured: 15 minutes from fresh VM to working server)

Scenario: systemd service starts cleanly + restarts on crash
  Given install-cf-server-systemd.sh has run
  When systemctl start cf-server runs
  Then service is active
  When the process is killed (simulating crash)
  Then systemd auto-restarts within 10 seconds
  And state recovers from /var/lib/cf-server/saves/

Scenario: launchd service on macOS persists across reboot
  Given install-cf-server-launchd.sh has run
  When the Mac reboots
  Then cf-server starts automatically
  And logs appear at /var/log/cf-server.out.log

Scenario: Terraform module provisions AWS instance
  Given the Terraform module + valid AWS credentials
  When `terraform apply` runs
  Then EC2 instance is provisioned with cf-server running
  And SSH key + ports + storage match the configuration
  And cf-server is reachable on port 7777

Scenario: Grafana dashboard shows metrics
  Given the Prometheus scrape config + Grafana dashboard imported
  When metrics are collected for 5 minutes
  Then the dashboard shows:
    - Tick rate (Hz)
    - Active player count
    - Memory usage
    - CPU usage
    - Network bandwidth
    - Save cadence
    - Crash count (should be 0)

Scenario: Backup + restore procedure works
  Given a cf-server running with state
  When the documented backup procedure runs:
    Then a backup tarball is created including saves + bans + audit log
  When the documented restore procedure runs against a fresh server:
    Then state is fully restored
    And clients reconnect to the same state

Scenario: Mod whitelist management
  Given a server admin wants to whitelist a community mod
  When they follow the documented procedure (cfctl admin mod whitelist add):
    Then mod is added to whitelist
    And client connections with that mod succeed
    And client connections without the mod fail with structured error

Scenario: 5 reference server.ron templates ship + validate
  Given tooling/templates/server.ron.template-*
  When `cargo run -p cf-mod -- validate tooling/templates/` runs
  Then all 5 templates validate against M9.10's settings schema
  And each template represents a distinct configuration pattern
```

## Content roster at M11.4

| Content | Roster |
|---|---|
| **Docker images** | 3 tiers (quick-start single-mode × 5 modes + production + MMO-cluster) |
| **systemd service files** | 1 (cf-server.service) + install script |
| **launchd plists** | 1 (io.corefall.cf-server.plist) + install script |
| **Cloud templates** | Terraform module + cloud-init template (covers AWS / GCP / Azure / DigitalOcean) |
| **Configuration management** | Ansible playbook (Puppet + Chef templates as stretch) |
| **Monitoring** | Grafana dashboard + Prometheus config + Loki/Promtail config |
| **Documentation** | docs/server-hosting.md (~2000 words; comprehensive) |
| **Server templates** | 5 reference server.ron templates |
| **Achievements** | adds "Public Server Host" achievement (track via server registration in lobby_directory) |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-server-ops` | MODIFY | Ensure metrics endpoint produces Prometheus-compatible output (M9 already promises this; verify) |
| `tooling/docker/` | NEW | Dockerfile + docker-compose + image build scripts |
| `tooling/terraform/cf-server/` | NEW | Terraform module |
| `tooling/cloud-init/` | NEW | cloud-init templates |
| `tooling/ansible/` | NEW | Ansible playbook |
| `tooling/monitoring/` | NEW | Grafana + Prometheus + Loki configs |
| `tooling/templates/` | NEW | 5 reference server.ron templates |
| `docs/server-hosting.md` | NEW (deep) | Hosting tutorial |

## Dependencies

- **M9 (cf-server foundation) must close** — M11.4 deploys M9's binary
- **M9.10 (server config + admin CLI) must close** — M11.4 templates author server.ron files per M9.10's schema
- **M11 (online co-op) must close** — M11.4 deploys public-facing servers per M11's lobby_directory pattern

## Closure procedure

Reference deploy on fresh DigitalOcean droplet (Docker) + reference deploy on Ubuntu VM (systemd) + reference deploy on macOS Mini (launchd) + reference Terraform apply on AWS. All 4 succeed. `docs/server-hosting.md` reviewed by 2 community members. PASS.

Move M11.4 → done/.

## Cross-DR

DR-005 (multiplayer), DR-013 (backend service scope), DR-024, **DR-034 (extends cf-server-ops with deployment)**, DR-052 (network determinism — verified across multi-shard deploy).

## Implementer notes

M11.4 is **mostly tooling + docs**. No new gameplay code. The implementer:

1. Builds Docker images (start with quick-start; then production; then MMO cluster)
2. Writes systemd + launchd service files + install scripts
3. Writes Terraform module + cloud-init + Ansible playbook
4. Writes Grafana + Prometheus + Loki configs
5. Authors `docs/server-hosting.md` tutorial
6. Authors 5 reference `server.ron` templates
7. Tests on actual cloud instance (DigitalOcean droplet works; total cost ~$1 for a few hours)
8. Submits for community-tester review (2 community members verify the hosting flow works for them)

Estimated implementation: 1-2 weeks human-time; AI-scale 2-4 hours.
```

### Step 2: Modify `README.md`

Find the active spec count badge (was 42 after Edit 4.1):

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-42%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-43%20%28M0.5..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Find the BP10 row for M11 and add a new row for M11.4 immediately AFTER M11 and BEFORE M11.5:

```markdown
| BP10 | **M11.4 — Self-Hosted Server Deployment** | Planned | Deployment toolkit for cf-server: 3 Docker image tiers (quick-start single-mode / production with volumes / MMO cluster) + systemd service files (Linux) + launchd plists (macOS) + Terraform module (AWS/GCP/Azure/DigitalOcean) + cloud-init templates + Ansible playbook + Grafana + Prometheus + Loki configs + `docs/server-hosting.md` comprehensive guide + 5 reference server.ron templates. 15-minute Discord-modder deploy target. |
```

### Acceptance criteria for Edit 4.2

```bash
# File exists
test -f specs/active/M11.4.md && echo "PASS: M11.4.md exists" || echo "FAIL"

# Deployment toolkit specified
grep -q "Docker images" specs/active/M11.4.md && echo "PASS: Docker section" || echo "FAIL"
grep -q "systemd service files" specs/active/M11.4.md && echo "PASS: systemd section" || echo "FAIL"
grep -q "launchd plists" specs/active/M11.4.md && echo "PASS: launchd section" || echo "FAIL"
grep -q "Terraform module" specs/active/M11.4.md && echo "PASS: Terraform section" || echo "FAIL"
grep -q "Grafana dashboard" specs/active/M11.4.md && echo "PASS: monitoring section" || echo "FAIL"
grep -q "docs/server-hosting.md" specs/active/M11.4.md && echo "PASS: docs section" || echo "FAIL"

# README updated
grep -q "active%20specs-43" README.md && echo "PASS: README badge 43" || echo "FAIL"
grep -q "M11.4 — Self-Hosted Server Deployment" README.md && echo "PASS: README BP10 lists M11.4" || echo "FAIL"
```

### Commit message for Edit 4.2

```
specs: Edit 4.2 — add M11.4 — Self-Hosted Server Deployment

M9 ships cf-server binary; M10/M11 ship LAN/online co-op via
self-hosted servers. But there was no spec for HOW a community member
actually deploys cf-server (Docker / systemd / launchd / cloud
templates / Terraform / monitoring / docs).

Created M11.4 as the deployment toolkit milestone:

- 3 Docker image tiers (quick-start / production / MMO cluster)
- systemd service files (Linux) + install scripts
- launchd plists (macOS) + install scripts
- Terraform module (AWS / GCP / Azure / DigitalOcean)
- cloud-init templates + Ansible playbook
- Grafana + Prometheus + Loki monitoring configs
- docs/server-hosting.md comprehensive guide
- 5 reference server.ron templates

Target: Discord modder deploys public Corefall server in 15 minutes.

- specs/active/M11.4.md created
- README.md updated (badge 42 → 43; BP10 table adds M11.4)
```

---

## Tier 4 — Full acceptance criteria

```bash
cd /Users/erol/projects/corefall

# Edit 4.1 checks
test -f specs/active/M0.5.md
grep -q "all_schemas.ron" specs/active/M0.5.md
grep -q "cf-mod validate --all-schemas" specs/active/M0.5.md
grep -q "M0.5..M12" README.md
grep -q "M0.5 — Universal Schema Locks" README.md

# Edit 4.2 checks
test -f specs/active/M11.4.md
grep -q "Docker images" specs/active/M11.4.md
grep -q "Terraform module" specs/active/M11.4.md
grep -q "M11.4 — Self-Hosted Server Deployment" README.md
grep -q "active%20specs-43" README.md

# File count
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "43"

# Workspace still builds
cd game && cargo build && cargo clippy --all-targets -- -D warnings
cd ..

echo "TIER 4 — ALL CHECKS PASS"
```

### Tier 4 PR template

**Title:** `specs: tier-4 coherence gaps (M0.5 schema locks + M11.4 server deployment)`

**Body:**

```markdown
## Summary

Tier 4 of the spec coherence pass per `specs/COHERENCE-PLAN.md`. Adds 2 missing milestones to fill gaps:

1. **Edit 4.1** — Create M0.5 — Universal Schema Locks (cross-milestone manifest of ~120 locked schemas + migration policy + `cf-mod validate --all-schemas` tool)
2. **Edit 4.2** — Create M11.4 — Self-Hosted Server Deployment (Docker / systemd / launchd / Terraform / cloud-init / Ansible / monitoring / docs)

## Active spec count

- Before: 41 (after Tier 3) OR 40 (if Tier 3 skipped — this PR can stand alone)
- After: 43 (added M0.5 + M11.4)

## Verification

All acceptance checks from `COHERENCE-TIER-4.md` § Tier 4 — Full acceptance criteria. All PASS.

## Coherence pass complete

After this PR + Tier 1 + Tier 2 (+ Tier 3 if done), the spec coherence pass is fully complete. See `specs/COHERENCE-PLAN.md` § Final acceptance for the master checklist.
```

---

## Done with Tier 4

Once the PR merges:
- ✅ M0.5 manifests all ~120 locked schemas + migration policy
- ✅ M11.4 ships full self-hosted deployment toolkit
- ✅ 43 active specs (range M0.5..M12)

---

## Master checklist — coherence pass complete

When Tier 1 + Tier 2 + Tier 3 + Tier 4 all merge, verify per `specs/COHERENCE-PLAN.md § Final acceptance`:

1. ✅ M7.8 has no hard dependency on M8.6 (Tier 1)
2. ✅ SmelterFurnace appears in exactly one spec (Tier 1)
3. ✅ M2.2A inventory has 3 reserved tank slots (Tier 1)
4. ✅ M5.8 references M7.6 / M5.9 / M5.10 for battery / tank / race-env data (Tier 1)
5. ✅ M7 + M7.1 + M7.2 each cover one coherent scope (Tier 2)
6. ✅ M11.5 + M11.6 + M11.7 each cover one coherent scope (Tier 2)
7. ✅ Boss schema defined once in M7, referenced elsewhere (Tier 2)
8. ✅ M5.7 has 22 afflictions (was 18; added hunger/thirst/sleep_dep/sanity_low) (Tier 2)
9. ✅ M2.5 + M2.5-SCHEMA split cleanly (Tier 3)
10. ✅ Storyteller API documented in M7 (Tier 3)
11. ✅ Damage-model specs have cross-reference headers (Tier 3)
12. ✅ M11.5 procgen acceptance covers all 12 worlds (Tier 3)
13. ✅ M0.5 — Schema Locks milestone exists (Tier 4)
14. ✅ M11.4 — Self-Hosted Server Deployment milestone exists (Tier 4)
15. ✅ README badge shows 43 active specs
16. ✅ README BP table reflects all new + split milestones
17. ✅ `cargo build` + `cargo test` + `cargo clippy` all green
18. ✅ `cargo run -p cf-mod -- validate content/` exits 0 (no spec/content drift)

All 18 boxes checked → spec coherence pass complete → M2.2A implementation can proceed cleanly.
