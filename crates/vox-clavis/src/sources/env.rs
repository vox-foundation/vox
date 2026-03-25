use secrecy::SecretString;

use crate::spec::SecretSpec;
use crate::types::{ResolutionStatus, SecretSource};

#[must_use]
pub fn resolve_env(
    spec: SecretSpec,
) -> (Option<SecretString>, Option<SecretSource>, ResolutionStatus) {
    if let Ok(v) = std::env::var(spec.canonical_env) {
        if v.trim().is_empty() {
            return (None, None, ResolutionStatus::InvalidEmpty);
        }
        return (
            Some(SecretString::new(v.into_boxed_str())),
            Some(SecretSource::EnvCanonical),
            ResolutionStatus::Present,
        );
    }
    for alias in spec.aliases {
        if let Ok(v) = std::env::var(alias) {
            if v.trim().is_empty() {
                return (None, None, ResolutionStatus::InvalidEmpty);
            }
            return (
                Some(SecretString::new(v.into_boxed_str())),
                Some(SecretSource::EnvAlias),
                ResolutionStatus::Present,
            );
        }
    }
    for alias in spec.deprecated_aliases {
        if let Ok(v) = std::env::var(alias) {
            if v.trim().is_empty() {
                return (None, None, ResolutionStatus::InvalidEmpty);
            }
            return (
                Some(SecretString::new(v.into_boxed_str())),
                Some(SecretSource::EnvAlias),
                ResolutionStatus::DeprecatedAliasUsed,
            );
        }
    }
    (None, None, ResolutionStatus::MissingOptional)
}
