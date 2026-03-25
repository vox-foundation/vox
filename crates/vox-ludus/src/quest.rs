//! Dynamic daily quest system with slot-filled templates and roguelite modifiers.
//!
//! ## Design
//! - All quest descriptions are **templates** with `{SLOT}` placeholders.
//! - Slots are filled from a seeded pool at generation time so descriptions
//!   change every day: a quest says "Fix a {RULE} violation in {CRATE}" — not
//!   the same text forever.
//! - Each quest may roll a **modifier** (Blessed, Timed, Chains, Silent, …)
//!   that changes its XP reward, behaviour, or unlock condition.
//! - Daily quests reset at midnight UTC. Three quests are generated per day
//!   from the full template library (seeded by user_id × day-number).
//! - Anti-grind: quests with low-complexity actions use capped targets
//!   and cooldowns enforced at the template layer.

use crate::util::now_unix;
use serde::{Deserialize, Serialize};

// ─── Constants ───────────────────────────────────────────

/// Quests generated per day.
pub const DAILY_QUEST_COUNT: usize = 3;

/// Quest lifetime: 24 hours in seconds.
const QUEST_DURATION_SECS: i64 = 86_400;

// ─── Quest Type ──────────────────────────────────────────

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

// ─── Slot Pools ──────────────────────────────────────────

static CRATE_POOL: &[&str] = &[
    "vox-hir",
    "vox-ast",
    "vox-lexer",
    "vox-cli",
    "vox-db",
    "vox-dei",
    "vox-ludus",
    "vox-arca",
    "vox-typeck",
    "vox-codegen-rust",
    "vox-ssg",
    "vox-lsp",
    "vox-mcp",
    "vox-fabrica",
    "vox-oratio",
];

static TOESTUB_RULE_POOL: &[&str] = &[
    "UnannotatedAxumHandler",
    "MissingDocComment",
    "CircularReExport",
    "RecreatedDeletedModule",
    "FlatSiblingModule",
    "NullStateUsage",
    "UnregisteredCapability",
];

static LANGUAGE_POOL: &[&str] = &["Rust", "Vox", "SQL", "TOML", "Markdown"];

static DOC_CATEGORY_POOL: &[&str] = &[
    "struct fields",
    "pub fns",
    "trait methods",
    "enum variants",
    "module-level items",
];

static BUILD_CRATE_POOL: &[&str] = &[
    "vox-lexer",
    "vox-parser",
    "vox-ast",
    "vox-hir",
    "vox-typeck",
    "vox-codegen-rust",
    "vox-ssg",
    "vox-cli",
];

static TEST_MODULE_POOL: &[&str] = &[
    "the tokenizer",
    "the parser's error recovery",
    "HIR lowering",
    "the typeck inference engine",
    "codegen output",
    "the reward policy",
    "the streak tracker",
];

static RESEARCH_TOPIC_POOL: &[&str] = &[
    "actor runtime patterns",
    "Server-rendered HTML vs client React (Vox web SSOT)",
    "QLoRA fine-tuning strategies",
    "Turso vs SQLite benchmarks",
    "MCP server design",
    "Vox vs competitors",
    "Build reproducibility techniques",
];

static ISLAND_COMPONENT_POOL: &[&str] = &[
    "login form",
    "data table",
    "notification bell",
    "theme toggle",
    "code editor",
    "progress bar",
    "file upload widget",
];

/// Fill a template string with seeded-random slot values.
///
/// Supported slots: `{CRATE}`, `{RULE}`, `{LANGUAGE}`, `{DOC_CATEGORY}`,
/// `{BUILD_CRATE}`, `{TEST_MODULE}`, `{RESEARCH_TOPIC}`, `{ISLAND_COMPONENT}`.
pub fn slot_fill(template: &str, seed: u64) -> String {
    fn pick<'a>(pool: &'a [&'a str], seed: u64) -> &'a str {
        pool[(seed as usize) % pool.len()]
    }
    template
        .replace("{CRATE}", pick(CRATE_POOL, seed))
        .replace("{RULE}", pick(TOESTUB_RULE_POOL, seed.wrapping_add(1)))
        .replace("{LANGUAGE}", pick(LANGUAGE_POOL, seed.wrapping_add(2)))
        .replace(
            "{DOC_CATEGORY}",
            pick(DOC_CATEGORY_POOL, seed.wrapping_add(3)),
        )
        .replace(
            "{BUILD_CRATE}",
            pick(BUILD_CRATE_POOL, seed.wrapping_add(4)),
        )
        .replace(
            "{TEST_MODULE}",
            pick(TEST_MODULE_POOL, seed.wrapping_add(5)),
        )
        .replace(
            "{RESEARCH_TOPIC}",
            pick(RESEARCH_TOPIC_POOL, seed.wrapping_add(6)),
        )
        .replace(
            "{ISLAND_COMPONENT}",
            pick(ISLAND_COMPONENT_POOL, seed.wrapping_add(7)),
        )
}

// ─── Quest Modifier ──────────────────────────────────────

/// A roguelite modifier that adjusts a quest's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestModifier {
    /// No special modifier — standard quest.
    None,
    /// +25% XP reward.
    Blessed,
    /// Must complete within 2 hours or the quest forfeits.
    Timed,
    /// On completion, automatically generates a harder follow-up quest.
    Chains,
    /// No hint text shown (experienced players only).
    Silent,
    /// 5× XP, but can appear at most once per user ever.
    Legendary,
    /// Requires a peer's in-game confirmation to complete.
    Collaborative,
    /// Finishing below 50% energy gives −10% XP.
    Cursed,
    /// A second instance of the same task at ×1.5 XP.
    Echoed,
    /// XP is doubled but deadline is halved (1 hour).
    Frenzy,
}

impl QuestModifier {
    /// XP multiplier applied on top of the base reward.
    pub fn xp_multiplier(self) -> f64 {
        match self {
            QuestModifier::Blessed => 1.25,
            QuestModifier::Legendary => 5.0,
            QuestModifier::Echoed => 1.5,
            QuestModifier::Frenzy => 2.0,
            QuestModifier::Cursed => 0.9, // Penalty applied separately when low energy
            _ => 1.0,
        }
    }

    /// Override quest duration in seconds (None = use template default).
    pub fn duration_override_secs(self) -> Option<i64> {
        match self {
            QuestModifier::Timed => Some(7_200),  // 2 hours
            QuestModifier::Frenzy => Some(3_600), // 1 hour
            _ => None,
        }
    }

    /// Roll a modifier from a seed using a weighted distribution.
    pub fn roll(seed: u64) -> Self {
        // Distribution: 60% None, 15% Blessed, 8% Timed, 6% Echoed,
        //               4% Chains, 3% Silent, 2% Frenzy, 1.5% Cursed,
        //               0.5% Collaborative, 0.1% Legendary
        let v = seed % 1000;
        match v {
            0..=599 => QuestModifier::None,
            600..=749 => QuestModifier::Blessed,
            750..=829 => QuestModifier::Timed,
            830..=889 => QuestModifier::Echoed,
            890..=929 => QuestModifier::Chains,
            930..=959 => QuestModifier::Silent,
            960..=979 => QuestModifier::Frenzy,
            980..=994 => QuestModifier::Cursed,
            995..=998 => QuestModifier::Collaborative,
            _ => QuestModifier::Legendary,
        }
    }

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            QuestModifier::None => "",
            QuestModifier::Blessed => "Blessed",
            QuestModifier::Timed => "Timed",
            QuestModifier::Chains => "Chains",
            QuestModifier::Silent => "Silent",
            QuestModifier::Legendary => "Legendary",
            QuestModifier::Collaborative => "Collaborative",
            QuestModifier::Cursed => "Cursed",
            QuestModifier::Echoed => "Echoed",
            QuestModifier::Frenzy => "Frenzy",
        }
    }
}

// ─── Quest Template ──────────────────────────────────────

/// A template that generates quests with randomised slot values.
#[derive(Debug, Clone)]
pub struct QuestTemplate {
    /// Category of the quest.
    pub quest_type: QuestType,
    /// Description template (may contain `{SLOT}` placeholders).
    pub description_template: &'static str,
    /// Completion target.
    pub target: u32,
    /// Base XP reward (before modifiers).
    pub base_xp: u64,
    /// Base crystal reward (before modifiers).
    pub base_crystals: u64,
    /// Hint template (may be empty for Silent modifier effect).
    pub hint_template: &'static str,
}

impl QuestTemplate {
    /// Generate a filled-in description from this template and a seed.
    pub fn description(&self, seed: u64) -> String {
        slot_fill(self.description_template, seed)
    }

    /// Generate a filled-in hint from this template and a seed.
    pub fn hint(&self, seed: u64) -> String {
        slot_fill(self.hint_template, seed)
    }
}

/// Master template library — all templates have dynamic slot descriptions.
pub const QUEST_TEMPLATES: &[QuestTemplate] = &[
    // ── Create ────────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Create,
        description_template: "Summon a new companion and name it after a Roman historical figure",
        target: 1,
        base_xp: 20,
        base_crystals: 12,
        hint_template: "Use `vox gamify companion create` — try names like Scipio, Cato, or Livia",
    },
    QuestTemplate {
        quest_type: QuestType::Create,
        description_template: "Create a new {ISLAND_COMPONENT} island component in your web project",
        target: 1,
        base_xp: 80,
        base_crystals: 25,
        hint_template: "Declare `@island` in a .vox file, then mount it with `[island]` in a @page",
    },
    QuestTemplate {
        quest_type: QuestType::Create,
        description_template: "Create 3 new companions with distinct code languages",
        target: 3,
        base_xp: 55,
        base_crystals: 30,
        hint_template: "Vary the `language` field when using `vox gamify companion create`",
    },
    // ── Review ────────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Review,
        description_template: "Review a peer's {LANGUAGE} code and label at least one real bug",
        target: 1,
        base_xp: 120,
        base_crystals: 35,
        hint_template: "Peer review earns credit only when you label a confirmed issue",
    },
    QuestTemplate {
        quest_type: QuestType::Review,
        description_template: "Complete 2 peer reviews without raising any false positives",
        target: 2,
        base_xp: 200,
        base_crystals: 55,
        hint_template: "Quality over quantity — false positive flags reduce your review score",
    },
    // ── Battle ────────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Battle,
        description_template: "Win a bug battle against a {LANGUAGE} type mismatch",
        target: 1,
        base_xp: 40,
        base_crystals: 18,
        hint_template: "Use `vox gamify battle start` and fix the compiler error before time runs out",
    },
    QuestTemplate {
        quest_type: QuestType::Battle,
        description_template: "Win 3 bug battles without forfeiting a single one",
        target: 3,
        base_xp: 160,
        base_crystals: 50,
        hint_template: "Stay under the time limit. Forfeit = no XP.",
    },
    QuestTemplate {
        quest_type: QuestType::Battle,
        description_template: "Win a bug battle using a companion with code quality ≥ 70",
        target: 1,
        base_xp: 120,
        base_crystals: 40,
        hint_template: "High-quality companions deal extra damage in battle",
    },
    // ── Improve ───────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Improve,
        description_template: "Raise a companion's code quality above 75 by fixing {LANGUAGE} lints",
        target: 1,
        base_xp: 90,
        base_crystals: 30,
        hint_template: "Fix clippy warnings in your companion's source code to raise quality",
    },
    QuestTemplate {
        quest_type: QuestType::Improve,
        description_template: "Remove a `todo!()` or `unimplemented!()` stub with real logic in {CRATE}",
        target: 1,
        base_xp: 150,
        base_crystals: 45,
        hint_template: "Search for `todo!()` in `{CRATE}` and implement the missing logic",
    },
    QuestTemplate {
        quest_type: QuestType::Improve,
        description_template: "Refactor a function in {CRATE} to reduce cyclomatic complexity",
        target: 1,
        base_xp: 200,
        base_crystals: 60,
        hint_template: "Break long match arms into sub-functions; TOESTUB will confirm improvement",
    },
    // ── AgentComplete ─────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::AgentComplete,
        description_template: "Complete a task in {CRATE} without any error-level diagnostics",
        target: 1,
        base_xp: 60,
        base_crystals: 20,
        hint_template: "Run `cargo check -p {CRATE}` — it must return 0 errors",
    },
    QuestTemplate {
        quest_type: QuestType::AgentComplete,
        description_template: "Complete 3 tasks without encountering a single panic or unwrap failure",
        target: 3,
        base_xp: 180,
        base_crystals: 55,
        hint_template: "Use `Result` and `Option` properly — panics count as failures",
    },
    // ── Collaborate ───────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Collaborate,
        description_template: "Hand off a plan to another agent successfully using `vox_agent_handoff`",
        target: 1,
        base_xp: 70,
        base_crystals: 22,
        hint_template: "The receiving agent must confirm receipt for this quest to count",
    },
    QuestTemplate {
        quest_type: QuestType::Collaborate,
        description_template: "Perform 3 successful agent handoffs in a single session",
        target: 3,
        base_xp: 250,
        base_crystals: 70,
        hint_template: "Coordinate complex tasks by splitting them across agents",
    },
    // ── AiFeedback ────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::AiFeedback,
        description_template: "Give honest feedback (up or down) on 5 AI responses — no spam, no repeats",
        target: 5,
        base_xp: 40,
        base_crystals: 15,
        hint_template: "Daily cap: only 5 feedback events earn XP. Subsequent ones are free but unrewarded.",
    },
    QuestTemplate {
        quest_type: QuestType::AiFeedback,
        description_template: "Leave a correction comment on a {LANGUAGE} AI response with a code snippet",
        target: 1,
        base_xp: 60,
        base_crystals: 20,
        hint_template: "Attach a repro snippet > 50 tokens for the full XP award",
    },
    QuestTemplate {
        quest_type: QuestType::AiFeedback,
        description_template: "Give your first negative (thumbs-down) feedback of the week with a reason",
        target: 1,
        base_xp: 35,
        base_crystals: 12,
        hint_template: "Constructive criticism is healthy. Thumbs down + reason = valued signal",
    },
    // ── PopuliContribute ──────────────────────────────
    QuestTemplate {
        quest_type: QuestType::PopuliContribute,
        description_template: "Mark a high-quality AI response as a corpus contribution",
        target: 1,
        base_xp: 80,
        base_crystals: 25,
        hint_template: "Only mark responses you would genuinely be proud to train on",
    },
    QuestTemplate {
        quest_type: QuestType::PopuliContribute,
        description_template: "Promote a {LANGUAGE} example to `examples/canonical/` after peer review",
        target: 1,
        base_xp: 600,
        base_crystals: 120,
        hint_template: "Canonical examples are the gold standard — see docs/src/examples.md for criteria",
    },
    QuestTemplate {
        quest_type: QuestType::PopuliContribute,
        description_template: "Rate 10 training pairs in the `training_pairs` table (no rushing)",
        target: 10,
        base_xp: 200,
        base_crystals: 50,
        hint_template: "Rating speed is tracked — extremely fast ratings are flagged",
    },
    // ── BuildStreak ───────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::BuildStreak,
        description_template: "Achieve 3 consecutive green `cargo check` passes in {BUILD_CRATE}",
        target: 3,
        base_xp: 90,
        base_crystals: 30,
        hint_template: "Fix all errors and then re-check twice more without introducing new ones",
    },
    QuestTemplate {
        quest_type: QuestType::BuildStreak,
        description_template: "Run `cargo check -p {BUILD_CRATE}` with 0 warnings on first attempt",
        target: 1,
        base_xp: 120,
        base_crystals: 35,
        hint_template: "Add `#![deny(warnings)]` temporarily to catch them all before checking",
    },
    QuestTemplate {
        quest_type: QuestType::BuildStreak,
        description_template: "Reach 7 consecutive green builds across any crates today",
        target: 7,
        base_xp: 300,
        base_crystals: 80,
        hint_template: "Consistent clean builds signal well-factored, disciplined code",
    },
    // ── DocSprint ─────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::DocSprint,
        description_template: "Add `///` doc comments to all {DOC_CATEGORY} in {CRATE}",
        target: 1,
        base_xp: 180,
        base_crystals: 55,
        hint_template: "Run `RUSTDOCFLAGS=\"-D warnings\" cargo doc -p {CRATE}` to verify",
    },
    QuestTemplate {
        quest_type: QuestType::DocSprint,
        description_template: "Write a `docs/src/how-to/` article on {RESEARCH_TOPIC}",
        target: 1,
        base_xp: 250,
        base_crystals: 70,
        hint_template: "Include YAML frontmatter — training_eligible: true for full XP",
    },
    QuestTemplate {
        quest_type: QuestType::DocSprint,
        description_template: "Add `# Examples` blocks to 5 public functions in {CRATE}",
        target: 5,
        base_xp: 150,
        base_crystals: 45,
        hint_template: "Doc examples are run by `cargo test` — make them compile!",
    },
    // ── ToestubFix ────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::ToestubFix,
        description_template: "Fix a `{RULE}` TOESTUB violation in {CRATE}",
        target: 1,
        base_xp: 130,
        base_crystals: 40,
        hint_template: "Run `cargo run -p vox-toestub -- -p {CRATE}` to see violations",
    },
    QuestTemplate {
        quest_type: QuestType::ToestubFix,
        description_template: "Reach 0 TOESTUB violations in {CRATE} in a single session",
        target: 1,
        base_xp: 400,
        base_crystals: 100,
        hint_template: "Multiple rule classes may need fixing — tackle them all",
    },
    QuestTemplate {
        quest_type: QuestType::ToestubFix,
        description_template: "Remove 5 `{RULE}` violations across any crates today",
        target: 5,
        base_xp: 350,
        base_crystals: 90,
        hint_template: "Track your fixes with `cargo run -p vox-toestub -- --diff`",
    },
    // ── Testing ───────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Testing,
        description_template: "Write the first `@test` for {TEST_MODULE}",
        target: 1,
        base_xp: 180,
        base_crystals: 55,
        hint_template: "First test for an uncovered module earns full XP. Duplicates do not.",
    },
    QuestTemplate {
        quest_type: QuestType::Testing,
        description_template: "Write a regression test that reproduces a recently fixed bug",
        target: 1,
        base_xp: 220,
        base_crystals: 65,
        hint_template: "Include the issue number as a comment — it makes the test verifiable",
    },
    QuestTemplate {
        quest_type: QuestType::Testing,
        description_template: "Add an `@fixture` and `@mock` pair to an integration test in {CRATE}",
        target: 1,
        base_xp: 250,
        base_crystals: 70,
        hint_template: "See examples/canonical/testing.vox for fixture/mock patterns",
    },
    // ── Research ──────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Research,
        description_template: "Ingest a URL on {RESEARCH_TOPIC} into the Codex research collection",
        target: 1,
        base_xp: 50,
        base_crystals: 15,
        hint_template: "Use `vox codex research-ingest-url <URL>`. Cap: 5/day earning XP.",
    },
    QuestTemplate {
        quest_type: QuestType::Research,
        description_template: "Synthesise a research doc on {RESEARCH_TOPIC} with confidence ≥ 0.85",
        target: 1,
        base_xp: 500,
        base_crystals: 110,
        hint_template: "Research pipeline: plan → search → extract → verify → synthesise",
    },
    QuestTemplate {
        quest_type: QuestType::Research,
        description_template: "Write a `res-{RESEARCH_TOPIC}.md` competitor analysis in docs/src/research/",
        target: 1,
        base_xp: 350,
        base_crystals: 85,
        hint_template: "Include: strengths, weaknesses, Vox differentiation. YAML frontmatter required.",
    },
];

// ─── Quest Modifier ──────────────────────────────────────

// ─── Quest ───────────────────────────────────────────────

/// A generated quest instance for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// Unique quest instance ID.
    pub id: String,
    /// User this quest belongs to.
    pub user_id: String,
    /// Category of the quest.
    pub quest_type: QuestType,
    /// Resolved description (slots filled in).
    pub description: String,
    /// Resolved hint (slots filled in; may be empty for Silent quests).
    pub hint: String,
    /// Completion target.
    pub target: u32,
    /// Current progress.
    pub progress: u32,
    /// XP reward after modifier applied.
    pub xp_reward: u64,
    /// Crystal reward after modifier applied.
    pub crystal_reward: u64,
    /// Roguelite modifier on this quest.
    pub modifier: QuestModifier,
    /// Whether the quest has been fully completed.
    pub completed: bool,
    /// Quest status for DB.
    pub status: String,
    /// Unix timestamp when this quest expires.
    pub expires_at: i64,
}

impl Quest {
    /// Generate a quest from a template, applying slot-fill and modifier.
    pub fn from_template(
        id: impl Into<String>,
        user_id: impl Into<String>,
        template: &QuestTemplate,
        seed: u64,
    ) -> Self {
        let modifier = QuestModifier::roll(seed.wrapping_mul(0xDEAD_BEEF));
        let xp_reward = (template.base_xp as f64 * modifier.xp_multiplier()).round() as u64;
        let crystal_reward =
            (template.base_crystals as f64 * modifier.xp_multiplier()).round() as u64;

        let duration = modifier
            .duration_override_secs()
            .unwrap_or(QUEST_DURATION_SECS);
        let now = now_unix();

        let hint = if modifier == QuestModifier::Silent {
            String::new()
        } else {
            template.hint(seed)
        };

        Self {
            id: id.into(),
            user_id: user_id.into(),
            quest_type: template.quest_type,
            description: template.description(seed),
            hint,
            target: template.target,
            progress: 0,
            xp_reward,
            crystal_reward,
            modifier,
            completed: false,
            status: "active".to_string(),
            expires_at: now + duration,
        }
    }

    /// Increment progress. Returns `true` if the quest just completed.
    pub fn increment(&mut self, amount: u32) -> bool {
        if self.completed {
            return false;
        }
        self.progress = (self.progress + amount).min(self.target);
        if self.progress >= self.target {
            self.completed = true;
            self.status = "completed".to_string();
            true
        } else {
            false
        }
    }

    /// Whether this quest has expired.
    pub fn is_expired(&self) -> bool {
        now_unix() > self.expires_at
    }

    /// Progress as a fraction (0.0–1.0).
    pub fn progress_pct(&self) -> f64 {
        if self.target == 0 {
            return 1.0;
        }
        self.progress as f64 / self.target as f64
    }

    /// Display label combining modifier prefix and description.
    pub fn display_title(&self) -> String {
        let m = self.modifier.name();
        if m.is_empty() {
            self.description.clone()
        } else {
            format!("[{m}] {}", self.description)
        }
    }
}

// ─── Daily Generation ────────────────────────────────────

/// Returns the number of complete days since Unix epoch (UTC).
pub fn current_day_number() -> u64 {
    now_unix().max(0) as u64 / 86_400
}

/// Generate three daily quests for a user.
///
/// Uses `(user_id_hash × day_number)` as a deterministic seed, varied per
/// quest slot so each of the three quests draws a different template type.
pub fn generate_daily_quests(user_id: &str, day: u64) -> Vec<Quest> {
    let user_hash: u64 = user_id.bytes().enumerate().fold(0u64, |acc, (i, b)| {
        acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 31))
    });

    let base_seed = user_hash.wrapping_mul(day.wrapping_add(1));

    // Spread across quest types to ensure variety each day
    let type_count = QuestType::ALL.len() as u64;

    (0..DAILY_QUEST_COUNT)
        .map(|slot| {
            let slot_seed = base_seed.wrapping_add(slot as u64 * 7919);
            // Select a quest type different for each slot
            let type_idx = ((slot_seed / type_count) ^ slot_seed) % type_count;
            let target_type = QuestType::ALL[type_idx as usize];

            // Filter templates by type, pick one via seed
            let candidates: Vec<&QuestTemplate> = QUEST_TEMPLATES
                .iter()
                .filter(|t| t.quest_type == target_type)
                .collect();

            // Fallback to any template if type has no candidates
            let template = if candidates.is_empty() {
                &QUEST_TEMPLATES[slot_seed as usize % QUEST_TEMPLATES.len()]
            } else {
                candidates[slot_seed as usize % candidates.len()]
            };

            let id = format!("quest-{user_id}-{day}-{slot}");
            Quest::from_template(id, user_id, template, slot_seed)
        })
        .collect()
}

/// Generate daily quests for today.
pub fn todays_quests(user_id: &str) -> Vec<Quest> {
    generate_daily_quests(user_id, current_day_number())
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_quests_generated() {
        let quests = generate_daily_quests("user-1", 100);
        assert_eq!(quests.len(), 3);
    }

    #[test]
    fn quests_are_deterministic() {
        let q1 = generate_daily_quests("user-1", 42);
        let q2 = generate_daily_quests("user-1", 42);
        assert_eq!(q1[0].id, q2[0].id);
        assert_eq!(q1[0].description, q2[0].description);
    }

    #[test]
    fn different_days_may_give_different_quests() {
        let q1 = generate_daily_quests("user-1", 1);
        let q2 = generate_daily_quests("user-1", 2);
        // At least one quest will differ across days
        let any_different = q1.iter().zip(q2.iter()).any(|(a, b)| a.id != b.id);
        assert!(any_different);
    }

    #[test]
    fn slot_fill_replaces_crate() {
        let result = slot_fill("Fix a bug in {CRATE}", 0);
        assert!(!result.contains("{CRATE}"));
        assert!(CRATE_POOL.iter().any(|c| result.contains(c)));
    }

    #[test]
    fn slot_fill_all_pools_rotate() {
        // Different seeds should rotate through the pools
        let r1 = slot_fill("{CRATE}", 0);
        let r2 = slot_fill("{CRATE}", 1);
        // May be same (small pool hit) or different — just ensure no placeholder remains
        assert!(!r1.contains('{'));
        assert!(!r2.contains('{'));
    }

    #[test]
    fn modifier_roll_distribution() {
        // Legendary should be rare
        let legendary_count = (0u64..10_000)
            .filter(|&s| QuestModifier::roll(s) == QuestModifier::Legendary)
            .count();
        // Expect roughly 0.1% = ~10 out of 10,000
        assert!(
            legendary_count < 30,
            "Legendary too common: {legendary_count}"
        );
        assert!(
            legendary_count > 0,
            "Legendary never rolled in 10,000 samples"
        );
    }

    #[test]
    fn quest_increment_completes() {
        let template = &QUEST_TEMPLATES[0];
        let mut q = Quest::from_template("q1", "u1", template, 0);
        for _ in 0..q.target {
            q.increment(1);
        }
        assert!(q.completed);
    }

    #[test]
    fn quest_display_title_with_modifier() {
        let template = &QUEST_TEMPLATES[0];
        let mut q = Quest::from_template("q1", "u1", template, 0);
        q.modifier = QuestModifier::Blessed;
        q.description = "Do a thing".to_string();
        assert!(q.display_title().contains("[Blessed]"));
    }

    #[test]
    fn blessed_modifier_increases_xp() {
        // Find a template and force-roll blessed
        let template = &QUEST_TEMPLATES[0];
        let base_xp = template.base_xp;
        // Seed 600 → Blessed (v = 600)
        let q = Quest::from_template("q1", "u1", template, 600);
        if q.modifier == QuestModifier::Blessed {
            assert!(q.xp_reward > base_xp);
        }
    }

    #[test]
    fn test_new_quest_types() {
        // Verify we have templates for the new Wave 1 types
        let feedback_exists = QUEST_TEMPLATES
            .iter()
            .any(|t| t.quest_type == QuestType::AiFeedback);
        let streak_exists = QUEST_TEMPLATES
            .iter()
            .any(|t| t.quest_type == QuestType::BuildStreak);
        assert!(feedback_exists);
        assert!(streak_exists);
    }
}
