---
title: "Clavis Cloudless Ops Runbook"
description: "Operator procedures for Cloudless secret custody, backup/restore, rotation, and incident handling."
category: "operations"
last_updated: 2026-04-06
training_eligible: true
---

# Clavis Cloudless Ops Runbook

## Purpose

Define operator-grade procedures for running Cloudless secret persistence safely across local, canonical, and replicated VoxDB modes.

## Operational invariants

- No plaintext secrets in persisted database rows.
- Secret values never logged.
- All privileged actions produce auditable events.
- Rotation is mandatory after incident-driven privileged access.

## Key custody model

- Account-level secrets are encrypted with DEK-per-record.
- KEK references are managed by approved custody path (local keyring bootstrap and/or approved backend).
- Key version is tracked and rewrap is supported.

## Backup procedure (encrypted data only)

1. Verify cluster/store health.
2. Snapshot encrypted secret rows and key-reference metadata.
3. Verify snapshot integrity hash and store in approved backup location.
4. Record audit event with operator identity and reason.

## Restore procedure

1. Restore encrypted rows and key-reference metadata.
2. Validate key-reference availability before enabling reads.
3. Run integrity checks for ciphertext parse/decryptability.
4. Enable read path in staged mode; then full mode after verification.

## Rotation procedure

1. Select rotation scope (single secret class, account, global class).
2. Generate replacement credentials.
3. Update encrypted records with new version.
4. Revoke prior credential versions.
5. Verify all consumers pass readiness checks.

## Incident handling

1. Trigger incident record and severity.
2. Restrict access boundaries (least privilege).
3. Execute break-glass only if approved and required.
4. Rotate all affected credentials immediately after containment.
5. Publish post-incident findings and closure criteria.

## Replication and consistency notes

- Treat stale replica reads as non-authoritative for secret mutation checks.
- Use strict consistency for write-critical operations.
- For replica-latest modes, enforce deterministic stale-data error handling.

## Health checks

- Backend availability.
- Encryption/decryption roundtrip checks.
- Rotation backlog age.
- Audit log append health.
