//! Non-nagging adaptive teaching subsystem.
//!
//! ## Design Principles
//! - **Profile-aware**: hint frequency and depth adapt to `GamifyMode`.
//! - **Non-nagging**: cooldown windows and confidence thresholds prevent spam.
//! - **Stop-after-ignored**: hints stop escalating once the user dismisses 3 in a row.
//! - **Event-triggered**: only fires on objective events (compile failure, repeated TODO, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::util::now_unix;

// ── Mistake taxonomy ──────────────────────────────────────

/// Categories of mistakes that trigger contextual guidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MistakeKind {
    /// Lexer/parser/type errors in Vox source.
    SyntaxError,
    /// Type inference or type check failure.
    TypeCheckError,
    /// Test assertion failures.
    TestFailure,
    /// Workflow/orchestrator logic errors.
    WorkflowError,
    /// Architectural anti-patterns (e.g., circular imports, TOESTUB violations).
    ArchitecturalIssue,
    /// TODO/FIXME comments accumulating beyond a threshold.
    TodoDebt,
    /// Missing documentation on public APIs.
    MissingDoc,
    /// Security-related patterns detected.
    SecurityHint,
}

impl MistakeKind {
    /// Human-readable category label.
    pub fn label(&self) -> &'static str {
        match self {
            MistakeKind::SyntaxError => "Syntax",
            MistakeKind::TypeCheckError => "Type Check",
            MistakeKind::TestFailure => "Test",
            MistakeKind::WorkflowError => "Workflow",
            MistakeKind::ArchitecturalIssue => "Architecture",
            MistakeKind::TodoDebt => "TODO Debt",
            MistakeKind::MissingDoc => "Documentation",
            MistakeKind::SecurityHint => "Security",
        }
    }

    /// Latin/Roman thematic term for this mistake category.
    pub fn roman_term(&self) -> &'static str {
        match self {
            MistakeKind::SyntaxError => "Erratum Scripturae",
            MistakeKind::TypeCheckError => "Conflictus Typorum",
            MistakeKind::TestFailure => "Probatio Cadit",
            MistakeKind::WorkflowError => "Via Confusa",
            MistakeKind::ArchitecturalIssue => "Fundamentum Instabile",
            MistakeKind::TodoDebt => "Debitum Agendum",
            MistakeKind::MissingDoc => "Codex Incompletus",
            MistakeKind::SecurityHint => "Porta Aperta",
        }
    }

    /// Cooldown in seconds between hints for this kind.
    pub fn base_cooldown_secs(&self) -> i64 {
        match self {
            MistakeKind::SyntaxError => 300,         // 5 min
            MistakeKind::TypeCheckError => 600,      // 10 min
            MistakeKind::TestFailure => 600,         // 10 min
            MistakeKind::WorkflowError => 900,       // 15 min
            MistakeKind::ArchitecturalIssue => 3600, // 1 hour
            MistakeKind::TodoDebt => 7200,           // 2 hours
            MistakeKind::MissingDoc => 86400,        // 1 day
            MistakeKind::SecurityHint => 1800,       // 30 min
        }
    }
}

// ── Tutorial progression ──────────────────────────────────

/// User's tutorial progression state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TutorialStage {
    /// First session — show onboarding hints.
    #[default]
    Onboarding,
    /// Familiar with basics — show contextual hints only.
    Guided,
    /// Experienced user — minimal guidance, only critical hints.
    Independent,
}

// ── Hint throttle ─────────────────────────────────────────

/// Per-kind cooldown state tracked in the teaching profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HintCooldownState {
    /// Last time a hint was shown for this kind. 0 = never.
    pub last_shown_unix: i64,
    /// How many consecutive hints have been dismissed/ignored.
    pub consecutive_dismissed: u32,
    /// Total hints shown ever.
    pub total_shown: u32,
}

impl HintCooldownState {
    /// Whether this kind is currently cooling down.
    pub fn is_cooling_down(&self, base_secs: i64) -> bool {
        if self.last_shown_unix == 0 {
            return false;
        }
        now_unix() - self.last_shown_unix < base_secs
    }

    /// Whether hints for this kind should be suppressed (user keeps ignoring them).
    pub fn is_suppressed(&self) -> bool {
        self.consecutive_dismissed >= 3
    }

    /// Record a hint as shown.
    pub fn record_shown(&mut self) {
        self.last_shown_unix = now_unix();
        self.total_shown += 1;
    }

    /// Record a hint as dismissed.
    pub fn record_dismissed(&mut self) {
        self.consecutive_dismissed += 1;
    }

    /// Record that the user acted on the hint (resets suppression).
    pub fn record_acted_on(&mut self) {
        self.consecutive_dismissed = 0;
    }
}

// ── Teaching profile ──────────────────────────────────────

/// Persistent teaching state for a user.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeachingProfile {
    /// Owning user ID.
    pub user_id: String,
    /// Current tutorial stage for this user.
    pub stage: TutorialStage,
    /// Per-kind hint cooldown tracking.
    pub cooldowns: HashMap<String, HintCooldownState>,
    /// Total mistakes recorded per kind (for confidence scoring).
    pub mistake_counts: HashMap<String, u32>,
    /// Whether the user has explicitly silenced teaching hints.
    pub silenced: bool,
}

impl TeachingProfile {
    /// Create a new profile for a user starting at onboarding.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            stage: TutorialStage::Onboarding,
            cooldowns: HashMap::new(),
            mistake_counts: HashMap::new(),
            silenced: false,
        }
    }

    /// Advance the tutorial stage based on milestone count.
    pub fn maybe_advance_stage(&mut self) {
        let total: u32 = self.mistake_counts.values().sum();
        self.stage = match total {
            0..=4 => TutorialStage::Onboarding,
            5..=19 => TutorialStage::Guided,
            _ => TutorialStage::Independent,
        };
    }

    /// Record a mistake occurrence and return whether a hint should be shown.
    pub fn record_mistake(
        &mut self,
        kind: MistakeKind,
        mode_hint_freq: f64,
    ) -> Option<HintRequest> {
        if self.silenced {
            return None;
        }

        let key = format!("{:?}", kind);
        *self.mistake_counts.entry(key.clone()).or_insert(0) += 1;
        self.maybe_advance_stage();

        // Mode gate: Serious mode → no hints
        if mode_hint_freq <= 0.0 {
            return None;
        }

        let cooldown = self.cooldowns.entry(key.clone()).or_default();

        // Suppression check
        if cooldown.is_suppressed() {
            return None;
        }

        // Cooldown check (with mode adjustment: Balanced halves the cooldown)
        let effective_cooldown =
            (kind.base_cooldown_secs() as f64 * (1.0 / mode_hint_freq.max(0.1))) as i64;
        if cooldown.is_cooling_down(effective_cooldown) {
            return None;
        }

        // Confidence: don't hint on the very first occurrence in Independent mode
        let count = self.mistake_counts.get(&key).copied().unwrap_or(0);
        if self.stage == TutorialStage::Independent && count == 1 {
            return None;
        }

        cooldown.record_shown();

        Some(HintRequest {
            kind,
            stage: self.stage.clone(),
            occurrence_count: count,
        })
    }

    /// User dismissed a hint for this kind.
    pub fn dismiss_hint(&mut self, kind: MistakeKind) {
        let key = format!("{:?}", kind);
        self.cooldowns.entry(key).or_default().record_dismissed();
    }

    /// User acted on a hint (e.g., fixed the error).
    pub fn acted_on_hint(&mut self, kind: MistakeKind) {
        let key = format!("{:?}", kind);
        self.cooldowns.entry(key).or_default().record_acted_on();
    }
}

// ── Hint request ──────────────────────────────────────────

/// A request for a contextual hint to be shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintRequest {
    /// The type of mistake that triggered this hint.
    pub kind: MistakeKind,
    /// The user's current tutorial stage.
    pub stage: TutorialStage,
    /// How many times this mistake kind has occurred.
    pub occurrence_count: u32,
}

// ── Hint content ──────────────────────────────────────────

/// A rendered hint ready to show to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hint {
    /// The mistake category this hint addresses.
    pub kind: MistakeKind,
    /// Latin/Roman thematic term for display.
    pub roman_term: String,
    /// Short headline for the hint.
    pub title: String,
    /// Detailed explanation or action guidance.
    pub body: String,
    /// Optional Vox-specific context (e.g., a relevant CLI command).
    pub vox_context: Option<String>,
    /// Whether the user can dismiss this hint.
    pub dismissable: bool,
}

impl Hint {
    /// Build a deterministic hint for a request (no AI needed).
    pub fn deterministic(req: &HintRequest) -> Self {
        let (title, body, ctx) =
            deterministic_hint_content(req.kind, req.stage.clone(), req.occurrence_count);
        Self {
            kind: req.kind,
            roman_term: req.kind.roman_term().to_string(),
            title,
            body,
            vox_context: ctx,
            dismissable: true,
        }
    }
}

fn deterministic_hint_content(
    kind: MistakeKind,
    stage: TutorialStage,
    count: u32,
) -> (String, String, Option<String>) {
    match kind {
        MistakeKind::SyntaxError => {
            let title = "Syntax Issue Detected".to_string();
            let body = match stage {
                TutorialStage::Onboarding => "Check your brackets, colons, and keyword spelling. Vox syntax is close to TypeScript.".to_string(),
                TutorialStage::Guided => "The parser error shows the line and column. Look at the token before the error marker.".to_string(),
                TutorialStage::Independent if count > 5 => format!("You've hit {} syntax errors in this session. `vox fmt` is not wired yet — compare your file to golden examples under `examples/` and `examples/STYLE.md`; use `vox check` for diagnostics.", count),
                _ => "Use `vox check` to see detailed diagnostics with source spans.".to_string(),
            };
            let ctx = Some("Vox: `vox check` shows all errors with column markers.".to_string());
            (title, body, ctx)
        }
        MistakeKind::TypeCheckError => {
            let title = "Type Conflict".to_string();
            let body = match stage {
                TutorialStage::Onboarding => "Vox uses bidirectional type inference. If the type can't be inferred, add an explicit annotation: `let x: u32 = ...`".to_string(),
                TutorialStage::Guided => "Check that function return types match their declared signatures. Generics require trait bounds.".to_string(),
                _ => "Use `Option[T]` for nullable values — `null` is banned in Vox.".to_string(),
            };
            (
                title,
                body,
                Some("Vox: `null` is always banned. Use `Option[T]` or `Result`.".to_string()),
            )
        }
        MistakeKind::TestFailure => {
            let title = "Test Failure".to_string();
            let body = match stage {
                TutorialStage::Onboarding => "Run `cargo test -p <crate>` to see the assertion that failed. Each test shows expected vs actual.".to_string(),
                _ => format!("Test failed {} time(s). Check if the test logic is testing the right contract, not implementation details.", count),
            };
            (title, body, None)
        }
        MistakeKind::TodoDebt => {
            let title = "TODO Debt Accumulating".to_string();
            let body = "You have TODO/FIXME comments building up. Consider using `vox gamify quest-generate` to turn them into trackable quests and earn XP for fixing them.".to_string();
            (title, body, Some("Vox: `vox gamify quest-generate` scans your workspace for TODOs and creates quests.".to_string()))
        }
        MistakeKind::ArchitecturalIssue => {
            let title = "Architecture Issue".to_string();
            let body = "Check the TOESTUB rules: modules split into sub-files need a `mod.rs` facade. Avoid circular re-exports.".to_string();
            (title, body, None)
        }
        MistakeKind::MissingDoc => {
            let title = "Public Item Missing Documentation".to_string();
            let body = "All public Rust items need `///` doc comments. CI runs `cargo doc --no-deps` with `-D warnings`.".to_string();
            (title, body, None)
        }
        MistakeKind::WorkflowError => {
            let title = "Workflow Error".to_string();
            let body = "Check the agent event log in `vox dashboard` for the root cause. Orchestrator errors include the full event payload.".to_string();
            (title, body, None)
        }
        MistakeKind::SecurityHint => {
            let title = "Security Pattern Detected".to_string();
            let body = "Review this code for potential injection, privilege escalation, or unsafe data handling.".to_string();
            (title, body, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_mistake_triggers_hint_in_balanced() {
        let mut profile = TeachingProfile::new("user-1");
        // First occurrence in onboarding + balanced (freq 0.4) → hint
        let hint = profile.record_mistake(MistakeKind::SyntaxError, 0.4);
        assert!(hint.is_some());
    }

    #[test]
    fn serious_mode_suppresses_hints() {
        let mut profile = TeachingProfile::new("user-1");
        let hint = profile.record_mistake(MistakeKind::SyntaxError, 0.0);
        assert!(hint.is_none(), "serious mode should suppress hints");
    }

    #[test]
    fn cooldown_prevents_repeat() {
        let mut profile = TeachingProfile::new("user-1");
        let _ = profile.record_mistake(MistakeKind::TestFailure, 1.0);
        // Second call should be cooled down
        let second = profile.record_mistake(MistakeKind::TestFailure, 1.0);
        assert!(second.is_none(), "cooldown should block second hint");
    }

    #[test]
    fn three_dismissals_suppresses() {
        let mut profile = TeachingProfile::new("user-1");
        // Force hint + dismiss 3 times (bypassing cooldown by resetting state)
        for _ in 0..3 {
            let key = format!("{:?}", MistakeKind::SyntaxError);
            let cooldown = profile.cooldowns.entry(key).or_default();
            cooldown.last_shown_unix = 0; // reset cooldown
            profile.dismiss_hint(MistakeKind::SyntaxError);
        }
        let hint = profile.record_mistake(MistakeKind::SyntaxError, 1.0);
        assert!(hint.is_none(), "3 dismissals should suppress hints");
    }

    #[test]
    fn acted_on_resets_suppression() {
        let mut profile = TeachingProfile::new("user-1");
        for _ in 0..3 {
            profile.dismiss_hint(MistakeKind::SyntaxError);
        }
        profile.acted_on_hint(MistakeKind::SyntaxError);
        let key = format!("{:?}", MistakeKind::SyntaxError);
        assert_eq!(profile.cooldowns[&key].consecutive_dismissed, 0);
    }

    #[test]
    fn stage_advances_with_mistakes() {
        let mut profile = TeachingProfile::new("user-1");
        assert_eq!(profile.stage, TutorialStage::Onboarding);
        for _ in 0..5 {
            *profile
                .mistake_counts
                .entry("SyntaxError".to_string())
                .or_insert(0) += 1;
        }
        profile.maybe_advance_stage();
        assert_eq!(profile.stage, TutorialStage::Guided);
    }

    #[test]
    fn deterministic_hint_non_empty() {
        let req = HintRequest {
            kind: MistakeKind::TypeCheckError,
            stage: TutorialStage::Onboarding,
            occurrence_count: 1,
        };
        let hint = Hint::deterministic(&req);
        assert!(!hint.title.is_empty());
        assert!(!hint.body.is_empty());
        assert!(!hint.roman_term.is_empty());
    }
}
