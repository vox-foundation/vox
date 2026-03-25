//! Build [`FileEntry`](crate::types::FileEntry) values and hotspot tiers.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::constants::{HOTSPOT_TIER1, HOTSPOT_TIER2_RUST};
use crate::counts::{count_md, count_python, count_rust_lines, count_shell, count_ts};
use crate::types::FileEntry;

fn tier1_set() -> HashSet<&'static str> {
    HOTSPOT_TIER1.iter().copied().collect()
}

fn tier2_rust_set() -> HashSet<&'static str> {
    HOTSPOT_TIER2_RUST.iter().copied().collect()
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

pub(crate) fn build_file_entry(root: &Path, rel: &str) -> Result<FileEntry> {
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
