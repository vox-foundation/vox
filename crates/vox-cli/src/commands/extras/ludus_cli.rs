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
    Mode {
        #[arg(long)]
        set: Option<String>,
    },
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
    NotifyList,
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
}

/// Dispatch `vox ludus …`.
pub async fn run(cmd: LudusCli) -> Result<()> {
    match cmd {
        LudusCli::Morning => ludus::morning_digest().await,
        LudusCli::Record => ludus::record_activity().await,
        LudusCli::Status => ludus::status().await,
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
        LudusCli::Mode { set } => ludus::mode_command(set.as_deref()).await,
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
        LudusCli::NotifyList => ludus::notify_list().await,
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
    }
}
