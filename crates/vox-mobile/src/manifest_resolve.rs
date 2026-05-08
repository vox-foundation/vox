//! Reads `Vox.toml` from the project directory and validates it for mobile builds.

use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use vox_pm::manifest::{validate_mobile, VoxManifest};

pub fn load(project_dir: &Path) -> Result<VoxManifest> {
    let manifest_path = project_dir.join("Vox.toml");
    let toml_src = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: VoxManifest = toml::from_str(&toml_src)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let target = manifest.build.as_ref().and_then(|b| b.target.as_deref());
    if target != Some("mobile") {
        bail!(
            "expected [build] target = \"mobile\" in {}; got {:?}",
            manifest_path.display(),
            target
        );
    }

    if manifest.mobile.is_none() {
        bail!(
            "{} has [build] target = \"mobile\" but is missing the [mobile] section",
            manifest_path.display()
        );
    }

    validate_mobile(&manifest)
        .map_err(|e| anyhow!("validating [mobile] in {}: {e}", manifest_path.display()))?;
    Ok(manifest)
}
