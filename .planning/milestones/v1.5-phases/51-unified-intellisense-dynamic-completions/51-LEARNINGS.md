---
phase: 51
phase_name: "unified-intellisense-dynamic-completions"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 20
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 51 Learnings: unified-intellisense-dynamic-completions

## Decisions

### Upgraded completions_for_document to inspect trigger character (@ -> dec...
Upgraded `completions_for_document` to inspect trigger character (`@` -> decorator-only completions with docs, `:` / `->` -> `SURFACE_TYPE_NAMES` + user types), word prefixes, and document AST symbols.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-01-SUMMARY.md

---

### Built resilient extract_symbols_from_source in [crates/vox-lsp/src/symbo...
Built resilient `extract_symbols_from_source` in [`crates/vox-lsp/src/symbols.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-lsp/src/symbols.rs) that handles incomplete buffer states via full parse, script parse, cursor-line blanking, and token fallback.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-01-SUMMARY.md

---

### Wired self.documents in LSP completion handler so active document text i...
Wired `self.documents` in LSP completion handler so active document text is passed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-01-SUMMARY.md

---

### Added render_nushell_completions in [crates/vox-cli/src/command_catalog....
Added `render_nushell_completions` in [`crates/vox-cli/src/command_catalog.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/command_catalog.rs) emitting valid `extern "vox"` and `extern "vox <cmd>"` definitions with `--help(-h)`, typed parameters, and descriptions.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-01-SUMMARY.md

---

### Extended CompletionShell with nushell / nu visible alias in [crates/vox-...
Extended `CompletionShell` with `nushell` / `nu` visible alias in [`crates/vox-cli/src/lib.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/lib.rs) and wired dispatch.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-01-SUMMARY.md

---

### vox ... (CLI command & flag suggestions)
`vox ...` (CLI command & flag suggestions)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-02-SUMMARY.md

---

### /... (builtin slash verbs and installed skills)
`/...` (builtin slash verbs and installed skills)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-02-SUMMARY.md

---

### !nu ... and | ... (Nushell pipeline stage completions)
`!nu ...` and `| ...` (Nushell pipeline stage completions)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-02-SUMMARY.md

---

### ?... (natural language prompt & recovery suggestions)
`?...` (natural language prompt & recovery suggestions)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-02-SUMMARY.md

---

### @term:... (terminal block references)
`@term:...` (terminal block references)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-02-SUMMARY.md

---

### Integrated computeIntelliSenseItems + discoverySuggest across all input ...
Integrated `computeIntelliSenseItems` + `discoverySuggest` across all input shapes (`vox ...`, `/slash`, `!nu`, `?prompt`, `@term`, `@file`, and pipeline stages `| ...`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-03-SUMMARY.md

---

### Connected <IntelliSensePopover /> and inline ghost text (data-testid="gh...
Connected `<IntelliSensePopover />` and inline ghost text (`data-testid="ghost"`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-03-SUMMARY.md

---

### ArrowDown / ArrowUp to cycle active suggestion and notify DiscoveryRail ...
`ArrowDown` / `ArrowUp` to cycle active suggestion and notify `DiscoveryRail` (`onActiveSuggestion`),

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-03-SUMMARY.md

---

### Tab / ArrowRight to accept ghost or selected popover suggestion,
`Tab` / `ArrowRight` to accept ghost or selected popover suggestion,

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-03-SUMMARY.md

---

### Ctrl+Space to trigger dynamic suggestions even on empty input,
`Ctrl+Space` to trigger dynamic suggestions even on empty input,

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-03-SUMMARY.md

---

### Added test verifying live integration of IntelliSensePopover with discov...
Added test verifying live integration of `IntelliSensePopover` with `discoverySuggest`, categorized tabs, and footer signatures. (7/7 passed).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-04-SUMMARY.md

---

### Added test verifying @ mentions open IntelliSensePopover and cycle candi...
Added test verifying `@` mentions open `IntelliSensePopover` and cycle candidates cleanly via `ArrowDown` (44/44 passed).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-04-SUMMARY.md

---

### 4/4 passed.
4/4 passed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-04-SUMMARY.md

---

### Added test exercising multi-source IntelliSense popover, category tabs, ...
Added test exercising multi-source IntelliSense popover, category tabs, and ghost text in the Axis Console prompt.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-04-SUMMARY.md

---

### Tested typing /mo to pop open /model and /help, inspecting category tabs...
Tested typing `/mo` to pop open `/model` and `/help`, inspecting category tabs (`All (2)`, `Slash & Skills (2)`), and accepting with `Tab`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 51-04-SUMMARY.md

## Lessons

### COMP-01: Backend LSP (vox-lsp), CLI Nushell completions (vox-cli), and n...
**`COMP-01`**: Backend LSP (`vox-lsp`), CLI Nushell completions (`vox-cli`), and `nucleo`-boosted `discoverySuggest` (`vox-gui`)

**Context:** Verified in phase acceptance checks.
**Source:** 51-VERIFICATION.md

---

### Command: cargo test -p vox-lsp -p vox-cli --test completion_doctor_test
Command: `cargo test -p vox-lsp -p vox-cli --test completion_doctor_test`

**Context:** Verified in phase acceptance checks.
**Source:** 51-VERIFICATION.md

## Patterns

## Surprises

