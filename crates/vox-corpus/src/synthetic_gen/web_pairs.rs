//! Web construct SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line, name_hash, rng::Rng};

include!("_web_pairs.inc");
