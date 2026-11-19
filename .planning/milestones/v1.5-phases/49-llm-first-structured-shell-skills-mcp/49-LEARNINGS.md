---
phase: 49
phase_name: "llm-first-structured-shell-skills-mcp"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 10
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 49 Learnings: llm-first-structured-shell-skills-mcp

## Decisions

### Upgraded NU_OSC633 in [crates/vox-terminal-core/src/pty.rs](file:///User...
Upgraded `NU_OSC633` in [`crates/vox-terminal-core/src/pty.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/pty.rs) to emit full `633;A`, `633;B`, `633;C`, `633;D;<exit>`, and `633;E;<cmd>` markers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-01-SUMMARY.md

---

### Added diagnose_nu_posix_isms and run_nu_structured_capture in pty.rs wit...
Added `diagnose_nu_posix_isms` and `run_nu_structured_capture` in `pty.rs` with graceful fallback when `nu` is absent on `PATH`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-01-SUMMARY.md

---

### Added shell: Option<String> to OpenSpec in [crates/vox-terminal-core/src...
Added `shell: Option<String>` to `OpenSpec` in [`crates/vox-terminal-core/src/host.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/host.rs) and `InputIntent::StructuredShell` routing in [`crates/vox-terminal-core/src/input.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/input.rs) and [`crates/vox-terminal-core/src/session.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/session.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-01-SUMMARY.md

---

### Added run_nu_structured and diagnose_nu_posix_isms in [crates/vox-cli/sr...
Added `run_nu_structured` and `diagnose_nu_posix_isms` in [`crates/vox-cli/src/commands/runtime/shell/backends/nushell.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/backends/nushell.rs) and wired `structured_data` + `syntax_hints` into [`crates/vox-cli/src/commands/runtime/shell/aci_adapter.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/aci_adapter.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-02-SUMMARY.md

---

### Extended [crates/vox-orchestrator-mcp/src/aci/envelope.rs](file:///Users...
Extended [`crates/vox-orchestrator-mcp/src/aci/envelope.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/aci/envelope.rs) to validate and preserve `structured_data` and `syntax_hints` inside `aci.execution_probe`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-02-SUMMARY.md

---

### Added run_check_with_backend in [crates/vox-cli/src/commands/runtime/she...
Added `run_check_with_backend` in [`crates/vox-cli/src/commands/runtime/shell/check_terminal.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/check_terminal.rs) to route `nu`/`grammar` checks through `vox_container::exec_grammar`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-02-SUMMARY.md

---

### Authored spec-compliant [crates/vox-skills/skills/superpowers/nushell-st...
Authored spec-compliant [`crates/vox-skills/skills/superpowers/nushell-structured-shell/SKILL.md`](file:///Users/brbrainerd/dev/vox/crates/vox-skills/skills/superpowers/nushell-structured-shell/SKILL.md) with `vox-shell: nushell` metadata and structured JSON pipeline patterns.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-03-SUMMARY.md

---

### Registered /nu in [crates/vox-terminal-core/src/commands.rs](file:///Use...
Registered `/nu` in [`crates/vox-terminal-core/src/commands.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/commands.rs) (`CommandRegistry::default()`) and routed `/nu --help` / bare `/nu` to help output while `/nu <pipeline>` executes structured capture.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-03-SUMMARY.md

---

### Added term-shell-mode-select (default | nu | vox) to [InputEditor.tsx](f...
Added `term-shell-mode-select` (`default | nu | vox`) to [`InputEditor.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx) with automatic `!nu ` prefixing in `nu` mode.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-04-SUMMARY.md

---

### Added structured JSON detection and term-block-structured-badge (structu...
Added structured JSON detection and `term-block-structured-badge` (`structured json`) rendering in [`BlockStreamView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 49-04-SUMMARY.md

## Lessons

### NU_OSC633 emits all 5 OSC-633 markers (A/B/C/D/E), diagnose_nu_posix_ism...
`NU_OSC633` emits all 5 OSC-633 markers (`A/B/C/D/E`), `diagnose_nu_posix_isms` catches POSIX idioms, `OpenSpec.shell` configures custom shells, and `!nu` / `/nu` dispatches structured captures via `run_nu_structured_capture`.

**Context:** Verified in phase acceptance checks.
**Source:** 49-VERIFICATION.md

---

### run_nu_structured executes Nushell pipelines with | to json --raw, aci.e...
`run_nu_structured` executes Nushell pipelines with `| to json --raw`, `aci.execution_probe` preserves `structured_data` and `syntax_hints`, and `vox shell check` supports `--backend auto|pwsh|nu|grammar` with pure-Rust `exec_grammar` validation.

**Context:** Verified in phase acceptance checks.
**Source:** 49-VERIFICATION.md

## Patterns

## Surprises

