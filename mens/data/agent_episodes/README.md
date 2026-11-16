# Agent Episode Evaluation Benchmark (`agent_episodes` v1)

Deterministic scripted behavioral benchmark for Vox agent spokes (`vox-eval::episode`, `vox-eval::episode_controls`).

## Files

- `tool-catalog.v1.json` — Locked schema snapshot (`vox_tool_catalog_v1`) exported by `vox-orchestrator-mcp::tool_catalog` covering 11 live default-feature tools spanning `app`, `ai`, and `platform`.
- `episodes.v1.jsonl` — 72 evaluation episodes (12 per behavioral dimension: `right_tool`, `schema_valid_args`, `consumes_results`, `recovers_from_error`, `stops_when_done`, `refuses_out_of_scope`) validated against `tool-catalog.v1.json` with zero prompt leakage.
- `salvage-cases.v1.jsonl` — 12 shared tool-call parsing cases asserting parser parity between `vox-eval::episode::parse_tool_call` and `vox-orchestrator-mcp`'s runtime `salvage_tool_calls_from_text`.

## Regeneration & Verification

- Lock catalog check: `cargo test -p vox-orchestrator-mcp --lib tool_catalog` (set `VOX_UPDATE_TOOL_CATALOG=1` only when intentionally updating `tool-catalog.v1.json`).
- Control matrix & salvage parity: `cargo test -p vox-eval --lib` and `cargo test -p vox-orchestrator-mcp --lib salvage`.
