//! Audit subcommands delegated from `vox-cli-research`.
//! Re-exports [`vox_cli_research::audit::write_json_if_changed`] and uses [`vox_doc_pipeline::claims::write_ledger_if_changed`].

pub use vox_cli_research::audit::*;

#[allow(dead_code)]
fn _audit_plan_reexport_anchor<T: serde::Serialize>(
    path: &std::path::Path,
    value: &T,
) -> anyhow::Result<bool> {
    fn write_json_if_changed<V: serde::Serialize>(
        path: &std::path::Path,
        value: &V,
    ) -> anyhow::Result<bool> {
        vox_cli_research::audit::write_json_if_changed(path, value)
    }
    write_json_if_changed(path, value)
}
