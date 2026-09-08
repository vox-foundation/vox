use std::collections::HashMap;

use chromiumoxide::Browser;
use chromiumoxide_cdp::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide_cdp::cdp::browser_protocol::page::{
    GetNavigationHistoryParams, NavigateToHistoryEntryParams, ReloadParams, StopLoadingParams,
};
use serde::Serialize;
use tracing::debug;

use crate::engine::BrowserEngine;
use crate::policy::{cookie_export_public_json, require_named_consent, validate_navigation_url};
use crate::resolve::history_capabilities;
use crate::snapshot::AxRef;

pub(crate) struct HostInner {
    pub(crate) _handler_task: tokio::task::JoinHandle<()>,
    pub(crate) browser: Browser,
    pub(crate) pages: HashMap<String, chromiumoxide::Page>,
    pub(crate) viewports: HashMap<String, ViewportMetrics>,
    pub(crate) ref_maps: HashMap<String, std::collections::BTreeMap<String, AxRef>>,
    pub(crate) consent_profile_id: Option<String>,
    /// When true (Attach), last-page drop disconnects and must not kill Chrome.
    pub(crate) disconnect_only: bool,
}

impl HostInner {
    pub(crate) fn new(
        handler_task: tokio::task::JoinHandle<()>,
        browser: Browser,
        consent_profile_id: Option<String>,
        disconnect_only: bool,
    ) -> Self {
        Self {
            _handler_task: handler_task,
            browser,
            pages: HashMap::new(),
            viewports: HashMap::new(),
            ref_maps: HashMap::new(),
            consent_profile_id,
            disconnect_only,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct PageSummary {
    page_id: String,
    url: String,
    title: String,
}

#[derive(Debug, Clone, Serialize)]
struct PageInfo {
    page_id: String,
    url: String,
    title: String,
    can_go_back: bool,
    can_go_forward: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewportMetrics {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Default for ViewportMetrics {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
        }
    }
}

impl BrowserEngine {
    pub async fn list_pages(&self) -> Result<serde_json::Value, String> {
        let pages: Vec<(String, chromiumoxide::Page)> = {
            let guard = self.host.lock().await;
            if guard.is_empty() {
                return Err("no browser host; call open first".to_string());
            }
            guard.all_pages()
        };
        let mut out = Vec::with_capacity(pages.len());
        for (page_id, page) in pages {
            let url = page_url(&page).await.unwrap_or_default();
            let title = page
                .get_title()
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            out.push(PageSummary {
                page_id,
                url,
                title,
            });
        }
        serde_json::to_value(out).map_err(|e| e.to_string())
    }

    pub async fn page_info(&self, page_id: &str) -> Result<serde_json::Value, String> {
        let page = self.page_ref(page_id).await?;
        let history = page
            .execute(GetNavigationHistoryParams::default())
            .await
            .map_err(Self::map_page_err)?;
        let current_index = history.current_index as usize;
        let total = history.entries.len();
        let url = page_url(&page).await.unwrap_or_default();
        let title = page
            .get_title()
            .await
            .unwrap_or_default()
            .unwrap_or_default();
        let (can_go_back, can_go_forward) = history_capabilities(current_index, total);
        let info = PageInfo {
            page_id: page_id.to_string(),
            url,
            title,
            can_go_back,
            can_go_forward,
        };
        serde_json::to_value(info).map_err(|e| e.to_string())
    }

    pub(crate) fn map_page_err(e: chromiumoxide::error::CdpError) -> String {
        e.to_string()
    }

    pub(crate) async fn page_ref(&self, page_id: &str) -> Result<chromiumoxide::Page, String> {
        let guard = self.host.lock().await;
        guard.page(page_id)
    }

    pub async fn close(&self, page_id: &str) -> Result<(), String> {
        let mut guard = self.host.lock().await;
        if let Some((page, dropped)) = guard.remove_page(page_id) {
            let _ = page.close().await;
            if let Some(inner) = dropped {
                inner._handler_task.abort();
                // Attach: drop disconnects the websocket only (no child, no Browser.close).
                if inner.disconnect_only {
                    drop(inner.browser);
                    debug!(target: "vox_plugin_browser", "attach host disconnected (no sessions)");
                } else {
                    drop(inner.browser);
                    debug!(target: "vox_plugin_browser", "browser host shut down (no sessions)");
                }
            }
        }
        Ok(())
    }

    pub async fn goto(&self, page_id: &str, url: &str) -> Result<(), String> {
        validate_navigation_url(url)?;
        let page = self.page_ref(page_id).await?;
        self.clear_ref_map(page_id).await;
        page.goto(url).await.map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn back(&self, page_id: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        self.clear_ref_map(page_id).await;
        let history = page
            .execute(GetNavigationHistoryParams::default())
            .await
            .map_err(Self::map_page_err)?;
        let index = history.current_index as usize;
        if index == 0 || history.entries.is_empty() {
            return Ok(());
        }
        let entry_id = history
            .entries
            .get(index - 1)
            .ok_or_else(|| "no previous history entry".to_string())?
            .id;
        page.execute(NavigateToHistoryEntryParams { entry_id })
            .await
            .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn forward(&self, page_id: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        self.clear_ref_map(page_id).await;
        let history = page
            .execute(GetNavigationHistoryParams::default())
            .await
            .map_err(Self::map_page_err)?;
        let index = history.current_index as usize;
        let next = index + 1;
        if history.entries.is_empty() || next >= history.entries.len() {
            return Ok(());
        }
        let entry_id = history.entries[next].id;
        page.execute(NavigateToHistoryEntryParams { entry_id })
            .await
            .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn reload(&self, page_id: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        self.clear_ref_map(page_id).await;
        page.execute(ReloadParams::default())
            .await
            .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn stop(&self, page_id: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        page.execute(StopLoadingParams::default())
            .await
            .map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn set_viewport(&self, page_id: &str, width: u32, height: u32) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let params = SetDeviceMetricsOverrideParams::builder()
            .width(width as i64)
            .height(height as i64)
            .device_scale_factor(1.0)
            .mobile(false)
            .build()
            .map_err(|e| e.to_string())?;
        page.execute(params).await.map_err(Self::map_page_err)?;
        let mut guard = self.host.lock().await;
        if let Ok(host) = guard.host_for_page_mut(page_id) {
            host.viewports
                .insert(page_id.to_string(), ViewportMetrics { width, height });
        }
        Ok(())
    }

    pub async fn cookies_export(&self, page_id: &str) -> Result<serde_json::Value, String> {
        let (is_attach, consent_id, dest_id) = {
            let guard = self.host.lock().await;
            let host = guard.host_for_page(page_id)?;
            let dest_id = host
                .consent_profile_id
                .clone()
                .unwrap_or_else(|| page_id.to_string());
            (
                host.disconnect_only,
                host.consent_profile_id.clone(),
                dest_id,
            )
        };
        if is_attach {
            let profiles_root = vox_config::paths::browser_profiles_dir();
            let id = consent_id.as_deref().unwrap_or("attach");
            require_named_consent(&profiles_root, id, false)?;
        }
        let cookies = {
            let guard = self.host.lock().await;
            let host = guard.host_for_page(page_id)?;
            host.browser
                .get_cookies()
                .await
                .map_err(|e| format!("get_cookies: {e}"))?
        };
        let count = cookies.len();
        let dest_dir = vox_config::paths::browser_profiles_dir().join(dest_id);
        std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
        let path = dest_dir.join("cookies.json");
        let json = serde_json::to_vec_pretty(&cookies).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(cookie_export_public_json(&path.to_string_lossy(), count))
    }

    pub async fn cookies_import(&self, page_id: &str, cookies_json: &str) -> Result<(), String> {
        let params = cookies_json_to_params(cookies_json)?;
        let guard = self.host.lock().await;
        let host = guard.host_for_page(page_id)?;
        host.browser
            .set_cookies(params)
            .await
            .map_err(|e| format!("set_cookies: {e}"))?;
        Ok(())
    }

    pub(crate) async fn viewport_for(&self, page_id: &str) -> ViewportMetrics {
        let guard = self.host.lock().await;
        guard
            .host_for_page(page_id)
            .ok()
            .and_then(|host| host.viewports.get(page_id))
            .copied()
            .unwrap_or_default()
    }
}

fn cookies_json_to_params(
    cookies_json: &str,
) -> Result<Vec<chromiumoxide_cdp::cdp::browser_protocol::network::CookieParam>, String> {
    serde_json::from_str(cookies_json).map_err(|e| format!("cookies_json: {e}"))
}

async fn page_url(page: &chromiumoxide::Page) -> Result<String, String> {
    page.evaluate("window.location.href")
        .await
        .map_err(|e| e.to_string())?
        .into_value::<String>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookies_json_to_params_reads_name_and_value() {
        let params = cookies_json_to_params(
            r#"[{"name":"sid","value":"abc","domain":"example.com","path":"/"}]"#,
        )
        .expect("parse");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "sid");
        assert_eq!(params[0].value, "abc");
        assert_eq!(params[0].domain.as_deref(), Some("example.com"));
        assert_eq!(params[0].path.as_deref(), Some("/"));
    }

    #[test]
    fn cookies_json_to_params_rejects_missing_name() {
        let err = cookies_json_to_params(r#"[{"value":"abc"}]"#).unwrap_err();
        assert!(err.contains("name"), "got {err}");
    }

    #[test]
    fn cookies_json_to_params_preserves_exported_samesite_none() {
        use chromiumoxide_cdp::cdp::browser_protocol::network::CookieSameSite;
        let params = cookies_json_to_params(
            r#"[{
                "name":"sid",
                "value":"abc",
                "domain":"example.com",
                "path":"/",
                "sameSite":"none",
                "sourcePort":443,
                "partitionKey":{
                    "topLevelSite":"https://example.com",
                    "hasCrossSiteAncestor":false
                }
            }]"#,
        )
        .expect("parse exported-shaped cookie JSON");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].same_site, Some(CookieSameSite::None));
        assert_eq!(params[0].source_port, Some(443));
        let key = params[0].partition_key.as_ref().expect("partitionKey");
        assert_eq!(key.top_level_site, "https://example.com");
        assert!(!key.has_cross_site_ancestor);
    }
}
