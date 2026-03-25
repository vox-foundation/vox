//! `vox install` — **not implemented** in the shipped binary; registry flows belong in `vox-pm`.

use anyhow::{Result, bail};

/// Refuse with a clear message until registry install is wired.
pub async fn run(package_name: Option<&str>, _offline: bool) -> Result<()> {
    let name = package_name.unwrap_or("(none)");
    bail!(
        "`vox install` does not download packages yet (requested: `{name}`). \
Registry install is tracked for `vox-pm`; see docs/src/ref-cli.md (`vox install`)."
    );
}
