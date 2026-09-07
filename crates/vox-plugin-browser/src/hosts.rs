use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chromiumoxide::Browser;
use chromiumoxide::browser::BrowserConfig;
use futures::StreamExt;
use tracing::debug;

use crate::engine::BrowserEngine;
use crate::host::{HostInner, ViewportMetrics};
use crate::policy::{
    BrowserLaunchMode, BrowserLaunchOptions, parse_profile_id, record_save_consent,
    require_named_consent, validate_navigation_url,
};

/// Process-local Chrome identity. One Chromium per key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostKey {
    Ephemeral,
    Named(String),
    Attach(String),
}

pub(crate) struct HostRegistry {
    hosts: HashMap<HostKey, HostInner>,
    page_keys: HashMap<String, HostKey>,
}

impl HostRegistry {
    pub(crate) fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            page_keys: HashMap::new(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.hosts.is_empty()
    }

    pub(crate) fn contains(&self, key: &HostKey) -> bool {
        self.hosts.contains_key(key)
    }

    pub(crate) fn get_mut(&mut self, key: &HostKey) -> Option<&mut HostInner> {
        self.hosts.get_mut(key)
    }

    pub(crate) fn insert_host(&mut self, key: HostKey, host: HostInner) {
        self.hosts.insert(key, host);
    }

    pub(crate) fn host_for_page(&self, page_id: &str) -> Result<&HostInner, String> {
        let key = self
            .page_keys
            .get(page_id)
            .ok_or_else(|| format!("unknown page_id {page_id:?}"))?;
        self.hosts
            .get(key)
            .ok_or_else(|| "browser host missing".to_string())
    }

    pub(crate) fn host_for_page_mut(&mut self, page_id: &str) -> Result<&mut HostInner, String> {
        let key = self
            .page_keys
            .get(page_id)
            .cloned()
            .ok_or_else(|| format!("unknown page_id {page_id:?}"))?;
        self.hosts
            .get_mut(&key)
            .ok_or_else(|| "browser host missing".to_string())
    }

    pub(crate) fn page(&self, page_id: &str) -> Result<chromiumoxide::Page, String> {
        if self.hosts.is_empty() {
            return Err("no browser host; call open first".to_string());
        }
        self.host_for_page(page_id)?
            .pages
            .get(page_id)
            .cloned()
            .ok_or_else(|| format!("unknown page_id {page_id:?}"))
    }

    pub(crate) fn all_pages(&self) -> Vec<(String, chromiumoxide::Page)> {
        self.hosts
            .values()
            .flat_map(|host| {
                host.pages
                    .iter()
                    .map(|(id, page)| (id.clone(), page.clone()))
            })
            .collect()
    }

    pub(crate) fn insert_page(&mut self, key: HostKey, page_id: String, page: chromiumoxide::Page) {
        if let Some(host) = self.hosts.get_mut(&key) {
            host.pages.insert(page_id.clone(), page);
            host.viewports
                .insert(page_id.clone(), ViewportMetrics::default());
        }
        self.page_keys.insert(page_id, key);
    }

    pub(crate) fn remove_page(
        &mut self,
        page_id: &str,
    ) -> Option<(chromiumoxide::Page, Option<HostInner>)> {
        let key = self.page_keys.remove(page_id)?;
        let (page, empty) = {
            let host = self.hosts.get_mut(&key)?;
            let page = host.pages.remove(page_id)?;
            host.viewports.remove(page_id);
            host.ref_maps.remove(page_id);
            (page, host.pages.is_empty())
        };
        let dropped = if empty { self.hosts.remove(&key) } else { None };
        Some((page, dropped))
    }

    pub(crate) fn debug_host_key(&self, page_id: &str) -> Option<HostKey> {
        self.page_keys.get(page_id).cloned()
    }
}

pub fn named_user_data_dir(root: &Path, profile_id: &str) -> PathBuf {
    root.join(profile_id)
}

fn build_browser_config(
    headless: bool,
    user_data_dir: Option<&Path>,
) -> Result<BrowserConfig, String> {
    let mut builder = BrowserConfig::builder()
        .request_timeout(vox_config::timeouts::BROWSER_CDP_REQUEST)
        .launch_timeout(vox_config::timeouts::D_60S);
    builder = if headless {
        builder.new_headless_mode()
    } else {
        builder.with_head()
    };
    if let Some(dir) = user_data_dir {
        builder = builder.user_data_dir(dir);
    }
    if let Ok(exe) = std::env::var("VOX_CHROME_EXECUTABLE") {
        let exe = exe.trim();
        if !exe.is_empty() {
            builder = builder.chrome_executable(exe);
        }
    }
    if std::env::var("VOX_BROWSER_NO_SANDBOX")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        builder = builder.no_sandbox();
    }
    builder.build().map_err(|e| format!("browser config: {e}"))
}

impl BrowserEngine {
    async fn ensure_host(
        &self,
        key: HostKey,
        headless: bool,
        user_data_dir: Option<PathBuf>,
    ) -> Result<(), String> {
        let mut guard = self.host.lock().await;
        if guard.contains(&key) {
            return match key {
                HostKey::Named(_) => Err("host_mode_conflict".to_string()),
                _ => Ok(()),
            };
        }

        let config = build_browser_config(headless, user_data_dir.as_deref())?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("Browser::launch failed: {e}"))?;
        let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });
        guard.insert_host(key, HostInner::new(handler_task, browser));
        debug!(target: "vox_plugin_browser", "chromium host launched");
        Ok(())
    }

    async fn open_on_host(
        &self,
        key: HostKey,
        url: &str,
        headless: bool,
        user_data_dir: Option<PathBuf>,
    ) -> Result<String, String> {
        validate_navigation_url(url)?;
        self.ensure_host(key.clone(), headless, user_data_dir)
            .await?;
        let mut guard = self.host.lock().await;
        let page = {
            let host = guard
                .get_mut(&key)
                .ok_or_else(|| "browser host missing".to_string())?;
            host.browser
                .new_page("about:blank")
                .await
                .map_err(|e| format!("new_page: {e}"))?
        };
        page.goto(url)
            .await
            .map_err(|e| format!("goto {url}: {e}"))?;
        let id = format!("page-{}", uuid::Uuid::new_v4());
        guard.insert_page(key, id.clone(), page);
        Ok(id)
    }

    pub async fn open(&self, url: &str, headless: bool) -> Result<String, String> {
        self.open_on_host(HostKey::Ephemeral, url, headless, None)
            .await
    }

    pub async fn open_ex(
        &self,
        opts: BrowserLaunchOptions,
        save_profile: bool,
    ) -> Result<String, String> {
        match opts.mode {
            BrowserLaunchMode::Ephemeral => {
                self.open_on_host(HostKey::Ephemeral, &opts.url, opts.headless, None)
                    .await
            }
            BrowserLaunchMode::Named => {
                let profile_id = parse_profile_id(opts.profile_id.as_deref().unwrap_or(""))?;
                let profiles_root = vox_config::paths::browser_profiles_dir();
                require_named_consent(&profiles_root, &profile_id, save_profile)?;
                if save_profile {
                    record_save_consent(&profiles_root, &profile_id)?;
                }
                let user_data_dir = named_user_data_dir(&profiles_root, &profile_id);
                self.open_on_host(
                    HostKey::Named(profile_id),
                    &opts.url,
                    opts.headless,
                    Some(user_data_dir),
                )
                .await
            }
            BrowserLaunchMode::Attach => Err("attach_not_implemented".to_string()),
        }
    }

    #[cfg(test)]
    pub(crate) async fn debug_host_key(&self, page_id: &str) -> Option<HostKey> {
        self.host.lock().await.debug_host_key(page_id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(unsafe_code)] // edition 2024 set_var in ignored Chrome smoke
    use super::*;
    use crate::policy::BrowserLaunchMode;

    #[test]
    fn host_key_ephemeral_and_named_are_distinct() {
        assert_ne!(HostKey::Ephemeral, HostKey::Named("staging-1".into()));
        assert_ne!(
            HostKey::Named("staging-1".into()),
            HostKey::Attach("http://127.0.0.1:9222".into())
        );
    }

    #[test]
    fn named_user_data_dir_is_under_profiles_root() {
        let root = PathBuf::from("/tmp/vox-profiles-test");
        let dir = named_user_data_dir(&root, "staging-1");
        assert_eq!(dir, root.join("staging-1"));
    }

    #[tokio::test]
    async fn attach_is_not_implemented() {
        let engine = BrowserEngine::new();
        let err = engine
            .open_ex(
                BrowserLaunchOptions {
                    url: "http://127.0.0.1/".into(),
                    headless: true,
                    mode: BrowserLaunchMode::Attach,
                    profile_id: None,
                    cdp_url: Some("http://127.0.0.1:9222".into()),
                },
                false,
            )
            .await
            .unwrap_err();
        assert_eq!(err, "attach_not_implemented");
    }

    #[tokio::test]
    #[ignore = "slow; requires local Chrome/Chromium binary"]
    async fn named_profile_isolates_cookies_from_ephemeral() {
        let profiles = std::env::temp_dir().join(format!(
            "vox-named-smoke-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        unsafe {
            std::env::set_var("VOX_BROWSER_PROFILES_DIR", &profiles);
        }
        let engine = BrowserEngine::new();
        let named_opts = BrowserLaunchOptions {
            url: "https://example.com".into(),
            headless: true,
            mode: BrowserLaunchMode::Named,
            profile_id: Some("staging-1".into()),
            cdp_url: None,
        };
        let named_id = engine
            .open_ex(named_opts.clone(), true)
            .await
            .expect("named open_ex");
        let named_page = engine.page_ref(&named_id).await.expect("named page");
        named_page
            .evaluate("document.cookie = 'vox_named=1; path=/'")
            .await
            .expect("set cookie");
        engine.close(&named_id).await.expect("close named");

        let again = engine
            .open_ex(named_opts, false)
            .await
            .expect("reopen named");
        let again_page = engine.page_ref(&again).await.expect("reopened page");
        let cookie = again_page
            .evaluate("document.cookie")
            .await
            .expect("read cookie")
            .into_value::<String>()
            .expect("cookie string");
        assert!(
            cookie.contains("vox_named=1"),
            "named cookie missing after relaunch: {cookie:?}"
        );

        let ephemeral = engine
            .open("https://example.com", true)
            .await
            .expect("ephemeral open");
        assert_ne!(
            engine.debug_host_key(&again).await,
            engine.debug_host_key(&ephemeral).await
        );
        let eph_page = engine.page_ref(&ephemeral).await.expect("ephemeral page");
        let eph_cookie = eph_page
            .evaluate("document.cookie")
            .await
            .expect("read ephemeral cookie")
            .into_value::<String>()
            .unwrap_or_default();
        assert!(
            !eph_cookie.contains("vox_named=1"),
            "ephemeral page saw named cookie: {eph_cookie:?}"
        );
        engine.close(&again).await.ok();
        engine.close(&ephemeral).await.ok();
        unsafe {
            std::env::remove_var("VOX_BROWSER_PROFILES_DIR");
        }
        let _ = std::fs::remove_dir_all(&profiles);
    }
}
