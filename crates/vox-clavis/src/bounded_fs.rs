//! Capped UTF-8 reads with [`crate::errors::SecretError`] — delegates to [`vox_bounded_fs`].

use std::path::Path;

use crate::errors::SecretError;

pub(crate) fn read_utf8_path_capped(path: &Path) -> Result<String, SecretError> {
    vox_bounded_fs::read_utf8_path_capped(path)
        .map_err(|e| SecretError::Io(e.to_string()))
}

pub(crate) fn read_utf8_path_capped_opt(path: &Path) -> Option<String> {
    read_utf8_path_capped(path).ok()
}
