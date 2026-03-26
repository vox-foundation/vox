//! Error → recovery training pairs.

use std::io::Write;

use serde_json::json;

use super::{SyntheticGenConfig, emit_line};

include!("bodies/_err_recovery_body.rs");
