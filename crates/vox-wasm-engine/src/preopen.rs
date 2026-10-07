//! Preopen directory helpers for WASI sandboxing.
//!
//! Encapsulates the mapping from host paths + permission modes into the
//! `FsPerms` type required by `wasmtime_wasi::WasiCtxBuilder`.

use std::path::PathBuf;

/// Permission mode for a WASI preopen directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreopenMode {
    /// Guest may only read files; writes are rejected by the WASI sandbox.
    ReadOnly,
    /// Guest may read and write files.
    ReadWrite,
}

/// A single preopen directory mapping.
#[derive(Debug, Clone)]
pub struct Preopen {
    /// Host filesystem path to expose.
    pub host: PathBuf,
    /// Guest-visible path (e.g. `"."` or `"/data"`).
    pub guest: String,
    /// Permission mode applied to the preopen.
    pub mode: PreopenMode,
}

impl Preopen {
    /// Construct a read-only preopen.
    pub fn read_only(host: impl Into<PathBuf>, guest: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            guest: guest.into(),
            mode: PreopenMode::ReadOnly,
        }
    }

    /// Construct a read-write preopen.
    pub fn read_write(host: impl Into<PathBuf>, guest: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            guest: guest.into(),
            mode: PreopenMode::ReadWrite,
        }
    }

    /// Return the `FsPerms` for this preopen.
    pub(crate) fn wasi_perms(&self) -> wasmtime_wasi::FsPerms {
        match self.mode {
            PreopenMode::ReadOnly => wasmtime_wasi::FsPerms::ReadOnly,
            PreopenMode::ReadWrite => wasmtime_wasi::FsPerms::ReadWrite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Preopen;
    use wasmtime_wasi::FsPerms;

    #[test]
    fn read_only_preopen_maps_to_read_only_perms() {
        assert!(matches!(
            Preopen::read_only("/h", "/g").wasi_perms(),
            FsPerms::ReadOnly
        ));
    }

    #[test]
    fn read_write_preopen_maps_to_read_write_perms() {
        assert!(matches!(
            Preopen::read_write("/h", "/g").wasi_perms(),
            FsPerms::ReadWrite
        ));
    }
}
