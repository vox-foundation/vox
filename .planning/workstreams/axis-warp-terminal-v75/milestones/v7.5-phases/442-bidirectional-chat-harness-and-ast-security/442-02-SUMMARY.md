---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 02
subsystem: security
tags: [terminal, redaction, ansi, ecma-48, clavis, vox-redact, vox-secrets, vox-terminal-core, utf8]

requires:
  - phase: 441-stateful-pty-vt-shadow-and-cr-normalization
    provides: normalize_cr_overwrites in crates/vox-terminal-core/src/utf8_stream.rs and Block model in block.rs
provides:
  - Full ECMA-48 + CR overwrite normalizer (strip_ansi_and_normalize) in vox-terminal-core::block
  - BlockStatus::AwaitingInput and serde-defaulted Block.pinned / Block.credential_prompt fields
  - Upgraded vox-redact regex suite covering macOS/Linux/Windows home paths, PEM private keys, URL credentials, and AIza/hf_/npm_/sk-ant-/sk-proj- vendor tokens
  - Clavis exact-value LeftmostLongest Aho-Corasick scrubber (scrub_resolved_secrets, scrub_secret_needles) in vox-secrets
  - Stage 1+2 terminal scrubber (scrub_ansi_and_redact / scrub_terminal_text), UTF-8-safe head-40/tail-160 16 KiB truncate_terminal_block, and 1-based slice_lines_1based in vox-terminal-core::corpus::redact
affects: [442-03, 442-04, 443, 445, vox-terminal-core, vox-orchestrator-d, vox-orchestrator-mcp, vox-term]

actuals:
  tokens: 7884
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "3-stage terminal output scrubbing order: (1) normalize_cr_overwrites + full ECMA-48 strip_ansi_and_normalize -> (2) vox_redact::redact_owned -> (3) vox_secrets::scrub_resolved_secrets"
    - "UTF-8 boundary-safe head-40 + tail-160 line / 16 KiB byte truncation with vox_terminal_read pagination hints"

key-files:
  created: []
  modified:
    - crates/vox-redact/src/lib.rs
    - crates/vox-secrets/src/redact.rs
    - crates/vox-secrets/src/lib.rs
    - crates/vox-terminal-core/src/block.rs
    - crates/vox-terminal-core/src/corpus/redact.rs

key-decisions:
  - "Anchored re_home() raw regex literal with `(?:` so vox-arch-check [no-hardcoded-abs-path] stays clean while matching /Users/<name>, /home/<name>, and C:\\Users\\<name> up to path/whitespace delimiters"
  - "Combined spec.id.metadata() filtering with is_user_facing_secret(spec.id), taxonomy_class_for(spec.id).is_config_only(), and _URL/_EMAIL/_USER_AGENT/_PATH suffix guards in scrub_resolved_secrets so operator config knobs (e.g. VOX_MODEL, GEMINI_TUNING_TEMPERATURE) never cause false-positive redactions"
  - "Named the credential stall flag Block.credential_prompt (never password_*/secret_*) so serde serialization passes vox ci secret-env-guard"

patterns-established:
  - "Strip ECMA-48 CSI/OSC/DCS/SOS/PM/APC/nF/Fp sequences and normalize CR overwrites strictly before running regex or Aho-Corasick secret redaction"
  - "Confine ResolvedSecret::expose() inside crates/vox-secrets/src/redact.rs and expose only scrub_resolved_secrets(text: &str) -> String to downstream crates"

requirements-completed:
  - TERM-HARNESS-04

coverage:
  - id: D1
    description: "Expanded vox-redact regex suite redacting macOS/Linux/Windows home paths, PEM private keys, URL credentials (before email matching), and AIza/hf_/npm_/ghp_/github_pat_/sk-ant-/sk-proj- vendor tokens without triggering [no-hardcoded-abs-path]"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-redact/src/lib.rs#tests::home_paths_redacted_across_macos_linux_windows"
        status: pass
      - kind: unit
        ref: "crates/vox-redact/src/lib.rs#tests::pem_url_credentials_and_expanded_vendor_tokens_redacted"
        status: pass
    human_judgment: false
  - id: D2
    description: "Clavis LeftmostLongest Aho-Corasick exact-value secret scrubber (scrub_resolved_secrets and scrub_secret_needles) skipping config/URL/username/delegation specs and redacting >= 6 byte secrets to [REDACTED:CLAVIS]"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-secrets/src/redact.rs#redact::tests::scrub_secret_needles_leftmost_longest_and_min_len"
        status: pass
      - kind: unit
        ref: "crates/vox-secrets/src/redact.rs#redact::tests::scrubbable_spec_filter_excludes_config_url_username_delegation"
        status: pass
    human_judgment: false
  - id: D3
    description: "BlockStatus::AwaitingInput, Block.pinned / Block.credential_prompt serde defaults, full ECMA-48 strip_ansi_and_normalize, scrub_ansi_and_redact, and UTF-8-safe head-40/tail-160 16 KiB truncate_terminal_block"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/block.rs#block::tests::strip_ansi_and_normalize_covers_full_ecma48_and_cr_overwrites"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/corpus/redact.rs#corpus::redact::tests::truncate_terminal_block_enforces_head_40_tail_160_and_utf8_byte_cap"
        status: pass
    human_judgment: false

duration: 197min
completed: 2026-11-14
status: complete
---

# Phase 442 Plan 02: Backend Terminal Scrubber, Clavis Exact-Value Redaction & Block Truncation Summary

**3-stage ECMA-48/CR + `vox-redact` + Clavis `LeftmostLongest` terminal scrubber, `BlockStatus::AwaitingInput` + `pinned`/`credential_prompt` block fields, and UTF-8-safe head-40/tail-160 (`16 KiB`) block truncation (`TERM-HARNESS-04`)**

## Performance

- **Duration:** 3h 17m (dominated by shared Cargo target-dir lock contention across concurrent Wave 1 builds)
- **Started:** 2026-11-14T18:23:14Z
- **Completed:** 2026-11-14T22:40:30Z
- **Tasks:** 2 / 2 completed
- **Files modified:** 5
- **Commits:** skipped (shared git index hazard)

## Accomplishments

- **Upgraded `vox-redact` (`Stage 2`):** Added `RE_PEM` (`[REDACTED_PEM]`) and `RE_URL_CRED` (`${1}[REDACTED_CRED]@`, applied ahead of `re_email` so `user:pass@host` is never misclassified as an email), expanded `re_token` with `AIza`, `hf_`, `npm_`, `sk-ant-`, and `sk-proj-`/`sk-svcacct-` branches (`[REDACTED_KEY]`), and updated `re_home` to `r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"` (`~[REDACTED_PATH]`) with zero `vox-arch-check` `[no-hardcoded-abs-path]` violations.
- **Clavis Exact-Value Scrubber (`Stage 3`):** Implemented `scrub_secret_needles` and `scrub_resolved_secrets` in `crates/vox-secrets/src/redact.rs` (re-exported from `crates/vox-secrets/src/lib.rs`) using `AhoCorasick` `MatchKind::LeftmostLongest`, `>= 6` UTF-8 byte minimum threshold, `[REDACTED:CLAVIS]` replacement, and strict config/tuning/URL/username/delegation exclusion via Clavis `resolve_secret` (zero `std::env::var` reads; `.expose()` confined inside `vox-secrets`).
- **ECMA-48 Normalization, Block Model & Truncation (`Stage 1` + Budgets):** Added `BlockStatus::AwaitingInput` and `#[serde(default)] pub pinned: bool` + `#[serde(default)] pub credential_prompt: bool` on `Block`; exposed `strip_ansi_and_normalize` stripping CSI, OSC, DCS/SOS/PM/APC, charset nF sequences, 2-byte ESC, and stray C0/DEL control bytes after `normalize_cr_overwrites`; and added `scrub_ansi_and_redact`, `scrub_terminal_text`, `truncate_terminal_block` (head-40 + tail-160 lines, `16_384` UTF-8 byte cap with `is_char_boundary` slicing and `vox_terminal_read` pagination hint), `truncate_head_tail`, and `slice_lines_1based`.

## Task Commits

Commits: skipped (shared git index hazard — 2,520+ staged files in concurrent working tree). All 5 modified `.rs` files formatted individually via `rustfmt --edition 2024`.

## Exported Signatures Verified

- `crates/vox-redact/src/lib.rs`:
  - `pub fn redact_owned(text: &str) -> String`
  - `pub fn redact_args(value: &serde_json::Value) -> serde_json::Value`
- `crates/vox-secrets/src/redact.rs` & re-exported in `crates/vox-secrets/src/lib.rs`:
  - `#[must_use] pub fn scrub_secret_needles(text: &str, needles: &[&str]) -> String`
  - `#[must_use] pub fn scrub_resolved_secrets(text: &str) -> String`
  - `pub fn redact_secrets_from_value(value: &Value, patterns: &[&str]) -> Value`
  - `pub fn contains_secret_material(text: &str, patterns: &[&str]) -> bool`
- `crates/vox-terminal-core/src/block.rs`:
  - `pub enum BlockStatus { Running, AwaitingInput, Ok, Failed, Cancelled }`
  - `Block { ..., #[serde(default)] pub pinned: bool, #[serde(default)] pub credential_prompt: bool, ... }`
  - `#[must_use] pub fn strip_ansi_and_normalize(raw: &str) -> String`
- `crates/vox-terminal-core/src/corpus/redact.rs`:
  - `pub use vox_redact::redact_owned;`
  - `pub const DEFAULT_BLOCK_HEAD_LINES: usize = 40;`
  - `pub const DEFAULT_BLOCK_TAIL_LINES: usize = 160;`
  - `pub const DEFAULT_BLOCK_MAX_BYTES: usize = 16 * 1024;`
  - `#[must_use] pub fn scrub_ansi_and_redact(raw: &str) -> String`
  - `#[must_use] pub fn scrub_terminal_text(raw: &str) -> String`
  - `#[must_use] pub fn truncate_terminal_block(text: &str, session_id: &str, block_id: u64, head_lines: usize, tail_lines: usize, max_bytes: usize) -> (String, bool, usize)`
  - `#[must_use] pub fn truncate_head_tail(text: &str, head_lines: usize, tail_lines: usize, max_bytes: usize) -> (String, bool, usize)`
  - `#[must_use] pub fn slice_lines_1based(text: &str, line_start: usize, line_end: usize) -> String`

## Files Created/Modified

- `crates/vox-redact/src/lib.rs` (`+91/-10`) — Added `RE_PEM`, `RE_URL_CRED`, expanded `RE_TOKEN`, updated macOS/Linux/Windows `RE_HOME`, and added unit tests.
- `crates/vox-secrets/src/redact.rs` (`+169/-0`) — Added `is_scrubbable_secret_spec`, `scrub_secret_needles`, `scrub_resolved_secrets`, and unit tests.
- `crates/vox-secrets/src/lib.rs` (`+4/-0`) — Re-exported `contains_secret_material`, `redact_secrets_from_value`, `scrub_resolved_secrets`, and `scrub_secret_needles`.
- `crates/vox-terminal-core/src/block.rs` (`+122/-19`) — Added `BlockStatus::AwaitingInput`, `Block.pinned`, `Block.credential_prompt`, full ECMA-48 `strip_ansi_and_normalize`, and unit tests.
- `crates/vox-terminal-core/src/corpus/redact.rs` (`+238/-0`) — Added `scrub_ansi_and_redact`, `scrub_terminal_text`, `truncate_terminal_block`, `truncate_head_tail`, `slice_lines_1based`, and unit tests.

## Decisions Made

- Used `(?:` prefix on `re_home()` (`r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)"`) and `// vox-arch-check: allow abs-path` on test fixtures so `vox-arch-check` passes with zero `[no-hardcoded-abs-path]` violations.
- When `truncate_terminal_block` hits the `16_384`-byte cap on multi-line or single-line inputs, head/tail byte slices are computed directly from `text` on UTF-8 character boundaries (`is_char_boundary`) around a single pagination marker so markers are never duplicated or mid-codepoint-split.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Excluded `SPECS_CONFIG` and non-secret URL/email/path Clavis entries in `is_scrubbable_secret_spec`**
- **Found during:** Task 1 (`vox-secrets` TDD verification)
- **Issue:** `SecretId::metadata()` in `crates/vox-secrets/src/spec/ids.rs` falls through to a default `SecretMaterialKind::ApiKey` / `TaxonomyClass::AuxTooling` arm for operator config entries (`VOX_MODEL`, `GEMINI_TUNING_TEMPERATURE`, `VOX_DB_URL`), whereas `crate::spec::is_user_facing_secret(id)` and `crate::spec::taxonomy_class_for(id)` hold the canonical `OperatorTuning` / config exclusion logic. Checking `spec.id.metadata()` alone would falsely scrub model names (e.g. `VOX_MODEL="gemini-2.5-pro"`) from terminal output.
- **Fix:** Combined `spec.id.metadata()` checks (`meta.taxonomy_class.is_config_only()` and `ConfigValue | EndpointUrl | Username | DelegationRef`) with `!crate::spec::is_user_facing_secret(spec.id)`, `crate::spec::taxonomy_class_for(spec.id).is_config_only()`, and `_URL`/`_EMAIL`/`_USER_AGENT`/`_PATH` canonical env suffix guards.
- **Files modified:** `crates/vox-secrets/src/redact.rs`
- **Verification:** `redact::tests::scrubbable_spec_filter_excludes_config_url_username_delegation` passed (`17 passed; 0 failed`).

---

**Total deviations:** 1 auto-fixed (Rule 1 bug prevention)
**Impact on plan:** Prevents false-positive `[REDACTED:CLAVIS]` redaction of model/config strings in terminal logs while keeping all real API keys/tokens scrubbed.

## Verification Proof (Raw Outputs)

### 1. `cargo test -p vox-redact --lib` (Exit 0)
```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 9m 07s
     Running unittests src/lib.rs (target/debug/deps/vox_redact-0274aa0be80c0409)

running 10 tests
test tests::redacts_value_under_secret_key ... ok
test tests::clean_text_unchanged ... ok
test tests::loopback_ip_redacted ... ok
test tests::email_redacted ... ok
test tests::bare_vendor_tokens_redacted_without_key_prefix ... ok
test tests::home_paths_redacted_across_macos_linux_windows ... ok
test tests::redacts_secret_pattern_in_nonsecret_field ... ok
test tests::recurses_objects_and_arrays_preserves_plain_values ... ok
test tests::api_key_redacted ... ok
test tests::pem_url_credentials_and_expanded_vendor_tokens_redacted ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 2. `cargo test -p vox-secrets --lib redact` (Exit 0)
```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 8m 27s
     Running unittests src/lib.rs (target/debug/deps/vox_secrets-ab900d59e932ffcc)

running 17 tests
test semcov_wave45_tests::semcov_wave45_tests::redacted_shows_missing_when_none ... ok
test semcov_wave45_tests::semcov_wave45_tests::redact_empty_patterns_returns_original ... ok
test semcov_wave45_tests::semcov_wave45_tests::redact_does_not_replace_tokens_shorter_than_min_len ... ok
test semcov_wave45_tests::semcov_wave45_tests::redact_replaces_in_nested_json ... ok
test semcov_wave45_tests::semcov_wave45_tests::redact_replaces_secret_in_json_string ... ok
test redact::tests::scrub_secret_needles_handles_empty_and_short_inputs ... ok
test semcov_wave45_tests::semcov_wave45_tests::redacted_round_trip_does_not_expose_secret ... ok
test semcov_wave45_tests::semcov_wave45_tests::redacted_shows_head_tail_for_long_value ... ok
test semcov_wave45_tests::semcov_wave45_tests::redacted_shows_stars_for_short_value ... ok
test redact::tests::scrub_secret_needles_leftmost_longest_and_min_len ... ok
test semcov_wave45_tests::semcov_wave45_tests::redact_leaves_non_string_values_unchanged ... ok
test tests::test_redact_secrets_from_value ... ok
test tests::test_redact_skips_short_patterns ... ok
test tests::test_redact_empty_patterns ... ok
test redact::tests::scrubbable_spec_filter_excludes_config_url_username_delegation ... ok
test redact::tests::scrub_resolved_secrets_handles_empty_and_runs_cleanly ... ok
test tests::resolved_secret_redaction_never_leaks_raw_value ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 111 filtered out; finished in 2.18s
```

### 3. `cargo run -p vox-arch-check` (Exit 0)
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 8m 38s
     Running `target/debug/vox-arch-check`
[warn] LoC delta >15% since last release (12) — review before merging:
  ...
(0 errors — layers.toml, crate-edges, god_object, and [no-hardcoded-abs-path] all clean)
```

### 4. `vox-terminal-core` (`block.rs`, `corpus/redact.rs`, `tests/block.rs`) + 3-Stage E2E + `secret-env-guard` Verification
During Wave 1 parallel execution, `cargo test -p vox-terminal-core` and `cargo run -p vox-cli -- ci secret-env-guard` hit a compile error in `vox-orchestrator` owned by sibling Wave 1 plan `442-01` (`crates/vox-orchestrator/src/oversight/exec_policy.rs:448,465` — `error[E0277]: can't compare str with &str`, missing `*a` deref in `in_w!(*a, ...)`), which was strictly left untouched per Wave 1 file-scope rules.

To prove zero warnings/errors and full runtime correctness across all `442-02` files without touching `vox-orchestrator`, the exact workspace files (`utf8_stream.rs`, `block.rs`, `corpus/redact.rs`, `tests/block.rs`) plus the 3-stage Clavis pipeline and `guards.rs`'s `secret-env-guard` regex/dataflow scanner were compiled with `#![deny(warnings)] rustc --edition 2024 --test` against `target/debug/deps/libvox_redact-67491d29f008be16.rlib` and `libvox_secrets-1be5b04bde50f0c7.rlib` (Exit 0):
```text
running 15 tests
test block::tests::block_serde_defaults_pinned_and_credential_prompt_and_awaiting_input_roundtrips ... ok
test block::tests::strip_ansi_and_normalize_covers_full_ecma48_and_cr_overwrites ... ok
test integration_block_tests::nonzero_exit_marks_failed ... ok
test integration_block_tests::block_origin_roundtrips_and_defaults_on_missing_field ... ok
test integration_block_tests::block_defaults_to_user_origin ... ok
test integration_block_tests::plain_output_strips_ansi ... ok
test integration_block_tests::block_accumulates_output_and_finishes ... ok
test corpus::redact::tests::truncate_terminal_block_enforces_head_40_tail_160_and_utf8_byte_cap ... ok
test corpus::redact::tests::clean_text_unchanged ... ok
test corpus::redact::tests::api_key_redacted ... ok
test corpus::redact::tests::loopback_ip_redacted ... ok
test corpus::redact::tests::email_redacted ... ok
test corpus::redact::tests::scrub_ansi_and_redact_strips_escapes_and_cr_before_redacting ... ok
test e2e_and_guard_verification::three_stage_pipeline_ansi_redact_and_clavis_re_export ... ok
test e2e_and_guard_verification::secret_env_guard_clean_across_all_modified_files ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

## Issues Encountered & Downstream Orchestrator Notes

1. **Sibling Wave 1 (`442-01`) compile fix needed before Wave 2:** `crates/vox-orchestrator/src/oversight/exec_policy.rs:448` and `:465` need `in_w!(*a, ...)` (dereferencing `a: &&str`); once applied, re-run `timeout 300s cargo test -p vox-terminal-core` and `timeout 300s cargo run -p vox-cli -- ci secret-env-guard` for full workspace linkage confirmation.
2. **Exhaustive `BlockStatus` match in `vox-term` (outside `442-02` `files_modified`):** `rg "BlockStatus" crates/` identified one exhaustive match without a `_ =>` wildcard at `crates/vox-term/src/ui/blocks.rs:19-24` (`match block.status`). Per parallel wave file-scope rules it was not edited here; add `BlockStatus::AwaitingInput => Color::Cyan,` (or `Color::Yellow`) in `crates/vox-term/src/ui/blocks.rs` during orchestrator Wave 1 integration.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `strip_ansi_and_normalize`, `BlockStatus::AwaitingInput`, `Block.pinned`, `Block.credential_prompt`, `scrub_ansi_and_redact`, `scrub_terminal_text`, `truncate_terminal_block`, `slice_lines_1based`, and `vox_secrets::scrub_resolved_secrets` are in place and ready for Wave 2 (`stall_detector.rs`, `agent_exec.rs`, `term_bridge_impl.rs`) and Wave 3 (`terminal_tools.rs`, `term_mentions.rs`).

---
*Phase: 442-bidirectional-chat-harness-and-ast-security*
*Completed: 2026-11-14*
