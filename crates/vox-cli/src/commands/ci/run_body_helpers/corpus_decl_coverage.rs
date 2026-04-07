//! `vox ci corpus-decl-coverage` — golden AST decl histogram for Mens maintainability.

use anyhow::Result;
use std::path::Path;

pub(crate) fn run_corpus_decl_coverage(root: &Path) -> Result<()> {
    let report = vox_corpus::corpus::decl_coverage::golden_decl_histogram(root)?;
    let out_dir = root.join("target/dogfood");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join("corpus_decl_coverage.json");
    std::fs::write(&out_path, serde_json::to_string_pretty(&report)?)?;
    eprintln!("Wrote {}", out_path.display());
    let fail = report["files_parse_fail"].as_u64().unwrap_or(0);
    if fail > 0 {
        anyhow::bail!(
            "corpus-decl-coverage: {} golden .vox file(s) failed to parse — fix syntax before training",
            fail
        );
    }
    vox_corpus::corpus::decl_coverage::assert_golden_decl_expectations(root, &report)?;
    Ok(())
}
