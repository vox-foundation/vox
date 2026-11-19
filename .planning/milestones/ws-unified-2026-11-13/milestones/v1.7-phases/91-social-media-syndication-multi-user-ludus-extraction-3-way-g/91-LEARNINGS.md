---
phase: 91
phase_name: "social-media-syndication-multi-user-ludus-extraction-3-way-g"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 9
  lessons: 0
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 91 Learnings: social-media-syndication-multi-user-ludus-extraction-3-way-g

## Decisions

### Hosted all 9 live social OAuth/webhook adapters and rate-limit windows i...
Hosted all 9 live social OAuth/webhook adapters and rate-limit windows inside vox-ops-syndication so local CLI binaries never require live social refresh tokens.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 91-01-SUMMARY.md

---

### Extracted twitter, reddit, youtube, discord, bluesky, mastodon, linkedin...
Extracted `twitter`, `reddit`, `youtube`, `discord`, `bluesky`, `mastodon`, `linkedin`, `hacker_news`, `opencollective`, and `canary` into `crates/vox-ops-syndication/src/adapters/`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-01-SUMMARY.md

---

### Defined SyndicationChannel and SyndicationHandoffPayloadV1 (#[serde(deny...
Defined `SyndicationChannel` and `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`) in `crates/vox-ops-syndication/src/lib.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-01-SUMMARY.md

---

### Implemented SyndicationOutboxStore, CLAIM_OUTBOX_BATCH_SQL (FOR UPDATE S...
Implemented `SyndicationOutboxStore`, `CLAIM_OUTBOX_BATCH_SQL` (`FOR UPDATE SKIP LOCKED`), `ENQUEUE_OUTBOX_SQL` (`ON CONFLICT (idempotency_key) DO NOTHING`), exponential backoff, and `DeliveryReceiptRow` recording in `crates/vox-ops-syndication/src/outbox.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-01-SUMMARY.md

---

### Extracted all 9 live social HTTP adapters (Twitter, Bluesky, Mastodon, D...
Extracted all 9 live social HTTP adapters (Twitter, Bluesky, Mastodon, Discord, Reddit, HackerNews, YouTube, LinkedIn, Forge) from public vox-publisher into vox-ops-syndication.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 91-02-SUMMARY.md

---

### Preserved SyndicationConfig andUnifiedNewsItem in vox-publisher so local...
Preserved SyndicationConfig andUnifiedNewsItem in vox-publisher so local Scientia preparation and scholarly pipelines compile and validate distribution manifests offline before exporting SyndicationHandoffPayloadV1.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 91-02-SUMMARY.md

---

### Created contracts/scientia/syndication-handoff.v1.schema.json and crates...
Created `contracts/scientia/syndication-handoff.v1.schema.json` and `crates/vox-publisher/src/handoff.rs` (`SyndicationHandoffPayloadV1`, `build_syndication_handoff_payload`, `validate_handoff_payload_schema`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-02-SUMMARY.md

---

### Deleted twitter.rs, bluesky.rs, mastodon.rs, discord.rs, reddit.rs, hack...
Deleted `twitter.rs`, `bluesky.rs`, `mastodon.rs`, `discord.rs`, `reddit.rs`, `hacker_news.rs`, `youtube.rs`, `linkedin.rs`, `forge.rs`, and `canary.rs` from `crates/vox-publisher/src/adapters/` and removed `bsky-sdk` + `atrium-api` from `crates/vox-publisher/Cargo.toml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-02-SUMMARY.md

---

### Added export_syndication_handoff_json to crates/vox-cli/src/commands/sci...
Added `export_syndication_handoff_json` to `crates/vox-cli/src/commands/scientia.rs` for offline handoff export.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 91-02-SUMMARY.md

## Lessons

## Patterns

### Idempotent per-channel outbox enqueueing via SHA-256(item_id || channel ...
Idempotent per-channel outbox enqueueing via SHA-256(item_id || channel || content_sha3_256) and dead-letter transition after max_attempts.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 91-01-SUMMARY.md

---

### Transactional outbox with FOR UPDATE SKIP LOCKED and SHA-256 idempotency...
Transactional outbox with FOR UPDATE SKIP LOCKED and SHA-256 idempotency keys

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 91-01-SUMMARY.md

---

### schema-validated JSON handoff artifact
schema-validated JSON handoff artifact

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 91-02-SUMMARY.md

---

### scholarly-vs-social separation of concerns
scholarly-vs-social separation of concerns

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 91-02-SUMMARY.md

## Surprises

