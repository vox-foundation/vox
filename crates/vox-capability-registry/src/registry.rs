//! Default [`CapabilityRegistry`](crate::CapabilityRegistry) construction.

use crate::types::{CapabilityDescriptor, CapabilityRegistry, InvocationForms, PopuliExposure};

/// Default registry: **Oratio** STT tools aligned with `vox-mcp` (`vox_oratio_*`).
#[must_use]
pub fn default_registry() -> CapabilityRegistry {
    CapabilityRegistry::from_descriptors(vec![
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
            CapabilityDescriptor {
                capability_id: "oratio.listen".into(),
                description: "Sessionized Oratio transcription with timeout/profile controls."
                    .into(),
                populi_exposure: PopuliExposure::Auto,
                invocation_forms: InvocationForms {
                    mcp_tool: Some("vox_oratio_listen".into()),
                },
            },
        ])
}
