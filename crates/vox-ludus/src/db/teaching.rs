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
            params![user_id.to_string()],
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

    db.connection().execute(
        "INSERT INTO gamify_teaching_profiles (user_id, stage, silenced, mistake_counts, cooldowns)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(user_id) DO UPDATE SET
            stage = excluded.stage,
            silenced = excluded.silenced,
            mistake_counts = excluded.mistake_counts,
            cooldowns = excluded.cooldowns,
            updated_at = datetime('now')",
        params![
            profile.user_id.clone(),
            stage_str.to_string(),
            if profile.silenced { 1i64 } else { 0i64 },
            counts_json,
            cooldowns_json,
        ],
    ).await?;
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
) -> Result<()> {
    db.connection().execute(
        "INSERT INTO gamify_policy_snapshots
         (user_id, event_type, base_xp, base_crystals, mode, effective_multiplier, xp_awarded, crystals_awarded, streak_days, grind_capped, lumens_awarded)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            user_id.to_string(),
            event_type.to_string(),
            base_xp as i64,
            base_crystals as i64,
            mode.to_string(),
            effective_multiplier,
            xp_awarded as i64,
            crystals_awarded as i64,
            streak_days as i64,
            if grind_capped { 1i64 } else { 0i64 },
            lumens_awarded,
        ],
    ).await?;
    Ok(())
}
