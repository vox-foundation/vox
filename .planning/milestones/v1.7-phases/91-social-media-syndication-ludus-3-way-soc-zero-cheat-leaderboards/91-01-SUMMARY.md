---
phase: 91-social-media-syndication-multi-user-ludus-extraction-3-way-g
plan: 01
subsystem: syndication
tags: [syndication, outbox, social-adapters, postgres, skip-locked]
requires: ["90-02"]
provides:
  - "All 9 live social/community broadcast adapters (twitter, reddit, youtube, discord, bluesky, mastodon, linkedin, hacker_news, opencollective) + canary.rs in vox-ops-syndication"
  - "PostgreSQL syndication.outbox worker with FOR UPDATE SKIP LOCKED, ON CONFLICT (idempotency_key) DO NOTHING, rate_limit_windows, exponential backoff, and delivery_receipts"
affects: [vox-ops-syndication]
tech-stack:
  added: []
  patterns: ["Transactional outbox with FOR UPDATE SKIP LOCKED and SHA-256 idempotency keys"]
key-files:
  created:
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-syndication/Cargo.toml
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-syndication/src/lib.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-syndication/src/outbox.rs
    - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-syndication/src/adapters/mod.rs
  modified: []
key-decisions:
  - "Hosted all 9 live social OAuth/webhook adapters and rate-limit windows inside vox-ops-syndication so local CLI binaries never require live social refresh tokens."
patterns-established:
  - "Idempotent per-channel outbox enqueueing via SHA-256(item_id || channel || content_sha3_256) and dead-letter transition after max_attempts."
requirements-completed: [SOCIAL-01, SOCIAL-02]
duration: 15min
completed: 2026-10-19
---

# Phase 91 Plan 01: `vox-ops-syndication` Extraction & PostgreSQL Outbox Worker Summary

**Extracted all 9 live social/community broadcast adapters and `canary.rs` into `~/dev/vox-ops/crates/vox-ops-syndication` and implemented the `syndication.outbox` worker (`FOR UPDATE SKIP LOCKED`, `syndication.rate_limit_windows`, `syndication.delivery_receipts`).**

## Accomplishments
- Extracted `twitter`, `reddit`, `youtube`, `discord`, `bluesky`, `mastodon`, `linkedin`, `hacker_news`, `opencollective`, and `canary` into `crates/vox-ops-syndication/src/adapters/`.
- Defined `SyndicationChannel` and `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`) in `crates/vox-ops-syndication/src/lib.rs`.
- Implemented `SyndicationOutboxStore`, `CLAIM_OUTBOX_BATCH_SQL` (`FOR UPDATE SKIP LOCKED`), `ENQUEUE_OUTBOX_SQL` (`ON CONFLICT (idempotency_key) DO NOTHING`), exponential backoff, and `DeliveryReceiptRow` recording in `crates/vox-ops-syndication/src/outbox.rs`.

## Verification
- `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-syndication` — passed.
