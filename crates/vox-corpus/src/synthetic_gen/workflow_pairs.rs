//! Workflow construct SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, name_hash, rng::Rng, templates::TEMPLATES, SyntheticGenConfig};

include!("_workflow_body.rs");
