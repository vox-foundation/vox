---
phase: 92
phase_name: "community-intelligence-flywheel-live-helios-deploy-history-scrub"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 92 Learnings: community-intelligence-flywheel-live-helios-deploy-history-scrub

## Decisions

### Authored /Users/brbrainerd/dev/vox-ops/Dockerfile (multi-stage rust:slim...
Authored `/Users/brbrainerd/dev/vox-ops/Dockerfile` (multi-stage `rust:slim-bookworm` builder -> `debian:bookworm-slim` runtime) and `/Users/brbrainerd/dev/vox-ops/deploy/helios-compose.yml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 92-03-SUMMARY.md

---

### Built vox-ops:latest on helios (178.156.212.19) under /opt/vox-ops and l...
Built `vox-ops:latest` on `helios` (`178.156.212.19`) under `/opt/vox-ops` and launched container `vox-ops-server` (`12cf029107d5`) on the `coolify` Docker network (`127.0.0.1:8420:8420`) wired to live PostgreSQL 17 container `q40ck888ggkkcsosgcwgc0oo` (`helios-postgres`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 92-03-SUMMARY.md

---

### GET http://127.0.0.1:8420/healthz -> {"fenced_roles":["vox_ops_owner","v...
`GET http://127.0.0.1:8420/healthz` -> `{"fenced_roles":["vox_ops_owner","vox_ingest_app","vox_ops_app"],"privilege_vector":"t|f|f|t|f|f|t","service":"vox-ops-server","status":"ok"}`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 92-03-SUMMARY.md

---

### POST http://127.0.0.1:8420/v1/logs with Authorization
`POST http://127.0.0.1:8420/v1/logs` with `Authorization: Bearer vox-ops-helios-token` and a schema-valid `vox.model_bench_case` OTLP batch -> `HTTP 202 Accepted` (`{"accepted_records":1,"role":"vox_ingest_app","status":"accepted"}`), while rejecting missing/malformed `vox.install_id` or non-allowlisted event names with HTTP `400 Bad Request`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 92-03-SUMMARY.md

---

### GET http://127.0.0.1:8420/v1/intelligence/manifest.json -> HTTP 200 OK r...
`GET http://127.0.0.1:8420/v1/intelligence/manifest.json` -> `HTTP 200 OK` returning a signed `CommunityIntelligenceBundleV1` (`bundle_version: "2026.10.19.1"`, `k_anonymity_min: 20`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 92-03-SUMMARY.md

## Lessons

## Patterns

## Surprises

