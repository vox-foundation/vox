//! Tool-call SFT pair generation.

use std::io::Write;

use serde_json::{Value, json};

use super::{
    SyntheticGenConfig, emit_tool_pair, name_hash, rng::Rng, templates::TEMPLATES,
};

include!("bodies/_tool_pairs_body.rs");
