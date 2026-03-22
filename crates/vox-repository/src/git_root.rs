//! Locate a Git work tree by walking parents for `.git`.

use std::path::{Path, PathBuf};

/// Walk upward from `start` and return the directory that contains `.git`, if any.
pub fn find_git_work_tree(start: impl AsRef<Path>) -> Option<PathBuf> {
    let mut dir = start.as_ref().to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
