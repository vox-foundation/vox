use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub mod contracts;
pub mod data_ssot_guards;
pub mod grammar;
pub mod guards;
pub mod orchestration_audit;
pub mod syntax_k;

pub fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path) -> Result<()>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        let t = entry.file_type()?;
        if t.is_dir() {
            visit_rs_files(&p, f)?;
        } else if t.is_file() && p.extension().and_then(|x| x.to_str()) == Some("rs") {
            f(&p)?;
        }
    }
    Ok(())
}
