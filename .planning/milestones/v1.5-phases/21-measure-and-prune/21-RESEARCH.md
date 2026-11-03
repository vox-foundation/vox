---
phase: "21"
title: "Phase 21 Research: Measure & Prune"
status: "current"
category: "architecture"
description: "Technical research and architectural specification for deterministic docs mention extraction, reality audit sync, prune report generation, and public URL preservation."
---

# Phase 21 Technical Research: Measure & Prune (docs-freshness)

## Executive Summary
This research investigation covers all architectural and implementation aspects required to execute **Phase 21: Measure & Prune** in the `docs-freshness` workstream. Phase 21 deterministically measures the Markdown/MDX documentation files across `docs/src`, extracts concrete mentions of code artifacts, resolves them against repository ground truth, identifies dead/retired references, and computes a multi-faceted prune report to safely transition non-public/research docs into the `Internals` navigation group or archive with HTTP 301 redirects.

---

## 1. `vox-doc-pipeline` Claims Module & Extractor Design (MEASURE-01)

### 1.1 Crate Context & Dependencies
`crates/vox-doc-pipeline` is a Layer 3 crate (`layers.toml`) currently implementing frontmatter lints, include anchor resolution, and doctest checks (`pipeline/mod.rs`, `pipeline/lint.rs`, `pipeline/doctest.rs`).
To support deterministic mention extraction, the crate requires the following workspace-available dependencies:
- `pulldown-cmark = { workspace = true }` (v0.13) for streaming Markdown AST parsing with byte offsets.
- `regex = { workspace = true }` (v1) for compiled mention token detectors.
- `serde = { workspace = true, features = ["derive"] }` & `serde_yaml = { workspace = true }` for schema serialization and contract loading.
- `walkdir = { workspace = true }` for scanning markdown trees.

### 1.2 Extractor Architecture (`crates/vox-doc-pipeline/src/claims/`)
```
crates/vox-doc-pipeline/src/claims/
├── mod.rs          # Public facade (scan_docs, scan_file, evaluate)
├── types.rs        # Core data structures (MentionKind, MentionRole, ExtractedMention, ResolvedMention)
├── extractor.rs    # pulldown-cmark AST event consumer + regex pattern matches
├── resolver.rs     # Ground-truth loader and domain resolution logic
├── findings.rs     # Reality audit finding generator, de-duplication & metrics updater
├── eval.rs         # False-positive measurement against hand-labelled ground truth sample
└── prune.rs        # Prune report engine (link graph, git mtime, clustering, dispositions)
```

### 1.3 Mention Classification & Roles
- **Mention Kinds**:
  - `FilePath`: Relative repo paths (`crates/vox-cli/src/...`, `docs/src/...`, `.planning/...`, `contracts/...`).
  - `CrateName`: First-party workspace crates (`vox-cli`, `vox-compiler`, `vox-runtime`, etc.).
  - `RustSymbol`: Path-qualified symbols (`vox_actor_runtime::llm`, `vox_doc_pipeline::claims`).
  - `CliCommand`: Subcommands and flags (`vox audit docs`, `vox check`, `--format`, `--prune-report`).
  - `EnvVar`: Environment variables (`VOX_*`, `RUST_LOG`, etc.).
  - `ConfigKey`: Configuration keys from `contracts/config/` schemas.
- **Mention Roles**:
  - `Prescribes`: Normative requirements, configuration examples, imports, command-line snippets. Missing/retired items here are flagged as actionable doc deficits.
  - `Mentions`: Historical notes, migration guides, illustrative prose, code blocks marked with `// vox:skip` or fenced with ````text`. Missing/retired items here are informational or marked as valid deprecations.

---

## 2. Deterministic Ground Truth Resolvers (MEASURE-02)

Each extracted mention is evaluated against ground truth sources:
1. **File Paths**: Resolved against repository filesystem root using normalized paths.
2. **Crate Names**: Resolved against workspace `Cargo.toml` `[workspace.members]`.
3. **CLI Commands & Flags**:
   - `contracts/cli/command-registry.yaml`
   - Clap command hierarchy parsed from `vox-cli`.
4. **Environment Variables**:
   - `contracts/config/env-vars.v1.yaml`
5. **Config Keys**:
   - `contracts/config/registry.v1.yaml`
6. **Retired & Renamed Symbols**:
   - `contracts/documentation/retired-symbols.v1.yaml` (provides mappings for retired CLI subcommands, deprecated decorators, and relocated crates).
7. **Rust Symbols**:
   - Resolved against AST symbol tables or code graph index when available; fallback to ripgrep symbol definition patterns (`pub (fn|struct|enum|trait|type) <Name>`).

**Resolution Statuses**:
- `Resolved`: Target exists and is active.
- `Missing`: Target cannot be found in filesystem or registries.
- `Retired`: Target matches an entry in `retired-symbols.v1.yaml` or has explicit deprecation tombstone.
- `Ambiguous`: Target matches multiple unrelated symbols or is insufficiently qualified.

---

## 3. Docs Reality Audit Schema Bump & Deduplication (MEASURE-04)

`contracts/reports/docs-reality-audit/findings.v1.schema.json` and `metrics.v1.schema.json`:
- **Finding Fields Added**:
  - `source`: `"mention_extractor"` | `"manual_audit"` | `"link_crawler"`
  - `fingerprint`: Deterministic SHA-256 hash of `(source_file, target_symbol, mention_kind, line_number)` to enable idempotent de-duplication across scans.
  - `detector`: Specific sub-detector (`"dead_file_path"`, `"retired_cli_command"`, `"missing_env_var"`, etc.).
- **Metrics Health Invariant**:
  - Fields added to `metrics.v1.json`:
    - `corpus_pages_scanned`: integer
    - `total_mentions_extracted`: integer
    - `resolved_mentions_count`: integer
    - `dead_mentions_count`: integer
  - When `corpus_pages_scanned > 0` and `findings_open == 0`, report status is **Healthy** (distinct from unpopulated baseline).

---

## 4. CLI Design: `vox audit docs` (MEASURE-03)

### 4.1 Subcommand Definition in `crates/vox-cli/src/commands/audit.rs`
```rust
pub enum AuditSubcommand {
    Arch,
    Code,
    Retirement,
    Core,
    Effort(EffortArgs),
    EffortRoute(EffortRouteArgs),
    ResearchParity(ResearchParityArgs),
    /// Deterministic Documentation Reality Audit & Mention Extractor
    Docs(DocsAuditArgs),
}
```

### 4.2 CLI Argument Surface
```rust
#[derive(clap::Args, Debug, Clone)]
pub struct DocsAuditArgs {
    /// Format of findings output: text (human summary), json, or jsonl.
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Generate prune report for the documentation corpus.
    #[arg(long)]
    pub prune_report: bool,

    /// Write machine-generated findings to contracts/reports/docs-reality-audit/findings.v1.json and update metrics.
    #[arg(long)]
    pub write_findings: bool,

    /// Hand-labelled evaluation sample path for false-positive rate calculation.
    #[arg(long)]
    pub eval_sample: Option<PathBuf>,

    /// Filter scan to specific relative doc paths.
    #[arg(long)]
    pub paths: Option<Vec<PathBuf>>,

    /// Output destination file for prune report (markdown or json).
    #[arg(long)]
    pub prune_out: Option<PathBuf>,
}
```

### 4.3 Umbrella Contract & Non-Blocking CI
- Emits JSONL records adhering to `schema_version: 1` envelope.
- Operates report-only (exit code 0) in Phase 21 to gather data before gating in Phase 22.

---

## 5. Prune Report Engine (MEASURE-05)

The prune engine collects six dimensions of evidence across documentation pages:
1. **Dead Mentions**: Count of resolved mentions where `status ∈ {Missing, Retired}` and `role == Prescribes`.
2. **Inbound Links**:
   - Inverted link graph `target_page -> HashSet<source_page>`.
   - Pages with 0 inbound links are marked as orphaned candidates.
3. **Last Substantive Edit**:
   - Single batched Git command:
     `git log --pretty=format:"COMMIT:%ct%x09%s" --name-only --no-renames -- docs/src`
   - Filters out mechanical commits (`chore(docs): regenerate`, `ssot-autoregen`, `style: fmt`, `ci: sync`).
4. **Status & Section Categorization**:
   - Frontmatter `status` (`current`, `research`, `roadmap`, `deprecated`, `legacy`) and `category`.
5. **Duplicate-Topic Clustering**:
   - Jaccard similarity ($\ge 0.6$) over title and URL slug stems.
6. **Disposition Recommendations**:
   - `archive`: 0 inbound links AND (dead mentions $\ge 3$ OR age > 180 days OR status is `legacy`/`deprecated`).
   - `move-to-Internals`: Status is `research` or `roadmap` OR category is an internal architecture/ADR note.
   - `merge`: Member of a duplicate-topic cluster where a more recent or canonical SSOT exists.
   - `keep`: Current status, active inbound links, low/zero dead mentions, part of core user-facing sections.

---

## 6. Public URL Integrity & Redirect Strategy (MEASURE-06)

Confirmed User Policy:
- Research and roadmap pages leave the public sidebar by default and move to the `Internals` group.
- Approved moves, merges, and archives are applied without causing 404 errors.

### Mechanism
- Pages staying in place have frontmatter `status: research` or `status: roadmap` set, placing them in `Internals` via Starlight sidebar configuration.
- Files relocated to `docs/src/archive/` or merged receive 301 entries in `docs-astro/public/_redirects`:
  ```
  /old/path/  /new/path/  301
  /old/path.html  /new/path/  301
  ```
- Link integrity is verified by `pnpm --dir docs-astro test`.

---

## 7. Recommended Wave Breakdown

- **Wave 1: Extractor & Resolvers (MEASURE-01, MEASURE-02)**
- **Wave 2: CLI Command, Evaluation & Reality Audit Bump (MEASURE-03, MEASURE-04)**
- **Wave 3: Prune Report Engine (MEASURE-05)**
- **Wave 4: Apply Prune Dispositions & Redirects (MEASURE-06)**
