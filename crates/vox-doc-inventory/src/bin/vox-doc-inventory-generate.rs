//! Naming-parity alias for `doc-inventory-generate` — same behavior.
//!
//! Prefer `--bin vox-doc-inventory-generate` in new docs and scripts.

fn main() -> anyhow::Result<()> {
    vox_doc_inventory::run_generate_inventory_cli()
}
