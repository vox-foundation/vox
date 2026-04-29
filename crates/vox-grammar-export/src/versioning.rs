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
    format!("{:x}", hasher.finalize())
}

/// The SHA256 hash of the grammar at the time this crate was built.
/// Updated via `vox grammar` sync.
pub const BUILT_GRAMMAR_HASH: &str =
    "9b88f31bbaa8dc2b6ddc9d6857ce1dd195281163130ef194149df837d1919a13";

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
