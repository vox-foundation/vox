//! Extras: marketplace snippets, share, optional ARS skill surface, optional Ludus.
//!
//! The historical in-tree **dashboard** HTTP handlers depended on the unwired `vox-dei` module tree; the supported
//! path is `vox-codex-api` / `vox dash` (see `docs/src/reference/cli.md` and `vox-codex-api`). Legacy handler
//! sources were removed from the tree; do not add `vox-dei` as a `vox-cli` dependency (use `crate::dei_daemon` + `vox-dei-d`).

#[cfg(feature = "ars")]
pub mod ars;
pub mod share;
pub mod share_cli;
#[cfg(feature = "ars")]
pub mod skill_cmd;
pub mod snippet;
pub mod snippet_cli;

#[cfg(feature = "extras-ludus")]
pub mod ludus;
#[cfg(feature = "extras-ludus")]
pub mod ludus_cli;
