//! Quest and battle persistence.

use anyhow::Result;
use vox_db::Codex;

use crate::battle::{Battle, BugType};
use crate::quest::{Quest, QuestModifier};

use super::helpers::parse_quest_type;

/// Load all active quests for a user.
pub async fn list_quests(db: &Codex, user_id: &str) -> Result<Vec<Quest>> {
    let rows = db.list_gamify_quests(user_id).await?;
    let mut quests = Vec::new();

    for row in rows {
        let completed = row[7]
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
            != 0;
        let modifier_str = row[10].as_deref().unwrap_or("none");
        let modifier = match modifier_str {
            "blessed" => QuestModifier::Blessed,
            "timed" => QuestModifier::Timed,
            "chains" => QuestModifier::Chains,
            "silent" => QuestModifier::Silent,
            "legendary" => QuestModifier::Legendary,
            "collaborative" => QuestModifier::Collaborative,
            "cursed" => QuestModifier::Cursed,
            "echoed" => QuestModifier::Echoed,
            "frenzy" => QuestModifier::Frenzy,
            _ => QuestModifier::None,
        };

        quests.push(Quest {
            id: row[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            quest_type: parse_quest_type(row[1].as_deref().unwrap_or("build")),
            description: row[2].clone().unwrap_or_default(),
            target: row[3]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1) as u32,
            progress: row[4]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0) as u32,
            crystal_reward: row[5]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(10) as u64,
            xp_reward: row[6]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(15) as u64,
            modifier,
            completed,
            status: row[11].clone().unwrap_or_else(|| "active".to_string()),
            expires_at: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            hint: row[9].clone().unwrap_or_default(),
        });
    }

    Ok(quests)
}

/// Upsert a quest.
pub async fn upsert_quest(db: &Codex, q: &Quest) -> Result<()> {
    db.upsert_gamify_quest(
        &q.id,
        &q.user_id,
        q.quest_type.as_str(),
        &q.description,
        q.xp_reward as i64,
        q.crystal_reward as i64,
        q.target as i64,
        q.progress as i64,
        &q.status,
        q.expires_at,
        q.completed,
    )
    .await?;
    Ok(())
}

/// Get a specific quest by ID.
pub async fn get_quest(db: &Codex, id: &str) -> Result<Option<Quest>> {
    let row = db
        .get_gamify_quest_by_id(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some((
        qid,
        user_id,
        quest_type_s,
        description,
        target,
        progress,
        crystal_reward,
        xp_reward,
        completed_i,
        expires_at,
        hint,
        modifier_str,
        status,
    )) = row
    {
        let completed = completed_i != 0;
        let modifier = serde_json::from_str::<QuestModifier>(&format!("\"{}\"", modifier_str))
            .unwrap_or(QuestModifier::None);

        Ok(Some(Quest {
            id: qid,
            user_id,
            quest_type: parse_quest_type(&quest_type_s),
            description,
            hint,
            target: target as u32,
            progress: progress as u32,
            crystal_reward: crystal_reward as u64,
            xp_reward: xp_reward as u64,
            modifier,
            completed,
            status: if status.is_empty() {
                if completed {
                    "completed".into()
                } else {
                    "active".into()
                }
            } else {
                status
            },
            expires_at,
        }))
    } else {
        Ok(None)
    }
}

/// Update quest status: "pending" | "active" | "completed" | "abandoned".
pub async fn update_quest_status(db: &Codex, user_id: &str, id: &str, status: &str) -> Result<()> {
    let completed = status == "completed";
    db.update_gamify_quest_status(id, user_id, status, completed)
        .await?;
    Ok(())
}

/// Count active/available quests for a user.
pub async fn count_quests(db: &Codex, user_id: &str) -> Result<u32> {
    Ok(db.count_gamify_quests(user_id).await? as u32)
}

/// Delete a quest.
pub async fn delete_quest(db: &Codex, id: &str) -> Result<()> {
    db.delete_gamify_quest(id).await?;
    Ok(())
}

/// Load recent battles for a user.
pub async fn list_battles(db: &Codex, user_id: &str, limit: i64) -> Result<Vec<Battle>> {
    let rows = db.list_gamify_battles(user_id, limit).await?;
    let mut battles = Vec::new();
    for row in rows {
        battles.push(Battle {
            id: row[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            companion_id: row[1].clone().unwrap_or_default(),
            bug_type: match row[2].as_deref().unwrap_or("") {
                "syntax" => BugType::Syntax,
                "logic" => BugType::Logic,
                "performance" => BugType::Performance,
                "security" => BugType::Security,
                _ => BugType::Syntax,
            },
            bug_description: row[3].clone().unwrap_or_default(),
            bug_code: row[4].clone(),
            submitted_code: row[5].clone(),
            success: row[6].as_deref().unwrap_or("0") != "0",
            crystals_earned: row[7].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            xp_earned: row[8].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            duration_secs: row[9].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            created_at: row[10].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
        });
    }
    Ok(battles)
}

/// Insert a new battle record.
pub async fn insert_battle(db: &Codex, b: &Battle) -> Result<()> {
    db.insert_gamify_battle(
        &b.id,
        &b.user_id,
        &b.companion_id,
        b.bug_type.as_str(),
        &b.bug_description,
        b.bug_code.as_deref(),
        b.submitted_code.as_deref(),
        b.success,
        b.crystals_earned as i64,
        b.xp_earned as i64,
        b.duration_secs as i64,
        b.created_at,
    )
    .await?;
    Ok(())
}

/// Get a specific battle by ID.
pub async fn get_battle(db: &Codex, id: &str) -> Result<Option<Battle>> {
    let cols = db
        .get_gamify_battle(id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some(c) = cols {
        fn col_str(c: &[Option<String>], i: usize) -> String {
            c.get(i).and_then(|x| x.clone()).unwrap_or_default()
        }
        fn col_i64(c: &[Option<String>], i: usize) -> i64 {
            c.get(i)
                .and_then(|x| x.as_deref())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        }
        fn col_opt(c: &[Option<String>], i: usize) -> Option<String> {
            c.get(i).and_then(|x| x.clone())
        }
        let bug_type_s = col_str(&c, 3);
        Ok(Some(Battle {
            id: col_str(&c, 0),
            user_id: col_str(&c, 1),
            companion_id: col_str(&c, 2),
            bug_type: match bug_type_s.as_str() {
                "syntax" => BugType::Syntax,
                "logic" => BugType::Logic,
                "performance" => BugType::Performance,
                "security" => BugType::Security,
                _ => BugType::Syntax,
            },
            bug_description: col_str(&c, 4),
            bug_code: col_opt(&c, 5),
            submitted_code: col_opt(&c, 6),
            success: col_i64(&c, 7) != 0,
            crystals_earned: col_i64(&c, 8) as u64,
            xp_earned: col_i64(&c, 9) as u64,
            duration_secs: col_i64(&c, 10) as u64,
            created_at: col_i64(&c, 11),
        }))
    } else {
        Ok(None)
    }
}

/// Update a battle.
pub async fn update_battle(db: &Codex, b: &Battle) -> Result<()> {
    db.update_gamify_battle(
        &b.id,
        b.submitted_code.as_deref(),
        b.success,
        b.crystals_earned as i64,
        b.xp_earned as i64,
        b.duration_secs as i64,
    )
    .await?;
    Ok(())
}

/// Count battles played by a user.
pub async fn count_battles(db: &Codex, user_id: &str) -> Result<i64> {
    Ok(db.count_gamify_battles(user_id).await?)
}
