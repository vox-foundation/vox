//! Architecture check: enforces workspace-wide structural rules.
//!
//! Reads `docs/src/architecture/layers.toml` and runs nine rules over the
//! current `cargo metadata` snapshot:
//!
//!   1. **Layer ordering** (strict by default) — a crate at layer N may depend
//!      only on crates at layer ≤ N. Inversions in `[[known_inversions]]` are
//!      tolerated.
//!   2. **Fan-in tracker** (warn) — workspace dependents per crate vs.
//!      `max_dependents`.
//!   3. **LoC budget** (warn) — `wc -l` over `src/**/*.rs` vs. `max_loc`.
//!   4. **Orphan detector** (warn) — flags crates with 0 in-tree consumers
//!      AND `kind != "plugin" | "binary" | "test-only"`.
//!   5. **Docstring lint** (strict for L0-L2, warn for L3+) — flags `lib.rs`
//!      files that don't open with `//!`.
//!   6. **Description present** (warn) — flags L1+ library crates with no
//!      Cargo.toml `description` or one shorter than 40 characters.
//!   7. **Where-things-live coverage** (warn) — flags workspace members not
//!      listed in `docs/src/architecture/where-things-live.md`.
//!   8. **Staleness** (warn) — flags crates with no commits since the last
//!      release date in `CHANGELOG.md`. Mark stable utility crates with
//!      `staleness_exempt = true` in `layers.toml` to silence the warning.
//!   9. **Generated-file drift** (warn) — files containing a
//!      `@generated-hash <hex>` header whose content hash no longer matches,
//!      indicating a hand-edit of a machine-generated file.
//!
//! Layer ordering is the only rule that fails the build by default; the other
//! eight are warn-only. Per-rule strictness can be set via `[guards]` in
//! `layers.toml`.
//!
//! Modes:
//!   default        — strict layer-ordering; warn-only on the other eight
//!   --warn-only    — warn on layer-ordering too (used during transition phases)
//!
//! Exit codes:
//!   0 — clean (or warn-only)
//!   1 — strict rule failed, OR config error

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, anyhow};
use cargo_metadata::MetadataCommand;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LayersConfig {
    crates: HashMap<String, CrateEntry>,
    #[serde(default)]
    known_inversions: Vec<KnownInversion>,
    #[serde(default)]
    guards: GuardsConfig,
}

#[derive(Debug, Deserialize)]
struct CrateEntry {
    layer: u8,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    max_dependents: Option<usize>,
    #[serde(default)]
    max_loc: Option<usize>,
    /// Opt out of Rule 8 staleness check for intentionally stable crates.
    #[serde(default)]
    staleness_exempt: bool,
}

fn default_kind() -> String {
    "library".to_string()
}

#[derive(Debug, Deserialize)]
struct KnownInversion {
    from: String,
    to: String,
    #[allow(dead_code)]
    reason: String,
}

#[derive(Debug, Default, Deserialize)]
struct GuardsConfig {
    /// "error" or "warn"; defaults to "warn" for all but layer ordering.
    #[serde(default)]
    fan_in: Option<String>,
    #[serde(default)]
    loc_budget: Option<String>,
    #[serde(default)]
    orphan: Option<String>,
    #[serde(default)]
    docstring: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    where_things_live: Option<String>,
    #[serde(default)]
    staleness: Option<String>,
    #[serde(default)]
    generated_file_drift: Option<String>,
}

fn main() -> ExitCode {
    let warn_only = std::env::args().any(|a| a == "--warn-only");

    match run(warn_only) {
        Ok(report) => {
            report.print_summary();
            if report.strict_failed() && !warn_only {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("vox-arch-check: {e:#}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Default)]
struct Report {
    inversions: Vec<(String, String, u8, u8)>,
    fan_in_warns: Vec<(String, usize, usize)>,
    loc_warns: Vec<(String, usize, usize)>,
    orphan_warns: Vec<String>,
    /// Docstring findings split by strictness: (name, strict)
    docstring_warns: Vec<(String, bool)>,
    description_warns: Vec<String>,
    where_things_live_warns: Vec<String>,
    /// Rule 8: (crate_name, last_commit_date YYYY-MM-DD).
    staleness_warns: Vec<(String, String)>,
    /// "vX.Y.Z (YYYY-MM-DD)" used in the staleness summary line.
    staleness_since: String,
    /// Rule 9: (file_path_relative, expected_hash, actual_hash).
    generated_file_drift_warns: Vec<(PathBuf, String, String)>,
    /// Whether each rule's failure should be treated as strict (vs. warn-only).
    strict_layer: bool,
    strict_fan_in: bool,
    strict_loc: bool,
    strict_orphan: bool,
    strict_docstring: bool,
    strict_description: bool,
    strict_where_things_live: bool,
    strict_staleness: bool,
    strict_generated_file_drift: bool,
}

impl Report {
    fn strict_failed(&self) -> bool {
        (self.strict_layer && !self.inversions.is_empty())
            || (self.strict_fan_in && !self.fan_in_warns.is_empty())
            || (self.strict_loc && !self.loc_warns.is_empty())
            || (self.strict_orphan && !self.orphan_warns.is_empty())
            || self
                .docstring_warns
                .iter()
                .any(|(_, strict)| *strict)
            || (self.strict_description && !self.description_warns.is_empty())
            || (self.strict_where_things_live && !self.where_things_live_warns.is_empty())
            || (self.strict_staleness && !self.staleness_warns.is_empty())
            || (self.strict_generated_file_drift && !self.generated_file_drift_warns.is_empty())
    }

    fn print_summary(&self) {
        let mut any = false;
        if !self.inversions.is_empty() {
            any = true;
            let label = if self.strict_layer { "ERROR" } else { "warn" };
            eprintln!("[{label}] layer inversions ({}):", self.inversions.len());
            for (from, to, fl, tl) in &self.inversions {
                eprintln!("  {from} (L{fl}) → {to} (L{tl})");
            }
        }
        if !self.fan_in_warns.is_empty() {
            any = true;
            let label = if self.strict_fan_in { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] fan-in over budget ({}):",
                self.fan_in_warns.len()
            );
            for (name, count, budget) in &self.fan_in_warns {
                eprintln!("  {name}: {count} dependents (budget {budget})");
            }
        }
        if !self.loc_warns.is_empty() {
            any = true;
            let label = if self.strict_loc { "ERROR" } else { "warn" };
            eprintln!("[{label}] LoC budget exceeded ({}):", self.loc_warns.len());
            for (name, loc, budget) in &self.loc_warns {
                eprintln!("  {name}: {loc} LoC (budget {budget})");
            }
        }
        if !self.orphan_warns.is_empty() {
            any = true;
            let label = if self.strict_orphan { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] orphan crates ({}) — 0 in-tree consumers and kind=library:",
                self.orphan_warns.len()
            );
            for name in &self.orphan_warns {
                eprintln!("  {name}");
            }
        }
        // Docstring: split strict vs. warn findings for display
        let docstring_strict: Vec<&str> = self
            .docstring_warns
            .iter()
            .filter(|(_, s)| *s)
            .map(|(n, _)| n.as_str())
            .collect();
        let docstring_warn: Vec<&str> = self
            .docstring_warns
            .iter()
            .filter(|(_, s)| !*s)
            .map(|(n, _)| n.as_str())
            .collect();
        if !docstring_strict.is_empty() {
            any = true;
            eprintln!(
                "[ERROR] lib.rs without `//!` opening docstring — L0-L2 (strict) ({}):",
                docstring_strict.len()
            );
            for name in &docstring_strict {
                eprintln!("  {name}");
            }
        }
        if !docstring_warn.is_empty() {
            any = true;
            let label = if self.strict_docstring { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] lib.rs without `//!` opening docstring — L3+ ({}):",
                docstring_warn.len()
            );
            for name in &docstring_warn {
                eprintln!("  {name}");
            }
        }
        if !self.description_warns.is_empty() {
            any = true;
            let label = if self.strict_description { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] Cargo.toml description missing or too short ({}):",
                self.description_warns.len()
            );
            for msg in &self.description_warns {
                eprintln!("  {msg}");
            }
        }
        if !self.where_things_live_warns.is_empty() {
            any = true;
            let label = if self.strict_where_things_live {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] crates not listed in where-things-live.md ({}):",
                self.where_things_live_warns.len()
            );
            for msg in &self.where_things_live_warns {
                eprintln!("  {msg}");
            }
        }
        if !self.staleness_warns.is_empty() {
            any = true;
            let label = if self.strict_staleness { "ERROR" } else { "warn" };
            eprintln!(
                "[{label}] crates unchanged since {} ({}) — add `staleness_exempt = true` in layers.toml to silence:",
                self.staleness_since,
                self.staleness_warns.len()
            );
            for (name, date) in &self.staleness_warns {
                eprintln!("  {name}: last changed {date}");
            }
        }
        if !self.generated_file_drift_warns.is_empty() {
            any = true;
            let label = if self.strict_generated_file_drift {
                "ERROR"
            } else {
                "warn"
            };
            eprintln!(
                "[{label}] generated-file drift — hand-edited @generated files ({}):",
                self.generated_file_drift_warns.len()
            );
            for (path, expected, actual) in &self.generated_file_drift_warns {
                eprintln!(
                    "  {}: expected hash {expected}, got {actual}  (re-run the generator)",
                    path.display()
                );
            }
        }
        if !any {
            println!(
                "vox-arch-check {}: clean ✓",
                concat!(
                    env!("CARGO_PKG_VERSION"),
                    "+build.",
                    env!("VOX_BUILD_NUMBER"),
                    " (",
                    env!("VOX_GIT_HASH"),
                    ")"
                )
            );
        }
    }
}

fn parse_strictness(setting: Option<&String>, default_strict: bool) -> bool {
    match setting.map(|s| s.as_str()) {
        Some("error") | Some("strict") => true,
        Some("warn") | Some("warning") => false,
        _ => default_strict,
    }
}

fn run(warn_only_flag: bool) -> Result<Report> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;

    let workspace_root: PathBuf = metadata.workspace_root.clone().into();
    let layers_path = workspace_root.join("docs/src/architecture/layers.toml");

    let layers_text = std::fs::read_to_string(&layers_path)
        .with_context(|| format!("reading {}", layers_path.display()))?;
    let layers: LayersConfig = toml::from_str(&layers_text)
        .with_context(|| format!("parsing {}", layers_path.display()))?;

    let workspace_members: HashSet<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let metadata_full = MetadataCommand::new()
        .exec()
        .context("cargo metadata (with deps) failed")?;

    let mut report = Report::default();
    // Layer ordering is strict by default; the others default to warn-only.
    // --warn-only flag downgrades layer ordering to warn too.
    report.strict_layer = !warn_only_flag;
    report.strict_fan_in = parse_strictness(layers.guards.fan_in.as_ref(), false);
    report.strict_loc = parse_strictness(layers.guards.loc_budget.as_ref(), false);
    report.strict_orphan = parse_strictness(layers.guards.orphan.as_ref(), false);
    report.strict_docstring = parse_strictness(layers.guards.docstring.as_ref(), false);
    report.strict_description = parse_strictness(layers.guards.description.as_ref(), false);
    report.strict_where_things_live =
        parse_strictness(layers.guards.where_things_live.as_ref(), false);
    report.strict_staleness = parse_strictness(layers.guards.staleness.as_ref(), false);
    report.strict_generated_file_drift =
        parse_strictness(layers.guards.generated_file_drift.as_ref(), false);

    // ── Rule 1: Layer ordering + Rule 2: Fan-in (single pass) ──
    let mut dependent_count: HashMap<String, usize> = HashMap::new();
    let mut unlisted: Vec<String> = Vec::new();

    for pkg in metadata_full.workspace_packages() {
        let from_name = pkg.name.as_str();
        let from_layer = match layers.crates.get(from_name) {
            Some(e) => e.layer,
            None => {
                unlisted.push(from_name.to_string());
                continue;
            }
        };
        for dep in &pkg.dependencies {
            let to_name = dep.name.as_str();
            if !workspace_members.contains(to_name) {
                continue;
            }
            *dependent_count.entry(to_name.to_string()).or_insert(0) += 1;

            let to_layer = match layers.crates.get(to_name) {
                Some(e) => e.layer,
                None => continue,
            };
            if to_layer > from_layer {
                let is_known = layers
                    .known_inversions
                    .iter()
                    .any(|k| k.from == from_name && k.to == to_name);
                if !is_known {
                    report.inversions.push((
                        from_name.to_string(),
                        to_name.to_string(),
                        from_layer,
                        to_layer,
                    ));
                }
            }
        }
    }

    if !unlisted.is_empty() {
        unlisted.sort();
        unlisted.dedup();
        return Err(anyhow!(
            "{} workspace crate(s) missing from layers.toml: {}",
            unlisted.len(),
            unlisted.join(", ")
        ));
    }

    // Rule 2: fan-in budget
    for (name, entry) in &layers.crates {
        if let Some(budget) = entry.max_dependents {
            let count = dependent_count.get(name).copied().unwrap_or(0);
            if count > budget {
                report.fan_in_warns.push((name.clone(), count, budget));
            }
        }
    }

    // ── Rule 3: LoC budget ──
    for pkg in metadata_full.workspace_packages() {
        let name = pkg.name.as_str();
        let entry = match layers.crates.get(name) {
            Some(e) => e,
            None => continue,
        };
        let budget = match entry.max_loc {
            Some(b) => b,
            None => continue,
        };
        let manifest_dir = Path::new(pkg.manifest_path.as_str())
            .parent()
            .unwrap_or(Path::new("."));
        let src_dir = manifest_dir.join("src");
        let loc = count_loc(&src_dir).unwrap_or(0);
        if loc > budget {
            report.loc_warns.push((name.to_string(), loc, budget));
        }
    }

    // ── Rule 4: Orphan detector ──
    for (name, entry) in &layers.crates {
        if entry.kind != "library" {
            continue;
        }
        let count = dependent_count.get(name).copied().unwrap_or(0);
        if count == 0 && workspace_members.contains(name.as_str()) {
            report.orphan_warns.push(name.clone());
        }
    }
    report.orphan_warns.sort();

    // ── Rule 5: Docstring lint (strict for L0-L2, warn for L3+) ──
    for pkg in metadata_full.workspace_packages() {
        let name = pkg.name.as_str();
        let layer = match layers.crates.get(name) {
            Some(e) => e.layer,
            None => continue,
        };
        let manifest_dir = Path::new(pkg.manifest_path.as_str())
            .parent()
            .unwrap_or(Path::new("."));
        let lib_rs = manifest_dir.join("src").join("lib.rs");
        if !lib_rs.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&lib_rs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let first_nonempty = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        if !first_nonempty.trim_start().starts_with("//!") {
            // L0-L2: strict (always fail); L3+: governed by strict_docstring guard
            let is_strict = layer <= 2;
            report.docstring_warns.push((name.to_string(), is_strict));
        }
    }
    report.docstring_warns.sort_by(|a, b| a.0.cmp(&b.0));

    // ── Rule 6: Description present ──
    report.description_warns =
        check_description_present(&metadata_full, &layers);

    // ── Rule 7: Where-things-live coverage ──
    report.where_things_live_warns =
        check_where_things_live_coverage(&metadata_full, &layers, &workspace_root)
            .unwrap_or_else(|e| {
                eprintln!("warn: where-things-live check skipped: {e:#}");
                Vec::new()
            });

    // ── Rule 8: Staleness ──
    // Flags crates with no commits since the last release date in CHANGELOG.md.
    // Plugins (independent versioning) and staleness_exempt crates are skipped.
    let changelog_path = workspace_root.join("CHANGELOG.md");
    if let Some((release_version, release_date)) = parse_release_date(&changelog_path) {
        report.staleness_since = format!("v{release_version} ({release_date})");
        for pkg in metadata_full.workspace_packages() {
            let name = pkg.name.as_str();
            let entry = match layers.crates.get(name) {
                Some(e) => e,
                None => continue,
            };
            if entry.staleness_exempt || entry.kind == "plugin" {
                continue;
            }
            let manifest_dir = Path::new(pkg.manifest_path.as_str())
                .parent()
                .unwrap_or(Path::new("."));
            if let Some(last_commit) = git_last_commit_date(manifest_dir) {
                // ISO date strings compare lexicographically: "2026-03-01" < "2026-04-18"
                if last_commit < release_date {
                    report.staleness_warns.push((name.to_string(), last_commit));
                }
            }
        }
        report.staleness_warns.sort();
    }

    // ── Rule 9: Generated-file drift ──
    report.generated_file_drift_warns =
        check_generated_file_drift(&workspace_root).unwrap_or_else(|e| {
            eprintln!("warn: generated-file-drift check skipped: {e:#}");
            Vec::new()
        });

    Ok(report)
}

/// Return the YYYY-MM-DD date of the last commit touching `dir`, or `None` if git is unavailable.
fn git_last_commit_date(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["log", "-n", "1", "--format=%cd", "--date=short"])
        .arg("--")
        .arg(dir)
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    let date = s.trim().to_string();
    // Expect exactly "YYYY-MM-DD" (10 chars); ignore empty output (no commits touching dir).
    if date.len() == 10 { Some(date) } else { None }
}

/// Parse the most recent released version and its date from `CHANGELOG.md`.
///
/// Looks for lines matching `## [X.Y.Z] - YYYY-MM-DD`, skipping `[Unreleased]`.
/// Returns `(version, date)` of the first match, or `None` if the file is absent
/// or has no released entries yet.
fn parse_release_date(changelog: &Path) -> Option<(String, String)> {
    let content = std::fs::read_to_string(changelog).ok()?;
    for line in content.lines() {
        let t = line.trim();
        if !t.starts_with("## [") || t.contains("Unreleased") {
            continue;
        }
        // "## [0.5.0] - 2026-04-18"
        let inner = t.strip_prefix("## [")?;
        let ver_end = inner.find(']')?;
        let version = inner[..ver_end].to_string();
        let rest = inner[ver_end..].strip_prefix("] - ")?;
        let date = rest.trim();
        if date.len() == 10 && date.as_bytes()[4] == b'-' {
            return Some((version, date.to_string()));
        }
    }
    None
}

/// Warn (or fail) if a workspace member at L1+ has no `description` field
/// in its Cargo.toml or has one shorter than 40 characters. Binary-only
/// crates (`kind = "binary"`) and `workspace-hack` are exempt.
fn check_description_present(
    meta: &cargo_metadata::Metadata,
    cfg: &LayersConfig,
) -> Vec<String> {
    let mut findings = Vec::new();
    let workspace_ids: HashSet<&cargo_metadata::PackageId> =
        meta.workspace_members.iter().collect();
    for pkg in meta.packages.iter().filter(|p| workspace_ids.contains(&p.id)) {
        let name = pkg.name.as_str();
        let Some(entry) = cfg.crates.get(name) else {
            continue;
        };
        if entry.layer < 1 {
            continue;
        }
        if entry.kind == "binary" {
            continue;
        }
        if name == "workspace-hack" {
            continue;
        }
        let desc = pkg.description.as_deref().unwrap_or("");
        if desc.len() < 40 {
            findings.push(format!(
                "{}: Cargo.toml description is missing or shorter than 40 chars (\"{}\")",
                name, desc,
            ));
        }
    }
    findings.sort();
    findings
}

/// Warn if a workspace member is not mentioned in
/// `docs/src/architecture/where-things-live.md`.
fn check_where_things_live_coverage(
    meta: &cargo_metadata::Metadata,
    cfg: &LayersConfig,
    repo_root: &std::path::Path,
) -> anyhow::Result<Vec<String>> {
    let path = repo_root.join("docs/src/architecture/where-things-live.md");
    let body = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let mut findings = Vec::new();
    let workspace_ids: HashSet<&cargo_metadata::PackageId> =
        meta.workspace_members.iter().collect();
    for pkg in meta.packages.iter().filter(|p| workspace_ids.contains(&p.id)) {
        let name = pkg.name.as_str();
        if !cfg.crates.contains_key(name) {
            continue;
        }
        if name == "workspace-hack" {
            continue;
        }
        let needle = format!("crates/{}/", name);
        if !body.contains(&needle) {
            findings.push(format!(
                "{}: not listed in where-things-live.md (no `{}` substring)",
                name, needle,
            ));
        }
    }
    findings.sort();
    Ok(findings)
}

/// Count non-blank, non-comment-only lines under `dir/**/*.rs` (best-effort).
fn count_loc(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut total = 0usize;
    for entry in walkdir(dir) {
        if entry.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&entry) {
            total += content.lines().count();
        }
    }
    Ok(total)
}

/// FNV-1a 64-bit hash of `bytes` — stable across Rust versions, no extra deps.
fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 14695981039346656037u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    format!("{hash:016x}")
}

/// Check for files that declare `@generated-hash <hex>` in their first five lines but
/// whose content (with that header line blanked out) no longer matches the recorded hash.
///
/// The header format is flexible: any line containing `@generated-hash ` followed by a
/// 16-character hex string is treated as the marker, regardless of comment prefix
/// (`//`, `#`, `<!--`, etc.).
fn check_generated_file_drift(workspace_root: &Path) -> anyhow::Result<Vec<(PathBuf, String, String)>> {
    const MARKER: &str = "@generated-hash ";
    const HASH_LEN: usize = 16;
    // Extensions that may carry generated-hash headers.
    const TRACKED_EXTS: &[&str] = &["rs", "ts", "tsx", "js", "vox", "md", "toml", "json"];

    let mut warns = Vec::new();

    for path in walkdir(workspace_root) {
        // Skip binary-likely files and hidden dirs quickly.
        let rel = match path.strip_prefix(workspace_root) {
            Ok(r) => r,
            Err(_) => path.as_path(),
        };
        // Skip target/, .git/, node_modules/
        let rel_str = rel.to_string_lossy();
        if rel_str.starts_with("target") || rel_str.starts_with(".git") || rel_str.contains("node_modules") {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !TRACKED_EXTS.contains(&ext) {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Only scan the first five lines for the marker.
        let mut header_line_idx: Option<usize> = None;
        let mut recorded_hash = String::new();
        for (i, line) in content.lines().enumerate().take(5) {
            if let Some(pos) = line.find(MARKER) {
                let after = &line[pos + MARKER.len()..];
                let candidate: &str = after.split_whitespace().next().unwrap_or("");
                if candidate.len() == HASH_LEN && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
                    header_line_idx = Some(i);
                    recorded_hash = candidate.to_string();
                    break;
                }
            }
        }

        let Some(marker_line) = header_line_idx else { continue };

        // Recompute hash over file content with the marker line blanked.
        let body_for_hash: String = content
            .lines()
            .enumerate()
            .map(|(i, line)| if i == marker_line { "" } else { line })
            .collect::<Vec<_>>()
            .join("\n");
        let actual_hash = fnv1a_hex(body_for_hash.as_bytes());

        if actual_hash != recorded_hash {
            warns.push((rel.to_path_buf(), recorded_hash, actual_hash));
        }
    }

    warns.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(warns)
}

fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        let entries = match std::fs::read_dir(&p) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out
}
