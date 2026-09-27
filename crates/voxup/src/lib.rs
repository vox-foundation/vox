//! Library half of `voxup`, the Vox toolchain multiplexer and installer.
//!
//! `main.rs` compiles these same module files itself. This library target exposes
//! them to voxup's integration tests and to vox-cli (the plugin install and verify
//! commands):
//! - [`home`]: home-directory resolution that refuses to guess;
//! - [`profiles`]: the typed distribution-profile SSOT;
//! - [`install_plan`]: pure install planning, i.e. which binaries and bundle a tier selects;
//! - [`uninstall`]: allowlisted removal of installer-owned paths.

pub mod home;
pub mod install_plan;
pub mod profiles;
pub mod uninstall;
