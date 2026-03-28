//! UTF-8 file reads capped by embedded scaling policy — delegates to [`vox_bounded_fs`].

use std::path::Path;

/// Reads the entire file as UTF-8 when valid and within [`vox_bounded_fs::max_file_bytes_hint`].
pub(crate) fn read_utf8_file_capped(path: &Path) -> Option<String> {
    vox_bounded_fs::read_utf8_path_capped_opt(path)
}
