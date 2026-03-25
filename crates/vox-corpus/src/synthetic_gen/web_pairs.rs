//! Web construct SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, name_hash, rng::Rng, SyntheticGenConfig};

include!("_web_pairs.inc");
