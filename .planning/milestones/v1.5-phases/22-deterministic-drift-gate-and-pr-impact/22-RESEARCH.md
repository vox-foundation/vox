---
phase: "22"
title: "Phase 22 Research: Deterministic Drift Gate & PR Impact"
status: "current"
category: "architecture"
description: "Technical research and architectural specification for symbol-span anchor hashing, freshness ledger, tighten-only drift gate, PR impact comments, definition-of-done warnings, and executable tutorial tests (TUT-02)."
---

# Phase 22 Technical Research: Deterministic Drift Gate & PR Impact

## Executive Summary

Phase 22 establishes the deterministic drift prevention, contributor impact awareness, and tutorial execution contract for the `docs-freshness` workstream in the Vox project. Building directly on the foundation delivered in Phase 21 (mention extractor, ground truth resolvers, false-positive evaluation, and prune reporting), Phase 22 shifts documentation health from passive auditing to active, automated, and deterministic enforcement:

1. **Zero LLM dependencies on PR paths**: In strict adherence to project doctrine (D10), all PR gates are 100% deterministic (AST parsing, token hashing, contract validation, and git diff analysis).
2. **Reverse index & PR impact awareness (DRIFT-01)**: Every PR changing code anchors referenced by reader-facing docs receives exactly one in-place edited comment detailing affected docs.
3. **Freshness ledger & symbol-span hashing (DRIFT-02)**: `contracts/reports/docs-freshness/ledger.v1.json` tracks per-page anchors and states (`verified`, `drifting`, `broken`, `unanchored`, `historical`). Reformatting or adding comments to code does *not* trigger false-positive drift churn.
4. **Frontmatter governance (DRIFT-03)**: Authored frontmatter gains only `owner`, `review_cadence`, and optional `code_anchors`, with hand-written `last_verified` strictly forbidden.
5. **Tighten-only ratchet gate (DRIFT-04)**: New broken mentions on reader-facing `status: current` docs block PR merges; pre-existing deficits are baselined and can only shrink.
6. **Expiring suppressions (DRIFT-05)**: Every suppression requires `owner`, `reason`, and `expires_after`; expired suppressions fail CI.
7. **Human re-attestation (DRIFT-06)**: Contributor CLI and PR label mechanisms allow humans to re-stamp verified state against HEAD after review.
8. **Docs Definition-of-Done (DRIFT-07)**: PRs altering public CLI commands, env vars, or contracts without doc or generator updates receive deterministic warnings.
9. **Executable tutorial tests (TUT-02)**: All tutorials run hermetically end-to-end as executable docs-as-tests via a `.vox` harness, blocking regressions.

---

## 1. Freshness Ledger & Data Model Architecture (DRIFT-02, DRIFT-03, DRIFT-05)

### 1.1 Freshness Ledger v1 (`ledger.v1.json`)

The Freshness Ledger is the machine-readable ground truth for document state across the corpus. It resides at:
`contracts/reports/docs-freshness/ledger.v1.json`

It is committed to VCS and configured in `.gitattributes` as:
```gitattributes
contracts/reports/docs-freshness/ledger.v1.json linguist-generated=true -diff
```

#### JSON Schema Specification
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "x-vox-version": 1,
  "title": "DocsFreshnessLedgerV1",
  "type": "object",
  "required": ["x-vox-version", "generated_by", "head_commit", "pages", "metrics"],
  "properties": {
    "x-vox-version": { "type": "integer", "const": 1 },
    "generated_by": { "type": "string" },
    "updated_at": { "type": "string", "format": "date-time" },
    "head_commit": { "type": "string", "pattern": "^[0-9a-f]{40}$" },
    "pages": {
      "type": "object",
      "additionalProperties": {
        "type": "object",
        "required": ["title", "category", "status", "state", "last_verified_commit", "anchors"],
        "properties": {
          "title": { "type": "string" },
          "category": { "type": "string" },
          "status": { "type": "string" },
          "owner": { "type": ["string", "null"] },
          "review_cadence": { "type": ["string", "null"] },
          "state": {
            "type": "string",
            "enum": ["verified", "drifting", "broken", "unanchored", "historical"]
          },
          "last_verified_commit": { "type": ["string", "null"] },
          "anchors": {
            "type": "array",
            "items": {
              "$ref": "#/$defs/AnchorRecord"
            }
          },
          "broken_mentions": {
            "type": "array",
            "items": { "type": "string" }
          }
        }
      }
    },
    "metrics": {
      "type": "object",
      "required": ["total_pages", "verified_count", "drifting_count", "broken_count", "unanchored_count", "historical_count"],
      "properties": {
        "total_pages": { "type": "integer", "minimum": 0 },
        "verified_count": { "type": "integer", "minimum": 0 },
        "drifting_count": { "type": "integer", "minimum": 0 },
        "broken_count": { "type": "integer", "minimum": 0 },
        "unanchored_count": { "type": "integer", "minimum": 0 },
        "historical_count": { "type": "integer", "minimum": 0 }
      }
    }
  },
  "$defs": {
    "AnchorRecord": {
      "type": "object",
      "required": ["id", "kind", "target", "content_hash", "verified_hash", "source"],
      "properties": {
        "id": { "type": "string" },
        "kind": {
          "type": "string",
          "enum": ["rust_symbol", "file_path", "cli_command", "env_var", "config_key", "contract"]
        },
        "target": { "type": "string" },
        "content_hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
        "verified_hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
        "source": {
          "type": "string",
          "enum": ["frontmatter", "prescriptive_mention"]
        }
      }
    }
  }
}
```

#### Page States Definition
- `verified`: Every anchor resolves in repo ground truth, every anchor's current `content_hash` matches its `verified_hash` at `last_verified_commit`, and the page contains zero actionable broken mentions.
- `drifting`: All anchors resolve in repo ground truth, but one or more anchors have `content_hash != verified_hash` (the underlying code has changed since the doc was attested).
- `broken`: At least one anchor or prescriptive code mention fails to resolve (`Missing` or `Retired` without replacement).
- `unanchored`: The page contains no explicit `code_anchors` in frontmatter and no prescriptive code mentions (e.g. pure philosophy, conceptual overview).
- `historical`: The page has frontmatter `status` of `archived`, `legacy`, `deprecated`, or is located in `docs/src/archive/` (exempt from active drift gating).

---

### 1.2 Authored Frontmatter Governance (DRIFT-03)

In accordance with DRIFT-03, authored markdown files under `docs/src/` may define exactly three new optional frontmatter keys:
1. `owner`: GitHub handle (`@username`) or team alias (`team-cli`, `team-compiler`, `team-runtime`).
2. `review_cadence`: Review frequency string (e.g. `30d`, `60d`, `90d`, `180d`, `monthly`, `quarterly`).
3. `code_anchors`: Sequence of explicit code target anchors (e.g. `crates/vox-cli/src/main.rs::run`, `contracts/cli/command-registry.yaml`).

#### Hard Prohibition: Hand-Authored `last_verified`
Similar to the project rule prohibiting hand-authored `last_updated:` (which is derived from git history), authors MUST NOT write `last_verified:` or `last_verified_commit:` in frontmatter.
- Enforcement: Handled directly in `crates/vox-doc-pipeline/src/pipeline/lint.rs` via `LintKind::HandAuthoredLastVerified`.
- Parity Testing: `crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs` verifies that all accepted frontmatter fields in `lint.rs` match `docs/src/contributors/documentation-governance.md` §Frontmatter Starter Template.

---

### 1.3 Baseline & Suppressions Schema (DRIFT-04, DRIFT-05)

#### Baseline Ledger (`contracts/reports/docs-freshness/baseline.v1.json`)
The baseline stores existing pre-Phase 22 deficits so that current tech debt does not block unrelated PRs:
```json
{
  "x-vox-version": 1,
  "schema_version": 1,
  "generated_at_commit": "b64377d402f207a588f9c6b2e71ede6679340105",
  "grandfathered_broken_mentions": [
    {
      "fingerprint": "a1b2c3d4...",
      "source_file": "docs/src/reference/example.md",
      "target": "vox_compiler::legacy_token",
      "kind": "rust_symbol"
    }
  ],
  "metrics": {
    "grandfathered_count": 18
  }
}
```

#### Suppressions Contract (`contracts/reports/docs-freshness/suppressions.v1.json`)
Explicit temporary exceptions for known breaking refactors or planned deprecations:
```json
{
  "x-vox-version": 1,
  "schema_version": 1,
  "suppressions": [
    {
      "doc_path": "docs/src/tutorials/tut-ui-integration.md",
      "target": "vox-plugin-react",
      "kind": "crate_name",
      "owner": "@alex",
      "reason": "React plugin packaging split tracked in issue #412",
      "expires_after": "2026-11-15"
    }
  ]
}
```
**Gate Rules for Suppressions**:
- Mandatory fields: `doc_path`, `target`, `kind`, `owner`, `reason`, `expires_after`.
- `expires_after` must be valid ISO-8601 date (`YYYY-MM-DD`).
- In CI: `if today() > expires_after`, emit `FATAL: Suppression for <target> in <doc_path> expired on <expires_after>. Gate failed.`

---

## 2. Symbol-Span Content Hashing Design (DRIFT-02)

### 2.1 The Formatting Churn Problem
A naive file-level SHA256 (`git hash-object path/to/file.rs`) has a fatal flaw:
- Any formatting pass (`cargo fmt`, `scripts/fmt.vox`), comment typo correction, whitespace tweak, or edit to an unrelated function in the same file produces a new SHA.
- If a doc page anchors to `crates/vox-cli/src/commands/audit.rs::run_docs_audit`, and another developer edits `run_arch_check` in the same file, the file SHA changes. The doc page would falsely flip to `drifting`.
- This creates alert fatigue, spurious re-attestation requests, and PR noise.

### 2.2 Token-Stream AST Hashing Architecture
To make hashing completely invariant to whitespace, indentation, comments, and unrelated file edits, hashing operates at the **symbol-span level**:

```
Rust Source File (.rs)
         │
         ▼
syn::parse_file()
         │
         ▼
RustVisitor AST Item Matcher (matches item by qualified symbol ID)
         │
         ▼
syn::Item (Fn, Struct, Enum, Trait, ImplItemFn, Type)
         │
         ▼
item.to_token_stream()  <── [Strips comments, normalizes trivia]
         │
         ▼
Compact Token String Normalization
         │
         ▼
BLAKE3 / SHA-256 Digest (32-byte hex)
```

#### Implementation in Rust
Leveraging the existing `syn` visitor infrastructure in `crates/vox-graph-reader/src/ast.rs`:
```rust
pub fn hash_rust_symbol_span(file_content: &str, symbol_name: &str) -> Result<Option<String>> {
    let file = syn::parse_file(file_content)?;
    let mut finder = SymbolSpanFinder::new(symbol_name);
    finder.visit_file(&file);
    
    if let Some(tokens) = finder.matched_tokens {
        let mut hasher = blake3::Hasher::new();
        // Tokens serialize without comments and with canonical token spacing
        hasher.update(tokens.to_string().as_bytes());
        Ok(Some(hasher.finalize().to_hex().to_string()))
    } else {
        Ok(None)
    }
}
```

### 2.3 Non-Rust Symbol & Contract Hashing
- **YAML & JSON Contracts (`contracts/**/*.yaml`, `contracts/**/*.json`)**:
  - Parsed into `serde_json::Value`.
  - Keys sorted recursively.
  - Serialized to compact canonical JSON bytes (without comments or whitespace variations).
  - Hashed via BLAKE3/SHA-256.
- **CLI Commands (`contracts/cli/command-registry.yaml`)**:
  - The hash of a CLI command anchor (e.g. `vox check`) is computed from the canonicalized JSON representation of that command's specific entry in `command-registry.yaml` (name, path, flags, description).
- **Environment Variables & Config Knobs**:
  - Hashed from their specific entry in `env-vars.v1.yaml` or `registry.v1.yaml`.
- **Arbitrary File Anchors (Fallback)**:
  - When an anchor explicitly targets an entire non-Rust file (e.g. `examples/mesh-compose.yml`), line endings are normalized (`
` -> `
`), trailing whitespace stripped per line, and comments optionally stripped.

---

## 3. Tighten-Only Drift Gate Algorithm (DRIFT-04)

### 3.1 Scope & Protected Pages
The gate runs on all PRs in `docs-quality.yml` and targets reader-facing, high-trust documentation:
- Frontmatter `status == "current"`
- Categories:
  - `Tutorials` (`docs/src/tutorials/**`)
  - `How-To Guides` (`docs/src/how-to/**`)
  - `Language Reference` (`docs/src/reference/**`)
  - `API Reference — Crates` (`docs/src/api/**`)
  - `Concepts` (`docs/src/concepts/**`)

Pages with `status: research`, `status: roadmap`, `status: legacy`, `status: deprecated`, or under `docs/src/archive/` are classified as internal/historical and exempt from blocking gates.

### 3.2 Gate Enforcement Logic
```rust
pub fn evaluate_drift_gate(
    root: &Path,
    live_mentions: &[ResolvedMention],
    baseline: &FreshnessBaseline,
    suppressions: &FreshnessSuppressions,
) -> Result<GateReport> {
    let now = chrono::Utc::now().naive_utc().date();
    let mut report = GateReport::default();

    // 1. Verify suppressions validity (DRIFT-05)
    for sup in &suppressions.suppressions {
        if sup.owner.trim().is_empty() || sup.reason.trim().is_empty() {
            report.fatal_errors.push(format!(
                "Invalid suppression for {}: owner and reason must be non-empty",
                sup.target
            ));
        }
        if sup.expires_after < now {
            report.fatal_errors.push(format!(
                "Expired suppression for {} in {}: expired on {}",
                sup.target, sup.doc_path, sup.expires_after
            ));
        }
    }

    // 2. Identify active actionable deficits on current reader pages (DRIFT-04)
    for m in live_mentions {
        if !m.is_actionable_deficit() {
            continue;
        }
        if !is_protected_reader_page(&m.extracted.source_file) {
            continue;
        }

        // Check if covered by active, unexpired suppression
        if suppressions.matches(&m.extracted) {
            report.suppressed_deficits.push(m.clone());
            continue;
        }

        // Check if covered by grandfathered baseline
        let fp = m.extracted.compute_fingerprint();
        if baseline.contains_fingerprint(&fp) {
            report.grandfathered_deficits.push(m.clone());
            continue;
        }

        // NEW broken mention: violates gate!
        report.new_violations.push(m.clone());
    }

    Ok(report)
}
```

### 3.3 Ratchet Invariant: Tighten-Only
Following the established pattern in `crates/vox-cli-ci/src/crate_edges.rs`:
- CI runs `vox ci docs-drift-gate --verify`.
- If `report.new_violations > 0` or `report.fatal_errors > 0`, the gate exits with non-zero status.
- When existing issues are fixed, running `vox ci docs-drift-gate --tighten`:
  1. Computes the intersection of current deficits and previous baseline.
  2. If any new deficit is present that was not in the prior baseline, `--tighten` aborts with an error:
     `--tighten would ADD N deficit(s) (tighten is removal-only)`.
  3. Updates `baseline.v1.json` with only the remaining deficits.

---

## 4. PR Impact Comment & Definition-of-Done (DRIFT-01, DRIFT-07)

### 4.1 Reverse Index Construction
A reverse index maps code artifacts to the documentation pages that prescribe or anchor them:
```
ReverseCodeIndex:
  "crates/vox-cli/src/commands/audit.rs::run_docs_audit" => [
    "docs/src/reference/cli-command-surface.md",
    "docs/src/contributors/docs-reality-audit-program.md"
  ]
  "contracts/cli/command-registry.yaml#vox check" => [
    "docs/src/tutorials/tut-getting-started.md",
    "docs/src/tutorials/tut-first-app.md"
  ]
```

### 4.2 In-Place PR Commenting Mechanism (DRIFT-01)
To prevent notification fatigue, each PR receives **at most one** comment, which is edited in place on subsequent pushes:
1. In CI, during the `docs-quality` workflow (or dedicated PR job with `pull_request` event):
2. Determine changed files using `git diff --name-only origin/${{ github.base_ref }}...HEAD`.
3. Compute affected symbols and map through the reverse index.
4. If affected reader-facing docs exist:
   Format the comment Markdown with unique hidden HTML marker: `<!-- vox-docs-freshness-pr-impact -->`.
5. Query GitHub API for existing PR comments:
   ```bash
   COMMENT_ID=$(gh api repos/$REPO/issues/$PR_NUMBER/comments --jq      '.[] | select(.body | contains("<!-- vox-docs-freshness-pr-impact -->")) | .id')
   ```
6. If `COMMENT_ID` exists:
   ```bash
   gh api -X PATCH repos/$REPO/issues/comments/$COMMENT_ID -f body="$BODY"
   ```
   Otherwise:
   ```bash
   gh pr comment $PR_NUMBER --body "$BODY"
   ```

#### Sample PR Comment Layout
```markdown
<!-- vox-docs-freshness-pr-impact -->
### 📚 Documentation Impact Report

This pull request modifies code anchors referenced by reader-facing documentation.

| Document | Section | Changed Anchor / Mention | Current State |
| :--- | :--- | :--- | :--- |
| [`docs/src/tutorials/tut-first-app.md`](#) | Tutorials | `contracts/cli/command-registry.yaml#vox check` | ⚠️ May need update |
| [`docs/src/reference/cli-audit.md`](#) | Language Reference | `crates/vox-cli/src/commands/audit.rs::run_docs_audit` | 🔄 Drifting |

> [!NOTE]
> If your changes do not require documentation revisions, you can re-attest pages after merge, or add the `docs-attested` label.
```

### 4.3 Docs Definition-of-Done (DoD) Checker (DRIFT-07)
A deterministic rule verifies that public API or contract changes are paired with documentation updates:
- **Trigger Conditions**:
  - PR touches `crates/vox-cli/src/commands/**` (CLI surface)
  - PR touches `contracts/config/env-vars.v1.yaml` (Environment variables)
  - PR touches `contracts/**` (System schemas / contracts)
- **Satisfying Conditions**:
  - PR also modifies at least one corresponding doc under `docs/src/**` OR
  - PR modifies a registered SSOT generator (e.g. `crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs` or `scripts/generate-*.vox`).
- **Verdict**:
  - If triggered and not satisfied: Emits a deterministic GitHub warning annotation (`::warning title=Docs Definition-of-Done::...`) and adds an alert in the PR impact comment.

---

## 5. Human Re-Attestation Flow (DRIFT-06)

When code changes cause a document to transition from `verified` to `drifting`, human review is required to verify whether the documentation remains accurate.

```
       Code Change Merged
              │
              ▼
   Anchor Content Hash Changes
              │
              ▼
    Page State: "drifting"
              │
              ▼
   Human Reviewer Inspects Doc
              │
    ┌─────────┴─────────┐
    ▼                   ▼
Doc Needs Update     Doc is Still Accurate
    │                   │
Update Markdown         │
    │                   │
    └─────────┬─────────┘
              ▼
     Human Re-Attests:
   `vox docs attest <doc-path>`
   (or PR label: `docs-attested`)
              │
              ▼
  Freshness Ledger Updated:
  - verified_hash = new_content_hash
  - last_verified_commit = HEAD
  - state = "verified"
              │
              ▼
  CI Verifies Ledger Integrity
```

### CLI Command Design
```bash
# Re-attest a single page
vox audit docs --attest docs/src/tutorials/tut-first-app.md

# Re-attest all drifting pages reviewed in current PR
vox audit docs --attest-reviewed
```

### Verification Invariants
The CLI recomputes the current symbol-span hashes from disk, stamps the current git HEAD commit SHA into `last_verified_commit`, updates the `verified_hash` of each anchor, and sets `state: "verified"`.
In CI: `vox ci docs-ledger-verify` ensures nobody hand-edits `verified_hash` or falsifies `last_verified_commit`.

---

## 6. Tutorial Executable Tests Harness (TUT-02)

### 6.1 The Need for Docs-as-Tests
While Phase 20 audited tutorials and verified that code snippets compile via `doctest-md`, compilation alone does not prove that a multi-step user journey works end-to-end. TUT-02 turns every tutorial in `docs/src/tutorials/` into a hermetic executable test.

### 6.2 Architecture: `scripts/docs/tutorial-test-runner.vox`
Written as a native `.vox` script executing under `vox run --mode interp`:
1. **Extraction**:
   - Parses Markdown AST of each tutorial in `docs/src/tutorials/*.md`.
   - Extracts ordered shell command fences (`bash`, `sh`, `console`) and file creation instructions.
2. **Isolation**:
   - For each tutorial, creates a clean temporary directory in `target/tmp/tutorial-sandbox/<tutorial-slug>/`.
   - Sanitizes environment (`VOX_CONFIG_DIR`, clean path).
3. **Execution**:
   - Executes commands sequentially (`vox init`, file writes, `vox check`, `vox build`, `vox run`).
   - Applies per-step timeout (e.g. 30s) and per-tutorial timeout (120s) using `process::spawn` with timeout monitoring.
4. **Assertions**:
   - Exit code must be 0 for all commands unless explicitly flagged as expected failure.
   - Stdout/stderr patterns match expected output if specified.
5. **CI Triggering & Gate**:
   - Runs in `.github/workflows/docs-quality.yml` (and `ci.yml`) on PRs touching:
     - `docs/src/tutorials/**`
     - `crates/vox-cli/**`
     - `crates/vox-compiler/**`
     - `crates/vox-codegen/**`
     - `crates/vox-runtime/**`
   - Failure of any tutorial blocks the PR.

---

## 7. Recommended Implementation Waves

### Wave 1: Data Model, Frontmatter Extensions & Symbol-Span Hashing (DRIFT-02, DRIFT-03)
- Deliverables:
  - `contracts/reports/docs-freshness/ledger.v1.json` initial ledger & schema.
  - Frontmatter linter update in `crates/vox-doc-pipeline/src/pipeline/lint.rs` accepting `owner`, `review_cadence`, `code_anchors` and rejecting hand-authored `last_verified`.
  - Parity test `crates/vox-doc-pipeline/tests/frontmatter_governance_parity.rs`.
  - Symbol-span content hasher (`syn` item visitor) in `vox-doc-pipeline`.
  - `.gitattributes` entry marking `ledger.v1.json` as `linguist-generated=true -diff`.

### Wave 2: Tighten-Only Baseline, Suppressions & Drift Gate (DRIFT-04, DRIFT-05, DRIFT-06)
- Deliverables:
  - `contracts/reports/docs-freshness/baseline.v1.json` & `suppressions.v1.json`.
  - Suppressions validator in `vox-doc-pipeline` (owner, reason, expiry check).
  - Tighten-only ratchet gate implementation (`--tighten` is removal-only).
  - Human re-attestation CLI: `vox audit docs --attest <path>`.
  - CI job integration into `docs-quality.yml` (blocking on reader-facing `current` docs).

### Wave 3: Reverse Index, PR Impact Comment & Definition of Done (DRIFT-01, DRIFT-07)
- Deliverables:
  - Reverse code-to-docs index generator in `crates/vox-doc-pipeline`.
  - GitHub PR impact reporter script / CLI subcommand (`vox ci docs-pr-impact`).
  - In-place GitHub comment editor using `gh` CLI in `.github/workflows/docs-quality.yml`.
  - Docs Definition-of-Done (DoD) check alerting on uncommented/unpropagated CLI/env/contract changes.

### Wave 4: TUT-02 Executable Tutorial Test Harness
- Deliverables:
  - `scripts/docs/tutorial-test-runner.vox` execution harness.
  - Step extractor for bash and `.vox` files from tutorials.
  - Hermetic temporary directory runner with per-command process timeouts.
  - Workflow wiring in `docs-quality.yml` and `ci.yml` blocking PRs on tutorial failures.

---

## 8. Risk Analysis & Fallbacks

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| **Rust AST Macro Hiding**: A symbol is defined inside a macro invocation that `syn` cannot resolve without expansion. | False negative or unanchored status | Fallback to regex symbol span extraction + normalized whitespace token hash. |
| **Tutorial Execution Flakiness**: A tutorial test fails intermittently due to file system locks or port collisions. | Spurious PR block | Use isolated temp directories with OS-assigned random ports or ephemeral sockets; retry once on environment error. |
| **GitHub API Rate Limits on PR Comments**: High PR volume exhausts token quota for commenting. | Comment step failure | Use repository token `GITHUB_TOKEN`; fail soft with warning if API returns 403/429 (gate check itself remains local and unaffected). |
| **Initial Grandfather Baseline Bloat**: Large number of legacy broken mentions overwhelm baseline. | High maintenance debt | Established in Phase 21 prune: retired symbols and dead files are already cleaned; baseline will only hold genuine remaining edges. |

---

## RESEARCH COMPLETE

### Key Findings Summary
1. **Symbol-span hashing** via `syn` token streaming completely eliminates false-positive drift churn caused by `cargo fmt` or file comments.
2. **Reverse index generation** directly connects code changes to affected reader-facing docs, enabling targeted PR comments updated in place.
3. **Tighten-only ratchet** provides an immediate blocking quality gate against new regressions while giving teams a clean path to burn down grandfathered debt.
4. **Executable tutorials (TUT-02)** transform tutorials into living integration tests, guaranteeing that code changes never silently invalidate onboarding documentation.
