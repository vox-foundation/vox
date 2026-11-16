---
phase: 91-social-media-syndication-multi-user-ludus-extraction-3-way-g
plan: 02
subsystem: publisher
tags: [scientia, publisher, syndication-handoff,scholarly, offline]
requires:
  - phase: 91-social-media-syndication-multi-user-ludus-extraction-3-way-g
    provides: vox-ops-syndication server-side social worker
provides:
  - SyndicationHandoffPayloadV1 contract and builder in vox-publisher::handoff
  - Removal of 9 live social media adapters and bsky-sdk/atrium-api dependencies from vox-publisher
  - Local scholarly publishing (arXiv, Zenodo, OpenReview, Crossref, ORCID, RSS, Markdown) 100% functional offline
affects: [vox-publisher, vox-cli]
tech-stack:
  added: []
  patterns: [schema-validated JSON handoff artifact, scholarly-vs-social separation of concerns]
key-files:
  created:
    - contracts/scientia/syndication-handoff.v1.schema.json
    - crates/vox-publisher/src/handoff.rs
  modified:
    - crates/vox-publisher/Cargo.toml
    - crates/vox-publisher/src/lib.rs
    - crates/vox-publisher/src/adapters/mod.rs
    - crates/vox-publisher/src/publisher/mod.rs
    - crates/vox-cli/src/commands/scientia.rs
key-decisions:
  - "Extracted all 9 live social HTTP adapters (Twitter, Bluesky, Mastodon, Discord, Reddit, HackerNews, YouTube, LinkedIn, Forge) from public vox-publisher into vox-ops-syndication."
  - "Preserved SyndicationConfig andUnifiedNewsItem in vox-publisher so local Scientia preparation and scholarly pipelines compile and validate distribution manifests offline before exporting SyndicationHandoffPayloadV1."
requirements-completed: [SOCIAL-01, SOCIAL-02, SOCIAL-03, SOCIAL-04]
duration: 18min
completed: 2026-10-19
---

# Phase 91 Plan 02: Public Scientia Social Extraction & Handoff Contract Summary

**Removed 9 live social media adapters and `bsky-sdk`/`atrium-api` from `vox-publisher`, introduced schema-validated `SyndicationHandoffPayloadV1` (`#[serde(deny_unknown_fields)]`), and verified 100% offline scholarly publishing (`arXiv`, `Zenodo`, `OpenReview`, `Crossref`, `ORCID`, `RSS`).**

## Accomplishments
- Created `contracts/scientia/syndication-handoff.v1.schema.json` and `crates/vox-publisher/src/handoff.rs` (`SyndicationHandoffPayloadV1`, `build_syndication_handoff_payload`, `validate_handoff_payload_schema`).
- Deleted `twitter.rs`, `bluesky.rs`, `mastodon.rs`, `discord.rs`, `reddit.rs`, `hacker_news.rs`, `youtube.rs`, `linkedin.rs`, `forge.rs`, and `canary.rs` from `crates/vox-publisher/src/adapters/` and removed `bsky-sdk` + `atrium-api` from `crates/vox-publisher/Cargo.toml`.
- Added `export_syndication_handoff_json` to `crates/vox-cli/src/commands/scientia.rs` for offline handoff export.

## Verification
- `cargo test -p vox-publisher --lib` — **195 passed, 0 failed**.
