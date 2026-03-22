//! Generate and verify [`doc-inventory.json`](../../docs/agents/doc-inventory.json) (schema v3).
//!
//! Replaces the retired Python inventory scripts; verify via `vox ci doc-inventory verify`.

use anyhow::{Context, Result, anyhow};
use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

static ITEM_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:pub\s+)?(?:struct|enum|fn|trait|type|mod|impl)\b|^\s*pub\s+\w+\s*:\s*")
        .expect("ITEM_LINE_RE")
});

const HOTSPOT_TIER1: &[&str] = &[
    "AGENTS.md",
    "docs/src/api/DOC_GAPS.md",
    "docs/src/api/vox-ast.md",
    "crates/vox-ast/src/expr.rs",
    "crates/vox-cli/src/lib.rs",
    "crates/vox-hir/src/hir.rs",
    "crates/vox-mcp/src/memory.rs",
    "crates/vox-orchestrator/src/events.rs",
    "crates/vox-orchestrator/src/oplog.rs",
    "crates/vox-orchestrator/src/orchestrator.rs",
    "crates/vox-orchestrator/src/session.rs",
    "crates/vox-orchestrator/src/types.rs",
    "crates/vox-pm/src/store/types.rs",
    "crates/vox-populi/src/tensor/qlora_preflight.rs",
    "crates/vox-mcp/src/tools/input_schemas.rs",
    "docs/src/ci/rust-modernization-baseline.md",
];

const HOTSPOT_TIER2_RUST: &[&str] = &[
    "crates/vox-populi/src/tensor/lora.rs",
    "crates/vox-cli/src/commands/populi/mod.rs",
    "crates/vox-orchestrator/src/memory.rs",
    "crates/vox-db/src/lib.rs",
    "crates/vox-forge/src/types.rs",
    "crates/vox-pm/src/store/ops.rs",
    "crates/vox-codegen-rust/src/emit.rs",
    "crates/vox-dei/src/research/orchestrator.rs",
    "crates/vox-gamify/src/db.rs",
    "crates/vox-mcp/src/tools/chat_tools.rs",
    "crates/vox-dei/src/selection.rs",
    "crates/vox-db/src/schema_digest.rs",
    "crates/vox-cli/src/cli_actions.rs",
    "crates/vox-ast/src/decl/mod.rs",
    "crates/vox-orchestrator/src/compaction.rs",
    "crates/vox-lexer/src/token.rs",
    "crates/vox-mcp/src/params.rs",
    "crates/vox-orchestrator/src/config.rs",
    "crates/vox-orchestrator/src/jj_backend.rs",
    "crates/vox-orchestrator/src/snapshot.rs",
];

const SYMBOL_HINT_PATHS: &[&str] = &[
    "crates/vox-ast/src/expr.rs",
    "crates/vox-cli/src/lib.rs",
    "crates/vox-hir/src/hir.rs",
    "crates/vox-mcp/src/memory.rs",
    "crates/vox-orchestrator/src/events.rs",
    "crates/vox-orchestrator/src/oplog.rs",
    "crates/vox-orchestrator/src/orchestrator.rs",
    "crates/vox-orchestrator/src/session.rs",
    "crates/vox-orchestrator/src/types.rs",
    "crates/vox-pm/src/store/types.rs",
    "crates/vox-populi/src/tensor/qlora_preflight.rs",
];

const SKIP_DIR_NAMES: &[&str] = &[
    "target",
    ".git",
    ".venv",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
];

const INVENTORY_DESCRIPTION: &str = "Per-file comment/doc counts for LLM batch targeting. Rust: lines_triple_slash=///, lines_inner_doc=//!, lines_plain_comment=// excluding doc. Markdown: lines_total=all lines; lines_other_doc_signal=# heading count. hotspot_tier: 1=plan-listed path, 2=high-density heuristic, 0=other. symbol_hints (schema v3): plan-hotspot Rust only; /// or //! linked to next item; containing_symbol, doc_preview, comment_type, quality_tag (mechanical|operational|section_divider|user_help|narrative|ssot_sensitive).";

/// Default output path relative to repository root.
pub const DEFAULT_INVENTORY_PATH: &str = "docs/agents/doc-inventory.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub kind: String,
    pub lines_total: u64,
    pub lines_triple_slash: u64,
    pub lines_inner_doc: u64,
    pub lines_plain_comment: u64,
    pub lines_other_doc_signal: u64,
    pub hotspot_tier: i32,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHint {
    pub doc_line: usize,
    pub item_line: usize,
    pub item_preview: String,
    pub containing_symbol: Option<serde_json::Value>,
    pub doc_preview: String,
    pub comment_type: String,
    pub quality_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHintGroup {
    pub path: String,
    pub hints: Vec<SymbolHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocInventory {
    pub schema_version: i32,
    pub generated_at: String,
    pub description: String,
    pub first_read_for_agents: Vec<String>,
    pub files: Vec<FileEntry>,
    pub symbol_hints: Vec<SymbolHintGroup>,
}

fn tier1_set() -> HashSet<&'static str> {
    HOTSPOT_TIER1.iter().copied().collect()
}

fn tier2_rust_set() -> HashSet<&'static str> {
    HOTSPOT_TIER2_RUST.iter().copied().collect()
}

fn should_skip_path(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        SKIP_DIR_NAMES.contains(&s.as_ref())
    })
}

fn iter_repo_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut found: HashSet<String> = HashSet::new();
    let singles = [root.join("AGENTS.md")];
    for s in singles {
        if s.is_file() {
            found.insert(s.strip_prefix(root)?.to_string_lossy().replace('\\', "/"));
        }
    }
    let roots = [
        root.join("crates"),
        root.join("docs"),
        root.join("vox-vscode"),
        root.join("scripts"),
        root.join(".github").join("workflows"),
    ];
    for base in roots {
        if !base.is_dir() {
            continue;
        }
        for e in WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
            let p = e.path();
            if should_skip_path(p) {
                continue;
            }
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
            if matches!(ext, "rs" | "md" | "ts" | "yml" | "yaml" | "sh" | "py") {
                let rel = p.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
                found.insert(rel);
            }
        }
    }
    let mut v: Vec<String> = found.into_iter().collect();
    v.sort();
    Ok(v.into_iter().map(|r| root.join(&r)).collect())
}

fn file_kind(rel: &str) -> &'static str {
    if rel.ends_with(".rs") {
        "rust"
    } else if rel.ends_with(".md") {
        "markdown"
    } else if rel.ends_with(".ts") {
        "typescript"
    } else if rel.ends_with(".yml") || rel.ends_with(".yaml") {
        "yaml"
    } else if rel.ends_with(".sh") {
        "shell"
    } else if rel.ends_with(".py") {
        "python"
    } else {
        "other"
    }
}

fn notes_for(kind: &str) -> &'static str {
    match kind {
        "rust" => "rustdoc + inline //",
        "markdown" => "prose doc; lines_other_doc_signal=heading count",
        "typescript" => "approx // and block comments",
        "yaml" => "comments + structure",
        "shell" => "comments",
        "python" => "docstrings + # (heuristic)",
        _ => "",
    }
}

fn count_rust_lines(text: &str) -> (u64, u64, u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let n_total = lines.len() as u64;
    let mut triple = 0u64;
    let mut inner = 0u64;
    let mut plain = 0u64;
    for line in &lines {
        let s = line.trim_start();
        if s.starts_with("//!") {
            inner += 1;
        } else if s.starts_with("///") {
            triple += 1;
        } else if let Some(idx) = line.find("//") {
            let before = &line[..idx];
            if before.contains('"') || before.contains('\'') {
                continue;
            }
            let rest = &line[idx + 2..];
            if rest.starts_with('/') || rest.is_empty() {
                continue;
            }
            plain += 1;
        }
    }
    (n_total, triple, inner, plain)
}

fn count_md(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let headings = lines
        .iter()
        .filter(|l| l.trim_start().starts_with('#'))
        .count() as u64;
    (lines.len() as u64, headings)
}

fn count_ts(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let mut plain = 0u64;
    for line in &lines {
        if let Some(idx) = line.find("//") {
            let before = &line[..idx];
            if before.contains('"') || before.contains('\'') {
                continue;
            }
            let rest = &line[idx + 2..];
            if rest.starts_with('/') {
                continue;
            }
            plain += 1;
        }
    }
    (lines.len() as u64, plain)
}

fn count_shell(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let plain = lines
        .iter()
        .filter(|l| l.trim_start().starts_with('#'))
        .count() as u64;
    (lines.len() as u64, plain)
}

fn count_python(text: &str) -> (u64, u64) {
    let lines: Vec<&str> = text.lines().collect();
    let plain = lines
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('#') && !t.starts_with("#!")
        })
        .count() as u64;
    (lines.len() as u64, plain)
}

fn classify_quality(doc_text: &str) -> &'static str {
    let u = doc_text.to_uppercase();
    if u.contains("SSOT")
        || u.contains("ADR ")
        || u.contains(" ADR")
        || u.contains("INVARIANT")
        || u.contains("CONTRACT")
        || u.contains("SOURCE OF TRUTH")
    {
        "ssot_sensitive"
    } else if doc_text.trim().len() < 60 {
        "mechanical"
    } else {
        "narrative"
    }
}

fn skip_attrs_empty(lines: &[&str], start: usize) -> usize {
    let mut i = start;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        if t.starts_with("#[") || t.starts_with("#![") {
            i += 1;
            continue;
        }
        break;
    }
    i
}

fn find_item_after(lines: &[&str], start: usize) -> (Option<usize>, String) {
    let i = skip_attrs_empty(lines, start);
    if i >= lines.len() {
        return (None, String::new());
    }
    let preview = lines[i].trim();
    let preview_short: String = preview.chars().take(200).collect();
    let _ = ITEM_LINE_RE.is_match(lines[i]);
    (Some(i + 1), preview_short)
}

pub(crate) fn rust_symbol_hints(text: &str) -> Vec<SymbolHint> {
    let owned: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    let lines: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let mut hints = Vec::new();
    let mut i = 0usize;
    let n = lines.len();
    while i < n {
        let stripped = lines[i].trim_start();
        if stripped.starts_with("//!") {
            let block_start = i;
            while i < n && lines[i].trim_start().starts_with("//!") {
                i += 1;
            }
            let (il, preview) = find_item_after(&lines, i);
            let item_line = il.unwrap_or_else(|| (i + 1).min(n));
            for (off, line) in lines[block_start..i].iter().enumerate() {
                let dl = block_start + off;
                let doc_body = line.trim_start();
                let doc_body = doc_body.strip_prefix("//!").unwrap_or(doc_body).trim();
                hints.push(SymbolHint {
                    doc_line: dl + 1,
                    item_line,
                    item_preview: preview.clone(),
                    containing_symbol: None,
                    doc_preview: doc_body.to_string(),
                    comment_type: "inner_doc".into(),
                    quality_tag: classify_quality(doc_body).to_string(),
                });
            }
            continue;
        }
        if stripped.starts_with("///") && !stripped.starts_with("////") {
            let block_start = i;
            while i < n && lines[i].trim_start().starts_with("///") {
                i += 1;
            }
            let (il, preview) = find_item_after(&lines, i);
            let item_line = il.unwrap_or_else(|| (i + 1).min(n));
            for (off, line) in lines[block_start..i].iter().enumerate() {
                let dl = block_start + off;
                let doc_body = line.trim_start();
                let doc_body = doc_body.strip_prefix("///").unwrap_or(doc_body).trim();
                hints.push(SymbolHint {
                    doc_line: dl + 1,
                    item_line,
                    item_preview: preview.clone(),
                    containing_symbol: None,
                    doc_preview: doc_body.to_string(),
                    comment_type: "outer_doc".into(),
                    quality_tag: classify_quality(doc_body).to_string(),
                });
            }
            continue;
        }
        i += 1;
    }
    hints
}

fn hotspot_tier(
    rel: &str,
    kind: &str,
    lines_plain_comment: u64,
    lines_other_doc_signal: u64,
) -> i32 {
    let t1 = tier1_set();
    let t2 = tier2_rust_set();
    if t1.contains(rel) {
        return 1;
    }
    if kind == "rust" && t2.contains(rel) {
        return 2;
    }
    if kind == "typescript" && lines_plain_comment >= 15 {
        return 2;
    }
    if kind == "markdown" && lines_other_doc_signal >= 8 {
        return 2;
    }
    if rel == "crates/vox-doc-inventory/src/lib.rs" {
        return 2;
    }
    0
}

fn build_file_entry(root: &Path, rel: &str) -> Result<FileEntry> {
    let path = root.join(rel);
    let kind = file_kind(rel);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut entry = FileEntry {
        path: rel.to_string(),
        kind: kind.to_string(),
        lines_total: 0,
        lines_triple_slash: 0,
        lines_inner_doc: 0,
        lines_plain_comment: 0,
        lines_other_doc_signal: 0,
        hotspot_tier: 0,
        notes: notes_for(kind).to_string(),
    };
    match kind {
        "rust" => {
            let (t, tr, inn, pl) = count_rust_lines(&text);
            entry.lines_total = t;
            entry.lines_triple_slash = tr;
            entry.lines_inner_doc = inn;
            entry.lines_plain_comment = pl;
        }
        "markdown" => {
            let (tot, h) = count_md(&text);
            entry.lines_total = tot;
            entry.lines_other_doc_signal = h;
        }
        "typescript" => {
            let (tot, pl) = count_ts(&text);
            entry.lines_total = tot;
            entry.lines_plain_comment = pl;
        }
        "yaml" => {
            let (tot, pl) = count_python(&text);
            entry.lines_total = tot;
            entry.lines_plain_comment = pl;
        }
        "shell" => {
            let (tot, pl) = count_shell(&text);
            entry.lines_total = tot;
            entry.lines_plain_comment = pl;
        }
        "python" => {
            let (tot, pl) = count_python(&text);
            entry.lines_total = tot;
            entry.lines_plain_comment = pl;
        }
        _ => {
            entry.lines_total = text.lines().count() as u64;
        }
    }
    entry.hotspot_tier = hotspot_tier(
        rel,
        kind,
        entry.lines_plain_comment,
        entry.lines_other_doc_signal,
    );
    Ok(entry)
}

/// Generate inventory JSON at `out_path` (default: `docs/agents/doc-inventory.json`).
pub fn generate(root: &Path, out_path: &Path) -> Result<DocInventory> {
    let paths = iter_repo_files(root)?;
    let mut files: Vec<FileEntry> = paths
        .iter()
        .map(|p| {
            let rel = p.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            build_file_entry(root, &rel)
        })
        .collect::<Result<Vec<_>>>()?;
    files.sort_by(|a, b| {
        let ta = a.hotspot_tier;
        let tb = b.hotspot_tier;
        let da = a.lines_triple_slash + a.lines_inner_doc;
        let db = b.lines_triple_slash + b.lines_inner_doc;
        tb.cmp(&ta)
            .then_with(|| (db as i64).cmp(&(da as i64)))
            .then_with(|| (b.lines_total as i64).cmp(&(a.lines_total as i64)))
            .then_with(|| a.path.cmp(&b.path))
    });

    let mut symbol_hints: Vec<SymbolHintGroup> = Vec::new();
    for rel in SYMBOL_HINT_PATHS {
        let p = root.join(rel);
        if !p.is_file() {
            continue;
        }
        let text = fs::read_to_string(&p).unwrap_or_default();
        let hints = rust_symbol_hints(&text);
        if !hints.is_empty() {
            symbol_hints.push(SymbolHintGroup {
                path: (*rel).to_string(),
                hints,
            });
        }
    }

    let inv = DocInventory {
        schema_version: 3,
        generated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        description: INVENTORY_DESCRIPTION.to_string(),
        first_read_for_agents: vec![
            "AGENTS.md".into(),
            "docs/agents/orchestrator.md".into(),
            "docs/src/ref-cli.md".into(),
            "crates/vox-mcp/src/tools/mod.rs".into(),
            "docs/agents/doc-inventory.json".into(),
        ],
        files,
        symbol_hints,
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&inv)? + "\n";
    fs::write(out_path, json)?;
    Ok(inv)
}

/// Strip `generated_at` for drift comparison.
pub fn strip_generated_at(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("generated_at");
    }
    v
}

pub(crate) fn normalize_json_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map
                .into_iter()
                .map(|(k, v)| (k, normalize_json_value(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().collect())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(normalize_json_value).collect()),
        other => other,
    }
}

/// Verify committed inventory matches a fresh generation (ignoring `generated_at`).
pub fn verify_fresh(root: &Path, committed_path: &Path) -> Result<()> {
    if !committed_path.is_file() {
        return Err(anyhow!(
            "missing: {} (run: vox ci doc-inventory generate)",
            committed_path.display()
        ));
    }
    let before_raw = fs::read_to_string(committed_path)?;
    let before: Value = serde_json::from_str(&before_raw)
        .with_context(|| format!("parse {}", committed_path.display()))?;
    let sv = before
        .get("schema_version")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    if sv < 3 {
        return Err(anyhow!("doc-inventory.json: expected schema_version >= 3"));
    }

    let tmp = std::env::temp_dir().join(format!(
        "vox-doc-inventory-verify-{}.json",
        std::process::id()
    ));
    generate(root, &tmp)?;
    let after_raw = fs::read_to_string(&tmp)?;
    let _ = fs::remove_file(&tmp);
    let after: Value = serde_json::from_str(&after_raw)?;

    let b = normalize_json_value(strip_generated_at(before));
    let a = normalize_json_value(strip_generated_at(after));
    if a != b {
        return Err(anyhow!(
            "doc-inventory.json is out of date; run: vox ci doc-inventory generate --output docs/agents/doc-inventory.json"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_generated_at_removes_field() {
        let v = serde_json::json!({"schema_version":3,"generated_at":"x","files":[]});
        let s = strip_generated_at(v);
        assert!(s.get("generated_at").is_none());
    }

    #[test]
    fn normalize_json_sorts_object_keys() {
        let v = serde_json::json!({"z":1,"a":{"y":2,"b":3}});
        let n = super::normalize_json_value(v);
        let obj = n.as_object().expect("object");
        let keys: Vec<_> = obj.keys().collect();
        assert_eq!(keys, vec!["a", "z"]);
    }

    #[test]
    fn symbol_hints_link_doc_to_next_item() {
        let src = "/// Example doc\nfn sample_fn() -> i32 { 0 }\n";
        let h = super::rust_symbol_hints(src);
        assert!(
            h.iter().any(|x| x.item_preview.contains("sample_fn")),
            "expected symbol hint for fn after ///, got {h:?}"
        );
    }
}
