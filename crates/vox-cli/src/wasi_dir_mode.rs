//! WASI preopened directory mode (shared by script execution and HTTP execution API).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasiDirMode {
    ReadOnly,
    ReadWrite,
}
