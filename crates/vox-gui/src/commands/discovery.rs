//! Tauri commands powering the Console discovery engine: candidate suggestion,
//! per-command help lookup, and exposure recording. Ranking/ledger logic lives in
//! `vox_gamify::discovery`; these commands adapt the command catalog to it.

use std::sync::Arc;

use serde::Serialize;
use serde_json::json;
use tauri::State;
use vox_cli::command_catalog::{CommandCatalogEntry, build_catalog};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

/// A single suggestion returned to the UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Suggestion {
    /// Canonical action id, e.g. "vox.scientia.review" or "slash.model".
    pub action_id: String,
    /// Full command line fragment to complete to, e.g. "scientia review".
    pub completion: String,
    pub about: String,
    /// Kind of candidate ("command", "group", "flag", "slash", "nu", "symbol").
    #[serde(default)]
    pub kind: String,
    /// UI category grouping ("command", "slash", "nu", "symbol").
    #[serde(default)]
    pub category: String,
    /// Compact flag/argument signature preview.
    #[serde(default)]
    pub signature: String,
    /// Fuzzy match score (higher is better).
    #[serde(default)]
    pub score: u32,
}

/// Build the canonical action id for a catalog entry: "vox" + dotted path.
pub fn action_id_for(entry: &CommandCatalogEntry) -> String {
    let mut parts = vec!["vox".to_string()];
    parts.extend(entry.path.iter().cloned());
    parts.join(".")
}

fn format_entry_signature(entry: &CommandCatalogEntry) -> String {
    let mut parts = Vec::new();
    for arg in entry.arguments.iter().take(4) {
        if let Some(long) = &arg.long {
            if arg.takes_value {
                parts.push(format!("--{long} <{}>", arg.name));
            } else {
                parts.push(format!("--{long}"));
            }
        } else {
            parts.push(format!("<{}>", arg.name));
        }
    }
    parts.join(" ")
}

const NU_PIPELINE_BUILTINS: &[(&str, &str, &str)] = &[
    (
        "ls",
        "ls | where size > 1kb",
        "List directory entries as a structured table",
    ),
    (
        "ps",
        "ps | where cpu > 5",
        "List running host processes as a structured table",
    ),
    (
        "open",
        "open Cargo.toml",
        "Parse structured file (TOML/JSON/YAML/CSV) into a value",
    ),
    (
        "where",
        "where status == 'ok'",
        "Filter rows of a structured stream by predicate",
    ),
    (
        "select",
        "select name status",
        "Project specific columns from a structured table",
    ),
    (
        "get",
        "get package.name",
        "Extract a cell or nested path from a record/table",
    ),
    (
        "to json",
        "to json --raw",
        "Serialize structured pipeline output as JSON",
    ),
    (
        "from json",
        "from json",
        "Parse JSON text into a structured Nushell value",
    ),
    (
        "sort-by",
        "sort-by size --reverse",
        "Sort structured table rows by column",
    ),
    (
        "each",
        "each {|it| $it }",
        "Map a closure over each element in a stream",
    ),
    (
        "flatten",
        "flatten",
        "Flatten nested lists or tables into top-level rows",
    ),
    (
        "polars",
        "polars into-df",
        "Convert structured stream into a Polars dataframe",
    ),
    (
        "http get",
        "http get https://example.com",
        "Fetch HTTP JSON/text response",
    ),
];

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut h_chars = haystack.chars();
    for n in needle.chars() {
        if !h_chars.any(|c| c == n) {
            return false;
        }
    }
    true
}

fn subsequence_match(needle: &str, haystack: &str) -> bool {
    let needle_lower = needle.to_ascii_lowercase();
    let haystack_lower = haystack.to_ascii_lowercase();
    let tokens: Vec<&str> = needle_lower.split_whitespace().collect();
    if tokens.is_empty() {
        return false;
    }
    tokens.iter().all(|t| is_subsequence(t, &haystack_lower))
}

/// Filter and rank the catalog (plus `/slash`, `!nu`, and `--flag` candidates) using
/// `vox_cli::fuzzy::FuzzyMatcher` (`nucleo`).
pub fn match_catalog(
    entries: &[CommandCatalogEntry],
    typed: &str,
    limit: usize,
) -> Vec<Suggestion> {
    if limit == 0 {
        return Vec::new();
    }
    let trimmed = typed.trim();

    // 1. Slash command completions (`/model`, `/nu`, `/help`, ...)
    if let Some(slash_query) = trimmed.strip_prefix('/') {
        let reg = vox_terminal_core::commands::CommandRegistry::default();
        let all_slash = reg.entries();
        let mut matcher = vox_cli::fuzzy::FuzzyMatcher::new();
        let needle = slash_query.trim().to_ascii_lowercase();
        let mut scored: Vec<(u32, Suggestion)> = Vec::new();
        for cmd in all_slash {
            let key = format!("{} {}", cmd.name, cmd.description);
            let mut score = if needle.is_empty() {
                100
            } else {
                matcher
                    .rank(&needle, &[&key])
                    .first()
                    .map(|(_, s)| *s)
                    .unwrap_or(0)
            };
            if score == 0 && !needle.is_empty() && subsequence_match(&needle, &key) {
                score = 200;
            }
            if !needle.is_empty() && cmd.name.starts_with(&needle) {
                score = score.saturating_add(500);
            }
            if needle.is_empty() || score > 0 {
                scored.push((
                    score,
                    Suggestion {
                        action_id: format!("slash.{}", cmd.name),
                        completion: format!("/{}", cmd.name),
                        about: cmd.description.to_string(),
                        kind: "slash".to_string(),
                        category: "slash".to_string(),
                        signature: format!("/{}", cmd.name),
                        score,
                    },
                ));
            }
        }
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.completion.cmp(&b.1.completion))
        });
        return scored.into_iter().take(limit).map(|(_, s)| s).collect();
    }

    // 2. Nushell structured shell completions (`!nu ...` or `! ...`)
    if trimmed.starts_with("!nu") || trimmed.starts_with('!') {
        let after_bang = trimmed
            .strip_prefix("!nu")
            .or_else(|| trimmed.strip_prefix('!'))
            .unwrap_or("")
            .trim();
        let stage_query = after_bang
            .rsplit('|')
            .next()
            .unwrap_or(after_bang)
            .trim()
            .to_ascii_lowercase();
        let prefix_before_stage = if let Some(idx) = after_bang.rfind('|') {
            format!("!nu {} | ", after_bang[..idx].trim())
        } else {
            "!nu ".to_string()
        };
        let mut matcher = vox_cli::fuzzy::FuzzyMatcher::new();
        let mut scored: Vec<(u32, Suggestion)> = Vec::new();
        for &(name, sig, about) in NU_PIPELINE_BUILTINS {
            let key = format!("{name} {sig} {about}");
            let mut score = if stage_query.is_empty() {
                100
            } else {
                matcher
                    .rank(&stage_query, &[&key])
                    .first()
                    .map(|(_, s)| *s)
                    .unwrap_or(0)
            };
            if score == 0 && !stage_query.is_empty() && subsequence_match(&stage_query, &key) {
                score = 200;
            }
            if !stage_query.is_empty() && name.starts_with(&stage_query) {
                score = score.saturating_add(500);
            }
            if stage_query.is_empty() || score > 0 {
                scored.push((
                    score,
                    Suggestion {
                        action_id: format!("nu.{}", name.replace(' ', "_")),
                        completion: format!("{prefix_before_stage}{name}"),
                        about: about.to_string(),
                        kind: "nu".to_string(),
                        category: "nu".to_string(),
                        signature: sig.to_string(),
                        score,
                    },
                ));
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        return scored.into_iter().take(limit).map(|(_, s)| s).collect();
    }

    // 3. Flag completion when the last token starts with `-` (e.g. `scientia review --`)
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if let Some(&last_tok) = tokens.last()
        && last_tok.starts_with('-')
        && tokens.len() >= 2
    {
        let cmd_tokens = &tokens[..tokens.len() - 1];
        let cmd_prefix = cmd_tokens.join(" ");
        if let Some(entry) = entries
            .iter()
            .find(|e| e.path.join(" ") == cmd_prefix || e.aliases.iter().any(|a| a == &cmd_prefix))
        {
            let flag_needle = last_tok.trim_start_matches('-').to_ascii_lowercase();
            let mut out = Vec::new();
            for arg in &entry.arguments {
                let Some(long) = &arg.long else { continue };
                if flag_needle.is_empty() || long.to_ascii_lowercase().contains(&flag_needle) {
                    let flag_str = format!("--{long}");
                    let sig = if arg.takes_value {
                        format!("{flag_str} <{}>", arg.name)
                    } else {
                        flag_str.clone()
                    };
                    out.push(Suggestion {
                        action_id: format!("{}.--{}", action_id_for(entry), long),
                        completion: format!("{cmd_prefix} {flag_str}"),
                        about: arg.help.clone().unwrap_or_else(|| entry.about.clone()),
                        kind: "flag".to_string(),
                        category: "command".to_string(),
                        signature: sig,
                        score: if long.starts_with(&flag_needle) {
                            900
                        } else {
                            500
                        },
                    });
                    if out.len() >= limit {
                        break;
                    }
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }

    // 4. Empty input: return top runnable leaves
    if trimmed.is_empty() {
        return entries
            .iter()
            .filter(|e| !e.has_subcommands)
            .take(limit)
            .map(|e| Suggestion {
                action_id: action_id_for(e),
                completion: e.path.join(" "),
                about: e.about.clone(),
                kind: "command".to_string(),
                category: "command".to_string(),
                signature: format_entry_signature(e),
                score: 100,
            })
            .collect();
    }

    // 5. Fuzzy + prefix matching across command paths, aliases, and about text
    let normalized_needle = tokens.join(" ").to_ascii_lowercase();
    let mut matcher = vox_cli::fuzzy::FuzzyMatcher::new();
    let search_keys: Vec<String> = entries
        .iter()
        .map(|e| {
            let path_str = e.path.join(" ");
            if e.aliases.is_empty() {
                format!("{path_str} — {}", e.about)
            } else {
                format!("{path_str} ({}) — {}", e.aliases.join(" "), e.about)
            }
        })
        .collect();
    let ranked = matcher.rank(&normalized_needle, &search_keys);
    let mut score_by_idx = std::collections::HashMap::with_capacity(ranked.len());
    for (idx, s) in ranked {
        score_by_idx.insert(idx, s);
    }

    let mut candidates: Vec<(u32, Suggestion)> = Vec::new();
    for (idx, e) in entries.iter().enumerate() {
        let path_str = e.path.join(" ");
        let path_lower = path_str.to_ascii_lowercase();
        let exact_prefix = path_lower.starts_with(&normalized_needle)
            || e.aliases
                .iter()
                .any(|a| a.to_ascii_lowercase().starts_with(&normalized_needle));
        let path_contains = path_lower.contains(&normalized_needle)
            || e.aliases
                .iter()
                .any(|a| a.to_ascii_lowercase().contains(&normalized_needle));
        let path_subseq = subsequence_match(&normalized_needle, &path_lower)
            || e.aliases
                .iter()
                .any(|a| subsequence_match(&normalized_needle, &a.to_ascii_lowercase()));
        let fuzzy_score = score_by_idx.get(&idx).copied().unwrap_or(0);
        let desc_subseq = if fuzzy_score == 0 && !exact_prefix && !path_contains && !path_subseq {
            let key = &search_keys[idx];
            subsequence_match(&normalized_needle, key)
        } else {
            false
        };
        if !exact_prefix && !path_contains && !path_subseq && fuzzy_score == 0 && !desc_subseq {
            continue;
        }

        let mut total_score = fuzzy_score;
        if exact_prefix {
            total_score = total_score.saturating_add(1_000);
        } else if path_contains {
            total_score = total_score.saturating_add(600);
        } else if path_subseq {
            total_score = total_score.saturating_add(400);
        } else if desc_subseq {
            total_score = total_score.saturating_add(20);
        }
        if !e.has_subcommands {
            total_score = total_score.saturating_add(150);
        }
        candidates.push((
            total_score,
            Suggestion {
                action_id: action_id_for(e),
                completion: path_str,
                about: e.about.clone(),
                kind: if e.has_subcommands {
                    "group".to_string()
                } else {
                    "command".to_string()
                },
                category: "command".to_string(),
                signature: format_entry_signature(e),
                score: total_score,
            },
        ));
    }

    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.completion.len().cmp(&b.1.completion.len()))
            .then_with(|| a.1.completion.cmp(&b.1.completion))
    });
    candidates.into_iter().take(limit).map(|(_, s)| s).collect()
}

#[tauri::command]
pub fn discovery_suggest(typed: String, limit: Option<usize>) -> Result<Vec<Suggestion>, String> {
    let catalog = build_catalog();
    let trimmed = typed.trim();
    let cleaned = if trimmed.starts_with('/') || trimmed.starts_with('!') {
        trimmed.to_string()
    } else {
        trimmed
            .strip_prefix("vox ")
            .or_else(|| trimmed.strip_prefix("vox"))
            .unwrap_or(trimmed)
            .trim()
            .to_string()
    };
    Ok(match_catalog(
        &catalog.entries,
        &cleaned,
        limit.unwrap_or(8),
    ))
}

/// Rich help for one action id, for the discovery rail.
#[derive(Debug, Clone, Serialize)]
pub struct ActionHelp {
    pub action_id: String,
    pub about: String,
    pub args: Vec<ArgHelp>,
    pub example: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArgHelp {
    pub name: String,
    pub help: String,
    pub required: bool,
}

#[tauri::command]
pub fn discovery_help(action_id: String) -> Result<Option<ActionHelp>, String> {
    if let Some(slash_name) = action_id.strip_prefix("slash.") {
        let reg = vox_terminal_core::commands::CommandRegistry::default();
        if let Some(cmd) = reg.entries().into_iter().find(|c| c.name == slash_name) {
            return Ok(Some(ActionHelp {
                action_id,
                about: cmd.description.to_string(),
                args: vec![],
                example: format!("/{}", cmd.name),
            }));
        }
    }
    if let Some(nu_name) = action_id.strip_prefix("nu.") {
        let lookup = nu_name.replace('_', " ");
        if let Some(&(name, sig, about)) =
            NU_PIPELINE_BUILTINS.iter().find(|(n, _, _)| *n == lookup)
        {
            return Ok(Some(ActionHelp {
                action_id,
                about: about.to_string(),
                args: vec![],
                example: format!("!nu {sig} ({name})"),
            }));
        }
    }
    let catalog = build_catalog();
    let base_action_id = action_id.split(".--").next().unwrap_or(&action_id);
    let entry = catalog
        .entries
        .iter()
        .find(|e| action_id_for(e) == base_action_id);
    Ok(entry.map(|e| {
        let args = e
            .arguments
            .iter()
            .map(|a| ArgHelp {
                name: a.long.clone().unwrap_or_else(|| a.name.clone()),
                help: a.help.clone().unwrap_or_default(),
                required: a.required,
            })
            .collect();
        ActionHelp {
            action_id: action_id.clone(),
            about: e.about.clone(),
            args,
            example: format!("vox {}", e.path.join(" ")),
        }
    }))
}

const RECORD_TOOL: &str = "vox_gamify_discovery_record";

pub(crate) async fn discovery_record_impl<B: BackendCalls>(
    b: &B,
    action_id: String,
    used: bool,
    now_ms: i64,
    dwell_ms: i64,
) -> Result<(), String> {
    let args =
        json!({ "action_id": action_id, "used": used, "now_ms": now_ms, "dwell_ms": dwell_ms });
    tool_data(RECORD_TOOL, b.call_tool(RECORD_TOOL, args).await?)
}

/// Record an exposure (seen/used) for the current user through the daemon's dev-only
/// `vox_gamify_discovery_record` op. `used=true` ⇒ Recall::Used.
#[tauri::command]
pub async fn discovery_record(
    backend: State<'_, Arc<BackendState>>,
    action_id: String,
    used: bool,
    now_ms: i64,
    dwell_ms: i64,
) -> Result<(), String> {
    discovery_record_impl(&backend.current().await, action_id, used, now_ms, dwell_ms).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_lists_runnable_leaves() {
        let catalog = build_catalog();
        let s = match_catalog(&catalog.entries, "", 5);
        assert!(!s.is_empty());
        assert!(s.len() <= 5);
    }

    #[test]
    fn zero_limit_returns_empty() {
        let catalog = build_catalog();
        assert!(match_catalog(&catalog.entries, "", 0).is_empty());
    }

    #[test]
    fn prefix_filters_to_matching_paths() {
        let catalog = build_catalog();
        // "config" is a stable top-level group across the CLI.
        let s = match_catalog(&catalog.entries, "config", 5);
        assert!(
            s.iter()
                .any(|x| x.completion.starts_with("config") || x.action_id.contains("config")),
            "got: {:?}",
            s.iter().map(|x| &x.completion).collect::<Vec<_>>()
        );
        assert!(
            s[0].completion.starts_with("config"),
            "top match should be exact prefix: {:?}",
            s[0]
        );
    }

    #[test]
    fn fuzzy_match_ranks_typos_and_subsequences() {
        let catalog = build_catalog();
        let s = match_catalog(&catalog.entries, "doctr", 5);
        assert!(
            s.iter().any(|x| x.completion.contains("doctor")),
            "expected fuzzy match for 'doctr' -> 'doctor', got: {:?}",
            s.iter().map(|x| &x.completion).collect::<Vec<_>>()
        );

        let rev = match_catalog(&catalog.entries, "scntia rev", 5);
        assert!(
            rev.iter()
                .any(|x| x.completion.contains("scientia") && x.completion.contains("rev")),
            "expected fuzzy match for 'scntia rev' to match a scientia review command, got: {:?}",
            rev.iter().map(|x| &x.completion).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flag_completion_suggests_command_flags() {
        let catalog = build_catalog();
        let s = match_catalog(&catalog.entries, "commands --", 10);
        assert!(
            s.iter()
                .any(|x| x.kind == "flag" && x.completion.contains("--format")),
            "expected `--format` flag completion for `commands --`, got: {:?}",
            s
        );
    }

    #[test]
    fn slash_and_nu_completions_work() {
        let catalog = build_catalog();
        let slash = match_catalog(&catalog.entries, "/nu", 5);
        assert!(
            slash
                .iter()
                .any(|x| x.completion == "/nu" && x.kind == "slash"),
            "expected `/nu` slash completion, got: {:?}",
            slash
        );

        let nu = match_catalog(&catalog.entries, "!nu ls | wh", 5);
        assert!(
            nu.iter()
                .any(|x| x.completion.contains("where") && x.kind == "nu"),
            "expected `!nu ls | where` completion, got: {:?}",
            nu
        );
    }

    #[tokio::test]
    async fn record_calls_the_discovery_op_with_the_tauri_arguments() {
        let b = crate::remote::calls::fake::FakeBackend::new()
            .on_tool(RECORD_TOOL, Ok(json!({ "success": true, "data": null })));
        discovery_record_impl(&b, "vox.config.show".into(), true, 10, 5)
            .await
            .unwrap();
        assert_eq!(
            b.calls()[0],
            (
                format!("tool:{RECORD_TOOL}"),
                json!({ "action_id": "vox.config.show", "used": true, "now_ms": 10, "dwell_ms": 5 })
            )
        );
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("discovery.rs");
        assert!(!src.contains(concat!("vox", "_db")));
    }
}
