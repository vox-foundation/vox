---
phase: 331-tolerant-cli-ergonomics-precomputed-semantic-routing
plan: 03
status: complete
requirements:
  - CLI-TOL-04
---

# Phase 331 Plan 03 Summary: Precomputed Semantic Synonym Compiler, Antonym Firewall & Lockfile

## Implemented
- Added optional `verb_aliases`, `noun_aliases`, and `foreign_idioms` to `CliProjection` (`crates/vox-cli/src/commands/ci/operations_catalog.rs`) and `contracts/operations/catalog.v1.schema.json`.
- Added `contracts/cli/semantic-aliases.v1.schema.json`, generated deterministic `contracts/cli/semantic-aliases.v1.json` (`3` verb aliases, `7` noun aliases, `4` foreign idioms), and registered `cli-semantic-aliases` / `cli-semantic-aliases-schema` in `contracts/index.yaml`.
- Created `crates/vox-cli/src/commands/ci/cli_synonyms.rs`, added `CiCmd::CliSynonyms { #[arg(long)] write: bool }` in `crates/vox-cli-ci/src/cmd_enums.rs`, wired `run_body.rs`, and hooked `cli_synonyms::run(repo_root, false)?` into `vox ci command-compliance` (`crates/vox-cli/src/commands/ci/command_compliance/mod.rs`).
- Enforced `ANTONYM_BANLIST` (`add`<->`remove`, `start`<->`stop`, `lock`<->`unlock`, `set`<->`get`, `import`<->`export`, `enable`<->`disable`, etc.), canonical path collision rejection, `MIN_COSINE_MARGIN = 0.08` via `vox_actor_runtime::llm::local_embed_text`, and optional structured LLM mining via `vox_actor_runtime::llm::llm_chat`.
- Embedded `contracts/cli/semantic-aliases.v1.json` into `crates/vox-cli/src/argv_normalize.rs` (`Stage 3a` exact foreign idioms before leaf elision + Stage 3 compositional verb/noun translation).

## Verified
- `cargo run -p vox-cli -- ci cli-synonyms --write`: passed.
- `cargo test -p vox-cli --lib cli_synonyms`: 4 passed, 0 failed.
- `cargo run -p vox-cli -- ci contracts-index`: `contracts-index OK (schema v1, 209 entries)`.
- `cargo run -p vox-cli -- ci command-compliance`: `cli-synonyms OK` + `command-compliance OK (registry schema v1, 1060 operations)`.
