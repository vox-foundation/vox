//! Capability registry for **Mens chat** tool surfaces.
//!
//! Entries with [`PopuliExposure::Auto`] are candidates for advertisement to LLM tool-calling
//! clients; callers must still intersect with in-process executors (e.g. `vox_tools::DirectToolExecutor`).
//! The `vox_tools::mens_chat` module builds OpenAI-style tool definitions from this registry ∩ executor.

/// Whether a capability is exposed to Mens chat tool lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopuliExposure {
    /// Advertise when an in-process executor also implements the MCP tool name.
    Auto,
    /// Never advertise to chat.
    Off,
}

/// How to invoke a capability (MCP name, etc.).
#[derive(Debug, Clone)]
pub struct InvocationForms {
    /// MCP tool id (e.g. `vox_oratio_transcribe`).
    pub mcp_tool: Option<String>,
}

/// One logical capability (may map to an MCP tool name).
#[derive(Debug, Clone)]
pub struct CapabilityDescriptor {
    /// Stable id for parameters lookup (e.g. `oratio.transcribe`).
    pub capability_id: String,
    /// Human-readable description for LLM tool lists.
    pub description: String,
    /// Chat exposure policy.
    pub populi_exposure: PopuliExposure,
    /// Invocation mapping (MCP name, …).
    pub invocation_forms: InvocationForms,
}

/// Full registry (extend with new capabilities as executors gain parity).
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    caps: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistry {
    /// Capabilities eligible for Mens chat (subject to executor intersection).
    pub fn mens_chat_capabilities(&self) -> impl Iterator<Item = &CapabilityDescriptor> + '_ {
        self.caps
            .iter()
            .filter(|c| c.populi_exposure == PopuliExposure::Auto)
    }
}

/// Default registry: **Oratio** STT tools aligned with `vox-mcp` (`vox_oratio_*`).
#[must_use]
pub fn default_registry() -> CapabilityRegistry {
    CapabilityRegistry {
        caps: vec![
            CapabilityDescriptor {
                capability_id: "oratio.transcribe".into(),
                description: "Transcribe audio to text via Vox Oratio (Candle Whisper). Arg: path (workspace-relative or absolute)."
                    .into(),
                populi_exposure: PopuliExposure::Auto,
                invocation_forms: InvocationForms {
                    mcp_tool: Some("vox_oratio_transcribe".into()),
                },
            },
            CapabilityDescriptor {
                capability_id: "oratio.status".into(),
                description:
                    "Oratio / Candle Whisper backend status and default model env (JSON)."
                        .into(),
                populi_exposure: PopuliExposure::Auto,
                invocation_forms: InvocationForms {
                    mcp_tool: Some("vox_oratio_status".into()),
                },
            },
        ],
    }
}

/// JSON Schema `parameters` for OpenAI-style tool calling (`type` must be `"object"`).
#[must_use]
pub fn mens_chat_parameters(capability_id: &str) -> serde_json::Value {
    match capability_id {
        "oratio.transcribe" => serde_json::from_str(
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Workspace-relative or absolute path to an audio file"}},"required":["path"]}"#,
        )
        .unwrap_or_else(|_| serde_json::json!({"type":"object"})),
        "oratio.status" => serde_json::from_str(
            r#"{"type":"object","additionalProperties":false}"#,
        )
        .unwrap_or_else(|_| serde_json::json!({"type":"object"})),
        _ => serde_json::json!({"type":"object"}),
    }
}

/// Build one OpenAI-compatible tool definition entry.
#[must_use]
pub fn capability_to_openai_function(
    name: &str,
    description: &str,
    parameters: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters,
        }
    })
}
