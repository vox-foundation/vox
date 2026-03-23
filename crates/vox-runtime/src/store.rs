use serde::{de::DeserializeOwned, Serialize};

/// A SQLite-backed KV store for Actor state.
///
/// When the `database` crate feature is enabled and the global `VoxDb` is initialised,
/// persists to `actor_state` via [`vox_pm::store::CodeStore::save_actor_state`]/
/// [`load_actor_state`]/[`delete_actor_state`] (schema V22).
pub struct StateStore;

impl StateStore {
    pub async fn save<T: Serialize>(key: &str, value: &T) -> Result<(), std::io::Error> {
        let db = crate::db::get_db().await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        db.save_actor_state(key, value).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    pub async fn load<T: DeserializeOwned>(key: &str) -> Result<Option<T>, std::io::Error> {
        let db = crate::db::get_db().await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        db.load_actor_state(key).await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }
}
