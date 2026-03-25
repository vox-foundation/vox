//! Tool-chain, routing decisions, and expanded negative preference pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, name_hash, rng::Rng, SyntheticGenConfig};

include!("_routing_body.rs");
