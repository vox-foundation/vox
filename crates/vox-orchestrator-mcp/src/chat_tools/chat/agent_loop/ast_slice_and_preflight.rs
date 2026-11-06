//! Milestone `v3.4` (`Phases 178–180`): Prefix-Locked Two-Tier Tool Selection (`CACHE-PREFIX-01..04`),
//! AST Symbol-Slice Read & Skeleton Folding (`AST-SLICE-01..04`), and Pre-Edit Cross-Crate /
//! Repo Invariant Grounding (`PREFLIGHT-GROUND-01..04`).

use serde::{Deserialize, Serialize};
use vox_mcp_registry::McpToolRegistryEntry;
use vox_skills::SkillRegistry;

use crate::llm_bridge::tool_selection::{TurnContext, select_tools_for_turn_ranked};

/// Canonical 12-tool core prefix (`CACHE-PREFIX-01`) kept in strictly invariant order at the front
/// of every turn's `tools` array so provider KV-caches (Anthropic, OpenAI, Gemini, DeepSeek, vLLM/MENS)
/// never invalidate the tool-schema prefix when BM25 query ranking changes across turns.
pub const FROZEN_PREFIX_CORE_TOOLS: &[&str] = &[
    "vox_apply_structured_edit",
    "vox_repo_query_file",
    "vox_repo_query_text",
    "vox_search_query",
    "vox_validate_file",
    "vox_check_workspace",
    "vox_run_tests",
    "vox_git_status",
    "vox_repo_status",
    "vox_kb_search",
    "vox_skill_search",
    "vox_spawn_agent",
];

/// Result of prefix-locked two-tier tool selection (`CACHE-PREFIX-01..03`).
#[derive(Debug, Clone)]
pub struct PrefixLockedToolSelection {
    /// Active tools for the turn: `0..frozen_prefix_count` are strictly ordered from
    /// [`FROZEN_PREFIX_CORE_TOOLS`], followed by spoke-ranked dynamic tools.
    pub selected: Vec<&'static McpToolRegistryEntry>,
    /// Number of leading entries in `selected` that belong to the immutable prefix.
    pub frozen_prefix_count: usize,
    /// Classified organic spoke for the turn.
    pub classified_spoke: vox_corpus::training::contract::OrganicSpokeKind,
    /// Compact 1-line-per-tool deferred catalog hint for relevant overflow tools (`CACHE-PREFIX-03`).
    pub deferred_catalog_hint: Option<String>,
}

/// Select tools with a frozen, byte-identical core prefix followed by spoke-ranked dynamic tools,
/// plus a compact trailing catalog hint for overflow tools (`CACHE-PREFIX-01..03`).
#[must_use]
pub fn select_prefix_locked_tools_for_turn(
    registry: &'static [McpToolRegistryEntry],
    skill_registry: &SkillRegistry,
    ctx: &TurnContext,
    user_prompt: &str,
    prior_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
) -> PrefixLockedToolSelection {
    let mut wide_ctx = ctx.clone();
    wide_ctx.max_tools = ctx.max_tools.saturating_add(16);
    let (ranked_pool, classified_spoke, _mode) = select_tools_for_turn_ranked(
        registry,
        skill_registry,
        &wide_ctx,
        user_prompt,
        prior_spoke,
    );

    let mut frozen_prefix: Vec<&'static McpToolRegistryEntry> = Vec::new();
    for &core_name in FROZEN_PREFIX_CORE_TOOLS {
        if frozen_prefix.len() >= ctx.max_tools {
            break;
        }
        if let Some(&entry) = ranked_pool.iter().find(|e| e.name == core_name) {
            frozen_prefix.push(entry);
        } else if let Some(entry) = registry.iter().find(|e| {
            e.name == core_name
                && (ctx.permission_mode.as_deref() != Some("plan") || e.http_read_role_eligible)
                && !ctx
                    .exclude_name_prefixes
                    .iter()
                    .any(|p| e.name.starts_with(p))
        }) {
            frozen_prefix.push(entry);
        }
    }

    let frozen_prefix_count = frozen_prefix.len();
    let mut selected = frozen_prefix;
    let mut overflow = Vec::new();

    for entry in ranked_pool {
        if selected.iter().any(|e| e.name == entry.name) {
            continue;
        }
        if selected.len() < ctx.max_tools {
            selected.push(entry);
        } else if overflow.len() < 8 {
            overflow.push(entry);
        }
    }

    let deferred_catalog_hint = if overflow.is_empty() {
        None
    } else {
        let items: Vec<String> = overflow
            .into_iter()
            .map(|e| {
                let short = e
                    .description
                    .split(". ")
                    .next()
                    .unwrap_or(e.description)
                    .trim_end_matches('.');
                format!("`{}` ({})", e.name, short)
            })
            .collect();
        Some(format!(
            "[DEFERRED MCP TOOLS AVAILABLE ON DEMAND: {}]",
            items.join(", ")
        ))
    };

    PrefixLockedToolSelection {
        selected,
        frozen_prefix_count,
        classified_spoke,
        deferred_catalog_hint,
    }
}

/// Extracted AST symbol slice with top-of-file imports and exact line span (`AST-SLICE-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSlice {
    pub symbol_name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub imports_header: Vec<String>,
    pub body: String,
}

/// Extract a target `fn`, `struct`, `enum`, `trait`, `impl`, `interface`, `type`, or `const` symbol
/// from Rust, TypeScript/TSX, or `.vox` source without reading the entire file (`AST-SLICE-01..02`).
#[must_use]
pub fn extract_symbol_slice(source: &str, symbol_name: &str) -> Option<SymbolSlice> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() || symbol_name.trim().is_empty() {
        return None;
    }
    let sym = symbol_name.trim();
    let mut imports_header = Vec::new();
    for line in lines.iter().take(80) {
        let t = line.trim();
        if t.starts_with("use ") || t.starts_with("import ") {
            imports_header.push(t.to_string());
        }
    }

    let decl_keywords = [
        "fn",
        "struct",
        "enum",
        "trait",
        "type",
        "interface",
        "const",
        "function",
        "impl",
    ];
    let decl_patterns: Vec<String> = decl_keywords
        .iter()
        .map(|kw| format!("{kw} {sym}"))
        .collect();

    let mut decl_idx = None;
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with('*') {
            continue;
        }
        if decl_patterns.iter().any(|pat| {
            t.find(pat.as_str()).is_some_and(|pos| {
                let after = &t[pos + pat.len()..];
                after.is_empty()
                    || after.starts_with(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            })
        }) {
            decl_idx = Some(idx);
            break;
        }
    }

    let mut start_idx = decl_idx?;
    while start_idx > 0 {
        let prev = lines[start_idx - 1].trim();
        if prev.starts_with("#[") || prev.starts_with("///") || prev.starts_with('@') {
            start_idx -= 1;
        } else {
            break;
        }
    }

    let mut brace_depth = 0i32;
    let mut saw_open_brace = false;
    let mut end_idx = decl_idx?;
    for (idx, line) in lines.iter().enumerate().skip(decl_idx?) {
        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
                saw_open_brace = true;
            } else if ch == '}' {
                brace_depth -= 1;
            }
        }
        end_idx = idx;
        if saw_open_brace && brace_depth <= 0 {
            break;
        }
        if !saw_open_brace && line.trim_end().ends_with(';') {
            break;
        }
    }

    let mut rendered = String::new();
    for (offset, line) in lines[start_idx..=end_idx].iter().enumerate() {
        let line_no = start_idx + offset + 1;
        rendered.push_str(&format!("{line_no:>4} | {line}\n"));
    }

    Some(SymbolSlice {
        symbol_name: sym.to_string(),
        start_line: start_idx + 1,
        end_line: end_idx + 1,
        imports_header,
        body: rendered.trim_end().to_string(),
    })
}

/// Fold a large source file or code block into a signature-preserving AST skeleton so truncated
/// tool outputs keep `pub fn`/`pub struct`/`export interface` declarations intact (`AST-SLICE-03`).
#[must_use]
pub fn fold_code_to_ast_skeleton(source: &str, max_chars: usize) -> String {
    if source.len() <= max_chars {
        return source.to_string();
    }
    let mut out = String::with_capacity(max_chars.min(source.len()));
    let mut brace_depth = 0i32;
    let mut elided_lines_in_block = 0usize;

    for line in source.lines() {
        let trimmed = line.trim();
        let opens = line.chars().filter(|&c| c == '{').count() as i32;
        let closes = line.chars().filter(|&c| c == '}').count() as i32;

        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        let is_signature_or_import =
            brace_depth == 0 || first_word == "fn" || trimmed.contains(" fn ");

        if is_signature_or_import
            || (brace_depth >= 1 && closes > 0 && brace_depth + opens - closes == 0)
        {
            if elided_lines_in_block > 0 {
                out.push_str(&format!(
                    "    /* ... {elided_lines_in_block} body line(s) elided ... */\n"
                ));
                elided_lines_in_block = 0;
            }
            out.push_str(line);
            out.push('\n');
        } else {
            elided_lines_in_block += 1;
        }

        brace_depth = (brace_depth + opens - closes).max(0);
        if out.len() >= max_chars {
            if elided_lines_in_block > 0 {
                out.push_str(&format!(
                    "    /* ... {elided_lines_in_block} body line(s) elided ... */\n"
                ));
            }
            out.push_str("/* ... [AST SKELETON BUDGET REACHED] ... */\n");
            break;
        }
    }
    out
}

/// Result of running deterministic pre-edit cross-crate and repo policy guards (`PREFLIGHT-GROUND-01..03`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightWriteReport {
    pub unlinked_crates: Vec<String>,
    pub non_blank_lines: usize,
    pub god_object_exceeded: bool,
    pub god_object_warning: bool,
    pub versioned_model_literals: Vec<String>,
    pub banned_imports: Vec<String>,
    pub formatted_advisory: Option<String>,
}

/// Evaluate pre-edit cross-crate `Cargo.toml` dependencies, `arch/god_object` LoC budget,
/// versioned model literals, and banned crypto/LLM imports in `<2ms` (`PREFLIGHT-GROUND-01..03`).
#[must_use]
pub fn preflight_check_write_guards(
    path: &str,
    content: &str,
    cargo_toml_content: Option<&str>,
) -> PreflightWriteReport {
    let norm_path = path.replace('\\', "/");
    let non_blank_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
    let god_object_exceeded = non_blank_lines > 500;
    let god_object_warning = (450..=500).contains(&non_blank_lines);

    let mut unlinked_crates = Vec::new();
    let mut banned_imports = Vec::new();
    let mut versioned_model_literals = Vec::new();

    if norm_path.ends_with(".rs") {
        for line in content.lines() {
            let t = line.trim();
            let rest = t
                .strip_prefix("use ")
                .or_else(|| t.strip_prefix("pub use "))
                .or_else(|| t.strip_prefix("pub(crate) use "));
            if let Some(after_use) = rest {
                let root = after_use
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("::")
                    .trim();
                if root.starts_with("vox_")
                    && let Some(toml_str) = cargo_toml_content
                {
                    let hyphenated = root.replace('_', "-");
                    if !toml_str.contains(&hyphenated)
                        && !toml_str.contains(root)
                        && !unlinked_crates.contains(&root.to_string())
                    {
                        unlinked_crates.push(root.to_string());
                    }
                }
                if !norm_path.contains("crates/vox-crypto/")
                    && matches!(
                        root,
                        "chacha20poly1305"
                            | "ed25519_dalek"
                            | "x25519_dalek"
                            | "sha2"
                            | "sha3"
                            | "md5"
                    )
                    && !banned_imports.contains(&root.to_string())
                {
                    banned_imports.push(root.to_string());
                }
            }
        }
    }

    if (norm_path.ends_with(".ts") || norm_path.ends_with(".tsx"))
        && !norm_path.ends_with("noVersionedModelIds.test.ts")
    {
        for forbidden in [
            "claude-sonnet-4.6",
            "claude-opus-4.6",
            "claude-3-5-sonnet",
            "gpt-4o-2024",
            "gemini-2.5-pro",
        ] {
            if content.contains(forbidden) {
                versioned_model_literals.push(forbidden.to_string());
            }
        }
    }

    let mut notes = Vec::new();
    if !unlinked_crates.is_empty() {
        notes.push(format!(
            "unlinked crate import(s) {:?} not found in target Cargo.toml (would fail with E0432)",
            unlinked_crates
        ));
    }
    if god_object_exceeded {
        notes.push(format!(
            "non-blank LoC={} exceeds 500-line arch/god_object limit; extract a submodule before finishing",
            non_blank_lines
        ));
    } else if god_object_warning {
        notes.push(format!(
            "non-blank LoC={} is approaching the 500-line arch/god_object ceiling",
            non_blank_lines
        ));
    }
    if !versioned_model_literals.is_empty() {
        notes.push(format!(
            "hardcoded versioned model ID(s) {:?} violate noVersionedModelIds policy",
            versioned_model_literals
        ));
    }
    if !banned_imports.is_empty() {
        notes.push(format!(
            "banned direct crypto/provider import(s) {:?} (use vox_crypto / vox_actor_runtime::llm)",
            banned_imports
        ));
    }
    let formatted_advisory = if notes.is_empty() {
        None
    } else {
        Some(format!(
            "[PREFLIGHT WRITE GUARD (`{norm_path}`): {}]",
            notes.join("; ")
        ))
    };

    PreflightWriteReport {
        unlinked_crates,
        non_blank_lines,
        god_object_exceeded,
        god_object_warning,
        versioned_model_literals,
        banned_imports,
        formatted_advisory,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_mcp_registry::TOOL_REGISTRY;
    use vox_skills::new_registry_arc;

    #[test]
    fn v34_prefix_locked_tools_ast_slice_skeleton_and_preflight_guards_work() {
        // 1. Prefix-locked tool selection keeps core prefix invariant across divergent prompts
        let skill_reg = new_registry_arc();
        let ctx = TurnContext::default();
        let turn_a = select_prefix_locked_tools_for_turn(
            TOOL_REGISTRY,
            &skill_reg,
            &ctx,
            "fix rust borrow checker error in vox-orchestrator",
            None,
        );
        let turn_b = select_prefix_locked_tools_for_turn(
            TOOL_REGISTRY,
            &skill_reg,
            &ctx,
            "search documentation kb and validate mermaid diagram",
            None,
        );
        assert!(turn_a.frozen_prefix_count >= 10);
        assert_eq!(turn_a.frozen_prefix_count, turn_b.frozen_prefix_count);
        let prefix_names_a: Vec<&str> = turn_a.selected[..turn_a.frozen_prefix_count]
            .iter()
            .map(|e| e.name)
            .collect();
        let prefix_names_b: Vec<&str> = turn_b.selected[..turn_b.frozen_prefix_count]
            .iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(prefix_names_a, prefix_names_b);

        // 2. AST symbol slice and skeleton folding
        let sample = [
            "use std::collections::HashMap;",
            "use serde::Serialize;",
            "",
            "pub struct WidgetConfig {",
            "    pub id: String,",
            "    pub count: usize,",
            "}",
            "",
            "/// Computes widget score.",
            "#[must_use]",
            "pub fn compute_widget_score(cfg: &WidgetConfig) -> usize {",
            "    let base = cfg.count * 10;",
            "    let bonus = if cfg.id.is_empty() { 0 } else { 5 };",
            "    base + bonus",
            "}",
        ]
        .join("\n");
        let slice = extract_symbol_slice(&sample, "compute_widget_score")
            .expect("must locate compute_widget_score");
        assert_eq!(slice.symbol_name, "compute_widget_score");
        assert_eq!(slice.imports_header.len(), 2);
        assert!(slice.body.contains("pub fn compute_widget_score"));
        assert!(!slice.body.contains("pub struct WidgetConfig"));

        let folded = fold_code_to_ast_skeleton(&sample, 180);
        assert!(folded.contains("pub struct WidgetConfig"));
        assert!(folded.contains("body line(s) elided"));

        // 3. Preflight write guards
        let cargo_toml = r#"
[package]
name = "vox-gui"
[dependencies]
vox-orchestrator = { workspace = true }
"#;
        let rs_edit = r#"
use vox_orchestrator::models::ModelRegistry;
use vox_telemetry::spool::upload;
use sha2::Sha256;
"#;
        let report = preflight_check_write_guards(
            "crates/vox-gui/src/commands/chat_arena_branch.rs",
            rs_edit,
            Some(cargo_toml),
        );
        assert_eq!(report.unlinked_crates, vec!["vox_telemetry".to_string()]);
        assert_eq!(report.banned_imports, vec!["sha2".to_string()]);
        assert!(report.formatted_advisory.is_some());

        let ts_report = preflight_check_write_guards(
            "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.test.tsx",
            "const m = 'anthropic/claude-sonnet-4.6';",
            None,
        );
        assert_eq!(
            ts_report.versioned_model_literals,
            vec!["claude-sonnet-4.6".to_string()]
        );
    }
}
