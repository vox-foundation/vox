use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSchema {
    pub component_type: String,
    pub props: serde_json::Value,
}

pub trait Visualizable {
    fn generate_ui_schema(&self) -> UiSchema;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilityProvider {
    pub name: String,
    pub sdui_version: String,
    pub capabilities: Vec<String>,
}

impl SystemCapabilityProvider {
    pub fn new() -> Self {
        Self {
            name: "Vox SDUI Core".to_string(),
            sdui_version: "v1.0".to_string(),
            capabilities: vec![
                "workflow_scrubbing".to_string(),
                "gamification_hud".to_string(),
                "orchestrator_canvas".to_string(),
            ],
        }
    }

    pub fn broadcast_sdui_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "provider": self,
            "components": [
                {
                    "type": "intentions",
                    "available": true
                },
                {
                    "type": "financial_command",
                    "available": true
                }
            ]
        })
    }
}
