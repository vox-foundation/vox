use super::kind::QuestType;
use super::slots::slot_fill;

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
        hint_template: "Use `vox ludus companion create` — try names like Scipio, Cato, or Livia",
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
        hint_template: "Vary the `language` field when using `vox ludus companion-create`",
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
    QuestTemplate {
        quest_type: QuestType::Review,
        description_template: "Complete a `security_review_passed` workflow once this week",
        target: 1,
        base_xp: 320,
        base_crystals: 85,
        hint_template: "Security review completion is a rare, high-signal event — pair it with checklist sign-off",
    },
    // ── Battle ────────────────────────────────────────
    QuestTemplate {
        quest_type: QuestType::Battle,
        description_template: "Win a bug battle against a {LANGUAGE} type mismatch",
        target: 1,
        base_xp: 40,
        base_crystals: 18,
        hint_template: "Use `vox ludus battle start` and fix the compiler error before time runs out",
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
    QuestTemplate {
        quest_type: QuestType::Testing,
        description_template: "Earn a `test_suite_green` policy event once today (full suite clean)",
        target: 1,
        base_xp: 260,
        base_crystals: 72,
        hint_template: "Green tests from CI or `vox test` flows record `test_suite_green` via Ludus producers",
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
