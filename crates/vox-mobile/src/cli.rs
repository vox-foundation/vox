//! Clap argument structures for the `vox-mobile` binary.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "vox-mobile",
    version,
    about = "Vox mobile build plugin: cross-compile for Android and iOS"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Inspect the local toolchain (cargo-ndk, NDK, rustup targets, Xcode CLT).
    Doctor,
    /// Cross-compile the current Vox project for a mobile platform.
    Build {
        /// Target platform: android, ios, or all (default).
        #[arg(long, default_value = "all")]
        platform: String,
        /// Build in release mode.
        #[arg(long)]
        release: bool,
    },
}
