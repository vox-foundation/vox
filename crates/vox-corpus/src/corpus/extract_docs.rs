//! Markdown documentation corpus extractor for Mens training data.
//!
//! Walks `docs/src/**/*.md` and extracts fenced code blocks tagged ` ```vox `
//! as training pairs, plus section-level Q&A pairs from architecture docs.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::Context;
use chrono::{NaiveDate, Utc};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use vox_bounded_fs::read_utf8_path_capped;

static VOX_DOC_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[^\]]+\]\(([^)]+\.vox)\)").expect("vox doc link regex"));

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct Frontmatter {
    last_updated: Option<String>,
    title: Option<String>,
    status: Option<String>,
}

/// Configuration for documentation extraction.
#[derive(Debug, Clone)]
pub struct ExtractDocsConfig {
    /// Root directory to walk (usually `docs/src/`).
    pub root: PathBuf,
    /// Whether to extract fenced Vox code blocks.
    pub extract_code_blocks: bool,
    /// Whether to extract Q&A pairs from section headings.
    pub extract_qa_pairs: bool,
    /// Minimum section body length (chars) to extract a Q&A pair.
    pub min_section_chars: usize,
    /// Maximum number of pairs to emit (0 = unlimited).
    pub limit: usize,
}

impl Default for ExtractDocsConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("docs/src"),
            extract_code_blocks: true,
            extract_qa_pairs: true,
            min_section_chars: 100,
            limit: 0,
        }
    }
}

/// One extracted documentation training pair.
#[derive(Debug, Clone)]
pub struct DocTrainingPair {
    /// Source file path.
    pub source_path: PathBuf,
    /// Category tag.
    pub category: String,
    /// The prompt.
    pub prompt: String,
    /// The response.
    pub response: String,
    /// Quality rating.
    pub rating: u8,
    /// Estimated difficulty (3-10).
    pub difficulty: u8,
    /// Data lane (codegen vs docs qa).
    pub lane: String,
    /// Expected response surface for this row.
    pub response_mode: String,
    /// Task family for downstream segmentation.
    pub task_family: String,
    /// Additional traceability metadata for later retrieval or review.
    pub metadata: serde_json::Value,
}

impl DocTrainingPair {
    /// Serialize to JSONL.
    #[must_use]
    pub fn to_jsonl(&self) -> String {
        let v = json!({
            "prompt": self.prompt,
            "response": self.response,
            "category": self.category,
            "rating": self.rating,
            "difficulty": self.difficulty,
            "source": self.source_path.display().to_string(),
            "format": "documentation",
            "lane": self.lane,
            "response_mode": self.response_mode,
            "task_family": self.task_family,
            "metadata": self.metadata,
        });
        v.to_string()
    }
}

/// Parse YAML frontmatter to determine eligibility and staleness penalty.
/// Returns parsed metadata and staleness penalty where penalty increases with age (0-3 scale).
fn parse_frontmatter(content: &str, path: &Path) -> (Frontmatter, bool, u8) {
    // Explicit deprecation check acts as a hard short-circuit
    if content.contains("status: deprecated")
        || content.contains("status: \"deprecated\"")
        || content.contains("status: 'deprecated'")
    {
        return (Frontmatter::default(), false, 0);
    }

    // Fail closed (TAX-06): a typo such as `origin: web-promotd` must not make a page eligible.
    let facets = match vox_kb_facets::parse_facets(content) {
        Ok(f) => f,
        Err(_) => return (Frontmatter::default(), false, 0),
    };
    // One shared rule with the facet lint (D-06); flag absent stays eligible (D-05).
    if !vox_kb_facets::effective_training_eligible(&facets) {
        return (Frontmatter::default(), false, 0);
    }
    // Fail closed (WR-03): a misspelled eligibility key (`training_eligble: false`) is ignored
    // by the typed parse, so the author's intent must not default to "eligible".
    if vox_kb_facets::facet_key_near_misses(content)
        .iter()
        .any(vox_kb_facets::KeyNearMiss::guards_eligibility)
    {
        return (Frontmatter::default(), false, 0);
    }
    // No frontmatter at all: keep the pre-TAX conservative opt-out so a stray
    // `training_eligible: false` in a fence-less file (a crate README) still excludes it.
    if vox_kb_facets::frontmatter_yaml(content).is_none()
        && (content.contains("training_eligible: false")
            || content.contains("training_eligible:false"))
    {
        return (Frontmatter::default(), false, 0);
    }

    let fm: Frontmatter = match vox_kb_facets::frontmatter_yaml(content) {
        Some(y) => serde_yaml::from_str(y).unwrap_or_default(),
        None => Frontmatter {
            title: path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.replace(['-', '_'], " ")),
            ..Frontmatter::default()
        },
    };

    let mut penalty = 0;
    if let Some(ref date_str) = fm.last_updated
        && let Ok(last_updated) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
    {
        let now = Utc::now().date_naive();
        let days_old = now.signed_duration_since(last_updated).num_days();

        // Penalize by 1 for every 90 days (approx 3 months), max penalty of 3
        if days_old > 0 {
            penalty = (days_old / 90).min(3) as u8;
        }
    }

    (fm, true, penalty)
}

/// Extract training pairs from a single markdown file.
pub fn extract_from_md_file(
    path: &Path,
    config: &ExtractDocsConfig,
) -> anyhow::Result<Vec<DocTrainingPair>> {
    let source = read_utf8_path_capped(path)?;

    let (frontmatter, eligible, staleness_penalty) = parse_frontmatter(&source, path);
    if !eligible {
        return Ok(Vec::new());
    }

    // The page-one summary block is a rewrite target, never training text (TAX-06); strip it
    // before both lanes. For `.mdx` files, also strip ESM imports and Starlight JSX wrappers.
    let source = vox_kb_facets::strip_summary_blocks(&source);
    let source = if path.extension().is_some_and(|e| e == "mdx") {
        vox_kb_facets::strip_mdx_boilerplate(&source)
    } else {
        source
    };

    let mut pairs = Vec::new();

    if config.extract_code_blocks {
        extract_code_blocks(&source, path, &frontmatter, staleness_penalty, &mut pairs);
    }

    if config.extract_qa_pairs {
        extract_qa_sections(
            &source,
            path,
            &frontmatter,
            staleness_penalty,
            config,
            &mut pairs,
        );
    }

    if config.limit > 0 {
        pairs.truncate(config.limit);
    }

    Ok(pairs)
}

fn prepend_doc_context(
    title: Option<&str>,
    h1: &str,
    h2: &str,
    current: &str,
    level: usize,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let root_label = title.map(str::trim).filter(|s| !s.is_empty()).or_else(|| {
        let trimmed = h1.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    if let Some(root) = root_label
        && !root.eq_ignore_ascii_case(current.trim())
    {
        parts.push(root);
    }
    if level >= 3 {
        let h2_trimmed = h2.trim();
        if !h2_trimmed.is_empty()
            && !h2_trimmed.eq_ignore_ascii_case(current.trim())
            && !parts.iter().any(|p| p.eq_ignore_ascii_case(h2_trimmed))
        {
            parts.push(h2_trimmed);
        }
    }
    parts.join(" / ")
}

fn format_qa_prompt(
    title: Option<&str>,
    h1: &str,
    h2: &str,
    current: &str,
    level: usize,
) -> String {
    let breadcrumb = prepend_doc_context(title, h1, h2, current, level);
    if breadcrumb.is_empty() {
        format!("Explain the Vox concept: {}", current.trim())
    } else {
        format!("Explain the Vox concept ({breadcrumb}): {}", current.trim())
    }
}

fn frontend_check_passes(source: &str, path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    vox_compiler::pipeline::run_frontend_str(source, &path_str).is_ok_and(|res| {
        !res.diagnostics
            .iter()
            .any(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error)
    })
}

/// Extract fenced code blocks tagged with `vox` language.
fn extract_code_blocks(
    source: &str,
    path: &Path,
    frontmatter: &Frontmatter,
    staleness_penalty: u8,
    out: &mut Vec<DocTrainingPair>,
) {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    let mut h1_heading = String::new();
    let mut h2_heading = String::new();
    let mut preceding_context = String::new();
    let mut in_other_fence = false;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if in_other_fence {
            if trimmed == "```" {
                in_other_fence = false;
            }
            i += 1;
            continue;
        }

        let is_vox_open = trimmed == "```vox" || trimmed.starts_with("```vox ");
        if trimmed.starts_with("```") && !is_vox_open {
            in_other_fence = true;
            i += 1;
            continue;
        }

        // Track headings and paragraphs for context
        if trimmed.starts_with('#') {
            let current_level = trimmed.chars().take_while(|&c| c == '#').count();
            let current_heading = trimmed.trim_start_matches('#').trim().to_string();
            if current_level == 1 {
                h1_heading = current_heading.clone();
                h2_heading.clear();
            } else if current_level == 2 {
                h2_heading = current_heading.clone();
            }
            let breadcrumb = prepend_doc_context(
                frontmatter.title.as_deref(),
                &h1_heading,
                &h2_heading,
                &current_heading,
                current_level,
            );
            preceding_context = if breadcrumb.is_empty() {
                current_heading.clone()
            } else if current_heading.is_empty() {
                breadcrumb
            } else {
                format!("{breadcrumb} / {current_heading}")
            };
        } else if !trimmed.is_empty()
            && !trimmed.starts_with("```")
            && !trimmed.starts_with("---")
            && !trimmed.starts_with('>')
        {
            // Accumulate paragraph text (last paragraph before code block)
            if preceding_context.len() < 300 {
                if !preceding_context.is_empty() {
                    preceding_context.push(' ');
                }
                preceding_context.push_str(trimmed);
            }
        }

        // Detect ```vox code blocks
        if is_vox_open {
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() {
                if lines[i].trim() == "```" {
                    break;
                }
                code_lines.push(lines[i]);
                i += 1;
            }

            let code = code_lines.join("\n");
            if code.contains("{{#include")
                || code.contains("// vox:skip")
                || !crate::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(&code)
                    .is_empty()
            {
                preceding_context.clear();
                i += 1;
                continue;
            }
            if code.len() >= 20 {
                let vox_check_passed = frontend_check_passes(&code, path);
                let base_rating = if vox_check_passed { 5u8 } else { 3u8 };
                let suffix =
                    " Use valid Vox only: annotate `fn` with `to` return types and 4-space indent.";
                let prompt = if !preceding_context.is_empty() {
                    format!(
                        "Show me Vox code for: {}{}",
                        preceding_context.chars().take(300).collect::<String>(),
                        suffix
                    )
                } else if let Some(title) = frontmatter.title.as_deref().filter(|s| !s.is_empty()) {
                    format!("Show me a complete Vox example for `{title}`.{suffix}")
                } else {
                    format!("Write an example Vox program.{suffix}")
                };

                let mut metadata =
                    build_metadata(path, frontmatter, Some(&preceding_context), "code_block");
                if let Some(obj) = metadata.as_object_mut() {
                    obj.insert("vox_check_passed".to_string(), json!(vox_check_passed));
                }

                out.push(DocTrainingPair {
                    source_path: path.to_path_buf(),
                    category: "documentation".to_string(),
                    prompt,
                    response: code,
                    rating: base_rating.saturating_sub(staleness_penalty).max(1),
                    difficulty: 7, // Code blocks are higher difficulty than prose
                    lane: "vox_codegen".to_string(),
                    response_mode: "code_only".to_string(),
                    task_family: "docs_code".to_string(),
                    metadata,
                });
            }
            preceding_context.clear();
        }

        i += 1;
    }
}

fn resolve_doc_confine_root(config_root: &Path) -> Option<PathBuf> {
    let canon = std::fs::canonicalize(config_root).ok()?;
    for ancestor in canon.ancestors().take(3) {
        if ancestor.join("Cargo.toml").is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    Some(canon)
}

fn flush_qa_section(
    current_heading: &str,
    current_body: &str,
    heading_level: usize,
    h1_heading: &str,
    h2_heading: &str,
    path: &Path,
    frontmatter: &Frontmatter,
    staleness_penalty: u8,
    config: &ExtractDocsConfig,
    out: &mut Vec<DocTrainingPair>,
) {
    if current_heading.is_empty()
        || current_body.len() < config.min_section_chars
        || heading_level < 2
        || is_link_list(current_body)
    {
        return;
    }

    let prompt = format_qa_prompt(
        frontmatter.title.as_deref(),
        h1_heading,
        h2_heading,
        current_heading,
        heading_level,
    );
    let mut response = current_body.trim().to_string();

    // Relational Chunking: Inject linked .vox examples directly into the training response
    {
        let confine_root = resolve_doc_confine_root(&config.root);
        let mut extra_context = String::new();
        for cap in VOX_DOC_LINK_RE.captures_iter(&response) {
            let target_path_str = &cap[1];
            let abs_target = path.parent().unwrap_or(Path::new("")).join(target_path_str);
            if let Ok(can) = std::fs::canonicalize(&abs_target)
                && confine_root
                    .as_ref()
                    .is_some_and(|root| can.starts_with(root))
                && let Ok(vox_code) = read_utf8_path_capped(&can)
                && !vox_code.contains("// vox:skip")
                && crate::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(
                    &vox_code,
                )
                .is_empty()
            {
                extra_context.push_str("\n\n```vox\n");
                extra_context.push_str(vox_code.trim());
                extra_context.push_str("\n```\n");
            }
        }
        response.push_str(&extra_context);
    }

    let has_code_fences = response.contains("```");
    let response_mode = if has_code_fences {
        "mixed"
    } else {
        "prose_only"
    };
    let base_rating = if frontmatter.status.as_deref() == Some("current") {
        4u8
    } else {
        3u8
    };
    let breadcrumb = prepend_doc_context(
        frontmatter.title.as_deref(),
        h1_heading,
        h2_heading,
        current_heading,
        heading_level,
    );
    let mut metadata = build_metadata(path, frontmatter, Some(current_heading), "qa_section");
    if let Some(obj) = metadata.as_object_mut() {
        obj.insert("doc_breadcrumb".to_string(), json!(breadcrumb));
        obj.insert("has_code_fences".to_string(), json!(has_code_fences));
    }

    out.push(DocTrainingPair {
        source_path: path.to_path_buf(),
        category: "documentation".to_string(),
        prompt,
        response,
        rating: base_rating.saturating_sub(staleness_penalty).max(1),
        difficulty: 5, // Q&A prose is mid-difficulty
        lane: "vox_docs_qa".to_string(),
        response_mode: response_mode.to_string(),
        task_family: "docs_qa".to_string(),
        metadata,
    });
}

/// Extract Q&A pairs from markdown section headings.
fn extract_qa_sections(
    source: &str,
    path: &Path,
    frontmatter: &Frontmatter,
    staleness_penalty: u8,
    config: &ExtractDocsConfig,
    out: &mut Vec<DocTrainingPair>,
) {
    let lines: Vec<&str> = source.lines().collect();
    let mut h1_heading = String::new();
    let mut h2_heading = String::new();
    let mut current_heading = String::new();
    let mut current_body = String::new();
    let mut heading_level = 0usize;

    // Fence-aware accumulation. A fence carrying `// vox:skip` (or an
    // unresolved `{{#include}}`) is dropped from the section body, matching
    // what `extract_code_blocks` already does for the code lane. Without this
    // the QA lane shipped skip-marked fences verbatim into `vox_docs_qa` --
    // including retired syntax the author had explicitly marked to exclude.
    let mut in_fence = false;
    let mut fence_buf: Vec<&str> = Vec::new();
    // The opening fence verbatim, so the language tag survives re-emission.
    let mut fence_open = "```";

    for line in &lines {
        let trimmed = line.trim();

        // Fence open/close, tracked before anything else so a heading-looking
        // line inside a fence cannot split the section.
        if trimmed.starts_with("```") {
            if in_fence {
                let body = fence_buf.join("\n");
                let fence_tag = fence_open.trim_start_matches('`').trim();
                let is_vox_fence = fence_tag == "vox" || fence_tag.starts_with("vox ");
                let retired_hit = is_vox_fence
                    && !crate::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(
                        &body,
                    )
                    .is_empty();
                if !(body.contains("// vox:skip") || body.contains("{{#include") || retired_hit) {
                    current_body.push_str(fence_open);
                    current_body.push('\n');
                    current_body.push_str(&body);
                    current_body.push_str("\n```\n");
                }
                fence_buf.clear();
                in_fence = false;
            } else {
                in_fence = true;
                fence_open = trimmed;
            }
            continue;
        }
        if in_fence {
            fence_buf.push(line);
            continue;
        }

        // New heading
        if trimmed.starts_with('#') {
            flush_qa_section(
                &current_heading,
                &current_body,
                heading_level,
                &h1_heading,
                &h2_heading,
                path,
                frontmatter,
                staleness_penalty,
                config,
                out,
            );

            heading_level = trimmed.chars().take_while(|&c| c == '#').count();
            current_heading = trimmed.trim_start_matches('#').trim().to_string();
            if heading_level == 1 {
                h1_heading = current_heading.clone();
                h2_heading.clear();
            } else if heading_level == 2 {
                h2_heading = current_heading.clone();
            }
            current_body.clear();
        } else if !trimmed.is_empty() {
            current_body.push_str(trimmed);
            current_body.push('\n');
        }
    }

    flush_qa_section(
        &current_heading,
        &current_body,
        heading_level,
        &h1_heading,
        &h2_heading,
        path,
        frontmatter,
        staleness_penalty,
        config,
        out,
    );
}

fn build_metadata(
    path: &Path,
    frontmatter: &Frontmatter,
    heading: Option<&str>,
    chunk_kind: &str,
) -> serde_json::Value {
    let heading = heading
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);
    json!({
        "doc_path": normalize_path(path),
        "canonical_path": normalize_path(path),
        "doc_title": frontmatter.title.clone(),
        "doc_status": frontmatter.status.clone(),
        "last_updated": frontmatter.last_updated.clone(),
        "heading": heading,
        "heading_slug": heading.as_deref().map(slugify_heading),
        "chunk_kind": chunk_kind,
        "source_kind": "documentation",
    })
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn slugify_heading(s: &str) -> String {
    let mut slug = String::with_capacity(s.len());
    let mut prev_dash = false;
    for ch in s.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

/// Walk a directory tree and extract pairs from all `.md` files.
///
/// Task 3.3: Automate dynamic inclusion of internal workspace crate documentation.
pub fn walk_and_extract_docs(config: &ExtractDocsConfig) -> anyhow::Result<Vec<DocTrainingPair>> {
    let mut all = Vec::new();
    walk_docs_dir(&config.root, config, &mut all)?;

    // Automated workspace discovery: find READMEs and /docs in sibling `crates/`
    if let Some(parent) = config.root.parent()
        && (parent.ends_with("docs") || parent.join("crates").is_dir())
    {
        let crates_dir = if parent.ends_with("docs") {
            parent.parent().unwrap_or(Path::new(".")).join("crates")
        } else {
            parent.join("crates")
        };

        if let Ok(entries) = std::fs::read_dir(&crates_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    // Try README.md
                    let readme = p.join("README.md");
                    if readme.is_file()
                        && let Ok(pairs) = extract_from_md_file(&readme, config)
                    {
                        all.extend(pairs);
                    }
                    // Try docs/ directory inside crate
                    let crate_docs = p.join("docs");
                    if crate_docs.is_dir() {
                        walk_docs_dir(&crate_docs, config, &mut all)?;
                    }
                }
            }
        }
    }

    Ok(all)
}

/// True when most non-empty lines are markdown links (an index/"see also" list).
/// Such sections have no explanatory content to learn from.
fn is_link_list(body: &str) -> bool {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return false;
    }
    let links = lines
        .iter()
        .filter(|l| {
            let l = l.trim_start_matches(['-', '*', '/', ' ']);
            l.starts_with('[') && l.contains("](")
        })
        .count();
    links * 2 > lines.len()
}

fn walk_docs_dir(
    dir: &Path,
    config: &ExtractDocsConfig,
    out: &mut Vec<DocTrainingPair>,
) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Tombstoned (AGENTS.md §Archival Protocol): never training data.
            if path.file_name().is_some_and(|n| n == "archive") {
                continue;
            }
            walk_docs_dir(&path, config, out)?;
        } else if path.extension().is_some_and(|e| e == "md" || e == "mdx") {
            match extract_from_md_file(&path, config) {
                Ok(mut pairs) => {
                    if config.limit > 0 {
                        let remaining = config.limit.saturating_sub(out.len());
                        pairs.truncate(remaining);
                    }
                    out.extend(pairs);
                    if config.limit > 0 && out.len() >= config.limit {
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("  [doc extract] skip {}: {e}", path.display());
                }
            }
        }
    }
    Ok(())
}

/// Write extracted doc pairs to a JSONL file.
pub fn write_docs_to_jsonl(pairs: &[DocTrainingPair], output: &Path) -> anyhow::Result<usize> {
    use std::io::Write;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output)
        .with_context(|| format!("open output {}", output.display()))?;
    for pair in pairs {
        writeln!(f, "{}", pair.to_jsonl())?;
    }
    Ok(pairs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fence the author marked `// vox:skip` must not reach the QA lane.
    ///
    /// The code lane already filters these (see `extract_code_blocks`), but the
    /// QA lane accumulated every non-empty line including fenced blocks. The
    /// result: `docs/src/architecture/wire-format-v1-ssot.md` is
    /// `training_eligible: true` and carries six `@endpoint(kind: ...)`
    /// occurrences -- syntax that became a hard parse error on 2026-06-30 --
    /// inside `vox:skip`-marked fences, and shipped all of them verbatim into
    /// `vox_docs_qa`. The marker the author wrote to keep it out of training
    /// data did nothing in this lane.
    const SKIP_FENCE_MD: &str = r#"# Wire Format

## Endpoint Shape

The v1 wire format describes each route in terms of its declaration. This
paragraph is long enough to clear the minimum section length so the section
is actually emitted as a training pair by the extractor under test.

```vox
// vox:skip — illustrative endpoint definition
@endpoint(kind: query) fn get_user(id: int) -> User {
    return db.User.get(id)
}
```

More prose after the fence, also part of the section body.
"#;

    #[test]
    fn qa_prompts_do_not_demand_arrow_returns_and_link_lists_are_skipped() {
        const MD: &str = r#"# Doc

## Returns

Functions declare their return type with `to`, for example a function that
adds two integers returns an int. This section is long enough to be emitted.

## See also

- [one.md](one.md)
- [two.md](two.md)
- [three.md](three.md)
"#;
        let mut out = Vec::new();
        let config = ExtractDocsConfig {
            min_section_chars: 40,
            ..ExtractDocsConfig::default()
        };
        extract_qa_sections(
            MD,
            Path::new("d.md"),
            &Frontmatter::default(),
            0,
            &config,
            &mut out,
        );
        assert_eq!(out.len(), 1, "only the prose section should be emitted");
        assert_eq!(out[0].prompt, "Explain the Vox concept (Doc): Returns");
        assert_eq!(out[0].response_mode, "prose_only");
        assert!(!out.iter().any(|p| p.prompt.contains("->")));
    }

    #[test]
    fn qa_prompt_tracks_frontmatter_title_breadcrumbs_and_mixed_mode() {
        const MD: &str = r#"# Ignored H1

## Architecture Overview

### Context

This section contains both prose explanation and an inline code fence so the extractor tags it as mixed rather than prose_only.

```vox
fn helper(x: int) to int {
    return x + 1
}
```
"#;
        let mut out = Vec::new();
        let config = ExtractDocsConfig {
            min_section_chars: 40,
            ..ExtractDocsConfig::default()
        };
        let fm = Frontmatter {
            title: Some("Scheduler Spec".to_string()),
            status: Some("current".to_string()),
            ..Frontmatter::default()
        };
        extract_qa_sections(MD, Path::new("spec.md"), &fm, 0, &config, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].prompt,
            "Explain the Vox concept (Scheduler Spec / Architecture Overview): Context"
        );
        assert_eq!(out[0].response_mode, "mixed");
        assert_eq!(out[0].rating, 4);
        assert_eq!(
            out[0].metadata["doc_breadcrumb"],
            "Scheduler Spec / Architecture Overview"
        );
    }

    #[test]
    fn code_blocks_skip_retired_syntax_and_annotate_compiler_check() {
        const MD: &str = r#"# Guide

## Retired Block

```vox
@component fn OldWidget() -> Element {
    return <div>hi</div>
}
```

## Valid Function

A minimal pure helper that adds two integers cleanly in Vox.

```vox
fn add_two(a: int, b: int) to int {
    return a + b
}
```
"#;
        let mut out = Vec::new();
        let fm = Frontmatter {
            title: Some("Functions".to_string()),
            ..Frontmatter::default()
        };
        extract_code_blocks(MD, Path::new("fn.md"), &fm, 0, &mut out);
        assert_eq!(out.len(), 1, "retired @component fence must be dropped");
        assert!(out[0].prompt.contains("Functions / Valid Function"));
        assert!(out[0].prompt.contains("with `to` return types"));
        assert_eq!(out[0].metadata["vox_check_passed"], json!(true));
        assert_eq!(out[0].rating, 5);
    }

    #[test]
    fn link_list_detection() {
        assert!(is_link_list("- [a](a.md)\n- [b](b.md)\n"));
        assert!(!is_link_list("Prose line.\n- [a](a.md)\nMore prose.\n"));
    }

    #[test]
    fn qa_lane_excludes_skip_marked_fences() {
        let mut out = Vec::new();
        let config = ExtractDocsConfig {
            min_section_chars: 40,
            ..ExtractDocsConfig::default()
        };
        extract_qa_sections(
            SKIP_FENCE_MD,
            Path::new("wire-format.md"),
            &Frontmatter::default(),
            0,
            &config,
            &mut out,
        );
        assert!(!out.is_empty(), "the section should still be extracted");
        let body = &out[0].response;
        assert!(
            !body.contains("@endpoint"),
            "a vox:skip fence must not reach the QA lane; got: {body}"
        );
        assert!(
            body.contains("wire format describes"),
            "surrounding prose must survive; got: {body}"
        );
        assert!(
            body.contains("More prose after the fence"),
            "prose after the fence must survive; got: {body}"
        );
    }

    #[test]
    fn qa_lane_keeps_unmarked_fences() {
        // Only skip-marked fences are dropped. An ordinary example is still
        // legitimate Q&A context and must survive.
        const OK_MD: &str = r#"# Actors

## Counter

This section explains the counter actor and is long enough to be emitted as
a training pair by the extractor under test without being filtered out.

```vox
actor Counter {
    state count: int = 0
}
```
"#;
        let mut out = Vec::new();
        let config = ExtractDocsConfig {
            min_section_chars: 40,
            ..ExtractDocsConfig::default()
        };
        extract_qa_sections(
            OK_MD,
            Path::new("actors.md"),
            &Frontmatter::default(),
            0,
            &config,
            &mut out,
        );
        assert!(!out.is_empty());
        assert!(
            out[0].response.contains("actor Counter"),
            "unmarked fences must survive: {}",
            out[0].response
        );
    }

    const SAMPLE_MD: &str = r#"# Vox Actors

## Actor Model

Vox actors are isolated entities with mailbox-based message passing.
Each actor has its own state and handles messages sequentially.
This prevents data races without explicit locks.

```vox
actor Counter {
    state count: int = 0

    on Increment() to int {
        count = count + 1
        return count
    }
}
```

## Workflows

Durable execution is a first-class feature.
"#;

    #[test]
    fn extracts_vox_code_block() {
        let config = ExtractDocsConfig::default();
        let _pairs = extract_from_md_file(Path::new("test.md"), &config);
        // Can't test with real file, test the extraction logic directly
        let mut out = Vec::new();
        extract_code_blocks(
            SAMPLE_MD,
            Path::new("test.md"),
            &Frontmatter::default(),
            0,
            &mut out,
        );
        assert!(!out.is_empty(), "should extract vox code block");
        assert!(out[0].response.contains("actor Counter"));
        assert_eq!(out[0].metadata["chunk_kind"], "code_block");
    }

    #[test]
    fn extracts_qa_sections() {
        let config = ExtractDocsConfig {
            min_section_chars: 50,
            ..Default::default()
        };
        let mut out = Vec::new();
        extract_qa_sections(
            SAMPLE_MD,
            Path::new("test.md"),
            &Frontmatter::default(),
            0,
            &config,
            &mut out,
        );
        assert!(!out.is_empty(), "should extract at least one Q&A pair");
        assert!(out[0].prompt.contains("Actor Model"));
        assert_eq!(out[0].metadata["chunk_kind"], "qa_section");
    }

    #[test]
    fn ignores_mdbook_include_directives() {
        const MD_WITH_INCLUDE: &str = r#"# Tutorial
Check out this code:

```vox
{{#include ../../../examples/golden/getting_started.vox:logic}}
```
"#;
        let mut out = Vec::new();
        extract_code_blocks(
            MD_WITH_INCLUDE,
            Path::new("test.md"),
            &Frontmatter::default(),
            0,
            &mut out,
        );
        assert!(
            out.is_empty(),
            "should ignore code block containing mdbook include"
        );
    }

    #[test]
    fn extract_from_mdx_file_strips_jsx_and_summary() {
        let dir = tempfile::tempdir().unwrap();
        let mdx_path = dir.path().join("guide.mdx");
        let mdx = r#"---
title: "MDX Guide"
authorship: "human"
training_eligible: true
---
import { Steps, Tabs, TabItem } from '@astrojs/starlight/components';

# MDX Guide

{/* kb:summary */}
SECRET_SUMMARY_TOKEN that must never leak into MENS training rows.
{/* /kb:summary */}

## Full-Stack Steps

<Steps>
1. Define a typed table and query in Vox so schema and backend handlers stay synchronized across SQL and Rust.
</Steps>

```vox
fn compute_total(a: int, b: int) to int {
    return a + b
}
```
"#;
        std::fs::write(&mdx_path, mdx).unwrap();
        let config = ExtractDocsConfig {
            root: dir.path().to_path_buf(),
            min_section_chars: 40,
            ..Default::default()
        };
        let pairs = walk_and_extract_docs(&config).unwrap();
        assert!(!pairs.is_empty(), "expected training pairs from .mdx file");
        for p in &pairs {
            assert!(!p.response.contains("SECRET_SUMMARY_TOKEN"));
            assert!(!p.response.contains("<Steps>"));
            assert!(!p.response.contains("@astrojs/starlight/components"));
        }
    }

    #[test]
    fn relational_chunking_skips_retired_linked_vox_file() {
        let dir = tempfile::tempdir().unwrap();
        let examples_dir = dir.path().join("examples");
        std::fs::create_dir_all(&examples_dir).unwrap();
        std::fs::write(
            examples_dir.join("retired.vox"),
            "@component fn LegacyPage() to Element {\n    return <div></div>\n}\n",
        )
        .unwrap();
        std::fs::write(
            examples_dir.join("clean.vox"),
            "fn healthy_add(a: int, b: int) to int {\n    return a + b\n}\n",
        )
        .unwrap();

        let doc_path = dir.path().join("guide.md");
        let md = "---\ntitle: \"Guide\"\nstatus: \"current\"\ntraining_eligible: true\n---\n\n# Guide\n\n## Examples\n\nCompare [retired](examples/retired.vox) and [clean](examples/clean.vox) while keeping at least sixty characters of explanation.\n";
        std::fs::write(&doc_path, md).unwrap();

        let config = ExtractDocsConfig {
            root: dir.path().to_path_buf(),
            min_section_chars: 40,
            ..Default::default()
        };
        let pairs = walk_and_extract_docs(&config).unwrap();
        let qa_pair = pairs
            .iter()
            .find(|p| p.task_family == "docs_qa")
            .expect("expected docs_qa pair");
        assert!(
            qa_pair.response.contains("fn healthy_add"),
            "clean linked .vox file should be injected"
        );
        assert!(
            !qa_pair.response.contains("LegacyPage"),
            "retired linked .vox file must NOT be injected"
        );
    }

    #[test]
    fn code_blocks_ignore_hash_comments_inside_other_fences() {
        const MD: &str = r#"# Guide

## Real Section

```bash
# not a markdown heading
echo hi
```

```vox
fn ok_func(x: int) to int {
    return x + 1
}
```
"#;
        let mut out = Vec::new();
        extract_code_blocks(
            MD,
            Path::new("guide.md"),
            &Frontmatter::default(),
            0,
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert!(!out[0].prompt.contains("not a markdown heading"));
        assert!(out[0].prompt.contains("Real Section"));
    }

    #[test]
    fn qa_sections_drop_retired_inline_vox_fence_and_reject_traversal_link() {
        let outer = tempfile::tempdir().unwrap();
        let docs_root = outer.path().join("docs");
        std::fs::create_dir_all(&docs_root).unwrap();
        std::fs::write(
            outer.path().join("outside.vox"),
            "fn outside_secret(x: int) to int {\n    return x\n}\n",
        )
        .unwrap();

        let md = "---\ntitle: \"Security\"\nstatus: \"current\"\n---\n\n# Security\n\n## Traversal And Legacy\n\nExplanation exceeding minimum section characters with [outside](../outside.vox) link.\n\n```vox\n@component fn LegacyInline() to Element {\n    return <div></div>\n}\n```\n";
        std::fs::write(docs_root.join("sec.md"), md).unwrap();

        let config = ExtractDocsConfig {
            root: docs_root,
            extract_code_blocks: false,
            min_section_chars: 20,
            ..Default::default()
        };
        let pairs = walk_and_extract_docs(&config).unwrap();
        assert_eq!(pairs.len(), 1);
        assert!(
            !pairs[0].response.contains("LegacyInline"),
            "retired inline ```vox fence must be dropped from QA section"
        );
        assert!(
            !pairs[0].response.contains("outside_secret"),
            "traversal link outside confine root must not be injected"
        );
    }
}

#[cfg(test)]
#[path = "extract_docs_facet_tests.rs"]
mod facet_tests;
