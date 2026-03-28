---
title: "How-To: Publish Scientia findings"
description: "Prepare, approve, and submit scientific findings from Vox Scientia using the publication manifest SSOT."
category: "how-to"
last_updated: 2026-03-25
training_eligible: true
---

# How-To: Publish Scientia findings

This workflow uses a single publication manifest in Codex (`publication_manifests`) with digest-bound approvals and scholarly submission tracking.

> Note: scholarly submit defaults to `local_ledger` (`VOX_SCHOLARLY_ADAPTER`). For architecture and lingo, see [VoxGiantia publication architecture](../architecture/voxgiantia-publication-architecture.md). For operator inputs vs derived fields, see [operator inputs](scientia-publication-operator-inputs.md). For remediation, see [publication playbook](../reference/scientia-publication-playbook.md). Policy SSOT: [scientia-publication-automation-ssot](../architecture/scientia-publication-automation-ssot.md), [worthiness rules](../reference/scientia-publication-worthiness-rules.md), [readiness audit](../architecture/scientia-publication-readiness-audit.md).

## 1) Prepare a manifest

```bash
vox scientia publication-prepare \
  --publication-id ai-research-2026-03 \
  --author "Your Name" \
  --title "Research update: planning-aware agents" \
  docs/src/research/ai-research-2026-03.md
```

Optional: pass `--abstract-text`, `--citations-json <file>`, and `--scholarly-metadata-json <file>` (structured JSON for `scientific_publication`: authors with optional ORCID/affiliation, `license_spdx`, `funding_statement`, `competing_interests_statement`, `reproducibility`, `ethics_and_impact` — see `vox_publisher::scientific_metadata`). The same `--scholarly-metadata-json` flag works on `vox db publication-prepare`.

Use `--preflight` (or `publication-prepare-validated`) to run `vox_publisher::publication_preflight` before persisting. Use `publication-preflight` to inspect readiness JSON for an existing id (including `manual_required`, `confidence`, and live-publish gate hints when VoxDb is attached); add `--with-worthiness` to score against `contracts/scientia/publication-worthiness.default.yaml`. With `--with-worthiness`, VoxDb rolls up recent `socrates_surface` metrics into `metadata_json.scientia_evidence` when that block is empty (requires `repository_id` in metadata). You may also embed `scientia_evidence` manually (eval-gate result, baseline/candidate run ids, `human_meaningful_advance`, `human_ai_disclosure_complete`) so worthiness blends orchestrator telemetry with explicit human attestations. Use `publication-zenodo-metadata` to emit a Zenodo `metadata` object (stdout) for manual or scripted upload.

## 2) Record approvals (two distinct approvers)

```bash
vox scientia publication-approve --publication-id ai-research-2026-03 --approver alice
vox scientia publication-approve --publication-id ai-research-2026-03 --approver bob
```

Approvals are bound to the current content digest. If content changes, re-approve the new digest.

## 3) Submit to scholarly adapter

```bash
vox scientia publication-submit-local --publication-id ai-research-2026-03
```

`publication-submit-local` uses the scholarly adapter selected by `VOX_SCHOLARLY_ADAPTER` (default `local_ledger`; `echo_ledger` for deterministic/no-network tests) and writes submission metadata to `scholarly_submissions`. Unknown adapter names **error** (no silent fallback).

## 4) Inspect lifecycle state

```bash
vox scientia publication-status --publication-id ai-research-2026-03
```

The status payload includes:

- current manifest state
- active content digest + version
- approval count for that digest
- scholarly submission rows and external submission ids
- media assets, publication attempt timeline, and status event timeline

## 5) Optional social distribution metadata

To drive Reddit/Hacker News/YouTube planning from the same manifest, embed a
**`metadata_json.syndication`** object conforming to:

- `contracts/scientia/distribution.schema.json`
- `contracts/scientia/distribution.default.yaml`

Legacy manifests may still use **`metadata_json.scientia_distribution`**. At hydrate time the publisher **deep-merges** legacy + canonical keys (canonical `syndication` wins on conflicts), normalizes contract `channels` / `channel_payloads` into the flat runtime shape, and logs a deprecation warning when the legacy root is present. `vox db publication-preflight` surfaces the same hint under `manual_required`.

Optional **`metadata_json.topic_pack`**: set to a pack id from `contracts/scientia/distribution.topic-packs.yaml` (for example `research_breakthrough`). At hydrate time the pack **merges** worthiness floors, template profiles, and topic filters into the effective syndication config. **Channel allowlists** in the pack **drop** any channel not listed for that pack (after merge), so operators can tighten routing without editing every manifest.

**Minimum-input recipe:** set `topic_pack` + enable only the channels you need (or rely on pack allowlists). Omit per-channel payloads when the pack supplies policy; add `channel_payloads` / flat `twitter` / `reddit` blocks only for overrides.

Example skeleton:

```json
{
  "topic_pack": "research_breakthrough",
  "syndication": {
    "channels": ["reddit", "hacker_news", "youtube"],
    "channel_payloads": {
      "reddit": {
        "subreddit": "MachineLearning",
        "kind": "link"
      },
      "hacker_news": {
        "mode": "manual_assist"
      },
      "youtube": {
        "video_asset_ref": "artifacts/videos/demo.mp4",
        "privacy_status": "private"
      }
    },
    "distribution_policy": {
      "approval_required": true,
      "dry_run": true,
      "channel_policy": {
        "reddit": {
          "enabled": true,
          "template_profile": "deep_dive_selfpost",
          "worthiness_floor": 0.82,
          "topic_filters": {
            "include_tags": ["research_breakthrough", "benchmark"],
            "exclude_tags": ["internal_only"],
            "min_topic_score": 0.2
          }
        }
      }
    }
  }
}
```

Notes:

- Hacker News support is manual-assist only (official API is read-only).
- YouTube support uses OAuth refresh + resumable upload and should remain policy-gated by quota and audit readiness.
- `crates_io` is modeled in routing policy and outcomes; live publish adapter wiring remains intentionally explicit (non-implicit).
- `distribution_policy.channel_policy.*.template_profile` **does not change copy** unless `VOX_SYNDICATION_TEMPLATE_PROFILE=1` / `true` (then Twitter/Reddit/YouTube derived text caps follow named profiles such as `brief` / `roomy`; see `docs/src/reference/env-vars.md`).
- Configure social credentials via `VOX_SOCIAL_*` environment variables (`docs/src/reference/env-vars.md`).
- SSOT precedence is: manifest overrides > distribution policy defaults/contracts > runtime env overrides.

## 6) Route simulation and controlled fan-out

Use `vox db` for operator controls that are broader than the `vox scientia` convenience subset:

```bash
vox db publication-route-simulate --publication-id ai-research-2026-03
vox db publication-route-simulate --publication-id ai-research-2026-03 --json
vox db publication-publish --publication-id ai-research-2026-03 --channels reddit,youtube --dry-run true
vox db publication-publish --publication-id ai-research-2026-03 --channels reddit,youtube --dry-run true --json
vox db publication-retry-failed --publication-id ai-research-2026-03 --dry-run true
vox db publication-retry-failed --publication-id ai-research-2026-03 --dry-run true --json
```

Add `--json` for machine-readable stdout (one structured object per invocation). MCP equivalents `vox_scientia_publication_publish` and `vox_scientia_publication_retry_failed` accept **`json: true`** for a single-line compact JSON tool envelope.

**Retry-failed idempotency:** `publication-retry-failed` / MCP `vox_scientia_publication_retry_failed` pick candidates from the latest **digest-bound** attempt. Channels that already have a `Success` outcome for that digest are **not** republished (they appear as `skipped_success_channels`). Explicit `--channel` / `channel` follows the same planner so operators cannot accidentally duplicate a succeeded post when retrying a subset.
