//! Achievement system for gamifying agent activities.
//!
//! Tracks milestones like first task completion, first handoff,
//! error-free streaks, and cost efficiency. Achievements are
//! persisted and shown on the dashboard.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique achievement identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AchievementId(pub String);

impl std::fmt::Display for AchievementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An achievement that can be unlocked by agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier.
    pub id: AchievementId,
    /// Display name.
    pub name: String,
    /// Description of how to unlock.
    pub description: String,
    /// Emoji icon.
    pub icon: String,
    /// Category of achievement.
    pub category: AchievementCategory,
    /// XP reward for unlocking.
    pub xp_reward: u32,
    /// Crystal reward for unlocking.
    pub crystal_reward: u32,
}

/// Achievement categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementCategory {
    /// Task-related milestones.
    Tasks,
    /// Collaboration milestones.
    Collaboration,
    /// Efficiency milestones.
    Efficiency,
    /// Exploration milestones.
    Discovery,
    /// Streak milestones.
    Streaks,
    /// AI corpus and feedback milestones.
    AiCorpus,
    /// Build, test, and code quality milestones.
    BuildMastery,
    /// Vox language feature milestones.
    LangExplorer,
    /// Security and safety milestones.
    Security,
    /// Documentation milestones.
    Documentation,
}

/// Record of an unlocked achievement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockedAchievement {
    /// The identifier of the achievement that was unlocked.
    pub achievement_id: AchievementId,
    /// Identifier of the agent that earned the achievement.
    pub agent_id: String,
    /// Unix timestamp when the achievement was unlocked.
    pub unlocked_at: u64,
}

/// Tracks achievements per agent.
#[derive(Debug, Default)]
pub struct AchievementTracker {
    /// All available achievements.
    definitions: Vec<Achievement>,
    /// Per-agent unlocked achievements.
    unlocked: HashMap<String, Vec<UnlockedAchievement>>,
    /// Per-agent counters for tracking progress.
    counters: HashMap<String, HashMap<String, u32>>,
}

impl AchievementTracker {
    /// Create a new tracker with the default achievement set.
    pub fn new() -> Self {
        let mut tracker = Self::default();
        tracker.register_defaults();
        tracker
    }

    /// Register the default set of achievements.
    pub fn register_defaults(&mut self) {
        let defaults = vec![
            // ── Tasks ─────────────────────────────────────────────
            Achievement {
                id: AchievementId("first_task".into()),
                name: "Hello World".into(),
                description: "Complete your first task".into(),
                icon: "🎯".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 50,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("five_tasks".into()),
                name: "Getting Started".into(),
                description: "Complete 5 tasks".into(),
                icon: "⭐".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 100,
                crystal_reward: 25,
            },
            Achievement {
                id: AchievementId("twenty_five_tasks".into()),
                name: "Workhorse".into(),
                description: "Complete 25 tasks".into(),
                icon: "🏆".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 500,
                crystal_reward: 100,
            },
            Achievement {
                id: AchievementId("hundred_tasks".into()),
                name: "Centurion's Burden".into(),
                description: "Complete 100 tasks".into(),
                icon: "🦅".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 1_500,
                crystal_reward: 300,
            },
            Achievement {
                id: AchievementId("five_hundred_tasks".into()),
                name: "Legion Commander".into(),
                description: "Complete 500 tasks".into(),
                icon: "👑".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 5_000,
                crystal_reward: 1_000,
            },
            Achievement {
                id: AchievementId("task_in_a_day".into()),
                name: "Dawn to Dusk".into(),
                description: "Complete 10 tasks in a single calendar day".into(),
                icon: "☀️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 200,
                crystal_reward: 50,
            },
            // ── Collaboration ─────────────────────────────────────
            Achievement {
                id: AchievementId("first_handoff".into()),
                name: "Team Player".into(),
                description: "Successfully hand off a plan to another agent".into(),
                icon: "🤝".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 75,
                crystal_reward: 15,
            },
            Achievement {
                id: AchievementId("ten_handoffs".into()),
                name: "Legatus".into(),
                description: "Complete 10 agent handoffs".into(),
                icon: "📋".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 400,
                crystal_reward: 80,
            },
            Achievement {
                id: AchievementId("fifty_handoffs".into()),
                name: "Grand Coordinator".into(),
                description: "Complete 50 agent handoffs".into(),
                icon: "🌐".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 1_500,
                crystal_reward: 300,
            },
            Achievement {
                id: AchievementId("conflict_resolver".into()),
                name: "Peacemaker".into(),
                description: "Resolve a VCS conflict".into(),
                icon: "🕊️".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 100,
                crystal_reward: 20,
            },
            // ── Streaks ───────────────────────────────────────────
            Achievement {
                id: AchievementId("streak_3".into()),
                name: "Three-Day Tribune".into(),
                description: "Maintain a 3-day activity streak".into(),
                icon: "🔥".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 80,
                crystal_reward: 16,
            },
            Achievement {
                id: AchievementId("streak_7".into()),
                name: "Week Warrior".into(),
                description: "Maintain a 7-day activity streak".into(),
                icon: "🔥".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 300,
                crystal_reward: 75,
            },
            Achievement {
                id: AchievementId("streak_14".into()),
                name: "Fortnight Forger".into(),
                description: "Maintain a 14-day activity streak".into(),
                icon: "🌙".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 700,
                crystal_reward: 140,
            },
            Achievement {
                id: AchievementId("streak_30".into()),
                name: "Month Master".into(),
                description: "Maintain a 30-day activity streak".into(),
                icon: "🌕".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 1_000,
                crystal_reward: 250,
            },
            Achievement {
                id: AchievementId("streak_90".into()),
                name: "Quarter Champion".into(),
                description: "Maintain a 90-day activity streak".into(),
                icon: "💫".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 3_000,
                crystal_reward: 600,
            },
            Achievement {
                id: AchievementId("streak_365".into()),
                name: "Eternal Consul".into(),
                description: "Maintain a 365-day activity streak".into(),
                icon: "⚡".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 10_000,
                crystal_reward: 2_000,
            },
            Achievement {
                id: AchievementId("error_free_five".into()),
                name: "Flawless Five".into(),
                description: "Complete 5 tasks in a row without errors".into(),
                icon: "💎".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 200,
                crystal_reward: 50,
            },
            // ── Efficiency ────────────────────────────────────────
            Achievement {
                id: AchievementId("budget_saver".into()),
                name: "Budget Conscious".into(),
                description: "Complete 10 tasks under $0.01 total cost".into(),
                icon: "💰".into(),
                category: AchievementCategory::Efficiency,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("speed_demon".into()),
                name: "Speed Demon".into(),
                description: "Complete a task in under 30 seconds".into(),
                icon: "⚡".into(),
                category: AchievementCategory::Efficiency,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("ultra_speed".into()),
                name: "Lightning Bolt".into(),
                description: "Complete a task in under 10 seconds".into(),
                icon: "⚡⚡".into(),
                category: AchievementCategory::Efficiency,
                xp_reward: 250,
                crystal_reward: 50,
            },
            Achievement {
                id: AchievementId("zero_cost_session".into()),
                name: "Frugal Senator".into(),
                description: "Complete a full session with zero AI cost (local inference only)"
                    .into(),
                icon: "🪙".into(),
                category: AchievementCategory::Efficiency,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("offline_session".into()),
                name: "Self Sufficient".into(),
                description: "Complete a full session using only Ollama or Deterministic providers"
                    .into(),
                icon: "🏕️".into(),
                category: AchievementCategory::Efficiency,
                xp_reward: 300,
                crystal_reward: 75,
            },
            // ── Discovery ─────────────────────────────────────────
            Achievement {
                id: AchievementId("first_continuation".into()),
                name: "Self Starter".into(),
                description: "Receive an auto-continuation prompt".into(),
                icon: "▶️".into(),
                category: AchievementCategory::Discovery,
                xp_reward: 25,
                crystal_reward: 5,
            },
            Achievement {
                id: AchievementId("challenge_solved".into()),
                name: "Challenger".into(),
                description: "Successfully solve a daily coding challenge".into(),
                icon: "🧩".into(),
                category: AchievementCategory::Discovery,
                xp_reward: 200,
                crystal_reward: 50,
            },
            Achievement {
                id: AchievementId("first_memory".into()),
                name: "Elephant Memory".into(),
                description: "Store your first long-term memory entry".into(),
                icon: "🧠".into(),
                category: AchievementCategory::Discovery,
                xp_reward: 25,
                crystal_reward: 5,
            },
            Achievement {
                id: AchievementId("polyglot".into()),
                name: "Polyglot Programmer".into(),
                description: "Work on files in 5 different programming languages".into(),
                icon: "🌍".into(),
                category: AchievementCategory::Discovery,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("polyglot_10".into()),
                name: "Translator".into(),
                description: "Work on files in 10 different programming languages".into(),
                icon: "🌐".into(),
                category: AchievementCategory::Discovery,
                xp_reward: 500,
                crystal_reward: 100,
            },
            // ── AI Corpus ─────────────────────────────────────────
            Achievement {
                id: AchievementId("first_thumbs".into()),
                name: "Voice of the People".into(),
                description: "Give your first AI response feedback".into(),
                icon: "👍".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 25,
                crystal_reward: 5,
            },
            Achievement {
                id: AchievementId("ten_thumbs".into()),
                name: "Trusted Critic".into(),
                description: "Give 10 AI response feedback events".into(),
                icon: "👍".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("hundred_thumbs".into()),
                name: "Corpus Senator".into(),
                description: "Give 100 AI response feedback events".into(),
                icon: "🏛️".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 500,
                crystal_reward: 100,
            },
            Achievement {
                id: AchievementId("first_thumbs_down".into()),
                name: "Sine Timore".into(),
                description: "Give your first negative AI feedback (zero fear!)".into(),
                icon: "👎".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 30,
                crystal_reward: 6,
            },
            Achievement {
                id: AchievementId("balanced_feedback".into()),
                name: "Just Magistrate".into(),
                description: "Give both positive and negative AI feedback".into(),
                icon: "⚖️".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 50,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("first_vox_example".into()),
                name: "The First Tablet".into(),
                description: "Write your first .vox example file".into(),
                icon: "📜".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("five_vox_examples".into()),
                name: "Scroll Keeper".into(),
                description: "Write 5 .vox example files".into(),
                icon: "📚".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("canonical_example".into()),
                name: "Codex Canonicus".into(),
                description: "Have an example promoted to examples/canonical/".into(),
                icon: "✨".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 500,
                crystal_reward: 100,
            },
            Achievement {
                id: AchievementId("first_corpus_contribution".into()),
                name: "Mens Patron".into(),
                description: "Contribute your first training data to the Mens corpus".into(),
                icon: "🧠".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("ten_corpus_contributions".into()),
                name: "Training Senator".into(),
                description: "Contribute 10 training examples to the Mens corpus".into(),
                icon: "🧬".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 600,
                crystal_reward: 120,
            },
            Achievement {
                id: AchievementId("first_finetune".into()),
                name: "Forged in Fire".into(),
                description: "Complete your first QLoRA fine-tuning epoch".into(),
                icon: "🔥".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 800,
                crystal_reward: 160,
            },
            Achievement {
                id: AchievementId("inference_regular".into()),
                name: "Vox Vulgaris".into(),
                description: "Run local Mens inference 50 times".into(),
                icon: "💬".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 200,
                crystal_reward: 40,
            },
            // ── Build Mastery ─────────────────────────────────────
            Achievement {
                id: AchievementId("first_green_build".into()),
                name: "First Light".into(),
                description: "First successful cargo build".into(),
                icon: "🏗️".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 50,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("build_streak_3".into()),
                name: "Triple Green".into(),
                description: "3 consecutive green builds".into(),
                icon: "🟢".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("build_streak_10".into()),
                name: "Unbroken Legion".into(),
                description: "10 consecutive green builds".into(),
                icon: "✅".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 400,
                crystal_reward: 80,
            },
            Achievement {
                id: AchievementId("build_streak_30".into()),
                name: "Iron Discipline".into(),
                description: "30 consecutive green builds".into(),
                icon: "💪".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 1_200,
                crystal_reward: 250,
            },
            Achievement {
                id: AchievementId("first_fix".into()),
                name: "Repaired in Battle".into(),
                description: "Fix a failing build in the same session it broke".into(),
                icon: "🔧".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 75,
                crystal_reward: 15,
            },
            Achievement {
                id: AchievementId("zero_warnings".into()),
                name: "Immaculate Code".into(),
                description: "cargo check with 0 warnings on first attempt".into(),
                icon: "🌟".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("toestub_clean_crate".into()),
                name: "Architectural Purity".into(),
                description: "0 TOESTUB violations in a crate".into(),
                icon: "🏛️".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("toestub_clean_workspace".into()),
                name: "Perfect Forum".into(),
                description: "0 TOESTUB violations across the entire workspace".into(),
                icon: "🏟️".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 2_000,
                crystal_reward: 400,
            },
            // ── Documentation ─────────────────────────────────────
            Achievement {
                id: AchievementId("first_doc".into()),
                name: "Pen to Scroll".into(),
                description: "Add your first /// doc comment to a public item".into(),
                icon: "✏️".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 25,
                crystal_reward: 5,
            },
            Achievement {
                id: AchievementId("fifty_docs".into()),
                name: "Diligent Librarian".into(),
                description: "Add 50 doc comments".into(),
                icon: "📖".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("five_hundred_docs".into()),
                name: "Grand Archivist".into(),
                description: "Add 500 doc comments".into(),
                icon: "📚".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 800,
                crystal_reward: 160,
            },
            Achievement {
                id: AchievementId("crate_doc_clean".into()),
                name: "Crate Complete".into(),
                description: "Achieve 0 missing_docs warnings in a crate".into(),
                icon: "📗".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("workspace_doc_clean".into()),
                name: "Omnia Scripta Sunt".into(),
                description: "0 missing_docs warnings across the entire workspace".into(),
                icon: "📕".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 3_000,
                crystal_reward: 600,
            },
            Achievement {
                id: AchievementId("research_doc_written".into()),
                name: "Historian".into(),
                description: "Write a research document in docs/src/research/".into(),
                icon: "🔭".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("adr_written".into()),
                name: "Architect's Decision".into(),
                description: "Write an Architecture Decision Record (ADR)".into(),
                icon: "📐".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 300,
                crystal_reward: 60,
            },
            // ── Language Explorer ─────────────────────────────────
            Achievement {
                id: AchievementId("first_vox_web_page".into()),
                name: "Web Legionary".into(),
                description: "Compile your first Vox web module with @page or client routes".into(),
                icon: "🌐".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("first_island".into()),
                name: "Island Founder".into(),
                description: "Mount and build your first @island".into(),
                icon: "🏝️".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("first_migration".into()),
                name: "Schema Consul".into(),
                description: "Apply your first @migration to a live database".into(),
                icon: "📦".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("first_seed".into()),
                name: "Sower of Data".into(),
                description: "Run your first @seed function to completion".into(),
                icon: "🌱".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 50,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("first_workflow".into()),
                name: "Workflow Praetor".into(),
                description: "Run a durable workflow to completion".into(),
                icon: "⚙️".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("first_actor".into()),
                name: "Actor Recruiter".into(),
                description: "Spawn your first actor".into(),
                icon: "🤖".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("first_mcp_tool".into()),
                name: "MCP Tribune".into(),
                description: "Register your first @mcp.tool via the capability registry".into(),
                icon: "🔌".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("first_openapi".into()),
                name: "OpenAPI Consul".into(),
                description: "Generate your first OpenAPI spec for a handler".into(),
                icon: "📄".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("five_islands".into()),
                name: "Island Archipelago".into(),
                description: "Build and mount 5 islands".into(),
                icon: "🏝️".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 400,
                crystal_reward: 80,
            },
            Achievement {
                id: AchievementId("first_pkg_publish".into()),
                name: "Market Publisher".into(),
                description: "Publish your first package to VoxPM".into(),
                icon: "🏪".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 300,
                crystal_reward: 60,
            },
            // ── Security ──────────────────────────────────────────
            Achievement {
                id: AchievementId("first_security_pass".into()),
                name: "Shield Bearer".into(),
                description: "Pass your first security review".into(),
                icon: "🛡️".into(),
                category: AchievementCategory::Security,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("no_null_violations".into()),
                name: "Null Purger".into(),
                description: "Pass a codebase scan with 0 null-state violations".into(),
                icon: "🚫".into(),
                category: AchievementCategory::Security,
                xp_reward: 500,
                crystal_reward: 100,
            },
            Achievement {
                id: AchievementId("perf_regression_caught".into()),
                name: "Vigilant Sentinel".into(),
                description: "Catch a performance regression before it merges".into(),
                icon: "⚡".into(),
                category: AchievementCategory::Security,
                xp_reward: 200,
                crystal_reward: 40,
            },
            // ── Daily Quest Milestones ─────────────────────────────
            Achievement {
                id: AchievementId("first_daily_quest".into()),
                name: "Daily Orders Received".into(),
                description: "Complete your first daily quest".into(),
                icon: "📋".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 75,
                crystal_reward: 15,
            },
            Achievement {
                id: AchievementId("daily_quest_streak_3".into()),
                name: "Three-Day Tribune".into(),
                description: "Complete at least one daily quest every day for 3 days".into(),
                icon: "📆".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("daily_quest_streak_7".into()),
                name: "Week of the Legion".into(),
                description: "Complete daily quests every day for a full week".into(),
                icon: "🗓️".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 600,
                crystal_reward: 100,
            },
            Achievement {
                id: AchievementId("daily_quest_streak_30".into()),
                name: "Month of the Imperium".into(),
                description: "Complete daily quests every day for 30 consecutive days".into(),
                icon: "🗓️".into(),
                category: AchievementCategory::Streaks,
                xp_reward: 3_000,
                crystal_reward: 500,
            },
            Achievement {
                id: AchievementId("perfect_daily_3".into()),
                name: "Triumphal March".into(),
                description: "Complete all 3 daily quests in a single day".into(),
                icon: "🏛️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 350,
                crystal_reward: 70,
            },
            Achievement {
                id: AchievementId("perfect_week".into()),
                name: "Fortis et Constans".into(),
                description: "Complete all 3 daily quests every day for a full week".into(),
                icon: "🏆".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 2_500,
                crystal_reward: 400,
            },
            Achievement {
                id: AchievementId("legendary_quest_complete".into()),
                name: "Fato Prudentia".into(),
                description: "Complete a Legendary-modifier quest".into(),
                icon: "⭐".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 1_000,
                crystal_reward: 200,
            },
            Achievement {
                id: AchievementId("chains_quest_complete".into()),
                name: "Chain of Command".into(),
                description: "Complete a Chains-modifier quest and its follow-up".into(),
                icon: "🔗".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 500,
                crystal_reward: 80,
            },
            // ── Level & Prestige Milestones ────────────────────────
            Achievement {
                id: AchievementId("reach_level_10".into()),
                name: "Decanus Rising".into(),
                description: "Reach Level 10".into(),
                icon: "🔟".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("reach_level_25".into()),
                name: "Optio Ascendant".into(),
                description: "Reach Level 25".into(),
                icon: "⚔️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("reach_level_50".into()),
                name: "Signifer's Standard".into(),
                description: "Reach Level 50".into(),
                icon: "🏳️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 700,
                crystal_reward: 120,
            },
            Achievement {
                id: AchievementId("reach_level_100".into()),
                name: "Centurion's Crown".into(),
                description: "Reach Level 100".into(),
                icon: "🦅".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 2_000,
                crystal_reward: 350,
            },
            Achievement {
                id: AchievementId("reach_level_200".into()),
                name: "Legatus Legionis".into(),
                description: "Reach Level 200 — eligible to prestige".into(),
                icon: "👑".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 5_000,
                crystal_reward: 800,
            },
            Achievement {
                id: AchievementId("reach_level_500".into()),
                name: "Consul of the Code".into(),
                description: "Reach Level 500".into(),
                icon: "🏟️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 15_000,
                crystal_reward: 2_500,
            },
            Achievement {
                id: AchievementId("reach_level_1000".into()),
                name: "Imperator Eternus".into(),
                description: "Reach Level 1000".into(),
                icon: "💫".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 50_000,
                crystal_reward: 8_000,
            },
            Achievement {
                id: AchievementId("first_prestige".into()),
                name: "Vindex Coronatus".into(),
                description: "Prestige for the first time by reaching Level 200".into(),
                icon: "🌟".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 8_000,
                crystal_reward: 1_200,
            },
            Achievement {
                id: AchievementId("prestige_5".into()),
                name: "Quinquennium".into(),
                description: "Prestige 5 times".into(),
                icon: "💎".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 25_000,
                crystal_reward: 5_000,
            },
            Achievement {
                id: AchievementId("prestige_10".into()),
                name: "Divus Perpetuus".into(),
                description: "Prestige 10 times — achieved true immortality".into(),
                icon: "✨".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 100_000,
                crystal_reward: 20_000,
            },
            // ── First-Time Actions ─────────────────────────────────
            Achievement {
                id: AchievementId("first_toestub_fix".into()),
                name: "Architectus Purus".into(),
                description: "Fix your first TOESTUB architecture violation".into(),
                icon: "🏛️".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 80,
                crystal_reward: 16,
            },
            Achievement {
                id: AchievementId("first_test_written".into()),
                name: "Primus Testis".into(),
                description: "Write your very first unit test".into(),
                icon: "🧪".into(),
                category: AchievementCategory::BuildMastery,
                xp_reward: 50,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("first_research_ingest".into()),
                name: "Explorator".into(),
                description: "Ingest your first URL into the Codex research collection".into(),
                icon: "🔭".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 30,
                crystal_reward: 6,
            },
            Achievement {
                id: AchievementId("first_adr".into()),
                name: "Architect's Decision".into(),
                description: "Write your first Architecture Decision Record".into(),
                icon: "📐".into(),
                category: AchievementCategory::Documentation,
                xp_reward: 300,
                crystal_reward: 60,
            },
            Achievement {
                id: AchievementId("first_bug_battle".into()),
                name: "Prima Pugna".into(),
                description: "Enter your first bug battle".into(),
                icon: "⚔️".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 30,
                crystal_reward: 8,
            },
            Achievement {
                id: AchievementId("first_battle_won".into()),
                name: "Victor!".into(),
                description: "Win your first bug battle".into(),
                icon: "🏆".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 75,
                crystal_reward: 18,
            },
            Achievement {
                id: AchievementId("first_corpus_rating".into()),
                name: "Iudex Mens".into(),
                description: "Rate your first training pair in the corpus".into(),
                icon: "⚖️".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 25,
                crystal_reward: 5,
            },
            Achievement {
                id: AchievementId("first_handoff_received".into()),
                name: "Messenger Received".into(),
                description: "Receive your first agent handoff from a peer".into(),
                icon: "📨".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 40,
                crystal_reward: 10,
            },
            Achievement {
                id: AchievementId("first_peer_teach".into()),
                name: "Magister".into(),
                description: "Help a newer user resolve a Vox error and receive a 5-star rating"
                    .into(),
                icon: "🎓".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 150,
                crystal_reward: 30,
            },
            Achievement {
                id: AchievementId("first_unsafe_removed".into()),
                name: "Puritas Absoluta".into(),
                description: "Remove an `unsafe` block by replacing it with safe Rust".into(),
                icon: "🛡️".into(),
                category: AchievementCategory::Security,
                xp_reward: 200,
                crystal_reward: 40,
            },
            Achievement {
                id: AchievementId("first_v0_import".into()),
                name: "Island Trader".into(),
                description: "Import your first v0.dev component via `vox import-v0`".into(),
                icon: "🏝️".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("first_scheduled_job".into()),
                name: "Cron Consul".into(),
                description: "Register and run your first `@scheduled` job".into(),
                icon: "⏰".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 120,
                crystal_reward: 25,
            },
            Achievement {
                id: AchievementId("first_turso_query".into()),
                name: "Turso Tribune".into(),
                description: "Execute your first query against a Turso remote database".into(),
                icon: "🗄️".into(),
                category: AchievementCategory::LangExplorer,
                xp_reward: 100,
                crystal_reward: 20,
            },
            Achievement {
                id: AchievementId("first_populi_serve".into()),
                name: "Orator Mens".into(),
                description: "Serve a trained model locally via `vox mens serve`".into(),
                icon: "📡".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 500,
                crystal_reward: 100,
            },
            // ── Social / Foundation ───────────────────────────────
            Achievement {
                id: AchievementId("ten_students_taught".into()),
                name: "Scholae Praepositus".into(),
                description: "Help 10 different users resolve Vox errors".into(),
                icon: "🎓".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 1_000,
                crystal_reward: 200,
            },
            Achievement {
                id: AchievementId("vox_foundation_contributor".into()),
                name: "Benefactor Publicus".into(),
                description: "Have a PR merged into a core Vox compiler crate".into(),
                icon: "🌐".into(),
                category: AchievementCategory::Collaboration,
                xp_reward: 5_000,
                crystal_reward: 800,
            },
            Achievement {
                id: AchievementId("million_lifetime_xp".into()),
                name: "Dives et Sapiens".into(),
                description: "Earn 1,000,000 total XP over your lifetime".into(),
                icon: "💰".into(),
                category: AchievementCategory::Tasks,
                xp_reward: 10_000,
                crystal_reward: 2_000,
            },
            Achievement {
                id: AchievementId("balanced_feedback_week".into()),
                name: "Iusta Statera".into(),
                description: "Give both positive and negative AI feedback in the same week".into(),
                icon: "⚖️".into(),
                category: AchievementCategory::AiCorpus,
                xp_reward: 80,
                crystal_reward: 20,
            },
        ];

        self.definitions = defaults;
    }

    /// Increment a counter for an agent and check for unlocks.
    pub fn increment_counter(&mut self, agent_id: &str, counter: &str) -> Vec<Achievement> {
        let count = {
            let c = self
                .counters
                .entry(agent_id.to_string())
                .or_default()
                .entry(counter.to_string())
                .or_insert(0);
            *c += 1;
            *c
        };

        self.check_unlocks(agent_id, counter, count)
    }

    /// Check if any achievements should unlock based on a counter value.
    pub fn check_unlocks(&mut self, agent_id: &str, counter: &str, value: u32) -> Vec<Achievement> {
        let mut unlocked = Vec::new();

        let thresholds: Vec<(&str, u32)> = match counter {
            "tasks_completed" => vec![
                ("first_task", 1),
                ("five_tasks", 5),
                ("twenty_five_tasks", 25),
                ("hundred_tasks", 100),
                ("five_hundred_tasks", 500),
            ],
            "tasks_today" => vec![("task_in_a_day", 10)],
            "handoffs_completed" => vec![
                ("first_handoff", 1),
                ("ten_handoffs", 10),
                ("fifty_handoffs", 50),
            ],
            "vcs_conflicts_resolved" => vec![("conflict_resolver", 1)],
            "error_free_streak" => vec![("error_free_five", 5)],
            "continuations_received" => vec![("first_continuation", 1)],
            "activity_streak" => vec![
                ("streak_3", 3),
                ("streak_7", 7),
                ("streak_14", 14),
                ("streak_30", 30),
                ("streak_90", 90),
                ("streak_365", 365),
            ],
            "challenges_solved" => vec![("challenge_solved", 1)],
            "memory_entries" => vec![("first_memory", 1)],
            "langs_used" => vec![("polyglot", 5), ("polyglot_10", 10)],
            // AI Corpus
            "ai_feedback_given" => vec![
                ("first_thumbs", 1),
                ("ten_thumbs", 10),
                ("hundred_thumbs", 100),
            ],
            "ai_negative_feedback_given" => vec![("first_thumbs_down", 1)],
            "ai_positive_feedback_given" => vec![("first_thumbs", 1)],
            "vox_examples_written" => vec![("first_vox_example", 1), ("five_vox_examples", 5)],
            "canonical_examples" => vec![("canonical_example", 1)],
            "corpus_contributions" => vec![
                ("first_corpus_contribution", 1),
                ("ten_corpus_contributions", 10),
            ],
            "finetune_epochs" => vec![("first_finetune", 1)],
            "inference_runs" => vec![("inference_regular", 50)],
            // Build Mastery
            "green_builds" => vec![("first_green_build", 1)],
            "consecutive_green_builds" => vec![
                ("build_streak_3", 3),
                ("build_streak_10", 10),
                ("build_streak_30", 30),
            ],
            "builds_fixed" => vec![("first_fix", 1)],
            "zero_warning_checks" => vec![("zero_warnings", 1)],
            "toestub_clean_crates" => vec![("toestub_clean_crate", 1)],
            "toestub_workspace_clean" => vec![("toestub_clean_workspace", 1)],
            // Documentation
            "doc_comments_added" => vec![
                ("first_doc", 1),
                ("fifty_docs", 50),
                ("five_hundred_docs", 500),
            ],
            "crates_doc_clean" => vec![("crate_doc_clean", 1)],
            "workspace_doc_clean" => vec![("workspace_doc_clean", 1)],
            "research_docs_written" => vec![("research_doc_written", 1)],
            "adrs_written" => vec![("adr_written", 1), ("first_adr", 1)],
            // Language Explorer
            "vox_web_pages_compiled" => vec![("first_vox_web_page", 1)],
            "islands_built" => vec![("first_island", 1), ("five_islands", 5)],
            "migrations_applied" => vec![("first_migration", 1)],
            "seeds_run" => vec![("first_seed", 1)],
            "workflows_completed" => vec![("first_workflow", 1)],
            "actors_spawned" => vec![("first_actor", 1)],
            "mcp_tools_registered" => vec![("first_mcp_tool", 1)],
            "openapi_specs_generated" => vec![("first_openapi", 1)],
            "packages_published" => vec![("first_pkg_publish", 1)],
            "v0_imports" => vec![("first_v0_import", 1)],
            "scheduled_jobs_run" => vec![("first_scheduled_job", 1)],
            "turso_queries" => vec![("first_turso_query", 1)],
            "populi_serves" => vec![("first_populi_serve", 1)],
            // Efficiency
            "fast_tasks_30s" => vec![("speed_demon", 1)],
            "fast_tasks_10s" => vec![("ultra_speed", 1)],
            "zero_cost_sessions" => vec![("zero_cost_session", 1)],
            "offline_sessions" => vec![("offline_session", 1)],
            // Security
            "security_reviews_passed" => vec![("first_security_pass", 1)],
            "null_clean_scans" => vec![("no_null_violations", 1)],
            "perf_regressions_caught" => vec![("perf_regression_caught", 1)],
            "unsafe_blocks_removed" => vec![("first_unsafe_removed", 1)],
            // Build mastery (new)
            "toestub_violations_fixed" => vec![("first_toestub_fix", 1)],
            "tests_written" => vec![("first_test_written", 1)],
            // Research / docs (new)
            "research_urls_ingested" => vec![("first_research_ingest", 1)],
            // Battle (new)
            "battles_entered" => vec![("first_bug_battle", 1)],
            "battles_won" => vec![
                ("first_battle_won", 1),
                ("victor_five", 5),
                ("victor_twenty", 20),
            ],
            // Corpus (new)
            "training_pairs_rated" => vec![("first_corpus_rating", 1)],
            // Collaboration (new)
            "handoffs_received" => vec![("first_handoff_received", 1)],
            "students_taught" => vec![("first_peer_teach", 1), ("ten_students_taught", 10)],
            "prs_merged" => vec![("vox_foundation_contributor", 1)],
            // Daily quests (new)
            "daily_quests_completed" => vec![("first_daily_quest", 1)],
            "daily_quest_streak" => vec![
                ("daily_quest_streak_3", 3),
                ("daily_quest_streak_7", 7),
                ("daily_quest_streak_30", 30),
            ],
            "perfect_daily_completions" => vec![("perfect_daily_3", 1)],
            "perfect_weeks" => vec![("perfect_week", 1)],
            "legendary_quests_completed" => vec![("legendary_quest_complete", 1)],
            "chains_quest_pairs_completed" => vec![("chains_quest_complete", 1)],
            // Level milestones (new) — caller sets counter to current level
            "player_level" => vec![
                ("reach_level_10", 10),
                ("reach_level_25", 25),
                ("reach_level_50", 50),
                ("reach_level_100", 100),
                ("reach_level_200", 200),
                ("reach_level_500", 500),
                ("reach_level_1000", 1000),
            ],
            // Prestige (new)
            "prestige_count" => vec![
                ("first_prestige", 1),
                ("prestige_5", 5),
                ("prestige_10", 10),
            ],
            // Lifetime XP milestone — caller converts u64→u32 (saturating) for large values
            "lifetime_xp_millions" => vec![("million_lifetime_xp", 1)],
            // Social feedback balance (new)
            "balanced_feedback_weeks" => vec![("balanced_feedback_week", 1)],
            _ => vec![],
        };

        for (achievement_id, threshold) in thresholds {
            if value >= threshold
                && !self.has_achievement(agent_id, achievement_id)
                && let Some(achievement) =
                    self.definitions.iter().find(|a| a.id.0 == achievement_id)
            {
                let record = UnlockedAchievement {
                    achievement_id: AchievementId(achievement_id.to_string()),
                    agent_id: agent_id.to_string(),
                    unlocked_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                self.unlocked
                    .entry(agent_id.to_string())
                    .or_default()
                    .push(record);
                unlocked.push(achievement.clone());
            }
        }

        unlocked
    }

    /// Check if an agent has a specific achievement.
    pub fn has_achievement(&self, agent_id: &str, achievement_id: &str) -> bool {
        self.unlocked
            .get(agent_id)
            .map(|list| list.iter().any(|a| a.achievement_id.0 == achievement_id))
            .unwrap_or(false)
    }

    /// Get all unlocked achievements for an agent.
    pub fn agent_achievements(&self, agent_id: &str) -> Vec<&Achievement> {
        let unlocked_ids: Vec<&str> = self
            .unlocked
            .get(agent_id)
            .map(|list| list.iter().map(|a| a.achievement_id.0.as_str()).collect())
            .unwrap_or_default();

        self.definitions
            .iter()
            .filter(|a| unlocked_ids.contains(&a.id.0.as_str()))
            .collect()
    }

    /// Get all available achievements.
    pub fn all_achievements(&self) -> &[Achievement] {
        &self.definitions
    }

    /// Get the counter value for an agent.
    pub fn counter_value(&self, agent_id: &str, counter: &str) -> u32 {
        self.counters
            .get(agent_id)
            .and_then(|c| c.get(counter))
            .copied()
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_task_achievement() {
        let mut tracker = AchievementTracker::new();
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");

        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id.0, "first_task");
        assert!(tracker.has_achievement("agent-1", "first_task"));
    }

    #[test]
    fn no_duplicate_unlock() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("agent-1", "tasks_completed");
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");
        // Second increment should not re-unlock
        assert!(unlocked.is_empty());
    }

    #[test]
    fn multiple_achievements_at_threshold() {
        let mut tracker = AchievementTracker::new();
        for _ in 0..4 {
            tracker.increment_counter("agent-1", "tasks_completed");
        }
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");
        // At 5 tasks: "five_tasks" unlocks
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id.0, "five_tasks");

        assert_eq!(tracker.agent_achievements("agent-1").len(), 2);
    }

    #[test]
    fn counter_tracking() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("agent-1", "tasks_completed");
        tracker.increment_counter("agent-1", "tasks_completed");
        assert_eq!(tracker.counter_value("agent-1", "tasks_completed"), 2);
    }
}
