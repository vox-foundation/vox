//! Bounded pure-Rust workspace Runbook discovery and `{{param}}` extraction (`TERM-WARP-04`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MAX_RUNBOOKS: usize = 128;
const MAX_RUNBOOK_FILE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunbookKind {
    #[serde(rename = "vox")]
    Vox,
    #[serde(rename = "markdown")]
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunbookParam {
    pub name: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunbookDescriptor {
    pub id: String,
    pub title: String,
    pub description: String,
    pub path: String,
    pub kind: RunbookKind,
    pub command_template: String,
    pub params: Vec<RunbookParam>,
    pub tags: Vec<String>,
}

pub fn scan_workspace_runbooks(workspace_root: &Path) -> Vec<RunbookDescriptor> {
    let Ok(canon_root) = fs::canonicalize(workspace_root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for rel_dir in ["scripts", ".vox/runbooks", "runbooks"] {
        for (rel_path, content) in read_confined_files(&canon_root, rel_dir, "vox") {
            out.push(parse_vox_runbook(&rel_path, &content));
        }
    }
    for (rel_dir, req_tag) in [
        (".vox/runbooks", false),
        ("runbooks", false),
        ("docs/src/how-to", true),
    ] {
        for (rel_path, content) in read_confined_files(&canon_root, rel_dir, "md") {
            if let Some(desc) = parse_markdown_runbook(&rel_path, &content, req_tag) {
                out.push(desc);
            }
        }
    }
    out.sort_by(|a, b| (&a.kind, &a.title, &a.path).cmp(&(&b.kind, &b.title, &b.path)));
    out.dedup_by(|a, b| a.path == b.path);
    out.truncate(MAX_RUNBOOKS);
    out
}

fn collect_ext_files(
    dir: &Path,
    canon_root: &Path,
    ext: &str,
    depth: usize,
    acc: &mut Vec<(String, PathBuf)>,
) {
    let Ok(canon_dir) = fs::canonicalize(dir) else {
        return;
    };
    if !canon_dir.starts_with(canon_root) {
        return;
    }
    let Ok(rd) = fs::read_dir(&canon_dir) else {
        return;
    };
    for entry in rd.flatten() {
        let candidate = entry.path();
        let Ok(canon) = fs::canonicalize(&candidate) else {
            continue;
        };
        if !canon.starts_with(canon_root) {
            continue;
        }
        let Ok(meta) = fs::metadata(&canon) else {
            continue;
        };
        if meta.is_dir() {
            if depth < 3 {
                collect_ext_files(&canon, canon_root, ext, depth + 1, acc);
            }
        } else if meta.is_file()
            && meta.len() <= MAX_RUNBOOK_FILE_BYTES
            && canon
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(ext))
            && let Ok(rel) = canon.strip_prefix(canon_root)
        {
            let rel_slash = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            acc.push((rel_slash, canon));
        }
    }
}

fn read_confined_files(canon_root: &Path, rel_dir: &str, ext: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    collect_ext_files(&canon_root.join(rel_dir), canon_root, ext, 0, &mut found);
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found.dedup_by(|a, b| a.0 == b.0);
    found.truncate(MAX_RUNBOOKS * 2);
    found
        .into_iter()
        .filter_map(|(rel, canon)| fs::read_to_string(&canon).ok().map(|c| (rel, c)))
        .collect()
}

fn tag_val<'a>(c: &'a str, tag: &str) -> Option<&'a str> {
    c.strip_prefix(tag).map(str::trim).filter(|v| !v.is_empty())
}

fn parse_vox_runbook(rel_path: &str, content: &str) -> RunbookDescriptor {
    let stem = Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(rel_path);
    let (mut title, mut description, mut cmd_override) = (None, None, None);
    let (mut tags, mut params) = (Vec::new(), Vec::new());

    for line in content.lines().map(str::trim) {
        let Some(c) = line
            .strip_prefix("//!")
            .or_else(|| line.strip_prefix("//"))
            .map(str::trim)
        else {
            if !line.is_empty() {
                break;
            }
            continue;
        };
        if let Some(v) = tag_val(c, "@title") {
            title = Some(v.to_string());
        } else if let Some(v) = tag_val(c, "@desc").or_else(|| tag_val(c, "@description")) {
            description = Some(v.to_string());
        } else if let Some(v) = tag_val(c, "@cmd") {
            cmd_override = Some(v.to_string());
        } else if let Some(v) = tag_val(c, "@tags") {
            tags.extend(parse_inline_list(v));
        } else if let Some(v) = tag_val(c, "@param") {
            if let Some(p) = parse_param_spec(v) {
                upsert_param(&mut params, p);
            }
        } else if description.is_none() && !c.is_empty() && !c.starts_with('@') {
            description = Some(c.to_string());
        }
    }

    collect_mustache_placeholders(content, &mut params);
    let command_template = cmd_override.unwrap_or_else(|| {
        if params.is_empty() {
            format!("vox run {rel_path}")
        } else {
            let flags = params
                .iter()
                .map(|p| format!("--{} {{{{{}}}}}", p.name, p.name))
                .collect::<Vec<_>>()
                .join(" ");
            format!("vox run {rel_path} -- {flags}")
        }
    });
    collect_mustache_placeholders(&command_template, &mut params);
    RunbookDescriptor {
        id: rel_path.to_string(),
        title: title.unwrap_or_else(|| humanize_stem(stem)),
        description: description.unwrap_or_default(),
        path: rel_path.to_string(),
        kind: RunbookKind::Vox,
        command_template,
        params,
        tags,
    }
}

fn parse_markdown_runbook(
    rel_path: &str,
    content: &str,
    require_tag: bool,
) -> Option<RunbookDescriptor> {
    let (fm, body) = split_frontmatter(content);
    let (title, description, tags, mut params, is_rb) = parse_frontmatter(fm);
    if require_tag && !is_rb && !content.contains("<!-- @runbook -->") {
        return None;
    }
    let command_template = extract_fenced_commands(body).join("\n");
    if command_template.trim().is_empty() {
        return None;
    }
    let stem = Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(rel_path);
    let heading = body
        .lines()
        .find_map(|l| l.trim().strip_prefix("# ").map(str::trim))
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    collect_mustache_placeholders(&command_template, &mut params);
    collect_mustache_placeholders(body, &mut params);
    Some(RunbookDescriptor {
        id: rel_path.to_string(),
        title: title.or(heading).unwrap_or_else(|| humanize_stem(stem)),
        description: description.unwrap_or_default(),
        path: rel_path.to_string(),
        kind: RunbookKind::Markdown,
        command_template,
        params,
        tags,
    })
}

fn split_frontmatter(content: &str) -> (&str, &str) {
    let s = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = s
        .strip_prefix("---\n")
        .or_else(|| s.strip_prefix("---\r\n"))
    else {
        return ("", s);
    };
    let Some(end) = rest.find("\n---") else {
        return ("", s);
    };
    (
        &rest[..end],
        rest[end + 4..].trim_start_matches(['\r', '\n']),
    )
}

type FrontmatterParsed = (
    Option<String>,
    Option<String>,
    Vec<String>,
    Vec<RunbookParam>,
    bool,
);

fn parse_frontmatter(fm: &str) -> FrontmatterParsed {
    let (mut title, mut desc, mut is_rb) = (None, None, fm.contains("@runbook"));
    let (mut tags, mut params, mut section) = (Vec::new(), Vec::new(), "");
    for raw in fm.lines() {
        let trimmed = raw.trim();
        let indented = raw.starts_with([' ', '\t']);
        if indented
            && section == "tags"
            && let Some(item) = trimmed.strip_prefix("- ")
        {
            let t = unquote(item.trim());
            if !t.is_empty() {
                tags.push(t);
            }
            continue;
        }
        if indented && section == "params" {
            if let Some(rest) = trimmed.strip_prefix("- ").map(str::trim) {
                if let Some(n) = rest.strip_prefix("name:") {
                    let name = unquote(n.trim());
                    if is_valid_ident(&name) {
                        upsert_param(
                            &mut params,
                            RunbookParam {
                                name,
                                default_value: None,
                                description: None,
                                options: Vec::new(),
                            },
                        );
                    }
                } else if let Some(sp) = parse_param_spec(&unquote(rest)) {
                    upsert_param(&mut params, sp);
                }
            } else if let (Some(last), Some((pk, pv))) =
                (params.last_mut(), trimmed.split_once(':'))
            {
                let pv = unquote(pv.trim());
                match pk.trim() {
                    "default" | "default_value" if !pv.is_empty() => last.default_value = Some(pv),
                    "description" if !pv.is_empty() => last.description = Some(pv),
                    "options" => last.options = parse_inline_list(&pv),
                    _ => {}
                }
            }
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            section = k.trim();
            let val = unquote(v.trim());
            match section {
                "title" if !val.is_empty() => title = Some(val),
                "description" if !val.is_empty() => desc = Some(val),
                "runbook" if val.eq_ignore_ascii_case("true") => is_rb = true,
                "tags" => tags.extend(parse_inline_list(&val)),
                "params" => {
                    for sp in parse_inline_list(&val)
                        .into_iter()
                        .filter_map(|p| parse_param_spec(&p))
                    {
                        upsert_param(&mut params, sp);
                    }
                }
                _ => {}
            }
        }
    }
    is_rb |= tags.iter().any(|t| t.eq_ignore_ascii_case("runbook"));
    (title, desc, tags, params, is_rb)
}

fn extract_fenced_commands(body: &str) -> Vec<String> {
    let (mut blocks, mut active, mut buf) = (Vec::new(), false, Vec::new());
    for line in body.lines() {
        if let Some(fence) = line.trim_start().strip_prefix("```") {
            if std::mem::take(&mut active) {
                let joined = std::mem::take(&mut buf).join("\n");
                if !joined.trim().is_empty() {
                    blocks.push(joined.trim().to_string());
                }
            } else {
                let l = fence.split_whitespace().next().unwrap_or("");
                active = [
                    "sh",
                    "bash",
                    "zsh",
                    "fish",
                    "pwsh",
                    "powershell",
                    "nu",
                    "vox",
                    "shell",
                ]
                .iter()
                .any(|k| l.eq_ignore_ascii_case(k));
            }
        } else if active {
            buf.push(line);
        }
    }
    blocks
}

fn non_empty(s: &str) -> Option<String> {
    let u = unquote(s.trim());
    (!u.is_empty()).then_some(u)
}

fn parse_param_spec(spec: &str) -> Option<RunbookParam> {
    let mut segs = spec.split('|').map(str::trim);
    let head = segs.next()?;
    let (name, default_value) = head
        .split_once('=')
        .map_or((head, None), |(n, d)| (n.trim(), non_empty(d)));
    if !is_valid_ident(name) {
        return None;
    }
    let (options, description) = match (segs.next(), segs.next()) {
        (Some(o), Some(d)) => (parse_inline_list(o), non_empty(d)),
        (Some(x), None) if x.contains(',') => (parse_inline_list(x), None),
        (Some(d), None) => (Vec::new(), non_empty(d)),
        _ => (Vec::new(), None),
    };
    Some(RunbookParam {
        name: name.to_string(),
        default_value,
        description,
        options,
    })
}

fn collect_mustache_placeholders(text: &str, params: &mut Vec<RunbookParam>) {
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else { break };
        let inner = after[..close].trim();
        let (ident, default_value) = inner
            .split_once('=')
            .map_or((inner, None), |(n, d)| (n.trim(), non_empty(d)));
        if is_valid_ident(ident) {
            let p = RunbookParam {
                name: ident.to_string(),
                default_value,
                description: None,
                options: Vec::new(),
            };
            upsert_param(params, p);
        }
        rest = &after[close + 2..];
    }
}

fn upsert_param(params: &mut Vec<RunbookParam>, incoming: RunbookParam) {
    if let Some(ex) = params.iter_mut().find(|p| p.name == incoming.name) {
        ex.default_value = ex.default_value.take().or(incoming.default_value);
        ex.description = ex.description.take().or(incoming.description);
        if ex.options.is_empty() {
            ex.options = incoming.options;
        }
    } else {
        params.push(incoming);
    }
}

fn is_valid_ident(s: &str) -> bool {
    let mut ch = s.chars();
    ch.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && ch.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_inline_list(raw: &str) -> Vec<String> {
    raw.trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(raw)
        .split(',')
        .map(|s| unquote(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn unquote(s: &str) -> String {
    s.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(s)
        .to_string()
}

fn humanize_stem(stem: &str) -> String {
    stem.split(['-', '_'])
        .filter_map(|w| {
            w.chars()
                .next()
                .map(|f| format!("{}{}", f.to_ascii_uppercase(), &w[f.len_utf8()..]))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "runbooks_tests.rs"]
mod tests;
