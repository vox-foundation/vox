//! A2A SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{
    A2A_MESSAGE_TYPES, EXAMPLE_AGENT_PAIRS, SyntheticGenConfig, emit_line, name_hash, rng::Rng,
    templates::TEMPLATES,
};

include!("_a2a_pairs_body.rs");
