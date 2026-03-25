//! Rust `///` / `//!` symbol hint extraction.

use std::sync::LazyLock;

use regex::Regex;

use crate::types::SymbolHint;

pub(crate) static ITEM_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:pub\s+)?(?:struct|enum|fn|trait|type|mod|impl)\b|^\s*pub\s+\w+\s*:\s*")
        .expect("ITEM_LINE_RE")
});

pub(crate) fn classify_quality(doc_text: &str) -> &'static str {
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
