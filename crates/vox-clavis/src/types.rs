use secrecy::{ExposeSecret, SecretString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStatus {
    Present,
    MissingOptional,
    MissingRequired,
    InvalidEmpty,
    DeprecatedAliasUsed,
    RejectedLegacyAlias,
    RejectedSourcePolicy,
    RejectedClassPolicy,
    BackendUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSource {
    EnvCanonical,
    EnvAlias,
    SecureStore,
    AuthJson,
    LegacyAuthToken,
    PopuliEnv,
    ExternalBackend,
}

#[derive(Clone)]
pub struct ResolvedSecret {
    pub id: crate::spec::SecretId,
    pub value: Option<SecretString>,
    pub source: Option<SecretSource>,
    pub status: ResolutionStatus,
    pub remediation: &'static str,
    pub detail: Option<String>,
}

impl ResolvedSecret {
    #[must_use]
    pub fn is_present(&self) -> bool {
        self.value.is_some()
    }

    #[must_use]
    pub fn expose(&self) -> Option<&str> {
        self.value.as_ref().map(ExposeSecret::expose_secret)
    }

    #[must_use]
    pub fn redacted(&self) -> String {
        match self.expose() {
            Some(v) if v.chars().count() > 6 => {
                let head: String = v.chars().take(4).collect();
                let tail: String = v
                    .chars()
                    .rev()
                    .take(2)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("{head}…{tail} (redacted)")
            }
            Some(_) => "***".to_string(),
            None => "(missing)".to_string(),
        }
    }
}
