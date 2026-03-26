---
title: "Scientia publication failure playbook"
description: "Deterministic remediation for common publication gate and syndication failures."
category: "reference"
last_updated: 2026-03-25
training_eligible: true
---

# Scientia publication failure playbook

Symptoms link to **stable gate reason codes** from `vox_publisher::gate` and structured tool/CLI errors.

## Gate: `live publish blocked by gate`

JSON includes `blocking_reasons[].code`:

| Code | Meaning | Fast fix |
|------|---------|----------|
| `missing_db` | Live publish without VoxDb | Connect Codex / use `vox db` with a real store; dry-run remains allowed |
| `missing_dual_approval` | Fewer than two distinct approvers for this digest | Run `publication-approve` twice with different approver ids |
| `publish_not_armed` | Armed flag false | Set `VOX_NEWS_PUBLISH_ARMED=1` and/or `[orchestrator.news].publish_armed = true` |
| *(implicit)* | Combined dry-run | Tool `dry_run`, orchestrator `[news].dry_run`, or `syndication.dry_run` — any true keeps fan-out non-live |

## Retry: `malformed syndication outcome_json for digest …`

Latest attempt row for the manifest digest contains JSON that is not a `SyndicationResult`. **Fix:** inspect `publication_attempts.outcome_json` in `publication-status`; delete bad rows or re-run a clean `publication-publish` / `publication-route-simulate` after repair.

## Retry: `no syndication attempt outcome for current manifest digest`

No attempt recorded for the **current** manifest hash (content changed after last run). **Fix:** run `publication-publish` (or orchestrator tick) once to create an attempt row for the new digest.

## Scholarly: `unsupported VOX_SCHOLARLY_ADAPTER`

Supported: `local_ledger` (default), `echo_ledger`. **Fix:** unset `VOX_SCHOLARLY_ADAPTER`, or set to one of those names; unknown values error (no silent stub).

## Live publish: `live publish blocked by worthiness`

JSON usually includes `worthiness_score` and `floor`. **[news]** / env: `worthiness_enforce` + `worthiness_score_min`, or `VOX_SOCIAL_WORTHINESS_ENFORCE` and `VOX_SOCIAL_WORTHINESS_SCORE_MIN`. Applies on **CLI**, **MCP**, and **orchestrator** when live fan-out would run (not dry-run). **Fix:** raise manifest/preflight signals, lower the floor in config, or disable enforcement for that environment.

## Credentials

Syndication tokens resolve through **Clavis** (`vox_clavis::resolve_secret`) for `VOX_NEWS_*` / `VOX_SOCIAL_*` specs. **Fix:** `vox clavis doctor`, set canonical or alias env vars, or auth JSON per [Clavis SSOT](clavis-ssot.md).

## crates.io channel

If `crates_io` appears in routing, expect **explicit** non-success outcomes until a real adapter exists—never assume a crate was published.
