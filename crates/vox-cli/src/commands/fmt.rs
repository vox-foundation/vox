//! `vox fmt` — **not implemented** in the shipped binary. `vox-fmt` is out of sync with the current AST.

use anyhow::{bail, Context, Result};
use std::{fs, path};

/// Read `file` and validate paths; formatting is not applied until `vox-fmt` is rewired.
pub fn run(file: &path::Path, _check: bool) -> Result<()> {
    let _source = fs::read_to_string(file)
        .with_context(|| format!("Failed to read source file: {}", file.display()))?;

    bail!(
        "`vox fmt` is not wired to the current AST (the `vox-fmt` crate is behind the parser). \
See docs/src/ref-cli.md (Formatter / `vox fmt`)."
    );
}
