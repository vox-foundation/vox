//! Capped UTF-8 reads returning [`std::io::Result`] for transport codepaths — delegates to [`vox_bounded_fs`].

use std::io;
use std::path::Path;

pub(crate) fn read_utf8_path_capped(path: &Path) -> io::Result<String> {
    vox_bounded_fs::read_utf8_path_capped(path).map_err(|e| io::Error::other(e.to_string()))
}
