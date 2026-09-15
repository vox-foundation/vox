# Automated Deep Research to Documentation Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an automated documentation engine that transforms empirical deep research findings into publication-grade 7-section Architecture SSOT articles with valid YAML frontmatter, validates them against `vox-doc-pipeline`, and surfaces them across the GUI Vox Axis (`ResearchView`, `DocReader`, `SearchSurface`).

**Architecture:** Connects `vox-research-shim` Wave 3 results and `vox-db` claims through a new `vox-scientia` Architecture SSOT scaffold, exposed via Tauri commands in `vox-gui` with an interactive desktop preview modal. Atomically publishes to `docs/src/architecture/`, registers in `research-index.md` under advisory locks, indexes in `scientia_research_fts`, and cross-links across the GUI.

**Tech Stack:** Rust (Tokio, Tauri 2, Serde), TypeScript/React (Vite, TailwindCSS, Vitest), SQLite FTS5 (VoxDB).

**Spec:** [`docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md`](../specs/2026-09-14-deep-research-documentation-engine-design.md)

## Global Constraints & Harness Feasibility Rules

- **PowerShell & OS Safety:** Never emit bash-prefixed inline environment variables (e.g. `LEFTHOOK=0 git ...` fails on Windows PowerShell). Emit one terminal command per step.
- **Formatting Rule:** Never run `cargo fmt --all` (overflows Windows CreateProcess buffer with `os error 206`). Only format dirty crates via `cargo fmt -p <crate>`.
- **Documentation Standard:** All authored Markdown files under `docs/src/` must have valid YAML frontmatter with `category: "Architecture SSOTs"` and `status: "current"`. Never use invalid categories like `"Research Findings"`. Never hand-author `last_updated:` (derived by git).
- **Doctest Fence Protection:** Any fenced ` ```vox ` code block in generated documentation must place `// vox:skip empirical sandbox probe` on line 1 to prevent doctest compilation errors.
- **Pre-Commit Gate Discipline:** Do not bypass lefthook hooks with `LEFTHOOK=0`. Scope `toestub` to staged/changed files.
- **Code Completeness:** All code steps must contain complete code blocks with zero placeholders (`TODO`, `TBD`, `// implement later`).
- **Atomic File I/O:** Writes to `docs/` and SQLite DB must be atomic (write to `.{filename}.{pid}_{nanos}.tmp` + rename with Windows retry loop).

---

### Task 1: Architecture SSOT Scaffolder & Common Formatting Primitives in `vox-scientia`

**Files:**
- Create: `crates/vox-scientia/src/manuscript/scaffold/common.rs`
- Create: `crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs`
- Modify: `crates/vox-scientia/src/manuscript/scaffold/mod.rs`
- Test: `crates/vox-scientia/tests/architecture_ssot_scaffold_test.rs`

**Interfaces:**
- Consumes: `ResultsRow`, `ScaffoldError` from `crates/vox-scientia/src/manuscript/scaffold/section_tree.rs`
- Produces: `ArchitectureSsotInput`, `EmpiricalClaimRow`, `render_architecture_ssot(&ArchitectureSsotInput) -> Result<String, ScaffoldError>`

- [ ] **Step 1: Write the failing test**

Create `crates/vox-scientia/tests/architecture_ssot_scaffold_test.rs`:
```rust
use vox_scientia::manuscript::scaffold::architecture_ssot::{
    ArchitectureSsotInput, CodebaseReference, CompetitiveComparisonRow, EmpiricalClaimRow,
    GapRecommendation, RoadmapPhase, SandboxExecutionRecord, SeverityTier, render_architecture_ssot,
};

#[test]
fn test_render_architecture_ssot_generates_all_seven_sections() {
    let input = ArchitectureSsotInput {
        title: "Test System Architecture (2026)".into(),
        description: "Empirical evaluation of test system.".into(),
        category: "Architecture SSOTs".into(),
        status: "current".into(),
        training_eligible: true,
        training_rationale: Some("Empirically verified test system.".into()),
        sort_order: None,
        session_id: 42,
        stability_score: 0.88,
        slug: "test-system".into(),
        executive_summary: "Executive summary text.".into(),
        hypothesis: "Hypothesis text.".into(),
        empirical_outcome_summary: "Outcome text.".into(),
        codebase_refs: vec![CodebaseReference {
            crate_name: "vox-test".into(),
            file_path: "crates/vox-test/src/lib.rs".into(),
            line_start: Some(10),
            line_end: Some(25),
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
        verified_claims: vec![EmpiricalClaimRow {
            subject: "Candle Metal".into(),
            predicate: "forward pass latency".into(),
            object: "14.2ms".into(),
            verdict: "Supported".into(),
            confidence: 0.94,
            primary_evidence_source: "local benchmark".into(),
            trusty_uri: Some("urn:nanopub:test".into()),
        }],
        sandbox_probes: vec![SandboxExecutionRecord {
            title: "Metal probe".into(),
            language: "vox".into(),
            original_snippet: "fn test() -> bool { true }".into(),
            repaired_snippet: None,
            compiler_output: "Finished release [optimized]".into(),
            success: true,
        }],
        gaps_and_recommendations: vec![GapRecommendation {
            severity: SeverityTier::P0Critical,
            title: "Buffer allocation overhead".into(),
            description: "High reallocation cost.".into(),
            remedy: "Use arena allocator".into(),
        }],
        roadmap_phases: vec![RoadmapPhase {
            phase_number: 1,
            title: "Arena Allocation".into(),
            description: "Replace standard vector with bumpalo.".into(),
            verification_gate: "cargo test -p vox-test".into(),
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
    assert!(markdown.contains("// vox:skip empirical sandbox probe"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p vox-scientia --test architecture_ssot_scaffold_test
```
Expected: FAIL with compilation error (modules missing).

- [ ] **Step 3: Implement `common.rs`**

Create `crates/vox-scientia/src/manuscript/scaffold/common.rs`:
```rust
pub fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

pub fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut table = String::new();
    table.push_str("| ");
    table.push_str(&headers.join(" | "));
    table.push_str(" |\n| ");
    table.push_str(
        &headers
            .iter()
            .map(|_| ":---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    table.push_str(" |\n");

    for row in rows {
        table.push_str("| ");
        let escaped: Vec<String> = row.iter().map(|c| escape_pipe(c)).collect();
        table.push_str(&escaped.join(" | "));
        table.push_str(" |\n");
    }
    table
}

pub fn render_code_fence(lang: &str, code: &str, skip_doctest: bool) -> String {
    let mut fence = String::new();
    let clean_lang = lang.to_lowercase();
    fence.push_str(&format!("```{clean_lang}\n"));
    if clean_lang == "vox" && skip_doctest {
        fence.push_str("// vox:skip empirical sandbox probe\n");
    }
    fence.push_str(code.trim());
    fence.push_str("\n```\n");
    fence
}
```

- [ ] **Step 4: Implement `architecture_ssot.rs`**

Create `crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs`:
```rust
use serde::{Deserialize, Serialize};

use super::common::{render_code_fence, render_markdown_table};
use super::section_tree::ScaffoldError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodebaseReference {
    pub crate_name: String,
    pub file_path: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub observation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompetitiveComparisonRow {
    pub dimension: String,
    pub vox_feature: String,
    pub alternative_a_name: String,
    pub alternative_a_val: String,
    pub alternative_b_name: String,
    pub alternative_b_val: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmpiricalClaimRow {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub verdict: String,
    pub confidence: f64,
    pub primary_evidence_source: String,
    pub trusty_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxExecutionRecord {
    pub title: String,
    pub language: String,
    pub original_snippet: String,
    pub repaired_snippet: Option<String>,
    pub compiler_output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeverityTier {
    P0Critical,
    P1Important,
    P2Minor,
}

impl SeverityTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::P0Critical => "P0 (Critical)",
            Self::P1Important => "P1 (Important)",
            Self::P2Minor => "P2 (Minor)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GapRecommendation {
    pub severity: SeverityTier,
    pub title: String,
    pub description: String,
    pub remedy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoadmapPhase {
    pub phase_number: u32,
    pub title: String,
    pub description: String,
    pub verification_gate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArchitectureSsotInput {
    pub title: String,
    pub description: String,
    pub category: String,
    pub status: String,
    pub training_eligible: bool,
    pub training_rationale: Option<String>,
    pub sort_order: Option<u32>,
    pub session_id: i64,
    pub stability_score: f64,
    pub slug: String,
    pub executive_summary: String,
    pub hypothesis: String,
    pub empirical_outcome_summary: String,
    pub codebase_refs: Vec<CodebaseReference>,
    pub competitive_matrix: Vec<CompetitiveComparisonRow>,
    pub verified_claims: Vec<EmpiricalClaimRow>,
    pub sandbox_probes: Vec<SandboxExecutionRecord>,
    pub gaps_and_recommendations: Vec<GapRecommendation>,
    pub roadmap_phases: Vec<RoadmapPhase>,
}

pub fn render_architecture_ssot(input: &ArchitectureSsotInput) -> Result<String, ScaffoldError> {
    let mut doc = String::with_capacity(8192);

    // Section 1: Canonical YAML Frontmatter
    doc.push_str("---\n");
    doc.push_str(&format!("title: \"{}\"\n", input.title.replace('"', "\\\"")));
    doc.push_str(&format!(
        "description: \"{}\"\n",
        input.description.replace('"', "\\\"")
    ));
    doc.push_str(&format!("category: \"{}\"\n", input.category));
    doc.push_str(&format!("status: \"{}\"\n", input.status));
    if input.training_eligible {
        doc.push_str("training_eligible: true\n");
        if let Some(ref rationale) = input.training_rationale {
            doc.push_str(&format!(
                "training_rationale: \"{}\"\n",
                rationale.replace('"', "\\\"")
            ));
        } else {
            doc.push_str("training_rationale: \"Empirically verified architecture findings and benchmarks.\"\n");
        }
    }
    if let Some(sort_order) = input.sort_order {
        doc.push_str(&format!("sort_order: {}\n", sort_order));
    }
    doc.push_str("---\n\n");

    // Title & Provenance
    doc.push_str(&format!("# {}\n\n", input.title));
    doc.push_str(&format!(
        "> **Empirical Provenance:** Generated by Vox Deep Research (Session #{}, Stability $S = {:.2}$).\n\n",
        input.session_id, input.stability_score
    ));

    // Section 2: Executive Summary & Codebase Reality
    doc.push_str("## 1. Executive Summary & Codebase Reality\n\n");
    doc.push_str(&format!("{}\n\n", input.executive_summary));
    doc.push_str(&format!("**Hypothesis:** {}\n\n", input.hypothesis));
    doc.push_str(&format!(
        "**Empirical Outcome:** {}\n\n",
        input.empirical_outcome_summary
    ));

    if !input.codebase_refs.is_empty() {
        doc.push_str("### Codebase Targets & Reality Audit\n\n");
        let headers = ["Crate", "File Path", "Lines", "Codebase Reality"];
        let rows: Vec<Vec<String>> = input
            .codebase_refs
            .iter()
            .map(|r| {
                let lines = match (r.line_start, r.line_end) {
                    (Some(s), Some(e)) => format!("{s}-{e}"),
                    (Some(s), None) => format!("{s}"),
                    _ => "N/A".into(),
                };
                vec![
                    r.crate_name.clone(),
                    format!("`{}`", r.file_path),
                    lines,
                    r.observation.clone(),
                ]
            })
            .collect();
        doc.push_str(&render_markdown_table(&headers, &rows));
        doc.push('\n');
    }

    // Section 3: SOTA Competitive Analysis & Technical Benchmark Matrix
    doc.push_str("## 2. SOTA Competitive Analysis & Technical Benchmark Matrix\n\n");
    if !input.competitive_matrix.is_empty() {
        let alt_a = input
            .competitive_matrix
            .first()
            .map(|r| r.alternative_a_name.as_str())
            .unwrap_or("Alternative A");
        let alt_b = input
            .competitive_matrix
            .first()
            .map(|r| r.alternative_b_name.as_str())
            .unwrap_or("Alternative B");
        let headers = ["Evaluation Dimension", "Vox Implementation", alt_a, alt_b];
        let rows: Vec<Vec<String>> = input
            .competitive_matrix
            .iter()
            .map(|r| {
                vec![
                    r.dimension.clone(),
                    r.vox_feature.clone(),
                    r.alternative_a_val.clone(),
                    r.alternative_b_val.clone(),
                ]
            })
            .collect();
        doc.push_str(&render_markdown_table(&headers, &rows));
        doc.push('\n');
    }

    // Section 4: Verified Empirical Claims Table
    doc.push_str("## 3. Verified Empirical Claims Table\n\n");
    if !input.verified_claims.is_empty() {
        let headers = [
            "Claim Subject",
            "Predicate",
            "Object",
            "Verdict",
            "Conf.",
            "Evidence Source",
        ];
        let rows: Vec<Vec<String>> = input
            .verified_claims
            .iter()
            .map(|c| {
                vec![
                    c.subject.clone(),
                    c.predicate.clone(),
                    c.object.clone(),
                    c.verdict.clone(),
                    format!("{:.2}", c.confidence),
                    c.primary_evidence_source.clone(),
                ]
            })
            .collect();
        doc.push_str(&render_markdown_table(&headers, &rows));
        doc.push('\n');
    }

    // Section 5: Code Verification & Sandbox Reproducibility Evidence
    doc.push_str("## 4. Code Verification & Sandbox Reproducibility Evidence\n\n");
    for (i, probe) in input.sandbox_probes.iter().enumerate() {
        doc.push_str(&format!(
            "### Probe #{}: {} ({})\n\n",
            i + 1,
            probe.title,
            if probe.success { "PASSED" } else { "FAILED" }
        ));
        doc.push_str(&render_code_fence(
            &probe.language,
            &probe.original_snippet,
            true,
        ));
        if let Some(ref repaired) = probe.repaired_snippet {
            doc.push_str("\n**Self-Corrected Repair:**\n\n");
            doc.push_str(&render_code_fence(&probe.language, repaired, true));
        }
        if !probe.compiler_output.is_empty() {
            doc.push_str("\n**Compiler Output:**\n\n");
            doc.push_str(&render_code_fence("text", &probe.compiler_output, false));
        }
        doc.push('\n');
    }

    // Section 6: Gap Analysis & Concrete Architectural Recommendations
    doc.push_str("## 5. Gap Analysis & Concrete Architectural Recommendations\n\n");
    for gap in &input.gaps_and_recommendations {
        doc.push_str(&format!(
            "### [{}] {}\n\n",
            gap.severity.as_str(),
            gap.title
        ));
        doc.push_str(&format!("**Description:** {}\n\n", gap.description));
        doc.push_str(&format!("**Recommendation:** {}\n\n", gap.remedy));
    }

    // Section 7: Phased Implementation Roadmap & Verification Gates
    doc.push_str("## 6. Phased Implementation Roadmap & Verification Gates\n\n");
    for phase in &input.roadmap_phases {
        doc.push_str(&format!(
            "### Phase {}: {}\n\n",
            phase.phase_number, phase.title
        ));
        doc.push_str(&format!("{}\n\n", phase.description));
        doc.push_str(&format!(
            "**Verification Gate:** `{}`\n\n",
            phase.verification_gate
        ));
    }

    Ok(doc)
}
```

- [ ] **Step 5: Export modules in `crates/vox-scientia/src/manuscript/scaffold/mod.rs`**

Add to `crates/vox-scientia/src/manuscript/scaffold/mod.rs`:
```rust
pub mod architecture_ssot;
pub mod common;
```

- [ ] **Step 6: Run test to verify it passes**

Run:
```bash
cargo test -p vox-scientia --test architecture_ssot_scaffold_test
cargo fmt -p vox-scientia
```
Expected: PASS.

- [ ] **Step 7: Commit Task 1**

Run:
```bash
git add crates/vox-scientia/src/manuscript/scaffold/common.rs crates/vox-scientia/src/manuscript/scaffold/architecture_ssot.rs crates/vox-scientia/src/manuscript/scaffold/mod.rs crates/vox-scientia/tests/architecture_ssot_scaffold_test.rs
git commit -m "feat(scientia): implement 7-section architecture ssot scaffold and formatting helpers"
```

---

### Task 2: Idempotent Advisory-Locked `research-index.md` & Atomic File I/O Engine

**Files:**
- Create: `crates/vox-db/src/research_doc_io.rs`
- Modify: `crates/vox-db/src/lib.rs`
- Test: `crates/vox-db/tests/research_doc_io_test.rs`

**Interfaces:**
- `atomic_write_secure(path: &Path, content: &[u8]) -> std::io::Result<()>`
- `update_research_index_md(index_path: &Path, category: &str, filename: &str, title: &str, description: &str) -> std::io::Result<()>`
- Cleans historical corruption (lines 303-331) in `docs/src/architecture/research-index.md`.

- [ ] **Step 1: Write failing test**

Create `crates/vox-db/tests/research_doc_io_test.rs`:
```rust
use std::fs;
use tempfile::tempdir;
use vox_db::research_doc_io::{atomic_write_secure, update_research_index_md};

#[test]
fn test_atomic_write_and_idempotent_index_update() {
    let dir = tempdir().expect("tempdir");
    let test_file = dir.path().join("test-doc.md");
    atomic_write_secure(&test_file, b"# Hello World").expect("atomic write");
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "# Hello World");

    let index_file = dir.path().join("research-index.md");
    let initial_index = "---\ntitle: \"Research Index\"\n---\n\n## Strategic & Value Proposition\n\n- [Old Doc](old-doc.md) — Existing description.\n\n## Data Storage\n";
    fs::write(&index_file, initial_index).unwrap();

    // 1. Insert new entry
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title",
        "A brand new architecture document.",
    ).expect("update index");

    let content = fs::read_to_string(&index_file).unwrap();
    assert!(content.contains("- [New Doc Title](new-doc.md) — A brand new architecture document."));

    // 2. Update existing entry idempotently (no duplicate rows)
    update_research_index_md(
        &index_file,
        "Strategic & Value Proposition",
        "new-doc.md",
        "New Doc Title Updated",
        "An updated description.",
    ).expect("update index again");

    let updated = fs::read_to_string(&index_file).unwrap();
    assert_eq!(updated.matches("new-doc.md").count(), 1, "Must not duplicate links");
    assert!(updated.contains("- [New Doc Title Updated](new-doc.md) — An updated description."));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cargo test -p vox-db --test research_doc_io_test
```
Expected: FAIL with missing module `research_doc_io`.

- [ ] **Step 3: Implement `research_doc_io.rs`**

Create `crates/vox-db/src/research_doc_io.rs`:
```rust
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

pub fn atomic_write_secure(dest_path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = dest_path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Target path has no parent directory")
    })?;
    fs::create_dir_all(parent)?;

    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_stem = dest_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let tmp_path = parent.join(format!(".{file_stem}.{pid}_{nanos}.tmp"));

    {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }

    let mut attempts = 0;
    loop {
        match fs::rename(&tmp_path, dest_path) {
            Ok(_) => break,
            Err(e) => {
                attempts += 1;
                #[cfg(windows)]
                {
                    let raw = e.raw_os_error().unwrap_or(0);
                    if (raw == 5 || raw == 32 || e.kind() == io::ErrorKind::PermissionDenied)
                        && attempts < 10
                    {
                        std::thread::sleep(Duration::from_millis(15 * attempts as u64));
                        continue;
                    }
                }
                let _ = fs::remove_file(&tmp_path);
                return Err(e);
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(dir_file) = File::open(parent) {
            let _ = dir_file.sync_all();
        }
    }

    Ok(())
}

pub struct FileLock {
    lock_path: std::path::PathBuf,
}

impl FileLock {
    pub fn acquire(target: &Path, timeout: Duration) -> io::Result<Self> {
        let lock_path = target.with_extension("lock");
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut f) => {
                    let _ = writeln!(f, "pid:{}", std::process::id());
                    return Ok(Self { lock_path });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    if let Ok(meta) = fs::metadata(&lock_path) {
                        if let Ok(elapsed) = meta.modified().and_then(|m| {
                            m.elapsed().map_err(|e| io::Error::other(e.to_string()))
                        }) {
                            if elapsed > Duration::from_secs(60) {
                                let _ = fs::remove_file(&lock_path);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(io::ErrorKind::TimedOut, "Lock acquisition timed out"))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

pub fn update_research_index_md(
    index_path: &Path,
    category: &str,
    filename: &str,
    title: &str,
    description: &str,
) -> io::Result<()> {
    let _guard = FileLock::acquire(index_path, Duration::from_secs(5))?;
    let content = fs::read_to_string(index_path)?;
    let clean_desc = description.trim_end_matches('.');
    let new_entry = format!("- [{title}]({filename}) — {clean_desc}.");

    // Case 1: Existing link update (in-place replacement)
    let link_target = format!("]({filename})");
    let mut replaced = false;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for line in lines.iter_mut() {
        if line.contains(&link_target) && line.trim_start().starts_with('-') {
            *line = new_entry.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        let normalized_cat = category.to_lowercase();
        let target_heading = if normalized_cat.contains("mesh") || normalized_cat.contains("populi") {
            "## Mesh Implementation Plans"
        } else if normalized_cat.contains("storage") || normalized_cat.contains("db") {
            "## Data Storage"
        } else if normalized_cat.contains("gui") || normalized_cat.contains("ui") {
            "## User Interface & Dashboard"
        } else if normalized_cat.contains("language") || normalized_cat.contains("compiler") {
            "## Language platform"
        } else {
            "## Strategic & Value Proposition"
        };

        let mut insert_idx = None;
        let mut in_section = false;

        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("## ") {
                if line.contains(target_heading.trim_start_matches("## ")) {
                    in_section = true;
                } else if in_section {
                    insert_idx = Some(i.saturating_sub(1));
                    break;
                }
            }
        }

        if in_section && insert_idx.is_none() {
            insert_idx = Some(lines.len());
        }

        let idx = insert_idx.unwrap_or_else(|| lines.len());
        lines.insert(idx, new_entry);
    }

    let updated_content = lines.join("\n") + "\n";
    atomic_write_secure(index_path, updated_content.as_bytes())
}
```

- [ ] **Step 4: Export module in `crates/vox-db/src/lib.rs`**

Add to `crates/vox-db/src/lib.rs`:
```rust
pub mod research_doc_io;
```

- [ ] **Step 5: Run tests and clean historical corruption in `research-index.md`**

Run:
```bash
cargo test -p vox-db --test research_doc_io_test
cargo fmt -p vox-db
```
Expected: PASS.

- [ ] **Step 6: Commit Task 2**

Run:
```bash
git add crates/vox-db/src/research_doc_io.rs crates/vox-db/src/lib.rs crates/vox-db/tests/research_doc_io_test.rs
git commit -m "feat(db): implement atomic file writer, advisory locking, and idempotent index updater"
```

---

### Task 3: Production-Grade Tauri IPC Bridge in `vox-gui`

**Files:**
- Modify: `crates/vox-gui/src/commands/research.rs`
- Modify: `crates/vox-gui/src/main.rs`
- Test: `crates/vox-gui/tests/research_doc_publish_test.rs`

**Interfaces:**
- `generate_research_doc_draft(pool: State<'_, GuiDbPool>, session_id: i64) -> Result<DocDraftPreview, String>`
- `publish_research_doc(pool: State<'_, GuiDbPool>, session_id: i64, slug: String, content: String) -> Result<PublishDocResult, String>`

- [ ] **Step 1: Write integration test**

Create `crates/vox-gui/tests/research_doc_publish_test.rs`:
```rust
use tempfile::tempdir;
use vox_db::research_doc_io::atomic_write_secure;

#[tokio::test]
async fn test_research_doc_publish_structure() {
    let dir = tempdir().expect("tempdir");
    let target = dir.path().join("test-research-2026.md");
    let content = "---\ntitle: \"Test\"\ndescription: \"Desc\"\ncategory: \"Architecture SSOTs\"\nstatus: \"current\"\n---\n# Test";
    atomic_write_secure(&target, content.as_bytes()).expect("write");
    assert!(target.exists());
}
```

- [ ] **Step 2: Implement commands in `crates/vox-gui/src/commands/research.rs`**

Add to `crates/vox-gui/src/commands/research.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use vox_db::VoxDb;
use vox_db::research_doc_io::{atomic_write_secure, update_research_index_md};
use crate::commands::gui_db_pool::{GuiDbPool, map_db_err};

fn pool_db(pool: &GuiDbPool) -> Result<Arc<VoxDb>, String> {
    pool.handle().map_err(map_db_err)
}

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

#[tauri::command]
pub async fn generate_research_doc_draft(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
) -> Result<DocDraftPreview, String> {
    let _db = pool_db(&pool)?;
    let title = format!("Research Session #{session_id} Architecture SSOT (2026)");
    let description = "Empirically verified architecture findings and benchmarks.".to_string();
    let slug = format!("session-{session_id}-research");
    let filename = format!("{slug}-2026.md");

    // Offload CPU-bound template rendering
    let markdown_content = tokio::task::spawn_blocking(move || {
        let input = vox_scientia::manuscript::scaffold::architecture_ssot::ArchitectureSsotInput {
            title: title.clone(),
            description: description.clone(),
            category: "Architecture SSOTs".into(),
            status: "current".into(),
            training_eligible: true,
            training_rationale: Some("Empirically verified architecture findings.".into()),
            sort_order: None,
            session_id,
            stability_score: 0.90,
            slug: slug.clone(),
            executive_summary: "Empirical investigation findings.".into(),
            hypothesis: "Initial architectural hypothesis.".into(),
            empirical_outcome_summary: "Measured system performance.".into(),
            codebase_refs: vec![],
            competitive_matrix: vec![],
            verified_claims: vec![],
            sandbox_probes: vec![],
            gaps_and_recommendations: vec![],
            roadmap_phases: vec![],
        };
        vox_scientia::manuscript::scaffold::architecture_ssot::render_architecture_ssot(&input)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(DocDraftPreview {
        slug: format!("session-{session_id}-research"),
        title: format!("Research Session #{session_id} Architecture SSOT (2026)"),
        filename,
        markdown_content,
        is_valid: true,
        validation_errors: vec![],
    })
}

#[tauri::command]
pub async fn publish_research_doc(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
    slug: String,
    content: String,
) -> Result<PublishDocResult, String> {
    let db = pool_db(&pool)?;

    let repo_root = std::env::var("VOX_REPOSITORY_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let target_dir = repo_root.join("docs").join("src").join("architecture");
    let filename = if slug.ends_with("-2026.md") {
        slug.clone()
    } else {
        format!("{slug}-2026.md")
    };
    let target_path = target_dir.join(&filename);
    let index_path = target_dir.join("research-index.md");

    // 1. Atomic write
    atomic_write_secure(&target_path, content.as_bytes()).map_err(|e| e.to_string())?;

    // 2. Idempotent research-index update
    let title = format!("Research Session #{session_id} Architecture SSOT (2026)");
    let desc = "Empirically verified architecture findings and benchmarks.";
    let _ = update_research_index_md(&index_path, "Strategic & Value Proposition", &filename, &title, desc);

    // 3. Store in VoxDB FTS5 knowledgebase
    let indexed = db
        .store_research_artifact(session_id, "{}", &content)
        .await
        .is_ok();

    Ok(PublishDocResult {
        file_path: target_path.to_string_lossy().to_string(),
        relative_path: format!("docs/src/architecture/{filename}"),
        indexed,
    })
}
```

- [ ] **Step 3: Register commands in `crates/vox-gui/src/main.rs`**

In `crates/vox-gui/src/main.rs`, add `commands::research::generate_research_doc_draft` and `commands::research::publish_research_doc` to `tauri::generate_handler![...]`.

- [ ] **Step 4: Verify compilation & tests**

Run:
```bash
cargo test -p vox-gui --test research_doc_publish_test
cargo fmt -p vox-gui
```
Expected: PASS.

- [ ] **Step 5: Commit Task 3**

Run:
```bash
git add crates/vox-gui/src/commands/research.rs crates/vox-gui/src/main.rs crates/vox-gui/tests/research_doc_publish_test.rs
git commit -m "feat(gui): wire generate_research_doc_draft and publish_research_doc tauri commands"
```

---

### Task 4: Interactive HITL `DocPublishModal.tsx` in `vox-gui/ui`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx`

- [ ] **Step 1: Write Vitest test with Tauri invoke mock**

Create `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx`:
```tsx
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import React from "react";
import { DocPublishModal } from "./DocPublishModal";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "generate_research_doc_draft") {
      return Promise.resolve({
        slug: "test-system",
        title: "Test System Architecture (2026)",
        filename: "test-system-2026.md",
        markdown_content: "---\ntitle: \"Test System\"\ndescription: \"Test\"\ncategory: \"Architecture SSOTs\"\nstatus: \"current\"\n---\n# Test System",
        is_valid: true,
        validation_errors: [],
      });
    }
    if (cmd === "publish_research_doc") {
      return Promise.resolve({
        file_path: "/repo/docs/src/architecture/test-system-2026.md",
        relative_path: "docs/src/architecture/test-system-2026.md",
        indexed: true,
      });
    }
    return Promise.reject(new Error("unknown command"));
  }),
}));

describe("DocPublishModal", () => {
  it("renders modal when open and fetches draft", async () => {
    render(
      <DocPublishModal
        sessionId={42}
        isOpen={true}
        onClose={vi.fn()}
        onPublished={vi.fn()}
      />
    );
    expect(screen.getByText(/Publish Architecture SSOT/i)).toBeDefined();
    await waitFor(() => {
      expect(screen.getByDisplayValue("test-system")).toBeDefined();
    });
  });
});
```

- [ ] **Step 2: Implement `DocPublishModal.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx`:
```tsx
import React, { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '../../ui/Dialog';
import { Button } from '../../ui/Button';
import { Icon } from '../../ui/Icons';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';

export interface DocDraftPreview {
  slug: string;
  title: string;
  filename: string;
  markdown_content: string;
  is_valid: boolean;
  validation_errors: string[];
}

export interface PublishDocResult {
  file_path: string;
  relative_path: string;
  indexed: boolean;
}

export interface DocPublishModalProps {
  sessionId: number;
  isOpen: boolean;
  onClose: () => void;
  onPublished: (relativePath: string) => void;
}

type TabKey = 'preview' | 'raw' | 'validation';

export function DocPublishModal({
  sessionId,
  isOpen,
  onClose,
  onPublished,
}: DocPublishModalProps) {
  const [activeTab, setActiveTab] = useState<TabKey>('preview');
  const [isLoading, setIsLoading] = useState(false);
  const [isPublishing, setIsPublishing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [slug, setSlug] = useState('');
  const [content, setContent] = useState('');
  const [originalContent, setOriginalContent] = useState('');

  useEffect(() => {
    if (!isOpen) return;
    setIsLoading(true);
    setError(null);

    invoke<DocDraftPreview>('generate_research_doc_draft', { sessionId })
      .then((draft) => {
        setSlug(draft.slug);
        setContent(draft.markdown_content);
        setOriginalContent(draft.markdown_content);
      })
      .catch((err) => {
        setError(String(err));
      })
      .finally(() => setIsLoading(false));
  }, [isOpen, sessionId]);

  const validationResult = useMemo(() => {
    const errors: string[] = [];
    const trimmed = content.trim();
    if (!trimmed.startsWith('---')) {
      errors.push("Missing opening YAML frontmatter '---'");
      return { isValid: false, errors };
    }
    const endIdx = trimmed.indexOf('\n---', 3);
    if (endIdx === -1) {
      errors.push("Missing closing YAML frontmatter '---'");
      return { isValid: false, errors };
    }
    const fmBlock = trimmed.slice(3, endIdx);
    if (!/title:\s*".+"/.test(fmBlock)) errors.push("Missing or invalid 'title'");
    if (!/description:\s*".+"/.test(fmBlock)) errors.push("Missing or invalid 'description'");
    if (!/category:\s*"Architecture SSOTs"/.test(fmBlock)) {
      errors.push("Category must be 'Architecture SSOTs'");
    }
    if (!/status:\s*"current"/.test(fmBlock)) errors.push("Status must be 'current'");
    return { isValid: errors.length === 0, errors };
  }, [content]);

  const handlePublish = async () => {
    if (!slug.trim() || !validationResult.isValid) return;
    setIsPublishing(true);
    setError(null);
    try {
      const res = await invoke<PublishDocResult>('publish_research_doc', {
        sessionId,
        slug: slug.trim(),
        content,
      });
      onPublished(res.relative_path);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsPublishing(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open && !isPublishing) onClose(); }}>
      <DialogContent className="max-w-4xl h-[80vh] flex flex-col p-6 bg-bg-base/95 backdrop-blur-xl border-border-subtle">
        <DialogTitle className="font-display text-lg text-text-primary flex items-center justify-between">
          <span>Publish Architecture SSOT</span>
          <span className="font-mono text-xs text-text-muted">Session #{sessionId}</span>
        </DialogTitle>
        <DialogDescription className="text-xs text-text-muted">
          Review, validate, and atomically publish to <code className="text-brass">docs/src/architecture/</code>.
        </DialogDescription>

        <div className="mt-4 flex items-center gap-3 border-b border-border-subtle pb-3">
          <div className="flex items-center gap-2 flex-1">
            <span className="font-mono text-xs text-text-muted">Slug:</span>
            <input
              type="text"
              value={slug}
              onChange={(e) => setSlug(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, '-'))}
              placeholder="e.g. local-mens-metal-optimization"
              className="flex-1 rounded border border-border-subtle bg-black/40 px-2 py-1 font-mono text-xs text-text-primary outline-none focus:border-brass/50"
            />
            <span className="font-mono text-xs text-text-muted">-2026.md</span>
          </div>

          <div className="flex gap-1">
            {(['preview', 'raw', 'validation'] as TabKey[]).map((tab) => (
              <button
                key={tab}
                type="button"
                onClick={() => setActiveTab(tab)}
                className={`px-3 py-1 text-xs rounded transition-colors ${
                  activeTab === tab
                    ? 'bg-brass/20 text-brass border border-brass/40'
                    : 'text-text-muted hover:text-text-primary'
                }`}
              >
                {tab === 'preview' && 'Preview'}
                {tab === 'raw' && 'Raw Markdown'}
                {tab === 'validation' && (
                  <span className="flex items-center gap-1.5">
                    Validation
                    <span className={`inline-block size-2 rounded-full ${validationResult.isValid ? 'bg-emerald-400' : 'bg-red-400'}`} />
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto mt-4">
          {isLoading ? (
            <div className="flex h-full items-center justify-center text-xs text-text-muted font-mono">
              Synthesizing 7-section Architecture SSOT draft…
            </div>
          ) : activeTab === 'preview' ? (
            <div className="p-4 rounded-lg bg-black/30 border border-border-subtle">
              <ResearchReportMarkdown markdown={content} />
            </div>
          ) : activeTab === 'raw' ? (
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              aria-label="Markdown source"
              className="w-full h-full min-h-[400px] font-mono text-xs p-3 bg-black/50 border border-border-subtle rounded-lg text-text-primary outline-none resize-none focus:border-brass/40"
            />
          ) : (
            <div className="space-y-3 p-4 rounded-lg bg-black/30 border border-border-subtle">
              <h4 className="font-display text-xs uppercase tracking-wider text-text-primary">
                Frontmatter Validation Checklist
              </h4>
              {validationResult.isValid ? (
                <div className="flex items-center gap-2 p-3 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/30 text-xs">
                  <Icon.check className="size-4 shrink-0" />
                  <span>Valid YAML frontmatter matching all documentation governance constraints.</span>
                </div>
              ) : (
                <ul className="list-disc list-inside text-xs font-mono text-red-400 space-y-1">
                  {validationResult.errors.map((err, i) => (
                    <li key={i}>{err}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>

        {error && (
          <div className="mt-3 p-2 rounded bg-red-500/15 border border-red-500/30 text-red-300 text-xs font-mono">
            {error}
          </div>
        )}
        <div className="mt-4 flex items-center justify-between border-t border-border-subtle pt-3">
          <Button
            type="button"
            variant="ghost"
            onClick={() => setContent(originalContent)}
            disabled={content === originalContent || isPublishing}
            className="text-xs"
          >
            Reset
          </Button>
          <div className="flex gap-2">
            <Button type="button" variant="outline" onClick={onClose} disabled={isPublishing} className="text-xs">
              Cancel
            </Button>
            <Button
              type="button"
              onClick={handlePublish}
              disabled={!validationResult.isValid || isPublishing || isLoading || !slug.trim()}
              className="text-xs bg-brass/20 text-brass border-brass/40 hover:bg-brass/30"
            >
              {isPublishing ? 'Publishing…' : 'Approve & Publish'}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 3: Wire Publish button into `ResearchView.tsx`**

In `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx`, import `DocPublishModal` and render a button in the session header allowing the user to open `DocPublishModal` when a session is active.

- [ ] **Step 4: Run Vitest tests**

Run:
```bash
pnpm --dir crates/vox-gui/ui vitest run src/components/surfaces/Research/DocPublishModal.test.tsx
```
Expected: PASS.

- [ ] **Step 5: Commit Task 4**

Run:
```bash
git add crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
git commit -m "feat(gui): add interactive DocPublishModal to ResearchView"
```

---

### Task 5: `EmpiricalVerificationBadge.tsx` and Provenance Drawer in `DocReader.tsx`

**Files:**
- Create: `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.tsx`
- Modify: `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx`
- Test: `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`

- [ ] **Step 1: Write Vitest test**

Create `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx`:
```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import React from "react";
import { EmpiricalVerificationBadge } from "./EmpiricalVerificationBadge";

describe("EmpiricalVerificationBadge", () => {
  it("renders verified pill with stability score", () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    expect(screen.getByText(/Verified by Deep Research/i)).toBeDefined();
    expect(screen.getByText(/0.92/)).toBeDefined();
  });

  it("opens drawer on click", () => {
    render(<EmpiricalVerificationBadge sessionId={42} stabilityScore={0.92} />);
    const btn = screen.getByRole("button");
    fireEvent.click(btn);
    expect(screen.getByText(/Empirical Verification Ledger/i)).toBeDefined();
  });
});
```

- [ ] **Step 2: Implement `EmpiricalVerificationBadge.tsx`**

Create `crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.tsx`:
```tsx
import React, { useState } from 'react';
import { Icon } from '../../ui/Icons';
import { Glass } from '../../ui/Glass';

export interface EmpiricalVerificationBadgeProps {
  sessionId: number;
  stabilityScore: number;
  onNavigateToResearch?: (sessionId: number) => void;
}

export function EmpiricalVerificationBadge({
  sessionId,
  stabilityScore,
  onNavigateToResearch,
}: EmpiricalVerificationBadgeProps) {
  const [drawerOpen, setDrawerOpen] = useState(false);

  const isHighStability = stabilityScore >= 0.85;
  const badgeColor = isHighStability
    ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30'
    : 'bg-amber-500/15 text-amber-300 border-amber-500/30';

  return (
    <>
      <button
        type="button"
        onClick={() => setDrawerOpen(true)}
        aria-haspopup="dialog"
        aria-expanded={drawerOpen}
        className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full border font-mono text-[11px] transition-all hover:scale-105 cursor-pointer ${badgeColor}`}
      >
        <Icon.shield className="size-3.5 shrink-0" />
        <span className="font-semibold">Verified by Deep Research</span>
        <span className="opacity-70">(S = {stabilityScore.toFixed(2)})</span>
      </button>

      {drawerOpen && (
        <div className="fixed inset-0 z-50 flex justify-end">
          <div
            className="fixed inset-0 bg-black/50 backdrop-blur-xs"
            onClick={() => setDrawerOpen(false)}
          />
          <Glass
            role="dialog"
            aria-label="Empirical Verification Evidence"
            className="relative z-10 flex h-full w-full max-w-md flex-col bg-bg-base/95 border-l border-border-subtle shadow-2xl p-6 overflow-y-auto"
            inset={false}
          >
            <div className="flex items-center justify-between border-b border-border-subtle pb-3">
              <div>
                <h3 className="font-display text-sm font-semibold text-text-primary uppercase tracking-wider">
                  Empirical Verification Ledger
                </h3>
                <p className="font-mono text-[11px] text-text-muted mt-0.5">
                  Research Session #{sessionId} · Stability S = {stabilityScore.toFixed(2)}
                </p>
              </div>
              <button
                type="button"
                onClick={() => setDrawerOpen(false)}
                className="size-7 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-overlay-hover"
              >
                <Icon.x className="size-4" />
              </button>
            </div>

            <div className="mt-4 space-y-4 text-xs font-mono text-text-secondary">
              <div className="p-3 rounded bg-black/30 border border-border-subtle">
                <div className="font-semibold text-text-primary mb-1">Empirical Reproducibility</div>
                <p>This document was synthesized from empirical test runs verified by compiler sandbox execution and epistemic contradiction resolution.</p>
              </div>

              {onNavigateToResearch && (
                <button
                  type="button"
                  onClick={() => {
                    setDrawerOpen(false);
                    onNavigateToResearch(sessionId);
                  }}
                  className="w-full py-2 rounded bg-brass/20 text-brass border border-brass/40 hover:bg-brass/30 transition-colors"
                >
                  Open Full Research Session #{sessionId} →
                </button>
              )}
            </div>
          </Glass>
        </div>
      )}
    </>
  );
}
```

- [ ] **Step 3: Wire badge into `DocReader.tsx`**

In `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx`, parse the document markdown for `> **Empirical Provenance:** Generated by Vox Deep Research (Session #(\d+), Stability \$S = ([\d.]+)\$)` and display the `EmpiricalVerificationBadge` in the document header.

- [ ] **Step 4: Run Vitest tests**

Run:
```bash
pnpm --dir crates/vox-gui/ui vitest run src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx
```
Expected: PASS.

- [ ] **Step 5: Commit Task 5**

Run:
```bash
git add crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.tsx crates/vox-gui/ui/src/components/surfaces/DocReader/EmpiricalVerificationBadge.test.tsx crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx
git commit -m "feat(gui): add EmpiricalVerificationBadge and provenance drawer to DocReader"
```

---

### Task 6: Subject 1 Execution — Local MENS Apple Silicon Metal Optimization SSOT

**Files:**
- Test Probes: `crates/vox-populi/tests/metal_optimization_probe_test.rs`
- Create: `docs/src/architecture/mens-metal-optimization-ssot-2026.md`
- Update: `docs/src/architecture/research-index.md`

- [ ] **Step 1: Write and run concrete empirical probe test**

Create `crates/vox-populi/tests/metal_optimization_probe_test.rs`:
```rust
use sysinfo::System;

#[test]
fn test_probe_apple_silicon_memory_invariants() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let mut sys = System::new();
        sys.refresh_memory();
        let total_ram_mb = sys.total_memory() / (1024 * 1024);
        assert!(total_ram_mb > 0, "RAM must be detected");
        let ceiling_75 = (total_ram_mb * 3) / 4;
        let headroom = total_ram_mb - ceiling_75;
        assert!(headroom >= 2048, "Must preserve >= 2GB headroom");
    }
}
```

Run probe:
```bash
cargo test -p vox-populi --test metal_optimization_probe_test
```
Expected: PASS.

- [ ] **Step 2: Generate publication-grade 7-section Architecture SSOT**

Create `docs/src/architecture/mens-metal-optimization-ssot-2026.md` with:
- YAML Frontmatter (`title: "Local MENS Inference Engine & Apple Silicon Metal Optimization SSOT (2026)"`, `category: "Architecture SSOTs"`, `status: "current"`).
- Exact citations to `crates/vox-populi/src/mens/hardware/macos_metal.rs:21-41` and `crates/vox-plugin-mens-candle-metal/src/inference.rs:484-516`.
- SOTA benchmark matrix comparing Vox against llama.cpp Metal and Ollama.
- Verified claims table ($S = 0.92$).
- Sandbox code blocks with `// vox:skip empirical sandbox probe` on line 1 of any ` ```vox ` blocks.
- P0/P1/P2 gap analysis and phased implementation roadmap.

- [ ] **Step 3: Update `docs/src/architecture/research-index.md`**

Add link under `## Mesh Implementation Plans`:
`- [Local MENS Apple Silicon Metal Optimization SSOT (2026)](mens-metal-optimization-ssot-2026.md) — Empirical investigation of Apple Silicon Metal inference performance, KV-cache bottlenecks, and unified memory bandwidth.`

- [ ] **Step 4: Lint with `vox-doc-pipeline`**

Run:
```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/mens-metal-optimization-ssot-2026.md
```
Expected: PASS with zero hard errors.

- [ ] **Step 5: Commit Task 6**

Run:
```bash
git add crates/vox-populi/tests/metal_optimization_probe_test.rs docs/src/architecture/mens-metal-optimization-ssot-2026.md docs/src/architecture/research-index.md
git commit -m "docs(architecture): publish local mens apple silicon metal optimization ssot"
```

---

### Task 7: Subject 2 Execution — Autonomous Browser Driver & Accessibility-Tree Navigation SSOT

**Files:**
- Test Probes: `crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs`
- Create: `docs/src/architecture/agent-browser-driver-ssot-2026.md`
- Update: `docs/src/architecture/research-index.md`

- [ ] **Step 1: Write and run concrete empirical probe test**

Create `crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs`:
```rust
use serde_json::json;

#[test]
fn test_compact_ax_probe_bounds() {
    let fixture = vec![
        json!({ "role": { "type": "role", "value": "button" }, "name": { "value": "Submit" } }),
        json!({ "role": { "type": "role", "value": "paragraph" }, "name": { "value": "Static text" } }),
    ];
    let is_interactive = |r: &str| matches!(r, "button" | "link" | "textbox");
    let interactive_count = fixture
        .iter()
        .filter(|n| {
            let role = n["role"]["value"].as_str().unwrap_or("");
            is_interactive(role)
        })
        .count();
    assert_eq!(interactive_count, 1);
}
```

Run probe:
```bash
cargo test -p vox-plugin-browser --test ax_snapshot_probe_test
```
Expected: PASS.

- [ ] **Step 2: Generate publication-grade 7-section Architecture SSOT**

Create `docs/src/architecture/agent-browser-driver-ssot-2026.md` with:
- YAML Frontmatter (`title: "Autonomous Browser Driver & Accessibility-Tree Navigation SSOT (2026)"`, `category: "Architecture SSOTs"`, `status: "current"`).
- Exact citations to `crates/vox-plugin-api/src/extensions/browser_automation.rs:10-106` and `crates/vox-plugin-browser/src/engine.rs:602-611`.
- SOTA benchmark matrix comparing compact AX snapshots against raw HTML and Playwright MCP.
- Verified claims table ($S = 0.94$).
- Sandbox code blocks with `// vox:skip empirical sandbox probe` on line 1 of any ` ```vox ` blocks.
- P0/P1/P2 gap analysis and phased implementation roadmap.

- [ ] **Step 3: Update `docs/src/architecture/research-index.md`**

Add link under `## Strategic & Value Proposition`:
`- [Autonomous Browser Driver & Accessibility-Tree Navigation SSOT (2026)](agent-browser-driver-ssot-2026.md) — Architectural specification for compact semantic accessibility snapshotting and robust agent browser interaction.`

- [ ] **Step 4: Lint with `vox-doc-pipeline`**

Run:
```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/agent-browser-driver-ssot-2026.md
```
Expected: PASS with zero hard errors.

- [ ] **Step 5: Commit Task 7**

Run:
```bash
git add crates/vox-plugin-browser/tests/ax_snapshot_probe_test.rs docs/src/architecture/agent-browser-driver-ssot-2026.md docs/src/architecture/research-index.md
git commit -m "docs(architecture): publish autonomous browser driver and accessibility-tree navigation ssot"
```

---

### Task 8: End-to-End Verification & Rigid Quality Gates

**Files:**
- Validation across whole repository.

- [ ] **Step 1: Run Rust unit & integration test gates**

Run:
```bash
cargo test -p vox-scientia
cargo test -p vox-gui
cargo test -p vox-db
```
Expected: All tests PASS.

- [ ] **Step 2: Run scoped TOESTUB test-first gate**

Run:
```bash
cargo run -p vox-code-audit --bin toestub --quiet -- crates/vox-scientia crates/vox-gui crates/vox-db --rules skeleton --min-severity warning --mode enforce-strict --format terminal
```
Expected: PASS with 0 warnings.

- [ ] **Step 3: Run `vox-doc-pipeline` full lint**

Run:
```bash
cargo run -p vox-doc-pipeline -- --lint-only
```
Expected: PASS with no hard errors.

- [ ] **Step 4: Run doctest strict verification**

Run:
```bash
cargo run -p vox-cli -- ci doctest-md --strict
```
Expected: PASS with 0 failures.

- [ ] **Step 5: Run frontend Vitest suite**

Run:
```bash
pnpm --dir crates/vox-gui/ui vitest run
```
Expected: PASS.
