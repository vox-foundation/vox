//! # vox-gamify
//!
//! Gamification layer for the Vox programming language.
//!
//! Provides code companions, daily quests, bug battles, ASCII sprites,
//! and a free multi-provider AI client (Pollinations / Ollama / Gemini).
//!
//! All features work fully offline with deterministic fallbacks.
//!
//! Gameplay and AI helpers are organized by submodule; this file only re-exports the main handles.
#![allow(clippy::collapsible_if)]
#![allow(clippy::empty_line_after_doc_comments)]

/// Achievements, unlock tracking, and per-agent counters.
pub mod achievement;
/// Free multi-provider AI client (Ollama, Pollinations, Gemini, deterministic fallback).
pub mod ai;
/// Bug battles seeded from TOESTUB-style findings.
pub mod battle;
/// Coding challenges, attempts, and the daily challenge generator.
pub mod challenge;
/// Code companions: mood, personality, interactions, and ASCII sprites.
pub mod companion;
/// Cost aggregation and summaries for gamification telemetry.
pub mod cost;
/// Persistence helpers for gamify tables via Codex (`VoxDb`).
pub mod db;
/// In-memory agent leaderboards and ranking metrics.
pub mod leaderboard;
/// User-facing notifications and a simple in-session manager.
pub mod notifications;
/// Player profile: XP, levels, crystals, energy, and streaks.
pub mod profile;
/// Quest templates, active quests, and progress helpers.
pub mod quest;
/// Embedded SQL schema versions for gamify migrations.
pub mod schema;
/// ASCII sprite generation and rendering helpers.
pub mod sprite;
/// Daily streak tracking with grace periods and bonus XP.
pub mod streak;
/// Small shared helpers (e.g. wall-clock timestamps).
pub mod util;

// Re-export key types for ergonomic access.
pub use achievement::{Achievement, AchievementTracker};
pub use ai::{AiError, FreeAiClient, FreeAiProvider};
pub use battle::{Battle, BugType};
pub use challenge::{Challenge, ChallengeManager, ChallengeType};
pub use companion::{Companion, Interaction, Mood};

pub use cost::{CostAggregator, CostRecord, CostSummary};
pub use leaderboard::{Leaderboard, LeaderboardMetric};
pub use notifications::{Notification, NotificationManager, NotificationType};
pub use profile::GamifyProfile;

pub use quest::{Quest, QuestType};
pub use schema::{SCHEMA_V5, SCHEMA_V6};
pub use streak::{StreakResult, StreakTracker};
