---
title: "TOESTUB contributor guide"
description: "Rule-by-rule troubleshooting for TOESTUB CI failures, with fix patterns, suppression guidance, and false-positive notes."
category: "contributor"
status: "current"
last_updated: "2026-05-09"
training_eligible: true
training_rationale: "Practical fix patterns for every TOESTUB rule ID — high utility for human and AI contributors."

schema_type: "TechArticle"
---

# TOESTUB contributor guide

This is the **fix-it companion** to the [architectural governance policy](../../agents/governance.md).
That page documents *what* the rules are. This page documents *how to fix them*.

## Run TOESTUB locally first

```bash
# Scoped to your changed crate (repeat per directory)
cargo run -p vox-cli --features stub-check -- stub-check crates/your-crate

# With fix suggestions
cargo run -p vox-cli --features stub-check -- stub-check crates/your-crate --suggest-fixes

# Severity filter (only errors and criticals)
cargo run -p vox-cli --features stub-check -- stub-check crates/your-crate --severity error

# Full workspace CI scan (what CI runs)
cargo run -p vox-code-audit --bin toestub -- crates/
```

Output is JSON when you need to pipe it: append `--format json`.
Schema: `contracts/toestub/toestub-run-json.v1.schema.json`.

---

## Rule-by-rule fix guide

### `arch/god_object` — Error

**Triggers:** A `.rs` file exceeds 500 non-blank lines, or a struct/impl has
more than 12 methods. Thresholds: 300 lines = Info, 400 = Warning, 500 = Error.

**Fix:** Split using `mod.rs` + `pub use`. Preserve public API surfaces exactly
via `pub use` re-exports so callers don't break. Follow the
[god object defactor checklist](../archive/research-2026-q1/god-object-defactor-checklist.md)
step by step — it has a PowerShell inventory script and a per-crate cargo test
matrix.

Typical split:

```text
// Before: large_module.rs (600 lines)
// After:
//   large_module/
//     mod.rs        ← pub use submodule::*
//     types.rs
//     core.rs
//     helpers.rs
```

**False positive:** Large generated files (e.g. include-fragment patterns).
Add to `contracts/toestub/suppressions.v1.json` with a reason and owner.

---

### `arch/sprawl` — Error (forbidden names) / Warning (directory sprawl)

**Triggers:** A directory contains more than 20 files, or a file uses a
forbidden generic name (`utils.rs`, `helpers.rs`, `misc.rs`, etc.).

**Fix for directory sprawl:** Create a feature-named subdirectory and move
related files into it. Example: 22 files in `src/commands/` → split into
`src/commands/corpus/`, `src/commands/training/`, etc.

**Fix for forbidden names:** Rename to something domain-specific.
`utils.rs` → `retry_policy.rs`, `schema_helpers.rs`, etc.

---

### `skeleton/no-test-for-pub-fn` — Warning

**Triggers:** A `fn` in an `examples/golden/` Vox file is not called from any
`@test` block in the same file. Scoped to golden examples only — scripts and
non-golden `.vox` files are not checked.

**Fix:** Add an `@test` block that calls the function *before* implementing it
(the @test-first gate from [contribution-loop.md](contribution-loop.md#test-first-for-golden-examples)):

```vox
// vox:skip
@test
fn test_my_fn() {
    let result = my_fn("input")
    assert(result is "expected")
}

fn my_fn(x: str) to str {
    // implement until the @test above passes
}
```

**Suppression (rare):** If a function is intentionally a private helper used
only by other helpers (not directly tested), suppress on that line:

```vox
// vox:skip
fn _internal_helper() { ... } // toestub-ignore(skeleton/no-test-for-pub-fn)
```

**False positive:** Helper functions that are called by other tested functions
will not trigger this rule — only completely uncovered functions fire.

---

### `skeleton/hollow-fn` — Warning

**Triggers:** A function body returns only a trivially-default value with no
side effects: `Ok(())`, `true`, `false`, `Vec::new()`, `None`, `String::new()`.

**Fix:** Implement the function body. If the function is genuinely a no-op by
design (e.g. an optional hook), document why in a comment and suppress:

```rust
// toestub-ignore(skeleton/hollow-fn) — intentional no-op default hook
fn on_agent_pause(&self) -> Result<()> { Ok(()) }
```

**False positive:** Test builder helpers. Use `// toestub-ignore(skeleton)`.

---

### `arch/empty_body` — Warning

**Triggers:** Empty or near-empty function bodies.

**Fix:** Same as `skeleton/hollow-fn`. Implement or suppress with reason.

---

### `stub/todo` / `stub/unimplemented` — Error

**Triggers:** `todo!()`, `unimplemented!()`, or `todo!("...")` in production
code paths (outside `tests/`).

**Fix:** Implement the function. There are no valid uses of `todo!()` in merged
production code. If the feature is deferred, remove the function entirely or
return a proper `Err(...)` with a structured error type.

**The `VictoryClaimDetector` co-fires** when "done / complete / fixed" appears
near `unimplemented!()`. Do not add completion comments to stub code.

---

### `security/hardcoded-secret` — Error

**Triggers:** High-entropy strings or credential-shaped literals in source code.

**Fix:** Route through vox-secrets:

```rust
use vox_secrets::resolve_secret;
let key = resolve_secret(SecretId::MyApiKey)?;
```

Declare the `SecretId` variant in `crates/vox-secrets/src/spec.rs`. See
[Secrets SSOT](../reference/secrets-ssot.md) for the full lifecycle.

**False positive:** Content-addressed hashes, test fixture values. Suppress
with `// toestub-ignore(security/hardcoded-secret) — SHA256 test fixture`.

---

### `arch/schema_compliance` — Error

**Triggers:** A `.vox` code block in `docs/src/` is neither included via
`{{#include ../../../examples/golden/hello.vox:display}}` nor annotated with `// vox:skip`.

**Fix:**

<!-- Option A: include from golden (preferred — gets compiled in CI) -->

```vox
{{#include ../../../examples/golden/hello.vox:display}}
```

<!-- Option B: illustrative snippet not meant to compile -->

```vox
// vox:skip
fn illustrative_example() { ... }
```

---

### `cross-platform/crlf` — Warning

**Triggers:** CRLF (`\r\n`) line endings in tracked source files. Common on
Windows with default git settings.

**Fix:**

```bash
# Prevent future CRLF on this machine
git config --global core.autocrlf false

# Fix existing files (from repo root)
cargo run -p vox-cli -- ci line-endings --fix
```

Check `.gitattributes` — `* text=auto eol=lf` is set repo-wide. If git is
converting anyway, confirm `core.autocrlf` is `false` or `input`.

---

### `arch/unwired` — Warning

**Triggers:** A `mod foo;` declaration in a `.rs` file where `foo` is never
subsequently used (`use`, `pub use`, or direct path reference). Only private
`mod` declarations are flagged; `pub mod` is assumed to be reachable from
other files.

**Fix:** Either wire the module (`use crate::foo::Bar;`) or delete the `mod`
declaration and its file if the module is unused.

---

### `dry-violation` — Warning

**Triggers:** Near-duplicate blocks of code detected heuristically (typically
≥5 identical or near-identical lines appearing in multiple locations).

**Fix:** Extract the shared logic to a named function or macro in a common
module. The heuristic has false positives on table-driven data; suppress those
with a short reason comment.

---

### `deprecated-usage` — Warning

**Triggers:** Use of a retired crate, symbol, or environment variable from the
[AGENTS.md retired surfaces table](../../../AGENTS.md).

**Fix:** Use the canonical replacement. Common cases:

| Found | Replace with |
| --- | --- |
| Legacy orchestrator codename | `vox-orchestrator` |
| Legacy ARS crate label | `vox-openclaw-runtime` |
| Legacy Ludus crate label | `vox-gamify` |
| Sync recall API | `recall_async()` |
| Legacy Turso URL env alias | `VOX_DB_URL` |

---

### `rust/unwrap-call` — Info

**Triggers:** `.unwrap()` in non-test code paths (heuristic; test paths are
skipped).

**Fix:** Replace with `?`, a `match`, or `.expect("invariant: reason")` when
the invariant is truly guaranteed. Prefer `?` in most cases.

---

### `victory-claim` — Warning

**Triggers:** "Done / solved / fixed / complete" style comments or strings.
`victory-claim/hack` (Info) fires for `HACK`, `FIXME` adjacent to stub code.

**Fix:** Remove the completion claim, or implement the code the claim refers to.

---

### `magic-value/*` — Warning / Error (sub-id varies)

**Triggers:** Hard-coded ports, very long strings, large integer literals
outside of clearly named constant context.

**Fix:** Extract to a named constant or move to `contracts/scaling/policy.yaml`
for values that belong to the policy surface.

---

### `stringly-typed-enum` — Warning

**Triggers:** A struct field typed as `String` with a comment listing the
valid values (common in config structs).

**Fix:** Replace with a `#[derive(Debug, Clone, serde::Deserialize)]` enum.

---

### `scaling/surfaces` — Info/Warning (14 sub-rules)

**Triggers:** Scaling anti-patterns: blocking I/O in async, unbounded reads,
SQL `SELECT` without `LIMIT`, `Regex::new()` in a hot path, etc.

These are mostly **Info** severity — CI rarely blocks on them. When you see
them: either fix the pattern (prefer `tokio::fs` over `std::fs` in async) or
suppress with a short reason:

```rust
// toestub-ignore(scaling/blocking-in-async) — intentional sync read at startup
let config = std::fs::read_to_string(path)?;
```

See [TOESTUB scaling rules SSOT](../archive/research-2026-q1/scaling-toestub-rules.md) for
all 14 sub-rule IDs.

---

## Suppression syntax

**Same-line inline suppression** (preferred for one-offs):

```rust
// toestub-ignore(rule/id) — short reason why
fn the_function() { ... }
```

**Persistent suppression contract** (for intentional waivers, especially
legacy files): add an entry to `contracts/toestub/suppressions.v1.json`:

```json
{
  "rule": "arch/god_object",
  "path": "crates/vox-oratio/src/backends/candle_engine.rs",
  "owner": "platform-ci",
  "reason": "near-threshold at 499 lines; refactor tracked in wave-9",
  "expires": "2026-07-01"
}
```

Schema: `contracts/toestub/suppression.v1.schema.json`.
**Before adding a suppression, refactor first.** The suppression file is for
genuine waivers, not for avoiding the work.

---

## Related

- [Architectural governance (TOESTUB)](../../agents/governance.md) — policy SSOT, run commands
- [God object defactor checklist](../archive/research-2026-q1/god-object-defactor-checklist.md) — split protocol
- [TOESTUB scaling rules SSOT](../archive/research-2026-q1/scaling-toestub-rules.md) — all 14 scaling sub-rules
- [TOESTUB self-healing architecture](../archive/research-2026-q1/toestub-self-healing-architecture-2026.md) — research on where TOESTUB is going
- [Contribution loop](contribution-loop.md) — why these rules also protect the training corpus
