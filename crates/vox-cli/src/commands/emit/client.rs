//! `vox emit client` — same artifacts as `vox build --target=client`.

use crate::cli_args::EmitClientArgs;
use anyhow::Result;
use std::path::Path;

pub async fn run(args: &EmitClientArgs) -> Result<()> {
    crate::commands::build::run(
        Path::new(&args.file),
        &args.out_dir,
        args.mobile_target.clone(),
        Some(vox_config::BuildTarget::Client),
        false,
        args.emit_ir,
        crate::cli_args::BuildMode::Library,
    )
    .await
}
