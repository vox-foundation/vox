//! Teaching profiles and policy snapshots.

use anyhow::Result;
use vox_db::Codex;

use crate::teaching::TeachingProfile;

/// Load a teaching profile. Returns a fresh default if none exists yet.
pub async fn get_teaching_profile(db: &Codex, user_id: &str) -> Result<TeachingProfile> {
    let row = db.get_gamify_teaching_profile_row(user_id).await?;
    if let Some((stage_str, silenced, counts_json, cooldowns_json)) = row {
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

    db.upsert_gamify_teaching_profile(
        profile.user_id.as_str(),
        stage_str,
        profile.silenced,
        counts_json.as_str(),
        cooldowns_json.as_str(),
    )
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
    db.insert_gamify_policy_snapshot(
        user_id,
        event_type,
        base_xp as i64,
        base_crystals as i64,
        mode,
        effective_multiplier,
        xp_awarded as i64,
        crystals_awarded as i64,
        streak_days as i64,
        grind_capped,
        lumens_awarded,
        metadata,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
