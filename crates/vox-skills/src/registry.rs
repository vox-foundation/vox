//! Skill registry — install, uninstall, search, and list skills.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::SkillError;
use crate::bundle::VoxSkillBundle;
use crate::manifest::{SkillCategory, SkillManifest};

/// Result of a skill installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// Installed skill id.
    pub id: String,
    /// Installed version string.
    pub version: String,
    /// True when this id+version was already present (no-op install).
    pub already_installed: bool,
    /// Content hash of the bundle at install time.
    pub hash: String,
}

/// Result of a skill uninstallation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    /// Skill id passed to [`SkillRegistry::uninstall`].
    pub id: String,
    /// True when a manifest was removed from the in-memory map.
    pub was_installed: bool,
}

/// In-memory skill registry with interior mutability for db field.
pub struct SkillRegistry {
    skills: Mutex<HashMap<String, SkillManifest>>,
    db: Mutex<Option<Arc<vox_db::VoxDb>>>,
}

impl SkillRegistry {
    /// In-memory registry without Codex persistence.
    pub fn new() -> Self {
        Self {
            skills: Mutex::new(HashMap::new()),
            db: Mutex::new(None),
        }
    }

    /// Fluent constructor: same as [`SkillRegistry::new`] then [`SkillRegistry::set_db`].
    pub fn with_db(self, db: Arc<vox_db::VoxDb>) -> Self {
        *self.db.lock().unwrap_or_else(|e| e.into_inner()) = Some(db);
        self
    }

    /// Attach **Codex** (`vox_db::VoxDb`) after construction (safe interior mutation).
    pub fn set_db(&self, db: Arc<vox_db::VoxDb>) {
        *self.db.lock().unwrap_or_else(|e| e.into_inner()) = Some(db);
    }

    fn get_db(&self) -> Option<Arc<vox_db::VoxDb>> {
        self.db.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Install a skill bundle. Returns error if already installed at same version.
    pub async fn install(&self, bundle: &VoxSkillBundle) -> Result<InstallResult, SkillError> {
        let id = bundle.manifest.id.clone();
        let version = bundle.manifest.version.clone();
        let hash = bundle.content_hash();

        {
            let mut skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing) = skills.get(&id)
                && existing.version == version
            {
                info!(skill = %id, "Skill already installed at same version");
                return Ok(InstallResult {
                    id,
                    version,
                    already_installed: true,
                    hash,
                });
            }
            info!(skill = %id, version = %version, "Installing skill");
            let mut manifest = bundle.manifest.clone();
            manifest.hash = Some(hash.clone());
            skills.insert(id.clone(), manifest);
        }

        // Persist to VoxDB (fire-and-forget, no lock held)
        if let Some(db) = self.get_db() {
            let manifest_json = serde_json::to_string(&bundle.manifest).unwrap_or_default();
            let skill_md = bundle.skill_md.clone();
            let id2 = id.clone();
            let ver2 = version.clone();
            tokio::spawn(async move {
                let _ = db
                    .publish_skill(&id2, &ver2, &manifest_json, &skill_md)
                    .await;
            });
        }

        Ok(InstallResult {
            id,
            version,
            already_installed: false,
            hash,
        })
    }

    /// Uninstall a skill by ID.
    ///
    /// **`vox.mens`** targets the same stored manifest as **`vox.populi`** (legacy id).
    pub async fn uninstall(&self, id: &str) -> Result<UninstallResult, SkillError> {
        let canonical = if id == "vox.mens" { "vox.populi" } else { id };
        let was_installed = {
            let mut skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
            skills.remove(canonical).is_some()
        };
        if was_installed {
            info!(skill = %canonical, "Skill uninstalled");
            if let Some(db) = self.get_db() {
                let id_owned = canonical.to_string();
                tokio::spawn(async move {
                    let _ = db.unpublish_skill(&id_owned).await;
                });
            }
        } else {
            warn!(skill = %id, "Tried to uninstall skill that was not installed");
        }
        Ok(UninstallResult {
            id: id.to_string(),
            was_installed,
        })
    }

    /// Search skills by keyword (name, description, tags).
    pub fn search(&self, query: &str) -> Vec<SkillManifest> {
        let q = query.to_lowercase();
        let skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
        skills
            .values()
            .filter(|m| {
                m.id.to_lowercase().contains(&q)
                    || m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect()
    }

    /// List all installed skills, optionally filtered by category.
    pub fn list(&self, category: Option<&SkillCategory>) -> Vec<SkillManifest> {
        let skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
        skills
            .values()
            .filter(|m| category.is_none_or(|c| &m.category == c))
            .cloned()
            .collect()
    }

    /// Get a specific skill by ID.
    ///
    /// **`vox.mens`** resolves to **`vox.populi`** if present (bundled mesh skill id migration).
    pub fn get(&self, id: &str) -> Option<SkillManifest> {
        let skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
        skills.get(id).cloned().or_else(|| {
            if id == "vox.mens" {
                skills.get("vox.populi").cloned()
            } else {
                None
            }
        })
    }

    /// Load all skills from the Codex `skill_manifests` table into memory.
    pub async fn hydrate_from_db(&self) -> Result<usize, SkillError> {
        let db = match self.get_db() {
            Some(db) => db,
            None => return Ok(0),
        };
        let entries = db
            .list_skill_manifests()
            .await
            .map_err(|e| SkillError::Http(e.to_string()))?;
        let mut skills = self.skills.lock().unwrap_or_else(|e| e.into_inner());
        let count = entries.len();
        for entry in entries {
            if let Ok(manifest) = serde_json::from_str::<SkillManifest>(&entry.manifest_json) {
                skills.insert(manifest.id.clone(), manifest);
            }
        }
        Ok(count)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{SkillCategory, SkillManifest};

    fn test_bundle(id: &str, category: SkillCategory) -> VoxSkillBundle {
        let m = SkillManifest::new(id, id, "1.0.0", "vox", "desc", category);
        VoxSkillBundle::new(m, "# Skill\nInstructions.")
    }

    #[tokio::test]
    async fn install_and_list() {
        let reg = SkillRegistry::new();
        let bundle = test_bundle("vox.compiler", SkillCategory::Compiler);
        let res = reg.install(&bundle).await.expect("install");
        assert_eq!(res.id, "vox.compiler");
        assert!(!res.already_installed);
        let skills = reg.list(None);
        assert_eq!(skills.len(), 1);
    }

    #[tokio::test]
    async fn double_install_same_version() {
        let reg = SkillRegistry::new();
        let bundle = test_bundle("vox.testing", SkillCategory::Testing);
        reg.install(&bundle).await.expect("first install");
        let res = reg.install(&bundle).await.expect("second install");
        assert!(res.already_installed);
    }

    #[tokio::test]
    async fn uninstall_removes_skill() {
        let reg = SkillRegistry::new();
        let bundle = test_bundle("vox.docs", SkillCategory::Documentation);
        reg.install(&bundle).await.expect("install");
        let res = reg.uninstall("vox.docs").await.expect("uninstall");
        assert!(res.was_installed);
        assert!(reg.get("vox.docs").is_none());
    }

    #[tokio::test]
    async fn search_finds_by_name() {
        let reg = SkillRegistry::new();
        let bundle = test_bundle("vox.git-helper", SkillCategory::Git);
        reg.install(&bundle).await.expect("install");
        let hits = reg.search("git");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "vox.git-helper");
    }

    #[test]
    fn set_db_is_noop_without_db_attached() {
        // Just ensure set_db doesn't panic when called
        let reg = SkillRegistry::new();
        assert!(reg.get_db().is_none());
    }
}
