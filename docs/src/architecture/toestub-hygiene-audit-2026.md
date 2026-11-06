---
title: "TOESTUB & Code Hygiene Architecture Audit (2026)"
description: "Audit of TOESTUB (vox-code-audit), separation of concerns across Vox CI/CD gates, historical diff analysis, and precision/token-saving improvements."
category: "CI & Quality"
status: "current"
topic: "code-audit"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "gemini-3.1-pro"
training_eligible: false
---

# TOESTUB & Code Hygiene Architecture Audit (2026)

<!-- kb:summary -->
This document audits TOESTUB (`crates/vox-code-audit`), analyzes historical diff patterns and suppression churn across the Vox repository, establishes a strict Separation of Concerns (SoC) matrix between `vox-code-audit`, `vox-arch-check`, `vox-drift-check`, `vox-audit`, pre-commit hooks, and CI gates, and details the engine and detector improvements that eliminate false positives, prevent CRLF token-map drift, reduce AST re-parsing overhead, and save LLM and human developer tokens.
<!-- /kb:summary -->

## 1. Executive Summary & Historical Diff Findings

An audit of `crates/vox-code-audit` (`toestub`), `contracts/toestub/suppressions.v1.json`, and historical commit history revealed six systemic sources of developer/LLM friction, false positives, and token waste:

1. **CRLF Byte-Offset Drift in `TokenMap` Lookups (`rules.rs` & `rust_context.rs`):**
   - `byte_offset_in_file` and `line_is_prose_safe` accumulated line offsets via `off = off.saturating_add(line.len()).saturating_add(1)` while iterating `content.lines()`.
   - Because `str::lines()` strips both `\n` (1 byte) and `\r\n` (2 bytes), any file with CRLF line endings drifted by `-1` byte per line. By line 400, `TokenMap` lookups (`rust_byte_is_non_code`, `rust_byte_is_comment`) were shifted by 400 bytes, causing string literals (such as `detail: Some("todo!()|unimplemented!()".into())` in `crates/vox-gui/src/commands/policy.rs:409`) to be misclassified as executable Rust code.
   - **Fix:** Compute exact byte offsets in `O(1)` using slice pointer arithmetic (`line.as_ptr() as usize - content.as_ptr() as usize`).

2. **Whole-File Pre-Commit Scans Causing Suppression Churn (`lefthook.yml`):**
   - Over 80% of entries in `contracts/toestub/suppressions.v1.json` were added because `lefthook.yml`'s `tdd-guard` ran `toestub {staged_files} --rules skeleton --min-severity warning --mode enforce-strict` against the *entire* staged file whenever a contributor made a 1-line bugfix in a large legacy module with pre-existing `skeleton/untested-pub-api` warnings.
   - **Fix:** Scope pre-commit `tdd-guard` to Error-level regressions or diff-touched lines (`--diff`), while keeping whole-file/workspace coverage reporting in `vox audit` and CI ratchets.

3. **Unanchored Regex & Non-Code String Matches in Security/Policy Detectors:**
   - `CryptoBanDetector` (`crypto_ban.rs`): `cargo_banned_dep` used `(?:aegis|ring|aws-lc-rs)\s*=\s*\{` without a leading word boundary `\b`, falsely matching `keyring = { workspace = true }` as `ring` in `vox-cli`, `vox-db`, and `vox-secrets`. Its `rust_banned_use` regex also matched inside string literals (e.g., training prompts in `vox-corpus/src/training/spoke_roundtrip.rs`).
   - `EnvSecretShapeDetector` (`env_secret_shape.rs`): Matched bare variable identifiers (`std::env::var(key)` flagging `key` for `"KEY"`, and `std::env::var(vox_secrets::...)` flagging `"vox_secrets"` for `"SECRET"`), and flagged non-secret Clavis configuration knobs (`VOX_SECRETS_BACKEND`, `VOX_SECRETS_CUTOVER_PHASE`, `VOX_GEN_MAX_TOKENS`, `VOX_GUI_DRIVE_TOKEN_PATH`).
   - `MagicValueDetector` (`magic_value.rs`): Flagged scheme-prefix checks (`lower.starts_with("postgres://")`, `starts_with("sqlite:")`) and ephemeral in-memory SQLite URIs (`sqlite::memory:`).
   - `LlmProviderCallDetector` (`llm_provider_call.rs`): Flagged `println!` status messages containing provider URLs (`vox-ml-cli/src/commands/ai/train.rs`) and read-only `/v1/models` catalog discovery endpoints.

4. **Detector Self-Flagging on Inline `#[cfg(test)]` Suites:**
   - Policy detectors in `crates/vox-code-audit/src/detectors/*.rs` embed positive violation snippets inside `#[cfg(test)] mod tests` and `minimal_repro()`. Scanning `crates/vox-code-audit` without skipping `#[cfg(test)]` blocks caused detectors to flag their own unit tests.
   - **Fix:** Compute the byte/line start of `#[cfg(test)]` blocks in `RustFileContext` and skip test blocks in production-policy detectors (`crypto_ban`, `env_secret_shape`, `llm_provider_call`, `unregistered_llm_env`, `retired_*`, `god_object`).

5. **Redundant AST Parsing & Filesystem Syscalls During Scans:**
   - `ScalingSurfacesDetector` (`scaling.rs`, `scaling_support.rs`) and `CrossCrateDupDetector` (`cross_crate_dup.rs`) re-ran `syn::parse_file(&file.content)` 3–4 times per file instead of reusing `rust_ctx.ast`.
   - `SprawlDetector` (`sprawl.rs`) executed `std::fs::read_dir(parent).count()` once per file instead of once per directory.

---

## 2. Separation of Concerns (SoC) Across Hygiene & CI Tools

To prevent overlapping rules, duplicate AST/cargo passes, and confusing diagnostics, each quality tool in the Vox workspace owns a distinct boundary:

| Tool / Gate | Primary Responsibility | Scope & Granularity | Enforcement Stage | Must NOT Own |
| --- | --- | --- | --- | --- |
| **`toestub` (`vox-code-audit`)** | Fast static source-code & AST policy analysis (`stub/*`, `vox/crypto/*`, `vox/secret/*`, `vox/llm/*`, `retired/*`, `scaling/*`, `arch/organization`). | Per-file & workspace AST/token pass (`< 3s` incremental, `< 15s` workspace). | Pre-commit (`lefthook.yml`), inline agent harness (`post_verification.rs`), `vox audit code`, PR CI. | Cargo dependency graph validation, cross-doc link checking, or git commit history LLM judging. |
| **`vox-arch-check` (`vox audit arch`)** | Workspace crate layer boundaries, fan-in/LoC budgets (`docs/src/architecture/layers.toml`), and allowed dependency inversions. | `Cargo.toml` / `cargo metadata` crate graph level. | `vox ci pre-push`, PR CI (`arch-check`). | Intra-file Rust syntax or token-level lints. |
| **`vox-drift-check` (`vox ci ssot-drift`)** | Contract-to-code and doc-to-generator SSOT parity (`cli-command-surface`, `mcp-tools`, `capability-index`, `kb-taxonomy`). | Cross-artifact determinism & schema validation. | `vox ci pre-push`, PR CI (`ssot-autoregen` / `ssot-drift`). | Source-level code smells or stub detection. |
| **`vox-doc-pipeline`** | Markdown frontmatter governance, KB facet taxonomy validation, and `.vox` doctest compilation. | `docs/src/**/*.md` and `contracts/documentation/*`. | Pre-push (`scoped doc-lint`), PR CI. | Rust/TS source code linting. |
| **`vox-effort-audit` (`vox audit effort`)** | AI-judged longitudinal commit-history and token-efficiency auditing; routes systemic patterns to `vox-effort-route`. | Git commit range (`base..head`) asynchronous analysis. | Post-merge / milestone audit (`vox audit effort`). | Blocking synchronous pre-commit hooks. |
| **`rustc` / `clippy` / `rustfmt`** | Type checking, borrow checking, standard Rust idioms, and canonical formatting (`edition 2024`). | Compiler crate graph and syntax formatting. | Inline agent harness (`rustc --emit=metadata`), `vox ci pre-push`, PR CI. | Vox-specific domain policies (Clavis secrets, `vox-crypto`, LLM facade). |

---

## 3. Engine & Detector Improvements Implemented

### 3.1 Exact Pointer-Arithmetic Byte Offsets & Character-Literal `TokenMap` Scanning (`rules.rs`, `rust_context.rs`, `token_map.rs`)
- Replaced `off = off.saturating_add(line.len()).saturating_add(1)` with slice pointer offset calculation (`line.as_ptr() as usize - content.as_ptr() as usize`), eliminating CRLF (`\r\n`) drift across all `TokenMap` queries (`rust_byte_is_non_code`, `rust_byte_is_comment`, `line_is_prose_safe`).
- Added `try_scan_char_literal` to `TokenMap::from_rust_source` (`crates/vox-code-audit/src/analysis/token_map.rs`) so character and byte-character literals containing double quotes (`'"'`, `'\"'`, `b'"'`, such as `.trim_matches('"')` in `crates/vox-gui/src/commands/policy.rs:53`) are consumed as 3–4 byte character literals rather than opening an unclosed string span that inverts code/string classification for the remainder of the file.

### 3.2 `#[cfg(test)]` Boundary Detection & Resolving the `god_object` vs. `untested-pub-api` Contradiction (`rust_context.rs`, `god_object.rs`, `untested_pub_api.rs`)
- Added `cfg_test_ranges` (`#[cfg(test)]` items and `fn minimal_repro` bodies) to `RustFileContext` (`crates/vox-code-audit/src/analysis/rust_context.rs`).
- Production policy detectors (`crypto_ban`, `env_secret_shape`, `llm_provider_call`, `unregistered_llm_env`, `retired_*`, `god_object`, `stub`, `file_organization`) now ignore inline unit test fixtures and `crates/vox-code-audit/src/detectors/` self-test strings without losing coverage on production code.
- Resolved the structural contradiction where `arch/god_object` penalized files for having large inline `#[cfg(test)]` modules while `skeleton/untested-pub-api` (`tdd-guard`) penalized files when tests were extracted to sibling `<stem>_tests.rs` or crate `tests/` directories (`UntestedPubApiDetector::has_external_test_companion`).

### 3.3 High-Precision Security & Boundary Detectors
- **`CryptoBanDetector` (`crypto_ban.rs`):** Anchored `cargo_banned_dep` with `\b(?:aegis|ring|aws-lc-rs)\s*=\s*\{` so `keyring` is never matched as `ring`; gated `rust_banned_use` with `rust_byte_is_non_code` and `rust_byte_is_cfg_test`; excluded auto-generated `crates/workspace-hack/Cargo.toml`.
- **`EnvSecretShapeDetector` (`env_secret_shape.rs`):** Restricted matching to string literals (`dq`/`sq`), excluded `crates/vox-secrets/` (the Clavis implementation) and test files, and excluded non-secret configuration suffixes (`_TOKENS`, `_PATH`, `_URL`, `_PHASE`, `_BACKEND`, `_PROFILE`, `_HARD_CUT`, `_SERVICE`, `_VERSION`, `_ENV`, `_ID`, `_LOG`, `_SUNSET`, `_REF`, `_DIR`, `_HOME`, `_CACHE`, `_ENABLED`, `_MODE`, `_TIMEOUT`, `_PORT`, `_HOST`).
- **`MagicValueDetector` (`magic_value.rs`):** Excluded scheme-prefix predicates (`starts_with("postgres://")`, `starts_with("sqlite:")`, etc.) and ephemeral in-memory SQLite URIs (`sqlite::memory:`, `sqlite://:memory:`).
- **`LlmProviderCallDetector` (`llm_provider_call.rs`):** Excluded formatting/logging macros (`println!`, `eprintln!`, `tracing::*`) and read-only model catalog discovery endpoints (`/api/models`, `/v1/models`, `/v1beta/models?`) while maintaining strict `Error` enforcement on direct inference calls outside `vox_actor_runtime::llm`.

### 3.4 AST Reuse, Scoped Cross-Refs & Directory Caching (`engine.rs`, `scaling.rs`, `sprawl.rs`, `god_object.rs`)
- **`ToestubEngine` (`engine.rs`):** Gated `collect_workspace_cross_refs` to run only when `skeleton/unwired`, `skeleton/reachability`, or `unresolved-ref` rules are active, and gated `detect_cross_crate_dup_in_batch` on `min_severity <= Severity::Info` and `rule_filter`. Added comma-delimited `--rules` parsing and sub-segment rule matching (`--rules stub` matching `arch/stub`).
- **`ScalingSurfacesDetector`:** Reuses `rust_ctx.ast` instead of parsing each Rust file three additional times with `syn::parse_file`.
- **`SprawlDetector`:** Caches directory entry counts per parent path across the scan pass.
- **`GodObjectDetector`:** Excludes `#[cfg(test)]` blocks (`#[cfg(test)]` and `#[cfg(all(test, ...))]`), test/generated paths (`/tests.rs`, `.test.ts*`, `.spec.ts*`, `/e2e/`, `.generated.`), and comment/docstring lines (`//`, `/*`, `*`) when counting `nonblank_lines`, and classifies method density (`> 12` `fn` items) as `Severity::Warning` so `Severity::Error` aligns strictly with `docs/src/architecture/layers.toml` (`Error at 500 non-blank lines`). This reduced workspace `arch/god_object` errors from `1,604` to `235` (`-85.3%`).

---

## 4. God Object Decomposition & Workspace Remediation Results

To maximize LLM context efficiency and human readability, the top 5 monolithic Rust God Objects (`19,337` combined lines) were decomposed via parallel Gemini Flash subagents into 34 cohesive submodules, each strictly under the 500 non-blank code line threshold (`0` `arch/god_object` errors across all 5 modules and their 34 submodules):

| Original God Object | Before (Total / Non-Blank Lines) | After Decomposition (`< 500` Non-Blank Code Lines per File) | `arch/god_object` Errors |
| --- | --- | --- | --- |
| `crates/vox-corpus/src/training/spoke_roundtrip.rs` | `5,077` / `3,980` | `spoke_roundtrip.rs` (`153` nb) + 12 submodules under `spoke_roundtrip/` (`verifiers_core`, `verifiers_shell_mcp`, `verifiers_harness`, `verifiers_ext_spokes`, `suites_five_spoke`, `organic_types`, `organic_oracles`, `organic_eval`, `suites_organic_part1`, `suites_organic_part2`, `suites_organic_adversarial`, `tests`) | `0` |
| `crates/vox-orchestrator/src/models/registry.rs` | `4,215` / `3,679` | `registry.rs` (`468` nb) + 7 submodules under `registry/` (`catalog_seed`, `mens_spoke_routing`, `conversational_routing`, `conversational_workflows`, `sota_harness_eval`, `selection_queries`, `tests`) | `0` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | `4,326` / `1,792` | `agent_loop.rs` (`480` nb) + 5 submodules under `agent_loop/` (`turn_events`, `salvage_and_stream`, `context_and_memo`, `batch_and_eval_gate`, `tests`) | `0` |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` | `3,313` / `1,875` | `message.rs` (`391` nb) + 5 submodules under `message/` (`posture_and_routing`, `try_agent_turn`, `preamble_and_retrieval`, `persist_and_envelope`, `tests`) | `0` |
| `crates/vox-arch-check/src/main.rs` | `2,406` / `1,724` | `main.rs` (`392` nb) + 4 submodules (`config_types`, `report_model`, `git_and_walk`, `rule_runners`) | `0` |
