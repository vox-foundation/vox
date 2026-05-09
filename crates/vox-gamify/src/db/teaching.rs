//! Teaching profiles and policy snapshots.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::teaching::TeachingProfile;

/// Load a teaching profile. Returns a fresh default if none exists yet.
pub async fn get_teaching_profile(db: &Codex, user_id: &str) -> Result<TeachingProfile> {
    let mut rows = db
        .connection()
        .query(
            "SELECT stage, silenced, mistake_counts, cooldowns
             FROM gamify_teaching_profiles WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        let stage_str: String = row.get(0)?;
        let silenced: i64 = row.get(1)?;
        let counts_json: String = row.get(2)?;
        let cooldowns_json: String = row.get(3)?;
        let stage = match stage_str.as_str() {
            "guided" => crate::teaching::TutorialStage::Guided,
            "independent" => crate::teaching::TutorialStage::Independent,
            _ => crate::teaching::TutorialStage::Onboarding,
        };
        let mistake_counts = serde_json::from_str(&counts_json).unwrap_or_default();
        let cooldowns = serde_json::from_str(&cooldowns_json).unwrap_or_default();
        Ok(TeachingProfile {
            user_id: user_id.to_string(),
            stage,
            silenced: silenced != 0,
            mistake_counts,
            cooldowns,
        })
    } else {
        Ok(TeachingProfile::new(user_id))
    }
}

/// Upsert a teaching profile.
pub async fn upsert_teaching_profile(db: &Codex, profile: &TeachingProfile) -> Result<()> {
    let stage_str = match profile.stage {
        crate::teaching::TutorialStage::Onboarding => "onboarding",
        crate::teaching::TutorialStage::Guided => "guided",
        crate::teaching::TutorialStage::Independent => "independent",
    };
    let counts_json = serde_json::to_string(&profile.mistake_counts).unwrap_or_default();
    let cooldowns_json = serde_json::to_string(&profile.cooldowns).unwrap_or_default();
    let user_id = profile.user_id.clone();
    let stage_str = stage_str.to_string();
    let silenced_flag: i64 = if profile.silenced { 1 } else { 0 };
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_teaching_profiles (user_id, stage, silenced, mistake_counts, cooldowns)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(user_id) DO UPDATE SET
                    stage = excluded.stage,
                    silenced = excluded.silenced,
                    mistake_counts = excluded.mistake_counts,
                    cooldowns = excluded.cooldowns,
                    updated_at = datetime('now')",
                params![
                    user_id.as_str(),
                    stage_str.as_str(),
                    silenced_flag,
                    counts_json.as_str(),
                    cooldowns_json.as_str(),
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Insert a reward policy diagnostic snapshot.
#[allow(clippy::too_many_arguments)]
pub async fn insert_policy_snapshot(
    db: &Codex,
    user_id: &str,
    event_type: &str,
    base_xp: u64,
    base_crystals: u64,
    mode: &str,
    effective_multiplier: f64,
    xp_awarded: u64,
    crystals_awarded: u64,
    streak_days: u32,
    grind_capped: bool,
    lumens_awarded: i64,
    metadata: Option<&str>,
) -> Result<()> {
    let user_id = user_id.to_string();
    let event_type = event_type.to_string();
    let mode = mode.to_string();
    let metadata = metadata.map(|s| s.to_string());
    let grind: i64 = if grind_capped { 1 } else { 0 };
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_policy_snapshots
                 (user_id, event_type, base_xp, base_crystals, mode_label, effective_multiplier,
                  awarded_xp, awarded_crystals, streak_days, grind_capped, lumens, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    user_id.as_str(),
                    event_type.as_str(),
                    base_xp as i64,
                    base_crystals as i64,
                    mode.as_str(),
                    effective_multiplier,
                    xp_awarded as i64,
                    crystals_awarded as i64,
                    streak_days as i64,
                    grind,
                    lumens_awarded,
                    metadata
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
