//! CLI command SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{CLI_COMMANDS, SyntheticGenConfig, emit_line};

include!("bodies/_cli_pairs.inc");
