//! Default [`CapabilityRegistry`](crate::CapabilityRegistry) from embedded SSOT YAML.

use crate::document::CapabilityRegistryDoc;
use crate::types::{CapabilityDescriptor, CapabilityRegistry, InvocationForms, PopuliExposure};

const EMBEDDED_REGISTRY_YAML: &str =
    include_str!("../../../contracts/capability/capability-registry.yaml");

/// Parse the embedded [`CapabilityRegistryDoc`](crate::document::CapabilityRegistryDoc).
#[must_use]
pub fn bundled_document() -> CapabilityRegistryDoc {
    serde_yaml::from_str(EMBEDDED_REGISTRY_YAML)
        .expect("embedded contracts/capability/capability-registry.yaml must parse")
}

/// Build the Mens-chat-oriented registry from embedded YAML (curated rows with `mcp_tool` and planner visibility).
#[must_use]
pub fn default_registry() -> CapabilityRegistry {
    let doc = bundled_document();
    registry_from_document(&doc)
}

#[must_use]
pub fn registry_from_document(doc: &CapabilityRegistryDoc) -> CapabilityRegistry {
    let mut descriptors: Vec<CapabilityDescriptor> = Vec::new();
    for row in &doc.curated {
        let Some(ref tool) = row.mcp_tool else {
            continue;
        };
        if row.mens_planner_visible == Some(false) {
            continue;
        }
        let desc_text = row
            .description_model
            .as_deref()
            .or(row.description_human.as_deref())
            .unwrap_or("Vox MCP tool");
        descriptors.push(CapabilityDescriptor {
            capability_id: row.id.clone(),
            description: desc_text.to_string(),
            populi_exposure: PopuliExposure::Auto,
            invocation_forms: InvocationForms {
                mcp_tool: Some(tool.clone()),
            },
        });
    }
    CapabilityRegistry::from_descriptors(descriptors)
}
