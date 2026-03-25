//! Tool-call SFT pair generation.

use std::io::Write;

use serde_json::{Value, json};

use super::{
    emit_tool_pair, name_hash, rng::Rng, templates::TEMPLATES, EXAMPLE_TASKS, SyntheticGenConfig,
};

include!("_tool_pairs_body.rs");
