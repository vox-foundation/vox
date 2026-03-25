//! Multi-agent conversation SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line};

include!("_multi_agent_body.rs");
