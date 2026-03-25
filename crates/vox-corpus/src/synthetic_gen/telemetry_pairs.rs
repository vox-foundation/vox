//! Telemetry interpretation SFT pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, SyntheticGenConfig};

include!("_telemetry_pairs.inc");
