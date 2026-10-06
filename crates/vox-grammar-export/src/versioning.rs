pub fn get_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 4, 0))
}

pub fn get_compiler_version() -> semver::Version {
    // In a real implementation, this might read from a different crate or a build-time constant.
    // For now, we use the package version but we'll differentiate the alignment check.
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 4, 0))
}

/// Compute a SHA256 hex hash for the grammar based on EBNF production rules.
pub fn compute_ebnf_hash() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(crate::ebnf::emit_ebnf().as_bytes());
    hex::encode(hasher.finalize())
}

/// The SHA256 hash of the grammar at the time this crate was built.
/// Updated via `vox grammar` sync.
pub const BUILT_GRAMMAR_HASH: &str =
    "fa51880e9bc0828c4d3c8f0d64f10f0f82bf0d13cc7a25e4e9e8a222d0ef737f";

pub fn verify_grammar_alignment() -> Result<(), String> {
    let live_hash = compute_ebnf_hash();

    if live_hash != BUILT_GRAMMAR_HASH {
        Err(format!(
            "Grammar mismatch: built hash {}..., live hash {}... Run `vox grammar --format ebnf > GRAMMAR.ebnf` and update BUILT_GRAMMAR_HASH.",
            &BUILT_GRAMMAR_HASH[..8],
            &live_hash[..8]
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebnf_hash_is_stable_sha256_hex() {
        let h = compute_ebnf_hash();
        assert_eq!(h.len(), 64);
        assert!(h.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(h, compute_ebnf_hash(), "hash must be deterministic");
    }

    #[test]
    fn pinned_hash_matches_live_grammar() {
        verify_grammar_alignment().unwrap();
    }
}
