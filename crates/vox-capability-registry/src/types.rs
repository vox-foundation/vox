//! Core capability registry types.

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
    /// Build a registry from descriptors (used by [`crate::default_registry`]).
    #[must_use]
    pub fn from_descriptors(caps: Vec<CapabilityDescriptor>) -> Self {
        Self { caps }
    }

    /// Capabilities eligible for Mens chat (subject to executor intersection).
    pub fn mens_chat_capabilities(&self) -> impl Iterator<Item = &CapabilityDescriptor> + '_ {
        self.caps
            .iter()
            .filter(|c| c.populi_exposure == PopuliExposure::Auto)
    }
}
