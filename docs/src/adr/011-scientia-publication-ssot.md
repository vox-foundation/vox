---
title: "ADR 011: Scientia publication manifest SSOT"
description: "Unifies Scientia, news, and scholarly submission around one publication manifest and digest-bound approvals."
category: "reference"
last_updated: 2026-03-25
training_eligible: true
---

# ADR 011: Scientia publication manifest SSOT

## Status

Accepted.

## Context

The repository has two adjacent but separate publication surfaces:

- `vox scientia` / `vox db` research ingestion and capability mapping.
- news syndication (`vox-publisher`, orchestrator `NewsService`, MCP `vox_news_*` tools).

The news path already enforces strong controls (digest-bound approvals and publish gates), but the scientific publication path had no first-class manifest lifecycle for journal-style interoperability.

## Decision

Adopt a single publication domain model centered on a canonical manifest persisted in Codex:

- New tables in `vox-db` publication domain:
  - `publication_manifests`
  - `publication_approvals`
  - `publication_attempts`
  - `scholarly_submissions`
  - `publication_status_events`
- Digest-bound approvals are the active approval model for publication workflows.
- `vox-publisher::publication::PublicationManifest` is the shared Rust contract type across community and scholarly workflows.
- `vox-publisher::scholarly::ScholarlyAdapter` is the adapter contract; `LocalLedgerAdapter` is the first integration path.
- News publishing writes through the publication manifest/attempt/state ledger while preserving existing community channels.

## Consequences

### Positive

- One lifecycle model for news and scientia publication artifacts.
- Clear provenance: immutable digest, dual approval counts, submission IDs, and status transitions.
- Reusable gate and approval logic across orchestrator, CLI, and MCP.

### Trade-offs

- Temporary overlap with legacy news approval tables during migration windows.
- Additional manifest synchronization responsibilities for callers that prepare content outside existing news files.

## Implementation notes

- DB ownership follows `docs/agents/database-nomenclature.md`.
- `vox scientia` now exposes publication lifecycle commands:
  - `publication-prepare`
  - `publication-approve`
  - `publication-submit-local`
  - `publication-status`
- MCP gains matching scientia publication tools for non-CLI clients.
