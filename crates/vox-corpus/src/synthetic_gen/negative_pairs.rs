//! Negative preference (rejection sampling) SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line};

include!("bodies/_negative_pairs.inc");
