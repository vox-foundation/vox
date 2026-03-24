//! Capability registry for Populi chat tools.
//!
//! Ported from `vox-capability-registry` for SSOT in `vox-tools`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Descriptor for a single tool or capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub description: String,
    pub invocation_forms: InvocationForms,
}

/// Various ways a capability can be called.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvocationForms {
    pub mcp_tool: Option<String>,
    pub vox_command: Option<String>,
}

/// Registry of all available capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityRegistry {
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn populi_chat_capabilities(&self) -> impl Iterator<Item = &CapabilityDescriptor> {
        self.capabilities.iter()
    }
}

/// Return the default workspace registry.
pub fn default_registry() -> CapabilityRegistry {
    CapabilityRegistry {
        capabilities: vec![
            CapabilityDescriptor {
                capability_id: "vox_oratio_transcribe".to_string(),
                description: "Transcribe audio files to text".to_string(),
                invocation_forms: InvocationForms {
                    mcp_tool: Some("vox_oratio_transcribe".to_string()),
                    ..Default::default()
                },
            },
        ],
    }
}

/// Get JSON parameters for a specific capability ID.
pub fn populi_chat_parameters(_id: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" }
        },
        "required": ["path"]
    })
}

/// Convert a capability to OpenAI function format.
pub fn capability_to_openai_function(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "parameters": parameters
    })
}
