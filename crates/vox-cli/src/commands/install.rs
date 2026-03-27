//! Retired `vox install` surface — package materialization is `vox sync`; declaration is `vox add`.

use anyhow::{Result, bail};

/// Deterministic migration error for hidden / legacy `vox install` invocations.
pub async fn run_retired(package_name: Option<String>) -> Result<()> {
    let hint = package_name
        .as_ref()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .map(|n| format!(" (you passed `{n}`)"))
        .unwrap_or_default();
    bail!(
        "`vox install` is retired{hint}.\n\
         • Declare dependencies: `vox add <name> [--version …] [--path …]`\n\
         • Resolve lockfile: `vox lock`\n\
         • Download packages: `vox sync`\n\
         • Registry workflows: `vox pm search|info|publish|verify|…`\n\
         See docs/src/reference/cli.md (package management section)."
    );
}
