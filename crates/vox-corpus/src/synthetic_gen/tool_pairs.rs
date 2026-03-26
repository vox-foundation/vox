//! Tool-call SFT pair generation.

use std::io::Write;

use serde_json::{Value, json};

use super::{
    EXAMPLE_TASKS, SyntheticGenConfig, emit_tool_pair, name_hash, rng::Rng, templates::TEMPLATES,
};

include!("bodies/_tool_pairs_body.rs");
