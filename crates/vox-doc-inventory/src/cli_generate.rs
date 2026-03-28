//! Shared `main` logic for `doc-inventory-generate` and `vox-doc-inventory-generate` binaries.

use std::path::PathBuf;

/// Parse argv and write inventory JSON (default path or `--output <path>`).
pub fn run_generate_inventory_cli() -> anyhow::Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let mut args = std::env::args().skip(1);
    let out = match (args.next(), args.next(), args.next()) {
        (None, _, _) => root.join(crate::DEFAULT_INVENTORY_PATH),
        (Some(flag), Some(path), None) if flag == "--output" => {
            let p = PathBuf::from(path);
            if p.is_absolute() { p } else { root.join(p) }
        }
        _ => anyhow::bail!(
            "usage: vox-doc-inventory-generate | doc-inventory-generate [--output <path>]\n\
             default: {}",
            crate::DEFAULT_INVENTORY_PATH
        ),
    };
    crate::generate(&root, &out)?;
    println!("Wrote {}", out.display());
    Ok(())
}
