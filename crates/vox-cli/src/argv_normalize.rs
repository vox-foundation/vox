//! Deterministic `<1.5 ms` CLI argv normalization, prefix/flag tolerance helper,
//! and dual-persona (`InteractiveHuman` vs `NonInteractiveOrAgent`) execution gate
//! (`CLI-TOL-01..04`).
//!
//! NEVER issues network calls or synchronous LLM inference on the CLI hot path.
//! All Stage 3 synonym & foreign-idiom rules come from the build-time lockfile
//! `contracts/cli/semantic-aliases.v1.json`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::VoxCliRoot;
use crate::command_catalog::{CommandCatalogEntry, build_catalog};
use crate::commands::ci::cli_synonyms::SemanticAliasesLockfile;

pub const MAX_RECOVERY_TOKENS: usize = 16;
pub const MAX_RECOVERY_CHARS: usize = 256;

const EMBEDDED_SEMANTIC_ALIASES: &str =
    include_str!("../../../contracts/cli/semantic-aliases.v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryKind {
    TokenPermutation,
    UniqueLeafElision,
    WrongSubcommandFlag,
    SynonymRewrite,
    FuzzyShortlist,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCandidate {
    pub canonical_path: Vec<String>,
    pub rewritten_argv: Vec<String>,
    pub capability_id: Option<String>,
    pub side_effect_class: String,
    pub kind: RecoveryKind,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Resolved(RecoveryCandidate),
    Shortlist(Vec<RecoveryCandidate>),
    Unrecoverable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerPersona {
    InteractiveHuman,
    NonInteractiveOrAgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliErrorRecovery {
    pub error_code: String,
    pub attempted: Vec<String>,
    pub canonical_suggestion: Option<Vec<String>>,
    pub shortlist: Vec<Vec<String>>,
    pub side_effect_class: String,
    pub auto_executed: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionDecision {
    AutoExecute(RecoveryCandidate),
    PromptConfirm(RecoveryCandidate),
    RejectWithDiagnostic(CliErrorRecovery),
}

struct RecoveryIndex {
    entries: Vec<CommandCatalogEntry>,
    top_levels: BTreeSet<String>,
    by_exact_path: HashMap<Vec<String>, usize>,
    by_sorted_tokens: HashMap<Vec<String>, Vec<usize>>,
    by_leaf: HashMap<String, Vec<usize>>,
    side_effects: HashMap<Vec<String>, String>,
    lockfile: SemanticAliasesLockfile,
}

static RECOVERY_INDEX: OnceLock<RecoveryIndex> = OnceLock::new();

fn build_recovery_index() -> RecoveryIndex {
    let catalog = build_catalog();
    let cap_index = vox_capability_registry::bundled_index();

    let mut side_effects = HashMap::new();
    for row in &cap_index.rows {
        if let Some(cli) = &row.cli {
            let path_lc: Vec<String> = cli.path.iter().map(|s| s.to_ascii_lowercase()).collect();
            if let Some(cls) = &row.side_effect_class {
                side_effects.insert(path_lc, cls.clone());
            }
        }
    }

    let mut top_levels = BTreeSet::new();
    let mut by_exact_path = HashMap::new();
    let mut by_sorted_tokens: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    let mut by_leaf: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, entry) in catalog.entries.iter().enumerate() {
        if entry.path.is_empty() {
            continue;
        }
        let path_lc: Vec<String> = entry.path.iter().map(|s| s.to_ascii_lowercase()).collect();
        if path_lc.len() == 1 {
            top_levels.insert(path_lc[0].clone());
            for alias in &entry.aliases {
                top_levels.insert(alias.to_ascii_lowercase());
            }
        }
        by_exact_path.insert(path_lc.clone(), idx);

        if path_lc.len() >= 2 && path_lc.len() <= 3 {
            let mut key = path_lc.clone();
            key.sort();
            by_sorted_tokens.entry(key).or_default().push(idx);
        }
        if path_lc.len() >= 2 {
            if let Some(last) = path_lc.last() {
                by_leaf.entry(last.clone()).or_default().push(idx);
            }
        }
    }

    let lockfile: SemanticAliasesLockfile = serde_json::from_str(EMBEDDED_SEMANTIC_ALIASES)
        .unwrap_or_else(|_| SemanticAliasesLockfile {
            schema_version: 1,
            generated_from: "contracts/operations/catalog.v1.yaml".to_string(),
            min_cosine_similarity: 0.72,
            min_cosine_margin: 0.08,
            antonym_banlist: vec![],
            verb_aliases: BTreeMap::new(),
            noun_aliases: BTreeMap::new(),
            foreign_idioms: vec![],
            rules: vec![],
        });

    RecoveryIndex {
        entries: catalog.entries,
        top_levels,
        by_exact_path,
        by_sorted_tokens,
        by_leaf,
        side_effects,
        lockfile,
    }
}

fn index() -> &'static RecoveryIndex {
    RECOVERY_INDEX.get_or_init(build_recovery_index)
}

/// Lookup `side_effect_class` conservatively: defaults to `"local_mutation"` when unmapped
/// so unannotated mutating commands never auto-execute.
pub fn lookup_side_effect_class(path: &[String]) -> String {
    let idx = index();
    let path_lc: Vec<String> = path.iter().map(|s| s.to_ascii_lowercase()).collect();
    if let Some(cls) = idx.side_effects.get(&path_lc) {
        return cls.clone();
    }
    // Check capability index row id `cli.<path>` directly as fallback.
    let cap_id = format!("cli.{}", path_lc.join("."));
    for row in &vox_capability_registry::bundled_index().rows {
        if row.id.eq_ignore_ascii_case(&cap_id) {
            if let Some(cls) = &row.side_effect_class {
                return cls.clone();
            }
        }
    }
    // Conservative default: non-read_only triggers [Y/n] confirmation for humans
    // and structured redirect for agents.
    "local_mutation".to_string()
}

fn is_file_like(tok: &str) -> bool {
    if tok.contains('/') || tok.contains('\\') {
        return true;
    }
    let lower = tok.to_ascii_lowercase();
    [".vox", ".rs", ".json", ".toml", ".yaml", ".yml", ".md"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

fn is_global_flag_without_value(tok: &str) -> bool {
    matches!(
        tok,
        "--json" | "--quiet" | "-q" | "--verbose" | "-v" | "-vv" | "-vvv"
    )
}

/// Partition `raw_args[1..]` into leading command tokens (before flags or file-like tokens)
/// and the trailing tokens preserved verbatim during argv rewriting.
fn split_command_tokens(raw_args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut cmd_tokens = Vec::new();
    let mut tail = Vec::new();
    let mut i = 1;
    let mut in_tail = false;

    while i < raw_args.len() {
        let tok = &raw_args[i];
        if !in_tail {
            if tok.starts_with('-') || is_file_like(tok) || cmd_tokens.len() >= 3 {
                in_tail = true;
                tail.push(tok.clone());
            } else {
                cmd_tokens.push(tok.clone());
            }
        } else {
            tail.push(tok.clone());
        }
        i += 1;
    }
    (cmd_tokens, tail)
}

fn make_candidate(
    idx: &RecoveryIndex,
    entry: &CommandCatalogEntry,
    tail: &[String],
    bin_name: &str,
    kind: RecoveryKind,
    note: Option<String>,
) -> RecoveryCandidate {
    let mut rewritten = Vec::with_capacity(1 + entry.path.len() + tail.len());
    rewritten.push(bin_name.to_string());
    rewritten.extend(entry.path.iter().cloned());
    rewritten.extend(tail.iter().cloned());
    RecoveryCandidate {
        canonical_path: entry.path.clone(),
        rewritten_argv: rewritten,
        capability_id: entry.capability_id.clone(),
        side_effect_class: lookup_side_effect_class(&entry.path),
        kind,
        note: note.or_else(|| {
            let _ = idx;
            None
        }),
    }
}

fn build_rewritten_argv(
    bin_name: &str,
    canonical_path: &[String],
    extra_args: &[String],
    tail: &[String],
) -> Vec<String> {
    let mut out = Vec::with_capacity(1 + canonical_path.len() + extra_args.len() + tail.len());
    out.push(bin_name.to_string());
    out.extend(canonical_path.iter().cloned());
    out.extend(extra_args.iter().cloned());
    out.extend(tail.iter().cloned());
    out
}

pub fn classify_recovery(raw_args: &[String]) -> RecoveryOutcome {
    if raw_args.len() <= 1 || raw_args.len() > MAX_RECOVERY_TOKENS {
        return RecoveryOutcome::Unrecoverable;
    }
    let total_chars: usize = raw_args.iter().map(|s| s.len()).sum();
    if total_chars > MAX_RECOVERY_CHARS {
        return RecoveryOutcome::Unrecoverable;
    }
    if raw_args
        .iter()
        .skip(1)
        .any(|t| matches!(t.as_str(), "-h" | "--help" | "-V" | "--version"))
    {
        return RecoveryOutcome::Unrecoverable;
    }

    let idx = index();
    let bin_name = raw_args[0].as_str();
    let (cmd_tokens, tail) = split_command_tokens(raw_args);
    if cmd_tokens.is_empty() {
        return RecoveryOutcome::Unrecoverable;
    }
    let cmd_lc: Vec<String> = cmd_tokens.iter().map(|s| s.to_ascii_lowercase()).collect();

    // 2a. Order-agnostic token permutation (prefixes of length 3 then 2).
    for prefix_len in [3_usize, 2] {
        if cmd_lc.len() >= prefix_len {
            let mut key = cmd_lc[..prefix_len].to_vec();
            key.sort();
            if let Some(matches) = idx.by_sorted_tokens.get(&key) {
                let mut rest_tail = cmd_tokens[prefix_len..].to_vec();
                rest_tail.extend(tail.iter().cloned());

                let mut permuted: Vec<&CommandCatalogEntry> = matches
                    .iter()
                    .map(|&i| &idx.entries[i])
                    .filter(|e| {
                        let e_lc: Vec<String> =
                            e.path.iter().map(|s| s.to_ascii_lowercase()).collect();
                        e_lc != cmd_lc[..prefix_len]
                    })
                    .collect();
                permuted.sort_by(|a, b| a.path.cmp(&b.path));
                permuted.dedup_by(|a, b| a.path == b.path);

                if permuted.len() == 1 {
                    return RecoveryOutcome::Resolved(make_candidate(
                        idx,
                        permuted[0],
                        &rest_tail,
                        bin_name,
                        RecoveryKind::TokenPermutation,
                        Some("reordered command tokens to canonical noun-verb path".to_string()),
                    ));
                } else if permuted.len() > 1 {
                    let list = permuted
                        .into_iter()
                        .map(|e| {
                            make_candidate(
                                idx,
                                e,
                                &rest_tail,
                                bin_name,
                                RecoveryKind::TokenPermutation,
                                None,
                            )
                        })
                        .collect();
                    return RecoveryOutcome::Shortlist(list);
                }
            }
        }
    }

    // Stage 3a: Exact precomputed foreign idioms (never collide with active canonical paths).
    let mut sorted_idioms: Vec<&crate::commands::ci::cli_synonyms::ForeignIdiomRule> =
        idx.lockfile.foreign_idioms.iter().collect();
    sorted_idioms.sort_by(|a, b| b.trigger.len().cmp(&a.trigger.len()));
    for idiom in sorted_idioms {
        if cmd_lc.len() >= idiom.trigger.len() && cmd_lc[..idiom.trigger.len()] == idiom.trigger[..]
        {
            let canon_lc: Vec<String> = idiom
                .canonical_path
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect();
            if let Some(&entry_idx) = idx.by_exact_path.get(&canon_lc) {
                let entry = &idx.entries[entry_idx];
                let rest_tail = [&cmd_tokens[idiom.trigger.len()..], &tail[..]].concat();
                let rewritten =
                    build_rewritten_argv(bin_name, &entry.path, &idiom.insert_args, &rest_tail);
                return RecoveryOutcome::Resolved(RecoveryCandidate {
                    canonical_path: entry.path.clone(),
                    rewritten_argv: rewritten,
                    capability_id: entry.capability_id.clone(),
                    side_effect_class: lookup_side_effect_class(&entry.path),
                    kind: RecoveryKind::SynonymRewrite,
                    note: Some(format!(
                        "mapped idiom `{}` to `{}`",
                        idiom.trigger.join(" "),
                        entry.command
                    )),
                });
            }
        }
    }

    // 2b. Unique leaf subcommand elision (e.g. `vox command-compliance` -> `vox ci command-compliance`).
    let t0 = &cmd_lc[0];
    if !idx.top_levels.contains(t0) {
        if let Some(matches) = idx.by_leaf.get(t0) {
            let mut candidates: Vec<&CommandCatalogEntry> =
                matches.iter().map(|&i| &idx.entries[i]).collect();
            candidates.sort_by(|a, b| a.path.cmp(&b.path));
            candidates.dedup_by(|a, b| a.path == b.path);
            let rest_tail = [&cmd_tokens[1..], &tail[..]].concat();

            if candidates.len() == 1 {
                return RecoveryOutcome::Resolved(make_candidate(
                    idx,
                    candidates[0],
                    &rest_tail,
                    bin_name,
                    RecoveryKind::UniqueLeafElision,
                    Some(format!(
                        "expanded omitted parent group for unique subcommand `{}`",
                        t0
                    )),
                ));
            } else if candidates.len() > 1 {
                let list = candidates
                    .into_iter()
                    .map(|e| {
                        make_candidate(
                            idx,
                            e,
                            &rest_tail,
                            bin_name,
                            RecoveryKind::UniqueLeafElision,
                            None,
                        )
                    })
                    .collect();
                return RecoveryOutcome::Shortlist(list);
            }
        }
    }

    // 2c. Wrong-subcommand flag redirection (e.g. `vox build --release app.vox` -> `vox compile --release app.vox`).
    for len in (1..=cmd_lc.len().min(2)).rev() {
        if let Some(&entry_idx) = idx.by_exact_path.get(&cmd_lc[..len]) {
            let current_entry = &idx.entries[entry_idx];
            let current_longs: BTreeSet<String> = current_entry
                .arguments
                .iter()
                .filter_map(|a| a.long.as_ref().map(|l| format!("--{}", l)))
                .collect();

            let unknown_long_flags: Vec<String> = tail
                .iter()
                .filter_map(|t| {
                    if !t.starts_with("--") {
                        return None;
                    }
                    let flag_name = t.split('=').next().unwrap_or(t.as_str());
                    if matches!(flag_name, "--color" | "--json" | "--verbose" | "--quiet") {
                        return None;
                    }
                    if current_longs.contains(flag_name) {
                        None
                    } else {
                        Some(flag_name.trim_start_matches("--").to_string())
                    }
                })
                .collect();

            if !unknown_long_flags.is_empty() {
                let mut sibling_hits: Vec<&CommandCatalogEntry> = idx
                    .entries
                    .iter()
                    .filter(|e| {
                        e.path != current_entry.path
                            && e.source_group == current_entry.source_group
                            && unknown_long_flags.iter().all(|req_flag| {
                                e.arguments
                                    .iter()
                                    .any(|a| a.long.as_deref() == Some(req_flag.as_str()))
                            })
                    })
                    .collect();
                sibling_hits.sort_by(|a, b| a.path.cmp(&b.path));
                sibling_hits.dedup_by(|a, b| a.path == b.path);

                let rest_tail = [&cmd_tokens[len..], &tail[..]].concat();
                if sibling_hits.len() == 1 {
                    return RecoveryOutcome::Resolved(make_candidate(
                        idx,
                        sibling_hits[0],
                        &rest_tail,
                        bin_name,
                        RecoveryKind::WrongSubcommandFlag,
                        Some(format!(
                            "`{}` does not accept --{}; sibling `{}` does",
                            current_entry.command, unknown_long_flags[0], sibling_hits[0].command
                        )),
                    ));
                } else if !sibling_hits.is_empty() {
                    // Canonical tie-breaker for `vox build --release` / `--target`: prefer `compile`.
                    if current_entry.path == ["build"]
                        && unknown_long_flags
                            .iter()
                            .any(|f| f == "release" || f == "target")
                    {
                        if let Some(compile_entry) =
                            sibling_hits.iter().find(|e| e.path == ["compile"])
                        {
                            return RecoveryOutcome::Resolved(make_candidate(
                                idx,
                                compile_entry,
                                &rest_tail,
                                bin_name,
                                RecoveryKind::WrongSubcommandFlag,
                                Some(format!(
                                    "`vox build` does not accept --{}; use `vox compile`",
                                    unknown_long_flags[0]
                                )),
                            ));
                        }
                    }
                    let list = sibling_hits
                        .into_iter()
                        .map(|e| {
                            make_candidate(
                                idx,
                                e,
                                &rest_tail,
                                bin_name,
                                RecoveryKind::WrongSubcommandFlag,
                                None,
                            )
                        })
                        .collect();
                    return RecoveryOutcome::Shortlist(list);
                }
            }
        }
    }

    // Stage 3: Precomputed foreign idioms + compositional slot synonym rewrite (`CLI-TOL-04`).
    for idiom in &idx.lockfile.foreign_idioms {
        if cmd_lc.len() >= idiom.trigger.len() && cmd_lc[..idiom.trigger.len()] == idiom.trigger[..]
        {
            let canon_lc: Vec<String> = idiom
                .canonical_path
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect();
            if let Some(&entry_idx) = idx.by_exact_path.get(&canon_lc) {
                let entry = &idx.entries[entry_idx];
                let rest_tail = [&cmd_tokens[idiom.trigger.len()..], &tail[..]].concat();
                let rewritten =
                    build_rewritten_argv(bin_name, &entry.path, &idiom.insert_args, &rest_tail);
                return RecoveryOutcome::Resolved(RecoveryCandidate {
                    canonical_path: entry.path.clone(),
                    rewritten_argv: rewritten,
                    capability_id: entry.capability_id.clone(),
                    side_effect_class: lookup_side_effect_class(&entry.path),
                    kind: RecoveryKind::SynonymRewrite,
                    note: Some(format!(
                        "mapped idiom `{}` to `{}`",
                        idiom.trigger.join(" "),
                        entry.command
                    )),
                });
            }
        }
    }

    // Compositional slot synonym translation (verbs + nouns) followed by direct or permuted path lookup.
    for prefix_len in (1..=cmd_lc.len().min(3)).rev() {
        let mut translated = Vec::with_capacity(prefix_len);
        let mut changed = false;
        for tok in &cmd_lc[..prefix_len] {
            if let Some(canon_noun) = idx.lockfile.noun_aliases.get(tok) {
                translated.push(canon_noun.clone());
                changed = true;
            } else if let Some(canon_verb) = idx.lockfile.verb_aliases.get(tok) {
                translated.push(canon_verb.clone());
                changed = true;
            } else {
                translated.push(tok.clone());
            }
        }
        if !changed {
            continue;
        }
        let rest_tail = [&cmd_tokens[prefix_len..], &tail[..]].concat();
        if let Some(&entry_idx) = idx.by_exact_path.get(&translated) {
            return RecoveryOutcome::Resolved(make_candidate(
                idx,
                &idx.entries[entry_idx],
                &rest_tail,
                bin_name,
                RecoveryKind::SynonymRewrite,
                Some("translated CLI synonym to canonical command".to_string()),
            ));
        }
        if prefix_len >= 2 {
            let mut key = translated.clone();
            key.sort();
            if let Some(matches) = idx.by_sorted_tokens.get(&key) {
                if matches.len() == 1 {
                    return RecoveryOutcome::Resolved(make_candidate(
                        idx,
                        &idx.entries[matches[0]],
                        &rest_tail,
                        bin_name,
                        RecoveryKind::SynonymRewrite,
                        Some(
                            "translated CLI synonym + token order to canonical command".to_string(),
                        ),
                    ));
                }
            }
        }
    }

    // 2d. Capability index fuzzy shortlist fallback.
    let query = cmd_tokens.join(" ");
    let mut query_chars = query;
    if query_chars.chars().count() > vox_capability_registry::MAX_QUERY_CHARS {
        query_chars = query_chars
            .chars()
            .take(vox_capability_registry::MAX_QUERY_CHARS)
            .collect();
    }
    if let Ok(res) =
        vox_capability_registry::resolve(vox_capability_registry::bundled_index(), &query_chars)
    {
        let mut shortlist = Vec::new();
        match res {
            vox_capability_registry::Resolution::Exact { primary, also } => {
                for row in std::iter::once(primary).chain(also.into_iter()) {
                    if let Some(cli) = &row.cli {
                        let path_lc: Vec<String> =
                            cli.path.iter().map(|s| s.to_ascii_lowercase()).collect();
                        if let Some(&eidx) = idx.by_exact_path.get(&path_lc) {
                            shortlist.push(make_candidate(
                                idx,
                                &idx.entries[eidx],
                                &tail,
                                bin_name,
                                RecoveryKind::FuzzyShortlist,
                                None,
                            ));
                        }
                    }
                }
            }
            vox_capability_registry::Resolution::Shortlist(rows) => {
                for cand in rows.into_iter().take(5) {
                    if let Some(cli) = &cand.row.cli {
                        let path_lc: Vec<String> =
                            cli.path.iter().map(|s| s.to_ascii_lowercase()).collect();
                        if let Some(&eidx) = idx.by_exact_path.get(&path_lc) {
                            shortlist.push(make_candidate(
                                idx,
                                &idx.entries[eidx],
                                &tail,
                                bin_name,
                                RecoveryKind::FuzzyShortlist,
                                None,
                            ));
                        }
                    }
                }
            }
            vox_capability_registry::Resolution::NoMatch => {}
        }
        if !shortlist.is_empty() {
            return RecoveryOutcome::Shortlist(shortlist);
        }
    }

    RecoveryOutcome::Unrecoverable
}

impl CallerPersona {
    pub fn detect_with(
        raw_args: &[String],
        stdin_tty: bool,
        stdout_tty: bool,
        env_non_interactive: bool,
    ) -> Self {
        let has_json_flag = raw_args
            .iter()
            .any(|a| a == "--json" || a.starts_with("--json="));
        if has_json_flag || env_non_interactive || !stdin_tty || !stdout_tty {
            Self::NonInteractiveOrAgent
        } else {
            Self::InteractiveHuman
        }
    }

    pub fn detect(raw_args: &[String]) -> Self {
        let env_non_interactive = ["CI", "VOX_MCP", "VOX_NONINTERACTIVE", "VOX_CLI_GLOBAL_JSON"]
            .iter()
            .any(|k| {
                std::env::var_os(k).is_some_and(|v| {
                    let s = v.to_string_lossy();
                    !s.is_empty() && s != "0" && !s.eq_ignore_ascii_case("false")
                })
            });
        Self::detect_with(
            raw_args,
            std::io::stdin().is_terminal(),
            std::io::stdout().is_terminal(),
            env_non_interactive,
        )
    }
}

fn error_code_for_kind(kind: RecoveryKind) -> &'static str {
    match kind {
        RecoveryKind::TokenPermutation => "CLI_TOKEN_ORDER_SWAP",
        RecoveryKind::UniqueLeafElision => "CLI_UNIQUE_LEAF_ELISION",
        RecoveryKind::WrongSubcommandFlag => "CLI_FLAG_ON_WRONG_SUBCOMMAND",
        RecoveryKind::SynonymRewrite => "CLI_SYNONYM_MATCH",
        RecoveryKind::FuzzyShortlist => "CLI_DID_YOU_MEAN",
    }
}

pub fn decide_execution(
    outcome: RecoveryOutcome,
    raw_args: &[String],
    persona: CallerPersona,
) -> Option<ExecutionDecision> {
    match outcome {
        RecoveryOutcome::Unrecoverable => None,
        RecoveryOutcome::Resolved(cand) => {
            if persona == CallerPersona::InteractiveHuman
                && cand.side_effect_class == "read_only"
                && matches!(
                    cand.kind,
                    RecoveryKind::TokenPermutation
                        | RecoveryKind::UniqueLeafElision
                        | RecoveryKind::SynonymRewrite
                )
            {
                Some(ExecutionDecision::AutoExecute(cand))
            } else if persona == CallerPersona::InteractiveHuman {
                Some(ExecutionDecision::PromptConfirm(cand))
            } else {
                let code = error_code_for_kind(cand.kind).to_string();
                let canonical_cmd = cand.rewritten_argv.join(" ");
                let note = cand
                    .note
                    .as_deref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default();
                Some(ExecutionDecision::RejectWithDiagnostic(CliErrorRecovery {
                    error_code: code,
                    attempted: raw_args.to_vec(),
                    canonical_suggestion: Some(cand.rewritten_argv.clone()),
                    shortlist: vec![cand.rewritten_argv],
                    side_effect_class: cand.side_effect_class,
                    auto_executed: false,
                    message: format!("non-canonical invocation{note}; retry `{canonical_cmd}`"),
                }))
            }
        }
        RecoveryOutcome::Shortlist(cands) => {
            let first = cands.first()?;
            let code = error_code_for_kind(first.kind).to_string();
            let list: Vec<Vec<String>> = cands.into_iter().map(|c| c.rewritten_argv).collect();
            Some(ExecutionDecision::RejectWithDiagnostic(CliErrorRecovery {
                error_code: code,
                attempted: raw_args.to_vec(),
                canonical_suggestion: list.first().cloned(),
                shortlist: list,
                side_effect_class: "local_mutation".to_string(),
                auto_executed: false,
                message: "ambiguous or unknown command; pick a canonical invocation from shortlist"
                    .to_string(),
            }))
        }
    }
}

pub fn format_recovery_diagnostic(recovery: &CliErrorRecovery, json_mode: bool) -> String {
    if json_mode {
        let payload = serde_json::json!({
            "error": recovery.message,
            "aci": {
                "error_recovery": recovery,
            }
        });
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| recovery.message.clone())
    } else {
        let mut lines = vec![format!(
            "error[{}]: {}",
            recovery.error_code, recovery.message
        )];
        if let Some(canon) = &recovery.canonical_suggestion {
            lines.push(format!("help: canonical command:\n  {}", canon.join(" ")));
        }
        if recovery.shortlist.len() > 1 {
            lines.push("candidates:".to_string());
            for item in &recovery.shortlist {
                lines.push(format!("  - {}", item.join(" ")));
            }
        }
        lines.join("\n")
    }
}

/// Extract ML delegation subcommand and preserved flags (including any leading `GlobalOpts` flags
/// passed before `mens`/`oratio`/`speech`/`populi`/`mesh`/`train`/`quantize`).
pub fn build_ml_delegation_args(args: &[String]) -> Option<(String, Vec<String>)> {
    let sub_idx = args
        .iter()
        .enumerate()
        .skip(1)
        .find(|(i, a)| {
            if a.as_str() == "--color" {
                return false;
            }
            if args.get(i.wrapping_sub(1)).map(String::as_str) == Some("--color") {
                return false;
            }
            if is_global_flag_without_value(a.as_str()) {
                return false;
            }
            !a.starts_with('-')
        })
        .map(|(i, _)| i)?;

    let cmd = args[sub_idx].as_str();
    let is_ml = matches!(
        cmd,
        "mens" | "oratio" | "speech" | "populi" | "mesh" | "train" | "quantize"
    );
    let is_ext_ml = cmd == "ext"
        && args.len() > sub_idx + 1
        && matches!(
            args[sub_idx + 1].as_str(),
            "mens" | "oratio" | "speech" | "populi" | "mesh" | "train" | "quantize"
        );

    if !is_ml && !is_ext_ml {
        return None;
    }

    let sub_start = if is_ext_ml { sub_idx + 1 } else { sub_idx };
    let primary_cmd = args[sub_start].clone();
    let leading_globals = &args[1..sub_idx];

    let mut forward = Vec::new();
    if primary_cmd == "train" {
        forward.push("mens".to_string());
    }
    forward.extend(args[sub_start..].iter().cloned());
    forward.extend(leading_globals.iter().cloned());

    Some((primary_cmd, forward))
}

/// Parse `VoxCliRoot` from `std::env::args()`, keeping the happy path at ~0 ms overhead and
/// applying deterministic tolerance + dual-persona safety gating only on clap parse errors.
pub fn parse_cli_with_tolerance() -> VoxCliRoot {
    let raw_args: Vec<String> = std::env::args().collect();
    match VoxCliRoot::try_parse_from(&raw_args) {
        Ok(root) => root,
        Err(clap_err) => {
            if matches!(
                clap_err.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                clap_err.exit();
            }
            let outcome = classify_recovery(&raw_args);
            let persona = CallerPersona::detect(&raw_args);
            let json_mode = raw_args
                .iter()
                .any(|a| a == "--json" || a.starts_with("--json="))
                || std::env::var_os("VOX_CLI_GLOBAL_JSON").is_some_and(|v| v == "1");

            let Some(decision) = decide_execution(outcome, &raw_args, persona) else {
                clap_err.exit();
            };

            match decision {
                ExecutionDecision::AutoExecute(cand) => {
                    eprintln!("→ running canonical: {}", cand.rewritten_argv.join(" "));
                    match VoxCliRoot::try_parse_from(&cand.rewritten_argv) {
                        Ok(root) => root,
                        Err(err2) => err2.exit(),
                    }
                }
                ExecutionDecision::PromptConfirm(cand) => {
                    eprint!(
                        "→ suggested canonical ({}): `{}`\n  Run this command? [Y/n] ",
                        cand.side_effect_class,
                        cand.rewritten_argv.join(" ")
                    );
                    let _ = std::io::stderr().flush();
                    let mut line = String::new();
                    if std::io::stdin().read_line(&mut line).is_err() {
                        std::process::exit(2);
                    }
                    let answer = line.trim().to_ascii_lowercase();
                    if answer.is_empty() || answer == "y" || answer == "yes" {
                        match VoxCliRoot::try_parse_from(&cand.rewritten_argv) {
                            Ok(root) => root,
                            Err(err2) => err2.exit(),
                        }
                    } else {
                        std::process::exit(2);
                    }
                }
                ExecutionDecision::RejectWithDiagnostic(diag) => {
                    let formatted = format_recovery_diagnostic(&diag, json_mode);
                    if json_mode {
                        println!("{formatted}");
                    } else {
                        eprintln!("{formatted}");
                    }
                    std::process::exit(2);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cli;

    fn args(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn prefix_unfolding_resolves_unambiguous_and_rejects_ambiguous() {
        let r1 = VoxCliRoot::try_parse_from(["vox", "sec", "stat"]).expect("vox sec stat parses");
        assert!(matches!(r1.cmd, Cli::Secrets { .. }));

        let r2 =
            VoxCliRoot::try_parse_from(["vox", "comm", "--rec"]).expect("vox comm --rec parses");
        assert!(matches!(
            r2.cmd,
            Cli::Commands {
                recommended: true,
                ..
            }
        ));

        // Ambiguous prefixes (`c`, `comp`) must fail rather than pick silently.
        assert!(VoxCliRoot::try_parse_from(["vox", "c"]).is_err());
        assert!(VoxCliRoot::try_parse_from(["vox", "comp"]).is_err());
    }

    #[test]
    fn global_flags_hoist_from_any_position() {
        let r = VoxCliRoot::try_parse_from(["vox", "secrets", "status", "--json", "-q"])
            .expect("trailing global flags parse");
        assert!(r.global.json);
        assert!(r.global.quiet);

        let r2 = VoxCliRoot::try_parse_from(["vox", "--json", "secrets", "status", "-vv"])
            .expect("mixed global flags parse");
        assert!(r2.global.json);
        assert_eq!(r2.global.verbose, 2);

        let ml = build_ml_delegation_args(&args(&["vox", "--json", "-q", "mens", "status"]))
            .expect("ml detected");
        assert_eq!(ml.0, "mens");
        assert_eq!(ml.1, vec!["mens", "status", "--json", "-q"]);

        let ml_train = build_ml_delegation_args(&args(&[
            "vox", "--color", "always", "train", "--device", "cpu",
        ]))
        .expect("train detected");
        assert_eq!(ml_train.0, "train");
        assert_eq!(
            ml_train.1,
            vec!["mens", "train", "--device", "cpu", "--color", "always"]
        );
    }

    #[test]
    fn verb_noun_permutation_resolves_canonical_path() {
        let out = classify_recovery(&args(&["vox", "status", "secrets", "--json"]));
        match out {
            RecoveryOutcome::Resolved(cand) => {
                assert_eq!(cand.canonical_path, vec!["secrets", "status"]);
                assert_eq!(
                    cand.rewritten_argv,
                    vec!["vox", "secrets", "status", "--json"]
                );
                assert_eq!(cand.kind, RecoveryKind::TokenPermutation);
            }
            other => panic!("expected Resolved permutation, got {other:?}"),
        }
    }

    #[test]
    fn unique_leaf_elision_resolves_parent_group() {
        let out = classify_recovery(&args(&["vox", "command-compliance"]));
        match out {
            RecoveryOutcome::Resolved(cand) => {
                assert_eq!(cand.canonical_path, vec!["ci", "command-compliance"]);
                assert_eq!(cand.rewritten_argv, vec!["vox", "ci", "command-compliance"]);
                assert_eq!(cand.kind, RecoveryKind::UniqueLeafElision);
            }
            other => panic!("expected Resolved leaf elision, got {other:?}"),
        }
    }

    #[test]
    fn wrong_subcommand_flag_suggests_sibling_command() {
        let out = classify_recovery(&args(&["vox", "build", "--release", "app.vox"]));
        match out {
            RecoveryOutcome::Resolved(cand) => {
                assert_eq!(cand.canonical_path, vec!["compile"]);
                assert_eq!(
                    cand.rewritten_argv,
                    vec!["vox", "compile", "--release", "app.vox"]
                );
                assert_eq!(cand.kind, RecoveryKind::WrongSubcommandFlag);
            }
            other => panic!("expected Resolved sibling redirect, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_help_inputs_stay_unrecoverable() {
        assert_eq!(
            classify_recovery(&args(&["vox"])),
            RecoveryOutcome::Unrecoverable
        );
        assert_eq!(
            classify_recovery(&args(&["vox", "--help"])),
            RecoveryOutcome::Unrecoverable
        );
        assert_eq!(
            classify_recovery(&args(&["vox", "--version"])),
            RecoveryOutcome::Unrecoverable
        );
    }

    #[test]
    fn persona_detects_json_flag_and_agent_env() {
        assert_eq!(
            CallerPersona::detect_with(&args(&["vox", "secrets", "status"]), true, true, false),
            CallerPersona::InteractiveHuman
        );
        assert_eq!(
            CallerPersona::detect_with(
                &args(&["vox", "secrets", "status", "--json"]),
                true,
                true,
                false
            ),
            CallerPersona::NonInteractiveOrAgent
        );
        assert_eq!(
            CallerPersona::detect_with(&args(&["vox", "secrets", "status"]), false, true, false),
            CallerPersona::NonInteractiveOrAgent
        );
        assert_eq!(
            CallerPersona::detect_with(&args(&["vox", "secrets", "status"]), true, true, true),
            CallerPersona::NonInteractiveOrAgent
        );
    }

    #[test]
    fn read_only_permutation_auto_executes_for_interactive_human() {
        let out = classify_recovery(&args(&["vox", "status", "secrets"]));
        let dec = decide_execution(
            out,
            &args(&["vox", "status", "secrets"]),
            CallerPersona::InteractiveHuman,
        );
        assert!(matches!(dec, Some(ExecutionDecision::AutoExecute(_))));
    }

    #[test]
    fn mutating_permutation_prompts_for_interactive_human() {
        let out = classify_recovery(&args(&["vox", "remove", "catalog", "demo"]));
        // `remove` is top-level so ensure a clean mutating command permutation: `catalog` <-> `remove`
        assert!(matches!(out, RecoveryOutcome::Resolved(_)));
        let dec = decide_execution(
            out,
            &args(&["vox", "remove", "catalog", "demo"]),
            CallerPersona::InteractiveHuman,
        );
        assert!(matches!(dec, Some(ExecutionDecision::PromptConfirm(_))));
    }

    #[test]
    fn non_interactive_agent_never_prompts_or_auto_executes() {
        for sample in [
            &["vox", "status", "secrets"][..],
            &["vox", "remove", "catalog", "demo"][..],
        ] {
            let argv = args(sample);
            let out = classify_recovery(&argv);
            let dec = decide_execution(out, &argv, CallerPersona::NonInteractiveOrAgent);
            match dec {
                Some(ExecutionDecision::RejectWithDiagnostic(rec)) => {
                    assert!(!rec.auto_executed);
                    assert!(rec.canonical_suggestion.is_some());
                }
                other => panic!("expected RejectWithDiagnostic for {sample:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn json_diagnostic_contains_aci_error_recovery_fields() {
        let recovery = CliErrorRecovery {
            error_code: "CLI_TOKEN_ORDER_SWAP".to_string(),
            attempted: args(&["vox", "status", "secrets", "--json"]),
            canonical_suggestion: Some(args(&["vox", "secrets", "status", "--json"])),
            shortlist: vec![args(&["vox", "secrets", "status", "--json"])],
            side_effect_class: "read_only".to_string(),
            auto_executed: false,
            message: "reordered tokens".to_string(),
        };
        let rendered = format_recovery_diagnostic(&recovery, true);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
        let er = &parsed["aci"]["error_recovery"];
        assert_eq!(er["error_code"], "CLI_TOKEN_ORDER_SWAP");
        assert_eq!(er["auto_executed"], false);
        assert_eq!(er["side_effect_class"], "read_only");
        assert!(er["canonical_suggestion"].is_array());
    }

    #[test]
    fn semantic_aliases_lockfile_resolves_synonyms_and_idioms() {
        let out1 = classify_recovery(&args(&["vox", "clippy"]));
        match out1 {
            RecoveryOutcome::Resolved(cand) => {
                assert_eq!(cand.canonical_path, vec!["audit", "code"]);
                assert_eq!(cand.rewritten_argv, vec!["vox", "audit", "code"]);
                assert_eq!(cand.kind, RecoveryKind::SynonymRewrite);
            }
            other => panic!("expected SynonymRewrite for clippy, got {other:?}"),
        }

        let out2 = classify_recovery(&args(&["vox", "diagnose", "clavis"]));
        match out2 {
            RecoveryOutcome::Resolved(cand) => {
                assert_eq!(cand.canonical_path, vec!["secrets", "status"]);
                assert_eq!(cand.rewritten_argv, vec!["vox", "secrets", "status"]);
                assert_eq!(cand.kind, RecoveryKind::SynonymRewrite);
            }
            other => panic!("expected SynonymRewrite for diagnose clavis, got {other:?}"),
        }
    }

    #[test]
    fn recovery_warm_loop_meets_1500us_latency_budget() {
        // Warm once so OnceLock catalog + index init is excluded from warm lookup latency.
        let _ = classify_recovery(&args(&["vox", "status", "secrets"]));

        let inputs = [
            args(&["vox", "status", "secrets", "--json"]),
            args(&["vox", "command-compliance"]),
            args(&["vox", "build", "--release", "app.vox"]),
            args(&["vox", "clippy"]),
        ];
        let start = std::time::Instant::now();
        for i in 0..100 {
            let _ = classify_recovery(&inputs[i % inputs.len()]);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 150,
            "100 warm classify_recovery calls took {}ms (budget < 150ms = <1.5ms/call)",
            elapsed.as_millis()
        );
    }
}
