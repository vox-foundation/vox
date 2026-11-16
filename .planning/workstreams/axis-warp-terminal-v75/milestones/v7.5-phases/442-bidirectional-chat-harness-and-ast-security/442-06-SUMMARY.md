---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 06
subsystem: chat-terminal-citations
tags: [terminal, chat, mentions, citations, redaction, ecma-48, clavis, vox-orchestrator-mcp, utf8]

requires:
  - phase: 442-02
    provides: upgraded vox_redact::redact_owned and vox_secrets::{scrub_resolved_secrets, scrub_secret_needles}
  - phase: 442-03
    provides: TerminalBridge, TermReadRequest, TermBlockDto, ServerState::{resolve_terminal_bridge, bind_or_resolve_terminal_session}
provides:
  - Boundary-safe TERM_MENTION_RE and parse_term_mentions supporting all 7 locked @term citation forms without matching workspace files (@terminal.rs, @term_helper.vox, @term.rs) or emails (user@term.io)
  - Local 3-stage terminal citation scrubber (strip_ecma48_and_normalize_cr -> vox_redact::redact_owned -> vox_secrets::scrub_resolved_secrets) in term_mentions.rs with zero vox-terminal-core dependency edge
  - Non-spawning async expand_term_mentions with <terminal_block status="empty" ... /> fallback and 32 KiB (32_768-byte) UTF-8 char-boundary envelope cap
  - Pre-MENTION_RE @term extraction in mentions.rs + message.rs preserving raw_prompt_for_research and attaching <terminal_context> strictly after prompt_canonical::canonicalize_prompt
affects: [443, 445, vox-orchestrator-mcp]

actuals:
  tokens: 5844
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Pre-MENTION_RE @term span replacement ([ref @term:...]) + post-canonicalize_prompt <terminal_context> attachment preserving raw_prompt_for_research and stack-trace fidelity"
    - "Non-spawning implicit session resolution: consult bridge.list_sessions() before bind_or_resolve_terminal_session so @term on idle workspaces emits <terminal_block status=\"empty\" /> instead of spawning a PTY"

key-files:
  created:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs
  modified:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs

key-decisions:
  - "Combined word-boundary @term\\b capture in TERM_MENTION_RE with explicit trailing delimiter validation so workspace file mentions (@terminal.rs, @term_helper.vox, @term.rs, @term/mod.rs) and malformed suffixes (@term:sess_1#42:L0-5) never partially match"
  - "Replaced matched @term spans by exact byte offsets in a single left-to-right pass rather than String::replace so overlapping prefixes (@term vs @term:last) never corrupt neighboring citations"
  - "Checked bridge.list_sessions() emptiness before calling ServerState::bind_or_resolve_terminal_session for implicit @term/@term:last/@term:failed/@term:pinned selectors so typing @term with zero open terminals never spawns an idle PTY shell"

patterns-established:
  - "External runtime context blocks (<terminal_context>) bypass prompt_canonical::canonicalize_prompt and raw_prompt_for_research while still contributing to chat_grounding_score via mention_count"

requirements-completed:
  - TERM-HARNESS-04

coverage:
  - id: D1
    description: "TERM_MENTION_RE and parse_term_mentions parse all 7 @term forms (@term, @term:last, @term:failed, @term:pinned, @term:<sid>, @term:<sid>#<id>, @term:<sid>#<id>:L<a-b>) without capturing @terminal.rs, @term_helper.vox, @term.rs, user@term.io, or @rule:tdd"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs#tests::parse_term_mentions_extracts_all_seven_forms_and_ignores_workspace_files"
        status: pending-batched-wave3-cargo-pass
    human_judgment: false
  - id: D2
    description: "expand_term_mentions resolves blocks via ServerState::resolve_terminal_bridge without spawning idle PTYs, scrubs command and output via ECMA-48/CR + vox_redact::redact_owned + vox_secrets::scrub_resolved_secrets, and caps <terminal_context> at 32_768 UTF-8 bytes on valid char boundaries"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs#tests::expand_term_mentions_scrubs_three_stages_and_never_spawns_empty_pty"
        status: pending-batched-wave3-cargo-pass
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs#tests::expand_term_mentions_caps_envelope_at_32_kib_on_utf8_char_boundary"
        status: pending-batched-wave3-cargo-pass
    human_judgment: false
  - id: D3
    description: "mentions.rs skips term/term:* in MENTION_RE, and message.rs extracts @term before resolve_mentions_with_rules, keeps raw_prompt_for_research untouched, and appends <terminal_context> strictly after prompt_canonical::canonicalize_prompt"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs#tests::resolve_mentions_with_rules_skips_term_citations_and_expands_real_files"
        status: pending-batched-wave3-cargo-pass
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs#tests::message_pipeline_preserves_raw_research_prompt_and_attaches_post_canonical"
        status: pending-batched-wave3-cargo-pass
    human_judgment: false

completed: 2026-11-15
status: complete
---

# Phase 442 Plan 06: `@term` Pre-Canonicalizer Chat Citations, 3-Stage Scrubbing & `32 KiB` Envelope Summary

**Boundary-safe `@term` chat citation parser (`7` selector forms), local 3-stage ECMA-48/CR + `vox-redact` + Clavis scrubber, `32 KiB` UTF-8 `<terminal_context>` budget cap, and pre-`MENTION_RE` / post-`canonicalize_prompt` `message.rs` pipeline integration (`TERM-HARNESS-04`)**

## Performance

- **Tasks:** 2 / 2 implemented & formatted (`rustfmt --edition 2024 --check` clean across all 4 files)
- **Files created/modified:** 4 (`1` created, `3` modified; `0` `Cargo.toml` / `Cargo.lock` changes; `0` new `crate-edges`)
- **Commits:** skipped (shared working tree & batched Wave 3 orchestrator verification rule)

## Accomplishments

- **Task 1 — `TERM_MENTION_RE` Parser, 3-Stage Scrubber & `32 KiB` Citation Expander (`term_mentions.rs` + `chat/mod.rs`):**
  - Created `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` (`~370` non-blank non-test LoC, `8` fns, `0` new Cargo edges) and exported `pub mod term_mentions;` from `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs`.
  - Implemented `TERM_MENTION_RE` and `parse_term_mentions` supporting `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<sid>`, `@term:<sid>#<id>`, and 1-indexed inclusive `@term:<sid>#<id>:L<a-b>` (rejecting `a == 0` or `b < a`), replacing matched byte spans with `[ref @term...]` in left-to-right appearance order while deduplicating repeated selectors and leaving `@terminal.rs`, `@term_helper.vox`, `@term.rs`, `user@term.io`, and `@rule:tdd` untouched.
  - Implemented `scrub_term_citation_text` performing full ECMA-48 CSI/OSC/DCS/SOS/PM/APC/nF/2-byte ESC stripping + `\r` and `CSI K` progress-bar overwrite normalization (`strip_ecma48_and_normalize_cr`), followed by `vox_redact::redact_owned` and `vox_secrets::scrub_resolved_secrets` with zero dependency on `vox-terminal-core`.
  - Implemented `expand_term_mentions` resolving citations over `ServerState::resolve_terminal_bridge()`, checking `bridge.list_sessions()` before `bind_or_resolve_terminal_session` so empty sessions emit `<terminal_block status="empty" selector="..." />` without spawning idle PTY shells, and capping `<terminal_context>...</terminal_context>` at `MAX_TERM_PROMPT_ENVELOPE_BYTES = 32_768` bytes via `str::floor_char_boundary` with an intact closing `</terminal_context>` tag.
- **Task 2 — Pre-`MENTION_RE` Extraction & Post-`canonicalize_prompt` Attachment (`mentions.rs` + `message.rs`):**
  - Added `if filename == "term" || filename.starts_with("term:") { continue; }` inside `resolve_mentions_with_rules` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`) so direct callers and `[ref @term:...]` markers never misclassify terminal citations as workspace files.
  - Wired `super::term_mentions::expand_term_mentions` ahead of `resolve_mentions_with_rules` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`, kept `raw_prompt_for_research = params.prompt.clone()` untouched for `classify_research_intent`, attached `term_context_block` strictly after `prompt_canonical::canonicalize_prompt(&expanded_prompt, true, true)` via `attach_terminal_context`, and added `term_mention_count` into `mention_count` for `chat_grounding_score`.
  - Added unit and async pipeline tests in `mentions.rs` and `term_mentions.rs` covering all 7 citation forms, workspace-file non-interference, empty-session non-spawning, 3-stage scrubbing, `32 KiB` multi-byte UTF-8 truncation (`"🦀"`, `"═"`), and byte-for-byte preservation of stack traces and CLI flags across `canonicalize_prompt`.

## Exported Signatures

- `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs`:
  - `pub const MAX_TERM_PROMPT_ENVELOPE_BYTES: usize = 32 * 1024;`
  - `pub static TERM_MENTION_RE: LazyLock<Regex>`
  - `pub enum TermMentionSelector { LatestSessionSummary, LastBlock, FailedBlocks, PinnedBlocks, SessionTail { session_id: String }, Block { session_id: String, block_id: u64, line_range: Option<(usize, usize)> } }`
  - `pub struct ParsedTermMention { pub raw: String, pub selector: TermMentionSelector }`
  - `#[must_use] pub fn parse_term_mentions(prompt: &str) -> (String, Vec<ParsedTermMention>)`
  - `#[must_use] pub fn scrub_term_citation_text(raw: &str) -> String`
  - `pub async fn expand_term_mentions(prompt: &str, state: &ServerState, chat_thread_id: Option<&str>) -> (String, String, usize)`
  - `#[must_use] pub fn attach_terminal_context(canonical_prompt: String, term_context_block: &str) -> String`

## Files Created/Modified

- `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs` (`+702` total lines; `~370` non-blank pre-test LoC) — New `@term` parser, 3-stage scrubber, `32 KiB` async citation expander, `attach_terminal_context`, and 4 unit/async tests.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs` (`+1/-0`) — Exported `pub mod term_mentions;`.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs` (`+34/-0`) — Added `term` / `term:*` guard in `resolve_mentions_with_rules` and unit test `resolve_mentions_with_rules_skips_term_citations_and_expands_real_files`.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` (`+11/-2`) — Wired `expand_term_mentions` before `resolve_mentions_with_rules`, preserved `raw_prompt_for_research`, attached `<terminal_context>` post-`canonicalize_prompt`, and included `term_mention_count` in `mention_count`.

## Deviations from Plan

None - plan executed exactly as written.

## Batched Verification Handoff (Wave 3 Rule 3)

- `rustfmt --edition 2024 --check` verified clean (`exit 0`) across all 4 created/modified `.rs` files.
- Per Wave 3 Strict Execution Rule 3 (shared `vox-orchestrator-mcp` target-dir lock across parallel plans `442-04`, `442-05`, `442-06`), Cargo test/check execution was deferred to the parent orchestrator's unified Wave 3 verification pass:
  1. `timeout 300s cargo test -p vox-orchestrator-mcp --lib term_mentions`
  2. `timeout 300s cargo test -p vox-orchestrator-mcp --lib mentions`
  3. `timeout 300s cargo run -p vox-arch-check`

---
*Phase: 442-bidirectional-chat-harness-and-ast-security*
*Completed: 2026-11-15*
