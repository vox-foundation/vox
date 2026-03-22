//! `vox profile` — measure build + start latency for a Vox source file.

use anyhow::Result;
use std::path::Path;

/// Profile lex → parse → typecheck → codegen → startup for `file`.
///
/// When `json` is true, emits a machine-readable JSON timing object.
/// When `no_cache` is true, discards any cached artifacts before measuring.
pub async fn run(file: &Path, json: bool, no_cache: bool) -> Result<()> {
    crate::dispatch::call_daemon(
        "vox-compilerd",
        "profile",
        serde_json::json!({
            "file": file,
            "json": json,
            "no_cache": no_cache,
        }),
        false,
    )
    .await?;
    Ok(())
}
