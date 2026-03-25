//! Agent definition + lifecycle SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line, name_hash, rng::Rng, templates::TEMPLATES};

include!("_agent_def_pairs.inc");
include!("_agent_lifecycle_pairs.inc");
