//! Dynamic system prompt runtime variable interpolation (`T3-RULES-SEARCH-02`, `CONT-04`).

use serde::{Deserialize, Serialize};
pub use vox_orchestrator::context_envelope::OperatingMode;

/// Runtime context values interpolated into system prompts (`T3-RULES-SEARCH-02`, `T3-PROMPT-LOOP-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptRuntimeVars {
    pub current_datetime: String,
    pub user_timezone: String,
    pub user_locale: String,
    pub user_language: String,
    pub user_platform: String,
    pub workspace_root: String,
    pub git_branch: String,
    pub active_spoke: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub active_file: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub cognitive_profile: String,
}

impl Default for SystemPromptRuntimeVars {
    fn default() -> Self {
        Self {
            current_datetime: "2026-10-25T00:00:00Z".to_string(),
            user_timezone: "UTC".to_string(),
            user_locale: "en-US".to_string(),
            user_language: "en".to_string(),
            user_platform: std::env::consts::OS.to_string(),
            workspace_root: ".".to_string(),
            git_branch: "main".to_string(),
            active_spoke: "root".to_string(),
            session_id: "default".to_string(),
            active_file: String::new(),
            model_id: "auto".to_string(),
            cognitive_profile: "standard".to_string(),
        }
    }
}

/// Structured per-turn volatile context appended after the day-stable system prompt prefix (`CONT-04`).
#[allow(dead_code)] // Retained for CONT-04 volatile turn context staging.
#[derive(Debug, Clone)]
pub(crate) struct VolatileTurnContext<'a> {
    pub attention_budget: String,
    pub operating_mode: Option<&'a OperatingMode>,
    pub vars: &'a SystemPromptRuntimeVars,
}

#[allow(dead_code)] // Retained for CONT-04 volatile turn context staging.
impl<'a> VolatileTurnContext<'a> {
    #[must_use]
    pub(crate) fn render(&self) -> String {
        build_volatile_turn_system_rider(&self.attention_budget, self.operating_mode, self.vars)
    }
}

fn normalize_budget_signal(raw_debug: String) -> String {
    let trimmed = raw_debug.trim().trim_matches('"');
    if let Some(inner) = trimmed
        .strip_prefix("Some(")
        .and_then(|s| s.strip_suffix(')'))
    {
        inner.trim().trim_matches('"').to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build the per-turn volatile system rider (`## Turn Runtime Context`) containing
/// `attention_budget`, `operating_mode`, and per-turn runtime variables (`active_file`,
/// `session_id`, `model_id`, `cognitive_profile`, `current_datetime`) (`CONT-04`).
#[must_use]
pub(crate) fn build_volatile_turn_system_rider(
    attention_budget: impl std::fmt::Debug,
    operating_mode: Option<&OperatingMode>,
    vars: &SystemPromptRuntimeVars,
) -> String {
    let signal = normalize_budget_signal(format!("{attention_budget:?}"));
    let mut out = String::from("\n\n## Turn Runtime Context\n");
    if !signal.is_empty() && signal != "None" && signal != "()" {
        out.push_str(&format!(
            "Attention Budget Signal: {signal}\nIf the budget is 'HighLoad' or 'Critical', you MUST summarize and abort your workflow immediately to defer to the operator.\n"
        ));
    }
    if !vars.session_id.is_empty() {
        out.push_str(&format!("Session ID: {}\n", vars.session_id));
    }
    if !vars.active_file.is_empty() {
        out.push_str(&format!("Active File: {}\n", vars.active_file));
    }
    if !vars.active_spoke.is_empty() {
        out.push_str(&format!("Active Spoke: {}\n", vars.active_spoke));
    }
    if !vars.model_id.is_empty() {
        out.push_str(&format!("Model ID: {}\n", vars.model_id));
    }
    if !vars.cognitive_profile.is_empty() {
        out.push_str(&format!("Cognitive Profile: {}\n", vars.cognitive_profile));
    }
    if !vars.current_datetime.is_empty() {
        out.push_str(&format!("Turn Timestamp: {}\n", vars.current_datetime));
    }
    if let Some(mode) = operating_mode {
        out.push_str(&mode.system_rider());
    }
    out
}

/// Resolve the active session's [`OperatingMode`] from `ServerState`'s context store, if present.
#[must_use]
pub(crate) fn resolve_session_operating_mode(
    state: &crate::server_state::ServerState,
    session_id: Option<&str>,
) -> Option<OperatingMode> {
    let sid = session_id?;
    let env_key = vox_orchestrator::socrates::session_context_envelope_key(sid);
    let store = state.orchestrator.context_store();
    let env_raw = crate::sync_poison::poison_rw_read(store.read(), "context_store")
        .ok()
        .and_then(|g| g.get(&env_key).clone())?;
    let env = serde_json::from_str::<vox_orchestrator::ContextEnvelope>(&env_raw).ok()?;
    env.operating_mode
}

/// Detect active Git branch from `<workspace_root>/.git/HEAD` (or linked worktree `.git` file `gitdir: ...`)
/// without spawning a subprocess (`T3-SALVAGE-RULES-03`, `T3-PROMPT-LOOP-01`).
#[must_use]
pub fn detect_workspace_git_branch(workspace_root: &std::path::Path) -> String {
    let dot_git = workspace_root.join(".git");
    let head_path = if dot_git.is_file() {
        if let Ok(pointer) = std::fs::read_to_string(&dot_git) {
            if let Some(gitdir_raw) = pointer.trim().strip_prefix("gitdir:") {
                let gitdir_trimmed = gitdir_raw.trim();
                let gitdir_path = std::path::Path::new(gitdir_trimmed);
                let resolved_gitdir = if gitdir_path.is_absolute() {
                    gitdir_path.to_path_buf()
                } else {
                    workspace_root.join(gitdir_path)
                };
                resolved_gitdir.join("HEAD")
            } else {
                dot_git.join("HEAD")
            }
        } else {
            dot_git.join("HEAD")
        }
    } else {
        dot_git.join("HEAD")
    };
    if let Ok(raw) = std::fs::read_to_string(head_path) {
        let trimmed = raw.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            return branch.trim().to_string();
        }
        if trimmed.len() >= 7 {
            return trimmed[..7].to_string();
        }
    }
    "main".to_string()
}

/// Construct live runtime variables from workspace root, date string, and active file (`T3-SALVAGE-RULES-03`, `T3-PROMPT-LOOP-01`).
#[must_use]
pub fn build_live_runtime_vars(
    workspace_root: &std::path::Path,
    date_str: &str,
    active_file: Option<&str>,
) -> SystemPromptRuntimeVars {
    let git_branch = detect_workspace_git_branch(workspace_root);
    let active_spoke = active_file
        .and_then(|f| {
            let norm = f.replace('\\', "/");
            if let Some(rest) = norm.strip_prefix("crates/") {
                rest.split('/').next().map(str::to_string)
            } else {
                norm.split('/').next().map(str::to_string)
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "root".to_string());
    let iso_datetime = if date_str.len() == 10 && !date_str.contains('T') {
        format!("{date_str}T00:00:00Z")
    } else {
        date_str.to_string()
    };
    SystemPromptRuntimeVars {
        current_datetime: iso_datetime,
        user_timezone: std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string()),
        user_locale: "en-US".to_string(),
        user_language: "en".to_string(),
        user_platform: std::env::consts::OS.to_string(),
        workspace_root: workspace_root.display().to_string(),
        git_branch,
        active_spoke,
        session_id: "default".to_string(),
        active_file: active_file.unwrap_or_default().to_string(),
        model_id: "auto".to_string(),
        cognitive_profile: "standard".to_string(),
    }
}

/// Interpolates only day-stable placeholders (`user_timezone`, `user_locale`, `user_language`,
/// `user_platform`, `workspace_root`, `git_branch`, `active_spoke`, and day-level `date_str`)
/// while neutralizing per-turn volatile tokens (`active_file`, `session_id`, `model_id`,
/// `cognitive_profile`, `current_datetime`) so the static system prefix remains `>= 0.99`
/// byte-stable across turns (`CONT-04`).
#[must_use]
pub fn interpolate_static_system_prompt_variables(
    template: &str,
    vars: &SystemPromptRuntimeVars,
) -> String {
    let day_only = vars
        .current_datetime
        .split('T')
        .next()
        .unwrap_or(&vars.current_datetime);
    let day_iso = format!("{day_only}T00:00:00Z");
    template
        .replace("$current_datetime", &day_iso)
        .replace("{{current_datetime}}", &day_iso)
        .replace("$user_timezone", &vars.user_timezone)
        .replace("{{user_timezone}}", &vars.user_timezone)
        .replace("$user_locale", &vars.user_locale)
        .replace("{{user_locale}}", &vars.user_locale)
        .replace("$user_language", &vars.user_language)
        .replace("{{user_language}}", &vars.user_language)
        .replace("$user_platform", &vars.user_platform)
        .replace("{{user_platform}}", &vars.user_platform)
        .replace("$workspace_root", &vars.workspace_root)
        .replace("{{workspace_root}}", &vars.workspace_root)
        .replace("$git_branch", &vars.git_branch)
        .replace("{{git_branch}}", &vars.git_branch)
        .replace("$active_spoke", "root")
        .replace("{{active_spoke}}", "root")
        .replace("$session_id", "default")
        .replace("{{session_id}}", "default")
        .replace("$active_file", "")
        .replace("{{active_file}}", "")
        .replace("$model_id", "auto")
        .replace("{{model_id}}", "auto")
        .replace("$cognitive_profile", "standard")
        .replace("{{cognitive_profile}}", "standard")
}

/// Interpolates volatile per-turn tokens (`active_file`, `session_id`, `model_id`,
/// `cognitive_profile`, `current_datetime`, `active_spoke`) in a volatile turn rider template.
#[must_use]
pub fn interpolate_volatile_tokens(template: &str, vars: &SystemPromptRuntimeVars) -> String {
    template
        .replace("$current_datetime", &vars.current_datetime)
        .replace("{{current_datetime}}", &vars.current_datetime)
        .replace("$active_spoke", &vars.active_spoke)
        .replace("{{active_spoke}}", &vars.active_spoke)
        .replace("$session_id", &vars.session_id)
        .replace("{{session_id}}", &vars.session_id)
        .replace("$active_file", &vars.active_file)
        .replace("{{active_file}}", &vars.active_file)
        .replace("$model_id", &vars.model_id)
        .replace("{{model_id}}", &vars.model_id)
        .replace("$cognitive_profile", &vars.cognitive_profile)
        .replace("{{cognitive_profile}}", &vars.cognitive_profile)
}

/// Replaces `$current_datetime`, `$user_timezone`, `$user_locale`, `$user_language`,
/// `$user_platform`, `$workspace_root`, `$git_branch`, `$active_spoke`, `$session_id`,
/// `$active_file`, `$model_id`, and `$cognitive_profile` in `template` (`T3-RULES-SEARCH-02`, `T3-PROMPT-LOOP-01`).
#[must_use]
pub fn interpolate_system_prompt_variables(
    template: &str,
    vars: &SystemPromptRuntimeVars,
) -> String {
    template
        .replace("$current_datetime", &vars.current_datetime)
        .replace("$user_timezone", &vars.user_timezone)
        .replace("$user_locale", &vars.user_locale)
        .replace("$user_language", &vars.user_language)
        .replace("$user_platform", &vars.user_platform)
        .replace("$workspace_root", &vars.workspace_root)
        .replace("$git_branch", &vars.git_branch)
        .replace("$active_spoke", &vars.active_spoke)
        .replace("$session_id", &vars.session_id)
        .replace("$active_file", &vars.active_file)
        .replace("$model_id", &vars.model_id)
        .replace("$cognitive_profile", &vars.cognitive_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_all_8_runtime_variables() {
        let vars = SystemPromptRuntimeVars {
            current_datetime: "2026-10-25T18:00:00Z".into(),
            user_timezone: "America/Los_Angeles".into(),
            user_locale: "en-US".into(),
            user_language: "English".into(),
            user_platform: "macos".into(),
            workspace_root: "/repo/vox".into(),
            git_branch: "feat/t3".into(),
            active_spoke: "vox-orchestrator".into(),
            session_id: "sess-42".into(),
            active_file: "crates/vox-orchestrator/src/lib.rs".into(),
            model_id: "claude-sonnet-4-6".into(),
            cognitive_profile: "reasoning".into(),
        };
        let rendered = interpolate_system_prompt_variables(
            "Now=$current_datetime TZ=$user_timezone Loc=$user_locale Lang=$user_language OS=$user_platform Root=$workspace_root Branch=$git_branch Spoke=$active_spoke Sess=$session_id File=$active_file Model=$model_id Profile=$cognitive_profile",
            &vars,
        );
        assert_eq!(
            rendered,
            "Now=2026-10-25T18:00:00Z TZ=America/Los_Angeles Loc=en-US Lang=English OS=macos Root=/repo/vox Branch=feat/t3 Spoke=vox-orchestrator Sess=sess-42 File=crates/vox-orchestrator/src/lib.rs Model=claude-sonnet-4-6 Profile=reasoning"
        );
    }

    #[test]
    fn detects_git_branch_through_worktree_gitdir_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let worktree_git_dir = tmp.path().join("actual_git_worktree");
        std::fs::create_dir_all(&worktree_git_dir).expect("mkdir");
        std::fs::write(
            worktree_git_dir.join("HEAD"),
            "ref: refs/heads/feat/worktree-branch\n",
        )
        .expect("write HEAD");
        let wt_root = tmp.path().join("linked_wt");
        std::fs::create_dir_all(&wt_root).expect("mkdir wt");
        std::fs::write(
            wt_root.join(".git"),
            format!("gitdir: {}\n", worktree_git_dir.display()),
        )
        .expect("write .git file");
        assert_eq!(
            detect_workspace_git_branch(&wt_root),
            "feat/worktree-branch"
        );
    }

    #[test]
    fn build_volatile_turn_system_rider_includes_budget_mode_and_active_file() {
        let vars = SystemPromptRuntimeVars {
            current_datetime: "2026-11-15T12:00:00Z".into(),
            active_spoke: "vox-orchestrator-mcp".into(),
            session_id: "sess-kv-448".into(),
            active_file: "crates/vox-orchestrator-mcp/src/chat_tools/mod.rs".into(),
            model_id: "claude-sonnet-4-6".into(),
            cognitive_profile: "reasoning".into(),
            ..SystemPromptRuntimeVars::default()
        };
        let mode = OperatingMode::Verification {
            reason: Some("verify citation grounding".into()),
        };
        let rider = build_volatile_turn_system_rider("HighLoad", Some(&mode), &vars);
        assert!(rider.contains("## Turn Runtime Context"));
        assert!(rider.contains("Attention Budget Signal: HighLoad"));
        assert!(rider.contains("If the budget is 'HighLoad' or 'Critical'"));
        assert!(rider.contains("Active File: crates/vox-orchestrator-mcp/src/chat_tools/mod.rs"));
        assert!(rider.contains("Session ID: sess-kv-448"));
        assert!(rider.contains("Model ID: claude-sonnet-4-6"));
        assert!(rider.contains("Cognitive Profile: reasoning"));
        assert!(rider.contains(&mode.system_rider()));
    }

    #[test]
    fn static_interpolation_keeps_prefix_day_stable_across_volatile_vars() {
        let template = "Root=$workspace_root Branch=$git_branch OS=$user_platform Date=$current_datetime File=$active_file {{active_file}} Sess=$session_id Model=$model_id Profile=$cognitive_profile";
        let base_vars = SystemPromptRuntimeVars {
            current_datetime: "2026-11-15T01:23:45Z".into(),
            user_platform: "linux".into(),
            workspace_root: "/workspace/vox".into(),
            git_branch: "main".into(),
            active_spoke: "vox-scientia".into(),
            session_id: "sess-1".into(),
            active_file: "crates/vox-scientia/src/lib.rs".into(),
            model_id: "model-a".into(),
            cognitive_profile: "fast".into(),
            ..SystemPromptRuntimeVars::default()
        };
        let static_prefix = interpolate_static_system_prompt_variables(template, &base_vars);
        let turn2_vars = SystemPromptRuntimeVars {
            current_datetime: "2026-11-15T22:59:59Z".into(),
            active_spoke: "vox-gui".into(),
            session_id: "sess-2".into(),
            active_file: "crates/vox-gui/ui/src/App.tsx".into(),
            model_id: "model-b".into(),
            cognitive_profile: "reasoning".into(),
            ..base_vars.clone()
        };
        let reinterpolated = interpolate_system_prompt_variables(&static_prefix, &turn2_vars);
        assert_eq!(static_prefix, reinterpolated);
        assert!(!static_prefix.contains("crates/vox-scientia/src/lib.rs"));
        assert!(!reinterpolated.contains("crates/vox-gui/ui/src/App.tsx"));
    }
}
