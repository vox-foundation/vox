# Speech-to-code contracts (Oratio → MCP → compiler → MENS)

JSON Schemas in this directory define **observability KPIs**, **failure taxonomy**, **project lexicon**, and **training trace** shapes for the speech-to-Vox pipeline.

| File | Purpose |
|------|---------|
| `kpi-baseline.schema.json` | Baseline and snapshot metrics (WER/CER, compile-pass, latency). |
| `failure-taxonomy.schema.json` | Tagged failure categories for telemetry and benchmarks. |
| `lexicon.schema.json` | Project speech lexicon (identifiers, aliases, pronunciation hints). |
| `speech_trace.schema.json` | Optional corpus / telemetry record for speech-origin sessions. |
| `speech_trace.mens.schema.json` | **MENS / SFT export**: same fields as `speech_trace` plus required `vox_code` (and optional `rating`). **`mens/schemas/speech_to_code_trace.schema.json`** `$ref`s this file. |
| `vox_grammar_artifact.json` | Lexer keyword/punctuator surface for constrained-decoding **hints** (validated by `vox-compiler` tests). |
| `labeling_rubric.md` | Human QA / training labels for speech-to-code rows. |
| `canary_policy.example.json` | Example KPI gates for speech canary promotion (optional CI via `VOX_SPEECH_CANARY_KPI`). |

See also: [`docs/src/reference/speech-to-code-pipeline.md`](../../docs/src/reference/speech-to-code-pipeline.md).
