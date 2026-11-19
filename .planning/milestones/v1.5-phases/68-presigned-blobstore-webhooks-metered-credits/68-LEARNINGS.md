---
phase: 68
phase_name: "presigned-blobstore-webhooks-metered-credits"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 68 Learnings: presigned-blobstore-webhooks-metered-credits

## Decisions

### Ship Upload[T] and BlobStore (CC-01) supporting local dev storage, S3/Cl...
Ship `Upload[T]` and `BlobStore` (`CC-01`) supporting local dev storage, S3/Cloudflare R2 presigned `PUT`/`GET` URLs, HTTP `Range` byte streaming (`206 Partial Content`), and a client-side OPFS chunked media cache helper.

**Rationale:** Primary phase objective established in plan.
**Source:** 68-01-PLAN.md

---

### Implement @webhook signature/replay verification in vox-codegen (emit_we...
Implement `@webhook` signature/replay verification in `vox-codegen` (`emit_webhook_guard_prelude`) and `vox-actor-runtime`, plus `@metered(credits: ...)` transactional credit reservation, debit on completion, and automatic refund on failure.

**Rationale:** Primary phase objective established in plan.
**Source:** 68-02-PLAN.md

## Lessons

## Patterns

## Surprises

