//! `vox ci` — repository guard checks (SSOT, manifests, feature matrix) without shell/Python.

mod command_compliance;
mod line_endings;
pub mod build_timings;
mod check_links;
mod release_build;

use anyhow::{Context, Result, anyhow};
use clap::{Subcommand, ValueEnum};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use std::time::Instant;

/// Resolve repository root: `VOX_REPO_ROOT`, else walk up from CWD for `AGENTS.md` + `Cargo.toml`.
pub fn repo_root() -> PathBuf {
    vox_repository::resolve_repo_root_for_ci()
}

fn cargo_bin() -> PathBuf {
    if let Ok(h) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let win = PathBuf::from(&h).join(".cargo/bin/cargo.exe");
        if win.is_file() {
            return win;
        }
    }
    PathBuf::from("cargo")
}

/// `nvcc --version` using `CUDA_PATH`/`CUDA_HOME` when set (agent shells often lack full `PATH`).
fn nvcc_version_command() -> Command {
    let try_cuda_bin = |base: &str| -> Option<PathBuf> {
        let root = PathBuf::from(base);
        let exe = if cfg!(windows) {
            root.join("bin").join("nvcc.exe")
        } else {
            root.join("bin").join("nvcc")
        };
        exe.is_file().then_some(exe)
    };
    if let Ok(p) = std::env::var("CUDA_PATH").or_else(|_| std::env::var("CUDA_HOME")) {
        if let Some(exe) = try_cuda_bin(&p) {
            return Command::new(exe);
        }
    }
    Command::new("nvcc")
}

fn nvcc_available() -> bool {
    nvcc_version_command()
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Command variations for Continuous Integration guards and internal codebase hygiene.
#[derive(Subcommand)]
pub enum CiCmd {
    /// `cargo metadata --locked --format-version 1 --no-deps` (workspace manifest resolves).
    Manifest,
    /// Documentation SSOT guard (required pages, doc-inventory schema, orphan inventory crate list).
    #[command(name = "check-docs-ssot")]
    CheckDocsSsot,
    /// Codex / Arca SSOT file and OpenAPI substring guard.
    #[command(name = "check-codex-ssot")]
    CheckCodexSsot,
    /// `cargo check -p vox-cli` for each supported feature set.
    #[command(name = "feature-matrix")]
    FeatureMatrix,
    /// Fail if `vox-cli` sources import `vox_dei::`.
    #[command(name = "no-vox-dei-import")]
    NoVoxDeiImport,
    /// Run `vox-doc-pipeline --check` to verify SUMMARY.md matches docs/src
    CheckSummaryDrift,
    /// Build all documentation artifacts
    BuildDocs,
    /// Doc inventory (schema v3): generate or verify.
    DocInventory {
        /// Subcommand execution variant.
        #[command(subcommand)]
        cmd: DocInventoryCmd,
    },
    /// Fail if workflow YAML references `scripts/` paths not in the allowlist file.
    #[command(name = "workflow-scripts")]
    WorkflowScripts {
        /// Allowlist path (one script path per line, repo-relative).
        #[arg(long, default_value = "docs/agents/workflow-script-allowlist.txt")]
        allowlist: PathBuf,
    },
    /// Fail if changed LF-policy text files contain CRLF / CR (`*.ps1` exempt). Forward-only unless `--all`.
    #[command(name = "line-endings")]
    LineEndings {
        /// Audit all tracked policy files (not just the diff).
        #[arg(long)]
        all: bool,
        /// Git ref for diff base (overrides `VOX_LINE_ENDINGS_BASE`; head defaults to `HEAD`).
        #[arg(long)]
        base: Option<String>,
    },
    /// Run Mens gate steps from `scripts/mens/gates.yaml`.
    #[command(name = "mens-gate")]
    PopuliGate {
        /// Profile name: `m1m4` or `training`.
        #[arg(long, default_value = "m1m4")]
        profile: String,
    },
    /// Scoped TOESTUB: `cargo run -p vox-toestub --bin toestub -- <ROOT>`.
    #[command(name = "toestub-scoped")]
    ToestubScoped {
        /// Root path for structural scope testing.
        #[arg(default_value = "crates/vox-repository")]
        root: PathBuf,
    },
    /// Optional CUDA feature compile checks when `nvcc` is on PATH (or skip via env).
    #[command(name = "cuda-features")]
    CudaFeatures,
    /// Wall-clock timings for key `cargo check` lanes (default CLI, GPU+stub, optional CUDA).
    #[command(name = "build-timings")]
    BuildTimings {
        /// Print one JSON object per lane (machine-readable).
        #[arg(long)]
        json: bool,
        /// Also time isolated `cargo check -p <crate>` lanes (compiler vs data vs Oratio vs Mens train).
        #[arg(long)]
        crates: bool,
        /// Detailed per-crate telemetry persisted to Arca (V34+).
        #[arg(long)]
        deep: bool,
        /// Persist results to VoxDB (default: true if deep).
        #[arg(long)]
        persist: Option<bool>,
        /// Name for this build run (deep only).
        #[arg(long)]
        name: Option<String>,
        /// Profile: `dev` or `release` (deep only).
        #[arg(long, default_value = "dev")]
        profile: String,
    },
    /// Compare grammar taxonomy fingerprint (`generate_system_prompt` SHA-256) to `mens/data/grammar_fingerprint.txt`; update file on drift.
    #[command(name = "grammar-drift")]
    GrammarDrift {
        /// Emit machine-readable `drift=true|false` for CI (e.g. append to `GITHUB_OUTPUT`).
        #[arg(long, value_enum)]
        emit: Option<GrammarDriftEmit>,
    },
    /// Repository hygiene guards (`TypeVar(0)` in codegen crates only, filtered `opencode` refs, stray root files) — GitLab parity.
    #[command(name = "repo-guards")]
    RepoGuards,
    /// Command registry parity: `contracts/cli/command-registry.yaml` vs `ref-cli`, reachability, compilerd, dei, MCP tools, script duals.
    #[command(name = "command-compliance")]
    CommandCompliance,
    /// Fail if internal Markdown links are broken in `docs/src` or root-level guides.
    #[command(name = "check-links")]
    CheckLinks,
    /// Build and package release artifacts for a target triple (binary + checksum manifest).
    #[command(name = "release-build")]
    ReleaseBuild {
        /// Rust target triple (for example `x86_64-unknown-linux-gnu`).
        #[arg(long)]
        target: String,
        /// Version tag used in artifact names (defaults to package version).
        #[arg(long)]
        version: Option<String>,
        /// Output directory for packaged artifacts.
        #[arg(long, default_value = "dist")]
        out_dir: PathBuf,
    },
}

/// Output channel for [`CiCmd::GrammarDrift`].
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GrammarDriftEmit {
    /// One line: `drift=true` or `drift=false` (GitHub Actions / shell).
    Github,
    /// Writes `drift.env` in the repo root with `drift=true|false` (GitLab-style artifact).
    Gitlab,
}

/// Subcommands for the doc inventory schema verifier.
#[derive(Subcommand)]
pub enum DocInventoryCmd {
    /// Write `docs/agents/doc-inventory.json` (or `--output`).
    Generate {
        /// Optional path to override the default JSON inventory location.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Fail if committed inventory differs from a fresh generation (ignores `generated_at`).
    Verify,
}

const DOCS_SSOT_FILES: &[&str] = &[

    "docs/src/architecture/forward-migration-charter.md",
    "docs/src/architecture/codex-arca-compatibility-boundaries.md",
    "docs/src/architecture/codex-arca-import-policy.md",
    "docs/src/architecture/codex-turso-allowlist.md",
    "docs/src/architecture/cli-scope-policy.md",
    "docs/src/architecture/compatibility-deprecation-windows.md",
    "docs/src/architecture/doc-to-code-acceptance-checklist.md",
    "docs/src/architecture/cli-reachability-ssot.md",
    "docs/src/architecture/cli-design-rules-ssot.md",
    "docs/src/architecture/trim-build-defer-policy.md",
    "docs/src/architecture/vox-cli-build-feature-inventory.md",
    "docs/src/architecture/crate-build-lanes-migration.md",
    "docs/src/architecture/crate-topology-buckets.md",
    "docs/src/architecture/deployment-compose-ssot.md",
    "docs/src/architecture/mens-training-ssot.md",
    "docs/src/how-to-train-mens-4080.md",
    "docs/src/architecture/phase0-migration-signoff.md",
    "docs/src/architecture/migration-script-dashboard.md",
    "docs/src/architecture/vox-automation-primitives.md",
    "docs/src/architecture/typescript-migration-boundary.md",
    "docs/src/architecture/vox-web-stack-ssot.md",
    "docs/src/architecture/external-scripts-boundary-archive.md",
    "docs/src/ci/runner-contract.md",
    "docs/src/ci/command-surface-duals.md",
    "docs/src/ci/command-compliance-ssot.md",
    "docs/src/ci/doc-inventory-ssot.md",
    "docs/src/ci/crate-hardening-matrix.md",
    "docs/src/ci/github-hosted-exceptions.md",
    "docs/src/ci/workflow-enumeration.md",
    "docs/src/ci/binary-release-contract.md",
];

const CODEX_SSOT_FILES: &[&str] = &[
    "contracts/codex-api.openapi.yaml",
    "docs/src/adr/004-codex-arca-turso-ssot.md",
    "docs/src/architecture/codex-vnext-schema.md",
    "docs/src/architecture/codex-baas.md",
    "docs/src/architecture/orphan-surface-inventory.md",
    "docs/src/architecture/codex-legacy-migration.md",
    "docs/src/architecture/forward-migration-charter.md",
    "docs/src/architecture/codex-arca-import-policy.md",
    "docs/src/architecture/codex-arca-compatibility-boundaries.md",
    "infra/coolify/docker-compose.yml",
    "scripts/check_codex_ssot.sh",
    "scripts/check_codex_ssot.ps1",
];

const OPENAPI_SUBSTRINGS: &[&str] = &[
    "openapi:",
    "/api/codex/research-session",
    "/api/codex/conversations/{conv_id}/versions",
    "/api/codex/conversation-edges",
    "/api/codex/topics/{topic_id}/evolution-events",
];

const MANIFEST_SNIPPETS: &[&str] = &[
    "BASELINE_VERSION",
    "SCHEMA_FRAGMENTS",
    "schema_baseline_digest_hex",
    "pub const BASELINE_VERSION: i64 = 1",
];

const FEATURE_SETS: &[&str] = &[
    "",
    "codex",
    "stub-check",
    "codex,stub-check",
    "live",
    "mens-dei",
    "mens-oratio",
    "dashboard",
    "ars",
    "extras-ludus",
    "gpu,mens-qlora,stub-check",
    "island",
    "island,mens-base",
    "script-execution",
    "script-execution,stub-check",
    "mens",
    "script-execution,mens",
    "workflow-runtime",
];

/// Run `vox ci` subcommand.
pub async fn run(cmd: CiCmd) -> Result<()> {
    let root = repo_root();
    match cmd {
        CiCmd::Manifest => run_manifest(&root),
        CiCmd::CheckDocsSsot => check_docs_ssot(&root),
        CiCmd::CheckCodexSsot => check_codex_ssot(&root),
        CiCmd::FeatureMatrix => run_feature_matrix(&root),
        CiCmd::NoVoxDeiImport => check_no_vox_dei(&root),
        CiCmd::CheckSummaryDrift => {
            let cargo = cargo_bin();
            let st = Command::new(&cargo)
                .current_dir(&root)
                .args(["run", "-p", "vox-doc-pipeline", "--", "--check"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("SUMMARY.md is out of sync with docs/src. Run 'cargo run -p vox-doc-pipeline' to fix."));
            }
            println!("SUMMARY.md is up to date.");
            Ok(())
        }
        CiCmd::BuildDocs => {
            let cargo = cargo_bin();
            // 1. Generate SUMMARY.md
            let st = Command::new(&cargo)
                .current_dir(&root)
                .args(["run", "-p", "vox-doc-pipeline"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("failed to generate SUMMARY.md"));
            }
            // 2. Run mdbook build docs (assuming mdbook is on PATH)
            let st = Command::new("mdbook")
                .current_dir(&root)
                .args(["build", "docs"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("mdbook build docs failed"));
            }
            println!("Documentation built successfully.");
            Ok(())
        }
        CiCmd::DocInventory { cmd: sub } => match sub {
            DocInventoryCmd::Generate { output } => {
                let out =
                    output.unwrap_or_else(|| root.join(vox_doc_inventory::DEFAULT_INVENTORY_PATH));
                vox_doc_inventory::generate(&root, &out)?;
                println!("Wrote {}", out.display());
                Ok(())
            }
            DocInventoryCmd::Verify => {
                let committed = root.join(vox_doc_inventory::DEFAULT_INVENTORY_PATH);
                vox_doc_inventory::verify_fresh(&root, &committed)?;
                println!("doc-inventory.json matches generator output (excluding generated_at)");
                Ok(())
            }
        },
        CiCmd::WorkflowScripts { allowlist } => check_workflow_scripts(&root, &allowlist),
        CiCmd::LineEndings { all, base } => line_endings::run(&root, all, base),
        CiCmd::PopuliGate { profile } => run_mens_gate(&root, &profile),
        CiCmd::ToestubScoped { root: scan_root } => run_toestub_scoped(&root, &scan_root),
        CiCmd::CudaFeatures => run_cuda_features(),
        CiCmd::BuildTimings { json, crates, deep, persist, name, profile } => {
            if deep {
                build_timings::bench_build_run(persist.unwrap_or(true), name, Some(profile)).await?;
                Ok(())
            } else {
                run_build_timings(&root, json, crates)
            }
        }
        CiCmd::GrammarDrift { emit } => run_grammar_drift(&root, emit),
        CiCmd::RepoGuards => run_repo_guards(&root),
        CiCmd::CommandCompliance => command_compliance::run(&root),
        CiCmd::CheckLinks => check_links::run(&root),
        CiCmd::ReleaseBuild {
            target,
            version,
            out_dir,
        } => release_build::run(&root, &target, version.as_deref(), &out_dir),
    }
}

fn sha256_hex_lower(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn run_grammar_drift(root: &Path, emit: Option<GrammarDriftEmit>) -> Result<()> {
    let prompt = crate::training::generate_system_prompt();
    let fingerprint = sha256_hex_lower(prompt.as_bytes());
    let path = root.join("mens/data/grammar_fingerprint.txt");
    let stored = if path.is_file() {
        fs::read_to_string(&path)
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };
    let drift = fingerprint != stored;
    if drift {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, format!("{fingerprint}\n"))?;
        eprintln!(
            "Grammar drift detected (fingerprint changed). Updated {}.",
            path.display()
        );
    } else {
        eprintln!("No grammar drift detected.");
    }
    let drift_line = if drift { "drift=true" } else { "drift=false" };
    match emit {
        Some(GrammarDriftEmit::Github) => println!("{drift_line}"),
        Some(GrammarDriftEmit::Gitlab) => {
            let p = root.join("drift.env");
            fs::write(&p, format!("{drift_line}\n"))?;
            eprintln!("Wrote {}", p.display());
        }
        None => {}
    }
    Ok(())
}

fn run_repo_guards(root: &Path) -> Result<()> {
    guard_no_typevar_zero(root)?;
    guard_no_opencode_refs(root)?;
    guard_no_stray_root_files(root)?;
    println!("repo-guards OK");
    Ok(())
}

fn guard_no_typevar_zero(root: &Path) -> Result<()> {
    // The typechecker legitimately references `TypeVar(0)`; guard codegen emitters only.
    let re = regex::Regex::new(r"TypeVar\(0\)")?;
    for rel in ["crates/vox-codegen-rust/src", "crates/vox-codegen-ts/src"] {
        let dir = root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        visit_rs_files(&dir, &mut |p: &Path| {
            let text = fs::read_to_string(p)?;
            if re.is_match(&text) {
                return Err(anyhow!(
                    "TypeVar(0) must not appear in codegen sources — use fresh inference vars ({})",
                    p.display()
                ));
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn guard_no_opencode_refs(root: &Path) -> Result<()> {
    let crates = root.join("crates");
    let needle = regex::Regex::new(r"opencode")?;
    visit_rs_files(&crates, &mut |p: &Path| {
        let text = fs::read_to_string(p)?;
        if !needle.is_match(&text) {
            return Ok(());
        }
        for (idx, line) in text.lines().enumerate() {
            if !line.contains("opencode") {
                continue;
            }
            if line.contains("tests_agent_session")
                || line.contains("// formerly")
                || line.contains("how-to-opencode")
            {
                continue;
            }
            return Err(anyhow!(
                "disallowed opencode reference in {}:{} — {}",
                p.display(),
                idx + 1,
                line.trim()
            ));
        }
        Ok(())
    })?;
    Ok(())
}

fn root_file_is_stray(name: &str) -> bool {
    if name.ends_with(".txt") || name.ends_with(".log") || name.ends_with(".err") {
        return true;
    }
    if (name.starts_with("patch_") || name.starts_with("fix_")) && name.ends_with(".py") {
        return true;
    }
    if name.ends_with(".vox")
        && (name.starts_with("temp") || name.starts_with("test_") || name.starts_with("debug_"))
    {
        return true;
    }
    false
}

fn guard_no_stray_root_files(root: &Path) -> Result<()> {
    let mut offenders = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let entry = entry?;
        let name_s = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_file() {
            continue;
        }
        if root_file_is_stray(&name_s) {
            offenders.push(name_s);
        }
    }
    if !offenders.is_empty() {
        return Err(anyhow!(
            "stray files at repository root: {}",
            offenders.join(", ")
        ));
    }
    Ok(())
}

fn run_manifest(root: &Path) -> Result<()> {
    let status = Command::new(cargo_bin())
        .current_dir(root)
        .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
        .stdout(Stdio::null())
        .status()
        .context("spawn cargo metadata")?;
    if !status.success() {
        return Err(anyhow!("cargo metadata --locked failed"));
    }
    println!("OK: workspace manifest resolves (cargo metadata --locked --no-deps)");
    Ok(())
}

fn check_docs_ssot(root: &Path) -> Result<()> {
    for rel in DOCS_SSOT_FILES {
        let p = root.join(rel);
        if !p.is_file() {
            return Err(anyhow!("missing: {}", p.display()));
        }
    }
    let doc_inv = root.join("docs/agents/doc-inventory.json");
    if !doc_inv.is_file() {
        return Err(anyhow!(
            "missing: {} (run: vox ci doc-inventory generate)",
            doc_inv.display()
        ));
    }
    let raw = fs::read_to_string(&doc_inv)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    let sv = v
        .get("schema_version")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    if sv < 3 {
        return Err(anyhow!("doc-inventory.json: expected schema_version >= 3"));
    }

    let inv = root.join("docs/src/architecture/orphan-surface-inventory.md");
    let inv_text = fs::read_to_string(&inv)?;
    if !inv_text.contains("workspace-crates-start") {
        return Err(anyhow!(
            "orphan inventory: missing workspace-crates-start marker"
        ));
    }
    if !inv_text.contains("workspace-crates-end") {
        return Err(anyhow!(
            "orphan inventory: missing workspace-crates-end marker"
        ));
    }

    let listed = parse_workspace_crate_block(&inv_text);
    let crates_dir = root.join("crates");
    for entry in
        fs::read_dir(&crates_dir).with_context(|| format!("read {}", crates_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let toml = entry.path().join("Cargo.toml");
        if !toml.is_file() {
            continue;
        }
        let name = read_package_name(&toml)?;
        if !listed.contains(&name) {
            return Err(anyhow!(
                "orphan inventory workspace crate list missing: {name} (from {})",
                toml.display()
            ));
        }
    }

    check_stale_doc_and_workflow_refs(root)?;

    println!("Docs SSOT guard OK");
    Ok(())
}

/// Fail if docs or GitHub workflows reference retired Python inventory paths or shell gates.
fn check_stale_doc_and_workflow_refs(root: &Path) -> Result<()> {
    const WORKFLOW_BANNED: &[&str] = &["verify_doc_inventory_fresh.py", "populi_release_gate.sh"];
    const DOC_BANNED: &[&str] = &["verify_doc_inventory_fresh.py", "populi_release_gate.sh"];

    let wf_dir = root.join(".github/workflows");
    if wf_dir.is_dir() {
        for entry in fs::read_dir(&wf_dir).with_context(|| format!("read {}", wf_dir.display()))? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("yml")
                && p.extension().and_then(|x| x.to_str()) != Some("yaml")
            {
                continue;
            }
            let text = fs::read_to_string(&p)?;
            for b in WORKFLOW_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: stale or retired reference {:?} (use `vox ci` guards; see docs/src/ci/doc-inventory-ssot.md)",
                        p.display(),
                        b
                    ));
                }
            }
        }
    }

    let docs_src = root.join("docs/src");
    if docs_src.is_dir() {
        let mut files = Vec::new();
        collect_text_files_under(&docs_src, &mut files)?;
        for p in files {
            let ext = p.extension().and_then(|x| x.to_str());
            if ext != Some("md") && ext != Some("yml") && ext != Some("yaml") {
                continue;
            }
            let text = fs::read_to_string(&p)?;
            for b in DOC_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: stale reference {:?} — removed from tree; update docs",
                        p.display(),
                        b
                    ));
                }
            }
        }
    }

    println!("stale doc/workflow ref scan OK");
    Ok(())
}

fn collect_text_files_under(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        let t = entry.file_type()?;
        if t.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" || name == "book" {
                continue;
            }
            collect_text_files_under(&p, out)?;
        } else if t.is_file() {
            out.push(p);
        }
    }
    Ok(())
}

static CRATE_LINE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-z0-9_-]+$").expect("CRATE_LINE_RE"));

fn parse_workspace_crate_block(md: &str) -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let mut out = HashSet::new();
    let mut in_block = false;
    for line in md.lines() {
        let t = line.trim_end();
        if t.contains("workspace-crates-start") {
            in_block = true;
            continue;
        }
        if t.contains("workspace-crates-end") {
            in_block = false;
            continue;
        }
        if in_block {
            let s = t.trim();
            if CRATE_LINE_RE.is_match(s) {
                out.insert(s.to_string());
            }
        }
    }
    out
}

fn read_package_name(toml_path: &Path) -> Result<String> {
    let text = fs::read_to_string(toml_path)?;
    let re = regex::Regex::new(r#"^name\s*=\s*"([^"]+)""#)?;
    for line in text.lines() {
        let t = line.trim();
        if let Some(c) = re.captures(t) {
            return Ok(c.get(1).unwrap().as_str().to_string());
        }
    }
    Err(anyhow!(
        "could not read package name from {}",
        toml_path.display()
    ))
}

fn check_codex_ssot(root: &Path) -> Result<()> {
    for rel in CODEX_SSOT_FILES {
        let p = root.join(rel);
        if !p.is_file() {
            return Err(anyhow!("missing: {}", p.display()));
        }
    }
    let m = root.join("crates/vox-pm/src/schema/manifest.rs");
    let manifest = fs::read_to_string(&m)?;
    for needle in MANIFEST_SNIPPETS {
        if !manifest.contains(needle) {
            return Err(anyhow!("manifest.rs must contain or match: {needle}"));
        }
    }
    let o = root.join("contracts/codex-api.openapi.yaml");
    let o_text = fs::read_to_string(&o)?;
    for needle in OPENAPI_SUBSTRINGS {
        if !o_text.contains(needle) {
            return Err(anyhow!("openapi guard failed: missing {needle}"));
        }
    }
    println!("Codex SSOT doc guard OK");
    Ok(())
}

fn run_feature_matrix(root: &Path) -> Result<()> {
    let cargo = cargo_bin();
    for f in FEATURE_SETS {
        if f.is_empty() {
            eprintln!("==> cargo check -p vox-cli (default features)");
            let st = Command::new(&cargo)
                .current_dir(root)
                .args(["check", "-p", "vox-cli"])
                .status()?;
            if !st.success() {
                return Err(anyhow!("cargo check -p vox-cli failed"));
            }
        } else {
            eprintln!("==> cargo check -p vox-cli --features {f}");
            let st = Command::new(&cargo)
                .current_dir(root)
                .args(["check", "-p", "vox-cli", "--features", f])
                .status()?;
            if !st.success() {
                return Err(anyhow!("cargo check -p vox-cli --features {f} failed"));
            }
        }
    }
    println!("vox-cli feature matrix OK");
    Ok(())
}

fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path) -> Result<()>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        let t = entry.file_type()?;
        if t.is_dir() {
            visit_rs_files(&p, f)?;
        } else if t.is_file() && p.extension().and_then(|x| x.to_str()) == Some("rs") {
            f(&p)?;
        }
    }
    Ok(())
}

fn check_no_vox_dei(root: &Path) -> Result<()> {
    let src = root.join("crates/vox-cli/src");
    let re = regex::Regex::new(r"\bvox_dei::")?;
    visit_rs_files(&src, &mut |p: &Path| {
        let text = fs::read_to_string(p)?;
        if re.is_match(&text) {
            return Err(anyhow!(
                "vox-cli must not reference vox_dei:: (crate is workspace-excluded). Offender: {}",
                p.display()
            ));
        }
        Ok(())
    })?;
    println!("vox-cli no-vox_dei guard OK");
    Ok(())
}

fn check_workflow_scripts(root: &Path, allowlist_path: &Path) -> Result<()> {
    let allow_path = root.join(allowlist_path);
    let allowed: std::collections::HashSet<String> = if allow_path.is_file() {
        fs::read_to_string(&allow_path)?
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    } else {
        return Err(anyhow!("missing allowlist: {}", allow_path.display()));
    };

    let wf_dir = root.join(".github/workflows");
    let re = regex::Regex::new(r"scripts/[A-Za-z0-9_./-]+")?;
    let mut violations = Vec::new();
    for entry in fs::read_dir(&wf_dir).with_context(|| format!("read {}", wf_dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        if p.extension().and_then(|x| x.to_str()) != Some("yml")
            && p.extension().and_then(|x| x.to_str()) != Some("yaml")
        {
            continue;
        }
        let text = fs::read_to_string(&p)?;
        for cap in re.find_iter(&text) {
            let path = cap.as_str().to_string();
            if !allowed.contains(&path) {
                violations.push(format!("{}: {}", p.display(), path));
            }
        }
    }
    if !violations.is_empty() {
        return Err(anyhow!(
            "workflow references scripts/ not in allowlist:\n{}",
            violations.join("\n")
        ));
    }
    println!("workflow-scripts allowlist OK");
    Ok(())
}

fn run_mens_gate(root: &Path, profile: &str) -> Result<()> {
    let manifest_path = root.join("scripts/mens/gates.yaml");
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw)?;
    let profiles = doc
        .get("profiles")
        .and_then(|p| p.as_mapping())
        .ok_or_else(|| anyhow!("gates.yaml: missing profiles"))?;
    let prof = profiles
        .get(serde_yaml::Value::String(profile.to_string()))
        .ok_or_else(|| anyhow!("unknown profile: {profile}"))?;
    let steps = prof
        .get("steps")
        .and_then(|s| s.as_sequence())
        .ok_or_else(|| anyhow!("profile {profile}: missing steps"))?;

    let cargo = cargo_bin();
    for step in steps {
        let cmd = step
            .get("command")
            .and_then(|c| c.as_str())
            .unwrap_or("cargo");
        let args = step
            .get("args")
            .and_then(|a| a.as_sequence())
            .ok_or_else(|| anyhow!("step missing args"))?;
        let arg_strs: Vec<String> = args
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        eprintln!(">> {cmd} {}", arg_strs.join(" "));
        let st = if cmd == "cargo" {
            Command::new(&cargo)
                .current_dir(root)
                .args(&arg_strs)
                .status()?
        } else {
            Command::new(cmd)
                .current_dir(root)
                .args(&arg_strs)
                .status()?
        };
        if !st.success() {
            return Err(anyhow!("mens-gate step failed: {cmd} {:?}", arg_strs));
        }
    }
    println!("Mens gate OK ({profile})");
    Ok(())
}

fn run_toestub_scoped(repo: &Path, scan_root: &Path) -> Result<()> {
    let root: PathBuf = if scan_root.is_absolute() {
        scan_root.to_path_buf()
    } else {
        repo.join(scan_root)
    };
    let cargo = cargo_bin();
    let st = Command::new(&cargo)
        .current_dir(repo)
        .args([
            "run",
            "-p",
            "vox-toestub",
            "--bin",
            "toestub",
            "--",
            root.to_string_lossy().as_ref(),
        ])
        .status()?;
    if !st.success() {
        return Err(anyhow!("toestub scoped run failed"));
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct TimingRecord {
    lane: &'static str,
    ok: bool,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Single source of truth: `docs/ci/build-timings/budgets.json` (see also crate-build-lanes-migration.md).
#[derive(Debug, Deserialize)]
struct BuildTimingBudgetsFile {
    lanes: std::collections::HashMap<String, u64>,
}

fn load_build_timing_budgets(root: &Path) -> Result<std::collections::HashMap<String, u128>> {
    let p = root.join("docs/ci/build-timings/budgets.json");
    let raw = fs::read_to_string(&p)
        .with_context(|| format!("read build timing budgets {}", p.display()))?;
    let parsed: BuildTimingBudgetsFile =
        serde_json::from_str(&raw).context("parse docs/ci/build-timings/budgets.json")?;
    Ok(parsed
        .lanes
        .into_iter()
        .map(|(k, v)| (k, u128::from(v)))
        .collect())
}

/// Soft budgets from `budgets.json`. `VOX_BUILD_TIMINGS_BUDGET_WARN=1` stderr for missing lane keys
/// and over-budget lanes. `VOX_BUILD_TIMINGS_BUDGET_FAIL=1` fails if any lane exceeded its cap (warn not required).
fn apply_build_timing_budgets(records: &[TimingRecord], root: &Path) -> Result<()> {
    let warn = std::env::var("VOX_BUILD_TIMINGS_BUDGET_WARN").unwrap_or_default() == "1";
    let fail = std::env::var("VOX_BUILD_TIMINGS_BUDGET_FAIL").unwrap_or_default() == "1";
    if !warn && !fail {
        return Ok(());
    }
    let budgets = load_build_timing_budgets(root)?;
    let mut any_over = false;
    for r in records {
        if !r.ok {
            continue;
        }
        match budgets.get(r.lane) {
            None => {
                if warn {
                    eprintln!(
                        "build-timings budget: lane {} has no entry in docs/ci/build-timings/budgets.json",
                        r.lane
                    );
                }
            }
            Some(max_ms) => {
                if r.duration_ms > *max_ms {
                    any_over = true;
                    if warn || fail {
                        eprintln!(
                            "build-timings budget: lane {} took {} ms (soft upper {} ms from budgets.json)",
                            r.lane, r.duration_ms, max_ms
                        );
                    }
                }
            }
        }
    }
    if any_over && fail {
        return Err(anyhow!(
            "one or more lanes exceeded soft budget (VOX_BUILD_TIMINGS_BUDGET_FAIL=1)"
        ));
    }
    Ok(())
}

fn run_cargo_lane(cargo: &Path, root: &Path, lane: &'static str, args: &[&str]) -> TimingRecord {
    let start = Instant::now();
    let st = Command::new(cargo).current_dir(root).args(args).status();
    let duration_ms = start.elapsed().as_millis();
    match st {
        Ok(s) if s.success() => TimingRecord {
            lane,
            ok: true,
            duration_ms,
            error: None,
        },
        Ok(_) => TimingRecord {
            lane,
            ok: false,
            duration_ms,
            error: Some("non-zero exit".into()),
        },
        Err(e) => TimingRecord {
            lane,
            ok: false,
            duration_ms,
            error: Some(e.to_string()),
        },
    }
}

fn run_build_timings(root: &Path, json: bool, crates: bool) -> Result<()> {
    let cargo = cargo_bin();
    let mut records: Vec<TimingRecord> = Vec::new();

    let lanes: &[(&str, &[&str])] = &[
        ("check_vox_cli_default", &["check", "-p", "vox-cli"]),
        (
            "check_vox_cli_gpu_stub",
            &[
                "check",
                "-p",
                "vox-cli",
                "--features",
                "gpu,mens-qlora,stub-check",
            ],
        ),
    ];

    for (lane, args) in lanes {
        records.push(run_cargo_lane(&cargo, root, lane, args));
    }

    if crates {
        let crate_lanes: &[(&str, &[&str])] = &[
            (
                "check_vox_cli_no_default_features",
                &["check", "-p", "vox-cli", "--no-default-features"],
            ),
            ("check_vox_db", &["check", "-p", "vox-db"]),
            ("check_vox_oratio", &["check", "-p", "vox-oratio"]),
            (
                "check_vox_mens_train",
                &["check", "-p", "vox-mens", "--features", "train"],
            ),
            (
                "check_vox_cli_populi_oratio",
                &["check", "-p", "vox-cli", "--features", "mens-oratio"],
            ),
        ];
        for (lane, args) in crate_lanes {
            records.push(run_cargo_lane(&cargo, root, lane, args));
        }
    }

    // Optional CUDA lane (same policy as `cuda-features`).
    if std::env::var("SKIP_CUDA_FEATURE_CHECK").unwrap_or_default() != "1" {
        let nvcc_ok = nvcc_available();
        if nvcc_ok {
            records.push(run_cargo_lane(
                &cargo,
                root,
                "check_vox_cli_gpu_populi_candle_cuda",
                &[
                    "check",
                    "-p",
                    "vox-cli",
                    "--features",
                    "gpu,mens-candle-cuda",
                ],
            ));
        }
    }

    if json {
        for r in &records {
            let line = serde_json::to_string(r).with_context(|| {
                format!(
                    "serialize build-timings JSON for lane {} (TimingRecord)",
                    r.lane
                )
            })?;
            println!("{line}");
        }
    } else {
        println!(
            "vox ci build-timings (wall-clock cargo check){}",
            if crates { " + per-crate lanes" } else { "" }
        );
        for r in &records {
            let status = if r.ok { "ok" } else { "FAIL" };
            println!("  {:<40} {}  {} ms", r.lane, status, r.duration_ms);
            if let Some(ref e) = r.error {
                println!("    ({e})");
            }
        }
        if std::env::var("SKIP_CUDA_FEATURE_CHECK").unwrap_or_default() == "1" {
            println!("  (CUDA lane skipped: SKIP_CUDA_FEATURE_CHECK=1)");
        } else if !records
            .iter()
            .any(|r| r.lane == "check_vox_cli_gpu_populi_candle_cuda")
        {
            println!("  (CUDA lane skipped: nvcc not on PATH)");
        }
    }

    if records.iter().any(|r| !r.ok) {
        return Err(anyhow!("one or more build-timings lanes failed"));
    }
    apply_build_timing_budgets(&records, root)?;
    let total_ms: u128 = records.iter().filter(|r| r.ok).map(|r| r.duration_ms).sum();
    crate::benchmark_telemetry::record_opt_blocking(
        "ci_build_timings",
        Some(total_ms as f64),
        Some(serde_json::json!({
            "crates": crates,
            "lanes": records.iter().map(|r| {
                serde_json::json!({"lane": r.lane, "ok": r.ok, "ms": r.duration_ms})
            }).collect::<Vec<_>>(),
        })),
    );
    Ok(())
}

fn run_cuda_features() -> Result<()> {
    if std::env::var("SKIP_CUDA_FEATURE_CHECK").unwrap_or_default() == "1" {
        println!("CUDA feature checks skipped (SKIP_CUDA_FEATURE_CHECK=1)");
        return Ok(());
    }
    let nvcc_ok = nvcc_available();
    if !nvcc_ok {
        println!(
            "CUDA feature checks skipped (nvcc not found — use PATH or CUDA_PATH/CUDA_HOME to toolkit root)"
        );
        return Ok(());
    }
    let root = repo_root();
    let cargo = cargo_bin();
    let st1 = Command::new(&cargo)
        .current_dir(&root)
        .args(["check", "-p", "vox-oratio", "--features", "cuda"])
        .status()?;
    if !st1.success() {
        return Err(anyhow!("cargo check -p vox-oratio --features cuda failed"));
    }
    let st2 = Command::new(&cargo)
        .current_dir(&root)
        .args([
            "check",
            "-p",
            "vox-cli",
            "--features",
            "gpu,mens-candle-cuda",
        ])
        .status()?;
    if !st2.success() {
        return Err(anyhow!(
            "cargo check -p vox-cli --features gpu,mens-candle-cuda failed"
        ));
    }
    println!("CUDA feature checks OK");
    Ok(())
}

#[cfg(test)]
mod build_timing_budget_tests {
    use std::path::PathBuf;

    use super::load_build_timing_budgets;

    /// Keep in sync with `run_build_timings` lane ids + `docs/ci/build-timings/budgets.json`.
    const EXPECTED_BUDGET_LANES: &[&str] = &[
        "check_vox_cli_default",
        "check_vox_cli_gpu_stub",
        "check_vox_cli_gpu_populi_candle_cuda",
        "check_vox_cli_no_default_features",
        "check_vox_db",
        "check_vox_oratio",
        "check_vox_mens_train",
        "check_vox_cli_populi_oratio",
    ];

    #[test]
    fn budgets_json_loads_and_defines_all_timing_lanes() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let m = load_build_timing_budgets(&root).expect("load budgets.json");
        for lane in EXPECTED_BUDGET_LANES {
            assert!(
                m.contains_key(*lane),
                "docs/ci/build-timings/budgets.json missing lane `{lane}`"
            );
        }
    }
}

#[cfg(test)]
mod feature_matrix_contract_tests {
    use super::FEATURE_SETS;

    #[test]
    fn feature_sets_include_script_execution_lane() {
        assert!(
            FEATURE_SETS.contains(&"script-execution"),
            "CI feature matrix must compile the script-execution lane"
        );
        assert!(
            FEATURE_SETS.contains(&"script-execution,stub-check"),
            "CI feature matrix must include a mixed script-execution + stub-check build"
        );
    }

    #[test]
    fn feature_sets_include_populi_oratio_lane() {
        assert!(
            FEATURE_SETS.contains(&"mens-oratio"),
            "CI feature matrix must compile the mens-oratio (Oratio STT) lane"
        );
    }
}
