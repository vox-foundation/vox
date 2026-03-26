# Speech-to-code labeling rubric (human + LLM-assisted)

Use for MENS / internal QA rows exported under [`speech_trace.mens.schema.json`](speech_trace.mens.schema.json).

## Required fields per row

- `refined_transcript`: final ASR + refinement text shown to the generator.
- `vox_code`: canonical answer the user accepted (or gold reference).
- `schema_version`: `1`.
- Optional: `intent_action`, `failure_category`, `compile_ok`, `repair_attempts`.

## Labels

1. **intent_ok** — Routing/action matches what the user meant (even if code wrong).
2. **compile_ok** — `validate_document_with_hir` passes on `vox_code`.
3. **semantic_ok** — Code does what the transcript asks (manual or scripted check).
4. **verbatim_sensitivity** — Transcript contains literals/identifiers that must be copied exactly; flag if model paraphrased them away.

## Severity

- **blocker**: wrong intent, privacy leak in trace, or compile_ok=false on claimed success.
- **major**: compile_ok=true but semantic_ok=false.
- **minor**: style/naming only.

## Export

Store labels beside JSONL or in `intent_slots` / `intent_envelope` mirrors; never ship raw audio in public datasets without explicit consent.
