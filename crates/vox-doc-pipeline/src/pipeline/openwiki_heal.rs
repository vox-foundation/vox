//! Tiered Self-Updating & Self-Fixing OpenWiki Engine (`WIKI-03`, `WIKI-04`):
//! - Tier 1: Zero-LLM AST Blast-Radius Auto-Heal (`<!-- openwiki:auto:start -->` + shifted `#Lstart-Lend`)
//! - Tier 2: Grounded Narrative Heal Synthesis + Four-Gate Deterministic Verification

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::claims::hasher::hash_normalized_code;
use crate::claims::llm_verifier::{
    is_valid_line_range_citation, is_valid_line_range_citation_on_disk,
};
use crate::claims::self_heal_bot::{BotDraftPrSpec, BotMetricsLedger, CandidateDocPatch};
use crate::pipeline::doctest::check_doctests;
use crate::pipeline::facets::{FacetCtx, facet_issues, facet_lint_rel_path};
use crate::pipeline::lint::lint_file;
use crate::pipeline::openwiki_polyglot::{
    extract_polyglot_wiki_snapshot, openwiki_build, openwiki_diff, openwiki_pages_dir,
    openwiki_pages_rel,
};
use crate::pipeline::{fence_issues, is_hard_lint_error};

const NEGATION_TOKENS: &[&str] = &["not", "never", "cannot", "without", "forbid", "prohibit"];
const RETIRED_SURFACES: &[&str] = &[
    "TantivyDocsIndex",
    "research-index.md",
    "architecture-index.md",
    "ProbeOnlyExecutor",
    "vox_kb_put",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactedDocTarget {
    pub doc_path: String,
    pub reasons: Vec<String>,
    pub cited_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlastRadiusReport {
    pub changed_inputs: Vec<String>,
    pub impacted_docs: Vec<ImpactedDocTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tier1HealReport {
    pub blast_radius: BlastRadiusReport,
    pub llm_calls: usize,
    pub updated_pages: Vec<String>,
    pub repaired_anchors: usize,
    pub auto_regions_updated: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tier2GateReport {
    pub passed: bool,
    pub rewrite_guard_passed: bool,
    pub doc_lint_passed: bool,
    pub diagram_check_passed: bool,
    pub doctest_passed: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tier2ProseHealCandidate {
    pub doc_path: String,
    pub summary: String,
    pub unified_diff: String,
    pub original_hash: String,
    pub patched_markdown: String,
    pub evidence_citations: Vec<String>,
    pub gate_report: Tier2GateReport,
    pub draft_pr: Option<BotDraftPrSpec>,
}

/// Resolve affected documentation pages by tracing modified file paths and symbol names across
/// reverse `documented_in`, `cites_symbol`, and `governed_by_adr` links (`WIKI-03`).
pub fn resolve_blast_radius(
    repo_root: &Path,
    changed_paths_or_symbols: &[String],
) -> BlastRadiusReport {
    let mut changed_inputs: Vec<String> = changed_paths_or_symbols
        .iter()
        .map(|s| s.trim().replace('\\', "/"))
        .filter(|s| !s.is_empty())
        .collect();
    let pages_rel = openwiki_pages_rel(repo_root);
    let pages_prefix = format!("{pages_rel}/");

    if changed_inputs.is_empty() {
        // Read-only diff lookup isolated from inherited GIT_* hook variables; `vox-doc-pipeline`
        // stays decoupled from `vox-git` / `GitExec` concurrency harness.
        // vox-arch-check: allow git-exec
        let mut cmd = std::process::Command::new("git");
        for (k, _) in std::env::vars_os() {
            if k.to_string_lossy().starts_with("GIT_") {
                cmd.env_remove(&k);
            }
        }
        if let Ok(out) = cmd
            .arg("-C")
            .arg(repo_root)
            .args(["diff", "--name-only", "HEAD"])
            .output()
            && out.status.success()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let norm = line.trim().replace('\\', "/");
                if !norm.is_empty() {
                    changed_inputs.push(norm);
                }
            }
        }
        if let Ok(diff) = openwiki_diff(repo_root, None) {
            for d in diff.diffs {
                if d.changed {
                    changed_inputs.push(format!("{pages_rel}/{}", d.rel_path));
                }
            }
        }
    }
    changed_inputs.sort();
    changed_inputs.dedup();

    let snapshot = extract_polyglot_wiki_snapshot(repo_root);
    let mut active_symbols: BTreeSet<String> = BTreeSet::new();
    for item in snapshot
        .interfaces
        .iter()
        .chain(snapshot.schemas.iter())
        .chain(snapshot.gui_surfaces.iter())
        .chain(snapshot.workflows.iter())
    {
        if changed_inputs
            .iter()
            .any(|inp| inp == &item.rel_path || inp == &item.name)
        {
            active_symbols.insert(item.name.clone());
        }
    }
    for inp in &changed_inputs {
        if !inp.contains('/') {
            active_symbols.insert(inp.clone());
        }
    }

    let mut doc_files = Vec::new();
    for dir_rel in [pages_rel.as_str(), "docs/src", "docs", "docs/adr"] {
        let dir = repo_root.join(dir_rel);
        if dir.is_dir() {
            gather_doc_candidates(&dir, repo_root, &mut doc_files);
        }
    }
    doc_files.sort();
    doc_files.dedup();

    let mut impacted: Vec<ImpactedDocTarget> = Vec::new();
    for doc_rel in doc_files {
        let full = repo_root.join(&doc_rel);
        let Ok(md) = fs::read_to_string(&full) else {
            continue;
        };
        let mut reasons: BTreeSet<String> = BTreeSet::new();
        let mut cited_symbols: BTreeSet<String> = BTreeSet::new();

        for inp in &changed_inputs {
            let basename = Path::new(inp)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(inp.as_str());
            if inp == &doc_rel {
                reasons.insert("direct_doc_drift".to_string());
            } else if md.contains(inp.as_str()) || (!basename.is_empty() && md.contains(basename)) {
                reasons.insert(format!("documented_in:{inp}"));
            } else if (inp.contains("/adr/") || inp.ends_with(".adr.md"))
                && (doc_rel.starts_with(&pages_prefix) || md.contains("adr"))
            {
                reasons.insert(format!("governed_by_adr:{inp}"));
            }
        }
        for sym in &active_symbols {
            if md.contains(sym.as_str()) {
                reasons.insert(format!("cites_symbol:{sym}"));
                cited_symbols.insert(sym.clone());
            }
        }

        // Fallback layer affinity for newly introduced files before any doc link existed.
        if reasons.is_empty() && doc_rel.starts_with(&pages_prefix) {
            for inp in &changed_inputs {
                let layer_file = classify_source_to_wiki_layer(inp);
                if doc_rel.ends_with(layer_file)
                    || doc_rel.ends_with(&format!("{pages_prefix}README.md"))
                {
                    reasons.insert(format!("layer_affinity:{inp}"));
                }
            }
        }

        if !reasons.is_empty() {
            impacted.push(ImpactedDocTarget {
                doc_path: doc_rel,
                reasons: reasons.into_iter().collect(),
                cited_symbols: cited_symbols.into_iter().collect(),
            });
        }
    }

    BlastRadiusReport {
        changed_inputs,
        impacted_docs: impacted,
    }
}

fn gather_doc_candidates(dir: &Path, repo_root: &Path, out: &mut Vec<String>) {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        let p = entry.path();
        if !p.is_file() || p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let rel = p
            .strip_prefix(repo_root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/");
        if rel.contains("/archive/") || rel.ends_with(".generated.md") {
            continue;
        }
        out.push(rel);
    }
}

fn classify_source_to_wiki_layer(rel_path: &str) -> &'static str {
    if rel_path.ends_with("Cargo.toml")
        || rel_path.ends_with("package.json")
        || rel_path.ends_with("pyproject.toml")
        || rel_path.ends_with("go.mod")
        || rel_path.ends_with("Vox.toml")
        || rel_path.ends_with("layers.toml")
    {
        "architecture-topology.md"
    } else if rel_path.ends_with(".sql")
        || rel_path.contains("schema")
        || rel_path.contains("model")
    {
        "data-models-and-schemas.md"
    } else if rel_path.ends_with(".tsx")
        || rel_path.ends_with(".jsx")
        || rel_path.contains("/components/")
    {
        "gui-surfaces-and-flows.md"
    } else if rel_path.starts_with(".github/workflows/") || rel_path.starts_with("scripts/") {
        "workflows-and-runbooks.md"
    } else {
        "interfaces-and-apis.md"
    }
}

/// Repair shifted `#L<start>-L<end>` or `:L<start>-L<end>` source references in `markdown` by
/// matching adjacent symbol identifiers against the live polyglot symbol index (`WIKI-03`).
pub fn repair_shifted_line_anchors_in_markdown(
    repo_root: &Path,
    doc_rel_path: &str,
    markdown: &str,
) -> (String, usize) {
    let snapshot = extract_polyglot_wiki_snapshot(repo_root);
    let mut by_file_and_symbol: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
    for item in snapshot
        .interfaces
        .iter()
        .chain(snapshot.schemas.iter())
        .chain(snapshot.gui_surfaces.iter())
        .chain(snapshot.workflows.iter())
    {
        by_file_and_symbol.insert(
            (item.rel_path.clone(), item.name.clone()),
            (item.start_line, item.end_line),
        );
        if let Some(short) = item.name.rsplit("::").next() {
            by_file_and_symbol.insert(
                (item.rel_path.clone(), short.to_string()),
                (item.start_line, item.end_line),
            );
        }
    }

    let Ok(link_re) = Regex::new(r"\[([^\]]+)\]\(([^)\s]+)#L(\d+)(?:-L(\d+))?\)") else {
        return (markdown.to_string(), 0);
    };
    let mut repairs = 0usize;
    let mut out = String::with_capacity(markdown.len());
    for line in markdown.lines() {
        let mut updated = line.to_string();
        for caps in link_re.captures_iter(line) {
            let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let label = &caps[1];
            let target_href = &caps[2];
            let old_start = caps[3].parse::<usize>().unwrap_or(0);
            let old_end = caps
                .get(4)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(old_start);

            let normalized_target = resolve_relative_href(doc_rel_path, target_href);
            // Match either symbol cited in the label (`fn foo` / `foo`) or any known symbol on that line.
            let candidate_span = by_file_and_symbol
                .iter()
                .filter(|((file, sym), _)| {
                    *file == normalized_target
                        && (label.contains(sym.as_str()) || line.contains(&format!("`{sym}`")))
                })
                .map(|(_, span)| *span)
                .next();

            if let Some((new_start, new_end)) = candidate_span
                && (new_start != old_start || new_end != old_end)
            {
                let new_frag = if new_end > new_start {
                    format!("#L{new_start}-L{new_end}")
                } else {
                    format!("#L{new_start}")
                };
                let replacement = format!("[{label}]({target_href}{new_frag})");
                if updated.contains(full_match) {
                    updated = updated.replace(full_match, &replacement);
                    repairs += 1;
                }
            }
        }
        out.push_str(&updated);
        out.push('\n');
    }
    if !markdown.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    (out, repairs)
}

fn resolve_relative_href(doc_rel_path: &str, href: &str) -> String {
    let mut doc_parts: Vec<String> = Path::new(doc_rel_path)
        .parent()
        .map(|p| {
            p.iter()
                .filter_map(|c| c.to_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for seg in href.replace('\\', "/").split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                let _ = doc_parts.pop();
            }
            other => doc_parts.push(other.to_string()),
        }
    }
    doc_parts.join("/")
}

/// Execute Tier 1 deterministic self-healing: resolve blast radius, rebuild deterministic
/// `openwiki:auto` blocks with zero LLM calls, and repair shifted line anchors (`WIKI-03`).
pub fn heal_tier1_deterministic(
    repo_root: &Path,
    changed_paths_or_symbols: &[String],
    write: bool,
    output_override: Option<&Path>,
) -> Result<Tier1HealReport, String> {
    let blast_radius = resolve_blast_radius(repo_root, changed_paths_or_symbols);
    let build = openwiki_build(repo_root, write, output_override)?;

    let mut updated_pages: BTreeSet<String> = build.updated_pages.iter().cloned().collect();
    let mut repaired_anchors = 0usize;

    let wiki_dir = match output_override {
        Some(dir) => dir.join("wiki"),
        None => openwiki_pages_dir(repo_root),
    };
    let mut candidate_docs: Vec<(String, std::path::PathBuf)> = Vec::new();
    for page in &build.generated_pages {
        let fname = Path::new(page)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(page.as_str());
        candidate_docs.push((page.clone(), wiki_dir.join(fname)));
    }
    if output_override.is_none() {
        let pages_prefix = format!("{}/", openwiki_pages_rel(repo_root));
        for imp in &blast_radius.impacted_docs {
            if !imp.doc_path.starts_with(&pages_prefix) {
                candidate_docs.push((imp.doc_path.clone(), repo_root.join(&imp.doc_path)));
            }
        }
    }

    for (rel, path) in candidate_docs {
        if !path.is_file() {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let (repaired, count) = repair_shifted_line_anchors_in_markdown(repo_root, &rel, &raw);
        if count > 0 && repaired != raw {
            repaired_anchors += count;
            updated_pages.insert(rel);
            if write {
                fs::write(&path, repaired)
                    .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
            }
        }
    }

    Ok(Tier1HealReport {
        blast_radius,
        llm_calls: 0,
        updated_pages: updated_pages.into_iter().collect(),
        repaired_anchors,
        auto_regions_updated: build.updated_regions,
    })
}

/// Produce a standard compact unified diff (`--- a/...`, `+++ b/...`, `@@ ... @@`) without extra deps.
pub fn render_unified_diff(original_md: &str, patched_md: &str, doc_path: &str) -> String {
    let norm_a = original_md.replace("\r\n", "\n");
    let norm_b = patched_md.replace("\r\n", "\n");
    if norm_a == norm_b {
        return String::new();
    }
    let a: Vec<&str> = norm_a.lines().collect();
    let b: Vec<&str> = norm_b.lines().collect();

    let mut prefix = 0usize;
    while prefix < a.len() && prefix < b.len() && a[prefix] == b[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix + prefix < a.len()
        && suffix + prefix < b.len()
        && a[a.len() - 1 - suffix] == b[b.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let a_start = prefix + 1;
    let a_count = a.len().saturating_sub(prefix + suffix);
    let b_start = prefix + 1;
    let b_count = b.len().saturating_sub(prefix + suffix);

    let mut diff = format!(
        "--- a/{doc_path}\n+++ b/{doc_path}\n@@ -{a_start},{a_count} +{b_start},{b_count} @@\n"
    );
    if prefix > 0 {
        diff.push_str(&format!(" {}\n", a[prefix - 1]));
    }
    for line in &a[prefix..a.len() - suffix] {
        diff.push_str(&format!("-{line}\n"));
    }
    for line in &b[prefix..b.len() - suffix] {
        diff.push_str(&format!("+{line}\n"));
    }
    if suffix > 0 {
        diff.push_str(&format!(" {}\n", a[a.len() - suffix]));
    }
    diff
}

/// Gate every Tier-2 prose heal patch through all 4 deterministic checks (`WIKI-04`):
/// 1. `kb_rewrite_guard` invariants + `[path:Lstart-Lend]` citation checks
/// 2. `vox-doc-pipeline` frontmatter + facet lint (`0` hard errors)
/// 3. `vox-diagram` structural check (`0` errors, `0` legacy Mermaid)
/// 4. `.vox` doctest compilation check (`0` errors)
pub fn verify_tier2_candidate_four_gates(
    repo_root: &Path,
    doc_rel_path: &str,
    original_md: &str,
    patched_md: &str,
    evidence_citations: &[String],
) -> Tier2GateReport {
    let mut diagnostics = Vec::new();

    // Gate 1: Rewrite Guard invariants (preserved code fences, summary numbers & negations, zero retired symbols, citations)
    let mut rewrite_guard_passed = true;
    if evidence_citations.is_empty()
        || !evidence_citations.iter().all(|c| {
            is_valid_line_range_citation(c) && is_valid_line_range_citation_on_disk(repo_root, c)
        })
    {
        rewrite_guard_passed = false;
        diagnostics.push(
            "rewrite_guard: missing or invalid on-disk [path:Lstart-Lend] citation".to_string(),
        );
    }
    if extract_fences(original_md) != extract_fences(patched_md) {
        rewrite_guard_passed = false;
        diagnostics.push("rewrite_guard: fenced code blocks were unexpectedly altered".to_string());
    }
    let orig_summary = extract_kb_summary(original_md);
    let new_summary = extract_kb_summary(patched_md);
    if !orig_summary.is_empty() {
        if new_summary.is_empty() {
            rewrite_guard_passed = false;
            diagnostics
                .push("rewrite_guard: <!-- kb:summary --> block missing in candidate".to_string());
        } else {
            let orig_nums = extract_numbers(&orig_summary);
            let new_nums = extract_numbers(&new_summary);
            if !orig_nums.is_subset(&new_nums) {
                rewrite_guard_passed = false;
                diagnostics
                    .push("rewrite_guard: numeric literal dropped from kb:summary".to_string());
            }
            let orig_negs = extract_negations(&orig_summary);
            let new_negs = extract_negations(&new_summary);
            if !orig_negs.is_subset(&new_negs) {
                rewrite_guard_passed = false;
                diagnostics
                    .push("rewrite_guard: negation polarity dropped from kb:summary".to_string());
            }
        }
    }
    for retired in RETIRED_SURFACES {
        if patched_md.contains(retired) && !original_md.contains(retired) {
            rewrite_guard_passed = false;
            diagnostics.push(format!(
                "rewrite_guard: retired surface `{retired}` introduced"
            ));
        }
    }

    // Gate 2: Doc frontmatter + facet lint
    let full_path = repo_root.join(doc_rel_path);
    let mut lint_errors = Vec::new();
    lint_file(&full_path, patched_md, repo_root, &mut lint_errors);
    if let Ok(ctx) = FacetCtx::load_or_embedded(repo_root)
        && let Some(rel) = facet_lint_rel_path(&full_path, repo_root)
    {
        let excused = ctx.backlog.rules_for(&rel).unwrap_or(&[]);
        for (line, issue) in facet_issues(patched_md, &ctx.taxonomy) {
            if !excused.iter().any(|r| r == issue.rule_id()) {
                lint_errors.push(crate::pipeline::types::LintError {
                    file: full_path.clone(),
                    line,
                    kind: crate::pipeline::types::LintKind::Facet(issue),
                });
            }
        }
    }
    let hard_lint: Vec<String> = lint_errors
        .iter()
        .filter(|e| is_hard_lint_error(e))
        .map(|e| format!("doc_lint line {}: {:?}", e.line, e.kind))
        .collect();
    let doc_lint_passed = hard_lint.is_empty();
    diagnostics.extend(hard_lint);

    // Gate 3: VoxDiagram check + zero legacy Mermaid
    let mut diagram_check_passed = true;
    for issue in fence_issues(patched_md) {
        diagnostics.push(format!("diagram: {issue:?}"));
        diagram_check_passed = false;
    }
    if !vox_diagram::fence::extract(patched_md, "mermaid").is_empty() {
        diagram_check_passed = false;
        diagnostics
            .push("diagram: legacy ```mermaid block forbidden in healed OpenWiki page".to_string());
    }

    // Gate 4: Doctest compilation
    let mut dt_errors = Vec::new();
    check_doctests(&full_path, patched_md, &mut dt_errors);
    let doctest_passed = dt_errors.is_empty();
    for e in dt_errors {
        diagnostics.push(format!("doctest line {}: {:?}", e.line, e.kind));
    }

    Tier2GateReport {
        passed: rewrite_guard_passed && doc_lint_passed && diagram_check_passed && doctest_passed,
        rewrite_guard_passed,
        doc_lint_passed,
        diagram_check_passed,
        doctest_passed,
        diagnostics,
    }
}

fn extract_fences(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut active = false;
    let mut cur = String::new();
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            cur.push_str(line);
            cur.push('\n');
            if active {
                out.push(std::mem::take(&mut cur));
                active = false;
            } else {
                active = true;
            }
        } else if active {
            cur.push_str(line);
            cur.push('\n');
        }
    }
    out
}

fn extract_kb_summary(md: &str) -> String {
    let Some(s) = md.find("<!-- kb:summary -->") else {
        return String::new();
    };
    let rest = &md[s + "<!-- kb:summary -->".len()..];
    rest.find("<!-- /kb:summary -->")
        .map(|e| rest[..e].trim().to_string())
        .unwrap_or_default()
}

fn extract_numbers(s: &str) -> BTreeSet<String> {
    s.split(|c: char| !c.is_ascii_digit() && c != '.')
        .map(|w| w.trim_matches('.'))
        .filter(|w| !w.is_empty() && w.chars().any(|c| c.is_ascii_digit()))
        .map(str::to_string)
        .collect()
}

fn extract_negations(s: &str) -> BTreeSet<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| NEGATION_TOKENS.contains(w))
        .map(str::to_string)
        .collect()
}

/// Synthesize a minimal grounded prose patch outside `openwiki:auto` fences, verify all 4
/// gates, render unified diff, and optionally prepare a guarded `BotDraftPrSpec` (`WIKI-04`).
/// Surgical, line-scoped Tier-2 fix (`LLM-HEAL-03`): replace one backticked token on one line
/// with `new_token`, leaving every other byte of the document alone. The result is gated by
/// [`verify_tier2_candidate_four_gates`] against `evidence_citation`.
pub fn synthesize_line_scoped_fix(
    repo_root: &Path,
    doc_rel_path: &str,
    line_no: usize,
    old_token: &str,
    new_token: &str,
    evidence_citation: &str,
) -> Result<Tier2ProseHealCandidate, String> {
    let full = repo_root.join(doc_rel_path);
    let original_md =
        fs::read_to_string(&full).map_err(|e| format!("failed to read {}: {e}", full.display()))?;
    let mut lines: Vec<String> = original_md.split('\n').map(str::to_string).collect();
    let idx = line_no
        .checked_sub(1)
        .filter(|i| *i < lines.len())
        .ok_or_else(|| format!("line {line_no} is out of range in {doc_rel_path}"))?;
    let from = format!("`{old_token}`");
    let to = format!("`{new_token}`");
    if !lines[idx].contains(&from) {
        return Err(format!(
            "line {line_no} of {doc_rel_path} does not contain {from}; refusing to guess"
        ));
    }
    lines[idx] = lines[idx].replace(&from, &to);
    let patched_markdown = lines.join("\n");

    let evidence_citations = vec![evidence_citation.to_string()];
    let gate_report = verify_tier2_candidate_four_gates(
        repo_root,
        doc_rel_path,
        &original_md,
        &patched_markdown,
        &evidence_citations,
    );
    Ok(Tier2ProseHealCandidate {
        doc_path: doc_rel_path.to_string(),
        summary: format!(
            "{doc_rel_path}:{line_no}: replace {from} with {to} ({evidence_citation})"
        ),
        unified_diff: render_unified_diff(&original_md, &patched_markdown, doc_rel_path),
        original_hash: hash_normalized_code(&original_md),
        patched_markdown,
        evidence_citations,
        gate_report,
        draft_pr: None,
    })
}

fn is_plain_replacement_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/'))
}

/// Pages the automatic fixer may rewrite. Decision records (`adr/`), history, the changelog and
/// any page whose file name carries an ISO date are *point-in-time records*: renaming a crate in
/// them rewrites history rather than correcting a stale claim. Those are for a human (or a
/// `status: legacy` demotion), never for `--apply`.
#[must_use]
pub fn is_autofix_eligible_path(doc_rel_path: &str) -> bool {
    let rel = doc_rel_path.replace('\\', "/");
    if rel.starts_with("docs/src/adr/")
        || rel.contains("/history/")
        || rel.contains("/archive/")
        || rel.ends_with("/changelog.md")
    {
        return false;
    }
    let name = rel.rsplit('/').next().unwrap_or(&rel);
    let bytes = name.as_bytes();
    // `YYYY-MM-DD` anywhere in the file name.
    !(0..bytes.len().saturating_sub(9)).any(|i| {
        let w = &bytes[i..i + 10];
        w[..4].iter().all(u8::is_ascii_digit)
            && w[4] == b'-'
            && w[5..7].iter().all(u8::is_ascii_digit)
            && w[7] == b'-'
            && w[8..10].iter().all(u8::is_ascii_digit)
    })
}

/// Plan one surgical fix per prescriptive retired-symbol mention in `doc_rel_path` that has a
/// plain canonical replacement. Evidence is the retired-symbols contract line, or the replacement
/// crate's manifest when the retirement is only known to the resolver. Nothing is written.
pub fn plan_retired_symbol_fixes(
    repo_root: &Path,
    registry: &crate::claims::GroundTruthRegistry,
    doc_rel_path: &str,
) -> Result<Vec<Tier2ProseHealCandidate>, String> {
    use crate::claims::{MentionRole, ResolutionStatus, extract_mentions};
    if !is_autofix_eligible_path(doc_rel_path) {
        return Ok(Vec::new());
    }
    let full = repo_root.join(doc_rel_path);
    let content =
        fs::read_to_string(&full).map_err(|e| format!("failed to read {}: {e}", full.display()))?;
    let mut out: Vec<Tier2ProseHealCandidate> = Vec::new();
    let mut seen: BTreeSet<(usize, String)> = BTreeSet::new();
    for m in extract_mentions(Path::new(doc_rel_path), &content) {
        let rm = registry.resolve_mention(&m);
        if rm.status != ResolutionStatus::Retired || rm.extracted.role != MentionRole::Prescribes {
            continue;
        }
        // Only a crate-name mention whose exact name is in the rename table is rewritten. Regex
        // retirements also match prefixes (`vox-dei-d`) and commands containing the name, and
        // some are ambiguous (`vox-dei`), so they are reported but never auto-fixed.
        if rm.extracted.kind != crate::claims::MentionKind::CrateName {
            continue;
        }
        let Some(replacement) = registry
            .exact_retired_crate_replacement(&rm.extracted.raw_text)
            .filter(|r| is_plain_replacement_token(r) && *r != rm.extracted.raw_text)
        else {
            continue;
        };
        if !seen.insert((rm.extracted.line_number, rm.extracted.raw_text.clone())) {
            continue;
        }
        let citation = crate::claims::self_heal_bot::contract_line_citation(
            repo_root,
            "contracts/documentation/retired-symbols.v1.yaml",
            &rm.extracted.normalized_target,
        )
        .or_else(|| {
            let manifest = format!("crates/{replacement}/Cargo.toml");
            repo_root
                .join(&manifest)
                .is_file()
                .then(|| format!("[{manifest}:L1-L1]"))
        });
        let Some(citation) = citation else { continue };
        if let Ok(c) = synthesize_line_scoped_fix(
            repo_root,
            doc_rel_path,
            rm.extracted.line_number,
            &rm.extracted.raw_text,
            replacement,
            &citation,
        ) {
            out.push(c);
        }
    }
    Ok(out)
}

/// Apply every gate-passing retired-symbol fix to `doc_rel_path`, one at a time, re-planning
/// after each write so overlapping fixes cannot clobber each other. Returns the applied
/// summaries. Fixes that fail any of the four gates are skipped, never forced.
pub fn apply_retired_symbol_fixes(
    repo_root: &Path,
    registry: &crate::claims::GroundTruthRegistry,
    doc_rel_path: &str,
) -> Result<Vec<String>, String> {
    let full = repo_root.join(doc_rel_path);
    let mut applied = Vec::new();
    let mut skipped_at: BTreeSet<String> = BTreeSet::new();
    for _ in 0..64 {
        let plans = plan_retired_symbol_fixes(repo_root, registry, doc_rel_path)?;
        let Some(next) = plans
            .into_iter()
            .find(|c| c.gate_report.passed && !skipped_at.contains(&c.summary))
        else {
            break;
        };
        if next.patched_markdown.is_empty() {
            skipped_at.insert(next.summary);
            continue;
        }
        fs::write(&full, &next.patched_markdown)
            .map_err(|e| format!("failed to write {}: {e}", full.display()))?;
        applied.push(next.summary);
    }
    Ok(applied)
}

pub fn synthesize_grounded_prose_patch(
    repo_root: &Path,
    doc_rel_path: &str,
    symbol_name: &str,
    new_fact: &str,
    evidence_citation: &str,
    draft_subsystem: Option<&str>,
) -> Result<Tier2ProseHealCandidate, String> {
    let full = repo_root.join(doc_rel_path);
    let original_md =
        fs::read_to_string(&full).map_err(|e| format!("failed to read {}: {e}", full.display()))?;
    let original_hash = hash_normalized_code(&original_md);
    let note_line =
        format!("- Verified contract (`{symbol_name}`): {new_fact} {evidence_citation}");

    let marker = "## Human Architecture Notes";
    let patched_markdown = if original_md.contains(&note_line) {
        original_md.clone()
    } else if let Some(pos) = original_md.find(marker) {
        let after = &original_md[pos + marker.len()..];
        let insert_at = pos + marker.len() + after.find('\n').map(|i| i + 1).unwrap_or(after.len());
        let mut out = String::with_capacity(original_md.len() + note_line.len() + 8);
        out.push_str(&original_md[..insert_at]);
        out.push('\n');
        out.push_str(&note_line);
        out.push('\n');
        out.push_str(&original_md[insert_at..]);
        out
    } else {
        let mut out = original_md.clone();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("\n{marker}\n\n{note_line}\n"));
        out
    };

    let evidence_citations = vec![evidence_citation.to_string()];
    let gate_report = verify_tier2_candidate_four_gates(
        repo_root,
        doc_rel_path,
        &original_md,
        &patched_markdown,
        &evidence_citations,
    );
    let unified_diff = render_unified_diff(&original_md, &patched_markdown, doc_rel_path);
    let summary =
        format!("Ground prose documentation for `{symbol_name}` against {evidence_citation}");

    let draft_pr = if gate_report.passed
        && let Some(subsystem) = draft_subsystem
        && (doc_rel_path.starts_with("docs/src/"))
    {
        let mut ledger = BotMetricsLedger::default();
        ledger
            .plan_or_update_draft_pr(&CandidateDocPatch {
                subsystem: subsystem.to_string(),
                target_doc_path: doc_rel_path.to_string(),
                updated_markdown: patched_markdown.clone(),
                evidence_citations: evidence_citations.clone(),
                summary: summary.clone(),
            })
            .map_err(|e| format!("draft PR rejected: {e:?}"))?
    } else {
        None
    };

    Ok(Tier2ProseHealCandidate {
        doc_path: doc_rel_path.to_string(),
        summary,
        unified_diff,
        original_hash,
        patched_markdown,
        evidence_citations,
        gate_report,
        draft_pr,
    })
}

/// Apply a verified heal patch safely under `repo_root/docs/`.
pub fn apply_heal_patch_to_file(
    repo_root: &Path,
    doc_rel_path: &str,
    patched_markdown: &str,
) -> Result<(), String> {
    let norm = doc_rel_path.trim().replace('\\', "/");
    if !norm.starts_with("docs/") || norm.contains("..") {
        return Err(format!("refusing out-of-scope doc write `{norm}`"));
    }
    let full = repo_root.join(&norm);
    fs::write(&full, patched_markdown)
        .map_err(|e| format!("failed to write {}: {e}", full.display()))
}

/// Demote a stale/unfixable documentation page to `status: "legacy"` in frontmatter (`WIKI-04`).
pub fn demote_doc_to_legacy(repo_root: &Path, doc_rel_path: &str) -> Result<String, String> {
    let norm = doc_rel_path.trim().replace('\\', "/");
    if !norm.starts_with("docs/") || norm.contains("..") {
        return Err(format!("refusing out-of-scope demotion `{norm}`"));
    }
    let full = repo_root.join(&norm);
    let raw =
        fs::read_to_string(&full).map_err(|e| format!("failed to read {}: {e}", full.display()))?;
    let updated = if let Some(rest) = raw.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        let fm = &rest[..end];
        let new_fm: String = if fm.lines().any(|l| l.trim_start().starts_with("status:")) {
            fm.lines()
                .map(|l| {
                    if l.trim_start().starts_with("status:") {
                        "status: \"legacy\"".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            format!("{fm}\nstatus: \"legacy\"")
        };
        format!("---\n{new_fm}{}", &rest[end..])
    } else {
        format!("---\nstatus: \"legacy\"\n---\n\n{raw}")
    };
    fs::write(&full, &updated).map_err(|e| format!("failed to write {}: {e}", full.display()))?;
    Ok(updated)
}

#[cfg(test)]
#[path = "openwiki_heal_tests.rs"]
mod tests;
