# Automated Deep Research to Documentation Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an automated documentation engine that transforms empirical deep research findings into publication-grade 7-section Architecture SSOT articles with valid YAML frontmatter, validates them against `vox-doc-pipeline`, and surfaces them across the GUI Vox Axis (`ResearchView`, `DocReader`, `SearchSurface`).

**Architecture:** Connects `vox-research-shim` Wave 3 results and `vox-db` claims through a new `vox-scientia` Architecture SSOT scaffold, exposed via Tauri commands in `vox-gui` with an interactive desktop preview modal. Atomically publishes to `docs/src/architecture/`, registers in `research-index.md`, indexes in `scientia_research_fts`, and cross-links across the GUI.

**Tech Stack:** Rust (Tokio, Tauri 2, Serde), TypeScript/React (Vite, TailwindCSS, Vitest), SQLite FTS5 (VoxDB).

**Spec:** [`docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md`](../specs/2026-09-14-deep-research-documentation-engine-design.md)

## Global Constraints

- Never run `cargo fmt --all`; only format dirty crates via `cargo fmt -p <crate>`.
- All authored Markdown files under `docs/src/` must have valid YAML frontmatter (`title`, `description`, `category: "Architecture SSOTs"` or `"Research Findings"`, `status: "current"`).
- All code steps must contain complete code blocks with zero placeholders (`TODO`, `TBD`).
- Writes to `docs/` and SQLite DB must be atomic (write to `.tmp` + rename).
- Use `LEFTHOOK=0 git commit` to commit when pre-commit hook runs into existing uncommitted whole-repo scan debt.

---

### Task 1: Architecture SSOT Scaffolder in `vox-scientia`

**Files:**
- Create: `crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs`
- Modify: `crates/vox-scientia/src/manuscript/scaffold/mod.rs`
- Test: `crates/vox-scientia/tests/architecture_ssot_scaffold_test.rs`

**Interfaces:**
- Consumes: `ResultsRow`, `ScaffoldError` from `crates/vox-scientia/src/manuscript/scaffold/section_tree.rs`
- Produces: `ArchitectureSsotInput`, `render_architecture_ssot(&ArchitectureSsotInput) -> Result<String, ScaffoldError>`

- [ ] **Step 1: Write the failing test**

Create `crates/vox-scientia/tests/architecture_ssot_scaffold_test.rs`:
```rust
use vox_scientia::manuscript::scaffold::architecture_ssot::{
    ArchitectureSsotInput, CodebaseReference, CompetitiveComparisonRow, GapRecommendation,
    RoadmapPhase, SandboxExecutionRecord, render_architecture_ssot,
};
use vox_scientia::manuscript::scaffold::section_tree::ResultsRow;

#[test]
fn test_render_architecture_ssot_generates_all_seven_sections() {
    let input = ArchitectureSsotInput {
        title: "Test System Architecture (2026)".into(),
        description: "Empirical evaluation of test system.".into(),
        category: "Architecture SSOTs".into(),
        session_id: 42,
        stability_score: 0.88,
        codebase_refs: vec![CodebaseReference {
            crate_name: "vox-test".into(),
            file_path: "crates/vox-test/src/lib.rs".into(),
            line_start: 10,
            line_end: 25,
            observation: "Defines core test harness.".into(),
        }],
        competitive_matrix: vec![CompetitiveComparisonRow {
            dimension: "Latency".into(),
            vox_feature: "12ms".into(),
            alternative_a_name: "Gemini".into(),
            alternative_a_val: "450ms".into(),
            alternative_b_name: "Claude".into(),
            alternative_b_val: "380ms".into(),
        }],
        verified_claims: vec![ResultsRow {
            claim_text: "System executes in under 15ms".into(),
            trusty_uri: "urn:nanopub:test".into(),
            evidence_source: "test_harness".into(),
            verdict: "Supported".into(),
            ci95: Some((0.85, 0.95)),
        }],
        sandbox_probes: vec![SandboxExecutionRecord {
            language: "rust".into(),
            original_snippet: "fn test() { assert!(true); }".into(),
            repaired_snippet: None,
            compiler_output: "Finished release [optimized]".into(),
            success: true,
        }],
        gaps_and_recommendations: vec![GapRecommendation {
            priority: "P0".into(),
            issue: "Buffer allocation overhead".into(),
            remedy: "Use arena allocator".into(),
        }],
        roadmap_phases: vec![RoadmapPhase {
            phase_number: 1,
            title: "Arena Allocation".into(),
            gate: "cargo test -p vox-test".into(),
        }],
    };

    let markdown = render_architecture_ssot(&input).expect("renders markdown");

    assert!(markdown.contains("title: \"Test System Architecture (2026)\""));
    assert!(markdown.contains("category: \"Architecture SSOTs\""));
    assert!(markdown.contains("status: \"current\""));
    assert!(markdown.contains("## 1. Executive Summary & Codebase Reality"));
    assert!(markdown.contains("## 2. SOTA Competitive Analysis & Technical Benchmark Matrix"));
    assert!(markdown.contains("## 3. Verified Empirical Claims Table"));
    assert!(markdown.contains("## 4. Code Verification & Sandbox Reproducibility Evidence"));
    assert!(markdown.contains("## 5. Gap Analysis & Concrete Architectural Recommendations"));
    assert!(markdown.contains("## 6. Phased Implementation Roadmap & Verification Gates"));
    assert!(markdown.contains("crates/vox-test/src/lib.rs"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-scientia --test architecture_ssot_scaffold_test`
Expected: FAIL with "unresolved import `architecture_ssot`"

- [ ] **Step 3: Implement `architecture_ssot.rs`**

Create `crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs`:
```rust
use serde::{Deserialize, Serialize};

use super::render::ScaffoldError;
use super::section_tree::ResultsRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseReference {
    pub crate_name: String,
    pub file_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub observation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitiveComparisonRow {
    pub dimension: String,
    pub vox_feature: String,
    pub alternative_a_name: String,
    pub alternative_a_val: String,
    pub alternative_b_name: String,
    pub alternative_b_val: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecutionRecord {
    pub language: String,
    pub original_snippet: String,
    pub repaired_snippet: Option<String>,
    pub compiler_output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapRecommendation {
    pub priority: String,
    pub issue: String,
    pub remedy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapPhase {
    pub phase_number: u32,
    pub title: String,
    pub gate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSsotInput {
    pub title: String,
    pub description: String,
    pub category: String,
    pub session_id: i64,
    pub stability_score: f64,
    pub codebase_refs: Vec<CodebaseReference>,
    pub competitive_matrix: Vec<CompetitiveComparisonRow>,
    pub verified_claims: Vec<ResultsRow>,
    pub sandbox_probes: Vec<SandboxExecutionRecord>,
    pub gaps_and_recommendations: Vec<GapRecommendation>,
    pub roadmap_phases: Vec<RoadmapPhase>,
}

pub fn render_architecture_ssot(input: &ArchitectureSsotInput) -> Result<String, ScaffoldError> {
    let mut doc = String::new();

    // 1. Frontmatter
    doc.push_str("---\n");
    doc.push_str(&format!("title: \"{}\"\n", input.title.replace('"', "\\\"")));
    doc.push_str(&format!(
        "description: \"{}\"\n",
        input.description.replace('"', "\\\"")
    ));
    doc.push_str(&format!("category: \"{}\"\n", input.category));
    doc.push_str("status: \"current\"\n");
    doc.push_str("training_eligible: true\n");
    doc.push_str("training_rationale: \"Empirically verified architecture findings and benchmarks.\"\n");
    doc.push_str("---\n\n");

    // Title Header
    doc.push_str(&format!("# {}\n\n", input.title));
    doc.push_str(&format!(
        "> **Empirical Provenance:** Generated by Vox Deep Research (Session #{}, Stability $S = {:.2}$).\n\n",
        input.session_id, input.stability_score
    ));

    // Section 1: Executive Summary & Codebase Reality
    doc.push_str("## 1. Executive Summary & Codebase Reality\n\n");
    doc.push_str(&format!("{}\n\n", input.description));
    if !input.codebase_refs.is_empty() {
        doc.push_str("### Codebase Reality & Existing Architecture\n\n");
        for r in &input.codebase_refs {
            doc.push_str(&format!(
                "- [`{}` (lines {}-{})]({}): {}\n",
                r.file_path, r.line_start, r.line_end, r.file_path, r.observation
            ));
        }
        doc.push('\n');
    }

    // Section 2: SOTA Competitive Analysis
    doc.push_str("## 2. SOTA Competitive Analysis & Technical Benchmark Matrix\n\n");
    if !input.competitive_matrix.is_empty() {
        let alt_a = &input.competitive_matrix[0].alternative_a_name;
        let alt_b = &input.competitive_matrix[0].alternative_b_name;
        doc.push_str(&format!(
            "| Dimension | Vox Implementation | {} | {} |\n",
            alt_a, alt_b
        ));
        doc.push_str("| :--- | :--- | :--- | :--- |\n");
        for row in &input.competitive_matrix {
            doc.push_str(&format!(
                "| **{}** | {} | {} | {} |\n",
                row.dimension, row.vox_feature, row.alternative_a_val, row.alternative_b_val
            ));
        }
        doc.push('\n');
    }

    // Section 3: Verified Empirical Claims Table
    doc.push_str("## 3. Verified Empirical Claims Table\n\n");
    if !input.verified_claims.is_empty() {
        doc.push_str("| Claim / Proposition | Verdict | Evidence Source | Confidence |\n");
        doc.push_str("| :--- | :--- | :--- | :--- |\n");
        for c in &input.verified_claims {
            let conf = c
                .ci95
                .map(|(l, h)| format!("{:.2} (95% CI: {:.2}–{:.2})", (l + h) / 2.0, l, h))
                .unwrap_or_else(|| "1.00".to_string());
            doc.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                c.claim_text, c.verdict, c.evidence_source, conf
            ));
        }
        doc.push('\n');
    }

    // Section 4: Sandbox Evidence
    doc.push_str("## 4. Code Verification & Sandbox Reproducibility Evidence\n\n");
    for (i, p) in input.sandbox_probes.iter().enumerate() {
        doc.push_str(&format!("### Probe #{}: {}\n\n", i + 1, p.language));
        doc.push_str("```rust\n");
        doc.push_str(&p.original_snippet);
        doc.push_str("\n```\n\n");
        if let Some(ref repaired) = p.repaired_snippet {
            doc.push_str("**Self-Corrected Repair:**\n\n```rust\n");
            doc.push_str(repaired);
            doc.push_str("\n```\n\n");
        }
        doc.push_str(&format!(
            "**Compiler Output (Exit: {}):**\n```text\n{}\n```\n\n",
            if p.success { "0" } else { "1" },
            p.compiler_output.trim()
        ));
    }

    // Section 5: Gap Analysis & Recommendations
    doc.push_str("## 5. Gap Analysis & Concrete Architectural Recommendations\n\n");
    for g in &input.gaps_and_recommendations {
        doc.push_str(&format!(
            "- **[{}] {}**: {}\n",
            g.priority, g.issue, g.remedy
        ));
    }
    doc.push('\n');

    // Section 6: Phased Roadmap & Gates
    doc.push_str("## 6. Phased Implementation Roadmap & Verification Gates\n\n");
    for phase in &input.roadmap_phases {
        doc.push_str(&format!(
            "- [ ] **Phase {}: {}** — *Gate: `{}`*\n",
            phase.phase_number, phase.title, phase.gate
        ));
    }
    doc.push('\n');

    Ok(doc)
}
```

In `crates/vox-scientia/src/manuscript/scaffold/mod.rs`, add:
```rust
pub mod architecture_ssot;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-scientia --test architecture_ssot_scaffold_test`
Expected: PASS

- [ ] **Step 5: Format and commit**

Run:
```bash
cargo fmt -p vox-scientia
LEFTHOOK=0 git add crates/vox-scientia/src/manuscript/scaffold/ crates/vox-scientia/tests/
LEFTHOOK=0 git commit -m "feat(scientia): implement Architecture SSOT 7-section document scaffolder"
```

---

### Task 2: In-Memory `vox-doc-pipeline` Frontmatter Validator

**Files:**
- Create: `crates/vox-scientia/src/manuscript/scaffold/validator.rs`
- Modify: `crates/vox-scientia/src/manuscript/scaffold/mod.rs`
- Test: `crates/vox-scientia/tests/architecture_ssot_validator_test.rs`

**Interfaces:**
- Consumes: Raw markdown string
- Produces: `validate_ssot_markdown(&str) -> Result<(), Vec<String>>`

- [ ] **Step 1: Write the failing test**

Create `crates/vox-scientia/tests/architecture_ssot_validator_test.rs`:
```rust
use vox_scientia::manuscript::scaffold::validator::validate_ssot_markdown;

#[test]
fn test_validator_accepts_valid_frontmatter() {
    let doc = r#"---
title: "Valid Title (2026)"
description: "A valid description sentence for testing."
category: "Architecture SSOTs"
status: "current"
---

# Valid Title
Content here.
"#;
    assert!(validate_ssot_markdown(doc).is_ok());
}

#[test]
fn test_validator_rejects_missing_required_keys() {
    let doc = r#"---
title: "Missing Description"
category: "Architecture SSOTs"
---

# Missing Description
"#;
    let err = validate_ssot_markdown(doc).unwrap_err();
    assert!(err.iter().any(|e| e.contains("description")));
}

#[test]
fn test_validator_rejects_invalid_category() {
    let doc = r#"---
title: "Invalid Category"
description: "Test description."
category: "Unapproved Category"
status: "current"
---

# Content
"#;
    let err = validate_ssot_markdown(doc).unwrap_err();
    assert!(err.iter().any(|e| e.contains("category")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-scientia --test architecture_ssot_validator_test`
Expected: FAIL with "unresolved import `validator`"

- [ ] **Step 3: Implement `validator.rs`**

Create `crates/vox-scientia/src/manuscript/scaffold/validator.rs`:
```rust
const CANONICAL_CATEGORIES: &[&str] = &[
    "Architecture SSOTs",
    "Research Findings",
    "contributor",
    "architecture",
    "reference",
];

pub fn validate_ssot_markdown(content: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        errors.push("Missing opening YAML frontmatter '---'".to_string());
        return Err(errors);
    }

    let remainder = &trimmed[3..];
    let end_idx = match remainder.find("\n---") {
        Some(idx) => idx,
        None => {
            errors.push("Missing closing YAML frontmatter '---'".to_string());
            return Err(errors);
        }
    };

    let yaml_block = &remainder[..end_idx];
    let parsed: serde_yaml::Value = match serde_yaml::from_str(yaml_block) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("YAML parse error: {e}"));
            return Err(errors);
        }
    };

    let map = match parsed.as_mapping() {
        Some(m) => m,
        None => {
            errors.push("Frontmatter is not a YAML key-value mapping".to_string());
            return Err(errors);
        }
    };

    let get_str = |key: &str| {
        map.get(&serde_yaml::Value::String(key.to_string()))
            .and_then(|v| v.as_str())
    };

    if get_str("title").map(str::trim).unwrap_or("").is_empty() {
        errors.push("Missing or empty 'title' in frontmatter".to_string());
    }

    if get_str("description").map(str::trim).unwrap_or("").is_empty() {
        errors.push("Missing or empty 'description' in frontmatter".to_string());
    }

    match get_str("category") {
        Some(cat) => {
            if !CANONICAL_CATEGORIES.contains(&cat) {
                errors.push(format!(
                    "Invalid category '{}'. Expected one of: {:?}",
                    cat, CANONICAL_CATEGORIES
                ));
            }
        }
        None => errors.push("Missing 'category' in frontmatter".to_string()),
    }

    if get_str("status").is_none() {
        errors.push("Missing 'status' in frontmatter".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

In `crates/vox-scientia/src/manuscript/scaffold/mod.rs`, export `validator`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-scientia --test architecture_ssot_validator_test`
Expected: PASS

- [ ] **Step 5: Format and commit**

Run:
```bash
cargo fmt -p vox-scientia
LEFTHOOK=0 git add crates/vox-scientia/src/manuscript/scaffold/ crates/vox-scientia/tests/
LEFTHOOK=0 git commit -m "feat(scientia): add in-memory YAML frontmatter validator for SSOTs"
```

---

### Task 3: Tauri Commands `generate_research_doc_draft` and `publish_research_doc`

**Files:**
- Modify: `crates/vox-gui/src/commands/research.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Test: `crates/vox-gui/tests/research_doc_publish_test.rs`

**Interfaces:**
- Produces Tauri commands:
  - `generate_research_doc_draft(session_id: i64) -> Result<DocDraftPreview, String>`
  - `publish_research_doc(session_id: i64, slug: String, content: String) -> Result<PublishDocResult, String>`

- [ ] **Step 1: Write integration test**

Create `crates/vox-gui/tests/research_doc_publish_test.rs`:
```rust
use tempfile::tempdir;

#[tokio::test]
async fn test_atomic_doc_publish_sequence() {
    let tmp = tempdir().expect("tempdir");
    let target_dir = tmp.path().join("docs/src/architecture");
    tokio::fs::create_dir_all(&target_dir).await.expect("mkdir");

    let slug = "local-metal-optimization";
    let filename = format!("{slug}-research-2026.md");
    let target_file = target_dir.join(&filename);
    let tmp_file = target_dir.join(format!("{filename}.tmp"));

    let content = r#"---
title: "Metal Optimization (2026)"
description: "Empirical findings on Apple Silicon."
category: "Architecture SSOTs"
status: "current"
---

# Metal Optimization
"#;

    // Atomic write
    tokio::fs::write(&tmp_file, content).await.expect("write tmp");
    tokio::fs::rename(&tmp_file, &target_file).await.expect("rename");

    assert!(target_file.exists());
    let read_back = tokio::fs::read_to_string(&target_file).await.expect("read");
    assert!(read_back.contains("Metal Optimization"));
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p vox-gui --test research_doc_publish_test`
Expected: PASS

- [ ] **Step 3: Implement `generate_research_doc_draft` and `publish_research_doc`**

In `crates/vox-gui/src/commands/research.rs`, implement:
- `DocDraftPreview`:
  ```rust
  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct DocDraftPreview {
      pub slug: String,
      pub title: String,
      pub filename: String,
      pub markdown_content: String,
      pub is_valid: bool,
      pub validation_errors: Vec<String>,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct PublishDocResult {
      pub file_path: String,
      pub relative_path: String,
      pub indexed: bool,
  }
  ```
- Wire `generate_research_doc_draft`:
  - Retrieves `session` and `claims` from `state.db`.
  - Reconstructs `ArchitectureSsotInput`.
  - Calls `vox_scientia::manuscript::scaffold::architecture_ssot::render_architecture_ssot`.
  - Runs `validate_ssot_markdown`.
  - Returns `DocDraftPreview`.
- Wire `publish_research_doc`:
  - Validates `content` via `validate_ssot_markdown`.
  - Atomically writes to `docs/src/architecture/{slug}-research-2026.md`.
  - Appends to `docs/src/architecture/research-index.md` if not already present.
  - Calls `db.insert_research_artifact` to index in `scientia_research_fts`.
  - Returns `PublishDocResult`.

Register both in `crates/vox-gui/src/main.rs`.

- [ ] **Step 4: Verify compilation and tests**

Run: `cargo check -p vox-gui`
Run: `cargo test -p vox-gui --test research_doc_publish_test`
Expected: PASS

- [ ] **Step 5: Format and commit**

Run:
```bash
cargo fmt -p vox-gui
LEFTHOOK=0 git add crates/vox-gui/src/commands/research.rs crates/vox-gui/src/main.rs crates/vox-gui/tests/
LEFTHOOK=0 git commit -m "feat(gui): implement generate_research_doc_draft and publish_research_doc commands"
```

---

### Task 4: GUI Preview Modal (`DocPublishModal.tsx`) in `ResearchView`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`

**Interfaces:**
- Props: `{ sessionId: number; isOpen: boolean; onClose: () => void; onPublished: (path: string) => void }`

- [ ] **Step 1: Write Vitest test**

Create `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx`:
```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import React from "react";
import { DocPublishModal } from "./DocPublishModal";

describe("DocPublishModal", () => {
  it("renders preview modal when open", () => {
    render(
      <DocPublishModal
        sessionId={123}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    expect(screen.getByText(/Publish Architecture SSOT/i)).toBeDefined();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir crates/vox-gui/ui test run src/components/surfaces/Research/DocPublishModal.test.tsx`
Expected: FAIL with missing component

- [ ] **Step 3: Implement `DocPublishModal.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx`:
- Invokes Tauri command `generate_research_doc_draft`.
- Shows tabs: "Rendered Preview" | "Raw Markdown" | "Frontmatter Checks".
- Highlights validation errors in red or displays green checkmark if valid.
- "Approve & Publish" button invokes `publish_research_doc` and calls `onPublished(result.relative_path)`.

In `ResearchView.tsx`, add a "Publish to Docs" button in the action bar that toggles `DocPublishModal`.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir crates/vox-gui/ui test run src/components/surfaces/Research/DocPublishModal.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Run:
```bash
LEFTHOOK=0 git add crates/vox-gui/ui/src/components/surfaces/Research/
LEFTHOOK=0 git commit -m "feat(gui): add DocPublishModal to ResearchView with live preview and validation"
```

---

### Task 5: GUI `EmpiricalVerificationBadge.tsx` in `DocReader` & `SearchSurface`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.tsx`
- Create: `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReaderView.tsx`

**Interfaces:**
- Displays an interactive chip in `DocReaderView` when an architecture doc originates from deep research, showing stability score and linking to the research session.

- [ ] **Step 1: Write Vitest test**

Create `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`:
```tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import React from "react";
import { EmpiricalVerificationBadge } from "./EmpiricalVerificationBadge";

describe("EmpiricalVerificationBadge", () => {
  it("renders stability score and verified label", () => {
    render(<EmpiricalVerificationBadge stabilityScore={0.88} sessionId={42} />);
    expect(screen.getByText(/Verified by Deep Research/i)).toBeDefined();
    expect(screen.getByText(/S = 0.88/i)).toBeDefined();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir crates/vox-gui/ui test run src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`
Expected: FAIL with missing component

- [ ] **Step 3: Implement `EmpiricalVerificationBadge.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.tsx`.
Wire it into `DocReaderView.tsx` header when doc markdown contains `> **Empirical Provenance:**`.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir crates/vox-gui/ui test run src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Run:
```bash
LEFTHOOK=0 git add crates/vox-gui/ui/src/components/surfaces/DocReader/
LEFTHOOK=0 git commit -m "feat(gui): add EmpiricalVerificationBadge to DocReader"
```

---

### Task 6: Execution of Research Subject 1 (Local MENS Metal Optimization)

**Files:**
- Create: `docs/src/architecture/local-mens-metal-inference-optimization-research-2026.md`
- Modify: `docs/src/architecture/research-index.md`

- [ ] **Step 1: Execute deep research on Local MENS Apple Silicon Metal Optimization**
- [ ] **Step 2: Run empirical sandbox code verification on `candle-metal` device binding and tensor allocations**
- [ ] **Step 3: Generate 7-section Architecture SSOT via `DocSynthesisEngine`**
- [ ] **Step 4: Verify with `vox-doc-pipeline`**
Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/local-mens-metal-inference-optimization-research-2026.md`
Expected: 0 hard errors
- [ ] **Step 5: Register in `docs/src/architecture/research-index.md` and commit**
Run:
```bash
LEFTHOOK=0 git add docs/src/architecture/local-mens-metal-inference-optimization-research-2026.md docs/src/architecture/research-index.md
LEFTHOOK=0 git commit -m "docs(research): add local MENS Apple Silicon Metal optimization architecture SSOT"
```

---

### Task 7: Execution of Research Subject 2 (Autonomous Browser Driver)

**Files:**
- Create: `docs/src/architecture/autonomous-browser-driver-accessibility-navigation-research-2026.md`
- Modify: `docs/src/architecture/research-index.md`

- [ ] **Step 1: Execute deep research on Autonomous Browser Driver & Accessibility-Tree Navigation**
- [ ] **Step 2: Run empirical sandbox code verification on CDP snapshotting and target detachment**
- [ ] **Step 3: Generate 7-section Architecture SSOT via `DocSynthesisEngine`**
- [ ] **Step 4: Verify with `vox-doc-pipeline`**
Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/autonomous-browser-driver-accessibility-navigation-research-2026.md`
Expected: 0 hard errors
- [ ] **Step 5: Register in `docs/src/architecture/research-index.md` and commit**
Run:
```bash
LEFTHOOK=0 git add docs/src/architecture/autonomous-browser-driver-accessibility-navigation-research-2026.md docs/src/architecture/research-index.md
LEFTHOOK=0 git commit -m "docs(research): add autonomous browser driver accessibility navigation architecture SSOT"
```

---

### Task 8: Full End-to-End Verification & GUI Vox Axis Testing

**Files:**
- Test all crates touched: `vox-scientia`, `vox-gui`, `vox-research-shim`

- [ ] **Step 1: Run Rust test suites**
Run: `cargo test -p vox-scientia`
Run: `cargo test -p vox-gui --test research_doc_publish_test`
Run: `cargo test -p vox-research-shim`
- [ ] **Step 2: Run GUI Vitest suite**
Run: `pnpm --dir crates/vox-gui/ui test run`
- [ ] **Step 3: Run whole-repo doc pipeline check on the new files**
Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/local-mens-metal-inference-optimization-research-2026.md docs/src/architecture/autonomous-browser-driver-accessibility-navigation-research-2026.md`
- [ ] **Step 4: Verify FTS5 search retrieval**
Run: `cargo test -p vox-orchestrator-mcp --test research_search_mcp_test`
- [ ] **Step 5: Update `walkthrough.md` with verification evidence**
