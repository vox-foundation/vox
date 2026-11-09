//! Gated check for CLI claims in documentation and agent instructions.
//! Ensures `vox ...` mentions name real commands and flags, and flags stale "planned" claims.

use anyhow::{Context, Result, bail};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use vox_doc_pipeline::claims::{
    ExtractedMention, MentionKind, MentionRole, extract_mentions, is_reader_facing_page,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct CompiledNode {
    pub(crate) path: Vec<String>,
    pub(crate) has_subcommands: bool,
    pub(crate) takes_positional: bool,
    pub(crate) long_flags: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CliSurface {
    pub(crate) compiled: HashMap<String, CompiledNode>,
    pub(crate) registry_only: HashSet<String>,
    pub(crate) registry_retired: HashSet<String>,
    pub(crate) root_long_flags: HashSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CliVerdict {
    Ok,
    UnknownCommand(String),
    UnknownSubcommand { parent: String, token: String },
    UnknownFlag { path: String, flag: String },
    Retired(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    AgentFiles,
    ReaderFacing,
}

pub(crate) fn is_vox_cli_invocation(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.starts_with("vox ") || trimmed == "vox" {
        return true;
    }
    if let Some(rest) = trimmed.strip_prefix("cargo run") {
        let rest = rest.trim_start();
        if let Some(after_p) = rest.strip_prefix("-p") {
            let pkg = after_p.split_whitespace().next().unwrap_or("");
            return pkg == "vox-cli" || pkg == "vox";
        }
    }
    false
}

fn is_placeholder(token: &str) -> bool {
    let t = token.trim().trim_matches('"').trim_matches('\'');
    if t.is_empty() {
        return true;
    }
    if (t.starts_with('<') && t.ends_with('>'))
        || (t.starts_with('[') && t.ends_with(']'))
        || (t.starts_with('{') && t.ends_with('}'))
        || t.starts_with('$')
        || t == "..."
        || t.starts_with("...")
        || t.ends_with("...")
    {
        return true;
    }
    if t.contains('/') || t.contains('\\') || t.contains('.') {
        return true;
    }
    if t.chars()
        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        && t.len() > 1
    {
        return true;
    }
    false
}

pub(crate) fn check_command(surface: &CliSurface, mention: &str) -> CliVerdict {
    let trimmed = mention.trim().trim_matches('`').trim();

    // Strip environment variables if present at start (e.g. CI=1 vox ...)
    let mut rest = trimmed;
    while let Some(space_idx) = rest.find(' ') {
        let first_word = &rest[..space_idx];
        if first_word.contains('=') && !first_word.starts_with('-') {
            rest = rest[space_idx + 1..].trim_start();
        } else {
            break;
        }
    }

    // Strip prefix
    if let Some(after) = rest.strip_prefix("vox") {
        rest = after.trim_start();
    } else if let Some(after) = rest.strip_prefix("cargo run") {
        let mut s = after.trim_start();
        if let Some(after_p) = s.strip_prefix("-p") {
            s = after_p.trim_start();
            if let Some(after_pkg) = s.strip_prefix("vox-cli") {
                s = after_pkg.trim_start();
            } else if let Some(after_pkg) = s.strip_prefix("vox") {
                s = after_pkg.trim_start();
            } else {
                return CliVerdict::Ok;
            }
        }
        if let Some(after_dash) = s.strip_prefix("--") {
            s = after_dash.trim_start();
        }
        rest = s;
    } else {
        return CliVerdict::Ok;
    }

    if rest.is_empty() {
        return CliVerdict::Ok;
    }

    let raw_tokens: Vec<&str> = rest.split_whitespace().collect();
    if raw_tokens.is_empty() {
        return CliVerdict::Ok;
    }

    // Stop at first shell operator token
    let mut tokens = Vec::new();
    for t in raw_tokens {
        let clean = t.trim_end_matches(';');
        if clean == "|"
            || clean == ">"
            || clean == "<"
            || clean == ">>"
            || clean == "&&"
            || clean == "||"
            || clean == "&"
            || clean == "2>&1"
            || clean == "→"
            || clean == "->"
            || clean.starts_with('|')
            || clean.starts_with('>')
            || clean.starts_with('<')
        {
            break;
        }
        if !clean.is_empty() {
            tokens.push(clean);
        }
        if t.ends_with(';') {
            break;
        }
    }

    if tokens.is_empty() {
        return CliVerdict::Ok;
    }

    // Skip leading root flags (tokens starting with '-' before the first command token)
    while !tokens.is_empty() && tokens[0].starts_with('-') {
        tokens.remove(0);
    }

    if tokens.is_empty() {
        return CliVerdict::Ok;
    }

    // Find the longest matching command path
    let mut best_match: Option<(usize, String)> = None;
    for i in (1..=tokens.len()).rev() {
        let candidate = tokens[..i].join(" ");
        if surface.compiled.contains_key(&candidate)
            || surface.registry_only.contains(&candidate)
            || surface.registry_retired.contains(&candidate)
        {
            best_match = Some((i, candidate));
            break;
        }
    }

    let (matched_tokens_count, matched_path) = match best_match {
        Some(m) => m,
        None => {
            // Check if root token is a prefix of any known command
            let root_tok = tokens[0];
            let is_prefix = surface
                .compiled
                .keys()
                .chain(surface.registry_only.iter())
                .chain(surface.registry_retired.iter())
                .any(|k| k.split_whitespace().next() == Some(root_tok));
            if is_prefix {
                let second = tokens.get(1).copied().unwrap_or("");
                return CliVerdict::UnknownSubcommand {
                    parent: root_tok.to_string(),
                    token: second.to_string(),
                };
            }
            return CliVerdict::UnknownCommand(root_tok.to_string());
        }
    };

    if surface.registry_retired.contains(&matched_path) {
        return CliVerdict::Retired(matched_path);
    }

    if surface.registry_only.contains(&matched_path) {
        // Registry-only path: existence verified; flags not checked
        return CliVerdict::Ok;
    }

    // Delegated commands that forward trailing arguments/flags to sidecar binaries
    let is_delegated = matched_path == "populi"
        || matched_path == "mens"
        || matched_path == "speech"
        || matched_path == "oratio";
    if is_delegated {
        return CliVerdict::Ok;
    }

    let node = &surface.compiled[&matched_path];
    let remaining = &tokens[matched_tokens_count..];

    for rem in remaining {
        if *rem == "--" {
            // Forwarding separator: remaining tokens are for child command
            break;
        }
        if rem.starts_with("--") {
            let flag_part = rem.trim_start_matches("--").split('=').next().unwrap_or("");
            if flag_part == "help" || flag_part == "version" {
                continue;
            }
            let in_node = node.long_flags.contains(flag_part);
            let in_root = surface.root_long_flags.contains(flag_part);
            let in_ancestor = (1..node.path.len()).any(|i| {
                let ancestor_key = node.path[..i].join(" ");
                surface
                    .compiled
                    .get(&ancestor_key)
                    .is_some_and(|anc| anc.long_flags.contains(flag_part))
            });
            if !in_node && !in_root && !in_ancestor {
                return CliVerdict::UnknownFlag {
                    path: matched_path,
                    flag: format!("--{flag_part}"),
                };
            }
            continue;
        }
        if rem.starts_with('-') {
            // Short flag, not checked
            continue;
        }
        if is_placeholder(rem) {
            continue;
        }
        if node.has_subcommands && !node.takes_positional {
            return CliVerdict::UnknownSubcommand {
                parent: matched_path,
                token: rem.to_string(),
            };
        }
    }

    CliVerdict::Ok
}

pub(crate) fn stale_planned_claims(
    content: &str,
    mentions: &[ExtractedMention],
    surface: &CliSurface,
) -> Vec<String> {
    let mut findings = Vec::new();
    for mention in mentions {
        if mention.kind != MentionKind::CliCommand {
            continue;
        }
        if !is_vox_cli_invocation(&mention.raw_text) {
            continue;
        }
        if mention.fence_info.is_some() {
            continue;
        }
        if mention.byte_offset >= content.len() {
            continue;
        }
        let after_code = &content[mention.byte_offset..];
        let after_open = after_code.trim_start_matches('`');
        let Some(close_idx) = after_open.find('`') else {
            continue;
        };
        let after_close = &after_open[close_idx..];
        let rest_of_line = after_close.trim_start_matches('`');
        let line_end = rest_of_line.find('\n').unwrap_or(rest_of_line.len());
        let line_rest = &rest_of_line[..line_end];

        let trimmed = line_rest.trim_start_matches(|c: char| {
            c == ' ' || c == '\t' || c == '-' || c == '—' || c == '–' || c == ':'
        });
        let distance = line_rest.len() - trimmed.len();
        if distance <= 8 {
            let lower = trimmed.to_lowercase();
            if lower.starts_with("(planned") || lower.starts_with("planned per") {
                if check_command(surface, &mention.raw_text) == CliVerdict::Ok {
                    findings.push(format!(
                        "{}:{}: `{}` exists in the CLI but is described as planned",
                        mention.source_file, mention.line_number, mention.raw_text
                    ));
                }
            }
        }
    }
    findings
}

fn extract_frontmatter_status(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("status:") {
            return Some(val.trim().to_string());
        }
    }
    None
}

pub(crate) fn scope_files(root: &Path, scope: Scope) -> Result<Vec<PathBuf>> {
    match scope {
        Scope::AgentFiles => {
            let direct = [
                "AGENTS.md",
                "CLAUDE.md",
                "GEMINI.md",
                ".github/copilot-instructions.md",
                "docs/src/AGENTS.md",
            ];
            let mut files = Vec::new();
            for rel in direct {
                let p = root.join(rel);
                if !p.is_file() {
                    bail!("missing mandatory agent file: {}", p.display());
                }
                files.push(p);
            }
            let cursor_rules_dir = root.join(".cursor/rules");
            if !cursor_rules_dir.is_dir() {
                bail!("missing .cursor/rules directory");
            }
            let mut mdc_count = 0;
            for entry in std::fs::read_dir(&cursor_rules_dir)
                .with_context(|| format!("read_dir {}", cursor_rules_dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "mdc") {
                    files.push(path);
                    mdc_count += 1;
                }
            }
            if mdc_count == 0 {
                bail!("no .cursor/rules/*.mdc files found");
            }
            files.sort();
            Ok(files)
        }
        Scope::ReaderFacing => {
            let docs_src = root.join("docs/src");
            if !docs_src.is_dir() {
                bail!("missing docs/src directory");
            }
            let mut files = Vec::new();
            for entry in walkdir::WalkDir::new(&docs_src)
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let ext = path.extension().unwrap_or_default();
                if ext != "md" && ext != "mdx" {
                    continue;
                }
                let norm = path.to_string_lossy().replace('\\', "/");
                if norm.contains("/archive/") {
                    continue;
                }
                if !is_reader_facing_page(&norm) {
                    continue;
                }
                let content = match vox_bounded_fs::read_utf8_path_capped(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if let Some(status) = extract_frontmatter_status(&content) {
                    let s = status.trim().trim_matches('"').trim_matches('\'');
                    if s == "current" {
                        files.push(path.to_path_buf());
                    }
                }
            }
            files.sort();
            Ok(files)
        }
    }
}

pub(crate) fn cli_surface(root: &Path) -> Result<CliSurface> {
    use clap::CommandFactory;

    let catalog = crate::command_catalog::build_catalog();
    let mut compiled = HashMap::new();
    for entry in catalog.entries {
        let key = entry.path.join(" ");
        let mut long_flags = HashSet::new();
        let mut takes_positional = false;
        for arg in entry.arguments {
            if let Some(long) = arg.long {
                long_flags.insert(long);
            } else if arg.short.is_none() {
                takes_positional = true;
            }
        }
        let node = CompiledNode {
            path: entry.path.clone(),
            has_subcommands: entry.has_subcommands,
            takes_positional,
            long_flags,
        };
        compiled.insert(key, node.clone());
        for alias in &entry.aliases {
            let mut alias_path = entry.path.clone();
            if let Some(last) = alias_path.last_mut() {
                *last = alias.clone();
                compiled.insert(alias_path.join(" "), node.clone());
            }
        }
    }

    let root_cmd = crate::VoxCliRoot::command();
    let mut root_long_flags = HashSet::new();
    for arg in root_cmd.get_arguments() {
        if let Some(long) = arg.get_long() {
            root_long_flags.insert(long.to_string());
        }
    }

    let reg_path = root.join("contracts/cli/command-registry.yaml");
    let reg_raw = vox_bounded_fs::read_utf8_path_capped(&reg_path)
        .with_context(|| format!("read {}", reg_path.display()))?;
    let reg: crate::command_registry_model::RegistryFile =
        serde_yaml::from_str(&reg_raw).context("parse contracts/cli/command-registry.yaml")?;

    let mut registry_only = HashSet::new();
    let mut registry_retired = HashSet::new();

    for op in reg.operations {
        if op.surface == "vox-cli" || op.surface.contains("cli") {
            let path_key = op.path.join(" ");
            if op.status == "retired" {
                registry_retired.insert(path_key);
            } else if !compiled.contains_key(&path_key) {
                registry_only.insert(path_key);
            }
        }
    }

    Ok(CliSurface {
        compiled,
        registry_only,
        registry_retired,
        root_long_flags,
    })
}

fn is_under_retired_heading(content: &str, byte_offset: usize) -> bool {
    let before = &content[..byte_offset.min(content.len())];
    for line in before.lines().rev() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let lower = trimmed.to_lowercase();
            return lower.contains("retired")
                || lower.contains("deprecated")
                || lower.contains("historical");
        }
    }
    false
}

pub(crate) fn run(root: &Path) -> Result<()> {
    let surface = cli_surface(root)?;
    let scopes = [Scope::AgentFiles, Scope::ReaderFacing];
    let mut fatal_findings = Vec::new();

    for scope in scopes {
        let files = scope_files(root, scope)?;
        let mut scope_findings = Vec::new();

        for file in files {
            let content = vox_bounded_fs::read_utf8_path_capped(&file)
                .with_context(|| format!("read {}", file.display()))?;
            let mentions = extract_mentions(&file, &content);

            let planned = stale_planned_claims(&content, &mentions, &surface);
            scope_findings.extend(planned);

            for mention in mentions {
                if mention.kind != MentionKind::CliCommand {
                    continue;
                }
                if !is_vox_cli_invocation(&mention.raw_text) {
                    continue;
                }
                if mention.role != MentionRole::Prescribes {
                    continue;
                }
                if is_under_retired_heading(&content, mention.byte_offset) {
                    continue;
                }
                match check_command(&surface, &mention.raw_text) {
                    CliVerdict::Ok => {}
                    CliVerdict::UnknownCommand(cmd) => {
                        scope_findings.push(format!(
                            "{}:{}: unknown command `vox {}`",
                            mention.source_file, mention.line_number, cmd
                        ));
                    }
                    CliVerdict::UnknownSubcommand { parent, token } => {
                        scope_findings.push(format!(
                            "{}:{}: unknown subcommand `{}` under `vox {}`",
                            mention.source_file, mention.line_number, token, parent
                        ));
                    }
                    CliVerdict::UnknownFlag { path, flag } => {
                        scope_findings.push(format!(
                            "{}:{}: unknown flag `{}` for `vox {}`",
                            mention.source_file, mention.line_number, flag, path
                        ));
                    }
                    CliVerdict::Retired(cmd) => {
                        scope_findings.push(format!(
                            "{}:{}: command `vox {}` is retired",
                            mention.source_file, mention.line_number, cmd
                        ));
                    }
                }
            }
        }

        fatal_findings.extend(scope_findings);
    }

    if !fatal_findings.is_empty() {
        for f in &fatal_findings {
            eprintln!("docs_cli_claims: FAIL {f}");
        }
        bail!("docs_cli_claims: {} finding(s)", fatal_findings.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_surface() -> CliSurface {
        let mut compiled = HashMap::new();
        compiled.insert(
            "audit".to_string(),
            CompiledNode {
                path: vec!["audit".to_string()],
                has_subcommands: true,
                takes_positional: false,
                long_flags: ["json".to_string()].into_iter().collect(),
            },
        );
        compiled.insert(
            "audit retirement".to_string(),
            CompiledNode {
                path: vec!["audit".to_string(), "retirement".to_string()],
                has_subcommands: false,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "audit effort".to_string(),
            CompiledNode {
                path: vec!["audit".to_string(), "effort".to_string()],
                has_subcommands: false,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "ci".to_string(),
            CompiledNode {
                path: vec!["ci".to_string()],
                has_subcommands: true,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "ci pre-push".to_string(),
            CompiledNode {
                path: vec!["ci".to_string(), "pre-push".to_string()],
                has_subcommands: false,
                takes_positional: false,
                long_flags: ["complete".to_string()].into_iter().collect(),
            },
        );
        compiled.insert(
            "ci retirement-audit".to_string(),
            CompiledNode {
                path: vec!["ci".to_string(), "retirement-audit".to_string()],
                has_subcommands: false,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "skill".to_string(),
            CompiledNode {
                path: vec!["skill".to_string()],
                has_subcommands: true,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "skill list".to_string(),
            CompiledNode {
                path: vec!["skill".to_string(), "list".to_string()],
                has_subcommands: false,
                takes_positional: false,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "skill search".to_string(),
            CompiledNode {
                path: vec!["skill".to_string(), "search".to_string()],
                has_subcommands: false,
                takes_positional: true,
                long_flags: HashSet::new(),
            },
        );
        compiled.insert(
            "run".to_string(),
            CompiledNode {
                path: vec!["run".to_string()],
                has_subcommands: false,
                takes_positional: true,
                long_flags: ["interp".to_string(), "mode".to_string()]
                    .into_iter()
                    .collect(),
            },
        );

        let mut registry_only = HashSet::new();
        registry_only.insert("dei status".to_string());

        let mut root_long_flags = HashSet::new();
        root_long_flags.insert("quiet".to_string());
        root_long_flags.insert("help".to_string());
        root_long_flags.insert("version".to_string());

        CliSurface {
            compiled,
            registry_only,
            registry_retired: HashSet::new(),
            root_long_flags,
        }
    }

    #[test]
    fn check_command_known_subcommand() {
        let s = sample_surface();
        assert_eq!(check_command(&s, "vox audit retirement"), CliVerdict::Ok);
    }

    #[test]
    fn check_command_unknown_subcommand() {
        let s = sample_surface();
        assert_eq!(
            check_command(&s, "vox audit nosuch"),
            CliVerdict::UnknownSubcommand {
                parent: "audit".to_string(),
                token: "nosuch".to_string(),
            }
        );
    }

    #[test]
    fn check_command_unknown_root() {
        let s = sample_surface();
        assert_eq!(
            check_command(&s, "vox nosuch"),
            CliVerdict::UnknownCommand("nosuch".to_string())
        );
    }

    #[test]
    fn check_command_positional_and_placeholder() {
        let s = sample_surface();
        assert_eq!(check_command(&s, "vox run scripts/foo.vox"), CliVerdict::Ok);
        assert_eq!(
            check_command(&s, "vox skill search <query>"),
            CliVerdict::Ok
        );
    }

    #[test]
    fn check_command_registry_only() {
        let s = sample_surface();
        assert_eq!(check_command(&s, "vox dei status"), CliVerdict::Ok);
    }

    #[test]
    fn check_command_cargo_run_syntax() {
        let s = sample_surface();
        assert_eq!(
            check_command(&s, "cargo run -p vox-cli -- audit effort"),
            CliVerdict::Ok
        );
    }

    #[test]
    fn cargo_run_other_crate_skipped() {
        assert!(!is_vox_cli_invocation(
            "cargo run -p vox-doc-pipeline -- --lint-only"
        ));
        assert!(is_vox_cli_invocation(
            "cargo run -p vox-cli -- audit effort"
        ));
        assert!(is_vox_cli_invocation("vox audit effort"));
    }

    #[test]
    fn check_command_flags() {
        let s = sample_surface();
        assert_eq!(
            check_command(&s, "vox ci pre-push --complete"),
            CliVerdict::Ok
        );
        assert_eq!(
            check_command(&s, "vox ci pre-push --nosuch"),
            CliVerdict::UnknownFlag {
                path: "ci pre-push".to_string(),
                flag: "--nosuch".to_string(),
            }
        );
        // Ancestor flag (e.g. audit defines --json)
        assert_eq!(
            check_command(&s, "vox audit retirement --json"),
            CliVerdict::Ok
        );
        // Root flag (e.g. --quiet)
        assert_eq!(
            check_command(&s, "vox audit retirement --quiet"),
            CliVerdict::Ok
        );
        // --help and --version
        assert_eq!(
            check_command(&s, "vox audit retirement --help"),
            CliVerdict::Ok
        );
        assert_eq!(
            check_command(&s, "vox audit retirement --version"),
            CliVerdict::Ok
        );
        // Short flags ignored
        assert_eq!(check_command(&s, "vox audit retirement -q"), CliVerdict::Ok);
        // Flag on registry-only not checked
        assert_eq!(
            check_command(&s, "vox dei status --anything"),
            CliVerdict::Ok
        );
    }

    #[test]
    fn stale_planned_claims_fixtures() {
        let s = sample_surface();
        let f205 = "- `vox audit retirement` — `vox ci retirement-audit` (planned per CR-L6)";
        let m205 = extract_mentions(Path::new("AGENTS.md"), f205);
        let res205 = stale_planned_claims(f205, &m205, &s);
        assert_eq!(
            res205.len(),
            1,
            "expected 1 planned finding for line 205 fixture, got {res205:?}"
        );

        let f217 = "Token-waste audits like `vox audit effort` (planned per [`docs/specs/foo.md`](bar)) self-improve";
        let m217 = extract_mentions(Path::new("AGENTS.md"), f217);
        let res217 = stale_planned_claims(f217, &m217, &s);
        assert_eq!(
            res217.len(),
            1,
            "expected 1 planned finding for line 217 fixture, got {res217:?}"
        );

        let f230 = "the `vox_skill_use` tier-2 tool (planned per Track B of the plan below).";
        let m230 = extract_mentions(Path::new("AGENTS.md"), f230);
        let res230 = stale_planned_claims(f230, &m230, &s);
        assert_eq!(
            res230.len(),
            0,
            "expected 0 planned findings for line 230 fixture, got {res230:?}"
        );

        let f234 = "(roots + bare-dir discovery are planned per Track A; today ... `vox skill discover` walks the workspace)";
        let m234 = extract_mentions(Path::new("AGENTS.md"), f234);
        let res234 = stale_planned_claims(f234, &m234, &s);
        assert_eq!(
            res234.len(),
            0,
            "expected 0 planned findings for line 234 fixture, got {res234:?}"
        );
    }

    #[test]
    fn retired_surfaces_table_role_mentions_skipped() {
        let _s = sample_surface();
        let text = "| `vox wasm run` | retired in v0.5 |";
        let mentions = extract_mentions(Path::new("AGENTS.md"), text);
        assert!(!mentions.is_empty());
        assert_eq!(mentions[0].role, MentionRole::Mentions);
    }

    #[test]
    fn run_errors_on_missing_mandatory_file() {
        let tmp = tempfile::tempdir().unwrap();
        let res = scope_files(tmp.path(), Scope::AgentFiles);
        assert!(res.is_err());
        let err = res.err().unwrap().to_string();
        assert!(err.contains("missing mandatory agent file") || err.contains("AGENTS.md"));
    }

    #[test]
    fn real_tree() {
        crate::command_catalog::run_on_big_stack(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let surface = cli_surface(&root).expect("build cli surface");
            assert!(surface.compiled.contains_key("audit retirement"));
            assert!(surface.compiled.contains_key("audit effort"));
            assert!(surface.compiled.contains_key("ci retirement-audit"));
            assert!(surface.registry_only.iter().any(|k| k.starts_with("dei")));

            // AgentFiles scope has zero findings
            let files = scope_files(&root, Scope::AgentFiles).expect("scope files");
            let mut findings = Vec::new();
            for file in files {
                let content = vox_bounded_fs::read_utf8_path_capped(&file).unwrap();
                let mentions = extract_mentions(&file, &content);
                findings.extend(stale_planned_claims(&content, &mentions, &surface));
                for mention in mentions {
                    if mention.kind != MentionKind::CliCommand {
                        continue;
                    }
                    if !is_vox_cli_invocation(&mention.raw_text) {
                        continue;
                    }
                    if mention.role != MentionRole::Prescribes {
                        continue;
                    }
                    if is_under_retired_heading(&content, mention.byte_offset) {
                        continue;
                    }
                    let v = check_command(&surface, &mention.raw_text);
                    if v != CliVerdict::Ok {
                        findings.push(format!(
                            "{}:{}: {:?} in `{}`",
                            mention.source_file, mention.line_number, v, mention.raw_text
                        ));
                    }
                }
            }
            assert!(findings.is_empty(), "agent-file findings: {findings:#?}");
        });
    }

    #[test]
    fn run_errors_on_reader_facing_unknown_command() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // Mandatory agent files
        for f in [
            "AGENTS.md",
            "CLAUDE.md",
            "GEMINI.md",
            ".github/copilot-instructions.md",
            "docs/src/AGENTS.md",
        ] {
            let p = root.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "# Title\n").unwrap();
        }
        let rules_dir = root.join(".cursor/rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(rules_dir.join("test.mdc"), "# Rule\n").unwrap();

        // contracts/cli/command-registry.yaml
        let reg_dir = root.join("contracts/cli");
        std::fs::create_dir_all(&reg_dir).unwrap();
        std::fs::write(
            reg_dir.join("command-registry.yaml"),
            "schema_version: 1\noperations: []\n",
        )
        .unwrap();

        // Reader-facing page with unknown command
        let tut = root.join("docs/src/tutorials/hello.md");
        std::fs::create_dir_all(tut.parent().unwrap()).unwrap();
        std::fs::write(
            tut,
            "---\ntitle: \"Hello\"\nstatus: current\n---\nRun `vox nosuch-command`.\n",
        )
        .unwrap();

        let res = run(root);
        assert!(
            res.is_err(),
            "expected run() to fail on reader-facing unknown command"
        );
        let err = res.err().unwrap().to_string();
        assert!(
            err.contains("unknown command `vox nosuch-command`") || err.contains("finding(s)"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn real_tree_reader_facing_clean() {
        crate::command_catalog::run_on_big_stack(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let surface = cli_surface(&root).expect("build cli surface");
            let files = scope_files(&root, Scope::ReaderFacing).expect("scope files");
            let mut findings = Vec::new();

            for file in files {
                let content = match vox_bounded_fs::read_utf8_path_capped(&file) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let mentions = extract_mentions(&file, &content);
                let planned = stale_planned_claims(&content, &mentions, &surface);
                findings.extend(planned);

                for mention in mentions {
                    if mention.kind != MentionKind::CliCommand {
                        continue;
                    }
                    if !is_vox_cli_invocation(&mention.raw_text) {
                        continue;
                    }
                    if mention.role != MentionRole::Prescribes {
                        continue;
                    }
                    if is_under_retired_heading(&content, mention.byte_offset) {
                        continue;
                    }
                    let v = check_command(&surface, &mention.raw_text);
                    if v != CliVerdict::Ok {
                        findings.push(format!(
                            "{}:{}: {:?} in `{}`",
                            mention.source_file, mention.line_number, v, mention.raw_text
                        ));
                    }
                }
            }

            assert!(findings.is_empty(), "reader-facing findings: {findings:#?}");
        });
    }
}
