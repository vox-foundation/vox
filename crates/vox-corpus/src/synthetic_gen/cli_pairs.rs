//! CLI command SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, SyntheticGenConfig, CLI_COMMANDS};

include!("_cli_pairs.inc");
