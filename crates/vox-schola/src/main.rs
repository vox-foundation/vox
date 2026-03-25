//! `vox-schola` — standalone GPU-native binary for Vox ML.
//!
//! Equivalent to `vox mens …` but links only to `vox-populi` (Mens via `mens` features).
//! No compiler stack (lexer/parser/hir/codegen) is compiled in.
//! Incremental build target: ~15 s vs ~45 s for the full `vox` binary.
//!
//! ## Quick start
//!
//! ```text
//! vox-schola                                  # defaults to `train`
//! vox-schola train --model Qwen/Qwen2.5-Coder-1.5B-Instruct
//! vox-schola serve --model mens/runs/latest --port 8080
//! vox-schola probe
//! vox-schola status
//! ```

mod cli;
mod merge;
mod probe;
mod serve;
mod status;
mod train;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .compact()
        .init();

    cli::run().await
}
