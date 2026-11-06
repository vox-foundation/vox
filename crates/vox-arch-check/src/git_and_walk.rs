use super::*;

/// True when the `loc_budget` guard is explicitly switched off.
///
/// Disables ONLY the absolute `max_loc` threshold check. The per-crate `max_loc`
/// values are deliberately retained, because Rule 13 (`loc_delta`) skips any
/// crate whose `max_loc` is `None` — deleting the budgets to silence the
/// absolute check would also disable growth detection, removing the guard that
/// replaces it. Under `off`, `max_loc` stops meaning "must be under this" and
/// means "growth past this baseline is the signal".
///
/// Note this cannot be folded into [`parse_strictness`], which returns a bool
/// and maps any unrecognised value to its default — so `"off"` there would
/// silently resolve to `"warn"` and keep printing.
pub(crate) fn loc_budget_disabled(setting: Option<&String>) -> bool {
    matches!(setting.map(|s| s.as_str()), Some("off") | Some("none"))
}

pub(crate) fn parse_strictness(setting: Option<&String>, default_strict: bool) -> bool {
    match setting.map(|s| s.as_str()) {
        Some("error") | Some("strict") => true,
        Some("warn") | Some("warning") => false,
        _ => default_strict,
    }
}

/// Built-in directory *names* (final path component) never recursed into by Rule 3/9/11.
pub(crate) const WALK_PRUNE_DEFAULT_DIR_NAMES: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    ".pnpm-store",
    "__pycache__",
    ".venv",
    ".mypy_cache",
    ".turbo",
    ".next",
    ".parcel-cache",
    ".cargo",
];

pub(crate) fn built_in_walk_prune_names() -> HashSet<String> {
    WALK_PRUNE_DEFAULT_DIR_NAMES
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

pub(crate) fn walk_prune_dir_names(cfg: &LayersConfig) -> HashSet<String> {
    let mut s = built_in_walk_prune_names();
    for extra in &cfg.arch_check.walk_prune.extra_skip_dir_names {
        let t = extra.trim();
        if !t.is_empty() {
            s.insert(t.to_string());
        }
    }
    s
}

pub(crate) fn dir_entry_should_be_pruned(path: &Path, prune_dir_names: &HashSet<String>) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| prune_dir_names.contains(name))
}

/// Recursive file listing for repo scans; skips heavy artifact trees (see `walk_prune_dir_names`).
pub(crate) fn walk_repo_files(root: &Path, prune_dir_names: &HashSet<String>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        !(e.file_type().is_dir() && dir_entry_should_be_pruned(e.path(), prune_dir_names))
    });
    for entry in walker.filter_map(Result::ok) {
        if entry.file_type().is_file() {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

/// Parent directory of `manifest_path`, relative to `repo`, using `/` separators.
pub(crate) fn manifest_parent_rel_to_repo(repo: &Path, manifest_path: &Path) -> Option<String> {
    let parent = manifest_path.parent()?;
    let rel = parent.strip_prefix(repo).ok()?;
    if rel.as_os_str().is_empty() {
        return Some(String::new());
    }
    let mut s = rel.to_string_lossy().replace('\\', "/");
    while s.ends_with('/') {
        s.pop();
    }
    Some(s)
}

/// True if a path reported by `git log --name-only` lies under the crate root `rel_dir`.
pub(crate) fn git_path_touches_crate_root(git_path: &str, rel_dir: &str) -> bool {
    let p = git_path.trim_start_matches("./").replace('\\', "/");
    let rel = rel_dir.trim_matches('/');
    if rel.is_empty() {
        // Root package (manifest at workspace root): touches are top-level source paths.
        return p == "Cargo.toml" || p.starts_with("src/") || p.starts_with("benches/");
    }
    p == rel || p.starts_with(&format!("{rel}/"))
}

/// Paths touched in commits selected by `git log --since {release_date}T00:00:00Z`
/// (`--name-only`, empty `--pretty`); matches Git's author-date `--since` semantics.
pub(crate) fn git_paths_touched_since(repo: &Path, release_date: &str) -> Option<HashSet<String>> {
    let since = format!("{release_date}T00:00:00Z");
    let git_bin = std::env::var("VOX_GIT_BIN").unwrap_or_else(|_| "git".to_string());
    let out = Command::new(&git_bin)
        .current_dir(repo)
        .args(["log", "--since", &since, "--name-only", "--pretty=format:"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut paths = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        paths.insert(line.replace('\\', "/"));
    }
    Some(paths)
}

/// hakari feature-unification crate: exempt both directions from fan-in tracking,
/// by design. Mirrors `vox_cli_ci::crate_edges::EXEMPT` (a separate crate at a
/// higher layer, so the constant is duplicated rather than depended on).
pub(crate) const FAN_IN_EXEMPT: &str = "workspace-hack";

/// Rule 2 fan-in counter: normal + build dependency edges among workspace
/// crates, deduped per (from, to) pair and excluding `FAN_IN_EXEMPT`. Mirrors
/// `vox_cli_ci::crate_edges::collect_live_edges`'s semantics (dev-deps don't
/// ship in the binary closure, so they don't count toward an architectural
/// fan-in budget) so this rule and `vox ci fan-in-budget` agree on what "fan-in"
/// means.
pub(crate) fn compute_dependent_counts(
    edges: &[(String, String, cargo_metadata::DependencyKind)],
) -> HashMap<String, usize> {
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (from, to, kind) in edges {
        if from == FAN_IN_EXEMPT || to == FAN_IN_EXEMPT {
            continue;
        }
        if !matches!(
            kind,
            cargo_metadata::DependencyKind::Normal | cargo_metadata::DependencyKind::Build
        ) {
            continue;
        }
        if seen.insert((from.as_str(), to.as_str())) {
            *counts.entry(to.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// Count lines in a byte slice with the same semantics as
/// `std::str::Lines::count()`: lines are split on `\n` (or `\r\n`), and a
/// trailing newline does NOT add an empty trailing line.
///
/// Used by Rule 13 to match what the original per-file `git show` + `lines().count()`
/// produced, now that we read blobs directly via `git cat-file --batch`.
pub(crate) fn count_lines_in_bytes(body: &[u8]) -> usize {
    if body.is_empty() {
        return 0;
    }
    let nl = body.iter().filter(|&&b| b == b'\n').count();
    if body.last() == Some(&b'\n') {
        nl
    } else {
        nl + 1
    }
}

/// Rule 13 helper — count LoC in every budgeted crate's `src/` tree at the given
/// git tag, using a single `git cat-file --batch` invocation.
///
/// Previously this was one `git show` *per file* per crate (potentially 900+
/// process spawns on this workspace). The batched approach uses two git
/// invocations total: one `git ls-tree -r` to enumerate paths at the tag, and
/// one `git cat-file --batch` fed all blob refs on stdin.
///
/// Returns `None` if git is unavailable or the tag doesn't exist; otherwise a
/// map from `manifest_dir` (the crate's directory) to its LoC at `tag`.
/// Crates whose `src/` had no `.rs` files at `tag` are absent from the map.
pub(crate) fn git_loc_at_tag_batch(
    tag: &str,
    workspace_root: &Path,
    manifest_dirs: &[PathBuf],
) -> Option<HashMap<PathBuf, usize>> {
    if manifest_dirs.is_empty() {
        return Some(HashMap::new());
    }
    // Build a {src_rel_path → manifest_dir} index so we can attribute lines.
    let mut src_to_manifest: HashMap<String, PathBuf> = HashMap::new();
    for md in manifest_dirs {
        let rel_src = md.join("src");
        if let Ok(stripped) = rel_src.strip_prefix(workspace_root)
            && let Some(s) = stripped.to_str()
        {
            src_to_manifest.insert(s.replace('\\', "/"), md.clone());
        }
    }
    if src_to_manifest.is_empty() {
        return None;
    }

    // 1) Single `git ls-tree -r --name-only <tag>` over the whole tree.
    // vox-arch-check: allow git-exec
    let ls = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", tag])
        .current_dir(workspace_root)
        .output()
        .ok()?;
    if !ls.status.success() {
        return None;
    }

    // 2) Collect `.rs` paths whose prefix matches a budgeted src dir.
    let mut blob_refs: Vec<String> = Vec::new();
    let mut blob_to_manifest: Vec<PathBuf> = Vec::new();
    for line in String::from_utf8_lossy(&ls.stdout).lines() {
        if !line.ends_with(".rs") {
            continue;
        }
        // Find longest matching src dir prefix.
        let mut owner: Option<&PathBuf> = None;
        for (src_rel, md) in &src_to_manifest {
            let prefix = format!("{src_rel}/");
            if line.starts_with(&prefix) {
                owner = Some(md);
                break;
            }
        }
        if let Some(md) = owner {
            blob_refs.push(format!("{tag}:{line}"));
            blob_to_manifest.push(md.clone());
        }
    }
    if blob_refs.is_empty() {
        return Some(HashMap::new());
    }

    // 3) One `git cat-file --batch`, feed all refs on stdin.
    // Drain stdout in a separate thread to avoid a deadlock when git's stdout
    // pipe buffer fills up faster than we can drain it (Windows pipes are ~4KB).
    // vox-arch-check: allow git-exec
    use std::io::{Read, Write};
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(workspace_root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let mut stdout = child.stdout.take()?;
    let blob_refs_for_thread = blob_refs.clone();
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        for r in &blob_refs_for_thread {
            stdin.write_all(r.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
        // Drop closes stdin → signals EOF to git
        Ok(())
    });

    let mut buf = Vec::new();
    stdout.read_to_end(&mut buf).ok()?;
    writer.join().ok()?.ok()?;
    let _ = child.wait();

    // 4) Parse output: each blob is `<sha> blob <size>\n<size bytes>\n`.
    // Count newlines within each blob's body and attribute to its manifest_dir.
    let mut result: HashMap<PathBuf, usize> = HashMap::new();
    let mut idx = 0usize;
    let mut blob_i = 0usize;
    while idx < buf.len() && blob_i < blob_to_manifest.len() {
        // Read header line up to '\n'
        let nl = match buf[idx..].iter().position(|&b| b == b'\n') {
            Some(p) => idx + p,
            None => break,
        };
        let header = match std::str::from_utf8(&buf[idx..nl]) {
            Ok(s) => s,
            Err(_) => break,
        };
        idx = nl + 1;
        // Header is like "<sha> blob <size>" or "<ref> missing"
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() == 3 && parts[1] == "blob" {
            let size: usize = parts[2].parse().ok()?;
            let body_end = (idx + size).min(buf.len());
            let lines = count_lines_in_bytes(&buf[idx..body_end]);
            let md = &blob_to_manifest[blob_i];
            *result.entry(md.clone()).or_insert(0) += lines;
            idx = body_end + 1; // skip trailing '\n' after blob body
        }
        blob_i += 1;
    }
    Some(result)
}

/// Rule 12 — three-way parity between `[crates]` in layers.toml, the
/// `crates/` directory on disk, and `where-things-live.md`.
///
/// Two directions are checked:
///
/// (a) Every `[crates]` entry in `layers.toml` whose `crates/<name>/`
///     directory does not exist on disk, unless the name is in `[planned]`.
/// (b) Every `crates/<name>/` occurrence in `where-things-live.md` whose
///     directory does not exist on disk, unless `<name>` is in `[planned]`
///     or in `[crates]` (the latter is already flagged by direction a).
pub(crate) fn check_wtl_parity(cfg: &LayersConfig, workspace_root: &Path) -> Vec<String> {
    use regex::Regex;

    let mut warns = Vec::new();
    let crates_dir = workspace_root.join("crates");

    // (a) layers.toml [crates] entries without a matching directory
    for name in cfg.crates.keys() {
        if name == "workspace-hack" {
            continue;
        }
        if !crates_dir.join(name).exists() && !cfg.planned.contains_key(name.as_str()) {
            warns.push(format!(
                "layers.toml [crates] has `{name}` but `crates/{name}/` does not exist on disk \
                 (add to [planned] if intended, or remove the entry)"
            ));
        }
    }

    // (b) where-things-live.md crate references without a matching directory
    let wtl_path = workspace_root.join("docs/src/architecture/where-things-live.md");
    if let Ok(wtl_body) = std::fs::read_to_string(&wtl_path) {
        // Match `crates/<name>/` patterns (name = alphanumeric + hyphens)
        let re = match Regex::new(r"crates/([a-zA-Z][a-zA-Z0-9-]+)/") {
            Ok(r) => r,
            Err(_) => return warns,
        };
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for cap in re.captures_iter(&wtl_body) {
            let cname = &cap[1];
            if !seen.insert(cname.to_string()) {
                continue;
            }
            if crates_dir.join(cname).exists() {
                continue;
            }
            if cfg.planned.contains_key(cname) {
                continue;
            }
            // Also skip if the layers.toml [crates] entry already covers it (direction a)
            if cfg.crates.contains_key(cname) {
                continue;
            }
            warns.push(format!(
                "where-things-live.md references `crates/{cname}/` but the directory does not \
                 exist and `{cname}` is not in [planned] (move row to 'Planned but not landed' \
                 section and add a [planned] entry)"
            ));
        }
    }

    warns.sort();
    warns
}

/// Return the YYYY-MM-DD **author** date of the last commit touching `dir`, or `None` if git is unavailable.
/// Uses author date so it matches `git log --since` filtering used by Rule 8 batching.
pub(crate) fn git_last_commit_date(dir: &Path) -> Option<String> {
    // vox-arch-check: allow git-exec
    let out = Command::new("git")
        .args(["log", "-n", "1", "--format=%ad", "--date=short"])
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
pub(crate) fn parse_release_date(changelog: &Path) -> Option<(String, String)> {
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
pub(crate) fn check_description_present(
    meta: &cargo_metadata::Metadata,
    cfg: &LayersConfig,
) -> Vec<String> {
    let mut findings = Vec::new();
    let workspace_ids: HashSet<&cargo_metadata::PackageId> =
        meta.workspace_members.iter().collect();
    for pkg in meta
        .packages
        .iter()
        .filter(|p| workspace_ids.contains(&p.id))
    {
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
pub(crate) fn check_where_things_live_coverage(
    meta: &cargo_metadata::Metadata,
    cfg: &LayersConfig,
    repo_root: &std::path::Path,
) -> anyhow::Result<Vec<String>> {
    let path = repo_root.join("docs/src/architecture/where-things-live.md");
    let body =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut findings = Vec::new();
    let workspace_ids: HashSet<&cargo_metadata::PackageId> =
        meta.workspace_members.iter().collect();
    for pkg in meta
        .packages
        .iter()
        .filter(|p| workspace_ids.contains(&p.id))
    {
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
pub(crate) fn count_loc(dir: &Path, prune_dir_names: &HashSet<String>) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut total = 0usize;
    for entry in walk_repo_files(dir, prune_dir_names) {
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
pub(crate) fn fnv1a_hex(bytes: &[u8]) -> String {
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
pub(crate) fn check_generated_file_drift(
    workspace_root: &Path,
    prune_dir_names: &HashSet<String>,
) -> anyhow::Result<Vec<(PathBuf, String, String)>> {
    const MARKER: &str = "@generated-hash ";
    const HASH_LEN: usize = 16;
    // Extensions that may carry generated-hash headers.
    const TRACKED_EXTS: &[&str] = &["rs", "ts", "tsx", "js", "vox", "md", "toml", "json"];

    let mut warns = Vec::new();

    for path in walk_repo_files(workspace_root, prune_dir_names) {
        let rel = match path.strip_prefix(workspace_root) {
            Ok(r) => r,
            Err(_) => path.as_path(),
        };
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

        let Some(marker_line) = header_line_idx else {
            continue;
        };

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

/// Rule 20: named build profiles must not contain forbidden crates.
pub(crate) fn check_profile_forbidden(
    profile: &str,
    tree: &[String],
    forbidden: &[String],
) -> Vec<String> {
    forbidden
        .iter()
        .filter(|f| tree.iter().any(|c| c == *f))
        .map(|f| format!("Rule 20: profile `{profile}` must not contain `{f}`"))
        .collect()
}
