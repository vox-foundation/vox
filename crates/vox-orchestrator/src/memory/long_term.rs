use std::fs;
use std::path::{Path, PathBuf};

use super::error::MemoryError;

/// Manages `MEMORY.md` — curated, human-editable long-term knowledge.
///
/// Sections are Markdown headings (`## key`). Each section contains free-form
/// text. [`LongTermMemory::get`] extracts the body under a heading; [`LongTermMemory::set`] upserts it.
pub struct LongTermMemory {
    path: PathBuf,
}

impl LongTermMemory {
    /// Open (or create) the MEMORY.md file.
    pub fn open(path: &Path) -> Result<Self, MemoryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(MemoryError::Io)?;
        }
        if !path.exists() {
            fs::write(
                path,
                "# Vox Long-Term Memory\n\nThis file is managed by the Vox orchestrator. Edit freely.\n\n",
            )
            .map_err(MemoryError::Io)?;
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Read all contents.
    pub fn read_all(&self) -> Result<String, MemoryError> {
        crate::bounded_fs::read_utf8_path_capped(&self.path).map_err(|e| {
            MemoryError::Io(std::io::Error::other(e.to_string()))
        })
    }

    /// Extract the body text under a `## key` heading.
    pub fn get(&self, key: &str) -> Result<Option<String>, MemoryError> {
        let content = self.read_all()?;
        let heading = format!("## {key}");
        let mut in_section = false;
        let mut body = String::new();
        for line in content.lines() {
            if line.trim() == heading.trim() {
                in_section = true;
                continue;
            }
            if in_section {
                if line.starts_with("## ") {
                    break;
                }
                body.push_str(line);
                body.push('\n');
            }
        }
        let trimmed = body.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    /// Upsert body text under a `## key` heading.
    pub fn set(&self, key: &str, value: &str) -> Result<(), MemoryError> {
        let content = self.read_all().unwrap_or_default();
        let heading = format!("## {key}");
        // Each section: heading + blank line + value + trailing newline
        let new_section = format!("{heading}\n{value}\n\n");

        let updated = if content.contains(&heading) {
            // Replace existing section
            let mut out = String::with_capacity(content.len());
            let mut in_section = false;
            let mut replaced = false;
            for line in content.lines() {
                if line.trim() == heading.trim() {
                    in_section = true;
                    if !replaced {
                        out.push_str(&new_section);
                        replaced = true;
                    }
                    continue;
                }
                if in_section {
                    if line.starts_with("## ") {
                        in_section = false;
                        out.push_str(line);
                        out.push('\n');
                    }
                    // skip old body lines
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out
        } else {
            // Append new section
            let mut out = content;
            out.push_str(&new_section);
            out
        };

        fs::write(&self.path, updated.as_bytes()).map_err(MemoryError::Io)?;
        Ok(())
    }

    /// List all `## heading` keys in MEMORY.md.
    pub fn list_keys(&self) -> Result<Vec<String>, MemoryError> {
        let content = self.read_all().unwrap_or_default();
        Ok(content
            .lines()
            .filter(|l| l.starts_with("## "))
            .map(|l| l[3..].trim().to_string())
            .collect())
    }
}
