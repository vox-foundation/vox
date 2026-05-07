use anyhow::{Context, Result};
use base64::Engine;
use rand::RngCore;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardToken(pub String);

impl DashboardToken {
    #[allow(dead_code)]
    pub fn generate_or_load(state_dir: &Path) -> Result<Self> {
        let token_path = state_dir.join("dashboard.token");

        if token_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&token_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        // 30 days
                        if elapsed.as_secs() < 30 * 24 * 3600 {
                            if let Ok(token_str) = std::fs::read_to_string(&token_path) {
                                let token_str = token_str.trim().to_string();
                                if !token_str.is_empty() {
                                    return Ok(Self(token_str));
                                }
                            }
                        }
                    }
                }
            }
        }

        if !state_dir.exists() {
            std::fs::create_dir_all(state_dir)
                .context("failed to create state_dir for dashboard token")?;
        }

        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        let token_str = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true).mode(0o600);
            let mut file = options
                .open(&token_path)
                .context("failed to open dashboard.token")?;
            use std::io::Write;
            file.write_all(token_str.as_bytes())
                .context("failed to write dashboard.token")?;

            let now = std::time::SystemTime::now();
            let _ = file.set_modified(now);
        }
        #[cfg(windows)]
        {
            std::fs::write(&token_path, &token_str).context("failed to write dashboard.token")?;
            if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&token_path) {
                let now = std::time::SystemTime::now();
                let _ = file.set_modified(now);
            }
        }

        Ok(Self(token_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_and_load() {
        let dir = tempdir().unwrap();
        let token1 = DashboardToken::generate_or_load(dir.path()).unwrap();
        assert_eq!(token1.0.len(), 43); // 32 bytes base64 encoded without padding

        // Should load the same token
        let token2 = DashboardToken::generate_or_load(dir.path()).unwrap();
        assert_eq!(token1, token2);
    }
}
