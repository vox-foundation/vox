//! # Typed, Level-Independent Safety Floor
//!
//! Pure action-only safety floor classifier (FLOOR-01, FLOOR-02).
//!
//! ## Architecture & Principles
//!
//! - **Action-Only (FLOOR-01):** `classify_floor` takes ONLY the action (tool name + args + repo root,
//!   or task write paths + tool hints). It deliberately takes no oversight level, `PermissionMode`,
//!   `RiskPosture`, or trust score. A floored call is never auto-approved by any `PermissionMode`
//!   and never satisfied by a persisted allowlist entry.
//! - **Composition (Phase 13):** Effective tier = `max(floor, level, trust)`.
//! - **Deliberately NOT Floored:**
//!   - `vox_redo`: Re-applies an op the human already allowed.
//!   - `vox_news_approve`: Records one maker-checker vote in VoxDb; live syndication requires two humans.
//!   - `vox_publish_message`: In-process agent bulletin board.
//!   - `vox_agy_delegate*`: Returns a diff for human review.
//!   - `vox_attention_reset`: P12-D5, governed by oversight level.
//!   - `vox_resolve_approval`: Not floored, since floored parks must remain resolvable by humans;
//!     12-07 instead refuses approve/modify of a floored park from in-process agent dispatchers,
//!     and the authenticated channel is FLOOR-04 / Phase 14 (T-12-08).
//!   - Plain shell and delete: D3, governed by oversight level.
//! - **Secrets-Write (P12-D3):** No dedicated secrets-write tool exists; path and key pattern matching
//!   is a best-effort structural guard.
//! - **Shell Classification Limits:** Command line pattern matching (`command_rewrites_history`)
//!   is a best-effort escalator, not a cryptographically hardened sandbox boundary. Shell aliases,
//!   nested invocations like `sh -c "$(…)"`, script wrappers, `GIT_*` environment variables,
//!   unusual encoding, and `+` refspecs configured in git config files are outside its lexical scope.

use crate::attention::ApprovalTier;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Safety floor categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorCategory {
    HistoryRewrite,
    PolicyGrant,
    ExternalEffect,
    SecretsWrite,
}

/// All floor categories in precedence order: SecretsWrite -> PolicyGrant -> HistoryRewrite -> ExternalEffect.
pub const ALL: [FloorCategory; 4] = [
    FloorCategory::SecretsWrite,
    FloorCategory::PolicyGrant,
    FloorCategory::HistoryRewrite,
    FloorCategory::ExternalEffect,
];

impl FloorCategory {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::HistoryRewrite => "history_rewrite",
            Self::PolicyGrant => "policy_grant",
            Self::ExternalEffect => "external_effect",
            Self::SecretsWrite => "secrets_write",
        }
    }
}

/// Tools whose invocation unconditionally rewrites or destroys repository history.
pub const HISTORY_REWRITE_TOOLS: &[&str] =
    &["vox_snapshot_restore", "vox_undo", "vox_workspace_merge"];

/// Tools whose invocation grants elevated or future permissions.
pub const POLICY_GRANT_TOOLS: &[&str] = &[
    "vox_add_approval_allowlist_entry",
    "vox_config_set",
    "vox_trust_override",
];

/// Tools whose invocation triggers external side effects, deployments, or untrusted installations.
pub const EXTERNAL_EFFECT_TOOLS: &[&str] = &[
    "vox_agent_gateway_call",
    "vox_deploy",
    "vox_mesh_dispatch",
    "vox_openclaw_gateway_call",
    "vox_plugin_install",
    "vox_scientia_publication_publish",
    "vox_scientia_publication_submit_local",
    "vox_skill_add",
    "vox_skill_install",
];

/// Shell command execution tools subject to history rewrite pattern matching.
pub const SHELL_COMMAND_TOOLS: &[&str] = &["vox_run_shell", "vox_skill_run"];

/// File write tools inspected for policy or secrets path violations.
pub const FILE_WRITE_TOOLS: &[&str] = &["vox_apply_structured_edit", "vox_generate_code"];

const PATH_ARG_KEYS: &[&str] = &["path", "file_path", "target_file", "output_path"];

/// Input to the safety floor classifier.
#[derive(Debug, Clone)]
pub enum FloorInput<'a> {
    /// MCP tool invocation.
    Tool {
        name: &'a str,
        args: &'a serde_json::Value,
        repo_root: &'a Path,
    },
    /// Autonomous task plan / declaration.
    Task {
        write_paths: &'a [PathBuf],
        tool_hints: &'a [String],
    },
}

impl<'a> FloorInput<'a> {
    /// Construct a tool call input.
    pub fn tool(name: &'a str, args: &'a serde_json::Value, repo_root: &'a Path) -> Self {
        Self::Tool {
            name,
            args,
            repo_root,
        }
    }

    /// Construct a task input.
    pub fn task(write_paths: &'a [PathBuf], tool_hints: &'a [String]) -> Self {
        Self::Task {
            write_paths,
            tool_hints,
        }
    }
}

/// Tools floored under a category by canonical name.
#[must_use]
pub fn tools_for(c: FloorCategory) -> &'static [&'static str] {
    match c {
        FloorCategory::HistoryRewrite => HISTORY_REWRITE_TOOLS,
        FloorCategory::PolicyGrant => POLICY_GRANT_TOOLS,
        FloorCategory::ExternalEffect => EXTERNAL_EFFECT_TOOLS,
        FloorCategory::SecretsWrite => &[],
    }
}

/// Arg-rule IDs applicable to a category.
#[must_use]
pub fn arg_rule_ids(c: FloorCategory) -> &'static [&'static str] {
    match c {
        FloorCategory::HistoryRewrite => &["shell_command_history_rewrite"],
        FloorCategory::PolicyGrant => &["policy_file_write"],
        FloorCategory::ExternalEffect => &[],
        FloorCategory::SecretsWrite => &["secrets_path_write", "secrets_config_key"],
    }
}

/// Check whether a tool name is unconditionally floored under any category.
#[must_use]
pub fn name_is_floored(name: &str) -> bool {
    ALL.iter().any(|&c| tools_for(c).contains(&name))
}

/// Map a safety floor decision to an attention `ApprovalTier`.
#[must_use]
pub fn floor_tier(c: Option<FloorCategory>) -> ApprovalTier {
    match c {
        Some(_) => ApprovalTier::Review,
        None => ApprovalTier::AutoApprove,
    }
}

/// Pure safety floor classifier. Deliberately takes only the action (name + args + root or paths + hints) —
/// no oversight level, PermissionMode, RiskPosture, or trust score (FLOOR-01).
///
/// Precedence (first match wins): SecretsWrite -> PolicyGrant -> HistoryRewrite -> ExternalEffect.
#[must_use]
pub fn classify_floor(input: &FloorInput<'_>) -> Option<FloorCategory> {
    match input {
        FloorInput::Tool {
            name,
            args,
            repo_root,
        } => {
            // 1. SecretsWrite
            if FILE_WRITE_TOOLS.contains(name) {
                if let Some(raw_path) = extract_path_arg(args) {
                    let resolved = resolved_path(repo_root, raw_path);
                    if path_is_secrets(&resolved) {
                        return Some(FloorCategory::SecretsWrite);
                    }
                }
            }
            if *name == "vox_config_set" && json_has_secret_key(args) {
                return Some(FloorCategory::SecretsWrite);
            }

            // 2. PolicyGrant
            if POLICY_GRANT_TOOLS.contains(name) {
                return Some(FloorCategory::PolicyGrant);
            }
            if FILE_WRITE_TOOLS.contains(name) {
                if let Some(raw_path) = extract_path_arg(args) {
                    let resolved = resolved_path(repo_root, raw_path);
                    if path_is_policy_file(&resolved) {
                        return Some(FloorCategory::PolicyGrant);
                    }
                }
            }

            // 3. HistoryRewrite
            if HISTORY_REWRITE_TOOLS.contains(name) {
                return Some(FloorCategory::HistoryRewrite);
            }
            if SHELL_COMMAND_TOOLS.contains(name) {
                if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                    if command_rewrites_history(cmd) {
                        return Some(FloorCategory::HistoryRewrite);
                    }
                }
            }

            // 4. ExternalEffect
            if EXTERNAL_EFFECT_TOOLS.contains(name) {
                return Some(FloorCategory::ExternalEffect);
            }

            None
        }
        FloorInput::Task {
            write_paths,
            tool_hints,
        } => {
            // 1. SecretsWrite
            for path in *write_paths {
                let norm = normalize_lexical(path);
                if path_is_secrets(&norm) {
                    return Some(FloorCategory::SecretsWrite);
                }
            }

            // 2. PolicyGrant
            for path in *write_paths {
                let norm = normalize_lexical(path);
                if path_is_policy_file(&norm) {
                    return Some(FloorCategory::PolicyGrant);
                }
            }
            for hint in *tool_hints {
                if POLICY_GRANT_TOOLS.contains(&hint.as_str()) {
                    return Some(FloorCategory::PolicyGrant);
                }
            }

            // 3. HistoryRewrite
            for hint in *tool_hints {
                if HISTORY_REWRITE_TOOLS.contains(&hint.as_str()) {
                    return Some(FloorCategory::HistoryRewrite);
                }
            }

            // 4. ExternalEffect
            for hint in *tool_hints {
                if EXTERNAL_EFFECT_TOOLS.contains(&hint.as_str()) {
                    return Some(FloorCategory::ExternalEffect);
                }
            }

            None
        }
    }
}

fn extract_path_arg(args: &serde_json::Value) -> Option<&str> {
    for &key in PATH_ARG_KEYS {
        if let Some(val) = args.get(key).and_then(|v| v.as_str()) {
            return Some(val);
        }
    }
    None
}

fn resolved_path(repo_root: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    let candidate = if p.is_absolute() || raw.starts_with('/') || raw.starts_with('\\') {
        p.to_path_buf()
    } else {
        repo_root.join(p)
    };
    normalize_lexical(&candidate)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = normalized.components().next_back() {
                    normalized.pop();
                } else if normalized.is_absolute() {
                    // At root, parent stays root
                } else {
                    normalized.push("..");
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                normalized.push(comp.as_os_str());
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
        }
    }
    normalized
}

fn path_is_secrets(path: &Path) -> bool {
    let comps: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    for (i, c) in comps.iter().enumerate() {
        if c == ".ssh" || c == ".secrets" {
            return true;
        }
        if c == ".aws" && comps.get(i + 1).map(|s| s.as_str()) == Some("credentials") {
            return true;
        }
    }

    if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
        if file_name == ".env"
            || file_name.starts_with(".env.")
            || file_name.ends_with(".pem")
            || file_name.ends_with(".key")
            || file_name.starts_with("id_rsa")
            || file_name.starts_with("id_ed25519")
            || file_name == ".secrets"
            || file_name == ".netrc"
            || file_name == ".npmrc"
        {
            return true;
        }
    }

    false
}

fn path_is_policy_file(path: &Path) -> bool {
    let comps: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    for (i, c) in comps.iter().enumerate() {
        if c == ".vox" && comps.get(i + 1).map(|s| s.as_str()) == Some("agents") {
            return true;
        }
        if c == "contracts" && comps.get(i + 1).map(|s| s.as_str()) == Some("orchestration") {
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.starts_with("permission-modes.") {
                    return true;
                }
            }
        }
    }

    if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
        if file_name == "Vox.toml" {
            return true;
        }
    }

    false
}

fn json_has_secret_key(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let lower = k.to_ascii_lowercase();
                if lower.contains("token")
                    || lower.contains("secret")
                    || lower.contains("password")
                    || lower.contains("api_key")
                    || lower.contains("apikey")
                    || lower.contains("credential")
                    || lower.contains("private_key")
                {
                    return true;
                }
                if json_has_secret_key(v) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => arr.iter().any(json_has_secret_key),
        _ => false,
    }
}

fn command_rewrites_history(cmd: &str) -> bool {
    let segments = cmd.split([';', '&', '|']);
    for segment in segments {
        let tokens: Vec<&str> = segment.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        for (i, &token) in tokens.iter().enumerate() {
            if token == "git" || token.ends_with("/git") || token.ends_with("\\git") {
                if git_tokens_rewrite_history(&tokens[i + 1..]) {
                    return true;
                }
            } else if token == "jj" || token.ends_with("/jj") || token.ends_with("\\jj") {
                if jj_tokens_rewrite_history(&tokens[i + 1..]) {
                    return true;
                }
            }
        }
    }
    false
}

fn git_tokens_rewrite_history(mut args: &[&str]) -> bool {
    while !args.is_empty() {
        if args[0] == "-c" || args[0] == "-C" {
            args = if args.len() > 1 { &args[2..] } else { &[] };
        } else if args[0].starts_with("-c") || args[0].starts_with("-C") {
            args = &args[1..];
        } else {
            break;
        }
    }

    if args.is_empty() {
        return false;
    }

    let subcmd = args[0];
    let rest = &args[1..];

    match subcmd {
        "push" => {
            for &arg in rest {
                if arg == "-f"
                    || arg == "--force"
                    || arg == "--force-if-includes"
                    || arg == "--mirror"
                    || arg == "--delete"
                    || arg == "-d"
                    || arg.starts_with("--force-with-lease")
                {
                    return true;
                }
                if arg.starts_with('-') && !arg.starts_with("--") && arg.contains('f') {
                    return true;
                }
                if arg.starts_with('+') && arg.len() > 1 {
                    return true;
                }
                if arg.starts_with(':') && arg.len() > 1 {
                    return true;
                }
            }
            false
        }
        "filter-branch" | "filter-repo" => true,
        "reset" => rest.contains(&"--hard"),
        "rebase" => {
            if let Some(&first) = rest.first() {
                if first == "--abort"
                    || first == "--continue"
                    || first == "--skip"
                    || first == "--quit"
                {
                    return false;
                }
            }
            true
        }
        "commit" => rest.contains(&"--amend"),
        "update-ref" => rest.contains(&"-d"),
        "reflog" => rest.contains(&"expire"),
        _ => false,
    }
}

fn jj_tokens_rewrite_history(args: &[&str]) -> bool {
    if args.is_empty() {
        return false;
    }
    if args.starts_with(&["git", "push"]) {
        return args[2..].contains(&"--deleted");
    }
    if args[0] == "abandon" || args[0] == "undo" {
        return true;
    }
    if args.starts_with(&["op", "restore"]) || args.starts_with(&["op", "undo"]) {
        return true;
    }
    false
}

#[cfg(test)]
pub mod safety_floor_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn safety_floor_config_set_is_policy_grant() {
        let root = Path::new("/tmp/test-repo");
        let args = json!({"max_agents": 2});
        let input = FloorInput::tool("vox_config_set", &args, root);
        assert_eq!(classify_floor(&input), Some(FloorCategory::PolicyGrant));
    }

    #[test]
    fn safety_floor_tier_is_review() {
        assert_eq!(
            floor_tier(Some(FloorCategory::PolicyGrant)),
            ApprovalTier::Review
        );
        assert_eq!(
            floor_tier(Some(FloorCategory::HistoryRewrite)),
            ApprovalTier::Review
        );
        assert_eq!(
            floor_tier(Some(FloorCategory::ExternalEffect)),
            ApprovalTier::Review
        );
        assert_eq!(
            floor_tier(Some(FloorCategory::SecretsWrite)),
            ApprovalTier::Review
        );
        assert_eq!(floor_tier(None), ApprovalTier::AutoApprove);
    }

    #[test]
    fn safety_floor_signature_is_action_only() {
        let f: fn(&FloorInput<'_>) -> Option<FloorCategory> = classify_floor;
        let _ = f;
    }

    #[test]
    fn safety_floor_history_tools() {
        let root = Path::new("/tmp/test-repo");
        let empty_args = json!({});
        for tool in ["vox_snapshot_restore", "vox_undo", "vox_workspace_merge"] {
            let input = FloorInput::tool(tool, &empty_args, root);
            assert_eq!(
                classify_floor(&input),
                Some(FloorCategory::HistoryRewrite),
                "tool {tool}"
            );
        }
        let redo_input = FloorInput::tool("vox_redo", &empty_args, root);
        assert_eq!(classify_floor(&redo_input), None);
    }

    #[test]
    fn safety_floor_force_push_argv() {
        let root = Path::new("/tmp/test-repo");
        let positive = [
            "git push --force origin main",
            "git push -f",
            "git push -uf origin x",
            "git push origin +main",
            "git push origin :old",
            "git push --force-with-lease=main",
            "git -c core.x=y push --mirror",
            "cd repo && git push --force",
            "git filter-repo --path x",
            "git reset --hard HEAD~1",
            "git rebase -i main",
            "git commit --amend",
            "jj git push --deleted",
            "jj op restore abc",
        ];

        for cmd in positive {
            let args = json!({"command": cmd});
            let input_skill = FloorInput::tool("vox_skill_run", &args, root);
            assert_eq!(
                classify_floor(&input_skill),
                Some(FloorCategory::HistoryRewrite),
                "cmd {cmd} via vox_skill_run"
            );
            let input_shell = FloorInput::tool("vox_run_shell", &args, root);
            assert_eq!(
                classify_floor(&input_shell),
                Some(FloorCategory::HistoryRewrite),
                "cmd {cmd} via vox_run_shell"
            );
        }

        let negative = [
            "git push origin main",
            "git rebase --continue",
            "git status",
            "echo force",
            "rm -rf build",
        ];

        for cmd in negative {
            let args = json!({"command": cmd});
            let input_skill = FloorInput::tool("vox_skill_run", &args, root);
            assert_eq!(classify_floor(&input_skill), None, "negative cmd {cmd}");
        }
    }

    #[test]
    fn safety_floor_policy_grants() {
        let root = Path::new("/tmp/test-repo");
        let empty_args = json!({});
        assert_eq!(
            classify_floor(&FloorInput::tool(
                "vox_add_approval_allowlist_entry",
                &empty_args,
                root
            )),
            Some(FloorCategory::PolicyGrant)
        );
        let config_args = json!({"max_agents": 2});
        assert_eq!(
            classify_floor(&FloorInput::tool("vox_config_set", &config_args, root)),
            Some(FloorCategory::PolicyGrant)
        );
        assert_eq!(
            classify_floor(&FloorInput::tool("vox_trust_override", &empty_args, root)),
            Some(FloorCategory::PolicyGrant)
        );

        for path in [
            "Vox.toml",
            // drift-allow(vox-path-literal): test verifies policy floor classification on .vox/agents/* path
            ".vox/agents/builder.md",
            "contracts/orchestration/permission-modes.v2.yaml",
        ] {
            let args = json!({"file_path": path, "replacement_code": "code"});
            let input = FloorInput::tool("vox_apply_structured_edit", &args, root);
            assert_eq!(
                classify_floor(&input),
                Some(FloorCategory::PolicyGrant),
                "policy path {path}"
            );
        }
    }

    #[test]
    fn safety_floor_external_effects() {
        let root = Path::new("/tmp/test-repo");
        let empty_args = json!({});
        for &tool in EXTERNAL_EFFECT_TOOLS {
            let input = FloorInput::tool(tool, &empty_args, root);
            assert_eq!(
                classify_floor(&input),
                Some(FloorCategory::ExternalEffect),
                "tool {tool}"
            );
        }
    }

    #[test]
    fn safety_floor_secrets_paths() {
        let root = Path::new("/tmp/test-repo");
        let secrets_paths = [
            ".env",
            "config/.env.local",
            "/Users/x/.ssh/id_ed25519",
            "../../.env",
            "certs/server.pem",
            "~/.aws/credentials",
        ];

        for path in secrets_paths {
            let args = json!({"file_path": path});
            let input = FloorInput::tool("vox_apply_structured_edit", &args, root);
            assert_eq!(
                classify_floor(&input),
                Some(FloorCategory::SecretsWrite),
                "path {path}"
            );
        }

        let gen_args = json!({"output_path": ".env.production"});
        let input_gen = FloorInput::tool("vox_generate_code", &gen_args, root);
        assert_eq!(
            classify_floor(&input_gen),
            Some(FloorCategory::SecretsWrite)
        );

        let non_secrets = ["src/env.rs", "docs/environment.md"];
        for path in non_secrets {
            let args = json!({"file_path": path});
            let input = FloorInput::tool("vox_apply_structured_edit", &args, root);
            assert_eq!(classify_floor(&input), None, "non-secret path {path}");
        }
    }

    #[test]
    fn safety_floor_secrets_config_keys() {
        let root = Path::new("/tmp/test-repo");
        let key_args = json!({"openai_api_key": "x"});
        let input1 = FloorInput::tool("vox_config_set", &key_args, root);
        assert_eq!(classify_floor(&input1), Some(FloorCategory::SecretsWrite));

        let nested_args = json!({"nested": {"db_password": "x"}});
        let input2 = FloorInput::tool("vox_config_set", &nested_args, root);
        assert_eq!(classify_floor(&input2), Some(FloorCategory::SecretsWrite));
    }

    #[test]
    fn safety_floor_task_input() {
        let secret_paths = [PathBuf::from(".env")];
        let empty_hints: [String; 0] = [];
        let task_secrets = FloorInput::task(&secret_paths, &empty_hints);
        assert_eq!(
            classify_floor(&task_secrets),
            Some(FloorCategory::SecretsWrite)
        );

        let empty_paths: [PathBuf; 0] = [];
        let ext_hints = ["vox_deploy".to_string()];
        let task_ext = FloorInput::task(&empty_paths, &ext_hints);
        assert_eq!(
            classify_floor(&task_ext),
            Some(FloorCategory::ExternalEffect)
        );

        let none_paths = [PathBuf::from("src/lib.rs")];
        let none_hints = ["vox_read_file".to_string()];
        let task_none = FloorInput::task(&none_paths, &none_hints);
        assert_eq!(classify_floor(&task_none), None);
    }

    #[test]
    fn safety_floor_tables_have_no_overlap_and_ids_are_unique() {
        let mut all_ids = std::collections::HashSet::new();
        for &cat in &ALL {
            assert!(all_ids.insert(cat.id()), "duplicate id: {}", cat.id());
        }

        let mut all_tools = std::collections::HashSet::new();
        for &t in HISTORY_REWRITE_TOOLS {
            assert!(all_tools.insert(t), "overlap for {t}");
        }
        for &t in POLICY_GRANT_TOOLS {
            assert!(all_tools.insert(t), "overlap for {t}");
        }
        for &t in EXTERNAL_EFFECT_TOOLS {
            assert!(all_tools.insert(t), "overlap for {t}");
        }
    }

    #[test]
    fn safety_floor_tools_for_and_arg_rule_ids_cover_every_category() {
        for &cat in &ALL {
            let _ = tools_for(cat);
            let rules = arg_rule_ids(cat);
            match cat {
                FloorCategory::HistoryRewrite => {
                    assert_eq!(rules, &["shell_command_history_rewrite"])
                }
                FloorCategory::PolicyGrant => assert_eq!(rules, &["policy_file_write"]),
                FloorCategory::ExternalEffect => assert!(rules.is_empty()),
                FloorCategory::SecretsWrite => {
                    assert_eq!(rules, &["secrets_path_write", "secrets_config_key"])
                }
            }
        }
    }
}
