use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use super::super::params::ChatMessageParams;

static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@([A-Za-z0-9_.:/\\-]+)")
        .expect("BUG: @mention regex is invalid — check the pattern literal")
});

static RULE_MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@rule:([A-Za-z0-9_-]+)").expect("BUG: @rule:<name> regex is invalid")
});

/// Activation mode for user-defined rules (`T3-RULES-SEARCH-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRuleActivationMode {
    Always,
    Manual,
}

/// User-defined rule entry (`T3-RULES-SEARCH-01`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserRuleEntry {
    pub name: String,
    pub mode: UserRuleActivationMode,
    pub body: String,
}

/// Extracts `@rule:<name>` tokens from `prompt`, strips them from the user text,
/// and builds a `<user_rules>` block combining `Always` rules + referenced `Manual` rules (`T3-RULES-SEARCH-01`).
#[must_use]
pub fn resolve_user_rule_mentions(
    prompt: &str,
    catalog: &[UserRuleEntry],
) -> (String, String, Vec<String>) {
    let mut mentioned_names = Vec::new();
    for cap in RULE_MENTION_RE.captures_iter(prompt) {
        let name = cap[1].to_string();
        if !mentioned_names.contains(&name) {
            mentioned_names.push(name);
        }
    }
    let cleaned_prompt = RULE_MENTION_RE.replace_all(prompt, "").trim().to_string();
    let mut active_rules = Vec::new();
    for rule in catalog {
        let include = match rule.mode {
            UserRuleActivationMode::Always => true,
            UserRuleActivationMode::Manual => mentioned_names.iter().any(|n| n == &rule.name),
        };
        if include {
            active_rules.push(format!(
                "<rule name=\"{}\">\n{}\n</rule>",
                rule.name, rule.body
            ));
        }
    }
    let block = if active_rules.is_empty() {
        String::new()
    } else {
        format!("<user_rules>\n{}\n</user_rules>", active_rules.join("\n"))
    };
    (cleaned_prompt, block, mentioned_names)
}

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

/// Extract a sub-10ms AST symbol-and-signature skeleton (`Lstart-Lend: signature`)
/// for `.vox`, `.rs`, and `.ts`/`.tsx` files (`SOTA-SLICE-01..04`).
#[must_use]
pub fn extract_compact_symbol_skeleton(path: &str, content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let mut entries = Vec::new();
    let mut i = 0usize;
    while i < total_lines {
        let trimmed = lines[i].trim();
        let is_symbol = trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(crate) async fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub(crate) struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("actor ")
            || trimmed.starts_with("workflow ")
            || trimmed.starts_with("activity ")
            || trimmed.starts_with("component ")
            || trimmed.starts_with("@mcp.tool")
            || trimmed.starts_with("@scheduled")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("export async function ")
            || trimmed.starts_with("export interface ")
            || trimmed.starts_with("export type ");
        if is_symbol {
            let start_line = i + 1;
            let sig = trimmed.trim_end_matches('{').trim_end_matches(';').trim();
            // Find matching closing brace for span estimation
            let mut depth = 0i32;
            let mut opened = false;
            let mut end_line = start_line;
            for (j, line) in lines.iter().enumerate().skip(i) {
                for ch in line.chars() {
                    if ch == '{' {
                        depth += 1;
                        opened = true;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                end_line = j + 1;
                if opened && depth <= 0 {
                    break;
                }
                if !opened && (line.trim_end().ends_with(';') || j > i + 4) {
                    break;
                }
            }
            entries.push(format!("  L{start_line}-L{end_line}: {sig}"));
        }
        i += 1;
    }
    if entries.is_empty() {
        safe_truncate_for_prompt(content, 1600)
    } else {
        format!(
            "[SYMBOL SKELETON: {path} ({total_lines} lines, {} symbols — use line ranges for targeted reads/edits)]\n{}",
            entries.len(),
            entries.join("\n")
        )
    }
}

/// Extract the exact `(start_line, end_line, slice_text)` of a named symbol (`fn`, `struct`, `actor`, `workflow`, etc.)
/// in `content` (`SOTA-SLICE-03`).
#[allow(
    dead_code,
    reason = "public symbol-slice helper invoked by AST slice callers and tests"
)]
#[must_use]
pub fn extract_symbol_span(content: &str, symbol_name: &str) -> Option<(usize, usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let needle_fn = format!("fn {symbol_name}");
    let needle_struct = format!("struct {symbol_name}");
    let needle_enum = format!("enum {symbol_name}");
    let needle_actor = format!("actor {symbol_name}");
    let needle_workflow = format!("workflow {symbol_name}");
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.contains(&needle_fn)
            || t.contains(&needle_struct)
            || t.contains(&needle_enum)
            || t.contains(&needle_actor)
            || t.contains(&needle_workflow)
        {
            let start_line = i + 1;
            let mut depth = 0i32;
            let mut opened = false;
            let mut end_idx = i;
            for (j, l) in lines.iter().enumerate().skip(i) {
                for ch in l.chars() {
                    if ch == '{' {
                        depth += 1;
                        opened = true;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                end_idx = j;
                if opened && depth <= 0 {
                    break;
                }
            }
            let slice = lines[i..=end_idx].join("\n");
            return Some((start_line, end_idx + 1, slice));
        }
    }
    None
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

/// Load default + workspace `.vox/rules.json` user rule catalog (`T3-SALVAGE-RULES-02`).
#[must_use]
pub fn load_workspace_user_rules(workspace_root: &std::path::Path) -> Vec<UserRuleEntry> {
    let mut catalog = vec![
        UserRuleEntry {
            name: "concise".to_string(),
            mode: UserRuleActivationMode::Manual,
            body: "Keep explanations under 2 sentences and prefer bullet points and direct code.".to_string(),
        },
        UserRuleEntry {
            name: "rust-strict".to_string(),
            mode: UserRuleActivationMode::Manual,
            body: "Forbid .unwrap() and .expect() in production Rust paths; propagate Result<T, E> with context.".to_string(),
        },
        UserRuleEntry {
            name: "tdd".to_string(),
            mode: UserRuleActivationMode::Manual,
            body: "Write failing unit/integration tests before modifying implementation code.".to_string(),
        },
        UserRuleEntry {
            name: "security".to_string(),
            mode: UserRuleActivationMode::Manual,
            body: "Validate untrusted inputs, route secrets through vox_secrets, and route crypto through vox_crypto.".to_string(),
        },
    ];
    let rules_json_path = workspace_root.join(".vox").join("rules.json");
    if let Ok(raw) = std::fs::read_to_string(&rules_json_path)
        && let Ok(custom) = serde_json::from_str::<Vec<UserRuleEntry>>(&raw)
    {
        for entry in custom {
            if let Some(existing) = catalog.iter_mut().find(|r| r.name == entry.name) {
                *existing = entry;
            } else {
                catalog.push(entry);
            }
        }
    }
    catalog
}

/// Resolve @filename mentions and `@rule:<name>` tags using a cached basename → path index.
/// Strips `@rule:<name>` tags before running `MENTION_RE` so `@rule:...` is never misclassified
/// as a missing workspace file (`T3-SALVAGE-RULES-02`).
#[allow(
    dead_code,
    reason = "convenience wrapper around resolve_mentions_with_rules using default workspace rules"
)]
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
    let rule_catalog = load_workspace_user_rules(workspace_root);
    resolve_mentions_with_rules(prompt, workspace_root, cache, &rule_catalog)
}

/// Resolve `@rule:<name>` mentions + `@filename` mentions against `rule_catalog` (`T3-SALVAGE-RULES-02`).
pub(super) fn resolve_mentions_with_rules(
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
    rule_catalog: &[UserRuleEntry],
) -> (String, Vec<String>) {
    let (cleaned_prompt, rules_block, _mentioned_rules) =
        resolve_user_rule_mentions(prompt, rule_catalog);
    let mut expanded = cleaned_prompt.clone();
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

    for cap in MENTION_RE.captures_iter(&cleaned_prompt) {
        let filename = &cap[1];
        if filename.starts_with("rule:") {
            continue;
        }
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
            let line_count = content.lines().count();
            let body = if line_count > 120 {
                extract_compact_symbol_skeleton(&rel, &content)
            } else {
                safe_truncate_for_prompt(&content, 8000)
            };
            let replacement = format!("\n\n--- @{filename} ({rel}) ---\n{body}\n---\n");
            expanded = expanded.replace(&cap[0], &replacement);
            resolved_files.push(rel);
        }
    }
    if !rules_block.is_empty() {
        expanded = format!("{rules_block}\n\n{expanded}");
    }
    (expanded, resolved_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_compact_symbol_skeleton_compresses_large_source_and_locates_spans() {
        let mut lines = Vec::new();
        lines.push("pub struct OrderBook {".to_string());
        for i in 0..30 {
            lines.push(format!("    pub field_{i}: u64,"));
        }
        lines.push("}".to_string());
        for fn_idx in 0..10 {
            lines.push(format!(
                "pub fn process_order_{fn_idx}(book: &mut OrderBook) -> Result<u64, String> {{"
            ));
            for step in 0..12 {
                lines.push(format!("    let val_{step} = book.field_0 + {step};"));
            }
            lines.push("    Ok(0)".to_string());
            lines.push("}".to_string());
        }
        let raw_source = lines.join("\n");
        assert!(raw_source.lines().count() > 120);

        let skeleton = extract_compact_symbol_skeleton("src/order_book.rs", &raw_source);
        assert!(skeleton.contains("[SYMBOL SKELETON: src/order_book.rs"));
        assert!(skeleton.contains("pub struct OrderBook"));
        assert!(skeleton.contains("pub fn process_order_5"));
        // Verify >= 65% compression on large source files
        assert!(
            skeleton.len() * 100 <= raw_source.len() * 35,
            "symbol skeleton must compress raw source by >= 65%: skeleton={}B vs raw={}B",
            skeleton.len(),
            raw_source.len()
        );

        let (start, end, slice) =
            extract_symbol_span(&raw_source, "process_order_5").expect("find process_order_5");
        assert!(start > 30 && end > start);
        assert!(slice.starts_with("pub fn process_order_5"));
        assert!(slice.ends_with('}'));
    }

    #[test]
    fn resolve_user_rule_mentions_combines_always_and_manual_rules() {
        let catalog = vec![
            UserRuleEntry {
                name: "concise".into(),
                mode: UserRuleActivationMode::Always,
                body: "Keep replies short.".into(),
            },
            UserRuleEntry {
                name: "rust-strict".into(),
                mode: UserRuleActivationMode::Manual,
                body: "Forbid unwrap in prod.".into(),
            },
        ];
        let (cleaned, block, names) =
            resolve_user_rule_mentions("Fix this module @rule:rust-strict please", &catalog);
        assert_eq!(cleaned, "Fix this module  please");
        assert_eq!(names, vec!["rust-strict".to_string()]);
        assert!(block.contains("<rule name=\"concise\">"));
        assert!(block.contains("<rule name=\"rust-strict\">"));
    }

    #[test]
    fn resolve_mentions_with_rules_strips_rule_tag_before_file_lookup_and_injects_block() {
        let catalog = vec![UserRuleEntry {
            name: "rust-strict".into(),
            mode: UserRuleActivationMode::Manual,
            body: "Forbid unwrap in production paths.".into(),
        }];
        let cache = Arc::new(parking_lot::Mutex::new(None));
        let tmp = tempfile::tempdir().expect("tempdir");
        let (expanded, files) = resolve_mentions_with_rules(
            "Audit this function @rule:rust-strict now",
            tmp.path(),
            &cache,
            &catalog,
        );
        assert!(files.is_empty());
        assert!(expanded.contains("<user_rules>"));
        assert!(expanded.contains("<rule name=\"rust-strict\">"));
        assert!(!expanded.contains("@rule:rust-strict"));
    }
}
