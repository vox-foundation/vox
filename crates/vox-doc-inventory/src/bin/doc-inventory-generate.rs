//! Regenerate `docs/agents/doc-inventory.json` without building the full `vox` binary.
//!
//! Run from repo root: `cargo run -p vox-doc-inventory --bin doc-inventory-generate`
//!
//! When the default path is locked (e.g. IDE mmap on Windows), write elsewhere then replace:
//! `cargo run -p vox-doc-inventory --bin doc-inventory-generate -- --output docs/agents/doc-inventory.gen.json`

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let mut args = std::env::args().skip(1);
    let out = match (args.next(), args.next(), args.next()) {
        (None, _, _) => root.join(vox_doc_inventory::DEFAULT_INVENTORY_PATH),
        (Some(flag), Some(path), None) if flag == "--output" => {
            let p = PathBuf::from(path);
            if p.is_absolute() { p } else { root.join(p) }
        }
        _ => anyhow::bail!(
            "usage: doc-inventory-generate [--output <path>]\n\
             default: {}",
            vox_doc_inventory::DEFAULT_INVENTORY_PATH
        ),
    };
    vox_doc_inventory::generate(&root, &out)?;
    println!("Wrote {}", out.display());
    Ok(())
}
