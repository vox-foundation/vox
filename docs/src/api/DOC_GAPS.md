---
title: "Known Documentation Gaps & Backlog"
description: "Living checklist of documentation gaps, backlog items, and recently completed doc work for contributors."
category: "api-crate"
status: current
last_updated: 2026-04-06

schema_type: "TechArticle"
---

# Known Documentation Gaps & Backlog

This is a living checklist for the Vox open source community and core contributors to track undocumented or under-documented language features.

## High Priority
- [ ] Add deep dive for `workflow` and `activity` compilation phases
- [ ] Document difference between `query` and `mutation` transactional boundaries natively
- [ ] Expand the `Codex` abstraction API reference 
- [ ] List all compiler auto-injected properties for `@table` types (`id`, `created_at`, `updated_at`)

## Medium Priority
- [ ] Explain the underlying generic instantiation (`<T>`) algorithm used by HIR logic
- [ ] Detail all `mcp.tool` options regarding rate limits and user confirmation schemas
- [ ] Add explicit HTTP request payload mapping examples for `@server` endpoints

## Completed 
- [x] Standard library built-ins (completed 2026-04-06)
- [x] Correct `@island` decorator syntax (completed 2026-04-06)
- [x] Example pipeline validation documentation (completed 2026-04-06)
