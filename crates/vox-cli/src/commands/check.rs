//! `vox check` — type-check only (no files written except logs to stderr).

use anyhow::Result;
use std::path::Path;

/// Lex, parse, and type-check `file`; fail the process if any error-level diagnostic is reported.
pub async fn run(file: &Path, emit_training_jsonl: Option<&Path>) -> Result<()> {
    let result = crate::pipeline::run_frontend(file, false).await?;
    crate::pipeline::print_diagnostics(&result, file, false);
    let error_count = result.error_count();
    let warning_count = result.warning_count();

    if let Some(output_path) = emit_training_jsonl {
        crate::training::append_jsonl(output_path, file, &result)?;
        println!("Appended training record to {}", output_path.display());
    }

    if result.has_errors() {
        anyhow::bail!("Check failed with {error_count} error(s) and {warning_count} warning(s)");
    }

    println!("Check passed with {warning_count} warning(s)");
    Ok(())
}
