---
phase: 88-mcp-tool-cli-agent-guidance
reviewed: 2026-10-17T21:00:00Z
depth: standard
files_reviewed: 31
files_reviewed_list:
  - AGENTS.md
  - contracts/capabilities/capability-index.v1.json
  - contracts/capability/capability-registry.yaml
  - contracts/ci/crate-edges.allow.v1.json
  - contracts/ci/crate-graph.v1.json
  - contracts/cli/command-registry.yaml
  - contracts/gui/surface-registry.v1.yaml
  - contracts/mcp/http-read-role-governance.yaml
  - contracts/mcp/tool-registry.canonical.yaml
  - contracts/operations/catalog.v1.yaml
  - contracts/profiles/lang.v1.yaml
  - crates/vox-cli/src/cli_dispatch/mod.rs
  - crates/vox-cli/src/commands/diagram.rs
  - crates/vox-cli/src/commands/mod.rs
  - crates/vox-cli/src/lib.rs
  - crates/vox-cli/tests/diagram_cli.rs
  - crates/vox-diagram/grammar-card.txt
  - crates/vox-diagram/src/lib.rs
  - crates/vox-diagram/src/validate.rs
  - crates/vox-diagram/tests/grammar_card.rs
  - crates/vox-diagram/tests/validate_envelope.rs
  - crates/vox-orchestrator-mcp/Cargo.toml
  - crates/vox-orchestrator-mcp/src/diagram_tools.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/input_schemas.rs
  - crates/vox-orchestrator-mcp/src/lib.rs
  - crates/vox-orchestrator-mcp/src/registry.rs
  - crates/vox-orchestrator-mcp/tests/validate_diagram_tool.rs
  - docs/src/architecture/where-things-live.md
  - docs/src/reference/cli.md
  - docs/src/reference/vox-diagram.md
findings:
  critical: 0
  warning: 6
  info: 6
  total: 12
status: issues_found
---

# Phase 88: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard
**Files Reviewed:** 31
**Status:** issues_found

## Summary

The MCP tool and the shared `vox_diagram::validate` envelope are sound. No blocker was found.

Probes ran against the existing debug `vox` binary (`target/debug/vox`, built 13:47, two minutes before HEAD) from scratch copies under `scratchpad/r88`. The worktree was not modified.

What held up under probing:
- **Input DoS.** Inputs that stayed clean:
  - A 64 KiB source of `{` (nesting bomb).
  - A 5000-deep `container` chain.
  - A single 65000-char label.
  - 400 near-miss ids that stress did-you-mean.
  - Unicode combining marks.
- **Mermaid import.** Inputs that stayed clean (all exit 1 with a named refusal in 0.03-0.2 s):
  - An `&` fan-out (3000 x 3000) hits `expansion-cap`.
  - A 600-node block hits `limit-nodes`.
  - An oversize block hits `limit-source`.
- **SVG render.** Worst cases at the render caps (100 nodes + 200 edges, dense or inside four nested containers with 180-char labels) took about 0.5 s in a debug build. Over-cap input is refused up front. `spawn_blocking` plus the 120 s default dispatch timeout is adequate for this.
- **SVG injection.** `</text><script>`, `<img onerror>`, control characters and a bidi override are escaped or rejected.
- **Envelope semantics.** `normalized_source` is always serialised and is null while invalid. The `{line, construct}` shape and the `unsupported` / `dropped` split match the docs.
- **Read-role governance.** `vox_validate_diagram` is correctly eligible for the read role. It writes nothing, and its `side_effect_class: none` matches the `vox_validate_vuv` precedent. The catalog and capability-registry rows are internally consistent.

All findings are in the CLI file-writing paths and in documentation accuracy. The `-o` write path clobbers its own input, and `fmt --write` replaces symlinks and bypasses read-only permissions.

## Warnings

### WR-01: `fmt --write` replaces a symlink with a regular file and leaves the real target unformatted

**File:** `crates/vox-cli/src/commands/diagram.rs:217-234`
**Issue:** `write_atomic` writes a sibling temp file and renames it over `path`. Rename replaces the symlink itself. Reproduced: with `link.vox -> ok.vox`, `vox diagram fmt --write link.vox` exits 0, `link.vox` becomes a regular file, and `ok.vox` stays non-canonical. The code carries a `ponytail:` note, but the CLI docs and the reference page promise "rewrites the file in place" with no caveat. In a docs tree that uses symlinked diagrams, this silently breaks the link and reports success.
**Fix:**
```rust
let path = &std::fs::canonicalize(path)?; // resolve symlinks first, then temp+rename next to the real file
```
Do this at the top of `write_atomic`, or caveat the behaviour in `docs/src/reference/cli.md`.

### WR-02: `write_atomic` temp file is predictable and not created exclusively

**File:** `crates/vox-cli/src/commands/diagram.rs:222-227`
**Issue:** The temp name is `.{name}.{pid}.tmp`, written with `std::fs::write`, which follows a pre-existing symlink at that path. In a shared writable directory such as `/tmp`, another local user can plant `.f.vox.<guessed-pid>.tmp -> victim` and have `fmt --write` overwrite the victim's file with diagram text. The file is also created with umask permissions and chmod'ed only afterwards (lines 225-226). A 0600 source therefore sits briefly at 0644 on disk.
**Fix:** Create the temp with `OpenOptions::new().write(true).create_new(true)` (O_EXCL, no symlink following). On Unix, use `.mode(perms.mode())` at creation, or `set_permissions` before writing the data.

### WR-03: `fmt --write` rewrites read-only files

**File:** `crates/vox-cli/src/commands/diagram.rs:156-159, 224-227`
**Issue:** Because the file is replaced via rename, directory write permission is all that matters. Reproduced: a mode 0444 file with non-canonical content was rewritten and `vox diagram fmt --write` exited 0. Direct `std::fs::write` would have failed. This bypasses a deliberate read-only flag, for example a vendored or generated diagram.
**Fix:** Before the rename, refuse when `metadata.permissions().readonly()`: `bail!("{}: file is read-only", path.display())`.

### WR-04: `-o FILE` can destroy the input and is not atomic

**File:** `crates/vox-cli/src/commands/diagram.rs:162-165`
**Issue:** `vox diagram export in.vox -o in.vox` reads the source and then overwrites it with Mermaid, exit 0. Reproduced. A user cannot undo it. The write is a plain `std::fs::write` that truncates first, so a failure mid-write also leaves a half file. Only `fmt --write` gets the safe path. `-o` accepts any path, which is expected CLI behaviour, but clobbering the diagram being converted is a data-loss trap, especially for `import -o x.vox` over a `.md` source.
**Fix:** Refuse when `fs::canonicalize(path)` equals `fs::canonicalize(input)`, with exit 2. Reuse `write_atomic`, or a variant that does not require the target to exist, for `-o`.

### WR-05: AGENTS.md tells agents to "apply every `fixes[]` entry" after naming a CLI that never prints fixes

**File:** `AGENTS.md:586-591`, `crates/vox-cli/src/commands/diagram.rs:177-183`
**Issue:** The new policy reads "Validate with the MCP tool or `vox diagram check <file|->`, apply every `fixes[]` entry". Plain `vox diagram check` prints only `file:line:col: code: message` and never the fix spans, so an agent using the CLI path has no fixes to apply. They exist only in the `--json` envelope. This is always-loaded policy text, so an agent following it literally gets an unusable repair loop.
**Fix:** Reword to "`vox --json diagram check <file|->` (fixes are only in the JSON envelope)". Alternatively print a one-line fix hint per diagnostic in text mode.

### WR-06: Docs claim the CLI `--json` output is "the same envelope the MCP tool returns", but the MCP tool wraps it

**File:** `docs/src/reference/cli.md` (the new `vox diagram` section), `docs/src/reference/vox-diagram.md` (CLI section), `crates/vox-cli/tests/diagram_cli.rs:103`
**Issue:** The MCP tool returns `{"success": true, "data": {<envelope>}}`. The tests read `v["data"]["valid"]`. `vox --json diagram check` prints the bare envelope. The reference page's "Output (`schema_version` is always 1)" table never mentions the `success`/`data` wrapper or the `success:false, error` form used for invalid arguments. An agent that parses the documented fields from the MCP response gets `undefined`. The test `check_json_prints_the_mcp_envelope` compares against `validate()` directly, not against the MCP output, so it cannot catch this drift.
**Fix:** Add one sentence to the reference page: the MCP response is `{success, data: <envelope>}`, and `error` replaces `data` when arguments are invalid. Say that the CLI `--json` prints `<envelope>` only. Rename the test to `check_json_prints_the_envelope`.

## Info

### IN-01: `fmt --check -` suggests a command that the tool then refuses

**File:** `crates/vox-cli/src/commands/diagram.rs:150-153`
**Issue:** On stdin the message is "not canonical; run `vox diagram fmt --write <stdin>`", but `fmt --write -` exits 2 ("needs a file"). The hint is misleading.
**Fix:** For stdin, say "run `vox diagram fmt -` to print the canonical form".

### IN-02: Text-mode diagnostics hide severity

**File:** `crates/vox-cli/src/commands/diagram.rs:177-184`
**Issue:** Errors and warnings print identically as `file:l:c: code: message`. `check_warning_only_exits_0` shows warnings exit 0, but the user cannot tell which lines are fatal. The docs say "Warnings print and do not fail".
**Fix:** Print `error:` or `warning:` after the position, as the `--json` `severity` field does.

### IN-03: `--json` I/O failures print plain text on stderr and nothing on stdout

**File:** `crates/vox-cli/src/commands/diagram.rs:85-91`
**Issue:** A missing, unreadable or non-UTF-8 input under `--json` exits 2 with a bare stderr line and an empty stdout. A JSON consumer cannot distinguish this from a crash. Not a bug per the documented exit codes, but it is a parity gap with the MCP tool, which returns `{success:false,error}`.
**Fix:** Optionally emit `{"success":false,"error":"..."}` on stdout when `json` is set.

### IN-04: Grammar card advertises limits that the SVG emit path does not honour

**File:** `crates/vox-diagram/grammar-card.txt` (Rule 6), `crates/vox-orchestrator-mcp/src/input_schemas.rs:482`
**Issue:** The card says "500 nodes, 1000 edges". `emit: "svg"` refuses above 100 nodes plus containers and 200 edges, and returns `emit_error`. Reproduced: 99 two-node containers give "diagram too large to render". The card does not warn agents. The schema also states "At most 65536 bytes" only in prose, with no `maxLength`. The core enforces it, so this is behaviour-correct but unadvertised.
**Fix:** Add "(SVG: 100 nodes, 200 edges)" to the card, keeping the 2600-byte budget in mind. Optionally add a `maxLength` of 65536 to the schema (chars, a lower bound of the byte cap).

### IN-05: Contract bookkeeping looks hand-filled

**File:** `contracts/ci/crate-build-map.v1.json` (new `vox-diagram` row), `docs/src/architecture/layers.toml:188`
**Issue:** The new row reads `compile_s: 0.0` under `measured_on: x86_64-pc-windows-msvc (cargo check)`, which is a placeholder, not a measurement. The `layers.toml` comment still says "first consumer: vox-doc-pipeline (Phase 87)" although vox-orchestrator-mcp is now a second consumer. The `crate-edges` entry's `authorized_by: user (explicit chat approval, 2026-10-16)` is an exception-ledger entry, which is user-authorized-only. The reviewer cannot verify the approval, so confirm it was the user's own entry. The vox-cli route via the `vox_doc_pipeline::diagram` re-export avoids the ledger entirely. That is legal, but it sidesteps the ratchet the policy is about.
**Fix:** Regenerate the build-map row from the generator, update the layers comment, and confirm the approval.

### IN-06: Minor test and portability notes

**File:** `crates/vox-diagram/tests/grammar_card.rs` (`reference_page_publishes_the_card_verbatim`), `crates/vox-cli/src/commands/diagram.rs:146-148`
**Issue:**
- The test `include_str!`s `../../../docs/src/reference/vox-diagram.md`, so the crate's tests cannot run from a packaged or isolated crate checkout.
- `fmt --check` is a byte comparison. A CRLF-checked-out file (Windows autocrlf) is reported "not canonical" while `check` passes, and a BOM is silently stripped by `--write`. Reproduced with CRLF. This is a doc-lint-versus-fmt mismatch to document, not a code defect.
- With `from_mermaid`, a failed import reports zeroed `stats` (reproduced), so it looks like an empty diagram. Fine, but undocumented.
**Fix:** Document the CRLF behaviour and the zeroed stats, or normalise line endings before comparing in `--check`.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
