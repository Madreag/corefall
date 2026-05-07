---
type: spec
status: closed-direction
authority: "Telemetry + crash reporting + in-game bug tool + AI-driven analysis. Sentry/GlitchTip. Privacy-by-default in EU. Anonymous opt-in elsewhere. AI agents triage + summarize weekly."
ready_when: "Crash reports symbolicate; bug tool F12 captures + uploads; gameplay telemetry opt-in flow GDPR-clean; AI weekly anomaly report runs."
feeds:
  - DR-013
  - DR-024
  - DR-029
  - DR-031
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-047-launch-and-live-operations|DR-047]] · [[spec/legal-and-compliance|legal]]

# Telemetry & Bug Tooling

## Crash Reporting

| Component | Detail |
|---|---|
| **Service** | Sentry (free tier 5K events/month) OR self-hosted GlitchTip (free; AGPL). |
| **Stack traces** | Symbolicated via `sentry-cli` upload of debug symbols on each release build. |
| **Auto-upload** | Consent prompt on first crash; remembered. Privacy-cleaned (no file paths, no chat content, no PII). |
| **Scope** | Panic + segfault + GPU hang + replay drift detected. |

## Anonymous Gameplay Telemetry

| Aspect | Detail |
|---|---|
| **Opt-in** | EU: default off, prompt on first launch; non-EU: default on, prompt on first launch. Disclosed in privacy policy. |
| **Captured** | Scenario id, mission outcome, time-to-death, weapon-of-death, faction picked, mods loaded (hash only), hardware specs (CPU/GPU/RAM/OS), crash signatures, perf counters. |
| **NEVER captured** | Chat content, player names (Steam ID hashed), inputs, file paths, mod content, save data. |
| **GDPR / CCPA / LGPD** | Right-to-deletion endpoint; data retention 12 months max; aggregate reports only. |

## Performance Telemetry

Frame ms / sim ms / dropped events / GPU memory / load times / VFX drop count / lighting drop count. Aggregated per-build. Drives perf regression detection.

## Balance Telemetry

TTK matrix per weapon/chassis combo; per-faction win-rate; per-mission completion-rate; per-mode dropout-rate. Drives M-BALANCE post-launch hotfix decisions.

## In-Game Bug Tool

**Trigger:** F12 in-game.

**Captures:**
- Screenshot (current frame)
- Last 30s replay snapshot
- Run-bundle attached
- User description prompt
- System info (already collected)
- Optional logs (anonymized)

**Uploads to:** Configurable endpoint (GitHub Issues / Sentry / dedicated bug-server). Privacy-cleaned.

## AI-Driven Analysis

Weekly auto-report by AI agent:

- Anomaly detection: sudden spike in crashes, balance outliers, regression candidates.
- Summary: top 5 issues by frequency + severity.
- Prioritized backlog suggestion.
- Email to project-owner.

## File Format

```ron
// content/telemetry/event_definition.ron
event: (
    id: "match.completed",
    capture: ["scenario_id", "match_kind", "outcome", "duration_s", "team_config"],
    privacy_clean: true,
    aggregate_at_endpoint: true,
)
```

## Done-Criteria

- [ ] Crash reports symbolicate.
- [ ] Bug tool F12 captures + uploads.
- [ ] Gameplay telemetry opt-in flow GDPR-clean.
- [ ] AI weekly anomaly report runs.
- [ ] Privacy policy auto-generated from event definitions.
- [ ] Right-to-deletion endpoint functional.

## Source Trail

- [[decisions/dr-047-launch-and-live-operations]]
- Sentry: https://sentry.io/
- GlitchTip: https://glitchtip.com/
- sentry-rs: https://docs.rs/sentry/
