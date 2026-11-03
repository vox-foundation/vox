//! Repeatable `--caps` tokens for a mesh-spawned interpreter child.
//!
//! `from_roots` + `to_tokens` only. Path roots never go through the comma-split
//! string grammar, so a job directory containing `,` is fine. The child parses
//! each token; this crate does not depend on `vox-compiler`.
//!
//! // vox:defactored-from vox-compiler 2026-09-06

use std::path::PathBuf;

/// Extra tokens the mesh executor may attach. `vox-compiler` parses each of
/// these individually so the two grammars cannot drift.
pub const MESH_EXTRA_TOKENS: &[&str] = &["time:real", "net:allow", "process:allow", "env:ro"];

/// A receiver-imposed capability grant, already turned into CLI tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapsSpec {
    tokens: Vec<String>,
}

impl CapsSpec {
    /// Typed constructor: roots are `PathBuf`s, extras are individual tokens.
    pub fn from_roots(ro: Vec<PathBuf>, rw: Vec<PathBuf>, extra: &[&str]) -> anyhow::Result<Self> {
        let mut tokens = Vec::new();
        for d in ro {
            let root = std::fs::canonicalize(&d).map_err(|e| {
                anyhow::anyhow!(
                    "fs root does not exist or is unreadable ({}): {e}",
                    d.display()
                )
            })?;
            tokens.push(format!("fs:ro={}", root.display()));
        }
        for d in rw {
            let root = std::fs::canonicalize(&d).map_err(|e| {
                anyhow::anyhow!(
                    "fs root does not exist or is unreadable ({}): {e}",
                    d.display()
                )
            })?;
            tokens.push(format!("fs:rw={}", root.display()));
        }
        for tok in extra {
            tokens.push((*tok).to_string());
        }
        Ok(Self { tokens })
    }

    /// Tokens suitable for `vox run --caps <token> --caps <token>`.
    pub fn to_tokens(&self) -> Vec<String> {
        self.tokens.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_roots_accepts_a_comma_in_the_directory() {
        let d = tempfile::Builder::new().prefix("a,b-").tempdir().unwrap();
        let spec = CapsSpec::from_roots(vec![], vec![d.path().to_path_buf()], &["time:real"])
            .expect("comma in the directory must be accepted");
        let tokens = spec.to_tokens();
        assert!(tokens.iter().any(|t| t.starts_with("fs:rw=")), "{tokens:?}");
        assert!(tokens.iter().any(|t| t == "time:real"), "{tokens:?}");
        let joined = tokens.join(",");
        assert!(
            joined.contains("a,b-") || tokens.iter().any(|t| t.contains(',')),
            "the path with a comma must survive as one token: {tokens:?}"
        );
    }

    #[test]
    fn extra_tokens_are_emitted_individually() {
        let d = tempfile::tempdir().unwrap();
        let spec = CapsSpec::from_roots(
            vec![],
            vec![d.path().to_path_buf()],
            &["time:real", "env:ro"],
        )
        .unwrap();
        let tokens = spec.to_tokens();
        assert!(tokens.iter().any(|t| t == "time:real"), "{tokens:?}");
        assert!(tokens.iter().any(|t| t == "env:ro"), "{tokens:?}");
        assert!(
            !tokens.iter().any(|t| t.contains("time:real,")),
            "{tokens:?}"
        );
    }
}
