---
title: "How-To: Publish Scientia findings"
description: "Prepare, approve, and submit scientific findings from Vox Scientia using the publication manifest SSOT."
category: "how-to"
last_updated: 2026-03-25
training_eligible: true
---

# How-To: Publish Scientia findings

This workflow uses a single publication manifest in Codex (`publication_manifests`) with digest-bound approvals and scholarly submission tracking.

> Note: the current submit path is `publication-submit-local` via the local scholarly ledger adapter. For policy boundaries and automation SSOT, see `docs/src/architecture/scientia-publication-automation-ssot.md` and `docs/src/reference/scientia-publication-worthiness-rules.md`. For implementation roadmap detail, see `docs/src/architecture/scientia-publication-readiness-audit.md`.

## 1) Prepare a manifest

```bash
vox scientia publication-prepare \
  --publication-id ai-research-2026-03 \
  --author "Your Name" \
  --title "Research update: planning-aware agents" \
  docs/src/research/ai-research-2026-03.md
```

Optional: pass `--abstract-text`, `--citations-json <file>`, and `--scholarly-metadata-json <file>` (structured JSON for `scientific_publication`: authors with optional ORCID/affiliation, `license_spdx`, `funding_statement`, `competing_interests_statement`, `reproducibility`, `ethics_and_impact` — see `vox_publisher::scientific_metadata`). The same `--scholarly-metadata-json` flag works on `vox db publication-prepare`.

Use `--preflight` (or `publication-prepare-validated`) to run `vox_publisher::publication_preflight` before persisting. Use `publication-preflight` to inspect readiness JSON for an existing id. Use `publication-zenodo-metadata` to emit a Zenodo `metadata` object (stdout) for manual or scripted upload.

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

`publication-submit-local` uses the first scholarly integration (`local_ledger`) and writes a deterministic submission id plus lifecycle state in `scholarly_submissions`.

## 4) Inspect lifecycle state

```bash
vox scientia publication-status --publication-id ai-research-2026-03
```

The status payload includes:

- current manifest state
- active content digest + version
- approval count for that digest
- scholarly submission rows and external submission ids
