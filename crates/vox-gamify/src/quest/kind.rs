use serde::{Deserialize, Serialize};

/// Categories of daily quests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestType {
    /// Create new companions or components.
    Create,
    /// Perform peer code reviews.
    Review,
    /// Win bug battles.
    Battle,
    /// Improve companion code quality (TOESTUB/clippy fixes).
    Improve,
    /// Complete agent tasks without errors.
    AgentComplete,
    /// Hand off plans to another agent.
    Collaborate,
    /// Give AI response feedback (thumbs up or down).
    AiFeedback,
    /// Contribute training examples to the Mens corpus.
    PopuliContribute,
    /// Achieve consecutive green builds.
    BuildStreak,
    /// Add documentation to public items.
    DocSprint,
    /// Fix TOESTUB architecture violations.
    ToestubFix,
    /// Write or improve tests.
    Testing,
    /// Ingest or synthesise a research document.
    Research,
    /// Accomplish a first-ever action (one-time quest type).
    FirstTime,
}

impl QuestType {
    /// All repeatable quest types (excluding FirstTime).
    pub const ALL: &'static [QuestType] = &[
        QuestType::Create,
        QuestType::Review,
        QuestType::Battle,
        QuestType::Improve,
        QuestType::AgentComplete,
        QuestType::Collaborate,
        QuestType::AiFeedback,
        QuestType::PopuliContribute,
        QuestType::BuildStreak,
        QuestType::DocSprint,
        QuestType::ToestubFix,
        QuestType::Testing,
        QuestType::Research,
    ];

    /// DB slug.
    pub const fn as_str(&self) -> &str {
        match self {
            QuestType::Create => "create",
            QuestType::Review => "review",
            QuestType::Battle => "battle",
            QuestType::Improve => "improve",
            QuestType::AgentComplete => "agent_complete",
            QuestType::Collaborate => "collaborate",
            QuestType::AiFeedback => "ai_feedback",
            QuestType::PopuliContribute => "populi_contribute",
            QuestType::BuildStreak => "build_streak",
            QuestType::DocSprint => "doc_sprint",
            QuestType::ToestubFix => "toestub_fix",
            QuestType::Testing => "testing",
            QuestType::Research => "research",
            QuestType::FirstTime => "first_time",
        }
    }

    /// Display emoji.
    pub fn emoji(&self) -> &str {
        match self {
            QuestType::Create => "🔨",
            QuestType::Review => "📝",
            QuestType::Battle => "⚔️",
            QuestType::Improve => "📈",
            QuestType::AgentComplete => "✅",
            QuestType::Collaborate => "🤝",
            QuestType::AiFeedback => "👍",
            QuestType::PopuliContribute => "🧠",
            QuestType::BuildStreak => "🟢",
            QuestType::DocSprint => "📜",
            QuestType::ToestubFix => "🏛️",
            QuestType::Testing => "🧪",
            QuestType::Research => "🔭",
            QuestType::FirstTime => "⭐",
        }
    }
}

impl std::fmt::Display for QuestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
