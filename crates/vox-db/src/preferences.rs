//! User preference and registry-specific settings facade.
//!
//! Wraps the Arca `user_preferences` table with higher-level registry scoping.
//! Key format: `{registry}.{key}`.

use crate::store::StoreError;

/// Set a preference for a specific registry (e.g. `google.api_key`).
pub async fn set_registry_preference(
    registry: &str,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    let db = crate::VoxDb::connect_default().await?;
    let full_key = format!("{}.{}", registry, key);
    db.set_user_preference("local_user", &full_key, value).await
}

/// Read a preference for a specific registry.
pub async fn get_registry_preference(
    registry: &str,
    key: &str,
) -> Result<Option<String>, StoreError> {
    let db = crate::VoxDb::connect_default().await?;
    let full_key = format!("{}.{}", registry, key);
    db.get_user_preference("local_user", &full_key).await
}

/// Reset all preferences for a given registry (DESTRUCTIVE).
pub async fn reset_registry_preferences(registry: &str) -> Result<(), StoreError> {
    let db = crate::VoxDb::connect_default().await?;
    let prefix = format!("{}.", registry);
    db
        .connection()
        .execute(
            "DELETE FROM user_preferences WHERE user_id = 'local_user' AND key LIKE ?1",
            (format!("{}%", prefix),),
        )
        .await?;
    Ok(())
}

/// All `(key, value)` pairs for a registry, with the prefix stripped.
pub async fn get_all_registry_preferences(
    registry: &str,
) -> Result<Vec<(String, String)>, StoreError> {
    let db = crate::VoxDb::connect_default().await?;
    let prefix = format!("{}.", registry);
    
    // Efficiently fetch only keys matching the registry prefix
    let matching = db.list_user_preferences("local_user", Some(&prefix)).await?;
    
    let mut out = Vec::new();
    for (k, v) in matching {
        if let Some(stripped) = k.strip_prefix(&prefix) {
            out.push((stripped.to_string(), v));
        }
    }
    Ok(out)
}
