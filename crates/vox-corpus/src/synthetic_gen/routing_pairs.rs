//! Tool-chain, routing decisions, and expanded negative preference pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line, name_hash, rng::Rng};

include!("bodies/_routing_body.rs");
