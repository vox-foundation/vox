//! Zero-subprocess contextual shell and Vox completion engine (`TERM-WARP-03`).

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::completions_specs::{
    SHELL_BUILTINS, SLASH_COMMANDS, collect_git_refs, find_cli_spec, scan_path_executables,
};

const PATH_CACHE_TTL: Duration = Duration::from_secs(30);
const MAX_DIR_SCAN_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermCompletionKind {
    Executable,
    Builtin,
    Path,
    Directory,
    GitRef,
    Subcommand,
    Flag,
    VoxMacro,
    Slash,
    History,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermCompletionRequest {
    pub line: String,
    pub cursor: usize,
    pub cwd: Option<String>,
    #[serde(default, alias = "shellMode")]
    pub shell_mode: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermCompletionItem {
    pub label: String,
    pub insert_text: String,
    pub kind: TermCompletionKind,
    pub detail: Option<String>,
    pub score: i32,
    pub replace_start: usize,
    pub replace_end: usize,
}

fn mk_item(
    label: impl Into<String>,
    insert: impl Into<String>,
    kind: TermCompletionKind,
    detail: &str,
    score: i32,
    span: (usize, usize),
) -> TermCompletionItem {
    TermCompletionItem {
        label: label.into(),
        insert_text: insert.into(),
        kind,
        detail: Some(detail.to_string()),
        score,
        replace_start: span.0,
        replace_end: span.1,
    }
}

#[derive(Default)]
pub struct CompletionEngine {
    path_cache: Mutex<Option<(Instant, Vec<String>)>>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn complete(&self, req: &TermCompletionRequest) -> Vec<TermCompletionItem> {
        let limit = req.limit.unwrap_or(32).clamp(1, 100);
        let cursor = (0..=req.cursor.min(req.line.len()))
            .rev()
            .find(|&i| req.line.is_char_boundary(i))
            .unwrap_or(0);
        let (seg_offset, tokens, trailing_ws) = parse_active_segment(&req.line[..cursor]);
        let (token_idx, replace_start, prefix) = if trailing_ws || tokens.is_empty() {
            (tokens.len(), cursor, "")
        } else {
            let last = tokens[tokens.len() - 1];
            (tokens.len() - 1, last.0, last.1)
        };
        let span = (replace_start, cursor);
        let cmd0 = tokens.first().map_or("", |t| t.1);
        let base_cwd = req
            .cwd
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .and_then(|p| fs::canonicalize(p).ok());

        let mut items = Vec::new();
        if seg_offset == 0
            && token_idx == 0
            && prefix.starts_with('/')
            && !prefix[1..].contains('/')
        {
            for &(s, desc) in SLASH_COMMANDS {
                if let Some(sc) = prefix_score(s, prefix, 96) {
                    items.push(mk_item(s, s, TermCompletionKind::Slash, desc, sc, span));
                }
            }
        }

        let is_path_tok = prefix.contains('/')
            || prefix.starts_with('.')
            || prefix.starts_with('~')
            || (cfg!(windows) && prefix.contains('\\'))
            || (token_idx >= 1 && matches!(cmd0, "cd" | "ls" | "cat" | "source"));

        if is_path_tok {
            if let Some(ref cwd) = base_cwd {
                complete_paths(cwd, cmd0 == "cd", prefix, span, 86, &mut items);
            }
        } else if token_idx == 0 {
            for &(b, desc) in SHELL_BUILTINS {
                if let Some(sc) = prefix_score(b, prefix, 95) {
                    items.push(mk_item(b, b, TermCompletionKind::Builtin, desc, sc, span));
                }
            }
            for bin in self.cached_path_executables() {
                if let Some(sc) = prefix_score(&bin, prefix, 82) {
                    items.push(mk_item(
                        bin.clone(),
                        bin,
                        TermCompletionKind::Executable,
                        "$PATH",
                        sc,
                        span,
                    ));
                }
            }
        } else if !prefix.starts_with(['-', '$'])
            && let Some(ref cwd) = base_cwd
        {
            complete_paths(cwd, false, prefix, span, 83, &mut items);
        }

        if token_idx >= 1
            && let Some(spec) = find_cli_spec(cmd0)
        {
            let prev = tokens.get(token_idx - 1).map_or("", |t| t.1);
            let subcmd = tokens[1..token_idx]
                .iter()
                .map(|t| t.1)
                .find(|t| !t.starts_with('-'));
            if prefix.starts_with('-') {
                for &(fl, desc) in spec.flags {
                    if let Some(sc) = prefix_score(fl, prefix, 90) {
                        items.push(mk_item(fl, fl, TermCompletionKind::Flag, desc, sc, span));
                    }
                }
            } else if subcmd.is_none() {
                for &(sub, desc) in spec.subcommands {
                    if let Some(sc) = prefix_score(sub, prefix, 92) {
                        items.push(mk_item(
                            sub,
                            sub,
                            TermCompletionKind::Subcommand,
                            desc,
                            sc,
                            span,
                        ));
                    }
                }
            }
            let wants_ref = spec.cmd == "git"
                && !prefix.starts_with('-')
                && (matches!(prev, "--branch" | "-b")
                    || subcmd.is_some_and(|sc| spec.git_ref_subcommands.contains(&sc)));
            if wants_ref && let Some(ref cwd) = base_cwd {
                for (r, kind_label) in collect_git_refs(cwd) {
                    if let Some(sc) = prefix_score(&r, prefix, 88) {
                        items.push(mk_item(
                            r.clone(),
                            r,
                            TermCompletionKind::GitRef,
                            kind_label,
                            sc,
                            span,
                        ));
                    }
                }
            }
        }

        items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));
        let mut deduped: Vec<TermCompletionItem> = Vec::with_capacity(items.len().min(limit));
        for item in items {
            if !deduped.iter().any(|e| e.insert_text == item.insert_text) {
                deduped.push(item);
                if deduped.len() >= limit {
                    break;
                }
            }
        }
        deduped
    }

    fn cached_path_executables(&self) -> Vec<String> {
        let mut guard = self.path_cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((ts, ref list)) = *guard
            && ts.elapsed() < PATH_CACHE_TTL
        {
            return list.clone();
        }
        let scanned = scan_path_executables();
        *guard = Some((Instant::now(), scanned.clone()));
        scanned
    }
}

fn prefix_score(candidate: &str, prefix: &str, base: i32) -> Option<i32> {
    if prefix.is_empty() {
        Some(base - 10)
    } else if candidate.starts_with(prefix) {
        Some(base + i32::from(candidate == prefix) * 5)
    } else if candidate
        .to_ascii_lowercase()
        .starts_with(&prefix.to_ascii_lowercase())
    {
        Some(base - 8)
    } else {
        None
    }
}

fn parse_active_segment(head: &str) -> (usize, Vec<(usize, &str)>, bool) {
    let b = head.as_bytes();
    let (mut seg_start, mut sq, mut dq, mut esc, mut i) = (0, false, false, false, 0);
    while i < b.len() {
        if esc {
            esc = false;
        } else if b[i] == b'\\' && !sq {
            esc = true;
        } else if b[i] == b'\'' && !dq {
            sq = !sq;
        } else if b[i] == b'"' && !sq {
            dq = !dq;
        } else if !sq && !dq {
            if b[i] == b';' {
                seg_start = i + 1;
            } else if b[i] == b'|' {
                let step = if b.get(i + 1) == Some(&b'|') { 2 } else { 1 };
                seg_start = i + step;
                i += step;
                continue;
            } else if b[i] == b'&' && b.get(i + 1) == Some(&b'&') {
                seg_start = i + 2;
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    let mut slice_start = seg_start;
    if seg_start == 0 {
        let trimmed = head.trim_start();
        let lead = head.len() - trimmed.len();
        if trimmed.starts_with("!nu ") {
            slice_start = lead + 4;
        } else if trimmed.starts_with(['!', '?']) {
            slice_start = lead + 1;
        }
    }
    let seg = &head[slice_start..];
    let (mut tokens, mut tok_start) = (Vec::new(), None);
    for (rel, ch) in seg.char_indices() {
        if ch.is_whitespace() {
            if let Some(st) = tok_start.take() {
                tokens.push((slice_start + st, &seg[st..rel]));
            }
        } else if tok_start.is_none() {
            tok_start = Some(rel);
        }
    }
    let trailing_ws = tok_start.is_none();
    if let Some(st) = tok_start {
        tokens.push((slice_start + st, &seg[st..]));
    }
    (seg_start, tokens, trailing_ws)
}

fn complete_paths(
    base_cwd: &Path,
    dirs_only: bool,
    prefix: &str,
    span: (usize, usize),
    base_score: i32,
    out: &mut Vec<TermCompletionItem>,
) {
    let (dir_part, leaf_prefix) = if prefix == "~" {
        ("~/", "")
    } else {
        prefix
            .rfind(['/', '\\'])
            .map_or(("", prefix), |idx| (&prefix[..=idx], &prefix[idx + 1..]))
    };
    let (target_dir, allowed_root) = if let Some(rest) = dir_part.strip_prefix("~/") {
        let Some(home) = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .and_then(|h| fs::canonicalize(h).ok())
        else {
            return;
        };
        if prefix == "~" {
            out.push(mk_item(
                "~/",
                "~/",
                TermCompletionKind::Directory,
                "home directory",
                base_score + 6,
                span,
            ));
        }
        (home.join(rest), home)
    } else if Path::new(dir_part).is_absolute() {
        (PathBuf::from(dir_part), base_cwd.to_path_buf())
    } else {
        let hops = Path::new(dir_part)
            .components()
            .filter(|c| matches!(c, Component::ParentDir))
            .count();
        if hops > 1 {
            return;
        }
        let root = if hops == 1 {
            base_cwd.parent().unwrap_or(base_cwd)
        } else {
            base_cwd
        };
        (base_cwd.join(dir_part), root.to_path_buf())
    };
    let Ok(canon_dir) = fs::canonicalize(&target_dir) else {
        return;
    };
    if !canon_dir.starts_with(&allowed_root) {
        return;
    }
    let Ok(entries) = fs::read_dir(&canon_dir) else {
        return;
    };
    for entry in entries.flatten().take(MAX_DIR_SCAN_ENTRIES) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && !leaf_prefix.starts_with('.') {
            continue;
        }
        let Some(score) = prefix_score(&name, leaf_prefix, base_score) else {
            continue;
        };
        let is_dir = entry.path().is_dir();
        if dirs_only && !is_dir {
            continue;
        }
        if is_dir {
            out.push(mk_item(
                format!("{name}/"),
                format!("{dir_part}{name}/"),
                TermCompletionKind::Directory,
                "directory",
                score + 2,
                span,
            ));
        } else {
            out.push(mk_item(
                &name,
                format!("{dir_part}{name}"),
                TermCompletionKind::Path,
                "file",
                score,
                span,
            ));
        }
    }
}

#[cfg(test)]
#[path = "completions_tests.rs"]
mod tests;
