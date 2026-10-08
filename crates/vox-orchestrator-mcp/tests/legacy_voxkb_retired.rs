//! Guard (RET-02, D-02): the legacy `vox_kb_*` CRUD tool surface stays retired.
//!
//! Static scan, `std::fs` only. A hit is a ban only when the match is a whole
//! identifier: the next character is not `[a-z0-9_]` (so the retained
//! `vox_kb_get` never collides with the retired `vox_kb_get_feed`, and vice versa)
//! and the previous one is not `[a-z0-9_.]` (so `self.kb.clip(..)` field access is
//! not a catalog id). Matching is case-sensitive on the exact ASCII ids.

use std::path::{Path, PathBuf};

/// Retired names. Extended by later plans (03-12).
const BANNED: &[&str] = &[
    // The twelve legacy MCP tool names (D-02: retired, no aliases).
    "vox_kb_create",
    "vox_kb_list",
    "vox_kb_delete",
    "vox_kb_add_entry",
    "vox_kb_delete_entry",
    "vox_kb_list_entries",
    "vox_kb_review_entry",
    "vox_kb_get_feed",
    "vox_kb_add_rule",
    "vox_kb_list_rules",
    "vox_kb_query",
    "vox_kb_clip",
    // Their operations-catalog ids.
    "kb.create",
    "kb.list",
    "kb.delete",
    "kb.add_entry",
    "kb.delete_entry",
    "kb.list_entries",
    "kb.review_entry",
    "kb.get_feed",
    "kb.add_rule",
    "kb.list_rules",
    "kb.query",
    "kb.clip",
    // The retrieval-bundle KB enrichment field and the three model-output signal adapters.
    "kb_lines",
    "signal_chat",
    "signal_research",
    "signal_scientia",
];

/// Generated or hand-maintained contract files that must not name a banned item.
const CONTRACT_FILES: &[&str] = &[
    "contracts/operations/catalog.v1.yaml",
    "contracts/mcp/tool-registry.canonical.yaml",
    "contracts/capability/capability-registry.yaml",
    "contracts/capability/model-manifest.generated.json",
    "contracts/mcp/http-read-role-governance.yaml",
    "contracts/reports/gui-surface-coverage.v1.json",
];

/// A scan that visits fewer files than this walked the wrong directory.
const MIN_FILES_SCANNED: usize = 100;

const THIS_FILE: &str = "legacy_voxkb_retired.rs";

fn is_ident(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
}

/// First banned whole-identifier hit in `text`, if any.
fn find_banned<'a>(text: &str, banned: &[&'a str]) -> Option<&'a str> {
    let bytes = text.as_bytes();
    for &name in banned {
        let mut from = 0;
        while let Some(pos) = text[from..].find(name) {
            let start = from + pos;
            let end = start + name.len();
            let next_ok = bytes.get(end).is_none_or(|&b| !is_ident(b));
            let prev_ok = start == 0 || !(is_ident(bytes[start - 1]) || bytes[start - 1] == b'.');
            if next_ok && prev_ok {
                return Some(name);
            }
            from = start + 1;
        }
    }
    None
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(name.as_ref(), "target" | "node_modules" | ".git") {
                collect_rs(&path, out);
            }
        } else if name.ends_with(".rs") && name != THIS_FILE {
            out.push(path);
        }
    }
}

#[test]
fn legacy_voxkb_matcher_respects_identifier_boundaries() {
    let banned = ["vox_kb_get_feed", "vox_kb_list"];
    assert_eq!(
        find_banned("\"vox_kb_get_feed\"", &banned),
        Some("vox_kb_get_feed")
    );
    assert_eq!(
        find_banned("  - vox_kb_list\n", &banned),
        Some("vox_kb_list")
    );
    // Retained names and longer retired names never cross-match.
    assert_eq!(
        find_banned("vox_kb_get vox_kb_search vox_kb_status", &banned),
        None
    );
    assert_eq!(find_banned("vox_kb_list_entries", &banned), None);
    assert_eq!(find_banned("my_vox_kb_list", &banned), None);
    // Field access is not a catalog id.
    assert_eq!(find_banned("self.kb.clip(x)", &["kb.clip"]), None);
    assert_eq!(find_banned("id: kb.clip\n", &["kb.clip"]), Some("kb.clip"));
    // Case-sensitive.
    assert_eq!(find_banned("VOX_KB_LIST", &banned), None);
    // A rejected first hit must not hide a later real one.
    assert_eq!(
        find_banned("xvox_kb_list and vox_kb_list", &banned),
        Some("vox_kb_list")
    );
}

#[test]
fn legacy_voxkb_surface_absent() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rs(&root.join("crates"), &mut files);
    files.extend(CONTRACT_FILES.iter().map(|rel| root.join(rel)));

    let mut scanned = 0usize;
    let mut hits = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            // Contract files must exist; sources that are not UTF-8 are skipped.
            assert!(
                !CONTRACT_FILES.iter().any(|rel| path.ends_with(rel)),
                "contract file unreadable: {}",
                path.display()
            );
            continue;
        };
        scanned += 1;
        if let Some(name) = find_banned(&text, BANNED) {
            hits.push(format!("{} mentions retired `{name}`", path.display()));
        }
    }
    assert!(
        scanned >= MIN_FILES_SCANNED,
        "scanned only {scanned} files; the walk is vacuous"
    );
    assert!(
        hits.is_empty(),
        "retired VoxKB surface reappeared:\n{}",
        hits.join("\n")
    );
}
