//! Generated project manifest types and path helpers (OP-0209).

use std::collections::HashMap;
use std::path::Path;

/// `path` value for a generated Cargo.toml `[dependencies]` entry.
///
/// On Windows, strips `\\?\` from canonical paths so Cargo accepts the literal (avoids `//?/C:/...`).
pub(crate) fn manifest_dependency_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        let rest = if let Some(r) = s.strip_prefix(r"\\?\") {
            r.to_string()
        } else {
            s.to_string()
        };
        let normalized = rest.replace('\\', "/");
        if let Some(unc) = normalized.strip_prefix("UNC/") {
            format!("//{unc}")
        } else {
            normalized
        }
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().replace('\\', "/")
    }
}

/// Output of code generation: a map of `filename -> content`.
#[derive(Debug)]
pub struct CodegenOutput {
    pub files: HashMap<String, String>,
    /// TypeScript API client for server functions (empty if no server fns).
    pub api_client_ts: String,
}

impl CodegenOutput {
    /// Write all generated files to the target directory.
    ///
    /// **Incremental Diffing:** Only writes a file if its local content differs
    /// from the existing file on disk. This preserves the file's modification
    /// time (mtime), which prevents Cargo from performing a redundant full rebuild
    /// when the generated code remains identical after a `vox run` re-eval.
    pub fn write_to_dir(&self, target_dir: &Path) -> std::io::Result<()> {
        for (filename, content) in &self.files {
            let path = target_dir.join(filename);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let needs_write = if path.exists() {
                let existing = std::fs::read_to_string(&path).ok();
                existing.as_ref() != Some(content)
            } else {
                true
            };

            if needs_write {
                std::fs::write(&path, content)?;
            }
        }
        Ok(())
    }
}
