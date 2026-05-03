use crate::errors::SkillNotInstalledError;
use std::collections::HashMap;
use std::sync::RwLock;
use vox_plugin_api::skill::LoadedSkill;

#[derive(Default)]
pub struct SkillRegistry {
    skills: RwLock<HashMap<String, LoadedSkill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn install(&self, skill: LoadedSkill) {
        let mut w = self.skills.write().unwrap();
        w.insert(skill.plugin_id.clone(), skill);
    }
    pub fn lookup(&self, id: &str) -> Result<LoadedSkill, SkillNotInstalledError> {
        let r = self.skills.read().unwrap();
        r.get(id).cloned().ok_or(SkillNotInstalledError {
            skill_id: id.to_string(),
        })
    }
    pub fn list_ids(&self) -> Vec<String> {
        self.skills.read().unwrap().keys().cloned().collect()
    }
}
