//! Clap surface for `vox ludus` (requires `--features extras-ludus`).

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use super::ludus;

/// Subcommands for `vox ludus`.
#[derive(Parser)]
pub enum LudusCli {
    Morning,
    Record,
    Status,
    /// Authenticate Ludus with a remote provider (e.g. GitHub).
    Auth {
        /// The provider to authenticate with (currently only "github" is supported).
        #[arg(value_name = "PROVIDER")]
        provider: String,
    },
    /// Synchronize external contribution data from GitHub and award XP.
    SyncGithub,
    /// Run a Monte Carlo battle simulation sweep to validate combat balance.
    MonteCarloSweep {
        /// Number of battles to simulate (default: 1000).
        #[arg(long, default_value_t = 1000)]
        iterations: u32,
        /// Directory to write JSONL/Markdown artifacts.
        #[arg(long, default_value = ".vox/artifacts/simulation")]
        output_dir: PathBuf,
    },
    CompanionList,
    CompanionCreate {
        #[arg(long)]
        name: String,
        #[arg(long)]
        code_file: PathBuf,
    },
    CompanionShow {
        #[arg(long)]
        name: String,
    },
    QuestList,
    QuestGenerate,
    BattleStart {
        #[arg(long)]
        companion_name: String,
    },
    BattleSubmit {
        #[arg(long)]
        companion_name: String,
        #[arg(long)]
        code_file: PathBuf,
    },
    CompanionInteract {
        #[arg(long)]
        name: String,
        #[arg(long)]
        interaction: String,
    },
    FeedbackRate {
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        response_id: String,
        #[arg(long)]
        thumbs_up: bool,
        #[arg(long)]
        comment: Option<String>,
        #[arg(long)]
        example: Option<PathBuf>,
    },
    RewardClaim,
    /// Enable Ludus and save to global config.
    Enable,
    /// Disable Ludus and save to global config.
    Disable,
    Mode {
        /// Show effective mode after env/session overrides (`VOX_LUDUS_SESSION_*`, kill-switch).
        #[arg(long)]
        effective: bool,
        #[arg(long)]
        set: Option<String>,
    },
    /// Local KPI aggregates (policy snapshots + hint telemetry).
    Metrics,
    /// Short combined summary (profile + policy rows).
    Digest,
    /// Rolling 7-day KPI + notifications + policy awards.
    DigestWeekly,
    /// Recent reward-policy rows (transparency / debugging).
    Audit {
        #[arg(long, default_value_t = 24)]
        limit: usize,
    },
    /// Copy `default` user progress into the local user when local has no profile.
    ProfileMerge,
    LeaderboardShow {
        #[arg(long)]
        metric: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    ShopList,
    ShopBuy {
        #[arg(long)]
        item_id: String,
    },
    ChallengeList,
    ChallengeStart {
        #[arg(long)]
        id: String,
    },
    ChallengeSubmit {
        #[arg(long)]
        id: String,
        #[arg(long)]
        code_file: PathBuf,
    },
    NotifyList {
        /// Mark notifications read after listing (default: peek only).
        #[arg(long)]
        read: bool,
    },
    NotifyClear,
    Hint {
        #[arg(long)]
        context: Option<String>,
    },
    GlyphList {
        #[arg(long, default_value_t = false)]
        unlocked_only: bool,
    },
    CollegiumNew {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    CollegiumList,
    CollegiumJoin {
        #[arg(long)]
        id: String,
    },
    CollegiumStatus {
        #[arg(long)]
        id: Option<String>,
    },
    ArenaShow,
    ArenaJoin,
    ArenaLeaderboard,
    PackList,
    PackInit {
        #[arg(long)]
        template: String,
    },
    ShieldUse,
    /// Live terminal HUD over the in-process orchestrator (requires `ludus-hud`).
    #[cfg(feature = "ludus-hud")]
    Hud,
    DisputeFile {
        #[arg(long)]
        target_user: String,
        #[arg(long)]
        event_id: Option<String>,
        #[arg(long)]
        rationale: String,
    },
    DisputeVote {
        #[arg(long)]
        dispute_id: String,
        #[arg(long)]
        verdict: String,
        #[arg(long)]
        rationale: Option<String>,
    },
    DisputeStatus {
        #[arg(long)]
        dispute_id: String,
    },
    DisputeAppeal {
        #[arg(long)]
        dispute_id: String,
    },
}

/// Dispatch `vox ludus …`.
pub async fn run(cmd: LudusCli) -> Result<()> {
    match cmd {
        LudusCli::Morning => ludus::morning_digest().await,
        LudusCli::Record => ludus::record_activity().await,
        LudusCli::Status => ludus::status().await,
        LudusCli::Auth { provider } => ludus::auth_command(&provider).await,
        LudusCli::SyncGithub => ludus::sync_command().await,
        LudusCli::MonteCarloSweep {
            iterations,
            output_dir,
        } => ludus::run_monte_carlo_sweep(iterations, output_dir).await,
        LudusCli::CompanionList => ludus::companion_list().await,
        LudusCli::CompanionCreate { name, code_file } => {
            ludus::companion_create(&name, &code_file).await
        }
        LudusCli::CompanionShow { name } => ludus::companion_show(&name).await,
        LudusCli::QuestList => ludus::quest_list().await,
        LudusCli::QuestGenerate => ludus::quest_generate().await,
        LudusCli::BattleStart { companion_name } => ludus::battle_start(&companion_name).await,
        LudusCli::BattleSubmit {
            companion_name,
            code_file,
        } => ludus::battle_submit(&companion_name, &code_file).await,
        LudusCli::CompanionInteract { name, interaction } => {
            ludus::companion_interact_str(&name, &interaction).await
        }
        LudusCli::FeedbackRate {
            session_id,
            response_id,
            thumbs_up,
            comment,
            example,
        } => {
            ludus::feedback_rate(
                &session_id,
                &response_id,
                thumbs_up,
                comment.as_deref(),
                example.as_deref(),
            )
            .await
        }
        LudusCli::RewardClaim => ludus::reward_claim().await,
        LudusCli::Enable => ludus::enable_ludus().await,
        LudusCli::Disable => ludus::disable_ludus().await,
        LudusCli::Mode { effective, set } => ludus::mode_command(set.as_deref(), effective).await,
        LudusCli::Metrics => ludus::metrics_show().await,
        LudusCli::Digest => ludus::session_digest().await,
        LudusCli::DigestWeekly => ludus::digest_weekly().await,
        LudusCli::Audit { limit } => ludus::audit_show(limit).await,
        LudusCli::ProfileMerge => ludus::profile_merge_from_default().await,
        LudusCli::LeaderboardShow { metric, limit } => {
            ludus::leaderboard_show(&metric, limit).await
        }
        LudusCli::ShopList => ludus::shop_list().await,
        LudusCli::ShopBuy { item_id } => ludus::shop_buy(&item_id).await,
        LudusCli::ChallengeList => ludus::challenge_list().await,
        LudusCli::ChallengeStart { id } => ludus::challenge_start(&id).await,
        LudusCli::ChallengeSubmit { id, code_file } => {
            ludus::challenge_submit(&id, &code_file).await
        }
        LudusCli::NotifyList { read } => ludus::notify_list(read).await,
        LudusCli::NotifyClear => ludus::notify_clear().await,
        LudusCli::Hint { context } => ludus::hint_show(context.as_deref()).await,
        LudusCli::GlyphList { unlocked_only } => ludus::glyph_list(unlocked_only).await,
        LudusCli::CollegiumNew { name, description } => {
            ludus::collegium_new(&name, description.as_deref()).await
        }
        LudusCli::CollegiumList => ludus::collegium_list().await,
        LudusCli::CollegiumJoin { id } => ludus::collegium_join(&id).await,
        LudusCli::CollegiumStatus { id } => ludus::collegium_status(id.as_deref()).await,
        LudusCli::ArenaShow => ludus::arena_show().await,
        LudusCli::ArenaJoin => ludus::arena_join().await,
        LudusCli::ArenaLeaderboard => ludus::arena_leaderboard().await,
        LudusCli::PackList => ludus::pack_list().await,
        LudusCli::PackInit { template } => ludus::pack_init(&template).await,
        LudusCli::ShieldUse => ludus::shield_use().await,
        #[cfg(feature = "ludus-hud")]
        LudusCli::Hud => ludus::ludus_hud_run().await,
        LudusCli::DisputeFile {
            target_user,
            event_id,
            rationale,
        } => ludus::dispute_file(&target_user, event_id.as_deref(), &rationale).await,
        LudusCli::DisputeVote {
            dispute_id,
            verdict,
            rationale,
        } => ludus::dispute_vote(&dispute_id, &verdict, rationale.as_deref()).await,
        LudusCli::DisputeStatus { dispute_id } => ludus::dispute_status(&dispute_id).await,
        LudusCli::DisputeAppeal { dispute_id } => ludus::dispute_appeal(&dispute_id).await,
    }
}
