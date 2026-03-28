//! Regenerate `docs/agents/doc-inventory.json` without building the full `vox` binary.
//!
//! Run from repo root (either binary name is equivalent):
//! `cargo run -p vox-doc-inventory --bin vox-doc-inventory-generate`
//! (legacy: `--bin doc-inventory-generate`).
//!
//! When the default path is locked (e.g. IDE mmap on Windows), write elsewhere then replace:
//! `cargo run -p vox-doc-inventory --bin vox-doc-inventory-generate -- --output docs/agents/doc-inventory.gen.json`

fn main() -> anyhow::Result<()> {
    vox_doc_inventory::run_generate_inventory_cli()
}
