/// Emit a deprecation warning to stderr telling the user the new canonical command.
///
/// Used by hidden root-level aliases that delegate to subcommand groups.
pub fn warn_deprecated(old: &str, new: &str) {
    eprintln!("⚠  `vox {old}` is deprecated — use `vox {new}` instead.");
}

/// Emit a deprecation warning using a stable capability id (reserved for future registry wiring).
///
/// Today this always uses `fallback` as the canonical replacement string so the CLI stays
/// self-contained without an external capability-registry crate.
pub fn warn_deprecated_from_registry(old: &str, _capability_id: &str, fallback: &str) {
    warn_deprecated(old, fallback);
}
