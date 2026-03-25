//! Multi-agent conversation SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, SyntheticGenConfig};

include!("_multi_agent_body.rs");
