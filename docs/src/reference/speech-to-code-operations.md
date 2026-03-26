---
title: "Speech-to-code — operations, security, rollout"
description: "Dashboards, privacy, canary, and release gates for spoken Vox codegen."
category: "reference"
last_updated: 2026-03-26
training_eligible: true
---

# Operations

## Observability

- Emit **correlation IDs** from Oratio/MCP (`correlation_id` JSON fields) and join with `RUST_LOG=vox_mcp_speech=debug`.
- KPI schema: [`contracts/speech-to-code/kpi-baseline.schema.json`](../../contracts/speech-to-code/kpi-baseline.schema.json).
- Benchmark manifest: [`contracts/speech-to-code/benchmark-fixtures.manifest.txt`](../../contracts/speech-to-code/benchmark-fixtures.manifest.txt).
- Schema drift guards: `cargo test -p vox-integration-tests --test speech_schema_parity`.
- Optional **canary gate**: set `VOX_SPEECH_CANARY_KPI` to a KPI JSON file and run `cargo test -p vox-integration-tests --test speech_canary` — thresholds default from [`canary_policy.example.json`](../../contracts/speech-to-code/canary_policy.example.json).

## Security and privacy

- Avoid persisting raw audio in shared logs; redact paths if needed. MCP `vox_oratio_listen` logs **path basename only** for protected path-like tokens when LLM polish rejects a correction.
- Speech trace / training rows: follow repo retention policy; use `mens/schemas/speech_to_code_trace.schema.json` only for **opt-in** export.
- Labeling rubric (human QA): [`contracts/speech-to-code/labeling_rubric.md`](../../contracts/speech-to-code/labeling_rubric.md).

## Release gates

- **Compile**: `cargo check -p vox-mcp -p vox-oratio -p vox-lsp`.
- **Quality**: MCP `validate_file` and `generate_vox_code` must use `validate_document_with_hir`.
- **Regression**: run `cargo test -p vox-oratio -p vox-lsp -p vox-corpus` speech-related tests.

## Canary / rollback (MENS)

- Promote speech-tuned checkpoints only when compile-pass@k on the frozen benchmark set improves vs baseline.
- Roll back if p95 latency or error-rate SLO regresses (define per deployment).

See [`speech-to-code-pipeline.md`](speech-to-code-pipeline.md).
