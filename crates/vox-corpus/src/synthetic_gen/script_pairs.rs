//! Shell script SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line};

include!("bodies/_script_pairs.inc");
