//! Pick a vision-capable model: first config-preference the registry marks
//! supports_vision; else (or with no registry at all) fall back to
//! preference\[0\].

pub trait VisionCatalog {
    fn supports_vision(&self, model_id: &str) -> Option<bool>;
}

pub fn choose_vision_model(preference: &[String], catalog: Option<&dyn VisionCatalog>) -> String {
    if let Some(catalog) = catalog {
        for m in preference {
            if catalog.supports_vision(m) == Some(true) {
                return m.clone();
            }
        }
    }
    preference.first().cloned().unwrap_or_else(|| {
        // Contract role `visual_review` (model-defaults.v1.yaml).
        vox_config::model_defaults::VISUAL_REVIEW[0].to_string()
    })
}

/// Registry-backed catalog. `ModelRegistry` is re-exported at
/// `vox_orchestrator::models::ModelRegistry`; `get` returns an owned
/// `Option<ModelSpec>`.
pub struct RegistryCatalog<'a>(pub &'a vox_orchestrator::models::ModelRegistry);
impl<'a> VisionCatalog for RegistryCatalog<'a> {
    fn supports_vision(&self, model_id: &str) -> Option<bool> {
        self.0
            .get(model_id)
            .map(|spec| spec.capabilities.supports_vision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    struct Fake(HashMap<String, bool>);
    impl VisionCatalog for Fake {
        fn supports_vision(&self, m: &str) -> Option<bool> {
            self.0.get(m).copied()
        }
    }
    #[test]
    fn picks_first_vision_capable() {
        let c = Fake(HashMap::from([("a".into(), false), ("b".into(), true)]));
        assert_eq!(
            choose_vision_model(&["a".into(), "b".into()], Some(&c)),
            "b"
        );
    }
    #[test]
    fn falls_back_to_first_when_registry_silent() {
        let silent = Fake(HashMap::new());
        assert_eq!(
            choose_vision_model(&["x".into(), "y".into()], Some(&silent)),
            "x"
        );
        assert_eq!(choose_vision_model(&["x".into(), "y".into()], None), "x");
    }
    #[test]
    fn empty_preference_falls_back_to_the_contract_default() {
        assert_eq!(
            choose_vision_model(&[], None),
            vox_config::model_defaults::VISUAL_REVIEW[0]
        );
    }
}
