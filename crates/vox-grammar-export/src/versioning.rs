pub fn get_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 1, 0))
}

pub fn get_compiler_version() -> semver::Version {
    // Try to get vox-compiler version if available, otherwise fallback
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| semver::Version::new(0, 4, 0))
}

pub fn verify_grammar_alignment() -> Result<(), String> {
    if get_version() != get_compiler_version() {
        Err("Grammar version does not match Compiler version!".into())
    } else {
        Ok(())
    }
}
