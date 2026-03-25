//! Error → recovery training pairs.

use std::io::Write;

use serde_json::json;

use super::{emit_line, SyntheticGenConfig};

include!("_err_recovery_body.rs");
