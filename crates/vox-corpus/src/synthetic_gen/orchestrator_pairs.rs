//! Skill + orchestrator command SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{
    emit_line, name_hash, rng::Rng, templates::TEMPLATES, tool_pairs::example_args_for_tool,
    SyntheticGenConfig, TOOL_REGISTRY_SLIM,
};

include!("_skill_orch_body.rs");
