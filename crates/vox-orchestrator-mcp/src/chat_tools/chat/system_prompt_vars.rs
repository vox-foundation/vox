//! Dynamic system prompt runtime variable interpolation (`T3-RULES-SEARCH-02`).

use serde::{Deserialize, Serialize};

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
}
