---
phase: 445-terminal-agentic-mode-and-mens-flywheel
plan: 01
subsystem: terminal
tags: [vox-terminal-core, vox-orchestrator-d, mens-flywheel, redaction, daemon-rpc, custom-blocks]

requires:
  - phase: 442-agent-pty-exec-stall-detection-and-safety-gates
    provides: SessionHost security/control modes, BlockOrigin, stall detection, and 3-stage output scrubbing
  - phase: 444-warp-editor-completions-and-runbooks
    provides: HistoryStore, CompletionEngine, runbooks, and SessionHost::voxify_subshell / VOXIFY RPC
provides:
  - Custom block streaming lifecycle (`begin_custom_block`, `append_block_chunk`, `finish_custom_block`, `cancel_custom_block`), `/clear` (`clear_unpinned_blocks`) and `/compact` (`compact_blocks`) `SnapshotResync` mutations, and `control_epoch` preemption on `Session`
  - 3-stage `CorpusWriter` + `corpus::redact` scrubbing (`strip_ansi_and_normalize` -> caller `extra_scrub` -> `vox_redact::redact_owned`), unconditional drop of `credential_prompt == true` blocks, `RedactingTranscriptSink` (`JournalSink` + `CorpusWriter` + `flush()`), and `DefaultPolicy` negative-preference retention
  - `SessionHost` extensions (`LocalHost` + `DaemonHost`), `Some("snapshot")` wire routing in `DaemonHost::route`, `TermDispatch` RPCs (`term.custom_open`, `term.custom_chunk`, `term.custom_finish`, `term.get_block`, `term.clear`, `term.compact`, `term.feedback`, `term.control_epoch`) with `refuse_agent_origin` guards, and durable `.vox/terminal-journal` wiring in `vox_orchestrator_d`
affects: [445-02, 445-03, vox-gui, vox-orchestrator-mcp, vox-terminal-core]

actuals:
  tokens: 13500
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Sibling submodule splits (`#[path = \"...\"]`) to keep new session/host/daemon/dispatch surfaces modular and < 350 non-blank LoC"
    - "3-stage pre-disk MENS scrubbing (`strip_ansi_and_normalize` -> Clavis `scrub_resolved_secrets` -> `vox_redact::redact_owned`) with unconditional drop of `credential_prompt == true` blocks"
    - "Wire-level origin separation (`\"block_origin\"` payload vs `\"origin\"` request envelope) so `refuse_agent_origin` protects human-only mutations (`term.clear`, `term.compact`, `term.feedback`)"

key-files:
  created:
    - crates/vox-terminal-core/src/session_agent.rs
    - crates/vox-terminal-core/src/host_agent.rs
    - crates/vox-terminal-core/src/daemon_host_ops.rs
    - crates/vox-orchestrator-d/src/term_session_ops.rs
  modified:
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-terminal-core/src/session.rs
    - crates/vox-terminal-core/src/commands.rs
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/daemon_host.rs
    - crates/vox-terminal-core/src/transcript.rs
    - crates/vox-terminal-core/src/corpus/redact.rs
    - crates/vox-terminal-core/src/corpus/writer.rs
    - crates/vox-terminal-core/src/corpus/curate.rs
    - crates/vox-orchestrator-d/src/term_dispatch.rs
    - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
    - docs/src/api/vox-terminal-core.generated.md

key-decisions:
  - "Disambiguated `CUSTOM_OPEN` wire payload key as `block_origin` so `BlockOrigin` never collides with the string `origin: \"user\" | \"agent\"` wire guard enforced by `refuse_agent_origin`."
  - "Linked `term_session_ops.rs` via `#[path = \"term_session_ops.rs\"] pub mod term_session_ops;` inside `term_dispatch.rs` to stay strictly within the 16 allowed files without modifying `crates/vox-orchestrator-d/src/lib.rs`."
  - "Provided default `SessionHost` trait methods returning `anyhow::bail!(\"... unsupported by this SessionHost\")` and full overrides on both `LocalHost` and `DaemonHost`, preserving 100% compatibility with `vox-gui` and Phase 444-01 (`voxify_subshell`, `VOXIFY`)."

patterns-established:
  - "Pre-destructive journal flush: `LocalHost::clear_unpinned_blocks` and `LocalHost::compact_blocks` call `sink.flush()` before mutating session blocks and emitting `SessionEvent::SnapshotResync`."
  - "Autopilot -> Manual preemption: `LocalHost::set_control_mode` bumps `control_epoch` (cancelling running `AgentTurn` blocks) and sends `TermSignal::Interrupt` to active agent-origin PTY blocks."

requirements-completed: [TERM-AGENT-01, TERM-AGENT-03, TERM-AGENT-04]

coverage:
  - id: D1
    description: "Custom block streaming (`AgentTurn` / `SlashCommand`), `/clear` & `/compact` `SnapshotResync` mutations preserving pinned/running blocks, bounded unpinned eviction, and `control_epoch` preemption on `Session` + `CommandRegistry`."
    requirement: TERM-AGENT-01
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/session_agent.rs#tests"
        status: pass
    human_judgment: false
  - id: D2
    description: "3-stage `CorpusWriter` + `RedactingTranscriptSink` scrubbing (`strip_ansi_and_normalize` -> caller `extra_scrub` -> `vox_redact::redact_owned`), unconditional drop of `credential_prompt == true` blocks, and `DefaultPolicy` negative-preference (`Rejected`) & non-identical `Corrected` retention."
    requirement: TERM-AGENT-03
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/corpus/writer.rs#accepted_and_rejected_blocks_scrub_3_stages_and_drop_credential_prompts"
        status: pass
      - kind: unit
        ref: "crates/vox-terminal-core/src/corpus/curate.rs#default_policy_retains_nonempty_rejected_and_drops_empty_or_credential_or_identical_correction"
        status: pass
    human_judgment: false
  - id: D3
    description: "`SessionHost` extensions (`LocalHost` + `DaemonHost`), `DaemonHost::route` `Some(\"snapshot\")` mapping, `TermDispatch` RPCs (`term.custom_open`, `term.custom_chunk`, `term.custom_finish`, `term.get_block`, `term.clear`, `term.compact`, `term.feedback`, `term.control_epoch`) with `refuse_agent_origin` guards, and `.vox/terminal-journal` wiring in `vox_orchestrator_d`."
    requirement: TERM-AGENT-04
    verification:
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_session_ops.rs#custom_blocks_clear_compact_feedback_and_origin_guards_work_over_daemon"
        status: pass
    human_judgment: false

duration: 48min
completed: 2026-11-15
status: complete
---

# Phase 445 Plan 01: Core Session Custom Blocks, 3-Stage MENS Corpus Redaction, and Daemon RPC Host Summary

**Shipped custom `AgentTurn`/`SlashCommand` block streaming, `/clear` & `/compact` `SnapshotResync` mutations, `control_epoch` preemption, 3-stage MENS `RedactingTranscriptSink` (`JournalSink` + `CorpusWriter`), and loopback-guarded `term.*` daemon RPCs across `vox-terminal-core` and `vox-orchestrator-d` while preserving 100% of Phase 444-01.**

## Performance

- **Duration:** 48 min
- **Started:** 2026-11-15T15:01:00Z
- **Completed:** 2026-11-15T15:49:30Z
- **Tasks:** 3
- **Files modified:** 16 (4 created, 12 modified — left uncommitted in shared worktree per orchestrator Git rule)

## Accomplishments

- **Task 1 (`TERM-AGENT-01`, `TERM-AGENT-04`):** Added `session_agent.rs` on `Session` (`begin_custom_block`, `append_block_chunk`, `finish_custom_block`, `cancel_custom_block`, `clear_unpinned_blocks`, `compact_blocks`, `control_epoch`, `bump_control_epoch`, `set_max_blocks`) with UTF-8 boundary-safe 256 KiB custom-block byte capping, unpinned-only ring eviction (`DEFAULT_MAX_BLOCKS = 2_000`), `SnapshotResync { resync_required: true, .. }` emission on `/clear` and `/compact`, and `clear` + `compact` entries in `CommandRegistry`.
- **Task 2 (`TERM-AGENT-03`):** Extended `TranscriptSink` with `Send + Sync + 'static` and `flush()`, added 3-stage `scrub_ansi_and_redact_with` / `redact_block_with` in `corpus::redact`, hardened `CorpusWriter::record_with` to scrub both `Accepted { block }` and `Rejected { block }` (`input`, `output`, `cwd`, `git_branch`) and unconditionally drop `credential_prompt == true` blocks before disk writes, added `RedactingTranscriptSink` fan-out (`JournalSink` + `CorpusWriter`), and updated `DefaultPolicy::decide` to retain non-empty non-credential `Rejected` blocks and non-identical `Corrected` events.
- **Task 3 (`TERM-AGENT-01`, `TERM-AGENT-03`, `TERM-AGENT-04`):** Extended `SessionHost` (`LocalHost` + `DaemonHost`), fixed the missing `Some("snapshot")` arm in `DaemonHost::route`, added `Autopilot -> Manual` `bump_control_epoch()` + agent PTY `TermSignal::Interrupt` preemption, implemented the 8 `TermDispatch` RPCs in `term_session_ops.rs` enforcing `refuse_agent_origin` on `term.clear`, `term.compact`, and `term.feedback`, wired `LocalHost::open_default_journal_sink` under `.vox/terminal-journal` with `vox_secrets::scrub_resolved_secrets` in `vox_orchestrator_d`, and synced crate API docs via `./target/debug/vox ci crate-api-sync --write`.

## TDD Gate Compliance

- **Task 1:** Wrote failing unit tests in `session_agent.rs` and `commands.rs` first -> observed RED (`4 failed` on `-- session commands`) -> implemented logic -> observed GREEN (`19 passed; 0 failed`).
- **Task 2:** Wrote failing unit tests in `corpus/writer.rs` and `corpus/curate.rs` first -> observed RED (`2 failed` on `-- corpus transcript`) -> implemented 3-stage redaction, `credential_prompt` drop, and `DefaultPolicy` updates -> observed GREEN (`17 passed; 0 failed`).
- **Task 3:** Wrote failing unit tests in `host_agent.rs` and `daemon_host.rs` first -> observed RED (`2 failed` in `vox-terminal-core`) -> implemented `LocalHost`, `DaemonHost`, and `TermDispatch` handlers -> observed GREEN across `vox-terminal-core` (default + `--features vox-lang`), `term_dispatch` + `term_session_ops`, `vox-gui` `commands::pty`, and `crate_api_docs`.

## Task Commits

Per strict shared-worktree execution rules (`NON-NEGOTIABLE: NEVER run git add / git commit`), all 16 created/modified files are formatted (`rustfmt --edition 2024`), verified, and left clean in the working tree for orchestrator verification and commit.

## Files Created/Modified

- `crates/vox-terminal-core/src/session_agent.rs` — Custom block streaming, `/clear` & `/compact` mutations, `control_epoch` preemption, unpinned ring eviction, and unit tests (311 non-blank LoC).
- `crates/vox-terminal-core/src/session.rs` — Linked `session_agent` submodule and added `control_epoch`, `max_blocks`, and `max_block_bytes` fields.
- `crates/vox-terminal-core/src/commands.rs` — Registered `clear` and `compact` built-ins in `CommandRegistry` and updated `/help`.
- `crates/vox-terminal-core/src/transcript.rs` — Added `Send + Sync + 'static` bound and `flush()` to `TranscriptSink` + `JournalSink::flush`.
- `crates/vox-terminal-core/src/corpus/redact.rs` — Added `scrub_ansi_and_redact_with`, `redact_block_with`, and `redact_block` covering `input`, `output`, `cwd`, and `git_branch`.
- `crates/vox-terminal-core/src/corpus/writer.rs` — Added `CorpusWriter::open`, `record_with`, `flush`, `credential_prompt` drop guard, and `RedactingTranscriptSink`.
- `crates/vox-terminal-core/src/corpus/curate.rs` — Updated `DefaultPolicy` to keep non-empty non-credential `Rejected` blocks and non-identical `Corrected` events.
- `crates/vox-terminal-core/src/host_agent.rs` — `BlockFeedbackVerdict`, `LocalHost` transcript sink constructors, custom block ops, `/clear` & `/compact` pre-flush ops, and `record_block_feedback` (277 non-blank LoC).
- `crates/vox-terminal-core/src/host.rs` — Linked `host_agent`, added `transcript_sink`/`transcript_seq` to `LocalHost`, `Autopilot -> Manual` preemption, and `SessionHost` trait methods + `LocalHost` overrides.
- `crates/vox-terminal-core/src/daemon_host_ops.rs` — Wire method constants (`CUSTOM_OPEN`..`CONTROL_EPOCH`) and `DaemonHost` client RPC helpers (154 non-blank LoC).
- `crates/vox-terminal-core/src/daemon_host.rs` — Linked `daemon_host_ops`, added `Some("snapshot")` routing in `DaemonHost::route`, and implemented `SessionHost` overrides on `DaemonHost`.
- `crates/vox-terminal-core/src/lib.rs` — Re-exported `BlockFeedbackVerdict` and `RedactingTranscriptSink` while preserving all Phase 444-01 exports (`completions`, `completions_specs`, `history`, `runbooks`, `voxify`).
- `crates/vox-orchestrator-d/src/term_session_ops.rs` — `try_handle_session_op` RPC handlers with `refuse_agent_origin` guards and loopback daemon integration test (252 non-blank LoC).
- `crates/vox-orchestrator-d/src/term_dispatch.rs` — Linked `term_session_ops` and delegated unknown `term.*` methods after `VOXIFY` to `try_handle_session_op`.
- `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` — Wired `LocalHost::open_default_journal_sink` under `.vox/terminal-journal` with `vox_secrets::scrub_resolved_secrets` and graceful in-memory fallback.
- `docs/src/api/vox-terminal-core.generated.md` — Synced public crate API surface via `./target/debug/vox ci crate-api-sync --write`.

## Deviations from Plan

### Minor Path & Verification Notes

**1. [Generated Crate API Doc Path] Actual repo path is `docs/src/api/vox-terminal-core.generated.md`**
- **Found during:** Task 3 (`crate-api-sync --write`)
- **Issue:** `445-01-PLAN.md` listed `docs/src/reference/cli/crate-api/vox-terminal-core.generated.md`, whereas `vox ci crate-api-sync --write` in HEAD writes to `docs/src/api/vox-terminal-core.generated.md`.
- **Resolution:** Ran `./target/debug/vox ci crate-api-sync --write` directly (no hand-editing) and verified zero drift with both `./target/debug/vox ci crate-api-sync` (`crate-api-sync OK (131 crates)`) and `crate_api_docs::tests::test_run_in_valid_repo`.

**2. [Concurrent Workstream `vox-scientia` Build Error] Direct `rustc --test` verification for `vox-orchestrator-d`, `vox-gui` `commands::pty`, and `vox-cli-ci` `crate_api_docs`**
- **Found during:** Task 3 full-crate `cargo test` runs on downstream crates
- **Issue:** In the shared working tree, concurrent workstream `scientia-deep-research-v76` currently has 3 in-progress compile errors in `crates/vox-scientia` (`review_flow.rs:1581`, `producers/socrates_telemetry.rs:333,336`), which blocks `cargo test -p vox-orchestrator-d`, `cargo test -p vox-gui`, and `cargo test -p vox-cli-ci` when Cargo compiles transitive `vox-scientia`.
- **Resolution:** Did NOT touch `crates/vox-scientia` (outside scope). Ran `cargo test -p vox-terminal-core` (default AND `--features vox-lang`) directly via Cargo (100% green), and verified `term_dispatch` + `term_session_ops` + `term_bridge_impl` (16/16 tests green), `vox-gui` `commands::pty` + `pty::warp` (15/15 tests green), and `vox-cli-ci` `crate_api_docs` (4/4 tests green) via `rustc --test --edition 2024` linked against `target/ws-axis-warp-terminal-v75/debug/deps` into scratch binaries outside the repo.

## Verification Proof

- `cargo test -p vox-terminal-core`: 82 unit tests + 50 integration tests passed, 0 failed, 0 warnings.
- `cargo test -p vox-terminal-core --features vox-lang`: 82 unit tests + 51 integration tests passed, 0 failed, 0 warnings.
- `rustc --test` on `crates/vox-orchestrator-d/src/{term_bridge_impl,term_dispatch,term_session_ops}.rs`: 16 passed, 0 failed, 0 warnings (including `custom_blocks_clear_compact_feedback_and_origin_guards_work_over_daemon` and Phase 444-01's `term_voxify_refuses_agent_origin_and_dispatches_human_origin`).
- `rustc --test` on `crates/vox-gui/src/commands/pty.rs` (`commands::pty` + `pty::warp`): 15 passed, 0 failed, 0 warnings.
- `./target/debug/vox ci crate-api-sync --write` & `./target/debug/vox ci crate-api-sync`: `crate-api-sync OK (131 crates)` + `crate_api_docs` unit suite `4 passed; 0 failed`.
- `./target/debug/vox-arch-check`: exit code 0.
