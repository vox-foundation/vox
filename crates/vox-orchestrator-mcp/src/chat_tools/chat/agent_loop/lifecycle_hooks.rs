//! Deterministic lifecycle hooks (`PreToolUse`, `PostToolBatch`, `Stop`) and
//! `Oracle` model escalation (`HOOKS-ORACLE-01..02`).
//!
//! Runs outside the main LLM context window at near-zero token cost to enforce
//! deterministic workspace guardrails (secret `.env` protection, generated-SSOT
//! write blocks, shell-chaining prevention, and `cargo fmt --all` auto-rewrite).

use serde::{Deserialize, Serialize};
use vox_orchestrator::models::{ModelRegistry, SelectionAxes, SelectionIntent, select};
use vox_orchestrator::types::TaskCategory;

/// Lifecycle hook event stage (`HOOKS-ORACLE-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEvent {
    PreToolUse,
    PostToolBatch,
    Stop,
}

/// Deterministic decision returned by a lifecycle hook (`HOOKS-ORACLE-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookDecision {
    Allow {
        updated_input: Option<serde_json::Value>,
        additional_context: Option<String>,
    },
    Block {
        reason: String,
    },
}

/// Structured advisory returned when a daily-driver model consults the Oracle tier (`HOOKS-ORACLE-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleConsultationAdvisory {
    pub oracle_model_id: String,
    pub provider: String,
    pub complexity: u8,
    pub selection_reason: String,
    pub advisory_summary: String,
}

/// Schema version tag for `PostToolDecisionPack` (`DEC-02`).
pub const POST_TOOL_DECISION_PACK_SCHEMA_V1: &str = "post_tool_decision_pack.v1";

/// Structured directive inside a [`PostToolDecisionPack`] (`DEC-02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostToolDirective {
    Continue,
    ReenterRepairLoop,
    ConsultOracle,
    StopBlock,
    /// `draft_pr` is rejected unless a PR tool already exists in the active ChatARC shortlist (`DEC-02`).
    DraftPr,
}

/// Calibrated post-tool decision pack (`DEC-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostToolDecisionPack {
    pub schema_version: String,
    /// Calibrated probability (`0.0..=1.0`) that the current model remains adequate after the tool batch.
    pub model_still_ok_prob: f64,
    /// Threshold below which `model_still_ok_prob` triggers `apply_repair_escalation_to_config`.
    pub confidence_threshold: f64,
    #[serde(default)]
    pub directive: Option<PostToolDirective>,
    #[serde(default)]
    pub block_reason: Option<String>,
}

impl PostToolDecisionPack {
    /// True when `model_still_ok_prob < confidence_threshold` or the directive explicitly requests repair escalation (`DEC-02`).
    #[must_use]
    pub fn should_reenter_repair_escalation(&self) -> bool {
        self.model_still_ok_prob < self.confidence_threshold
            || matches!(self.directive, Some(PostToolDirective::ReenterRepairLoop))
    }

    /// Validate whether `directive` is permitted given `shortlist_has_pr_tool` (`DEC-02`).
    #[must_use]
    pub fn effective_directive(&self, shortlist_has_pr_tool: bool) -> Option<PostToolDirective> {
        match self.directive {
            Some(PostToolDirective::DraftPr) if !shortlist_has_pr_tool => None,
            other => other,
        }
    }
}

const PROTECTED_GENERATED_FILES: &[&str] = &[
    "docs/src/SUMMARY.md",
    "docs/src/feed.xml",
    "docs/src/architecture/architecture-index.md",
    "docs/src/architecture/research-index.md",
    ".cursorignore",
    ".aiignore",
    ".aiexclude",
];

/// Evaluate deterministic `PreToolUse` guardrails before dispatching an MCP tool call (`HOOKS-ORACLE-01`).
#[must_use]
pub fn evaluate_pre_tool_use_hook(tool_name: &str, args: &serde_json::Value) -> HookDecision {
    let path_arg = args
        .get("path")
        .or_else(|| args.get("file_path"))
        .or_else(|| args.get("target_file"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let norm_path = path_arg.replace('\\', "/");
    // Guard the path the tool will actually open: step 1b strips a `#Symbol` suffix
    // before reading, so checking the raw arg would let `.env#KEY` through.
    let guarded_path = norm_path
        .split_once('#')
        .map_or(norm_path.as_str(), |(p, _)| p);
    let file_name = guarded_path
        .rsplit('/')
        .next()
        .unwrap_or(guarded_path)
        .to_ascii_lowercase();

    // 1. Secret .env guard (allow .env.example only; case-insensitive filesystems)
    if (file_name == ".env" || (file_name.starts_with(".env.") && file_name != ".env.example"))
        && !norm_path.is_empty()
    {
        return HookDecision::Block {
            reason: format!(
                "PreToolUse hook blocked access to secret file '{norm_path}'. Use vox_secrets::resolve_secret(...) or inspect .env.example instead."
            ),
        };
    }

    let is_write = crate::scope_guard::WRITE_TOOLS.contains(&tool_name)
        || matches!(
            tool_name,
            "vox_write_file"
                | "vox_edit_file"
                | "vox_patch_file"
                | "vox_apply_structured_edit"
                | "vox_generate_code"
        );

    // 1b. Read-tool `#SymbolName` AST symbol-slice extraction (`AST-SLICE-01..02` live path)
    if !is_write
        && let Some((clean_file, sym_name)) = norm_path.split_once('#')
        && !clean_file.is_empty()
        && !sym_name.is_empty()
    {
        let mut updated = args.clone();
        if let Some(obj) = updated.as_object_mut() {
            for k in ["path", "file_path", "target_file"] {
                if obj.contains_key(k) {
                    obj.insert(
                        k.to_string(),
                        serde_json::Value::String(clean_file.to_string()),
                    );
                }
            }
        }
        // This read runs before the tool's own path checks, so apply the same repo-root
        // confinement the read tools use (canonicalized, so `..` and symlinks cannot escape).
        let repo_root = std::env::current_dir()
            .map(|cwd| vox_repository::discover_repository_or_fallback(&cwd).root)
            .ok();
        let slice_note = repo_root
            .and_then(|root| {
                vox_repository::resolve_local_path_under_repo_root(&root, clean_file).ok()
            })
            .and_then(|real| std::fs::read_to_string(real).ok())
            .and_then(|src| super::ast_slice_and_preflight::extract_symbol_slice(&src, sym_name))
            .map(|slice| {
                format!(
                    "[AST SYMBOL SLICE (`{clean_file}#{sym_name}`, lines {}..={})]:\n{}",
                    slice.start_line, slice.end_line, slice.body
                )
            });
        return HookDecision::Allow {
            updated_input: Some(updated),
            additional_context: slice_note,
        };
    }

    // 2. Auto-generated SSOT file write guard
    if is_write {
        for protected in PROTECTED_GENERATED_FILES {
            if norm_path.ends_with(protected) {
                return HookDecision::Block {
                    reason: format!(
                        "PreToolUse hook blocked manual edit to generated SSOT file '{norm_path}'. Edit the source contract/frontmatter and run `vox ci command-sync` or `vox ci sync-ignore-files`."
                    ),
                };
            }
        }
        if let Some(write_body) = args
            .get("content")
            .or_else(|| args.get("replacement"))
            .or_else(|| args.get("new_str"))
            .or_else(|| args.get("replacement_code"))
            .and_then(|v| v.as_str())
        {
            let mut cargo_toml_disk = None;
            if norm_path.ends_with(".rs") {
                let mut cur = std::path::Path::new(&norm_path).parent();
                for _ in 0..6 {
                    let Some(dir) = cur else { break };
                    let candidate = dir.join("Cargo.toml");
                    if candidate.is_file() {
                        cargo_toml_disk = std::fs::read_to_string(&candidate).ok();
                        break;
                    }
                    cur = dir.parent();
                }
            }
            let report = super::ast_slice_and_preflight::preflight_check_write_guards(
                &norm_path,
                write_body,
                cargo_toml_disk.as_deref(),
            );
            if !report.banned_imports.is_empty()
                || !report.versioned_model_literals.is_empty()
                || !report.unlinked_crates.is_empty()
            {
                if let Some(reason) = report.formatted_advisory {
                    return HookDecision::Block { reason };
                }
            }
            let is_full_write = matches!(
                tool_name,
                "vox_write_file" | "write_file" | "vox_generate_code"
            );
            let old_disk = if is_full_write {
                std::fs::read_to_string(&norm_path).ok()
            } else {
                None
            };
            let v35_report = super::zero_paper_and_ratchet::audit_zero_paper_and_laziness(
                &norm_path,
                write_body,
                old_disk.as_deref(),
                None,
            );
            if v35_report.should_block_write
                && let Some(reason) = v35_report.formatted_advisory
            {
                return HookDecision::Block { reason };
            }
            let mut combined_notes = Vec::new();
            if let Some(a) = report.formatted_advisory {
                combined_notes.push(a);
            }
            if let Some(b) = v35_report.formatted_advisory {
                combined_notes.push(b);
            }
            if let Some(cat) = v35_report.symbol_catalog_header {
                combined_notes.push(cat);
            }
            if !combined_notes.is_empty() {
                return HookDecision::Allow {
                    updated_input: None,
                    additional_context: Some(combined_notes.join("\n")),
                };
            }
        }
    }

    // 3. Command tool rewrite (`cargo fmt --all`, `vox-gui --lib`, and `pnpm test -- --run` canonicalization)
    if let Some(cmd) = args
        .get("command")
        .or_else(|| args.get("cmd"))
        .and_then(|v| v.as_str())
    {
        if let Some((rewritten, reason)) = canonicalize_workspace_command(cmd) {
            let mut updated = args.clone();
            if let Some(obj) = updated.as_object_mut() {
                if obj.contains_key("command") {
                    obj.insert("command".into(), serde_json::Value::String(rewritten));
                } else if obj.contains_key("cmd") {
                    obj.insert("cmd".into(), serde_json::Value::String(rewritten));
                }
            }
            return HookDecision::Allow {
                updated_input: Some(updated),
                additional_context: Some(reason),
            };
        }
    }

    HookDecision::Allow {
        updated_input: None,
        additional_context: None,
    }
}

/// Deterministically rewrite known slow or invalid workspace shell commands (`FAST-APPLY-01`).
#[must_use]
pub fn canonicalize_workspace_command(cmd: &str) -> Option<(String, String)> {
    if cmd.contains("cargo fmt --all") {
        let rewritten = cmd.replace("cargo fmt --all", "vox run scripts/fmt.vox -- --all");
        return Some((
            rewritten,
            "PreToolUse hook rewrote `cargo fmt --all` to `vox run scripts/fmt.vox -- --all` (Windows CreateProcess safe)."
                .to_string(),
        ));
    }
    if cmd.contains("-p vox-gui") && cmd.contains("--lib") {
        let rewritten = cmd.replace("--lib", "--bin vox-gui");
        return Some((
            rewritten,
            "PreToolUse hook rewrote `vox-gui --lib` to `--bin vox-gui` (binary-only crate target)."
                .to_string(),
        ));
    }
    if cmd.contains("test -- --run ") {
        let rewritten = cmd.replace("test -- --run ", "exec vitest run ");
        return Some((
            rewritten,
            "PreToolUse hook rewrote `pnpm ... test -- --run <file>` to `exec vitest run <file>` (29x faster targeted suite execution)."
                .to_string(),
        ));
    }
    None
}

/// Resolve the local 5-Spoke MENS fast-apply model candidate when available in `ModelRegistry` (`FAST-APPLY-03`).
#[must_use]
pub fn resolve_mens_fast_apply_target(
    registry: &ModelRegistry,
    spoke: vox_corpus::training::contract::OrganicSpokeKind,
) -> Option<(String, String)> {
    let mut intent = SelectionIntent::for_task(TaskCategory::CodeGen);
    intent.axes = SelectionAxes::FAST;
    intent.complexity = 3;
    intent.prefer_local = true;
    intent.caller_hint = Some(spoke.as_str());
    let outcome = select(&intent, registry)?;
    Some((outcome.model_spec.id, outcome.model_spec.provider))
}

/// Resolve the local 5-Spoke MENS fast-apply model candidate with an optional pre-turn decision pack (`DEC-01`, `DEC-03`).
///
/// When `pack` is `None`, delegates directly to [`resolve_mens_fast_apply_target`] for byte-for-byte identity.
#[must_use]
pub fn resolve_mens_fast_apply_target_with_pack(
    registry: &ModelRegistry,
    spoke: vox_corpus::training::contract::OrganicSpokeKind,
    pack: Option<&crate::llm_bridge::TurnDecisionPack>,
) -> Option<(String, String)> {
    let Some(pack) = pack else {
        return resolve_mens_fast_apply_target(registry, spoke);
    };
    let mut intent = SelectionIntent::for_task(pack.task.unwrap_or(TaskCategory::CodeGen));
    intent.axes = SelectionAxes::FAST;
    intent.complexity = pack.complexity.map(|c| c.clamp(1, 10)).unwrap_or(3);
    intent.prefer_local = pack.prefer_local.unwrap_or(true);
    intent.caller_hint = Some(spoke.as_str());
    let outcome = select(&intent, registry)?;
    Some((outcome.model_spec.id, outcome.model_spec.provider))
}

/// Evaluate `PostToolBatch` hook after a batch of tool calls completes (`HOOKS-ORACLE-01`).
#[must_use]
pub fn evaluate_post_tool_batch_hook(
    calls: &[vox_actor_runtime::llm::EgressToolCall],
) -> Option<String> {
    let mut touched_rs = Vec::new();
    for call in calls {
        let is_write = crate::scope_guard::WRITE_TOOLS.contains(&call.name.as_str())
            || matches!(
                call.name.as_str(),
                "vox_write_file" | "vox_edit_file" | "vox_patch_file" | "vox_apply_structured_edit"
            );
        if is_write
            && let Some(p) = call
                .arguments
                .get("path")
                .or_else(|| call.arguments.get("file_path"))
                .and_then(|v| v.as_str())
            && p.ends_with(".rs")
        {
            touched_rs.push(p.to_string());
        }
    }
    if touched_rs.is_empty() {
        None
    } else {
        Some(format!(
            "[POST_TOOL_BATCH_HOOK: modified {} Rust file(s) ({}) — verify AST/compile cleanliness before finishing turn]",
            touched_rs.len(),
            touched_rs.join(", ")
        ))
    }
}

/// Evaluate `PostToolBatch` hook extended with an optional [`PostToolDecisionPack`] (`DEC-02`).
///
/// - Preserves the base [`evaluate_post_tool_batch_hook`] output when `pack` is `None`.
/// - Rejects `PostToolDirective::DraftPr` unless `shortlist_has_pr_tool` is `true`.
/// - Appends repair-escalation or PR-tool notes when confidence is below threshold or a valid directive is active.
#[must_use]
pub fn evaluate_post_tool_batch_with_decision_pack(
    calls: &[vox_actor_runtime::llm::EgressToolCall],
    pack: Option<&PostToolDecisionPack>,
    shortlist_has_pr_tool: bool,
) -> Option<String> {
    let base = evaluate_post_tool_batch_hook(calls);
    let Some(pack) = pack else {
        return base;
    };
    let mut notes: Vec<String> = base.into_iter().collect();
    if pack.should_reenter_repair_escalation() {
        notes.push(format!(
            "[POST_TOOL_DECISION_PACK: model_still_ok={:.2} < threshold={:.2} — escalating via repair_loop]",
            pack.model_still_ok_prob, pack.confidence_threshold
        ));
    }
    if let Some(directive) = pack.effective_directive(shortlist_has_pr_tool) {
        if directive == PostToolDirective::DraftPr {
            notes.push(
                "[POST_TOOL_DECISION_PACK: draft_pr directive permitted by active ChatARC PR tool shortlist]"
                    .to_string(),
            );
        }
    } else if pack.directive == Some(PostToolDirective::DraftPr) {
        notes.push(
            "[POST_TOOL_DECISION_PACK: rejected draft_pr directive because no PR tool is in the ChatARC shortlist]"
                .to_string(),
        );
    }
    if notes.is_empty() {
        None
    } else {
        Some(notes.join("\n"))
    }
}

/// Evaluate `Stop` hook when the model finishes responding (`HOOKS-ORACLE-01`).
#[must_use]
pub fn evaluate_stop_hook(
    final_content: &str,
    unresolved_verification_errors: usize,
) -> HookDecision {
    if unresolved_verification_errors > 0 {
        return HookDecision::Block {
            reason: format!(
                "Stop hook blocked turn completion: {unresolved_verification_errors} inline verification error(s) remain unresolved."
            ),
        };
    }
    if final_content.trim().is_empty() {
        return HookDecision::Block {
            reason: "Stop hook blocked empty assistant response.".to_string(),
        };
    }
    HookDecision::Allow {
        updated_input: None,
        additional_context: None,
    }
}

/// Evaluate `Stop` hook extended with an optional [`PostToolDecisionPack`] (`DEC-02`).
///
/// When `pack` is `None`, delegates directly to [`evaluate_stop_hook`].
#[must_use]
pub fn evaluate_stop_hook_with_decision_pack(
    final_content: &str,
    unresolved_verification_errors: usize,
    pack: Option<&PostToolDecisionPack>,
) -> HookDecision {
    let base = evaluate_stop_hook(final_content, unresolved_verification_errors);
    if matches!(base, HookDecision::Block { .. }) {
        return base;
    }
    let Some(pack) = pack else {
        return base;
    };
    if pack.directive == Some(PostToolDirective::StopBlock)
        || pack.should_reenter_repair_escalation()
    {
        let reason = pack.block_reason.clone().unwrap_or_else(|| {
            format!(
                "Stop hook blocked completion due to low post-tool confidence (model_still_ok={:.2} < {:.2}).",
                pack.model_still_ok_prob, pack.confidence_threshold
            )
        });
        return HookDecision::Block { reason };
    }
    base
}

/// Resolve the highest-precision Oracle reasoning model from `ModelRegistry` (`HOOKS-ORACLE-02`).
#[must_use]
pub fn consult_oracle_model_advisory(
    registry: &ModelRegistry,
    question: &str,
) -> Option<OracleConsultationAdvisory> {
    let mut intent = SelectionIntent::for_task(TaskCategory::Planning);
    intent.axes = SelectionAxes::QUALITY_FIRST;
    intent.complexity = 9;
    intent.caller_hint = Some("oracle-consult");
    intent.prefer_local = false;
    intent.allow_free_in_performance_mode = true;
    let outcome = select(&intent, registry)?;
    let spec = outcome.model_spec;
    Some(OracleConsultationAdvisory {
        oracle_model_id: spec.id.clone(),
        provider: spec.provider.clone(),
        complexity: 9,
        selection_reason: outcome.reason.to_string(),
        advisory_summary: format!(
            "[ORACLE ESCALATION ({} via {}): Architectural & root-cause verification recommended for: {}]",
            spec.id,
            spec.provider,
            question.chars().take(160).collect::<String>()
        ),
    })
}

/// Consult Oracle model advisory with an optional [`PostToolDecisionPack`] (`DEC-02`, `DEC-03`).
///
/// When `pack` is `None`, delegates directly to [`consult_oracle_model_advisory`] for byte-for-byte identity.
#[must_use]
pub fn consult_oracle_model_advisory_with_pack(
    registry: &ModelRegistry,
    question: &str,
    pack: Option<&PostToolDecisionPack>,
) -> Option<OracleConsultationAdvisory> {
    let Some(pack) = pack else {
        return consult_oracle_model_advisory(registry, question);
    };
    if pack.directive != Some(PostToolDirective::ConsultOracle)
        && !pack.should_reenter_repair_escalation()
    {
        return None;
    }
    consult_oracle_model_advisory(registry, question)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pre_tool_use_hook_blocks_dotenv_and_generated_ssot_writes_and_rewrites_cargo_fmt() {
        let block_env = evaluate_pre_tool_use_hook("vox_read_file", &json!({"path": ".env.local"}));
        assert!(matches!(block_env, HookDecision::Block { .. }));

        let allow_example =
            evaluate_pre_tool_use_hook("vox_read_file", &json!({"path": ".env.example"}));
        assert!(matches!(allow_example, HookDecision::Allow { .. }));

        let block_gen = evaluate_pre_tool_use_hook(
            "vox_write_file",
            &json!({"path": "docs/src/SUMMARY.md", "content": "x"}),
        );
        assert!(matches!(block_gen, HookDecision::Block { .. }));

        let rewrite_fmt = evaluate_pre_tool_use_hook(
            "vox_run_command",
            &json!({"command": "cargo fmt --all -- --check"}),
        );
        match rewrite_fmt {
            HookDecision::Allow {
                updated_input: Some(u),
                additional_context: Some(ctx),
            } => {
                assert!(
                    u["command"]
                        .as_str()
                        .unwrap()
                        .contains("vox run scripts/fmt.vox -- --all")
                );
                assert!(ctx.contains("PreToolUse hook rewrote"));
            }
            other => panic!("expected rewrite Allow, got {other:?}"),
        }
    }

    #[test]
    fn dotenv_guard_cannot_be_bypassed_with_a_symbol_suffix_or_case() {
        // `#Symbol` is stripped before the read, so the guard must see the stripped path.
        for path in [".env#KEY", "config/.env.local#X", ".ENV", "sub/.Env.Prod#Y"] {
            let d = evaluate_pre_tool_use_hook("vox_read_file", &json!({ "path": path }));
            assert!(
                matches!(d, HookDecision::Block { .. }),
                "{path} must be blocked, got {d:?}"
            );
        }
        let ok = evaluate_pre_tool_use_hook("vox_read_file", &json!({"path": ".env.example#A"}));
        assert!(matches!(ok, HookDecision::Allow { .. }));
    }

    #[test]
    fn symbol_slice_reads_only_inside_the_repository() {
        // Control: an in-repo file and real symbol still yield a slice.
        let in_repo = "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/lifecycle_hooks.rs#evaluate_stop_hook";
        let d = evaluate_pre_tool_use_hook("vox_read_file", &json!({ "path": in_repo }));
        assert!(
            matches!(d, HookDecision::Allow { additional_context: Some(ref c), .. } if c.contains("evaluate_stop_hook")),
            "control slice missing: {d:?}"
        );

        // A real file with a real symbol outside the repo, reached via `..` from the root.
        let outside = std::env::temp_dir().join("vox_hook_slice_escape_probe.rs");
        std::fs::write(&outside, "pub fn escaped_symbol() -> u8 { 7 }\n").unwrap();
        let canon = std::fs::canonicalize(&outside).unwrap();
        let dotdot = format!(
            "{}{}#escaped_symbol",
            "../".repeat(32),
            canon.to_string_lossy().trim_start_matches('/')
        );
        let d = evaluate_pre_tool_use_hook("vox_read_file", &json!({ "path": dotdot }));
        let _ = std::fs::remove_file(&outside);
        assert!(
            matches!(
                d,
                HookDecision::Allow {
                    additional_context: None,
                    ..
                }
            ),
            "`..` escape leaked a slice: {d:?}"
        );
    }

    #[test]
    fn post_tool_batch_and_stop_hooks_enforce_verification_and_oracle_advisory_resolves() {
        let calls = vec![vox_actor_runtime::llm::EgressToolCall {
            id: "c1".into(),
            name: "vox_apply_structured_edit".into(),
            arguments: json!({"path": "crates/vox-cli/src/lib.rs"}),
        }];
        let note = evaluate_post_tool_batch_hook(&calls).expect("must emit rust batch note");
        assert!(note.contains("crates/vox-cli/src/lib.rs"));

        assert!(matches!(
            evaluate_stop_hook("All tests pass.", 1),
            HookDecision::Block { .. }
        ));
        assert!(matches!(
            evaluate_stop_hook("All tests pass.", 0),
            HookDecision::Allow { .. }
        ));

        let reg = ModelRegistry::new();
        let oracle =
            consult_oracle_model_advisory(&reg, "Diagnose async deadlock in actor mailbox")
                .expect("oracle must resolve a candidate");
        assert!(!oracle.oracle_model_id.is_empty());
        assert_eq!(oracle.complexity, 9);
    }

    #[test]
    fn canonicalize_workspace_command_and_preflight_write_guards_enforce_v34_fast_paths() {
        let gui_cmd = evaluate_pre_tool_use_hook(
            "vox_run_command",
            &json!({"command": "cargo test -p vox-gui --lib chat_arena_branch"}),
        );
        assert!(
            matches!(gui_cmd, HookDecision::Allow { updated_input: Some(ref u), .. }
                if u["command"].as_str() == Some("cargo test -p vox-gui --bin vox-gui chat_arena_branch"))
        );

        let vitest_cmd = evaluate_pre_tool_use_hook(
            "vox_run_command",
            &json!({"command": "pnpm --dir crates/vox-gui/ui test -- --run src/App.test.tsx"}),
        );
        assert!(
            matches!(vitest_cmd, HookDecision::Allow { updated_input: Some(ref u), .. }
                if u["command"].as_str() == Some("pnpm --dir crates/vox-gui/ui exec vitest run src/App.test.tsx"))
        );

        let blocked_versioned = evaluate_pre_tool_use_hook(
            "vox_apply_structured_edit",
            &json!({
                "path": "crates/vox-gui/ui/src/components/Chat.tsx",
                "replacement": "const m = 'anthropic/claude-sonnet-4.6';"
            }),
        );
        assert!(matches!(blocked_versioned, HookDecision::Block { .. }));

        let reg = ModelRegistry::new();
        assert!(
            resolve_mens_fast_apply_target(
                &reg,
                vox_corpus::training::contract::OrganicSpokeKind::RustWorkspace,
            )
            .is_some()
        );

        let probe_dir = std::env::temp_dir().join("vox_v34_live_hook_probe");
        let probe_src = probe_dir.join("src");
        let _ = std::fs::create_dir_all(&probe_src);
        std::fs::write(
            probe_dir.join("Cargo.toml"),
            "[package]\nname = \"probe\"\n[dependencies]\nvox-orchestrator = { workspace = true }\n",
        )
        .unwrap();
        let target_rs = probe_src.join("lib.rs");
        std::fs::write(
            &target_rs,
            concat!(
                "use vox_orchestrator::models::SelectionAxes;\n\n",
                "pub ",
                "struct TargetSymbol {\n    pub ok: bool,\n}\n"
            ),
        )
        .unwrap();

        let unlinked_block = evaluate_pre_tool_use_hook(
            "vox_apply_structured_edit",
            &json!({
                "path": target_rs.to_string_lossy(),
                "replacement": "use vox_telemetry::spool::upload;\n"
            }),
        );
        assert!(
            matches!(unlinked_block, HookDecision::Block { ref reason } if reason.contains("vox_telemetry"))
        );

        let sym_query = format!("{}#TargetSymbol", target_rs.to_string_lossy());
        let sym_hook =
            evaluate_pre_tool_use_hook("vox_repo_query_file", &json!({ "path": sym_query }));
        // The probe lives in the OS temp dir, outside the repository: the `#Symbol` suffix is
        // still stripped, but no slice may be read from outside the repo root.
        assert!(matches!(
            sym_hook,
            HookDecision::Allow { updated_input: Some(ref u), additional_context: None }
                if u["path"].as_str() == Some(target_rs.to_string_lossy().as_ref())
        ));
        let _ = std::fs::remove_dir_all(&probe_dir);

        let mut big_code = String::from("HEAD_START\n");
        big_code.push_str(&"// padding\n".repeat(320));
        big_code.push_str(concat!(
            "\npub ",
            "struct MiddleImportantType {\n    pub v: u32,\n}\n"
        ));
        big_code.push_str(&"// tail padding\n".repeat(250));
        big_code.push_str("TAIL_END");
        let (capped, _) =
            super::super::cap_and_summarize_tool_output("vox_repo_query_file", &big_code);
        assert!(
            capped.contains("[OMITTED REGION AST SKELETON]")
                && capped.contains("MiddleImportantType")
        );
        assert!(
            super::super::try_parse_streaming_read_prefetch(
                "vox_repo_query_file",
                r#"{"path":"crates/vox-cli/src/main.rs"}"#
            )
            .is_some()
        );
    }

    #[test]
    fn dec_02_post_tool_low_confidence_reenters_repair_escalation_and_guards_draft_pr() {
        use std::path::PathBuf;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use vox_orchestrator::{
            AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
        };
        use vox_repository::{RepoCapabilities, RepositoryContext};
        use vox_skills::new_registry_arc;

        let cfg = OrchestratorConfig::for_testing();
        let orch_cfg = cfg.clone();
        let groups = AffinityGroupRegistry::new(vec![]);
        let session_cfg = SessionConfig {
            persist: false,
            sessions_dir: std::env::temp_dir().join("vox-dec02-post-tool-sessions"),
            ..SessionConfig::default()
        };
        let session_manager = SessionManager::new(session_cfg).expect("session manager");
        let repository = RepositoryContext {
            root: PathBuf::from("."),
            git_root: None,
            repository_id: "dec02-test".into(),
            origin_url: None,
            capabilities: RepoCapabilities {
                vox_project: false,
                cargo_workspace: false,
                cargo_package: false,
                node_workspace: false,
                python_project: false,
                go_module: false,
                git: false,
            },
            has_vox_agents_dir: false,
            vox_toml: None,
        };
        let state = crate::server_state::ServerState::hermetic_stub(
            cfg,
            repository,
            Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
            Arc::new(Mutex::new(session_manager)),
            new_registry_arc(),
        );

        let template = vox_actor_runtime::llm::LlmConfig::openrouter("initial-model");
        let mut active_cfg = template.clone();
        let mut telemetry = std::collections::HashMap::new();

        let low_conf_pack = PostToolDecisionPack {
            schema_version: POST_TOOL_DECISION_PACK_SCHEMA_V1.to_string(),
            model_still_ok_prob: 0.31,
            confidence_threshold: 0.70,
            directive: Some(PostToolDirective::DraftPr),
            block_reason: None,
        };

        // shortlist_has_pr_tool = false -> draft_pr rejected, low confidence triggers repair escalation
        let escalated = super::super::apply_post_tool_confidence_escalation(
            &state,
            &template,
            &mut active_cfg,
            &mut telemetry,
            Some(&low_conf_pack),
            false,
        );
        assert!(
            escalated,
            "low model_still_ok_prob must trigger repair escalation"
        );
        assert_eq!(
            active_cfg.telemetry_strength_tag.as_deref(),
            Some("sota_repair_cascade")
        );
        assert_eq!(active_cfg.temperature, Some(0.1));
        assert_eq!(
            telemetry.get("post_tool_low_confidence_escalated"),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            telemetry.get("post_tool_rejected_directive"),
            Some(&serde_json::Value::String("draft_pr".to_string())),
            "draft_pr must be rejected when no PR tool is in the ChatARC shortlist"
        );

        let calls = vec![vox_actor_runtime::llm::EgressToolCall {
            id: "c1".into(),
            name: "vox_apply_structured_edit".into(),
            arguments: json!({"path": "crates/vox-cli/src/lib.rs"}),
        }];
        let batch_note =
            evaluate_post_tool_batch_with_decision_pack(&calls, Some(&low_conf_pack), false)
                .expect("batch note");
        assert!(batch_note.contains("escalating via repair_loop"));
        assert!(batch_note.contains("rejected draft_pr directive"));

        let batch_note_with_pr =
            evaluate_post_tool_batch_with_decision_pack(&calls, Some(&low_conf_pack), true)
                .expect("batch note with pr");
        assert!(batch_note_with_pr.contains("draft_pr directive permitted"));

        let stop_dec =
            evaluate_stop_hook_with_decision_pack("Finished edits.", 0, Some(&low_conf_pack));
        assert!(
            matches!(stop_dec, HookDecision::Block { ref reason } if reason.contains("model_still_ok=0.31"))
        );
    }

    #[test]
    fn dec_03_flag_off_produces_byte_identical_outputs_across_all_four_hooks() {
        use crate::llm_bridge::{
            CallerExplicitRouteFlags, build_selection_request,
            build_selection_request_with_decision_pack,
        };
        use std::path::PathBuf;
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use vox_orchestrator::config::CostPreference;
        use vox_orchestrator::mode::{ClutchProfile, RiskPosture};
        use vox_orchestrator::models::Capability;
        use vox_orchestrator::{
            AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
        };
        use vox_repository::{RepoCapabilities, RepositoryContext};
        use vox_skills::new_registry_arc;

        // 1. build_selection_request
        let req_base = build_selection_request(
            TaskCategory::CodeGen,
            6,
            true,
            false,
            CostPreference::Performance,
            vec![Capability::SupportsToolUse],
            Some(ClutchProfile::Balanced),
            Some(RiskPosture::Moderate),
            Some(0.5),
        );
        let req_flag_off = build_selection_request_with_decision_pack(
            TaskCategory::CodeGen,
            6,
            true,
            false,
            CostPreference::Performance,
            vec![Capability::SupportsToolUse],
            Some(ClutchProfile::Balanced),
            Some(RiskPosture::Moderate),
            Some(0.5),
            CallerExplicitRouteFlags::default(),
            None,
        );
        assert_eq!(
            format!("{req_base:?}").as_bytes(),
            format!("{req_flag_off:?}").as_bytes()
        );

        // 2. resolve_mens_fast_apply_target
        let reg = ModelRegistry::new();
        let spoke = vox_corpus::training::contract::OrganicSpokeKind::RustWorkspace;
        let fast_base = resolve_mens_fast_apply_target(&reg, spoke);
        let fast_flag_off = resolve_mens_fast_apply_target_with_pack(&reg, spoke, None);
        assert_eq!(
            serde_json::to_vec(&fast_base).unwrap(),
            serde_json::to_vec(&fast_flag_off).unwrap()
        );

        // 3. consult_oracle_model_advisory
        let q = "Verify actor supervision tree deadlock freedom";
        let oracle_base = consult_oracle_model_advisory(&reg, q);
        let oracle_flag_off = consult_oracle_model_advisory_with_pack(&reg, q, None);
        assert_eq!(
            serde_json::to_vec(&oracle_base).unwrap(),
            serde_json::to_vec(&oracle_flag_off).unwrap()
        );

        // 4. apply_repair_escalation_to_config
        let cfg = OrchestratorConfig::for_testing();
        let orch_cfg = cfg.clone();
        let groups = AffinityGroupRegistry::new(vec![]);
        let session_cfg = SessionConfig {
            persist: false,
            sessions_dir: std::env::temp_dir().join("vox-dec03-flag-off-sessions"),
            ..SessionConfig::default()
        };
        let session_manager = SessionManager::new(session_cfg).expect("session manager");
        let repository = RepositoryContext {
            root: PathBuf::from("."),
            git_root: None,
            repository_id: "dec03-test".into(),
            origin_url: None,
            capabilities: RepoCapabilities {
                vox_project: false,
                cargo_workspace: false,
                cargo_package: false,
                node_workspace: false,
                python_project: false,
                go_module: false,
                git: false,
            },
            has_vox_agents_dir: false,
            vox_toml: None,
        };
        let state = crate::server_state::ServerState::hermetic_stub(
            cfg,
            repository,
            Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
            Arc::new(Mutex::new(session_manager)),
            new_registry_arc(),
        );
        let template = vox_actor_runtime::llm::LlmConfig::openrouter("baseline-model");
        let mut cfg_a = template.clone();
        let mut tel_a = std::collections::HashMap::new();
        super::super::apply_repair_escalation_to_config(&state, &template, &mut cfg_a, &mut tel_a);

        let mut cfg_b = template.clone();
        let mut tel_b = std::collections::HashMap::new();
        // Flag-off post-tool pack is a no-op before/after direct repair escalation:
        let did_escalate = super::super::apply_post_tool_confidence_escalation(
            &state, &template, &mut cfg_b, &mut tel_b, None, false,
        );
        assert!(!did_escalate);
        assert_eq!(
            format!("{template:?}").as_bytes(),
            format!("{cfg_b:?}").as_bytes()
        );
        assert!(tel_b.is_empty());

        super::super::apply_repair_escalation_to_config(&state, &template, &mut cfg_b, &mut tel_b);
        assert_eq!(
            format!("{cfg_a:?}").as_bytes(),
            format!("{cfg_b:?}").as_bytes()
        );
        assert_eq!(
            serde_json::to_vec(&tel_a).unwrap(),
            serde_json::to_vec(&tel_b).unwrap()
        );
    }
}
