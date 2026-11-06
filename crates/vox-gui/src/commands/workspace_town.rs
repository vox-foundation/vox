// crates/vox-gui/src/commands/workspace_town.rs
//! Polyglot workspace scan for the Vox Urbs town map (`URBS-POLYGLOT-SCAN-01..04`):
//! partitions any repository (Cargo `crates/*`, JS/TS `packages/*` & `apps/*`, Go `cmd/*`/`internal/*`/`pkg/*`,
//! Python `src/<pkg>`, or single-app `src/<module>`) into modules -> files -> line counts + live Git diff churn.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

const MAX_FILES: usize = 20_000;
/// Rescan at most this often; the town layout is not a file watcher.
const CACHE_TTL_MS: i64 = 60_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TownFileDto {
    /// Path relative to the workspace root, forward slashes.
    pub path: String,
    pub lines: u32,
    /// Optional git working-tree status code (`"M"`, `"A"`, `"U"`, `"?"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_status: Option<String>,
    /// Uncommitted added lines from `git diff --numstat`.
    #[serde(default)]
    pub diff_added_lines: u32,
    /// Uncommitted deleted lines from `git diff --numstat`.
    #[serde(default)]
    pub diff_deleted_lines: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TownCrateDto {
    pub name: String,
    /// Module root relative to the workspace root, forward slashes.
    pub root: String,
    pub files: Vec<TownFileDto>,
    /// Intra-workspace module dependencies parsed from manifests and source imports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    /// Number of files in this module with uncommitted git changes.
    #[serde(default)]
    pub dirty_file_count: u32,
    /// Sum of `diff_added_lines + diff_deleted_lines` across all files in this module.
    #[serde(default)]
    pub total_churn_lines: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TownDependencyEdge {
    pub from_district_id: String,
    pub to_district_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imported_symbols: Vec<String>,
    #[serde(default)]
    pub call_site_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_layer: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_layer: Option<u8>,
    #[serde(default)]
    pub layer_violation: bool,
    #[serde(default)]
    pub broken_contract_errors: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompilerDiagnosticSpanDto {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileDiffAndDiagnosticsDto {
    pub file_path: String,
    pub added_lines: u32,
    pub deleted_lines: u32,
    pub unified_diff: String,
    pub staged: bool,
    #[serde(default)]
    pub diagnostics: Vec<CompilerDiagnosticSpanDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TownScanDto {
    pub crates: Vec<TownCrateDto>,
    /// Cross-module dependency edges with imported symbols and `layers.toml` violation flags (`URBS-VIADUCT-01`).
    #[serde(default)]
    pub edges: Vec<TownDependencyEdge>,
    /// Pre-hydrated unified diff + diagnostic payloads for dirty working-tree files (`URBS-LOUPE-01`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_diffs: Vec<FileDiffAndDiagnosticsDto>,
    /// Absolute workspace root, forward slashes — lets the UI build absolute
    /// paths for open_locator without a second command.
    pub root: String,
    pub scanned_at_ms: i64,
    pub truncated: bool,
    /// Repository directory basename (e.g. `"vox"`, `"next-app"`, `"fastapi-service"`).
    #[serde(default)]
    pub repo_name: String,
    /// Dominant source language detected by line count (`"rust"`, `"typescript"`, `"python"`, `"go"`, etc.).
    #[serde(default)]
    pub primary_language: String,
}

const WORKSPACE_CONTAINER_PREFIXES: &[&str] = &[
    "crates/",
    "packages/",
    "apps/",
    "services/",
    "libs/",
    "cmd/",
    "internal/",
    "pkg/",
    "modules/",
];

/// Group scanned source files across polyglot module roots (`crates/<name>`, `packages/<name>`,
/// `apps/<name>`, `services/<name>`, `libs/<name>`, `cmd/<name>`, `internal/<name>`, `pkg/<name>`,
/// or `src/<subdir>`); everything else falls under `"(workspace)"`. Pure — unit-testable without IO.
pub(crate) fn group_by_crate(files: Vec<TownFileDto>) -> Vec<TownCrateDto> {
    let mut map: BTreeMap<(String, String), Vec<TownFileDto>> = BTreeMap::new();
    for f in files {
        let mut matched_key: Option<(String, String)> = None;
        for prefix in WORKSPACE_CONTAINER_PREFIXES {
            if let Some(rest) = f.path.strip_prefix(prefix)
                && let Some(name) = rest.split('/').next()
                && !name.is_empty()
                && rest.contains('/')
            {
                let clean_prefix = prefix.trim_end_matches('/');
                matched_key = Some((name.to_string(), format!("{clean_prefix}/{name}")));
                break;
            }
        }
        if matched_key.is_none()
            && let Some(rest) = f.path.strip_prefix("src/")
            && let Some(subdir) = rest.split('/').next()
            && !subdir.is_empty()
            && rest.contains('/')
        {
            matched_key = Some((subdir.to_string(), format!("src/{subdir}")));
        }
        let key = matched_key.unwrap_or_else(|| ("(workspace)".to_string(), String::new()));
        map.entry(key).or_default().push(f);
    }
    map.into_iter()
        .map(|((name, root), files)| {
            let dirty_file_count = files
                .iter()
                .filter(|f| {
                    f.git_status.is_some() || f.diff_added_lines > 0 || f.diff_deleted_lines > 0
                })
                .count() as u32;
            let total_churn_lines = files
                .iter()
                .map(|f| f.diff_added_lines.saturating_add(f.diff_deleted_lines))
                .sum();
            TownCrateDto {
                name,
                root,
                files,
                deps: Vec::new(),
                dirty_file_count,
                total_churn_lines,
            }
        })
        .collect()
}

/// Extract intra-workspace dependencies for `self_name` against all discovered `known_modules`
/// (plus any `vox-*` Cargo dependencies for backwards compatibility).
pub(crate) fn parse_workspace_deps(
    root: &Path,
    crate_root: &str,
    self_name: &str,
    known_modules: &BTreeSet<String>,
    module_files: &[TownFileDto],
) -> Vec<String> {
    if crate_root.is_empty() {
        return Vec::new();
    }
    let mut deps = BTreeSet::new();
    let mod_dir = root.join(crate_root);

    // 1. Check Cargo.toml (`<dep> = ...` or `vox-*`)
    if let Ok(cargo_toml) = std::fs::read_to_string(mod_dir.join("Cargo.toml")) {
        for line in cargo_toml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.starts_with('[') {
                continue;
            }
            if let Some((lhs, _rhs)) = trimmed.split_once('=') {
                let candidate = lhs.trim().trim_matches('"');
                if candidate != self_name
                    && (known_modules.contains(candidate) || candidate.starts_with("vox-"))
                {
                    deps.insert(candidate.to_string());
                }
            }
        }
    }

    // 2. Check package.json / pyproject.toml / go.mod for references to known sibling modules
    for manifest_name in ["package.json", "pyproject.toml", "go.mod"] {
        if let Ok(text) = std::fs::read_to_string(mod_dir.join(manifest_name)) {
            for candidate in known_modules {
                if candidate == self_name || candidate == "(workspace)" {
                    continue;
                }
                if text.contains(&format!("\"{candidate}\""))
                    || text.contains(&format!("/{candidate}\""))
                    || text.contains(&format!("/{candidate}"))
                {
                    deps.insert(candidate.clone());
                }
            }
        }
    }

    // 3. Lightweight source-import scan (first 40 lines of up to 32 files in the module)
    for file in module_files.iter().take(32) {
        let full_path = root.join(&file.path);
        let Ok(content) = std::fs::read_to_string(full_path) else {
            continue;
        };
        for line in content.lines().take(40) {
            let trimmed = line.trim();
            if !(trimmed.starts_with("import ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("use ")
                || trimmed.contains("require("))
            {
                continue;
            }
            for candidate in known_modules {
                if candidate == self_name || candidate == "(workspace)" {
                    continue;
                }
                let snake = candidate.replace('-', "_");
                if trimmed.contains(&format!("/{candidate}"))
                    || trimmed.contains(&format!("'{candidate}'"))
                    || trimmed.contains(&format!("\"{candidate}\""))
                    || trimmed.contains(&format!("from {snake}"))
                    || trimmed.contains(&format!("import {snake}"))
                    || trimmed.contains(&format!("use {snake}::"))
                {
                    deps.insert(candidate.clone());
                }
            }
        }
    }

    deps.into_iter().collect()
}

pub(crate) fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "mjs"
                | "cjs"
                | "vox"
                | "py"
                | "go"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "java"
                | "kt"
                | "swift"
                | "rb"
                | "cs"
                | "zig"
                | "sql"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "css"
                | "scss"
                | "html"
                | "vue"
                | "svelte"
                | "md"
        )
    )
}

fn language_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => "typescript",
        "py" => "python",
        "go" => "go",
        "vox" => "vox",
        "c" | "cpp" | "h" | "hpp" => "cpp",
        "java" | "kt" => "jvm",
        "swift" => "swift",
        "rb" => "ruby",
        "cs" => "csharp",
        "zig" => "zig",
        _ => "other",
    }
}

fn collect_git_working_tree_diffs(root: &Path) -> HashMap<String, (Option<String>, u32, u32)> {
    let mut map: HashMap<String, (Option<String>, u32, u32)> = HashMap::new();
    let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());

    if let Ok(out) = Command::new(&git_bin)
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        && out.status.success()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }
            let status_raw = line[..2].trim();
            let path_part = line[3..].trim();
            let rel_path = path_part
                .rsplit_once(" -> ")
                .map(|(_, dst)| dst)
                .unwrap_or(path_part)
                .trim_matches('"')
                .replace('\\', "/");
            let code = if status_raw.contains('?') {
                "U"
            } else if status_raw.contains('A') {
                "A"
            } else if status_raw.contains('D') {
                "D"
            } else {
                "M"
            };
            map.entry(rel_path)
                .or_insert((Some(code.to_string()), 0, 0))
                .0 = Some(code.to_string());
        }
    }

    if let Ok(out) = Command::new(&git_bin)
        .args(["diff", "--numstat", "HEAD"])
        .current_dir(root)
        .output()
        && out.status.success()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let mut parts = line.split_whitespace();
            let Some(add_str) = parts.next() else {
                continue;
            };
            let Some(del_str) = parts.next() else {
                continue;
            };
            let Some(path_str) = parts.next() else {
                continue;
            };
            let added = add_str.parse::<u32>().unwrap_or(0);
            let deleted = del_str.parse::<u32>().unwrap_or(0);
            let rel_path = path_str.replace('\\', "/");
            let entry = map
                .entry(rel_path)
                .or_insert_with(|| (Some("M".to_string()), 0, 0));
            if entry.0.is_none() {
                entry.0 = Some("M".to_string());
            }
            entry.1 = added;
            entry.2 = deleted;
        }
    }

    map
}

pub(crate) fn scan(root: &Path) -> TownScanDto {
    let mut files = Vec::new();
    let mut truncated = false;
    let git_diffs = collect_git_working_tree_diffs(root);
    let mut lang_lines: HashMap<&'static str, u64> = HashMap::new();

    let walker = ignore::WalkBuilder::new(root).hidden(true).build();
    for entry in walker.flatten() {
        if files.len() >= MAX_FILES {
            truncated = true;
            break;
        }
        let path = entry.path();
        if !path.is_file() || !is_source_file(path) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.starts_with("target/")
            || rel.contains("node_modules/")
            || rel.starts_with("dist/")
            || rel.starts_with("build/")
            || rel.starts_with(".venv/")
            || rel.starts_with("venv/")
        {
            continue;
        }
        let lines = std::fs::read_to_string(path)
            .map(|c| c.lines().count() as u32)
            .unwrap_or(0);
        let lang = language_for_path(&rel);
        if lang != "other" {
            *lang_lines.entry(lang).or_insert(0) += u64::from(lines.max(1));
        }
        let (git_status, diff_added_lines, diff_deleted_lines) =
            git_diffs.get(&rel).cloned().unwrap_or((None, 0, 0));
        files.push(TownFileDto {
            path: rel,
            lines,
            git_status,
            diff_added_lines,
            diff_deleted_lines,
        });
    }

    let mut crates = group_by_crate(files);
    let known_modules: BTreeSet<String> = crates.iter().map(|c| c.name.clone()).collect();
    for c in &mut crates {
        c.deps = parse_workspace_deps(root, &c.root, &c.name, &known_modules, &c.files);
    }
    let edges = build_dependency_edges(root, &crates);
    let mut file_diffs = Vec::new();
    for c in &crates {
        for f in &c.files {
            if file_diffs.len() >= 16 {
                break;
            }
            if f.diff_added_lines > 0 || f.diff_deleted_lines > 0 || f.git_status.is_some() {
                if let Ok(diff_dto) = inspect_file_diff_and_diagnostics(root, &f.path) {
                    file_diffs.push(diff_dto);
                }
            }
        }
    }

    let repo_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string();
    let primary_language = lang_lines
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(lang, _)| lang.to_string())
        .unwrap_or_else(|| "polyglot".to_string());

    TownScanDto {
        crates,
        edges,
        file_diffs,
        root: root.to_string_lossy().replace('\\', "/"),
        scanned_at_ms: chrono::Utc::now().timestamp_millis(),
        truncated,
        repo_name,
        primary_language,
    }
}

/// Parse `docs/src/architecture/layers.toml` (when present) into a map of `crate_name -> layer_number`.
pub(crate) fn parse_layers_toml(root: &Path) -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let layers_path = root.join("docs/src/architecture/layers.toml");
    let Ok(content) = std::fs::read_to_string(layers_path) else {
        return map;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            let key = lhs.trim().trim_matches('"');
            let val_str = rhs
                .trim()
                .trim_matches('"')
                .trim_start_matches('L')
                .trim_start_matches('l');
            if let Ok(layer_num) = val_str.parse::<u8>() {
                map.insert(key.to_string(), layer_num);
            }
        }
    }
    map
}

/// Build cross-module `TownDependencyEdge` items (`URBS-VIADUCT-01`) with imported symbol samples,
/// call-site counts, and `layers.toml` upward-dependency violation flags.
pub(crate) fn build_dependency_edges(
    root: &Path,
    crates: &[TownCrateDto],
) -> Vec<TownDependencyEdge> {
    let layer_map = parse_layers_toml(root);
    let mut edges = Vec::new();

    for c in crates {
        if c.deps.is_empty() {
            continue;
        }
        let from_layer = layer_map.get(&c.name).copied();
        for dep in &c.deps {
            let to_layer = layer_map.get(dep).copied();
            let snake_dep = dep.replace('-', "_");
            let mut symbols = BTreeSet::new();
            let mut call_site_count = 0u32;

            for file in c.files.iter().take(24) {
                let Ok(text) = std::fs::read_to_string(root.join(&file.path)) else {
                    continue;
                };
                for line in text.lines().take(60) {
                    let trimmed = line.trim();
                    if trimmed.contains(&snake_dep) || trimmed.contains(dep.as_str()) {
                        call_site_count = call_site_count.saturating_add(1);
                        if symbols.len() < 4
                            && (trimmed.starts_with("use ")
                                || trimmed.starts_with("import ")
                                || trimmed.starts_with("from "))
                        {
                            let cleaned = trimmed
                                .trim_end_matches(';')
                                .chars()
                                .take(72)
                                .collect::<String>();
                            symbols.insert(cleaned);
                        }
                    }
                }
            }
            if symbols.is_empty() {
                symbols.insert(format!("{snake_dep}::*"));
            }
            let layer_violation =
                matches!((from_layer, to_layer), (Some(from_l), Some(to_l)) if from_l < to_l);
            edges.push(TownDependencyEdge {
                from_district_id: c.name.clone(),
                to_district_id: dep.clone(),
                imported_symbols: symbols.into_iter().collect(),
                call_site_count: call_site_count.max(1),
                from_layer,
                to_layer,
                layer_violation,
                broken_contract_errors: 0,
            });
        }
    }
    edges
}

fn validate_safe_rel_path(file_path: &str) -> Result<String, String> {
    let normalized = file_path.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains("..")
        || normalized.starts_with('-')
    {
        return Err(format!(
            "unsafe or invalid workspace-relative path: {file_path}"
        ));
    }
    Ok(normalized)
}

/// Extract unified Git diff (`git diff --unified=3 HEAD -- <file_path>`) and structured diagnostics (`URBS-LOUPE-01`).
pub fn inspect_file_diff_and_diagnostics(
    root: &Path,
    file_path: &str,
) -> Result<FileDiffAndDiagnosticsDto, String> {
    let rel = validate_safe_rel_path(file_path)?;
    let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());

    let diff_out = Command::new(&git_bin)
        .args(["diff", "--unified=3", "HEAD", "--", &rel])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    let unified_diff = String::from_utf8_lossy(&diff_out.stdout).to_string();

    let mut added_lines = 0u32;
    let mut deleted_lines = 0u32;
    let mut diagnostics = Vec::new();
    for (idx, line) in unified_diff.lines().enumerate() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added_lines = added_lines.saturating_add(1);
            if line.contains("todo!(") || line.contains("unimplemented!(") {
                diagnostics.push(CompilerDiagnosticSpanDto {
                    file: rel.clone(),
                    line: (idx + 1) as u32,
                    column: 1,
                    severity: "warning".to_string(),
                    code: Some("TOESTUB-STUB".to_string()),
                    message: "Stub macro detected in added diff line".to_string(),
                });
            }
        } else if line.starts_with('-') {
            deleted_lines = deleted_lines.saturating_add(1);
        }
    }

    let staged_out = Command::new(&git_bin)
        .args(["diff", "--cached", "--name-only", "--", &rel])
        .current_dir(root)
        .output()
        .ok();
    let staged = staged_out
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);

    Ok(FileDiffAndDiagnosticsDto {
        file_path: rel,
        added_lines,
        deleted_lines,
        unified_diff,
        staged,
        diagnostics,
    })
}

/// Stage a single workspace file via `git add -- <file_path>` (`URBS-LOUPE-03`).
#[allow(dead_code)]
pub fn stage_workspace_file(root: &Path, file_path: &str) -> Result<bool, String> {
    let rel = validate_safe_rel_path(file_path)?;
    let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());
    let out = Command::new(&git_bin)
        .args(["add", "--", &rel])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(out.status.success())
}

/// Revert a single workspace file via `git checkout -- <file_path>` (`URBS-LOUPE-03`).
#[allow(dead_code)]
pub fn revert_workspace_file(root: &Path, file_path: &str) -> Result<bool, String> {
    let rel = validate_safe_rel_path(file_path)?;
    let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());
    let out = Command::new(&git_bin)
        .args(["checkout", "--", &rel])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(out.status.success())
}

static CACHE: Mutex<Option<TownScanDto>> = Mutex::new(None);

#[tauri::command]
pub async fn workspace_town_scan() -> Result<TownScanDto, String> {
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(cached) = CACHE.lock().unwrap().clone()
        && now - cached.scanned_at_ms < CACHE_TTL_MS
    {
        return Ok(cached);
    }
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let fresh = tokio::task::spawn_blocking(move || scan(&root))
        .await
        .map_err(|e| e.to_string())?;
    *CACHE.lock().unwrap() = Some(fresh.clone());
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_crate_files_and_workspace_files() {
        let files = vec![
            TownFileDto {
                path: "crates/vox-db/src/lib.rs".into(),
                lines: 100,
                ..Default::default()
            },
            TownFileDto {
                path: "crates/vox-db/src/store.rs".into(),
                lines: 50,
                git_status: Some("M".into()),
                diff_added_lines: 18,
                diff_deleted_lines: 4,
            },
            TownFileDto {
                path: "crates/vox-cli/src/main.rs".into(),
                lines: 10,
                ..Default::default()
            },
            TownFileDto {
                path: "docs/src/intro.md".into(),
                lines: 5,
                ..Default::default()
            },
        ];
        let crates = group_by_crate(files);
        let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["(workspace)", "vox-cli", "vox-db"]);
        assert_eq!(crates[2].files.len(), 2);
        assert_eq!(crates[2].root, "crates/vox-db");
        assert_eq!(crates[2].dirty_file_count, 1);
        assert_eq!(crates[2].total_churn_lines, 22);
    }

    #[test]
    fn groups_polyglot_typescript_python_go_and_single_app_trees() {
        let files = vec![
            TownFileDto {
                path: "packages/ui-kit/src/Button.tsx".into(),
                lines: 80,
                ..Default::default()
            },
            TownFileDto {
                path: "apps/web-portal/src/App.tsx".into(),
                lines: 140,
                ..Default::default()
            },
            TownFileDto {
                path: "cmd/gateway/main.go".into(),
                lines: 60,
                ..Default::default()
            },
            TownFileDto {
                path: "internal/ledger/store.go".into(),
                lines: 120,
                ..Default::default()
            },
            TownFileDto {
                path: "src/payments/stripe.py".into(),
                lines: 95,
                ..Default::default()
            },
        ];
        let crates = group_by_crate(files);
        let names: Vec<&str> = crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["gateway", "ledger", "payments", "ui-kit", "web-portal"]
        );
    }

    #[test]
    fn scan_walks_a_fixture_tree_and_extracts_non_vox_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let core_src = dir.path().join("packages/core-sdk/src");
        let app_src = dir.path().join("apps/dashboard/src");
        std::fs::create_dir_all(&core_src).unwrap();
        std::fs::create_dir_all(&app_src).unwrap();
        std::fs::write(core_src.join("index.ts"), "export const v = 1;\n").unwrap();
        std::fs::write(
            dir.path().join("apps/dashboard/package.json"),
            r#"{"name":"dashboard","dependencies":{"@acme/core-sdk":"workspace:*"}}"#,
        )
        .unwrap();
        std::fs::write(
            app_src.join("main.tsx"),
            "import { v } from '@acme/core-sdk';\nconsole.log(v);\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("ignored.png"), b"\x89PNG").unwrap();

        let result = scan(dir.path());
        assert!(!result.truncated);
        assert_eq!(result.primary_language, "typescript");
        let dash = result
            .crates
            .iter()
            .find(|c| c.name == "dashboard")
            .unwrap();
        assert_eq!(dash.deps, vec!["core-sdk".to_string()]);
        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.edges[0].from_district_id, "dashboard");
        assert_eq!(result.edges[0].to_district_id, "core-sdk");
        assert!(!result.edges[0].imported_symbols.is_empty());
    }

    #[test]
    fn build_dependency_edges_flags_upward_layer_violation_from_layers_toml() {
        let dir = tempfile::tempdir().unwrap();
        let arch_dir = dir.path().join("docs/src/architecture");
        std::fs::create_dir_all(&arch_dir).unwrap();
        std::fs::write(
            arch_dir.join("layers.toml"),
            "\"core-types\" = 1\n\"app-ui\" = 3\n",
        )
        .unwrap();

        let crates = vec![
            TownCrateDto {
                name: "core-types".into(),
                root: "crates/core-types".into(),
                files: vec![],
                deps: vec!["app-ui".into()],
                dirty_file_count: 0,
                total_churn_lines: 0,
            },
            TownCrateDto {
                name: "app-ui".into(),
                root: "crates/app-ui".into(),
                files: vec![],
                deps: vec!["core-types".into()],
                dirty_file_count: 0,
                total_churn_lines: 0,
            },
        ];

        let edges = build_dependency_edges(dir.path(), &crates);
        assert_eq!(edges.len(), 2);
        let upward = edges
            .iter()
            .find(|e| e.from_district_id == "core-types")
            .unwrap();
        assert_eq!(upward.from_layer, Some(1));
        assert_eq!(upward.to_layer, Some(3));
        assert!(upward.layer_violation);

        let downward = edges
            .iter()
            .find(|e| e.from_district_id == "app-ui")
            .unwrap();
        assert!(!downward.layer_violation);
    }

    #[test]
    fn inspect_file_diff_rejects_traversal_paths() {
        let dir = tempfile::tempdir().unwrap();
        assert!(inspect_file_diff_and_diagnostics(dir.path(), "../secret.txt").is_err());
        assert!(inspect_file_diff_and_diagnostics(dir.path(), "/etc/passwd").is_err());
    }
}
