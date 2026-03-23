/// Configuration for opening **Codex** / [`crate::VoxDb`] (Turso or libSQL).
#[derive(Debug, Clone)]
pub enum DbConfig {
    /// Remote Turso / libSQL (sync client; see `vox-pm` `open_remote`).
    Remote {
        /// Database URL (e.g. `libsql://...`).
        url: String,
        /// Auth token for the remote.
        token: String,
    },

    /// Persistent local file (requires `local` feature).
    #[cfg(feature = "local")]
    Local {
        /// Path passed to `turso::Builder::new_local`.
        path: String,
    },

    /// Ephemeral `:memory:` database for unit tests (requires `local` feature).
    #[cfg(feature = "local")]
    Memory,

    /// Local path replicated against a remote primary (requires `replication` feature).
    #[cfg(feature = "replication")]
    EmbeddedReplica {
        /// On-disk path for the replica.
        local_path: String,
        /// Remote URL.
        url: String,
        /// Remote auth token.
        token: String,
    },
}

fn try_remote_from_compat_env() -> Option<DbConfig> {
    if let (Ok(url), Ok(token)) = (
        std::env::var("VOX_TURSO_URL"),
        std::env::var("VOX_TURSO_TOKEN"),
    ) {
        return Some(DbConfig::remote(url, token));
    }
    if let (Ok(url), Ok(token)) = (
        std::env::var("TURSO_URL"),
        std::env::var("TURSO_AUTH_TOKEN"),
    ) {
        return Some(DbConfig::remote(url, token));
    }
    None
}

impl DbConfig {
    /// Create a remote config from URL and token.
    pub fn remote(url: impl Into<String>, token: impl Into<String>) -> Self {
        Self::Remote {
            url: url.into(),
            token: token.into(),
        }
    }

    /// Create a local file config (requires `local` feature).
    #[cfg(feature = "local")]
    pub fn local(path: impl Into<String>) -> Self {
        Self::Local { path: path.into() }
    }

    /// Create an in-memory config for testing (requires `local` feature).
    #[cfg(feature = "local")]
    pub fn memory() -> Self {
        Self::Memory
    }

    /// Create an embedded replica config (requires `replication` feature).
    #[cfg(feature = "replication")]
    pub fn embedded_replica(
        local_path: impl Into<String>,
        url: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        Self::EmbeddedReplica {
            local_path: local_path.into(),
            url: url.into(),
            token: token.into(),
        }
    }

    /// Read config from `VOX_DB_URL` + `VOX_DB_TOKEN` (remote), or `VOX_DB_PATH` (local), or all
    /// three for embedded replica when `replication` is enabled. Empty env + `local` → [`Self::Memory`].
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("VOX_DB_URL").ok();
        let token = std::env::var("VOX_DB_TOKEN").ok();
        let path = std::env::var("VOX_DB_PATH").ok();

        match (url, token, path) {
            (Some(_u), Some(_t), Some(_p)) => {
                #[cfg(feature = "replication")]
                return Ok(Self::embedded_replica(_p, _u, _t));
                #[cfg(not(feature = "replication"))]
                return Err("Embedded replica config requires 'replication' feature".into());
            }
            (Some(u), Some(t), None) => Ok(Self::remote(u, t)),
            (None, None, Some(_p)) => {
                #[cfg(feature = "local")]
                return Ok(Self::local(_p));
                #[cfg(not(feature = "local"))]
                return Err("Local DB config requires 'local' feature".into());
            }
            (None, None, None) => {
                #[cfg(feature = "local")]
                return Ok(Self::memory());
                #[cfg(not(feature = "local"))]
                return Err("Memory DB config requires 'local' feature".into());
            }
            _ => Err("Invalid database configuration in environment variables".into()),
        }
    }

    /// Resolve configuration for long-running apps and CLIs: canonical `VOX_DB_*`, then compatibility
    /// aliases `VOX_TURSO_URL`/`VOX_TURSO_TOKEN`, then legacy `TURSO_URL`/`TURSO_AUTH_TOKEN`, then local
    /// file (`VOX_DB_PATH`, platform default, or `app.db`).
    ///
    /// Unlike [`Self::from_env`], never returns [`Self::Memory`] when the `local` feature is enabled;
    /// an empty environment selects a concrete file path instead.
    pub fn resolve_standalone() -> Result<Self, String> {
        let path_fallback = || {
            std::env::var("VOX_DB_PATH")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    vox_config::paths::default_db_path().map(|p| p.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| "app.db".into())
        };

        match Self::from_env() {
            Ok(cfg) => {
                #[cfg(feature = "local")]
                if matches!(cfg, Self::Memory) {
                    return Ok(Self::local(path_fallback()));
                }
                Ok(cfg)
            }
            Err(_) => {
                if let Some(cfg) = try_remote_from_compat_env() {
                    return Ok(cfg);
                }
                #[cfg(feature = "local")]
                {
                    Ok(Self::local(path_fallback()))
                }
                #[cfg(not(feature = "local"))]
                Err(
                    "Database resolution requires the `local` feature or valid VOX_DB_URL+VOX_DB_TOKEN"
                        .into(),
                )
            }
        }
    }

    /// Resolve config for the **project** Arca [`vox_pm::store::CodeStore`] (snippets, share, etc.).
    ///
    /// Uses canonical [`Self::from_env`] (`VOX_DB_*`), mapping an empty environment to the project
    /// file [`vox_pm::store::DEFAULT_PROJECT_STORE_PATH`] instead of the user data default from
    /// [`Self::resolve_standalone`]. On failure of `from_env`, applies the same Turso compatibility
    /// aliases as [`Self::resolve_standalone`], then falls back to the project store path.
    pub fn resolve_project_code_store_config() -> Result<Self, String> {
        match Self::from_env() {
            Ok(cfg) => {
                #[cfg(feature = "local")]
                if matches!(cfg, Self::Memory) {
                    return Ok(Self::local(
                        vox_pm::store::DEFAULT_PROJECT_STORE_PATH.to_string(),
                    ));
                }
                Ok(cfg)
            }
            Err(_) => {
                if let Some(cfg) = try_remote_from_compat_env() {
                    return Ok(cfg);
                }
                #[cfg(feature = "local")]
                {
                    Ok(Self::local(
                        vox_pm::store::DEFAULT_PROJECT_STORE_PATH.to_string(),
                    ))
                }
                #[cfg(not(feature = "local"))]
                Err(
                    "Project CodeStore requires the `local` feature or valid VOX_DB_URL+VOX_DB_TOKEN"
                        .into(),
                )
            }
        }
    }

    /// Resolve configuration specifically for a mesh node:
    /// - If `VOX_DB_URL`, `VOX_DB_TOKEN`, AND `VOX_DB_PATH` are set, use [`Self::EmbeddedReplica`].
    /// - If only `VOX_DB_URL` + `VOX_DB_TOKEN` are set, use [`Self::Remote`].
    /// - Otherwise, fall back to [`Self::resolve_standalone`] (local file).
    pub fn resolve_for_mesh() -> Result<Self, String> {
        let url = std::env::var("VOX_DB_URL").ok();
        let token = std::env::var("VOX_DB_TOKEN").ok();
        let path = std::env::var("VOX_DB_PATH").ok();

        match (url, token, path) {
            (Some(u), Some(t), Some(_p)) => {
                #[cfg(feature = "replication")]
                return Ok(Self::EmbeddedReplica {
                    local_path: _p,
                    url: u,
                    token: t,
                });
                #[cfg(not(feature = "replication"))]
                {
                    tracing::warn!("EmbeddedReplica requested for mesh but 'replication' feature is disabled; falling back to Remote");
                    Ok(Self::Remote { url: u, token: t })
                }
            }
            (Some(u), Some(t), None) => Ok(Self::Remote { url: u, token: t }),
            _ => Self::resolve_standalone(),
        }
    }
}
