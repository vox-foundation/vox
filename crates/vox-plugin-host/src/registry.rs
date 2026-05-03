use crate::skill_registry::SkillRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use vox_plugin_api::manifest::PluginPayload;

#[derive(Clone)]
pub struct PluginEntry {
    pub id: String,
    pub version: String,
    pub install_dir: PathBuf,
    pub payload: PluginPayload,
}

pub struct Registry {
    entries: RwLock<HashMap<String, PluginEntry>>,
    pub skills: SkillRegistry,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            skills: SkillRegistry::new(),
        }
    }
    pub fn record(&self, entry: PluginEntry) {
        self.entries
            .write()
            .unwrap()
            .insert(entry.id.clone(), entry);
    }
    pub fn list_ids(&self) -> Vec<String> {
        self.entries.read().unwrap().keys().cloned().collect()
    }
    pub fn has(&self, id: &str) -> bool {
        self.entries.read().unwrap().contains_key(id)
    }

    /// Return a clone of the full [`PluginEntry`] for `id`, or `None` if not registered.
    pub fn get_full_entry(&self, id: &str) -> Option<PluginEntry> {
        self.entries.read().unwrap().get(id).cloned()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
