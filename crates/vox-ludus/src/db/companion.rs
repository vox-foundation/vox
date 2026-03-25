//! Companion persistence.

use anyhow::Result;
use vox_db::Codex;

use crate::companion::{Companion, Mood, Personality};

/// Load all companions for a user.
pub async fn list_companions(db: &Codex, user_id: &str) -> Result<Vec<Companion>> {
    let rows = db.list_gamify_companions(user_id).await?;
    let mut comps = Vec::new();

    for row in rows {
        let personality_str = row[14].as_deref().unwrap_or("focused");
        let personality = personality_str.parse::<Personality>().unwrap_or_default();

        comps.push(Companion {
            id: row[0].clone().unwrap_or_default(),
            user_id: row[1].clone().unwrap_or_else(|| user_id.to_string()),
            name: row[2].clone().unwrap_or_default(),
            description: row[3].clone(),
            code_hash: row[4].clone(),
            language: row[5].clone().unwrap_or_default(),
            ascii_sprite: row[6].clone(),
            mood: row[7]
                .as_deref()
                .unwrap_or("neutral")
                .parse::<Mood>()
                .unwrap_or(Mood::Neutral),
            health: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_health: row[9]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            energy: row[10]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_energy: row[11]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            code_quality: row[12]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(50) as u8,
            last_active: row[13]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            personality,
        });
    }

    Ok(comps)
}

/// Upsert a companion (includes personality JSON).
pub async fn upsert_companion(db: &Codex, c: &Companion) -> Result<()> {
    db.upsert_gamify_companion(
        &c.id,
        &c.user_id,
        &c.name,
        c.description.clone(),
        c.code_hash.clone(),
        &c.language,
        c.ascii_sprite.clone(),
        c.mood.as_str(),
        c.health as i64,
        c.max_health as i64,
        c.energy as i64,
        c.max_energy as i64,
        c.code_quality as i64,
        c.last_active,
        c.personality.as_str(),
    )
    .await?;
    Ok(())
}

/// Get a specific companion.
pub async fn get_companion(db: &Codex, id: &str) -> Result<Option<Companion>> {
    let row = db.get_gamify_companion(id).await?;
    if let Some(row) = row {
        let personality_str = row[13].as_deref().unwrap_or("focused");
        let personality = personality_str.parse::<Personality>().unwrap_or_default();

        Ok(Some(Companion {
            id: row[0].clone().unwrap_or_default(),
            user_id: row[1].clone().unwrap_or_default(),
            name: row[2].clone().unwrap_or_default(),
            description: row[3].clone(),
            code_hash: row[4].clone(),
            language: row[5].clone().unwrap_or_default(),
            ascii_sprite: row[6].clone(),
            mood: row[7]
                .as_deref()
                .unwrap_or("neutral")
                .parse::<Mood>()
                .unwrap_or(Mood::Neutral),
            health: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_health: row[9]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            energy: row[10]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_energy: row[11]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            code_quality: row[12]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(50) as u8,
            last_active: row[13]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            personality,
        }))
    } else {
        Ok(None)
    }
}

/// Delete a companion.
pub async fn delete_companion(db: &Codex, id: &str) -> Result<()> {
    db.delete_gamify_companion(id).await?;
    Ok(())
}
