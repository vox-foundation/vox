use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use super::super::params::ChatMessageParams;

static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@([A-Za-z0-9_.:/\\-]+)")
        .expect("BUG: @mention regex is invalid — check the pattern literal")
});

pub fn chat_grounding_score(params: &ChatMessageParams, mention_count: usize) -> f64 {
    let mut n = 0u32;
    if !params.open_files.is_empty() {
        n += 1;
    }
    if params.active_file.is_some() {
        n += 1;
    }
    if !params.diagnostics.is_empty() {
        n += 1;
    }
    n += (mention_count.min(5)) as u32;
    (0.52 + 0.07 * f64::from(n)).min(0.94)
}

fn rebuild_mention_basename_index(
    workspace_root: &std::path::Path,
) -> std::collections::HashMap<String, Vec<std::path::PathBuf>> {
    let mut map: std::collections::HashMap<String, Vec<std::path::PathBuf>> =
        std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(workspace_root)
        .follow_links(false)
        .max_depth(16)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | "target" | "node_modules" | "dist" | "build" | ".venv" | ".vox"
            )
        })
        .filter_map(std::result::Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let entry_path = entry.path().to_path_buf();
        let Some(name) = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        map.entry(name).or_default().push(entry_path);
    }
    map
}

pub fn safe_truncate_for_prompt(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    let boundary = content.floor_char_boundary(max_bytes);
    format!("{}\n...[truncated]...", &content[..boundary])
}

fn pick_mention_path(
    candidates: &[std::path::PathBuf],
    filename: &str,
    workspace_root: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let mut exact: Option<(usize, std::path::PathBuf)> = None;
    let mut suffix: Option<(usize, std::path::PathBuf)> = None;
    for path in candidates {
        let rel = path
            .strip_prefix(workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel == filename {
            let score = rel.len();
            if exact.as_ref().map(|(s, _)| score < *s).unwrap_or(true) {
                exact = Some((score, path.clone()));
            }
            continue;
        }
        if rel.ends_with(filename) {
            let score = rel.len();
            if suffix.as_ref().map(|(s, _)| score < *s).unwrap_or(true) {
                suffix = Some((score, path.clone()));
            }
        }
    }
    exact.or(suffix).map(|(_, p)| p)
}

/// Resolve @filename mentions using a cached basename → path index (refreshed when workspace changes).
pub(super) fn resolve_mentions(
    prompt: &str,
    workspace_root: &std::path::Path,
    cache: &Arc<
        parking_lot::Mutex<
            Option<(
                std::path::PathBuf,
                Arc<std::collections::HashMap<String, Vec<std::path::PathBuf>>>,
            )>,
        >,
    >,
) -> (String, Vec<String>) {
    let mut expanded = prompt.to_string();
    let mut resolved_files = Vec::new();

    let index: Arc<HashMap<String, Vec<std::path::PathBuf>>> = {
        let mut guard = cache.lock();
        let need_rebuild = guard
            .as_ref()
            .map(|(root, _)| root != workspace_root)
            .unwrap_or(true);
        if need_rebuild {
            let m = rebuild_mention_basename_index(workspace_root);
            *guard = Some((workspace_root.to_path_buf(), Arc::new(m)));
        }
        guard
            .as_ref()
            .map(|(_, m)| Arc::clone(m))
            .unwrap_or_else(|| Arc::new(HashMap::new()))
    };

    for cap in MENTION_RE.captures_iter(prompt) {
        let filename = &cap[1];
        let found = index
            .get(filename)
            .and_then(|paths| pick_mention_path(paths, filename, workspace_root))
            .or_else(|| {
                let all_candidates: Vec<std::path::PathBuf> = index
                    .values()
                    .flat_map(|paths| paths.iter().cloned())
                    .collect();
                pick_mention_path(&all_candidates, filename, workspace_root)
            });
        if let Some(path) = found
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            let rel = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let truncated = safe_truncate_for_prompt(&content, 8000);
            let replacement = format!("\n\n--- @{filename} ({rel}) ---\n{truncated}\n---\n");
            expanded = expanded.replace(&cap[0], &replacement);
            resolved_files.push(rel);
        }
    }
    (expanded, resolved_files)
}
