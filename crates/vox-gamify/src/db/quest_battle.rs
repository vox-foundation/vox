//! Quest and battle persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::battle::{Battle, BugType};
use crate::quest::{Quest, QuestModifier};

use super::helpers::parse_quest_type;

/// Load all active quests for a user.
pub async fn list_quests(db: &Codex, user_id: &str) -> Result<Vec<Quest>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, quest_type, description, CAST(target AS TEXT), CAST(progress AS TEXT),
                    CAST(crystal_reward AS TEXT), CAST(xp_reward AS TEXT), CAST(completed AS TEXT),
                    CAST(expires_at AS TEXT), hint, modifier, status
             FROM gamify_quests WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    let mut quests = Vec::new();
    while let Some(row) = rows.next().await? {
        let row_cols: Vec<Option<String>> = (0..12)
            .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
            .collect();
        let completed = row_cols[7]
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
            != 0;
        let modifier_str = row_cols[10].as_deref().unwrap_or("none");
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
            id: row_cols[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            quest_type: parse_quest_type(row_cols[1].as_deref().unwrap_or("build")),
            description: row_cols[2].clone().unwrap_or_default(),
            target: row_cols[3]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1) as u32,
            progress: row_cols[4]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0) as u32,
            crystal_reward: row_cols[5]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(10) as u64,
            xp_reward: row_cols[6]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(15) as u64,
            modifier,
            completed,
            status: row_cols[11].clone().unwrap_or_else(|| "active".to_string()),
            expires_at: row_cols[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            hint: row_cols[9].clone().unwrap_or_default(),
        });
    }
    Ok(quests)
}

/// Upsert a quest.
pub async fn upsert_quest(db: &Codex, q: &Quest) -> Result<()> {
    let id = q.id.clone();
    let user_id = q.user_id.clone();
    let quest_type = q.quest_type.as_str().to_string();
    let description = q.description.clone();
    let status = q.status.clone();
    let xp_reward = q.xp_reward as i64;
    let crystal_reward = q.crystal_reward as i64;
    let target = q.target as i64;
    let progress = q.progress as i64;
    let expires_at = q.expires_at;
    let completed_flag: i64 = if q.completed { 1 } else { 0 };
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_quests
                 (id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                   quest_type=excluded.quest_type,
                   description=excluded.description, xp_reward=excluded.xp_reward,
                   crystal_reward=excluded.crystal_reward, target=excluded.target,
                   progress=excluded.progress, status=excluded.status,
                   expires_at=excluded.expires_at, completed=excluded.completed",
                params![
                    id.as_str(), user_id.as_str(), quest_type.as_str(),
                    description.as_str(), target, progress, crystal_reward, xp_reward,
                    completed_flag, expires_at, status.as_str(),
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Get a specific quest by ID.
pub async fn get_quest(db: &Codex, id: &str) -> Result<Option<Quest>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at,
                    hint, modifier, status
             FROM gamify_quests WHERE id = ?1",
            params![id],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        let qid: String = row.get(0)?;
        let user_id: String = row.get(1)?;
        let quest_type_s: String = row.get(2)?;
        let description: String = row.get(3)?;
        let target: i64 = row.get(4)?;
        let progress: i64 = row.get(5)?;
        let crystal_reward: i64 = row.get(6)?;
        let xp_reward: i64 = row.get(7)?;
        let completed_i: i64 = row.get(8)?;
        let expires_at: i64 = row.get(9)?;
        let hint: String = row.get::<String>(10).unwrap_or_default();
        let modifier_str: String = row.get::<String>(11).unwrap_or_else(|_| "none".to_string());
        let status: String = row.get::<String>(12).unwrap_or_default();
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
                if completed { "completed".into() } else { "active".into() }
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
    let id = id.to_string();
    let user_id = user_id.to_string();
    let status = status.to_string();
    let completed_flag: i64 = if status == "completed" { 1 } else { 0 };
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_quests SET status = ?1, completed = ?2 WHERE id = ?3 AND user_id = ?4",
                params![status.as_str(), completed_flag, id.as_str(), user_id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Count active/available quests for a user.
pub async fn count_quests(db: &Codex, user_id: &str) -> Result<u32> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let mut rows = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_quests
             WHERE user_id = ?1 AND status = 'active' AND (expires_at = 0 OR expires_at > ?2)",
            params![user_id, now],
        )
        .await?;
    Ok(rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0) as u32)
}

/// Delete a quest.
pub async fn delete_quest(db: &Codex, id: &str) -> Result<()> {
    let id = id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "DELETE FROM gamify_quests WHERE id = ?1",
                params![id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Load recent battles for a user.
pub async fn list_battles(db: &Codex, user_id: &str, limit: i64) -> Result<Vec<Battle>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, companion_id, bug_type, bug_description, bug_code, submitted_code,
                    CAST(success AS TEXT), CAST(crystals_earned AS TEXT),
                    CAST(xp_earned AS TEXT), CAST(duration_secs AS TEXT), CAST(created_at AS TEXT)
             FROM gamify_battles WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            params![user_id, limit],
        )
        .await?;
    let mut battles = Vec::new();
    while let Some(row) = rows.next().await? {
        let row_cols: Vec<Option<String>> = (0..11)
            .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
            .collect();
        battles.push(Battle {
            id: row_cols[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            companion_id: row_cols[1].clone().unwrap_or_default(),
            bug_type: match row_cols[2].as_deref().unwrap_or("") {
                "syntax" => BugType::Syntax,
                "logic" => BugType::Logic,
                "performance" => BugType::Performance,
                "security" => BugType::Security,
                _ => BugType::Syntax,
            },
            bug_description: row_cols[3].clone().unwrap_or_default(),
            bug_code: row_cols[4].clone(),
            submitted_code: row_cols[5].clone(),
            success: row_cols[6].as_deref().unwrap_or("0") != "0",
            crystals_earned: row_cols[7].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            xp_earned: row_cols[8].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            duration_secs: row_cols[9].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            created_at: row_cols[10].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
        });
    }
    Ok(battles)
}

/// Insert a new battle record.
#[allow(clippy::too_many_arguments)]
pub async fn insert_battle(db: &Codex, b: &Battle) -> Result<()> {
    let id = b.id.clone();
    let user_id = b.user_id.clone();
    let companion_id = b.companion_id.clone();
    let bug_type = b.bug_type.as_str().to_string();
    let bug_description = b.bug_description.clone();
    let bug_code = b.bug_code.clone();
    let submitted_code = b.submitted_code.clone();
    let success_flag: i64 = if b.success { 1 } else { 0 };
    let crystals_earned = b.crystals_earned as i64;
    let xp_earned = b.xp_earned as i64;
    let duration_secs = b.duration_secs as i64;
    let created_at = b.created_at;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_battles
                 (id, user_id, companion_id, bug_type, bug_description, bug_code, submitted_code,
                  success, crystals_earned, xp_earned, duration_secs, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id.as_str(), user_id.as_str(), companion_id.as_str(),
                    bug_type.as_str(), bug_description.as_str(),
                    bug_code.as_deref(), submitted_code.as_deref(),
                    success_flag, crystals_earned, xp_earned, duration_secs, created_at
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Get a specific battle by ID.
pub async fn get_battle(db: &Codex, id: &str) -> Result<Option<Battle>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, user_id, companion_id, bug_type, bug_description, bug_code, submitted_code,
                    CAST(success AS TEXT), CAST(crystals_earned AS TEXT),
                    CAST(xp_earned AS TEXT), CAST(duration_secs AS TEXT), CAST(created_at AS TEXT)
             FROM gamify_battles WHERE id = ?1",
            params![id],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        let c: Vec<Option<String>> = (0..12)
            .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
            .collect();
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
    let id = b.id.clone();
    let submitted_code = b.submitted_code.clone();
    let success_flag: i64 = if b.success { 1 } else { 0 };
    let crystals_earned = b.crystals_earned as i64;
    let xp_earned = b.xp_earned as i64;
    let duration_secs = b.duration_secs as i64;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_battles SET submitted_code=?1, success=?2, crystals_earned=?3,
                 xp_earned=?4, duration_secs=?5 WHERE id=?6",
                params![
                    submitted_code.as_deref(), success_flag, crystals_earned,
                    xp_earned, duration_secs, id.as_str(),
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Count battles played by a user.
pub async fn count_battles(db: &Codex, user_id: &str) -> Result<i64> {
    let mut rows = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_battles WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    Ok(rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0))
}
