//! Replayable dynamic quest engine.
//!
//! Generates quests from:
//! 1. Workspace code issues (TODO/FIXME scan) — primary source
//! 2. Rotating archetype templates — ensures variety per session
//! 3. AI-assisted flavor text and hints (optional, free-tier)
//!
//! Anti-repetition: tracks recently completed quest types in a session ring buffer.
//! Quest rotation: cycles through 4 archetypes (Centurion/Architectus/Scriba/Legatus).
//! Daily cap: max 10 active quests per user at a time.

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::bounded_fs::read_utf8_path_capped;
use crate::quest::{Quest, QuestModifier, QuestType};
use crate::util::now_unix;

// ── Constants ────────────────────────────────────────────

const MAX_ACTIVE_QUESTS: usize = 10;
const QUEST_EXPIRY_SECS: i64 = 86_400; // 24 h

// ── Code scan ────────────────────────────────────────────

/// A code issue found by scanning the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    /// Category of code issue.
    pub kind: CodeIssueKind,
    /// Path to the file containing the issue.
    pub file_path: PathBuf,
    /// Line number where the issue was found.
    pub line: u32,
    /// Surrounding source text context.
    pub text: String,
}

/// Category of a code quality issue detected during workspace scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeIssueKind {
    /// `TODO` comment indicating unfinished work.
    Todo,
    /// `FIXME` comment indicating a known bug.
    Fixme,
    /// `HACK` comment indicating a workaround.
    Hack,
    /// `unsafe` block requiring review.
    Unsafe,
    /// `#[deprecated]` attribute on a public item.
    Deprecated,
}

impl CodeIssueKind {
    /// Return the source text tag associated with this issue kind.
    pub fn tag(&self) -> &'static str {
        match self {
            CodeIssueKind::Todo => "TODO",
            CodeIssueKind::Fixme => "FIXME",
            CodeIssueKind::Hack => "HACK",
            CodeIssueKind::Unsafe => "unsafe",
            CodeIssueKind::Deprecated => "#[deprecated]",
        }
    }
}

/// Scan a workspace directory for code issues.
///
/// Limits to the first 200 results to avoid blocking the event loop.
pub fn scan_workspace(root: &std::path::Path) -> Vec<CodeIssue> {
    let patterns = [
        ("TODO", CodeIssueKind::Todo),
        ("FIXME", CodeIssueKind::Fixme),
        ("HACK", CodeIssueKind::Hack),
    ];

    let mut issues = Vec::new();
    let extensions = ["rs", "vox", "ts", "tsx", "js"];

    let walker = walkdir(root, &extensions);
    'outer: for path in walker {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for (line_no, line) in content.lines().enumerate() {
                for (tag, kind) in &patterns {
                    if line.contains(tag) {
                        issues.push(CodeIssue {
                            kind: *kind,
                            file_path: path.clone(),
                            line: line_no as u32 + 1,
                            text: line.trim().to_string(),
                        });
                        if issues.len() >= 200 {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    issues
}

fn walkdir(root: &std::path::Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths(root, extensions, &mut paths, 0);
    paths
}

fn collect_paths(dir: &std::path::Path, exts: &[&str], out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !matches!(name, "target" | "node_modules" | ".git" | ".cursor") {
                collect_paths(&path, exts, out, depth + 1);
            }
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && exts.contains(&ext)
        {
            out.push(path);
        }
    }
}

// ── Quest generation ──────────────────────────────────────

/// Archetype rotation: controls which template families rotate per user per day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestArchetype {
    /// Combat/battle-focused quests for fixing bugs.
    Centurion,
    /// Refactor/improve-focused quests for code quality.
    Architectus,
    /// Documentation/review-focused quests.
    Scriba,
    /// Collaboration/handoff-focused quests.
    Legatus,
}

impl QuestArchetype {
    /// All archetypes in rotation order.
    pub const ALL: &'static [QuestArchetype] = &[
        QuestArchetype::Centurion,
        QuestArchetype::Architectus,
        QuestArchetype::Scriba,
        QuestArchetype::Legatus,
    ];

    /// Derive today's archetype from the date and user_id hash (deterministic, daily rotation).
    pub fn today_for_user(user_id: &str) -> Self {
        use std::hash::{Hash, Hasher};
        let today = crate::util::now_unix() / QUEST_EXPIRY_SECS;
        let mut h = twox_hash::XxHash64::default();
        today.hash(&mut h);
        user_id.hash(&mut h);
        let idx = (h.finish() as usize) % Self::ALL.len();
        Self::ALL[idx]
    }

    /// Return the primary [`QuestType`] for this archetype.
    pub fn primary_quest_type(&self) -> QuestType {
        match self {
            QuestArchetype::Centurion => QuestType::Battle,
            QuestArchetype::Architectus => QuestType::Improve,
            QuestArchetype::Scriba => QuestType::Review,
            QuestArchetype::Legatus => QuestType::Collaborate,
        }
    }

    /// Return the Roman-style display title for this archetype.
    pub fn roman_title(&self) -> &'static str {
        match self {
            QuestArchetype::Centurion => "Centurio",
            QuestArchetype::Architectus => "Architectus",
            QuestArchetype::Scriba => "Scriba",
            QuestArchetype::Legatus => "Legatus",
        }
    }
}

/// Dynamic quest generated from a code issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicQuest {
    /// Core quest metadata (XP, crystals, progress, expiry).
    pub quest: Quest,
    /// Source issue that triggered this quest.
    pub source_issue: Option<CodeIssue>,
    /// Optional AI-generated flavor text.
    pub flavor_text: Option<String>,
    /// Hint for how to resolve the issue.
    pub hint: Option<String>,
}

/// Generate dynamic quests from workspace issues.
///
/// - Scans `workspace_root` for TODOs/FIXMEs.
/// - Generates up to `max_quests` quests, preferring unresolved issues.
/// - Mixes in archetype-based template quests if not enough issues.
pub fn generate_dynamic_quests(
    user_id: &str,
    workspace_root: &std::path::Path,
    existing_count: usize,
    max_new: usize,
) -> Vec<DynamicQuest> {
    if existing_count >= MAX_ACTIVE_QUESTS {
        return Vec::new();
    }

    let budget = (MAX_ACTIVE_QUESTS - existing_count).min(max_new);
    let issues = scan_workspace(workspace_root);

    let mut rng = rand::thread_rng();
    let archetype = QuestArchetype::today_for_user(user_id);
    let mut quests = Vec::new();

    // Issue-sourced quests (shuffle for variety)
    let mut issue_sample: Vec<&CodeIssue> = issues.iter().collect();
    issue_sample.shuffle(&mut rng);
    for issue in issue_sample.iter().take(budget.saturating_sub(1)) {
        let quest = quest_from_issue(user_id, issue);
        quests.push(DynamicQuest {
            quest,
            source_issue: Some((*issue).clone()),
            flavor_text: None,
            hint: deterministic_hint_for_issue(issue),
        });
        if quests.len() >= budget {
            break;
        }
    }

    // Archetype template quest if we still have budget
    if quests.len() < budget {
        quests.push(DynamicQuest {
            quest: archetype_quest(user_id, archetype),
            source_issue: None,
            flavor_text: None,
            hint: None,
        });
    }

    quests
}

/// Verify a dynamic quest: re-check whether the source issue is resolved.
///
/// Returns `true` if the issue is resolved (i.e., the quest should be completed).
/// For quests without a source issue, always returns `false` (manual verification required).
pub fn verify_quest(dq: &DynamicQuest) -> bool {
    let Some(issue) = &dq.source_issue else {
        return false;
    };

    // Re-read the file and check if the tag still appears on that line
    let Ok(content) = read_utf8_path_capped(&issue.file_path) else {
        // File deleted → consider resolved
        return true;
    };

    let line = content.lines().nth((issue.line as usize).saturating_sub(1));
    match line {
        Some(l) => !l.contains(issue.kind.tag()),
        None => true, // line no longer exists → resolved
    }
}

fn quest_from_issue(user_id: &str, issue: &CodeIssue) -> Quest {
    let expires_at = now_unix() + QUEST_EXPIRY_SECS * 3; // 3-day window for fix quests
    let id = format!(
        "dq-{}-{}-{}",
        issue.kind.tag().to_lowercase(),
        issue
            .file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown"),
        issue.line
    );

    let description = format!(
        "{} on line {} in {} | {} | {}",
        issue.kind.tag(),
        issue.line,
        issue.file_path.display(),
        issue.file_path.display(),
        issue.line,
    );

    let (xp, crystals, quest_type) = match issue.kind {
        CodeIssueKind::Todo => (50, 10, QuestType::Improve),
        CodeIssueKind::Fixme => (100, 20, QuestType::Battle),
        CodeIssueKind::Hack => (150, 30, QuestType::Improve),
        CodeIssueKind::Unsafe => (300, 60, QuestType::Battle),
        CodeIssueKind::Deprecated => (30, 5, QuestType::Review),
    };

    Quest {
        id,
        user_id: user_id.to_string(),
        quest_type,
        description,
        hint: deterministic_hint_for_issue(issue).unwrap_or_default(),
        target: 1,
        progress: 0,
        crystal_reward: crystals as u64,
        xp_reward: xp as u64,
        modifier: QuestModifier::None,
        completed: false,
        status: "active".into(),
        expires_at,
    }
}

fn archetype_quest(user_id: &str, archetype: QuestArchetype) -> Quest {
    let expires_at = now_unix() + QUEST_EXPIRY_SECS;
    let id = format!(
        "arch-{}-{}",
        archetype.roman_title().to_lowercase(),
        now_unix() / QUEST_EXPIRY_SECS
    );

    let (description, target, xp, crystals) = match archetype {
        QuestArchetype::Centurion => (
            "Win 2 bug battles to prove your valor, Centurio.".to_string(),
            2,
            100,
            20,
        ),
        QuestArchetype::Architectus => (
            "Refactor or improve 1 component: reduce complexity or add missing tests.".to_string(),
            1,
            120,
            25,
        ),
        QuestArchetype::Scriba => (
            "Review 2 files and add missing `///` doc comments.".to_string(),
            2,
            80,
            15,
        ),
        QuestArchetype::Legatus => (
            "Complete 1 successful agent handoff or collaboration task.".to_string(),
            1,
            110,
            22,
        ),
    };

    Quest {
        id,
        user_id: user_id.to_string(),
        quest_type: archetype.primary_quest_type(),
        description,
        hint: String::new(),
        target,
        progress: 0,
        crystal_reward: crystals as u64,
        xp_reward: xp as u64,
        modifier: QuestModifier::None,
        completed: false,
        status: "active".into(),
        expires_at,
    }
}

fn deterministic_hint_for_issue(issue: &CodeIssue) -> Option<String> {
    match issue.kind {
        CodeIssueKind::Todo => Some(format!(
            "Resolve the TODO at {}:{} to earn XP. Use `vox gamify quest-generate` to track progress.",
            issue.file_path.display(),
            issue.line
        )),
        CodeIssueKind::Fixme => Some(format!(
            "FIXME at {}:{}: Fix this bug to earn bonus crystals and improve code quality.",
            issue.file_path.display(),
            issue.line
        )),
        CodeIssueKind::Hack => Some(format!(
            "HACK at {}:{}: Refactor this to earn 'Architectus' reputation.",
            issue.file_path.display(),
            issue.line
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn archetype_is_deterministic_for_same_user_day() {
        let a = QuestArchetype::today_for_user("user-test");
        let b = QuestArchetype::today_for_user("user-test");
        assert_eq!(a, b);
    }

    #[test]
    fn different_users_may_differ() {
        // Not guaranteed to differ but shouldn't panic
        let _a = QuestArchetype::today_for_user("alice");
        let _b = QuestArchetype::today_for_user("bob");
    }

    #[test]
    fn scan_workspace_finds_todos() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let f = tmp.path().join("test.rs");
        let mut file = std::fs::File::create(&f).unwrap();
        writeln!(file, "// TODO: fix this later").unwrap();
        writeln!(file, "fn foo() {{}}").unwrap();

        let issues = scan_workspace(tmp.path());
        assert!(issues.iter().any(|i| i.kind == CodeIssueKind::Todo));
    }

    #[test]
    fn verify_quest_resolves_when_todo_gone() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let f = tmp.path().join("verify.rs");

        // Write file without TODO
        std::fs::write(&f, "fn foo() {}").unwrap();

        let dq = DynamicQuest {
            quest: Quest {
                id: "test".to_string(),
                user_id: "u".to_string(),
                quest_type: QuestType::Improve,
                description: "test".to_string(),
                hint: String::new(),
                target: 1,
                progress: 0,
                crystal_reward: 5,
                xp_reward: 10,
                modifier: QuestModifier::None,
                completed: false,
                status: "active".into(),
                expires_at: 0,
            },
            source_issue: Some(CodeIssue {
                kind: CodeIssueKind::Todo,
                file_path: f.clone(),
                line: 1,
                text: "fn foo() {}".to_string(),
            }),
            flavor_text: None,
            hint: None,
        };

        assert!(verify_quest(&dq));
    }
}
