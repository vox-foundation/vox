//! `vox test` — run inline tests declared in a Vox source file.

use anyhow::Result;
use std::path::Path;

/// Compile and run all `@test` functions in `file`, reporting pass/fail.
pub async fn run(file: &Path) -> Result<()> {
    crate::dispatch::call_daemon(
        "vox-compilerd",
        "test",
        serde_json::json!({ "file": file }),
        false,
    )
    .await?;
    Ok(())
}
