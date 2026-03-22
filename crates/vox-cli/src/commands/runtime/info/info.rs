//! `vox info` in the runtime command tree — delegates to [`crate::commands::info`] (SSOT).

use anyhow::Result;

/// Display registry (or local store) metadata for `package_name`.
pub async fn run(package_name: &str, registry_url: Option<&str>) -> Result<()> {
    crate::commands::info::run(package_name, registry_url).await
}
